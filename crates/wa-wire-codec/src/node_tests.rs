//! Parser tests.
//!
//! Frames are assembled byte by byte here rather than through an encoder: the
//! crate parses only, and hand-built fixtures keep the wire layout visible in
//! the test itself.

use super::*;
extern crate alloc;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use crate::token::{
    BINARY_8, BINARY_20, BINARY_32, DICTIONARY_0, HEX_8, JID_FB, JID_INTEROP, JID_PAIR, JID_USER,
    LIST_8, LIST_16, LIST_EMPTY, NIBBLE_8,
};

// --- fixture table ---------------------------------------------------------
//
// Small and explicit, so a test says what it means. Tags are 1-indexed on the
// wire, so token 1 is `SINGLE[0]`.

static SINGLE: [&str; 8] = [
    "message",        // tag 1
    "type",           // tag 2
    "text",           // tag 3
    "from",           // tag 4
    "enc",            // tag 5
    "s.whatsapp.net", // tag 6
    "participants",   // tag 7
    "to",             // tag 8
];
static DICT_0: [&str; 3] = ["alpha", "beta", "gamma"];
static DICT_1: [&str; 2] = ["delta", "epsilon"];
static DICTS: [&[&str]; 2] = [&DICT_0, &DICT_1];

fn table() -> TokenTable<'static> {
    TokenTable::new(&SINGLE, &DICTS)
}

fn parser() -> Parser<'static> {
    Parser::new(table())
}

// --- frame construction ----------------------------------------------------

/// One slot of a node: a scalar value in its on-wire form.
#[derive(Clone)]
enum Slot {
    Token(u8),
    Dict(u8, u8),
    Binary8(Vec<u8>),
    Binary20(Vec<u8>),
    Binary32(Vec<u8>),
    Nibble(Vec<u8>, bool),
    Hex(Vec<u8>, bool),
    Nil,
    Raw(Vec<u8>),
}

impl Slot {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Token(tag) => out.push(*tag),
            Self::Dict(dictionary, index) => {
                out.push(DICTIONARY_0 + dictionary);
                out.push(*index);
            }
            Self::Binary8(bytes) => {
                out.push(BINARY_8);
                out.push(bytes.len() as u8);
                out.extend_from_slice(bytes);
            }
            Self::Binary20(bytes) => {
                out.push(BINARY_20);
                let len = bytes.len() as u32;
                out.push(((len >> 16) & 0x0f) as u8);
                out.push(((len >> 8) & 0xff) as u8);
                out.push((len & 0xff) as u8);
                out.extend_from_slice(bytes);
            }
            Self::Binary32(bytes) => {
                out.push(BINARY_32);
                out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Nibble(bytes, odd) => {
                out.push(NIBBLE_8);
                out.push(if *odd {
                    0x80 | bytes.len() as u8
                } else {
                    bytes.len() as u8
                });
                out.extend_from_slice(bytes);
            }
            Self::Hex(bytes, odd) => {
                out.push(HEX_8);
                out.push(if *odd {
                    0x80 | bytes.len() as u8
                } else {
                    bytes.len() as u8
                });
                out.extend_from_slice(bytes);
            }
            Self::Nil => out.push(LIST_EMPTY),
            Self::Raw(bytes) => out.extend_from_slice(bytes),
        }
    }
}

/// A node under construction.
#[derive(Clone)]
struct Frame {
    tag: Slot,
    attrs: Vec<(Slot, Slot)>,
    body: Option<Body>,
}

#[derive(Clone)]
enum Body {
    Value(Slot),
    Children(Vec<Frame>),
    Empty,
}

impl Frame {
    fn new(tag: Slot) -> Self {
        Self {
            tag,
            attrs: Vec::new(),
            body: None,
        }
    }

    fn attr(mut self, key: Slot, value: Slot) -> Self {
        self.attrs.push((key, value));
        self
    }

    fn value(mut self, value: Slot) -> Self {
        self.body = Some(Body::Value(value));
        self
    }

    fn children(mut self, children: Vec<Frame>) -> Self {
        self.body = Some(Body::Children(children));
        self
    }

    fn empty_body(mut self) -> Self {
        self.body = Some(Body::Empty);
        self
    }

