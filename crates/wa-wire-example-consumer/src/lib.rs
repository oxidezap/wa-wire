//! A consumer, written once, run against any engine.
//!
//! This is the claim the project rests on, in code:
//!
//! > Swap the engine underneath and the consumer does not change.
//!
//! What makes it true is what this crate does *not* depend on. There is no
//! engine here, no runtime, no transport, no async — only the boundary types.
//! `cargo tree -p wa-wire-example-consumer` lists four crates and none of them
//! is a WhatsApp client, which is the argument: code that cannot name an engine
//! cannot be coupled to one.
//!
//! The logic itself is deliberately small — count what arrived, remember the
//! ids — because the interesting part is not what a consumer computes. It is
//! that the same bytes produce the same answer no matter who produced them.
//!
//! ```
//! use wa_wire_example_consumer::Tally;
//!
//! # fn example(envelopes: &[Vec<u8>], table: wa_wire_codec::TokenTable<'_>) {
//! let mut tally = Tally::default();
//! for envelope in envelopes {
//!     tally.accept(envelope, table);
//! }
//! println!("{} stanzas, {} events", tally.stanzas, tally.derived);
//! # }
//! ```

#![no_std]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
    )
)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use wa_wire_codec::{Parser, TokenTable};
use wa_wire_contract::{Capability, CapabilitySet, Direction, EnvelopeRef};
use wa_wire_l1::content::{MessageKind, derive_content};
use wa_wire_l1::{Event, derive};

/// What this consumer needs of whatever engine it is pointed at.
///
/// Stated so an adapter that cannot do it refuses to install, rather than the
/// consumer running against traffic that quietly lacks what it reads. A
/// consumer that needs nothing in particular declares nothing; this one reads
/// the frame and the derivation, so it needs the inbound tap and nothing more.
///
/// Deliberately narrow. Requiring `l0.plaintext` here would exclude an engine
/// this consumer works perfectly well against, and a requirement that is not
/// really a requirement is worse than none — it is a refusal nobody can
/// evaluate.
pub const REQUIRED: CapabilitySet = CapabilitySet::NONE.with(Capability::L0InboundTap);

/// Why a stanza produced no event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Skipped {
    /// The envelope did not decode. The boundary itself is broken.
    Undecodable,
    /// The frame did not parse as a binary node.
    Unparsable,
    /// It parsed, and the derivation models no event for it. Ordinary: the
    /// derivation covers the stanzas WA Web itself parses, not every stanza.
    NotModelled,
}

/// What a consumer saw, in a form two runs can be compared by.
///
/// Deliberately order-independent in its maps and order-*dependent* in
/// [`order`](Self::order): an engine that delivered the same stanzas in a
/// different sequence is a difference worth seeing, not one to average away.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tally {
    /// Envelopes accepted.
    pub stanzas: usize,
    /// Envelopes that produced an event.
    pub derived: usize,
    /// Events by the stanza tag they came from.
    pub by_tag: BTreeMap<&'static str, usize>,
    /// Stanzas that produced no event, by reason.
    pub skipped: BTreeMap<Skipped, usize>,
    /// Every `id` attribute seen, in arrival order.
    pub ids: Vec<String>,
    /// The tag of each stanza, in arrival order.
    pub order: Vec<String>,
    /// Stanzas the engine reported as inbound.
    pub inbound: usize,
    /// Stanzas the engine reported as outbound.
    ///
    /// Counted separately because a consumer that cares which way a stanza was
    /// going should be comparable on that too.
    pub outbound: usize,

    /// Plaintexts read, by the kind of message they turned out to be.
    ///
    /// The half of L0-plain that used to cross and go unread: the frame says a
    /// `<message>` arrived, and only the payload says what it was.
    pub messages_by_kind: BTreeMap<MessageKind, usize>,
    /// Every message that carried text, in arrival order.
    pub texts: Vec<String>,
    /// Payloads that did not read as a message, by reason.
    pub unreadable_payloads: usize,
}

