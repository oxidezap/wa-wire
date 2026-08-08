//! Joiner tests.
//!
//! The joiner is driven directly with synthetic stanzas and payloads, so the
//! correlation rules are covered without a connection. What cannot be tested
//! here is the engine emitting those events in that order — that is the
//! engine's own test surface.

use super::*;

use wa_wire_adapter::CountingSink;
use wa_wire_contract::{EnvelopeRef, PlaintextStatus};
use whatsapp_rust::wacore_binary::builder::NodeBuilder;
use whatsapp_rust::wacore_binary::marshal;
use whatsapp_rust::wacore_binary::node::Node;

/// Captures whole envelopes, which is what the joiner's output has to be
/// inspected as: the plaintext table only exists once encoded.
#[derive(Default)]
struct Capture {
    envelopes: Vec<Vec<u8>>,
}

impl StanzaSink for Capture {
    fn accept(&mut self, stanza: RawStanza<'_>) {
        self.envelopes
            .push(stanza.encode_to_vec().expect("envelope encodes"));
    }
}

impl Capture {
    fn entries(&self, index: usize) -> Vec<(Vec<u16>, PlaintextStatus, Vec<u8>)> {
        let envelope = EnvelopeRef::decode(&self.envelopes[index]).expect("decodes");
        envelope
            .entries()
            .map(|entry| {
                (
                    entry.path.iter().collect(),
                    entry.status,
                    entry.payload.to_vec(),
                )
            })
            .collect()
    }

    fn frame(&self, index: usize) -> Vec<u8> {
        EnvelopeRef::decode(&self.envelopes[index])
            .expect("decodes")
            .frame()
            .to_vec()
    }
}

fn owned(node: &Node) -> Arc<OwnedNodeRef> {
    let encoded = marshal::marshal(node).expect("marshals");
    Arc::new(OwnedNodeRef::new(encoded[1..].to_vec()).expect("decodes"))
}

/// A `<message>` whose children are the given `<enc>` types, in order.
fn message_with(id: &str, enc_types: &[&str]) -> Node {
    let children: Vec<Node> = enc_types
        .iter()
        .map(|enc_type| {
            NodeBuilder::new("enc")
                .attr("type", *enc_type)
                .bytes(vec![0u8; 4])
                .build()
        })
        .collect();
    NodeBuilder::new("message")
        .attr("id", id)
        .attr("from", "5511999998888@s.whatsapp.net")
        .children(children)
        .build()
}

fn plaintext(id: &str, enc_index: usize, payload: &[u8]) -> DecryptedEnc {
    DecryptedEnc {
        message_id: id.to_owned(),
        enc_index,
        payload: Bytes::copy_from_slice(payload),
    }
}

// --- stanzas that never wait ------------------------------------------------

#[test]
fn a_stanza_without_enc_goes_straight_through() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);

    assert_eq!(sink.envelopes.len(), 1, "emitted without waiting");
    assert_eq!(joiner.pending(), 0);
    assert!(sink.entries(0).is_empty(), "and carries no plaintext table");
}

#[test]
fn a_message_without_an_id_is_not_held() {
    // Nothing could match a plaintext back to it, so waiting would only delay
    // the frame and then emit it unobserved.
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    let node = NodeBuilder::new("message")
        .children([NodeBuilder::new("enc").attr("type", "msg").build()])
        .build();
    joiner.accept_frame(&owned(&node), &mut sink);

    assert_eq!(sink.envelopes.len(), 1);
    assert_eq!(joiner.pending(), 0);
}

#[test]
fn a_fan_out_stanza_is_emitted_as_l0_wire() {
    // The engine numbers `<participants><to>` encs after the direct ones, and
    // only the ones addressed to its own device. Without that device's JID the
    // adapter cannot say which node an index means — so it says nothing rather
    // than attaching a payload to the wrong `<enc>`.
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    let node = NodeBuilder::new("message")
        .attr("id", "FAN1")
        .children([
            NodeBuilder::new("enc")
                .attr("type", "skmsg")
                .bytes(vec![1u8; 4])
                .build(),
            NodeBuilder::new("participants")
                .children([NodeBuilder::new("to")
                    .attr("jid", "5511999998888:1@s.whatsapp.net")
                    .children([NodeBuilder::new("enc")
                        .attr("type", "pkmsg")
                        .bytes(vec![2u8; 4])
                        .build()])
                    .build()])
                .build(),
        ])
        .build();
    joiner.accept_frame(&owned(&node), &mut sink);

    assert_eq!(sink.envelopes.len(), 1, "emitted immediately");
    assert_eq!(joiner.pending(), 0, "and never waits");
    assert!(
        sink.entries(0).is_empty(),
        "no table beats a table addressing the wrong node"
    );
    assert_eq!(joiner.abandoned(), 0, "not giving up — never started");
}

// --- the common case --------------------------------------------------------