    fn slots(&self) -> usize {
        1 + self.attrs.len() * 2 + usize::from(self.body.is_some())
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_list_header(out, self.slots());
        self.tag.write(out);
        for (key, value) in &self.attrs {
            key.write(out);
            value.write(out);
        }
        match &self.body {
            None => {}
            Some(Body::Empty) => out.push(LIST_EMPTY),
            Some(Body::Value(value)) => value.write(out),
            Some(Body::Children(children)) => {
                write_list_header(out, children.len());
                for child in children {
                    child.write(out);
                }
            }
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write(&mut out);
        out
    }
}

fn write_list_header(out: &mut Vec<u8>, size: usize) {
    if u8::try_from(size).is_ok() {
        out.push(LIST_8);
        out.push(size as u8);
    } else {
        out.push(LIST_16);
        out.extend_from_slice(&(size as u16).to_be_bytes());
    }
}

// --- the shapes the protocol actually produces -----------------------------

#[test]
fn a_bare_node_parses() {
    let frame = Frame::new(Slot::Token(1)).bytes();
    let node = parser().parse(&frame).expect("parses");

    assert_eq!(node.tag(), Value::Token("message"));
    assert!(node.tag().eq_str("message"));
    assert_eq!(node.attr_count(), 0);
    assert!(!node.has_content());
    assert_eq!(node.attrs().count(), 0);
    assert_eq!(node.content(), Content::None);
    assert!(node.content().is_none());
    assert_eq!(node.children().count(), 0);
}

#[test]
fn attributes_parse_in_order_and_are_addressable() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Token(3))
        .attr(Slot::Token(4), Slot::Binary8(b"5511@x".to_vec()))
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    assert_eq!(node.attr_count(), 2);
    assert!(!node.has_content());

    let attrs: Vec<_> = node.attrs().collect();
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0], (Value::Token("type"), Value::Token("text")));
    assert_eq!(attrs[1].0, Value::Token("from"));
    assert_eq!(attrs[1].1.as_str(), Some("5511@x"));

    assert_eq!(node.attr("type"), Some(Value::Token("text")));
    assert_eq!(node.attr("from").and_then(Value::as_str), Some("5511@x"));
    assert_eq!(node.attr("missing"), None);

    assert!(node.attr_eq("type", "text"));
    assert!(!node.attr_eq("type", "image"));
    assert!(!node.attr_eq("missing", "text"));
}

#[test]
fn attrs_iterator_reports_an_exact_size() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Token(3))
        .attr(Slot::Token(4), Slot::Token(8))
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    let mut attrs = node.attrs();
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs.size_hint(), (2, Some(2)));
    assert!(attrs.next().is_some());
    assert_eq!(attrs.len(), 1);
    assert!(attrs.next().is_some());
    assert_eq!(attrs.len(), 0);
    assert!(attrs.next().is_none());
    assert!(attrs.next().is_none(), "exhausted stays exhausted");
}

#[test]
fn a_binary_body_is_borrowed_from_the_frame() {
    let payload = b"ciphertext".to_vec();
    let frame = Frame::new(Slot::Token(5))
        .attr(Slot::Token(2), Slot::Token(3))
        .value(Slot::Binary8(payload.clone()))
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    assert!(node.has_content());
    assert_eq!(node.content().as_bytes(), Some(&payload[..]));
    assert_eq!(node.content().as_children(), None);
    assert_eq!(node.content().as_value(), Some(Value::Bytes(&payload)));
}

#[test]
fn an_explicitly_empty_body_reads_as_none() {
    let frame = Frame::new(Slot::Token(1)).empty_body().bytes();
    let node = parser().parse(&frame).expect("parses");
    assert!(node.has_content(), "a slot is present");
    assert_eq!(node.content(), Content::None, "but it is the empty tag");
    assert_eq!(node.content().as_value(), None);
}

#[test]
fn children_parse_and_are_addressable() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Token(3))
        .children(vec![
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"one".to_vec())),
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"two".to_vec())),
            Frame::new(Slot::Token(8)),
        ])
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    let children: Vec<_> = node.children().collect();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].content().as_bytes(), Some(&b"one"[..]));
    assert_eq!(children[1].content().as_bytes(), Some(&b"two"[..]));
    assert_eq!(children[2].tag(), Value::Token("to"));

    assert_eq!(
        node.child_at(1).and_then(|c| c.content().as_bytes()),
        Some(&b"two"[..])
    );
    assert_eq!(node.child_at(3), None);

    let enc = node.child("enc").expect("first enc");
    assert_eq!(enc.content().as_bytes(), Some(&b"one"[..]));
    assert!(node.child("nope").is_none());
}

