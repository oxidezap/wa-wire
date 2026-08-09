//! What the inspector says, without a filesystem.
//!
//! The cases that matter are the ones a healthy recording never reaches. A
//! reader opens a `.wawr` file precisely when something is wrong with it, so a
//! damaged trailer, a dictionary this build does not carry and a capability it
//! has no constant for each get a test of their own.

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
    wa_wire_adapter::RawStanza::inbound(builder.build().bytes())
        .encode_to_vec()
        .expect("encodes")
}

fn outbound_envelope(builder: FixtureBuilder) -> Vec<u8> {
    wa_wire_adapter::RawStanza::outbound(builder.build().bytes())
        .encode_to_vec()
        .expect("encodes")
}

/// A `<message>` whose one `<enc>` produced a plaintext.
fn message_with_enc() -> Vec<u8> {
    let fixture = Fixture::node("message")
        .attr("id", "M1")
        .jid_attr("from", "5511999998888")
        .child(Fixture::node("enc").attr("type", "msg").bytes(&[1, 2, 3]))
        .build();

    let mut at = wa_wire_adapter::NodePathBuf::new();
    at.push(0).expect("path");
    let plaintexts = [wa_wire_adapter::Plaintext::ok(at.as_path(), b"\x0a\x02hi")];

    wa_wire_adapter::RawStanza::inbound(fixture.bytes())
        .with_plaintexts(&plaintexts)
        .encode_to_vec()
        .expect("encodes")
}

fn describe(bytes: &[u8]) -> String {
    inspect(bytes, Detail::Summary).text
}

fn with_envelopes(bytes: &[u8]) -> String {
    inspect(bytes, Detail::Envelopes).text
}

/// A recording carrying whatever the test needs it to carry.
fn built(capabilities: &[&str], dictionary: Option<&str>, stanzas: &[Vec<u8>]) -> Vec<u8> {
    let mut meta = MetaBuilder::new()
        .adapter("zapo", "0.1.0", "1.7", 1, capabilities.iter().copied())
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class");
    if let Some(name) = dictionary {
        meta = meta.dictionary(name).expect("dictionary");
    }

    let mut writer = RecordingWriter::new(meta).expect("writer");
    for stanza in stanzas {
        writer.envelope(stanza).expect("envelope");
    }
    writer.finish()
}

#[test]
fn a_healthy_recording_reports_what_it_carries() {
    let bytes = built(
        &["l0.inbound.tap", "l0.plaintext"],
        None,
        &[envelope(receipt("R1")), outbound_envelope(receipt("R2"))],
    );

    let text = describe(&bytes);
    assert!(text.contains("zapo 0.1.0 · engine 1.7"), "{text}");
    assert!(text.contains("2 · 1 inbound, 1 outbound"), "{text}");
    assert!(text.contains("complete"), "{text}");
    assert!(
        text.contains("l0.inbound.tap, l0.plaintext"),
        "capabilities in the order written: {text}"
    );
    assert_eq!(inspect(&bytes, Detail::Summary).exit_code(), exit::PASS);
}

#[test]
fn something_that_is_not_a_recording_is_refused_rather_than_described() {
    let report = inspect(b"not a recording at all", Detail::Summary);

    assert!(!report.readable, "{}", report.text);
    assert_eq!(report.exit_code(), exit::INPUT);
    assert!(
        report.text.starts_with("not a recording:"),
        "{}",
        report.text
    );
}

#[test]
fn a_truncated_recording_is_still_described() {
    // The case the container was specified to survive (D-076): a crash
    // recorder's most valuable artifact is the one that was cut short, so
    // refusing to open it would refuse exactly the file worth opening.
    let whole = built(&["l0.inbound.tap"], None, &[envelope(receipt("R1"))]);
    let cut = &whole[..whole.len() - 4];

    let text = describe(cut);
    assert!(text.contains("TRUNCATED"), "{text}");
    assert!(
        inspect(cut, Detail::Summary).readable,
        "a cut recording still reads: {text}"
    );
}

#[test]
fn bytes_after_the_trailer_are_named_rather_than_ignored() {
    // The checksum covers everything the trailer knew about, so an appended
    // record reads as a complete recording with traffic silently left out.
    let mut bytes = built(&["l0.inbound.tap"], None, &[envelope(receipt("R1"))]);
    bytes.extend_from_slice(b"appended");

    let text = describe(&bytes);
    assert!(text.contains("TRAILING BYTES"), "{text}");
    assert!(text.contains("8 byte(s)"), "{text}");
}