#[test]
fn a_message_waits_for_its_plaintext_and_then_emits_once() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M1", &["msg"])), &mut sink);
    assert!(sink.envelopes.is_empty(), "nothing emitted while waiting");
    assert_eq!(joiner.pending(), 1);

    joiner.accept_plaintext(&plaintext("M1", 0, b"hello"), &mut sink);

    assert_eq!(sink.envelopes.len(), 1, "one envelope, not two");
    assert_eq!(joiner.pending(), 0);
    assert_eq!(
        sink.entries(0),
        [(vec![0], PlaintextStatus::Ok, b"hello".to_vec())]
    );
    assert_eq!(joiner.abandoned(), 0);
}

#[test]
fn the_last_plaintext_is_what_releases_a_multi_enc_message() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M2", &["pkmsg", "msg"])), &mut sink);
    joiner.accept_plaintext(&plaintext("M2", 1, b"second"), &mut sink);
    assert!(sink.envelopes.is_empty(), "still one short");

    joiner.accept_plaintext(&plaintext("M2", 0, b"first"), &mut sink);

    assert_eq!(sink.envelopes.len(), 1);
    assert_eq!(
        sink.entries(0),
        [
            (vec![0], PlaintextStatus::Ok, b"first".to_vec()),
            (vec![1], PlaintextStatus::Ok, b"second".to_vec()),
        ],
        "in stanza order, whatever order they arrived in"
    );
}

#[test]
fn a_path_addresses_the_enc_among_all_children_not_among_the_encs() {
    // The engine counts `<enc>` nodes; the envelope addresses children. A
    // stanza that carries anything else first makes the two differ, and a
    // plaintext landing on the wrong node is a message attributed to the wrong
    // sender.
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    let node = NodeBuilder::new("message")
        .attr("id", "M3")
        .children([
            NodeBuilder::new("device-identity")
                .bytes(vec![9u8; 2])
                .build(),
            NodeBuilder::new("enc")
                .attr("type", "msg")
                .bytes(vec![0u8; 4])
                .build(),
        ])
        .build();
    joiner.accept_frame(&owned(&node), &mut sink);
    joiner.accept_plaintext(&plaintext("M3", 0, b"body"), &mut sink);

    assert_eq!(
        sink.entries(0),
        [(vec![1], PlaintextStatus::Ok, b"body".to_vec())],
        "child 1, though it is the stanza's first <enc>"
    );
}

#[test]
fn the_frame_crosses_byte_for_byte() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();
    let node = message_with("M4", &["msg"]);
    let owned = owned(&node);
    let expected = owned.backing_bytes().to_vec();

    joiner.accept_frame(&owned, &mut sink);
    joiner.accept_plaintext(&plaintext("M4", 0, b"x"), &mut sink);

    assert_eq!(sink.frame(0), expected, "held, not re-encoded");
}

// --- giving up --------------------------------------------------------------

#[test]
fn a_message_whose_plaintext_never_comes_is_emitted_unobserved() {
    let lookahead = NonZeroUsize::new(2).expect("nonzero");
    let mut joiner = PlaintextJoiner::with_lookahead(lookahead);
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M5", &["msg"])), &mut sink);
    for _ in 0..lookahead.get() {
        joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);
    }
    assert_eq!(joiner.pending(), 1, "still waiting at the limit");

    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);

    assert_eq!(joiner.pending(), 0, "one stanza past the limit gives up");
    assert_eq!(joiner.abandoned(), 1);
    let abandoned = sink
        .envelopes
        .iter()
        .position(|bytes| {
            EnvelopeRef::decode(bytes)
                .expect("decodes")
                .entry_count()
                .eq(&1)
        })
        .expect("the abandoned message");
    assert_eq!(
        sink.entries(abandoned),
        [(vec![0], PlaintextStatus::Unobserved, Vec::new())],
        "the node is named, with no cause claimed for it"
    );
}

#[test]
fn a_partly_decrypted_message_keeps_the_payloads_it_got() {
    let mut joiner = PlaintextJoiner::with_lookahead(NonZeroUsize::new(1).expect("nonzero"));
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M6", &["msg", "skmsg"])), &mut sink);
    joiner.accept_plaintext(&plaintext("M6", 0, b"got this one"), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);

    assert_eq!(joiner.abandoned(), 1);
    // Emitted after the receipt that aged it out, so find it by shape.
    let abandoned = sink
        .envelopes
        .iter()
        .position(|bytes| EnvelopeRef::decode(bytes).expect("decodes").entry_count() == 2)
        .expect("the abandoned message");
    assert_eq!(
        sink.entries(abandoned),
        [
            (vec![0], PlaintextStatus::Ok, b"got this one".to_vec()),
            (vec![1], PlaintextStatus::Unobserved, Vec::new()),
        ]
    );
}

#[test]
fn a_late_plaintext_is_dropped_rather_than_attached_to_nothing() {
    let mut joiner = PlaintextJoiner::with_lookahead(NonZeroUsize::new(1).expect("nonzero"));
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M7", &["msg"])), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);
    let emitted = sink.envelopes.len();

    joiner.accept_plaintext(&plaintext("M7", 0, b"too late"), &mut sink);

    assert_eq!(
        sink.envelopes.len(),
        emitted,
        "no second envelope for a frame already emitted"
    );
}

