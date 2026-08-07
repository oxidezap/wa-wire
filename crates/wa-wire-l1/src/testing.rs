//! Building stanzas to derive from.
//!
//! Every string is written as a length-prefixed binary value rather than a
//! token, so a fixture depends on no real dictionary and cannot go stale when
//! one moves. That costs bytes and buys a test that means the same thing next
//! year.
//!
//! The one exception is a JID's server, which the wire format requires to be a
//! token — so fixtures carry a three-entry table holding just those.
//!
//! Available to dependents under the `testing` feature: an adapter or a
//! conformance runner needs to build stanzas too, and a second copy of this
//! would be a second thing to keep true.

extern crate alloc;
use alloc::vec::Vec;

use wa_wire_codec::{NodeRef, Parser, TokenTable};

const LIST_8: u8 = 248;
const LIST_16: u8 = 249;
const LIST_EMPTY: u8 = 0;
const BINARY_8: u8 = 252;
const BINARY_32: u8 = 254;
const JID_PAIR: u8 = 250;

/// Servers a fixture JID can name. Slot 0 is the `LIST_EMPTY` placeholder, as
/// in every real table.
static FIXTURE_SINGLE: [&str; 3] = ["", "s.whatsapp.net", "lid"];
const SERVER_PN_TAG: u8 = 1;

/// The table fixtures parse against.
///
/// Deliberately tiny: the wire format forces a JID's server to be a token, and
/// nothing else in a fixture is one. A fixture that accidentally leaned on a
/// real dictionary fails loudly here rather than silently tracking whichever
/// one happened to be bundled.
pub const FIXTURE_TABLE: TokenTable<'static> = TokenTable::new(&FIXTURE_SINGLE, &[]);

/// An assembled stanza, ready to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    bytes: Vec<u8>,
}

impl Fixture {
    /// Start building a node with `tag`.
    #[must_use]
    pub fn node(tag: &str) -> FixtureBuilder {
        FixtureBuilder::new(tag)
    }

    /// The encoded frame.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A node under construction.
#[derive(Debug, Clone)]
pub struct FixtureBuilder {
    tag: Vec<u8>,
    attrs: Vec<(Vec<u8>, Vec<u8>)>,
    body: Body,
}

#[derive(Debug, Clone)]
enum Body {
    None,
    Empty,
    Bytes(Vec<u8>),
    Children(Vec<FixtureBuilder>),
}

impl FixtureBuilder {
    fn new(tag: &str) -> Self {
        Self {
            tag: binary(tag.as_bytes()),
            attrs: Vec::new(),
            body: Body::None,
        }
    }

    /// Add a string attribute.
    #[must_use]
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attrs
            .push((binary(key.as_bytes()), binary(value.as_bytes())));
        self
    }

    /// Add an attribute whose value is a `user@s.whatsapp.net` JID.
    ///
    /// Written in the wire's own JID form rather than as text, because that is
    /// what a real stanza carries and what the codec will hand back as a JID.
    #[must_use]
    pub fn jid_attr(mut self, key: &str, user: &str) -> Self {
        let mut encoded = alloc::vec![JID_PAIR];
        encoded.extend_from_slice(&binary(user.as_bytes()));
        encoded.push(SERVER_PN_TAG);
        self.attrs.push((binary(key.as_bytes()), encoded));
        self
    }

    /// Add an attribute with no user part — a bare server JID.
    #[must_use]
    pub fn server_jid_attr(mut self, key: &str) -> Self {
        let encoded = alloc::vec![JID_PAIR, LIST_EMPTY, SERVER_PN_TAG];
        self.attrs.push((binary(key.as_bytes()), encoded));
        self
    }

    /// Give the node a raw byte body.
    #[must_use]
    pub fn bytes(mut self, body: &[u8]) -> Self {
        self.body = Body::Bytes(body.to_vec());
        self
    }

    /// Give the node an explicitly empty body.
    #[must_use]
    pub fn empty_body(mut self) -> Self {
        self.body = Body::Empty;
        self
    }

    /// Append a child node.
    #[must_use]
    pub fn child(mut self, child: FixtureBuilder) -> Self {
        match &mut self.body {
            Body::Children(children) => children.push(child),
            _ => self.body = Body::Children(alloc::vec![child]),
        }
        self
    }

    /// Finish the node.
    #[must_use]
    pub fn build(self) -> Fixture {
        let mut bytes = Vec::new();
        self.write(&mut bytes);
        Fixture { bytes }
    }

    fn slots(&self) -> usize {
        1usize
            .saturating_add(self.attrs.len().saturating_mul(2))
            .saturating_add(usize::from(!matches!(self.body, Body::None)))
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_list(out, self.slots());
        out.extend_from_slice(&self.tag);
        for (key, value) in &self.attrs {
            out.extend_from_slice(key);
            out.extend_from_slice(value);
        }
        match &self.body {
            Body::None => {}
            Body::Empty => out.push(LIST_EMPTY),
            Body::Bytes(body) => out.extend_from_slice(&binary(body)),
            Body::Children(children) => {
                write_list(out, children.len());
                for child in children {
                    child.write(out);
                }
            }
        }
    }
}

