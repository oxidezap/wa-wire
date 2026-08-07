//! The two crates meeting: an envelope's plaintext entries addressing nodes
//! inside its frame.
//!
//! This is the whole point of path addressing. `wa-wire-contract` carries the
//! frame verbatim and a side table of decrypted payloads keyed by node path;
//! `wa-wire-codec` walks the frame to those paths. If these two ever disagree
//! about what a path means, a decrypted message ends up attributed to the wrong
//! recipient — so the agreement is asserted here rather than assumed.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use wa_wire_codec::{Parser, TokenTable, Value};
use wa_wire_contract::{
    EnvelopeBuilder, EnvelopeRef, Flags, NodePath, PlaintextEntry, PlaintextStatus,
};

// Slot 0 is the placeholder for LIST_EMPTY: the tag byte indexes the table
// directly, exactly as the engine's own table is laid out.
static SINGLE: [&str; 6] = ["<none>", "message", "from", "enc", "to", "participants"];
static DICTS: [&[&str]; 0] = [];

fn table() -> TokenTable<'static> {
    TokenTable::new(&SINGLE, &DICTS)
}

const LIST_8: u8 = 248;
const BINARY_8: u8 = 252;

/// `<message><enc>…</enc><enc>…</enc><enc>…</enc></message>`
///
/// The multi-device shape: one ciphertext per recipient device, each decrypting
/// to a different plaintext.
fn multi_device_frame(ciphertexts: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![LIST_8, 2, 1, LIST_8, ciphertexts.len() as u8];
    for ciphertext in ciphertexts {
        out.extend_from_slice(&[LIST_8, 2, 3, BINARY_8, ciphertext.len() as u8]);
        out.extend_from_slice(ciphertext);
    }
    out
}

fn path_bytes(components: &[u16]) -> Vec<u8> {
    components.iter().flat_map(|c| c.to_le_bytes()).collect()
}

#[test]
fn each_plaintext_reaches_the_enc_it_came_from() {
    let frame = multi_device_frame(&[b"cipher-a", b"cipher-b", b"cipher-c"]);

    // The adapter emits one entry per <enc>, addressed by its path.
    let paths: Vec<Vec<u8>> = (0..3u16).map(|i| path_bytes(&[i])).collect();
    let entries: Vec<PlaintextEntry<'_>> = paths
        .iter()
        .zip([&b"plain-a"[..], b"plain-b", b"plain-c"])
        .map(|(path, payload)| PlaintextEntry {
            path: NodePath::from_le_bytes(path),
            status: PlaintextStatus::Ok,
            payload,
        })
        .collect();

    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .with_entries(entries.iter().copied())
        .encode_to_vec()
        .expect("encodes");

    // The host side: decode the envelope, parse its frame, and reunite each
    // plaintext with its node.
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let parser = Parser::new(table());
    let root = parser.parse(envelope.frame()).expect("parses");

    assert_eq!(root.tag(), Value::Token("message"));
    assert_eq!(root.children().len(), 3);
    assert_eq!(envelope.entry_count(), 3);

    let mut reunited = 0;
    for entry in envelope.entries() {
        let node = root
            .at_path(entry.path.iter())
            .expect("every path must address a node");
        assert_eq!(node.tag(), Value::Token("enc"), "paths address the <enc>s");

        let ciphertext = node.content().as_bytes().expect("a ciphertext body");
        let expected = match entry.payload {
            b"plain-a" => &b"cipher-a"[..],
            b"plain-b" => b"cipher-b",
            _ => b"cipher-c",
        };
        assert_eq!(ciphertext, expected, "plaintext matched the wrong <enc>");
        reunited += 1;
    }
    assert_eq!(reunited, 3, "every entry was reunited");
}