impl Tally {
    /// Take one envelope, whatever produced it.
    pub fn accept(&mut self, envelope: &[u8], table: TokenTable<'_>) {
        self.stanzas = self.stanzas.saturating_add(1);

        let Ok(decoded) = EnvelopeRef::decode(envelope) else {
            self.skip(Skipped::Undecodable);
            return;
        };
        match decoded.flags().direction {
            Direction::Inbound => self.inbound = self.inbound.saturating_add(1),
            Direction::Outbound => self.outbound = self.outbound.saturating_add(1),
        }

        let Ok(node) = Parser::new(table).parse(decoded.frame()) else {
            self.skip(Skipped::Unparsable);
            return;
        };
        self.order.push(node.tag().to_string());
        if let Some(id) = node.attr("id").and_then(wa_wire_codec::Value::as_str) {
            self.ids.push(id.to_string());
        }

        // The plaintexts are the other half of L0-plain, and reading them is
        // a separate derivation with a separate oracle: the stanza comes from
        // whatspec, the payload from `waE2E.proto`.
        for entry in decoded.entries().filter(|entry| entry.status.is_ok()) {
            match derive_content(entry.payload) {
                Ok(content) => {
                    let seen = self.messages_by_kind.entry(content.kind).or_insert(0);
                    *seen = seen.saturating_add(1);
                    if let Some(text) = content.text {
                        self.texts.push(text.to_string());
                    }
                }
                Err(_) => {
                    self.unreadable_payloads = self.unreadable_payloads.saturating_add(1);
                }
            }
        }

        match derive(&node) {
            Ok(event) => {
                self.derived = self.derived.saturating_add(1);
                let seen = self.by_tag.entry(event.tag()).or_insert(0);
                *seen = seen.saturating_add(1);
                self.on_event(&event);
            }
            Err(_) => self.skip(Skipped::NotModelled),
        }
    }

    /// Where a real consumer would do its work.
    ///
    /// Left empty on purpose: anything here would be this example's behaviour
    /// rather than the boundary's, and the boundary is what is being shown.
    #[expect(clippy::unused_self, reason = "the shape a consumer would fill in")]
    fn on_event(&self, _event: &Event<'_>) {}

    fn skip(&mut self, reason: Skipped) {
        let seen = self.skipped.entry(reason).or_insert(0);
        *seen = seen.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wa_wire_contract::{EnvelopeBuilder, Flags};

    fn table() -> TokenTable<'static> {
        wa_wire_codec::tokens::TABLE
    }

    /// A minimal envelope around `frame`, since this crate has no adapter to
    /// build one for it — which is itself the point.
    fn envelope(frame: &[u8], flags: Flags) -> Vec<u8> {
        EnvelopeBuilder::new(flags, frame)
            .encode_to_vec()
            .expect("encodes")
    }

    #[test]
    fn a_broken_envelope_is_counted_rather_than_dropped() {
        let mut tally = Tally::default();
        tally.accept(b"not an envelope", table());

        assert_eq!(tally.stanzas, 1);
        assert_eq!(tally.derived, 0);
        assert_eq!(tally.skipped.get(&Skipped::Undecodable), Some(&1));
        assert!(tally.order.is_empty(), "nothing to name it by");
    }

    #[test]
    fn an_unparsable_frame_is_distinguished_from_an_unmodelled_one() {
        let mut tally = Tally::default();
        tally.accept(&envelope(&[0xFF, 0xFF, 0xFF], Flags::inbound()), table());

        assert_eq!(tally.skipped.get(&Skipped::Unparsable), Some(&1));
        assert_eq!(tally.skipped.get(&Skipped::NotModelled), None);
    }

    #[test]
    fn direction_is_carried_through() {
        let mut tally = Tally::default();
        tally.accept(&envelope(&[], Flags::inbound()), table());
        tally.accept(&envelope(&[], Flags::outbound()), table());

        assert_eq!(tally.inbound, 1);
        assert_eq!(tally.outbound, 1);
    }

    #[test]
    fn a_stanza_that_derives_is_counted_by_tag_and_recorded_in_order() {
        // The path the cross-engine test exercises, asserted here too: this
        // crate's own coverage is not measured from the adapter workspace, so
        // without this the busiest function in it would look untested.
        use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture};

        let first = Fixture::node("receipt")
            .attr("id", "ABCD1234")
            .jid_attr("from", "5511999998888")
            .attr("type", "read")
            .build();
        let second = Fixture::node("receipt")
            .attr("id", "EFGH5678")
            .jid_attr("from", "5511999998888")
            .attr("type", "read")
            .build();

        let mut tally = Tally::default();
        tally.accept(&envelope(first.bytes(), Flags::inbound()), FIXTURE_TABLE);
        tally.accept(&envelope(second.bytes(), Flags::inbound()), FIXTURE_TABLE);

        assert_eq!(tally.stanzas, 2);
        assert_eq!(tally.derived, 2);
        assert_eq!(tally.inbound, 2);
        assert_eq!(tally.by_tag.get("receipt"), Some(&2));
        assert_eq!(tally.order, ["receipt", "receipt"]);
        assert_eq!(tally.ids, ["ABCD1234", "EFGH5678"]);
        assert!(tally.skipped.is_empty());
    }