#[test]
fn children_iterator_reports_an_exact_size() {
    let frame = Frame::new(Slot::Token(1))
        .children(vec![Frame::new(Slot::Token(5)), Frame::new(Slot::Token(8))])
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    let mut children = node.children();
    assert_eq!(children.len(), 2);
    assert!(!children.is_empty());
    assert_eq!(children.size_hint(), (2, Some(2)));
    children.next();
    assert_eq!(children.len(), 1);
    children.next();
    assert!(children.is_empty());
    assert!(children.next().is_none());
}

#[test]
fn a_node_without_children_yields_an_empty_iterator() {
    let frame = Frame::new(Slot::Token(1))
        .value(Slot::Binary8(b"body".to_vec()))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let children = node.children();
    assert!(children.is_empty());
    assert_eq!(children.len(), 0);
    assert_eq!(node.child_at(0), None);
    assert_eq!(node.child("enc"), None);
}

#[test]
fn a_path_walks_to_a_nested_node() {
    // <message><participants><to><enc>payload</enc></to></participants></message>
    let frame = Frame::new(Slot::Token(1))
        .children(vec![Frame::new(Slot::Token(7)).children(vec![
            Frame::new(Slot::Token(8)).children(vec![
                Frame::new(Slot::Token(5)).value(Slot::Binary8(b"payload".to_vec())),
            ]),
        ])])
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    let enc = node.at_path([0u16, 0, 0]).expect("walks");
    assert_eq!(enc.tag(), Value::Token("enc"));
    assert_eq!(enc.content().as_bytes(), Some(&b"payload"[..]));

    // The empty path is the node itself — the root's own address.
    let root = node.at_path([]).expect("identity");
    assert_eq!(root.tag(), node.tag());

    assert_eq!(node.at_path([0u16, 0, 1]), None, "no second child");
    assert_eq!(node.at_path([1u16]), None, "no second participant");
    assert_eq!(node.at_path([0u16, 0, 0, 0]), None, "enc has no children");
}

#[test]
fn a_path_addresses_each_enc_of_a_multi_device_message() {
    // The shape that makes path-addressing necessary: several <enc> siblings,
    // each with its own plaintext in the envelope.
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(4), Slot::Token(6))
        .children(vec![
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"first".to_vec())),
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"second".to_vec())),
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"third".to_vec())),
        ])
        .bytes();
    let node = parser().parse(&frame).expect("parses");

    for (index, expected) in [(0u16, &b"first"[..]), (1, b"second"), (2, b"third")] {
        let enc = node.at_path([index]).expect("addressable");
        assert_eq!(enc.content().as_bytes(), Some(expected), "index {index}");
    }
}

// --- every value form ------------------------------------------------------

#[test]
fn dictionary_tokens_resolve_through_the_right_dictionary() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Dict(0, 1), Slot::Dict(1, 0))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let (key, value) = node.attrs().next().expect("one attr");
    assert_eq!(key, Value::Token("beta"));
    assert_eq!(value, Value::Token("delta"));
}

#[test]
fn all_three_binary_widths_parse() {
    for slot in [
        Slot::Binary8(vec![0xAA; 5]),
        Slot::Binary20(vec![0xBB; 300]),
        Slot::Binary32(vec![0xCC; 70_000]),
    ] {
        let frame = Frame::new(Slot::Token(1)).value(slot.clone()).bytes();
        let node = parser().parse(&frame).expect("parses");
        let bytes = node.content().as_bytes().expect("bytes body");
        match slot {
            Slot::Binary8(expected) | Slot::Binary20(expected) | Slot::Binary32(expected) => {
                assert_eq!(bytes, expected.as_slice());
            }
            _ => unreachable!("the fixture list holds only binary slots"),
        }
    }
}

#[test]
fn an_empty_binary_payload_parses() {
    let frame = Frame::new(Slot::Token(1))
        .value(Slot::Binary8(Vec::new()))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    assert_eq!(node.content().as_bytes(), Some(&[][..]));
}

