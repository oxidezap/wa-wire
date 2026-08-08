//! Compare two recordings and say whether the candidate may ship.
//!
//! Everything this does already existed as library code and was reachable only
//! from tests. That is the gap it closes: a container, a comparator and a set
//! of profiles that nobody can run is a design, not a tool.
//!
//! ```sh
//! wa-wire-gate --profile regression baseline.wawr candidate.wawr
//! ```
//!
//! # The three answers
//!
//! `pass`, `fail`, and **`incomparable`** — which is not a pass. A gate that
//! folded "these were never comparable" into either of the other two would
//! report a conclusion drawn from a comparison that never happened, and that is
//! precisely the failure the container was specified to prevent.
//!
//! Exit codes follow, so a CI step can tell the three apart without parsing
//! the output.
//!
//! # Which dictionary
//!
//! Frames are parsed against the token dictionary they were encoded with
//! (D-082), and this build has exactly one: the bundled table. A recording that
//! names a different one cannot be parsed here, and the gate says so rather
//! than parsing it with the wrong table and blaming an engine for the result.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
    )
)]

use std::fmt::Write as _;

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet};
use wa_wire_codec::TokenTable;
use wa_wire_conformance::{
    Comparability, ComparisonProfile, Incomparable, Recording, Tables, Verdict, compare,
};
use wa_wire_recording::RecordingRef;

/// How many findings are printed before the rest are summarised.
pub const DEFAULT_MAX_FINDINGS: usize = 20;

/// What the process exits with.
///
/// Distinct codes because a CI step has to tell a regression apart from a
/// comparison it should never have been asked to make.
pub mod exit {
    /// Nothing failed under the profile.
    pub const PASS: i32 = 0;
    /// At least one finding failed.
    pub const FAIL: i32 = 1;
    /// The recordings may not be compared.
    pub const INCOMPARABLE: i32 = 2;
    /// The arguments were wrong.
    pub const USAGE: i32 = 64;
    /// A recording could not be read.
    pub const INPUT: i32 = 66;
}

/// Everything the tool was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    /// The recording taken as the reference.
    pub baseline: String,
    /// The recording under test.
    pub candidate: String,
    /// Which question is being asked.
    pub profile: ComparisonProfile,
    /// How many findings to print in full.
    pub max_findings: usize,
}

/// Why the arguments could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// `--help` was asked for. Not a mistake, but not a run either.
    HelpRequested,
    /// A flag needs a value and did not get one.
    MissingValue(String),
    /// A flag's value is not one this build knows.
    BadValue {
        /// The flag.
        flag: String,
        /// What was given.
        value: String,
    },
    /// A flag this build does not know.
    UnknownFlag(String),
    /// Two recordings are needed and fewer were given.
    MissingPaths,
    /// More than two paths were given.
    TooManyPaths,
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HelpRequested => f.write_str(USAGE),
            Self::MissingValue(flag) => write!(f, "{flag} needs a value"),
            Self::BadValue { flag, value } => {
                write!(f, "{flag}: `{value}` is not one this build knows")
            }
            Self::UnknownFlag(flag) => write!(f, "unknown flag `{flag}`"),
            Self::MissingPaths => {
                f.write_str("two recordings are needed: a baseline and a candidate")
            }
            Self::TooManyPaths => f.write_str("exactly two recordings, no more"),
        }
    }
}

impl std::error::Error for UsageError {}

/// What to print when asked, and when the arguments make no sense.
pub const USAGE: &str = "\
wa-wire-gate — compare two recordings and say whether the candidate may ship

    wa-wire-gate [options] <baseline.wawr> <candidate.wawr>

Options:
    --profile <interop|regression>  which question is being asked
                                    (default: regression)
    --max-findings <n>              how many findings to print in full
                                    (default: 20; 0 prints all)
    -h, --help                      this

Profiles:
    interop      two engines, one input: do they mean the same thing?
    regression   one engine, two builds: did the newer one lose anything?
                 Directional — the baseline is the reference.

Exit codes:
    0  pass          2  incomparable    66  a recording could not be read
    1  fail         64  bad arguments
";