    #[test]
    fn a_stanza_nobody_models_is_counted_apart_from_one_that_derives() {
        use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture};

        let unmodelled = Fixture::node("presence").attr("type", "available").build();
        let mut tally = Tally::default();
        tally.accept(
            &envelope(unmodelled.bytes(), Flags::inbound()),
            FIXTURE_TABLE,
        );

        assert_eq!(tally.stanzas, 1);
        assert_eq!(tally.derived, 0);
        assert_eq!(tally.skipped.get(&Skipped::NotModelled), Some(&1));
        assert_eq!(
            tally.order,
            ["presence"],
            "a stanza with no event still happened, and the order says so"
        );
        assert!(tally.ids.is_empty(), "it carried no id");
    }

    #[test]
    fn a_plaintext_that_crosses_the_boundary_is_read_as_a_message() {
        // The point of L0-plain, end to end: the frame says a `<message>`
        // arrived and only the payload says what it was. Before this the
        // payload crossed and nothing read it.
        use wa_wire_adapter::Plaintext;
        use wa_wire_contract::NodePath;
        use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture};

        let fixture = Fixture::node("message")
            .attr("id", "ABCD1234")
            .jid_attr("from", "5511999998888")
            .child(Fixture::node("enc").attr("type", "msg"))
            .build();
        // waE2E.Message { conversation: "hello there" }
        let payload = b"\x0a\x0bhello there";
        let path = [0u8, 0];
        let plaintexts = [Plaintext::ok(NodePath::from_le_bytes(&path), payload)];
        let envelope = wa_wire_adapter::RawStanza::inbound(fixture.bytes())
            .with_plaintexts(&plaintexts)
            .encode_to_vec()
            .expect("encodes");

        let mut tally = Tally::default();
        tally.accept(&envelope, FIXTURE_TABLE);

        assert_eq!(
            tally.messages_by_kind.get(&MessageKind::Conversation),
            Some(&1)
        );
        assert_eq!(tally.texts, ["hello there"]);
        assert_eq!(tally.unreadable_payloads, 0);
    }

    #[test]
    fn a_payload_the_reader_cannot_make_sense_of_is_counted_rather_than_dropped() {
        use wa_wire_adapter::Plaintext;
        use wa_wire_contract::NodePath;
        use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture};

        let fixture = Fixture::node("message").attr("id", "A").build();
        // Declares 127 bytes and supplies none.
        let path = [0u8, 0];
        let plaintexts = [Plaintext::ok(
            NodePath::from_le_bytes(&path),
            b"\x0a\x7f\x01",
        )];
        let envelope = wa_wire_adapter::RawStanza::inbound(fixture.bytes())
            .with_plaintexts(&plaintexts)
            .encode_to_vec()
            .expect("encodes");

        let mut tally = Tally::default();
        tally.accept(&envelope, FIXTURE_TABLE);

        assert_eq!(tally.unreadable_payloads, 1);
        assert!(tally.messages_by_kind.is_empty());
        assert!(tally.texts.is_empty());
    }

    #[test]
    fn two_tallies_over_the_same_input_are_equal() {
        // The property the cross-engine test relies on: the tally is a pure
        // function of the envelopes, so a difference between two runs is a
        // difference in what the engines produced.
        let input = [
            envelope(b"not an envelope", Flags::inbound()),
            envelope(&[], Flags::inbound()),
        ];
        let mut first = Tally::default();
        let mut second = Tally::default();
        for bytes in &input {
            first.accept(bytes, table());
            second.accept(bytes, table());
        }
        assert_eq!(first, second);
    }
}