#[test]
fn a_nested_path_reaches_through_participants() {
    // `<message><participants><to><enc>…</enc></to></participants></message>`
    #[rustfmt::skip]
    let frame = vec![
        LIST_8, 2, 1,                        // <message> with a body
        LIST_8, 1,                           //   one child
        LIST_8, 2, 5,                        //   <participants> with a body
        LIST_8, 1,                           //     one child
        LIST_8, 2, 4,                        //     <to> with a body
        LIST_8, 1,                           //       one child
        LIST_8, 2, 3,                        //       <enc> with a body
        BINARY_8, 6, b'c', b'i', b'p', b'h', b'e', b'r',
    ];

    let path = path_bytes(&[0, 0, 0]);
    let entry = PlaintextEntry {
        path: NodePath::from_le_bytes(&path),
        status: PlaintextStatus::Ok,
        payload: b"plaintext",
    };

    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .with_entries(core::iter::once(entry))
        .encode_to_vec()
        .expect("encodes");

    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    let entry = envelope.entries().next().expect("one entry");
    let node = root.at_path(entry.path.iter()).expect("addresses the enc");
    assert_eq!(node.tag(), Value::Token("enc"));
    assert_eq!(node.content().as_bytes(), Some(&b"cipher"[..]));
}

#[test]
fn a_failed_decryption_still_addresses_its_node() {
    // The point of carrying a status rather than dropping the entry: the
    // consumer can see *which* <enc> failed.
    let frame = multi_device_frame(&[b"ok", b"bad"]);
    let paths: Vec<Vec<u8>> = (0..2u16).map(|i| path_bytes(&[i])).collect();
    let entries = [
        PlaintextEntry {
            path: NodePath::from_le_bytes(&paths[0]),
            status: PlaintextStatus::Ok,
            payload: b"plain",
        },
        PlaintextEntry {
            path: NodePath::from_le_bytes(&paths[1]),
            status: PlaintextStatus::DecryptFailed,
            payload: b"",
        },
    ];

    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .with_entries(entries.iter().copied())
        .encode_to_vec()
        .expect("encodes");

    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    let failed: Vec<_> = envelope
        .entries()
        .filter(|entry| !entry.status.is_ok())
        .collect();
    assert_eq!(failed.len(), 1);
    let node = root
        .at_path(failed[0].path.iter())
        .expect("the failed enc is still addressable");
    assert_eq!(node.content().as_bytes(), Some(&b"bad"[..]));
    assert!(failed[0].payload.is_empty());
}

#[test]
fn the_root_path_addresses_the_stanza_itself() {
    // Some payloads belong to the stanza rather than to a child — an empty
    // path is the root's own address, in both crates.
    let frame = multi_device_frame(&[b"x"]);
    let entry = PlaintextEntry {
        path: NodePath::root(),
        status: PlaintextStatus::Ok,
        payload: b"whole-stanza",
    };

    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .with_entries(core::iter::once(entry))
        .encode_to_vec()
        .expect("encodes");

    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");

    let entry = envelope.entries().next().expect("one entry");
    assert!(entry.path.is_empty());
    let node = root.at_path(entry.path.iter()).expect("the root itself");
    assert_eq!(node.tag(), root.tag());
}

#[test]
fn an_envelope_without_plaintexts_still_parses_its_frame() {
    // Most stanzas — receipts, acks, presence — carry nothing encrypted.
    let frame = vec![LIST_8, 1, 1];
    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .encode_to_vec()
        .expect("encodes");

    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert!(envelope.is_plaintext_empty());

    let root = Parser::new(table())
        .parse(envelope.frame())
        .expect("parses");
    assert_eq!(root.tag(), Value::Token("message"));
    assert_eq!(root.children().len(), 0);
}

#[test]
fn the_frame_survives_the_boundary_byte_for_byte() {
    // The contract's central claim: nothing is re-encoded.
    let frame = multi_device_frame(&[b"a", b"bb", b"ccc"]);
    let bytes = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .encode_to_vec()
        .expect("encodes");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");

    assert_eq!(envelope.frame(), frame.as_slice());
    assert!(envelope.flags().is_verbatim());
}
