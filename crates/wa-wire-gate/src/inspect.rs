//! Say what is inside one recording.
//!
//! The boundary format is published and frozen, and until this existed the only
//! way to look at a `.wawr` file was to write Rust against
//! [`wa_wire_recording`]. A format nobody can open is a format nobody can
//! check, which is the same argument that put the gate beside the comparator.
//!
//! What it reports is what the file *says*, including where that disagrees with
//! itself. A damaged trailer, a dictionary this build does not carry, a
//! capability identifier from a newer adapter: each is printed rather than
//! resolved, because a reader that quietly normalised them would hide exactly
//! the thing someone opens a recording to find.

use std::fmt::Write as _;

use wa_wire_codec::Parser;
use wa_wire_contract::{Direction, EnvelopeRef, FrameOrigin, PlaintextStatus};
use wa_wire_recording::{ArtifactClass, Integrity, ReadError, RecordingRef, Tag};

use crate::{Dictionary, UsageError, content_counts, direction_counts, exit, table};

/// How much of the envelope list to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Totals only.
    Summary,
    /// Every envelope, one line each.
    Envelopes,
}

/// What `inspect` produced.
#[derive(Debug)]
pub struct Report {
    /// The rendered report, ready to print.
    pub text: String,
    /// Whether the recording could be read at all.
    pub readable: bool,
}

impl Report {
    /// The process exit code this report implies.
    ///
    /// A recording that decodes exits zero even when its trailer disagrees with
    /// its records: the tool was asked what the file contains and it answered.
    /// The disagreement is in the answer, not in the asking.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        if self.readable {
            exit::PASS
        } else {
            exit::INPUT
        }
    }
}

/// Describe one recording.
///
/// Takes bytes rather than a path so the whole rendering is testable without a
/// filesystem, the same trade [`crate::run`] makes.
#[must_use]
pub fn inspect(bytes: &[u8], detail: Detail) -> Report {
    let recording = match RecordingRef::decode(bytes) {
        Ok(recording) => recording,
        Err(error) => {
            return Report {
                text: format!("not a recording: {}\n", describe_read_error(&error)),
                readable: false,
            };
        }
    };

    let dictionary = Dictionary::resolve(&recording);
    let mut text = String::new();

    render_identity(&mut text, &recording, &dictionary);
    render_integrity(&mut text, &recording);
    render_counts(&mut text, &recording);
    render_capabilities(&mut text, &recording);

    if detail == Detail::Envelopes {
        render_envelopes(&mut text, &recording, &dictionary);
    }

    Report {
        text,
        readable: true,
    }
}

fn field(text: &mut String, name: &str, value: &str) {
    // Ignoring the error: writing to a String cannot fail, and the alternative
    // is an unwrap on a branch no test can reach.
    let _ = writeln!(text, "  {name:<14}{value}");
}

fn render_identity(text: &mut String, recording: &RecordingRef<'_>, dictionary: &Dictionary) {
    match recording.adapter() {
        Some(meta) => {
            field(
                text,
                "adapter",
                &escaped(&format!(
                    "{} {} · engine {}",
                    meta.id, meta.version, meta.engine_version
                )),
            );
            field(text, "contract", &format!("v{}", meta.contract_version));
        }
        // `adapter` is a critical tag, so absent and unreadable are separate
        // findings and only one of them is a corrupt file. The container
        // decodes either way, which is why the raw tag has to be looked for
        // rather than inferred from `adapter()` returning nothing.
        None if recording.value(Tag::ADAPTER).is_some() => field(
            text,
            "adapter",
            "MALFORMED — the tag is present and its payload does not parse",
        ),
        None => field(text, "adapter", "undeclared"),
    }

    field(
        text,
        "class",
        recording
            .artifact_class()
            .map_or("unclassified", ArtifactClass::name),
    );
    field(text, "dictionary", &escaped(&dictionary.describe()));

    if let Some(provenance) = recording.provenance() {
        field(
            text,
            "spec",
            &escaped(&format!(
                "WhatsApp {} · manifest {} · generator {}",
                provenance.whatsapp_version, provenance.manifest_hash, provenance.generator_version
            )),
        );
    }

    if let Some((from, to)) = recording.transform() {
        field(text, "transform", &escaped(&format!("{from} → {to}")));
    }
    if let Some(digest) = recording.input_digest() {
        field(text, "input digest", &escaped(&readable_digest(digest)));
    }
    if let Some(at) = recording.created_at() {
        // The number the file carries, labelled with the unit the format
        // defines. Rendering it as a date needs a calendar this crate does not
        // have, and would have to pick a timezone the recording never named.
        field(text, "created", &format!("{at} ms since the Unix epoch"));
    }
    if let Some(note) = recording.note() {
        field(text, "note", &escaped(note));
    }
}