#[test]
fn packed_runs_parse_in_both_alphabets_and_parities() {
    let cases: [(Slot, &str); 4] = [
        (Slot::Nibble(vec![0x55, 0x11], false), "5511"),
        (Slot::Nibble(vec![0x55, 0x1f], true), "551"),
        (Slot::Hex(vec![0xAB, 0xCD], false), "ABCD"),
        (Slot::Hex(vec![0xAB, 0xC0], true), "ABC"),
    ];
    for (slot, expected) in cases {
        let frame = Frame::new(Slot::Token(1))
            .attr(Slot::Token(2), slot)
            .bytes();
        let node = parser().parse(&frame).expect("parses");
        let value = node.attr("type").expect("present");
        assert!(value.eq_str(expected), "{value} != {expected}");
        assert_eq!(value.to_string(), expected);
        assert!(value.as_packed().is_some());
    }
}

#[test]
fn a_nil_attribute_value_parses_as_absent() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Nil)
        .bytes();
    // Nil is not textual, so it is not a valid attribute value.
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::NonStringAttr),
        "an absent attribute value must be rejected"
    );
}

#[test]
fn jid_pairs_parse_with_and_without_a_user() {
    let with_user = Frame::new(Slot::Token(1))
        .attr(
            Slot::Token(4),
            Slot::Raw(vec![JID_PAIR, NIBBLE_8, 0x02, 0x55, 0x11, 6]),
        )
        .bytes();
    let node = parser().parse(&with_user).expect("parses");
    let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
    assert_eq!(jid.to_string(), "5511@s.whatsapp.net");
    assert!(!jid.is_server_only());
    assert!(node.attr_eq("from", "5511@s.whatsapp.net"));

    let server_only = Frame::new(Slot::Token(1))
        .attr(Slot::Token(4), Slot::Raw(vec![JID_PAIR, LIST_EMPTY, 6]))
        .bytes();
    let node = parser().parse(&server_only).expect("parses");
    let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
    assert!(jid.is_server_only());
    assert_eq!(jid.to_string(), "s.whatsapp.net");
}

#[test]
fn a_jid_user_can_arrive_as_raw_bytes() {
    // Not the common encoding — phone numbers pack into nibbles — but a JID
    // user slot accepts a binary string and must round-trip through it.
    let frame = Frame::new(Slot::Token(1))
        .attr(
            Slot::Token(4),
            Slot::Raw(vec![JID_PAIR, BINARY_8, 4, b'u', b's', b'e', b'r', 6]),
        )
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
    assert_eq!(jid.user(), User::Bytes(b"user"));
    assert_eq!(jid.to_string(), "user@s.whatsapp.net");
    assert!(node.attr_eq("from", "user@s.whatsapp.net"));
}

#[test]
fn user_jids_map_every_domain_type() {
    let cases: [(u8, &str); 5] = [
        (0x00, "message:3@s.whatsapp.net"),
        (0x01, "message:3@lid"),
        (0x81, "message:3@hosted.lid"),
        (0x80, "message:3@hosted"),
        (0x7f, "message:3@s.whatsapp.net"),
    ];
    for (domain_type, expected) in cases {
        let frame = Frame::new(Slot::Token(1))
            .attr(Slot::Token(4), Slot::Raw(vec![JID_USER, domain_type, 3, 1]))
            .bytes();
        let node = parser().parse(&frame).expect("parses");
        let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
        assert_eq!(jid.to_string(), expected, "domain type {domain_type:#04x}");
        assert_eq!(jid.device(), 3);
    }
}

#[test]
fn a_user_jid_on_the_primary_device_omits_the_device() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(4), Slot::Raw(vec![JID_USER, 0x00, 0, 1]))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    assert!(node.attr_eq("from", "message@s.whatsapp.net"));
}

#[test]
fn interop_jids_carry_device_and_integrator() {
    let frame = Frame::new(Slot::Token(1))
        .attr(
            Slot::Token(4),
            Slot::Raw(vec![JID_INTEROP, 1, 0x00, 0x07, 0x00, 0x2A, 6]),
        )
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
    assert_eq!(jid.device(), 7);
    assert_eq!(jid.integrator(), Some(42));
    assert_eq!(jid.to_string(), "42-message:7@interop");
}

#[test]
fn messenger_jids_parse() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(4), Slot::Raw(vec![JID_FB, 1, 0x00, 0x05, 6]))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let jid = node.attr("from").and_then(Value::as_jid).expect("a jid");
    assert_eq!(jid.device(), 5);
    assert_eq!(jid.to_string(), "message:5@msgr");
}

