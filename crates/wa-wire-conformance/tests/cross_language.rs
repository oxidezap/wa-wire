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

    assert_eq!(envelope.entry_count(), 3);
    assert_eq!(root.children().len(), 3);

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