#[test]
fn a_dictionary_this_build_does_not_carry_is_said_so() {
    let bytes = built(
        &["l0.inbound.tap"],
        Some("sha256:some-other-table"),
        &[envelope(receipt("R1"))],
    );

    let text = describe(&bytes);
    assert!(text.contains("not available here"), "{text}");
}

#[test]
fn without_the_dictionary_a_stanza_is_bytes_rather_than_a_guess() {
    // Decoding against the wrong table would print a tag, and a tag printed
    // from the wrong dictionary looks exactly like a fact.
    let bytes = built(
        &["l0.inbound.tap"],
        Some("sha256:some-other-table"),
        &[envelope(receipt("R1"))],
    );

    let text = with_envelopes(&bytes);
    assert!(text.contains("no dictionary"), "{text}");
    assert!(!text.contains("<receipt"), "{text}");
}

#[test]
fn a_capability_this_build_has_no_constant_for_is_still_reported() {
    // A recording from a newer adapter. Dropping the name would make it look
    // like it claimed less than it did.
    let bytes = built(
        &["l0.inbound.tap", "l0.something.newer"],
        None,
        &[envelope(receipt("R1"))],
    );

    let text = describe(&bytes);
    assert!(text.contains("l0.something.newer"), "{text}");
    assert!(
        text.contains("unknown to this build: l0.something.newer"),
        "and marked as one we cannot resolve: {text}"
    );
}

#[test]
fn envelopes_are_listed_with_their_direction_and_stanza() {
    let bytes = built(
        &["l0.inbound.tap"],
        None,
        &[envelope(receipt("R1")), outbound_envelope(receipt("R2"))],
    );

    let text = with_envelopes(&bytes);
    assert!(text.contains("#0   in "), "{text}");
    assert!(text.contains("<receipt id=R1>"), "{text}");
    assert!(text.contains("#1   out"), "{text}");
    assert!(text.contains("<receipt id=R2>"), "{text}");
}

#[test]
fn plaintext_statuses_are_counted_and_listed() {
    let bytes = built(&["l0.plaintext"], None, &[message_with_enc()]);

    let summary = describe(&bytes);
    assert!(summary.contains("plaintexts"), "{summary}");

    let listed = with_envelopes(&bytes);
    assert!(listed.contains("plaintext(s):"), "{listed}");
}

#[test]
fn a_summary_does_not_list_envelopes() {
    let bytes = built(&["l0.inbound.tap"], None, &[envelope(receipt("R1"))]);

    assert!(!describe(&bytes).contains("#0"), "summary stays a summary");
    assert!(with_envelopes(&bytes).contains("#0"));
}

#[test]
fn a_digest_that_is_text_is_printed_as_text() {
    let meta = MetaBuilder::new()
        .adapter("zapo", "0.1.0", "1.7", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .input_digest(b"cross-language-fixture")
        .expect("input");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    writer.envelope(&envelope(receipt("R1"))).expect("envelope");
    let bytes = writer.finish();

    let text = describe(&bytes);
    assert!(text.contains("cross-language-fixture"), "{text}");
}

#[test]
fn a_digest_that_is_not_text_is_printed_as_hex() {
    let meta = MetaBuilder::new()
        .adapter("zapo", "0.1.0", "1.7", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .input_digest(&[0x00, 0xff, 0x10])
        .expect("input");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    writer.envelope(&envelope(receipt("R1"))).expect("envelope");
    let bytes = writer.finish();

    let text = describe(&bytes);
    assert!(text.contains("00ff10"), "{text}");
}

#[test]
fn the_arguments_are_read() {
    let cli = InspectCli::parse(["r.wawr"]).expect("parses");
    assert_eq!(cli.path, "r.wawr");
    assert_eq!(cli.detail, Detail::Summary);

    let cli = InspectCli::parse(["--envelopes", "r.wawr"]).expect("parses");
    assert_eq!(cli.detail, Detail::Envelopes);

    // Options after the path, the way the gate accepts them too.
    let cli = InspectCli::parse(["r.wawr", "--envelopes"]).expect("parses");
    assert_eq!(cli.detail, Detail::Envelopes);
}

#[test]
fn the_arguments_are_refused_when_they_make_no_sense() {
    assert!(matches!(
        InspectCli::parse(["-h"]),
        Err(UsageError::HelpRequested)
    ));
    assert!(matches!(
        InspectCli::parse(Vec::<String>::new()),
        Err(UsageError::MissingPaths)
    ));
    assert!(matches!(
        InspectCli::parse(["a.wawr", "b.wawr"]),
        Err(UsageError::TooManyPaths)
    ));
    assert!(matches!(
        InspectCli::parse(["--nope", "a.wawr"]),
        Err(UsageError::UnknownFlag(_))
    ));
}