#[test]
fn a_jid_body_parses_as_a_value() {
    let frame = Frame::new(Slot::Token(1))
        .value(Slot::Raw(vec![JID_PAIR, LIST_EMPTY, 6]))
        .bytes();
    let node = parser().parse(&frame).expect("parses");
    let value = node.content().as_value().expect("a value body");
    assert!(value.as_jid().is_some());
    assert_eq!(node.content().as_bytes(), None);
}

// --- list widths -----------------------------------------------------------

#[test]
fn a_sixteen_bit_list_header_parses() {
    // 128 attributes needs 257 slots, past what LIST_8 can describe.
    let mut frame = Frame::new(Slot::Token(1));
    for _ in 0..128 {
        frame = frame.attr(Slot::Token(2), Slot::Token(3));
    }
    let bytes = frame.bytes();
    assert_eq!(bytes[0], LIST_16, "fixture must exercise the wide header");

    let node = parser().parse(&bytes).expect("parses");
    assert_eq!(node.attr_count(), 128);
    assert_eq!(node.attrs().count(), 128);
}

#[test]
fn a_wide_child_list_parses() {
    let children: Vec<_> = (0..300).map(|_| Frame::new(Slot::Token(5))).collect();
    let frame = Frame::new(Slot::Token(1)).children(children).bytes();
    let node = parser().parse(&frame).expect("parses");
    assert_eq!(node.children().len(), 300);
    assert_eq!(node.children().count(), 300);
    assert_eq!(
        node.at_path([299u16]).map(|n| n.tag()),
        Some(Value::Token("enc"))
    );
}

// --- rejection -------------------------------------------------------------

#[test]
fn truncation_at_every_offset_is_rejected() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Token(3))
        .children(vec![
            Frame::new(Slot::Token(5)).value(Slot::Binary8(b"payload".to_vec())),
        ])
        .bytes();

    for cut in 0..frame.len() {
        assert!(
            parser().parse(&frame[..cut]).is_err(),
            "truncating at {cut} must not parse"
        );
    }
    assert!(parser().parse(&frame).is_ok());
}

#[test]
fn a_frame_that_does_not_start_with_a_list_is_rejected() {
    assert_eq!(
        parser().parse(&[0x01, 0x02]),
        Err(ParseError::ExpectedList { found: 0x01 })
    );
    assert_eq!(
        parser().parse(&[LIST_EMPTY]),
        Err(ParseError::ExpectedList { found: LIST_EMPTY })
    );
}

#[test]
fn an_empty_buffer_is_rejected() {
    assert_eq!(
        parser().parse(&[]),
        Err(ParseError::UnexpectedEof {
            needed: 1,
            available: 0
        })
    );
}

#[test]
fn a_zero_length_node_is_rejected() {
    // Every node has at least a tag, so a size of zero is impossible.
    assert_eq!(parser().parse(&[LIST_8, 0x00]), Err(ParseError::EmptyNode));
    assert_eq!(
        parser().parse(&[LIST_16, 0x00, 0x00]),
        Err(ParseError::EmptyNode)
    );
}

#[test]
fn trailing_bytes_are_rejected() {
    let mut frame = Frame::new(Slot::Token(1)).bytes();
    frame.push(0xFF);
    assert_eq!(parser().parse(&frame), Err(ParseError::TrailingBytes(1)));
    frame.extend_from_slice(&[0, 0]);
    assert_eq!(parser().parse(&frame), Err(ParseError::TrailingBytes(3)));
}

#[test]
fn parse_prefix_accepts_what_parse_rejects() {
    let mut frame = Frame::new(Slot::Token(1)).bytes();
    let node_len = frame.len();
    frame.extend_from_slice(b"tail");

    assert!(parser().parse(&frame).is_err());
    let (node, rest) = parser().parse_prefix(&frame).expect("prefix parses");
    assert_eq!(node.tag(), Value::Token("message"));
    assert_eq!(rest, b"tail");
    assert_eq!(frame.len() - rest.len(), node_len);
}

#[test]
fn an_unknown_single_byte_token_is_rejected() {
    // Tag 9 is past the fixture table, which is what an outdated table looks
    // like from the parser's side.
    let frame = Frame::new(Slot::Token(9)).bytes();
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::UnknownToken {
            dictionary: None,
            index: 9
        })
    );
}

#[test]
fn an_out_of_range_dictionary_index_is_rejected() {
    let frame = Frame::new(Slot::Dict(0, 9)).bytes();
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::UnknownToken {
            dictionary: Some(0),
            index: 9
        })
    );
}

