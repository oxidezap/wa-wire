//! Envelopes written by the TypeScript encoder, read by the Rust one.
//!
//! The boundary format is described twice — once in `wa-wire-contract`, once in
//! the `zapo` adapter — because an adapter has to run inside a JavaScript
//! engine. Two descriptions of one format that are only ever tested separately
//! are two formats waiting to diverge.
//!
//! The fixtures are produced by `adapters/zapo/scripts/emit-fixtures.ts` and
//! committed. CI regenerates them and requires no diff, so a change to either
//! encoder shows up here rather than as traffic nobody can read.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;

use wa_wire_codec::{Parser, TokenTable};
use wa_wire_contract::{Direction, EnvelopeRef, FrameOrigin, PlaintextStatus};
use wa_wire_recording::{ArtifactClass, CONTAINER_VERSION, Integrity, RecordingRef, Tag};

/// The `zapo` adapter re-encodes with the real dictionaries, so reading its
/// output needs the real table.
fn table() -> TokenTable<'static> {
    wa_wire_codec::tokens::TABLE
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/zapo/fixtures")
        .join(format!("{name}.bin"));
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun adapters/zapo/scripts/emit-fixtures.ts",
            path.display()
        )
    })
}

#[test]
fn a_receipt_written_in_typescript_decodes_here() {
    let bytes = fixture("receipt");
    let envelope = EnvelopeRef::decode(&bytes).expect("the Rust decoder reads it");

    assert_eq!(envelope.version().get(), 1);
    assert_eq!(envelope.flags().direction, Direction::Inbound);
    assert_eq!(
        envelope.flags().frame_origin,
        FrameOrigin::ReEncoded,
        "zapo's filter sees a decoded node, so it re-encodes"
    );
    assert!(envelope.is_plaintext_empty());

    let node = Parser::new(table())
        .parse(envelope.frame())
        .expect("and the frame parses");
    assert!(node.tag().eq_str("receipt"));
    assert!(node.attr_eq("id", "ABCD1234"));
    assert!(node.attr_eq("type", "read"));
    assert!(node.attr_eq("from", "5511999998888@s.whatsapp.net"));
}

#[test]
fn a_message_written_in_typescript_keeps_its_enc_body() {
    let bytes = fixture("message-with-enc");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let node = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    assert!(node.tag().eq_str("message"));
    assert!(node.attr_eq("t", "1700000000"));

    let enc = node.child("enc").expect("the enc child");
    assert!(enc.attr_eq("v", "2"));
    assert_eq!(enc.content().as_bytes(), Some(&b"ciphertext-bytes"[..]));
}

#[test]
fn direction_and_origin_survive_the_crossing() {
    let bytes = fixture("outbound-verbatim");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert_eq!(envelope.flags().direction, Direction::Outbound);
    assert!(envelope.flags().is_verbatim());
}

#[test]
fn paths_written_in_typescript_address_the_right_nodes() {
    // The claim that would be silently wrong if the two encoders disagreed on
    // byte order: a plaintext ending up on the wrong `<enc>` means a decrypted
    // message attributed to the wrong recipient.
    let bytes = fixture("multi-device-with-plaintexts");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    assert_eq!(envelope.entry_count(), 4);
    assert_eq!(root.children().len(), 4);

    let entries: Vec<_> = envelope.entries().collect();

    let first = root
        .at_path(entries[0].path.iter())
        .expect("addresses a node");
    assert!(first.tag().eq_str("enc"));
    assert!(first.attr_eq("type", "msg"));
    assert_eq!(entries[0].status, PlaintextStatus::Ok);
    assert_eq!(entries[0].payload, b"plain-one");

    let second = root
        .at_path(entries[1].path.iter())
        .expect("addresses a node");
    assert!(second.attr_eq("type", "pkmsg"));
    assert_eq!(entries[1].status, PlaintextStatus::DecryptFailed);
    assert!(
        entries[1].payload.is_empty(),
        "a failed decryption carries no payload but still names its node"
    );

    let third = root
        .at_path(entries[2].path.iter())
        .expect("addresses a node");
    assert!(third.attr_eq("type", "skmsg"));
    assert_eq!(entries[2].status, PlaintextStatus::Unsupported);

    let fourth = root
        .at_path(entries[3].path.iter())
        .expect("addresses a node");
    assert!(fourth.attr_eq("type", "msg"));
    assert_eq!(
        entries[3].status,
        PlaintextStatus::Unobserved,
        "an adapter that never learned why still names the node"
    );
    assert!(entries[3].payload.is_empty());
}

#[test]
fn the_root_path_means_the_same_thing_in_both_languages() {
    let bytes = fixture("root-path-plaintext");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    let entry = envelope.entries().next().expect("one entry");
    assert!(entry.path.is_empty());
    let node = root.at_path(entry.path.iter()).expect("the root itself");
    assert_eq!(node.tag(), root.tag());
    assert_eq!(entry.payload, b"whole");
}