impl Cli {
    /// Read the arguments, without the program name.
    ///
    /// # Errors
    ///
    /// [`UsageError`] describing what was wrong, or that help was asked for.
    pub fn parse<I, S>(args: I) -> Result<Self, UsageError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut paths: Vec<String> = Vec::new();
        let mut profile = ComparisonProfile::Regression;
        let mut max_findings = DEFAULT_MAX_FINDINGS;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg.as_ref();
            match arg {
                "-h" | "--help" => return Err(UsageError::HelpRequested),
                "--profile" => {
                    let value = args
                        .next()
                        .ok_or_else(|| UsageError::MissingValue(arg.to_owned()))?;
                    profile = match value.as_ref() {
                        "interop" => ComparisonProfile::Interop,
                        "regression" => ComparisonProfile::Regression,
                        other => {
                            return Err(UsageError::BadValue {
                                flag: arg.to_owned(),
                                value: other.to_owned(),
                            });
                        }
                    };
                }
                "--max-findings" => {
                    let value = args
                        .next()
                        .ok_or_else(|| UsageError::MissingValue(arg.to_owned()))?;
                    max_findings =
                        value
                            .as_ref()
                            .parse::<usize>()
                            .map_err(|_| UsageError::BadValue {
                                flag: arg.to_owned(),
                                value: value.as_ref().to_owned(),
                            })?;
                }
                flag if flag.starts_with('-') && flag.len() > 1 => {
                    return Err(UsageError::UnknownFlag(flag.to_owned()));
                }
                path => paths.push(path.to_owned()),
            }
        }

        let mut paths = paths.into_iter();
        let (Some(baseline), Some(candidate)) = (paths.next(), paths.next()) else {
            return Err(UsageError::MissingPaths);
        };
        if paths.next().is_some() {
            return Err(UsageError::TooManyPaths);
        }

        Ok(Self {
            baseline,
            candidate,
            profile,
            max_findings,
        })
    }
}

/// What the gate concluded, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The verdict, or `None` when a recording could not be read at all.
    pub verdict: Option<Verdict>,
    /// The whole report.
    pub report: String,
}

impl Outcome {
    /// What the process should exit with.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self.verdict {
            Some(Verdict::Pass) => exit::PASS,
            Some(Verdict::Fail) => exit::FAIL,
            Some(Verdict::Incomparable(_)) => exit::INCOMPARABLE,
            None => exit::INPUT,
        }
    }
}

/// Which token dictionary a recording's frames need, and whether we have it.
enum Dictionary {
    /// The recording named the bundled table, or named none at all.
    Bundled {
        /// Whether it said so, or we assumed.
        declared: bool,
    },
    /// The recording named a dictionary this build does not carry.
    Unavailable(String),
}

impl Dictionary {
    fn resolve(recording: &RecordingRef<'_>) -> Self {
        match recording.dictionary() {
            None => Self::Bundled { declared: false },
            Some(name) if name == wa_wire_codec::tokens::SOURCE_DIGEST => {
                Self::Bundled { declared: true }
            }
            Some(other) => Self::Unavailable(other.to_owned()),
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::Bundled { declared: true } => "bundled".to_owned(),
            Self::Bundled { declared: false } => "undeclared, assuming bundled".to_owned(),
            Self::Unavailable(name) => format!("{name} — not available here"),
        }
    }

    const fn is_available(&self) -> bool {
        matches!(self, Self::Bundled { .. })
    }
}