#[test]
fn an_absent_dictionary_is_rejected() {
    let frame = Frame::new(Slot::Dict(3, 0)).bytes();
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::UnknownToken {
            dictionary: Some(3),
            index: 0
        })
    );
}

#[test]
fn a_non_string_tag_is_rejected() {
    // LIST_EMPTY in the tag slot decodes to Nil, which is not a name.
    let frame = Frame::new(Slot::Nil).bytes();
    assert_eq!(parser().parse(&frame), Err(ParseError::NonStringTag));
}

#[test]
fn a_nil_attribute_key_is_rejected() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Nil, Slot::Token(3))
        .bytes();
    assert_eq!(parser().parse(&frame), Err(ParseError::NonStringAttr));
}

#[test]
fn an_invalid_packed_length_is_rejected() {
    let frame = Frame::new(Slot::Token(1))
        .attr(Slot::Token(2), Slot::Raw(vec![NIBBLE_8, 0x80]))
        .bytes();
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::InvalidPackedLength { byte: 0x80 })
    );
}

#[test]
fn a_jid_inside_a_jid_user_slot_is_rejected() {
    // A nested JID cannot be a user part; the shape is malformed.
    let frame = Frame::new(Slot::Token(1))
        .attr(
            Slot::Token(4),
            Slot::Raw(vec![JID_PAIR, JID_PAIR, LIST_EMPTY, 6, 6]),
        )
        .bytes();
    assert_eq!(
        parser().parse(&frame),
        Err(ParseError::UnexpectedTag { tag: JID_PAIR })
    );
}

#[test]
fn a_jid_server_must_be_a_token() {
    let frame = Frame::new(Slot::Token(1))
        .attr(
            Slot::Token(4),
            Slot::Raw(vec![JID_PAIR, LIST_EMPTY, BINARY_8, 1, b'x']),
        )
        .bytes();
    // BINARY_8 is not a token tag, so the table lookup fails.
    assert!(matches!(
        parser().parse(&frame),
        Err(ParseError::UnknownToken { .. })
    ));
}

#[test]
fn nesting_past_the_depth_limit_is_rejected() {
    fn nest(depth: usize) -> Frame {
        let mut frame = Frame::new(Slot::Token(5));
        for _ in 0..depth {
            frame = Frame::new(Slot::Token(1)).children(vec![frame]);
        }
        frame
    }

    let shallow = nest(3).bytes();
    let deep = nest(40).bytes();

    let strict = Parser::new(table()).with_max_depth(5);
    assert_eq!(strict.max_depth(), 5);
    assert!(strict.parse(&shallow).is_ok());
    assert!(matches!(
        strict.parse(&deep),
        Err(ParseError::DepthLimitExceeded { .. })
    ));

    // The default limit is generous enough for anything real.
    assert!(parser().parse(&deep).is_ok());
    assert_eq!(parser().max_depth(), DEFAULT_MAX_DEPTH);
}

#[test]
fn a_zero_depth_budget_rejects_even_the_root() {
    let frame = Frame::new(Slot::Token(1)).bytes();
    let parser = Parser::new(table()).with_max_depth(0);
    assert!(matches!(
        parser.parse(&frame),
        Err(ParseError::DepthLimitExceeded { .. })
    ));
}

#[test]
fn an_overstated_child_count_is_rejected() {
    let mut frame = Frame::new(Slot::Token(1))
        .children(vec![Frame::new(Slot::Token(5))])
        .bytes();
    // The child list header is the last LIST_8 before the child itself.
    let position = frame.len() - 3;
    assert_eq!(frame[position], LIST_8);
    frame[position + 1] = 4;
    assert!(parser().parse(&frame).is_err());
}

// --- the parser itself -----------------------------------------------------

#[test]
fn the_parser_exposes_its_configuration() {
    let parser = parser();
    assert_eq!(parser.table(), table());
    assert_eq!(parser.max_depth(), DEFAULT_MAX_DEPTH);
    let custom = parser.with_max_depth(9);
    assert_eq!(custom.max_depth(), 9);
    assert_eq!(custom.table(), table());
}

#[test]
fn parsers_and_nodes_are_comparable() {
    let frame = Frame::new(Slot::Token(1)).bytes();
    let a = parser().parse(&frame).expect("parses");
    let b = parser().parse(&frame).expect("parses");
    assert_eq!(a, b);
    assert!(!alloc::format!("{a:?}").is_empty());
    assert_eq!(parser(), parser());

    let other = Frame::new(Slot::Token(2)).bytes();
    let c = parser().parse(&other).expect("parses");
    assert_ne!(a, c);
}