fn render_integrity(text: &mut String, recording: &RecordingRef<'_>) {
    let described = match recording.integrity() {
        Integrity::Complete => "complete — count and checksum both hold".to_owned(),
        Integrity::Damaged {
            claimed,
            found,
            checksum_ok,
        } => format!(
            "DAMAGED — trailer claims {claimed}, found {found}; checksum {}",
            if checksum_ok {
                "holds"
            } else {
                "does not hold"
            }
        ),
        Integrity::TrailingBytes { found, trailing } => format!(
            "TRAILING BYTES — {found} records accounted for, {trailing} byte(s) follow the trailer \
             and are not read"
        ),
        Integrity::Truncated { found, dangling } => format!(
            "TRUNCATED — no trailer; {found} record(s) read, {dangling} byte(s) left dangling"
        ),
    };
    field(text, "integrity", &described);
}

fn render_counts(text: &mut String, recording: &RecordingRef<'_>) {
    let (inbound, outbound) = direction_counts(recording);
    field(
        text,
        "envelopes",
        &format!(
            "{} · {inbound} inbound, {outbound} outbound",
            recording.envelope_count()
        ),
    );

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut unsupported = 0usize;
    let mut unobserved = 0usize;
    let mut undecodable = 0usize;
    for bytes in recording.envelopes() {
        let Ok(envelope) = EnvelopeRef::decode(bytes) else {
            undecodable = undecodable.saturating_add(1);
            continue;
        };
        for entry in envelope.entries() {
            let counter = match entry.status {
                PlaintextStatus::Ok => &mut ok,
                PlaintextStatus::DecryptFailed => &mut failed,
                PlaintextStatus::Unsupported => &mut unsupported,
                PlaintextStatus::Unobserved => &mut unobserved,
            };
            *counter = counter.saturating_add(1);
        }
    }

    let total = ok
        .saturating_add(failed)
        .saturating_add(unsupported)
        .saturating_add(unobserved);
    if total > 0 {
        field(
            text,
            "plaintexts",
            &format!(
                "{total} · {ok} ok, {failed} decrypt-failed, {unsupported} unsupported, \
                 {unobserved} unobserved"
            ),
        );
    }
    if undecodable > 0 {
        field(
            text,
            "unreadable",
            &format!("{undecodable} envelope(s) did not decode"),
        );
    }

    let skipped = recording.skipped_records();
    if skipped > 0 {
        field(
            text,
            "skipped",
            &format!(
                "{skipped} record(s) of a kind this build does not read — the file holds more \
                 than this report describes"
            ),
        );
    }

    let unknown_tags = recording.unknown_critical_tags();
    if unknown_tags > 0 {
        field(
            text,
            "unknown meta",
            &format!("{unknown_tags} critical tag(s) this build cannot read"),
        );
    }

    let counts = content_counts(recording);
    if !counts.is_empty() {
        let rendered: Vec<String> = counts
            .iter()
            .map(|(kind, seen)| format!("{kind} {seen}"))
            .collect();
        field(text, "content", &rendered.join(", "));
    }
}

fn render_capabilities(text: &mut String, recording: &RecordingRef<'_>) {
    let Some(meta) = recording.adapter() else {
        return;
    };

    // Every identifier the file declares, including ones this build has no
    // constant for. Dropping those would make a recording from a newer adapter
    // look like one that claimed less than it did.
    let declared: Vec<&str> = meta.capabilities.iter().collect();
    if declared.is_empty() {
        field(text, "capabilities", "none declared");
        return;
    }

    field(text, "capabilities", &escaped(&declared.join(", ")));

    let unknown: Vec<&str> = declared
        .iter()
        .copied()
        .filter(|name| wa_wire_contract::Capability::from_identifier(name).is_none())
        .collect();
    if !unknown.is_empty() {
        field(
            text,
            "",
            &escaped(&format!("unknown to this build: {}", unknown.join(", "))),
        );
    }
}

fn render_envelopes(text: &mut String, recording: &RecordingRef<'_>, dictionary: &Dictionary) {
    let _ = writeln!(text);
    let parser = dictionary.is_available().then(|| Parser::new(table()));

    for (index, bytes) in recording.envelopes().enumerate() {
        let Ok(envelope) = EnvelopeRef::decode(bytes) else {
            let _ = writeln!(text, "  #{index:<4}did not decode");
            continue;
        };

        let flags = envelope.flags();
        let direction = match flags.direction {
            Direction::Inbound => "in ",
            Direction::Outbound => "out",
        };
        let origin = match flags.frame_origin {
            FrameOrigin::Original => "verbatim",
            FrameOrigin::ReEncoded => "re-encoded",
        };

        let line = format!(
            "  #{index:<4}{direction}  {origin:<11}{:<34}{}",
            stanza_label(parser.as_ref(), envelope.frame()),
            plaintext_label(&envelope)
        );
        let _ = writeln!(text, "{}", line.trim_end());
    }
}