/// Compare two recordings and produce the report.
///
/// Takes bytes rather than paths so that the whole decision is testable without
/// a filesystem.
#[must_use]
pub fn run(baseline: &[u8], candidate: &[u8], profile: ComparisonProfile, max: usize) -> Outcome {
    let (left, right) = match (
        RecordingRef::decode(baseline),
        RecordingRef::decode(candidate),
    ) {
        (Ok(left), Ok(right)) => (left, right),
        (left, right) => {
            let mut report = String::new();
            for (label, result) in [("baseline", left), ("candidate", right)] {
                if let Err(error) = result {
                    let _ = writeln!(report, "{label}: not a readable recording: {error}");
                }
            }
            return Outcome {
                verdict: None,
                report,
            };
        }
    };

    let mut report = String::new();
    let dictionaries = (Dictionary::resolve(&left), Dictionary::resolve(&right));
    for (label, recording, dictionary) in [
        ("baseline ", &left, &dictionaries.0),
        ("candidate", &right, &dictionaries.1),
    ] {
        let _ = writeln!(report, "{label}  {}", describe(recording, dictionary));
    }
    let _ = writeln!(report);

    // Refused before parsing rather than after: the frames would be read with
    // the wrong table and every difference blamed on an engine.
    if !dictionaries.0.is_available() || !dictionaries.1.is_available() {
        let verdict = Verdict::Incomparable(Incomparable::UnresolvableDictionary);
        let _ = writeln!(report, "verdict: {}", render_verdict(verdict, profile));
        return Outcome {
            verdict: Some(verdict),
            report,
        };
    }

    let (left_envelopes, right_envelopes): (Vec<&[u8]>, Vec<&[u8]>) =
        (left.envelopes().collect(), right.envelopes().collect());
    let left_recording = Recording::new(adapter_info(&left), &left_envelopes)
        .with_comparability(Comparability::of(&left));
    let right_recording = Recording::new(adapter_info(&right), &right_envelopes)
        .with_comparability(Comparability::of(&right));

    let comparison = compare(&left_recording, &right_recording, Tables::shared(table()));
    let verdict = comparison.evaluate(profile);

    let _ = writeln!(
        report,
        "verdict: {}, {} stanza(s) compared",
        render_verdict(verdict, profile),
        comparison.compared()
    );

    section(
        &mut report,
        "failures",
        comparison.failures(profile).map(ToString::to_string),
        max,
    );
    section(
        &mut report,
        "improvements",
        comparison.improvements(profile).map(ToString::to_string),
        max,
    );
    // Everything recorded that this profile neither fails nor celebrates. Shown
    // because a finding a profile tolerates is still a finding, and a reader
    // deciding whether to trust the verdict wants to see them.
    section(
        &mut report,
        "other findings",
        comparison
            .divergences()
            .filter(|d| !profile.is_failure(d) && !profile.is_improvement(d))
            .map(ToString::to_string),
        max,
    );

    Outcome {
        verdict: Some(verdict),
        report,
    }
}

fn render_verdict(verdict: Verdict, profile: ComparisonProfile) -> String {
    match verdict {
        Verdict::Pass => format!("PASS under {profile}"),
        Verdict::Fail => format!("FAIL under {profile}"),
        Verdict::Incomparable(reason) => format!("INCOMPARABLE ({reason})"),
    }
}

fn describe(recording: &RecordingRef<'_>, dictionary: &Dictionary) -> String {
    let adapter = recording.adapter();
    let id = adapter.map_or("unknown adapter", |a| a.id);
    let version = adapter.map_or("?", |a| a.version);
    let engine = adapter.map_or("?", |a| a.engine_version);
    let class = recording
        .artifact_class()
        .map_or_else(|| "unclassified".to_owned(), |c| c.name().to_owned());

    format!(
        "{id} {version} · engine {engine} · {class} · {} stanza(s) · dictionary: {}",
        recording.envelope_count(),
        dictionary.describe()
    )
}

/// Rebuild the adapter's declaration from what the recording carries.
///
/// A capability identifier this build does not know is dropped rather than
/// refused: the set is used for reporting, and a newer adapter naming a
/// capability we have no constant for is expected, not an error. Comparability
/// is decided by the critical tags, not by this.
fn adapter_info<'a>(recording: &RecordingRef<'a>) -> AdapterInfo<'a> {
    let Some(meta) = recording.adapter() else {
        return AdapterInfo::new("unknown", "?", "?", CapabilitySet::NONE);
    };
    let capabilities = meta
        .capabilities
        .iter()
        .filter_map(Capability::from_identifier)
        .fold(CapabilitySet::NONE, CapabilitySet::with);

    let info = AdapterInfo::new(meta.id, meta.version, meta.engine_version, capabilities);
    match recording.provenance() {
        Some(provenance) => info.with_provenance(provenance),
        None => info,
    }
}

/// Print a section, saying what it dropped rather than trimming in silence.
fn section<I>(report: &mut String, title: &str, entries: I, max: usize)
where
    I: Iterator<Item = String>,
{
    let entries: Vec<String> = entries.collect();
    let _ = writeln!(report, "\n{title} ({}):", entries.len());

    let shown = if max == 0 { entries.len() } else { max };
    for entry in entries.iter().take(shown) {
        let _ = writeln!(report, "  {entry}");
    }
    if let Some(hidden) = entries.len().checked_sub(shown).filter(|n| *n > 0) {
        let _ = writeln!(report, "  … and {hidden} more (raise --max-findings)");
    }
}

fn table() -> TokenTable<'static> {
    wa_wire_codec::tokens::TABLE
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