// An iterator built from a validated tree cannot hit its own error paths, so
// they are reached here by hand — the same discipline the envelope's decoder
// follows. Degrading to "no more items" beats panicking on a caller that
// assembled an iterator itself.

#[test]
fn an_attrs_iterator_over_exhausted_input_stops_instead_of_panicking() {
    let mut attrs = Attrs {
        table: table(),
        reader: Reader::new(&[]),
        remaining: 3,
    };
    assert_eq!(attrs.next(), None);
}

#[test]
fn an_attrs_iterator_stops_when_a_value_is_unreadable() {
    // A key parses; its value runs off the end.
    let mut attrs = Attrs {
        table: table(),
        reader: Reader::new(&[1]),
        remaining: 1,
    };
    assert_eq!(attrs.next(), None);
}

#[test]
fn a_children_iterator_over_exhausted_input_stops_instead_of_panicking() {
    let mut children = Children {
        table: table(),
        reader: Reader::new(&[]),
        remaining: 2,
        depth_budget: 4,
    };
    assert_eq!(children.next(), None);
}

#[test]
fn a_children_iterator_stops_when_a_child_body_is_unreadable() {
    // A well-formed header promising an attribute that is not there.
    let mut children = Children {
        table: table(),
        reader: Reader::new(&[LIST_8, 0x03, 1]),
        remaining: 1,
        depth_budget: 4,
    };
    assert_eq!(children.next(), None);
}

#[test]
fn content_variants_reject_the_wrong_accessor() {
    assert_eq!(Content::None.as_value(), None);
    assert_eq!(Content::None.as_bytes(), None);
    assert_eq!(Content::None.as_children(), None);
    assert!(Content::None.is_none());

    let value = Content::Value(Value::Token("x"));
    assert!(!value.is_none());
    assert_eq!(value.as_bytes(), None, "a token body is not raw bytes");
    assert_eq!(value.as_children(), None);
}

// --- against the bundled table --------------------------------------------

#[cfg(feature = "bundled-tokens")]
mod bundled {
    use super::*;
    use crate::tokens;

    #[test]
    fn the_bundled_table_has_the_expected_shape() {
        assert_eq!(tokens::TABLE.single_byte_len(), 236);
        assert_eq!(tokens::TABLE.dictionary_count(), 4);
        for dictionary in 0..4u8 {
            assert!(tokens::TABLE.dictionary(dictionary, 0).is_some());
            assert!(tokens::TABLE.dictionary(dictionary, 255).is_some());
        }
        assert!(tokens::SOURCE_DIGEST.starts_with("sha256:"));
    }

    #[test]
    fn known_tokens_resolve_at_their_wire_positions() {
        // Tags are 1-indexed, so tag 1 is the first entry.
        assert_eq!(tokens::TABLE.single_byte(2), Some("xmlstreamstart"));
        assert_eq!(tokens::TABLE.single_byte(3), Some("xmlstreamend"));
        assert_eq!(tokens::TABLE.single_byte(4), Some("s.whatsapp.net"));
        assert_eq!(tokens::TABLE.single_byte(5), Some("type"));
        assert_eq!(tokens::TABLE.single_byte(0), None);
        assert_eq!(tokens::TABLE.single_byte(237), None);
    }

    #[test]
    fn a_realistic_message_stanza_parses() {
        // <message from=<jid> type="text"><enc>…</enc></message>
        // Tag 8 is "receipt"; 5 is "type". Built against the real table so the
        // fixture would break if the dictionaries moved.
        let frame = Frame::new(Slot::Token(1))
            .attr(Slot::Token(5), Slot::Binary8(b"text".to_vec()))
            .children(vec![
                Frame::new(Slot::Token(1)).value(Slot::Binary8(b"cipher".to_vec())),
            ])
            .bytes();

        let parser = Parser::new(tokens::TABLE);
        let node = parser.parse(&frame).expect("parses");
        assert!(node.attr_eq("type", "text"));
        assert_eq!(node.children().len(), 1);
        assert_eq!(
            node.at_path([0u16]).and_then(|n| n.content().as_bytes()),
            Some(&b"cipher"[..])
        );
    }
}
