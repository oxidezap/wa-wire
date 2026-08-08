//! What the gate decides, without a filesystem.

use super::*;

use wa_wire_l1::testing::{Fixture, FixtureBuilder};
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingWriter};

fn receipt(id: &str) -> FixtureBuilder {
    Fixture::node("receipt")
        .attr("id", id)
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
}

fn envelope(builder: FixtureBuilder) -> Vec<u8> {
    let fixture = builder.build();
    wa_wire_adapter::RawStanza::inbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes")
}

/// A recording of `stanzas`, declaring `input` as the traffic it replays.
fn recording(adapter: &str, input: &[u8], stanzas: &[Vec<u8>]) -> Vec<u8> {
    recording_with(adapter, input, stanzas, None, false)
}

fn recording_with(
    adapter: &str,
    input: &[u8],
    stanzas: &[Vec<u8>],
    dictionary: Option<&str>,
    re_encoded: bool,
) -> Vec<u8> {
    let mut meta = MetaBuilder::new()
        .adapter(adapter, "0.1.0", "1.0", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class")
        .input_digest(input)
        .expect("input");
    if let Some(name) = dictionary {
        meta = meta.dictionary(name).expect("dictionary");
    }

    let mut writer = RecordingWriter::new(meta).expect("writer");
    for stanza in stanzas {
        let bytes = if re_encoded {
            // Same frame, described as re-encoded rather than verbatim.
            let decoded = wa_wire_contract::EnvelopeRef::decode(stanza).expect("decodes");
            wa_wire_adapter::RawStanza::inbound(decoded.frame())
                .re_encoded()
                .encode_to_vec()
                .expect("encodes")
        } else {
            stanza.clone()
        };
        writer.envelope(&bytes).expect("envelope");
    }
    writer.finish()
}

fn gate(baseline: &[u8], candidate: &[u8], profile: ComparisonProfile) -> Outcome {
    run(baseline, candidate, profile, DEFAULT_MAX_FINDINGS)
}

// -- arguments --------------------------------------------------------------

#[test]
fn the_defaults_are_the_ones_a_gate_wants() {
    let cli = Cli::parse(["a.wawr", "b.wawr"]).expect("parses");
    assert_eq!(cli.baseline, "a.wawr");
    assert_eq!(cli.candidate, "b.wawr");
    assert_eq!(
        cli.profile,
        ComparisonProfile::Regression,
        "the tool exists to answer the regression question"
    );
    assert_eq!(cli.max_findings, DEFAULT_MAX_FINDINGS);
}

#[test]
fn every_option_is_read() {
    let cli =
        Cli::parse(["--profile", "interop", "--max-findings", "3", "a", "b"]).expect("parses");
    assert_eq!(cli.profile, ComparisonProfile::Interop);
    assert_eq!(cli.max_findings, 3);

    let regression = Cli::parse(["--profile", "regression", "a", "b"]).expect("parses");
    assert_eq!(regression.profile, ComparisonProfile::Regression);
}

#[test]
fn options_may_follow_the_paths() {
    let cli = Cli::parse(["a", "b", "--profile", "interop"]).expect("parses");
    assert_eq!(
        (cli.baseline.as_str(), cli.profile),
        ("a", ComparisonProfile::Interop)
    );
}

#[test]
fn every_way_the_arguments_can_be_wrong_is_named() {
    assert_eq!(
        Cli::parse(["-h"]).expect_err("help"),
        UsageError::HelpRequested
    );
    assert_eq!(
        Cli::parse(["--help"]).expect_err("help"),
        UsageError::HelpRequested
    );
    assert_eq!(
        Cli::parse(["--profile"]).expect_err("no value"),
        UsageError::MissingValue("--profile".to_owned())
    );
    assert_eq!(
        Cli::parse(["--max-findings"]).expect_err("no value"),
        UsageError::MissingValue("--max-findings".to_owned())
    );
    assert_eq!(
        Cli::parse(["--profile", "sideways", "a", "b"]).expect_err("bad value"),
        UsageError::BadValue {
            flag: "--profile".to_owned(),
            value: "sideways".to_owned()
        }
    );
    assert_eq!(
        Cli::parse(["--max-findings", "many", "a", "b"]).expect_err("bad value"),
        UsageError::BadValue {
            flag: "--max-findings".to_owned(),
            value: "many".to_owned()
        }
    );
    assert_eq!(
        Cli::parse(["--colour", "a", "b"]).expect_err("unknown"),
        UsageError::UnknownFlag("--colour".to_owned())
    );
    assert_eq!(
        Cli::parse(["only-one"]).expect_err("too few"),
        UsageError::MissingPaths
    );
    assert_eq!(
        Cli::parse(["a", "b", "c"]).expect_err("too many"),
        UsageError::TooManyPaths
    );
}

#[test]
fn every_usage_error_renders_something_a_reader_can_act_on() {
    let errors = [
        UsageError::HelpRequested,
        UsageError::MissingValue("--profile".to_owned()),
        UsageError::BadValue {
            flag: "--profile".to_owned(),
            value: "x".to_owned(),
        },
        UsageError::UnknownFlag("--x".to_owned()),
        UsageError::MissingPaths,
        UsageError::TooManyPaths,
    ];
    for error in &errors {
        assert!(!error.to_string().is_empty(), "{error:?}");
    }
    assert!(
        UsageError::HelpRequested
            .to_string()
            .contains("wa-wire-gate")
    );
    assert!(
        UsageError::MissingPaths.to_string().contains("baseline"),
        "it has to say what was missing"
    );
    assert_error(&UsageError::MissingPaths);
}

fn assert_error<E: std::error::Error>(_: &E) {}

#[test]
fn the_usage_text_documents_every_exit_code() {
    for code in ["0", "1", "2", "64", "66"] {
        assert!(USAGE.contains(code), "usage must document exit {code}");
    }
    for name in ["interop", "regression", "--max-findings"] {
        assert!(USAGE.contains(name), "usage must document {name}");
    }
}

// -- verdicts ---------------------------------------------------------------

#[test]
fn two_identical_recordings_pass_under_both_profiles() {
    let stanzas = [envelope(receipt("A")), envelope(receipt("B"))];
    let bytes = recording("engine", b"monday", &stanzas);

    for profile in [ComparisonProfile::Interop, ComparisonProfile::Regression] {
        let outcome = gate(&bytes, &bytes, profile);
        assert_eq!(outcome.verdict, Some(Verdict::Pass), "{profile}");
        assert_eq!(outcome.exit_code(), exit::PASS);
        assert!(outcome.report.contains("PASS"), "{}", outcome.report);
        assert!(outcome.report.contains("2 stanza(s) compared"));
    }
}

#[test]
fn a_candidate_that_lost_a_stanza_fails() {
    let all = [envelope(receipt("A")), envelope(receipt("B"))];
    let fewer = [envelope(receipt("A"))];
    let baseline = recording("engine", b"monday", &all);
    let candidate = recording("engine", b"monday", &fewer);

    let outcome = gate(&baseline, &candidate, ComparisonProfile::Regression);
    assert_eq!(outcome.verdict, Some(Verdict::Fail));
    assert_eq!(outcome.exit_code(), exit::FAIL);
    assert!(
        outcome.report.contains("failures (1)"),
        "{}",
        outcome.report
    );
    assert!(outcome.report.contains("recordings differ in length"));
}

#[test]
fn the_profile_decides_and_the_report_says_which_one_answered() {
    // The same pair, judged twice. One adapter hands over its engine's buffer
    // and the other re-encodes: expected between engines, a loss between two
    // builds of one.
    let stanzas = [envelope(receipt("A"))];
    let baseline = recording("engine", b"monday", &stanzas);
    let candidate = recording_with("engine", b"monday", &stanzas, None, true);

    let interop = gate(&baseline, &candidate, ComparisonProfile::Interop);
    assert_eq!(interop.verdict, Some(Verdict::Pass));
    assert!(interop.report.contains("PASS under interop"));
    assert!(
        interop.report.contains("other findings (1)"),
        "tolerated is not the same as unreported:\n{}",
        interop.report
    );

    let regression = gate(&baseline, &candidate, ComparisonProfile::Regression);
    assert_eq!(regression.verdict, Some(Verdict::Fail));
    assert!(regression.report.contains("FAIL under regression"));
    assert!(regression.report.contains("frame origins differ"));
}

#[test]
fn two_recordings_of_different_traffic_are_incomparable_rather_than_failing() {
    // Not a fail: nothing about the engines was established. A gate that
    // reported this as a regression would send someone looking for a bug that
    // is not there.
    let stanzas = [envelope(receipt("A"))];
    let monday = recording("engine", b"monday", &stanzas);
    let tuesday = recording("engine", b"tuesday", &stanzas);

    let outcome = gate(&monday, &tuesday, ComparisonProfile::Regression);
    assert_eq!(
        outcome.verdict,
        Some(Verdict::Incomparable(Incomparable::DifferentInput))
    );
    assert_eq!(outcome.exit_code(), exit::INCOMPARABLE);
    assert_ne!(
        outcome.exit_code(),
        exit::PASS,
        "incomparable is not a pass"
    );
    assert!(outcome.report.contains("INCOMPARABLE"));
    assert!(outcome.report.contains("different input traffic"));
}

#[test]
fn a_truncated_recording_is_read_and_still_refused() {
    // Readable, so its header still describes it; not whole, so no verdict
    // about it would mean anything.
    let stanzas = [envelope(receipt("A"))];
    let whole = recording("engine", b"monday", &stanzas);
    let frozen = &whole[..whole.len() - 13];

    let outcome = gate(&whole, frozen, ComparisonProfile::Regression);
    assert_eq!(
        outcome.verdict,
        Some(Verdict::Incomparable(Incomparable::NotWhole))
    );
    assert!(
        outcome.report.contains("1 stanza(s)"),
        "the truncated side still describes itself:\n{}",
        outcome.report
    );
}

// -- the dictionary ---------------------------------------------------------

#[test]
fn a_recording_naming_the_bundled_dictionary_is_compared() {
    let stanzas = [envelope(receipt("A"))];
    let bytes = recording_with(
        "engine",
        b"monday",
        &stanzas,
        Some(wa_wire_codec::tokens::SOURCE_DIGEST),
        false,
    );

    let outcome = gate(&bytes, &bytes, ComparisonProfile::Regression);
    assert_eq!(outcome.verdict, Some(Verdict::Pass));
    assert!(
        outcome.report.contains("dictionary: bundled"),
        "{}",
        outcome.report
    );
}

#[test]
fn a_dictionary_this_build_does_not_have_stops_the_comparison() {
    // Parsing those frames with the bundled table would read them wrongly and
    // blame an engine for the result.
    let stanzas = [envelope(receipt("A"))];
    let bytes = recording_with(
        "engine",
        b"monday",
        &stanzas,
        Some("sha256:from-a-newer-whatspec"),
        false,
    );

    let outcome = gate(&bytes, &bytes, ComparisonProfile::Regression);
    assert_eq!(
        outcome.verdict,
        Some(Verdict::Incomparable(Incomparable::UnresolvableDictionary))
    );
    assert!(
        outcome.report.contains("not available here"),
        "{}",
        outcome.report
    );
}

#[test]
fn a_recording_that_declares_no_dictionary_says_what_was_assumed() {
    let stanzas = [envelope(receipt("A"))];
    let bytes = recording("engine", b"monday", &stanzas);

    let outcome = gate(&bytes, &bytes, ComparisonProfile::Regression);
    assert!(
        outcome.report.contains("undeclared, assuming bundled"),
        "an assumption a reader cannot see is an assumption nobody checked:\n{}",
        outcome.report
    );
}

// -- reading ----------------------------------------------------------------

#[test]
fn a_buffer_that_is_not_a_recording_names_which_side_it_was() {
    let good = recording("engine", b"monday", &[envelope(receipt("A"))]);

    let outcome = gate(b"not a recording at all", &good, ComparisonProfile::Interop);
    assert_eq!(outcome.verdict, None);
    assert_eq!(outcome.exit_code(), exit::INPUT);
    assert!(outcome.report.contains("baseline"), "{}", outcome.report);
    assert!(!outcome.report.contains("candidate:"), "{}", outcome.report);

    let other = gate(&good, b"not a recording at all", ComparisonProfile::Interop);
    assert!(other.report.contains("candidate"), "{}", other.report);

    let both = gate(b"nope", b"also nope", ComparisonProfile::Interop);
    assert!(both.report.contains("baseline") && both.report.contains("candidate"));
}

// -- the report -------------------------------------------------------------

#[test]
fn the_header_describes_both_recordings() {
    let stanzas = [envelope(receipt("A"))];
    let baseline = recording("engine-a", b"monday", &stanzas);
    let candidate = recording("engine-b", b"monday", &stanzas);

    let report = gate(&baseline, &candidate, ComparisonProfile::Interop).report;
    assert!(report.contains("engine-a"), "{report}");
    assert!(report.contains("engine-b"), "{report}");
    assert!(report.contains("engine 1.0"), "{report}");
    assert!(report.contains("replayed"), "{report}");
}

#[test]
fn a_recording_with_no_adapter_tag_is_described_rather_than_refused() {
    let meta = MetaBuilder::new()
        .artifact_class(ArtifactClass::Replayed)
        .expect("class")
        .input_digest(b"monday")
        .expect("input");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    writer.envelope(&envelope(receipt("A"))).expect("envelope");
    let bytes = writer.finish();

    let outcome = gate(&bytes, &bytes, ComparisonProfile::Regression);
    assert_eq!(outcome.verdict, Some(Verdict::Pass));
    assert!(
        outcome.report.contains("unknown adapter"),
        "{}",
        outcome.report
    );
}

#[test]
fn a_long_list_of_findings_says_what_it_did_not_print() {
    // A cap nobody is told about reads as "that was all of them".
    let baseline: Vec<Vec<u8>> = (0..8)
        .map(|i| envelope(receipt(&format!("A{i}"))))
        .collect();
    let candidate: Vec<Vec<u8>> = (0..8)
        .map(|i| envelope(receipt(&format!("B{i}"))))
        .collect();
    let left = recording("engine", b"monday", &baseline);
    let right = recording("engine", b"monday", &candidate);

    let capped = run(&left, &right, ComparisonProfile::Regression, 3);
    assert!(
        capped.report.contains("failures (16):"),
        "{}",
        capped.report
    );
    assert!(capped.report.contains("and 13 more"), "{}", capped.report);
    assert!(capped.report.contains("--max-findings"));

    let uncapped = run(&left, &right, ComparisonProfile::Regression, 0);
    assert!(
        !uncapped.report.contains("more (raise"),
        "zero means all of them:\n{}",
        uncapped.report
    );
}

#[test]
fn an_improvement_is_reported_and_does_not_fail_the_run() {
    // A candidate that reaches its engine's own buffer where the baseline
    // re-encoded has improved, and a gate that failed it would be useless.
    let stanzas = [envelope(receipt("A"))];
    let baseline = recording_with("engine", b"monday", &stanzas, None, true);
    let candidate = recording("engine", b"monday", &stanzas);

    let outcome = gate(&baseline, &candidate, ComparisonProfile::Regression);
    assert_eq!(outcome.verdict, Some(Verdict::Pass));
    assert!(
        outcome.report.contains("improvements (1)"),
        "{}",
        outcome.report
    );
}

#[test]
fn a_recording_that_records_its_spec_carries_it_into_the_comparison() {
    // Provenance is what makes an L1 difference answerable: without it, the
    // first question — were these generated from the same spec? — has no
    // answer, and every finding after it is ambiguous.
    let stanzas = [envelope(receipt("A"))];
    let mut sides = Vec::new();
    for manifest in ["sha256:old", "sha256:new"] {
        let meta = MetaBuilder::new()
            .adapter("engine", "0.1.0", "1.0", 1, ["l0.inbound.tap"])
            .expect("adapter")
            .provenance(&wa_wire_contract::Provenance::new(
                "2.3000.1", manifest, "0.1.0",
            ))
            .expect("provenance")
            .artifact_class(ArtifactClass::Replayed)
            .expect("class")
            .input_digest(b"monday")
            .expect("input");
        let mut writer = RecordingWriter::new(meta).expect("writer");
        for stanza in &stanzas {
            writer.envelope(stanza).expect("envelope");
        }
        sides.push(writer.finish());
    }

    let outcome = gate(&sides[0], &sides[1], ComparisonProfile::Regression);
    assert_eq!(
        outcome.verdict,
        Some(Verdict::Pass),
        "a spec difference is a warning, not a failure"
    );
    assert!(
        outcome.report.contains("different specs"),
        "but it must be visible, or every L1 finding after it is ambiguous:\n{}",
        outcome.report
    );
}

#[test]
fn an_outcome_is_debuggable_and_comparable() {
    let stanzas = [envelope(receipt("A"))];
    let bytes = recording("engine", b"monday", &stanzas);
    let one = gate(&bytes, &bytes, ComparisonProfile::Interop);
    let two = gate(&bytes, &bytes, ComparisonProfile::Interop);
    assert_eq!(one, two, "the gate is a pure function of its inputs");
    assert!(!format!("{one:?}").is_empty());
}