/// A stanza's tag and id, or why neither could be read.
fn stanza_label(parser: Option<&Parser<'_>>, frame: &[u8]) -> String {
    let Some(parser) = parser else {
        // Without the dictionary the frame is bytes. Saying so beats printing a
        // tag decoded against the wrong table, which would look like a fact.
        return format!("<{} bytes, no dictionary>", frame.len());
    };

    match parser.parse(frame) {
        Ok(node) => {
            let tag = escaped(node.tag().as_str().unwrap_or("?"));
            match node.attr("id").and_then(wa_wire_codec::Value::as_str) {
                Some(id) => format!("<{tag} id={}>", escaped(id)),
                None => format!("<{tag}>"),
            }
        }
        Err(_) => format!("<unparsed, {} bytes>", frame.len()),
    }
}

fn plaintext_label(envelope: &EnvelopeRef<'_>) -> String {
    let statuses: Vec<&str> = envelope
        .entries()
        .map(|entry| match entry.status {
            PlaintextStatus::Ok => "ok",
            PlaintextStatus::DecryptFailed => "decrypt-failed",
            PlaintextStatus::Unsupported => "unsupported",
            PlaintextStatus::Unobserved => "unobserved",
        })
        .collect();

    if statuses.is_empty() {
        String::new()
    } else {
        format!("{} plaintext(s): {}", statuses.len(), statuses.join(", "))
    }
}

/// A digest as text when it is text, and as hex when it is not.
///
/// The tag holds bytes, and writers use both: a hash, or a label naming the
/// input. Rendering a label as hex hides what it says, and rendering a hash as
/// lossy text would invent characters.
fn readable_digest(bytes: &[u8]) -> String {
    match core::str::from_utf8(bytes) {
        Ok(text) if text.chars().all(|c| c.is_ascii_graphic() || c == ' ') => text.to_owned(),
        _ => hex(bytes),
    }
}

/// Render a string from the recording so it cannot forge the report.
///
/// Everything printed here comes out of a file this tool exists to inspect, so
/// the file is not trusted. A stanza id carrying a newline would forge a line
/// of the report; one carrying `ESC [` would send the terminal a control
/// sequence, which is a command rather than a character. Both are valid UTF-8
/// and neither survives this.
///
/// Escapes C0, DEL and C1 — the ranges a terminal acts on rather than draws —
/// and leaves ordinary text, including non-Latin scripts, exactly as it is.
fn escaped(value: &str) -> String {
    if !value.chars().any(needs_escaping) {
        // The overwhelming case, and worth not allocating for.
        return value.to_owned();
    }

    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if needs_escaping(character) {
            let _ = write!(out, "\\u{{{:x}}}", character as u32);
        } else {
            out.push(character);
        }
    }
    out
}

fn needs_escaping(character: char) -> bool {
    character.is_control() || matches!(character, '\u{7f}'..='\u{9f}')
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn describe_read_error(error: &ReadError) -> String {
    format!("{error}")
}

/// What to print when asked, and when the arguments make no sense.
pub const INSPECT_USAGE: &str = "\
wa-wire-inspect — say what is inside one recording

    wa-wire-inspect [options] <recording.wawr>

Options:
    --envelopes    list every envelope, one line each
    -h, --help     this

Exit codes:
    0  the recording was read    64  bad arguments
                                 66  it could not be read
";

/// The inspect command's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectCli {
    /// The recording to describe.
    pub path: String,
    /// How much to print.
    pub detail: Detail,
}

impl InspectCli {
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
        let mut detail = Detail::Summary;

        for arg in args {
            match arg.as_ref() {
                "-h" | "--help" => return Err(UsageError::HelpRequested),
                "--envelopes" => detail = Detail::Envelopes,
                flag if flag.starts_with('-') && flag.len() > 1 => {
                    return Err(UsageError::UnknownFlag(flag.to_owned()));
                }
                path => paths.push(path.to_owned()),
            }
        }

        let mut paths = paths.into_iter();
        let Some(path) = paths.next() else {
            return Err(UsageError::MissingPaths);
        };
        if paths.next().is_some() {
            return Err(UsageError::TooManyPaths);
        }

        Ok(Self { path, detail })
    }
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
