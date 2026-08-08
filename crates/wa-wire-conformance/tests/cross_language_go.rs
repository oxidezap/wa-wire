//! Envelopes written by the Go encoder, read by the Rust one.
//!
//! The boundary format is described three times: in `wa-wire-contract`, in the
//! `zapo` adapter's TypeScript, and in the `hypermeow` adapter's Go. Three
//! because an adapter runs inside its engine and the engines are in three
//! languages — Rust in Go means cgo, and cgo in the per-stanza hot path is the
//! cost the boundary exists to avoid.
//!
//! Three descriptions only ever tested separately are three formats waiting to
//! diverge. The Go side has no Rust to check itself against, so this is the
//! check.
//!
//! Regenerate with `cd adapters/hypermeow && go run ./cmd/emit-fixtures`. CI
//! does it and requires no diff.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;

use wa_wire_contract::{Direction, EnvelopeRef, FrameOrigin, PlaintextStatus};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/hypermeow/fixtures")
        .join(format!("{name}.bin"));
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun: cd adapters/hypermeow && go run ./cmd/emit-fixtures",
            path.display()
        )
    })
}

/// The common case: a stanza with nothing encrypted.
#[test]
fn a_stanza_with_no_plaintexts_crosses_from_go() {
    let bytes = fixture("receipt");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");

    assert_eq!(envelope.flags().direction, Direction::Inbound);
    assert_eq!(envelope.flags().frame_origin, FrameOrigin::Original);
    assert_eq!(envelope.entries().count(), 0);
    assert_eq!(
        envelope.frame(),
        &[0xf8, 0x03, 0x2c, 0xfc, 0x04, b't', b'e', b's', b't']
    );
}

/// A frame of no bytes is legal, and is where a length prefix goes wrong.
#[test]
fn an_empty_frame_from_go_is_not_an_error() {
    let bytes = fixture("empty-frame");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert!(envelope.frame().is_empty());
    assert_eq!(envelope.entries().count(), 0);
}

/// The direction bit means the same thing in both languages.
#[test]
fn the_direction_bit_survives_the_crossing() {
    let bytes = fixture("outbound-verbatim");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert_eq!(envelope.flags().direction, Direction::Outbound);
    assert_eq!(envelope.flags().frame_origin, FrameOrigin::Original);
    assert_eq!(envelope.frame(), &[0x01, 0x02, 0x03]);
}

/// A payload addressed to a child that is not the first.
///
/// The number the Go adapter writes is the engine's own child index, and the
/// Rust side walks that path — so the two agreeing here is the whole of what
/// keeps a decrypted message attached to the node it came from.
#[test]
fn a_payload_addressed_to_a_child_arrives_at_that_child() {
    let bytes = fixture("message-with-enc");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");

    let entry = envelope.entries().next().expect("one entry");
    assert_eq!(entry.path.iter().collect::<Vec<_>>(), vec![1]);
    assert_eq!(entry.status, PlaintextStatus::Ok);
    assert_eq!(entry.payload, b"decrypted");
}

/// Several payloads and one node that produced nothing.
#[test]
fn a_table_with_a_gap_crosses_with_the_gap_named() {
    let bytes = fixture("multi-device-with-plaintexts");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");

    let entries: Vec<_> = envelope.entries().collect();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].payload, b"first");
    assert_eq!(entries[1].payload, b"second");
    // Named at the position it occupies, and carrying nothing.
    assert_eq!(entries[2].status, PlaintextStatus::Unobserved);
    assert_eq!(entries[2].path.iter().collect::<Vec<_>>(), vec![3]);
    assert!(entries[2].payload.is_empty());
}

/// The empty path means the root node, in both languages.
#[test]
fn the_root_path_means_the_same_thing_in_go() {
    let bytes = fixture("root-path-plaintext");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let entry = envelope.entries().next().expect("one entry");
    assert_eq!(entry.path.iter().count(), 0);
    assert_eq!(entry.payload, b"root");
}

/// A path of several components, so a reader stopping after the first is caught.
#[test]
fn a_nested_path_from_go_keeps_every_component() {
    let bytes = fixture("nested-path-plaintext");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let entry = envelope.entries().next().expect("one entry");
    assert_eq!(entry.path.iter().collect::<Vec<_>>(), vec![2, 0, 7]);
    assert_eq!(entry.payload, b"nested");
}

/// A failure carries no payload, and the reader must not go looking for one.
#[test]
fn a_failure_from_go_carries_nothing() {
    let bytes = fixture("decrypt-failed");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let entry = envelope.entries().next().expect("one entry");
    assert_eq!(entry.status, PlaintextStatus::DecryptFailed);
    assert!(entry.payload.is_empty());
}

/// Every fixture decodes, so a new one cannot be added without being readable.
///
/// The named tests above say what each file means; this says that none of them
/// is the only thing keeping an unreadable file out.
#[test]
fn every_go_fixture_decodes() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/hypermeow/fixtures");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|ext| ext != "bin") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("readable");
        EnvelopeRef::decode(&bytes).unwrap_or_else(|error| panic!("{}: {error:?}", path.display()));
        seen += 1;
    }
    assert!(seen >= 8, "only {seen} fixtures found");
}