fn write_list(out: &mut Vec<u8>, size: usize) {
    if let Ok(small) = u8::try_from(size) {
        out.push(LIST_8);
        out.push(small);
    } else {
        out.push(LIST_16);
        let wide = u16::try_from(size).unwrap_or(u16::MAX);
        out.extend_from_slice(&wide.to_be_bytes());
    }
}

fn binary(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len().saturating_add(5));
    if let Ok(small) = u8::try_from(payload.len()) {
        out.push(BINARY_8);
        out.push(small);
    } else {
        out.push(BINARY_32);
        let wide = u32::try_from(payload.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&wide.to_be_bytes());
    }
    out.extend_from_slice(payload);
    out
}

/// Parse a fixture, panicking if it does not.
///
/// A fixture that does not parse is a broken test, not a finding.
///
/// # Panics
///
/// If the fixture is not a well-formed stanza.
// A fixture that does not parse is a broken test, and panicking says so at the
// line that built it. This module is test scaffolding, not library code.
#[allow(clippy::panic)]
#[must_use]
pub fn parse(fixture: &Fixture) -> NodeRef<'_> {
    match Parser::new(FIXTURE_TABLE).parse(fixture.bytes()) {
        Ok(node) => node,
        Err(error) => panic!("fixture does not parse: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn a_bare_node_round_trips() {
        let fixture = Fixture::node("receipt").build();
        let node = parse(&fixture);
        assert!(node.tag().eq_str("receipt"));
        assert_eq!(node.attr_count(), 0);
        assert!(!node.has_content());
    }

    #[test]
    fn attributes_round_trip() {
        let fixture = Fixture::node("receipt")
            .attr("id", "ABCD")
            .attr("type", "read")
            .build();
        let node = parse(&fixture);
        assert_eq!(node.attr_count(), 2);
        assert!(node.attr_eq("id", "ABCD"));
        assert!(node.attr_eq("type", "read"));
    }

    #[test]
    fn jid_attributes_round_trip_as_jids() {
        let fixture = Fixture::node("receipt")
            .jid_attr("from", "5511999998888")
            .server_jid_attr("to")
            .build();
        let node = parse(&fixture);

        let from = node
            .attr("from")
            .and_then(wa_wire_codec::Value::as_jid)
            .expect("a jid");
        assert_eq!(from.to_string(), "5511999998888@s.whatsapp.net");

        let to = node
            .attr("to")
            .and_then(wa_wire_codec::Value::as_jid)
            .expect("a jid");
        assert!(to.is_server_only());
        assert_eq!(to.to_string(), "s.whatsapp.net");
    }

    #[test]
    fn bodies_round_trip() {
        let with_bytes = Fixture::node("enc").bytes(b"cipher").build();
        assert_eq!(
            parse(&with_bytes).content().as_bytes(),
            Some(&b"cipher"[..])
        );

        let empty = Fixture::node("enc").empty_body().build();
        let node = parse(&empty);
        assert!(node.has_content());
        assert!(node.content().is_none());
    }

    #[test]
    fn children_round_trip_in_order() {
        let fixture = Fixture::node("receipt")
            .child(Fixture::node("user").attr("n", "1"))
            .child(Fixture::node("user").attr("n", "2"))
            .build();
        let node = parse(&fixture);
        assert_eq!(node.children().len(), 2);
        assert!(node.child_at(0).expect("first").attr_eq("n", "1"));
        assert!(node.child_at(1).expect("second").attr_eq("n", "2"));
    }

    #[test]
    fn a_wide_node_uses_the_sixteen_bit_list_header() {
        let mut builder = Fixture::node("iq");
        for index in 0..200 {
            builder = builder.attr("k", &alloc::format!("{index}"));
        }
        let fixture = builder.build();
        assert_eq!(fixture.bytes()[0], LIST_16, "fixture must exercise LIST_16");
        assert_eq!(parse(&fixture).attr_count(), 200);
    }

    #[test]
    fn a_large_body_uses_the_wide_binary_prefix() {
        let body = alloc::vec![0xABu8; 400];
        let fixture = Fixture::node("enc").bytes(&body).build();
        assert_eq!(parse(&fixture).content().as_bytes(), Some(&body[..]));
    }

    #[test]
    fn fixtures_are_comparable() {
        assert_eq!(
            Fixture::node("a").attr("k", "v").build(),
            Fixture::node("a").attr("k", "v").build()
        );
        assert_ne!(Fixture::node("a").build(), Fixture::node("b").build());
    }

    #[test]
    fn the_fixture_table_holds_only_servers() {
        assert_eq!(FIXTURE_TABLE.dictionary_count(), 0);
        assert_eq!(
            FIXTURE_TABLE.single_byte(0),
            Some(""),
            "the LIST_EMPTY slot"
        );
        assert_eq!(
            FIXTURE_TABLE.single_byte(SERVER_PN_TAG),
            Some("s.whatsapp.net")
        );
        assert_eq!(FIXTURE_TABLE.single_byte(2), Some("lid"));
        assert_eq!(FIXTURE_TABLE.single_byte(3), None);
    }
}