#[test]
fn an_enc_index_the_frame_does_not_have_is_ignored() {
    // The engine and the frame disagreeing about the stanza should not lose the
    // frame; the entry it cannot place is what gets dropped.
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M8", &["msg"])), &mut sink);
    joiner.accept_plaintext(&plaintext("M8", 7, b"nowhere"), &mut sink);
    assert!(sink.envelopes.is_empty(), "still waiting for its own enc");

    joiner.accept_plaintext(&plaintext("M8", 0, b"here"), &mut sink);
    assert_eq!(
        sink.entries(0),
        [(vec![0], PlaintextStatus::Ok, b"here".to_vec())]
    );
}

#[test]
fn a_plaintext_for_an_unknown_message_is_dropped() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_plaintext(&plaintext("never-seen", 0, b"x"), &mut sink);

    assert!(sink.envelopes.is_empty());
    assert_eq!(joiner.pending(), 0);
}

// --- shutdown ---------------------------------------------------------------

#[test]
fn flush_emits_what_is_still_waiting() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M9", &["msg"])), &mut sink);
    joiner.accept_frame(&owned(&message_with("M10", &["msg"])), &mut sink);
    joiner.accept_plaintext(&plaintext("M10", 0, b"done"), &mut sink);
    assert_eq!(joiner.pending(), 1);

    joiner.flush(&mut sink);

    assert_eq!(joiner.pending(), 0);
    assert_eq!(
        sink.envelopes.len(),
        2,
        "the complete one and the flushed one"
    );
    assert_eq!(joiner.abandoned(), 1, "flushing an incomplete one counts");
}

#[test]
fn flushing_an_empty_joiner_does_nothing() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = CountingSink::new();

    joiner.flush(&mut sink);

    assert_eq!(sink.stanzas(), 0);
    assert_eq!(joiner.abandoned(), 0);
}

// --- ordering ---------------------------------------------------------------

#[test]
fn stanzas_leave_in_the_order_they_arrived() {
    // A held `<message>` holds up the stream behind it, and that is the point.
    //
    // Emitting an unheld stanza the moment it arrives puts it ahead of a held
    // one that came first, and a recording compared position by position
    // reports the reordering as a divergence in whichever engine happened to
    // be slower. What leaves is what arrived, in that order.
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("M11", &["msg"])), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("receipt").build()), &mut sink);
    joiner.accept_frame(&owned(&NodeBuilder::new("notification").build()), &mut sink);

    assert_eq!(sink.envelopes.len(), 0, "the two behind it wait their turn");
    assert_eq!(joiner.pending(), 1, "only the message is waiting on anything");
    assert_eq!(joiner.queued(), 3);

    joiner.accept_plaintext(&plaintext("M11", 0, b"late but fine"), &mut sink);
    assert_eq!(sink.envelopes.len(), 3, "and all three leave at once");
    // The message first: it arrived first.
    assert_eq!(
        sink.entries(0),
        [(vec![0], PlaintextStatus::Ok, b"late but fine".to_vec())]
    );
    assert!(sink.entries(1).is_empty(), "the receipt carries no table");
}

#[test]
fn two_messages_wait_independently() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();

    joiner.accept_frame(&owned(&message_with("A", &["msg"])), &mut sink);
    joiner.accept_frame(&owned(&message_with("B", &["msg"])), &mut sink);
    assert_eq!(joiner.pending(), 2);

    // B completes first and still waits: A arrived first and is what the wire
    // had first. Independent waiting, ordered leaving.
    joiner.accept_plaintext(&plaintext("B", 0, b"bee"), &mut sink);
    assert_eq!(joiner.pending(), 1, "only A is still waiting on a payload");
    assert_eq!(sink.envelopes.len(), 0, "B is behind A");

    joiner.accept_plaintext(&plaintext("A", 0, b"ay"), &mut sink);
    assert_eq!(
        sink.entries(0),
        [(vec![0], PlaintextStatus::Ok, b"ay".to_vec())]
    );
    assert_eq!(
        sink.entries(1),
        [(vec![0], PlaintextStatus::Ok, b"bee".to_vec())]
    );
}

#[test]
fn the_declaration_holds_for_a_stanza_carrying_plaintext() {
    let mut joiner = PlaintextJoiner::new();
    let mut sink = Capture::default();
    joiner.accept_frame(&owned(&message_with("M12", &["msg"])), &mut sink);
    joiner.accept_plaintext(&plaintext("M12", 0, b"body"), &mut sink);

    let envelope = EnvelopeRef::decode(&sink.envelopes[0]).expect("decodes");
    assert!(envelope.flags().is_verbatim(), "zero-copy still holds");
    assert!(crate::INFO.has(wa_wire_adapter::Capability::L0Plaintext));
}