#[test]
fn an_empty_frame_crosses_intact() {
    let bytes = fixture("empty-frame");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert!(envelope.frame().is_empty());
    assert!(envelope.is_plaintext_empty());
    assert_eq!(bytes.len(), 10, "header plus an empty count");
}

#[test]
fn every_fixture_decodes_and_none_has_trailing_bytes() {
    // A trailing byte would mean the two encoders disagree about a length,
    // which is exactly the kind of drift that only shows up cross-language.
    for name in [
        "receipt",
        "message-with-enc",
        "outbound-verbatim",
        "multi-device-with-plaintexts",
        "root-path-plaintext",
        "empty-frame",
    ] {
        let bytes = fixture(name);
        EnvelopeRef::decode(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

// --- the container ---------------------------------------------------------

fn container(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/zapo/fixtures")
        .join(format!("{name}.wawr"));
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun adapters/zapo/scripts/emit-fixtures.ts",
            path.display()
        )
    })
}

#[test]
fn a_recording_written_in_typescript_reads_here() {
    // The container is written twice for the same reason the envelope is: two
    // descriptions of one format that are only ever tested separately are two
    // formats waiting to diverge.
    let bytes = container("recording");
    let recording = RecordingRef::decode(&bytes).expect("the Rust reader reads it");

    assert_eq!(recording.container_version(), CONTAINER_VERSION);
    assert_eq!(
        recording.integrity(),
        Integrity::Complete,
        "the count and the checksum both agree across languages"
    );

    let adapter = recording.adapter().expect("adapter");
    assert_eq!(adapter.id, "zapo");
    assert_eq!(adapter.version, "0.1.0");
    assert_eq!(adapter.engine_version, "1.7");
    assert_eq!(adapter.contract_version, 1);
    assert!(adapter.capabilities.contains("l0.plaintext"));
    assert_eq!(adapter.capabilities.len(), 3);

    let provenance = recording.provenance().expect("provenance");
    assert_eq!(provenance.whatsapp_version, "2.3000.1044659339");
    assert_eq!(provenance.manifest_hash, "sha256:fixture");

    assert_eq!(recording.dictionary(), Some("whatspec@2.3000.1044659339"));
    assert_eq!(recording.artifact_class(), Some(ArtifactClass::Synthetic));
    assert_eq!(
        recording.input_digest(),
        Some(&b"cross-language-fixture"[..])
    );
    assert_eq!(recording.created_at(), Some(1_754_000_000_000));
    assert!(
        recording
            .note()
            .is_some_and(|note| note.contains("emit-fixtures"))
    );
}

#[test]
fn the_envelopes_inside_it_are_the_ones_written_separately() {
    // Ties the two formats together: the container's payloads must be exactly
    // the envelope fixtures, byte for byte.
    let bytes = container("recording");
    let recording = RecordingRef::decode(&bytes).expect("reads");

    let envelopes: Vec<_> = recording.envelopes().collect();
    assert_eq!(
        envelopes.len(),
        2,
        "the mark and the unknown kind are not stanzas"
    );
    assert_eq!(envelopes[0], fixture("receipt").as_slice());
    assert_eq!(envelopes[1], fixture("message-with-enc").as_slice());

    for envelope in envelopes {
        EnvelopeRef::decode(envelope).expect("and each is a valid envelope");
    }
}

#[test]
fn a_mark_and_an_unknown_kind_survive_the_crossing() {
    let bytes = container("recording");
    let recording = RecordingRef::decode(&bytes).expect("reads");

    let mark = recording
        .records()
        .find_map(|record| record.as_mark())
        .expect("the mark crossed");
    assert_eq!(mark.delta_us, 1_500);
    assert_eq!(mark.label, "stream:error");

    assert_eq!(
        recording.skipped_records(),
        1,
        "a kind this build does not know is counted, not lost"
    );
    assert_eq!(recording.records().count(), 4);
}

#[test]
fn an_unknown_ancillary_tag_written_there_is_preserved_here() {
    let bytes = container("recording");
    let recording = RecordingRef::decode(&bytes).expect("reads");

    assert_eq!(recording.unknown_critical_tags(), 0);
    assert_eq!(
        recording.value(Tag(0x0042)),
        Some(&b"ancillary"[..]),
        "preserved rather than dropped, per RFC-009"
    );
}

#[test]
fn a_recording_frozen_mid_write_there_is_still_readable_here() {
    // The artifact a crash recorder exists to produce, written by the other
    // implementation. If either side treated a missing trailer as a parse
    // error, this is where it would show.
    let bytes = container("recording-truncated");
    let recording = RecordingRef::decode(&bytes).expect("readable without a trailer");

    assert!(matches!(
        recording.integrity(),
        Integrity::Truncated { found: 4, .. }
    ));
    assert_eq!(
        recording.envelopes().count(),
        2,
        "every complete record before the cut is usable"
    );
    assert_eq!(recording.adapter().map(|a| a.id), Some("zapo"));
}
