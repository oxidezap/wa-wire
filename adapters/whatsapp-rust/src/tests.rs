//! Adapter tests.
//!
//! The event handler is exercised directly with synthetic `Event::RawNode`
//! values, so the whole forwarding path is covered without a connection. What
//! cannot be tested here is the engine dispatching those events in the first
//! place — that is the engine's own test surface.

use super::*;

use std::sync::{Arc, Mutex};

use wa_wire_adapter::{CountingSink, Direction, NullSink};
use wa_wire_codec::{Parser, tokens};
use wa_wire_contract::EnvelopeRef;
use whatsapp_rust::types::events;
use whatsapp_rust::wacore_binary::marshal;
use whatsapp_rust::wacore_binary::node::Node;
use whatsapp_rust::{NodeBuilder, OwnedNodeRef};

/// A stanza shaped like the ones this adapter will really see.
fn message_node() -> Node {
    NodeBuilder::new("message")
        .attr("from", "5511999998888@s.whatsapp.net")
        .attr("type", "text")
        .attr("id", "ABCD1234")
        .children([NodeBuilder::new("enc")
            .attr("v", "2")
            .attr("type", "msg")
            .bytes(b"ciphertext-bytes".to_vec())
            .build()])
        .build()
}

/// The buffer a decoder consumes: marshalled, minus the leading format byte.
fn frame_of(node: &Node) -> Vec<u8> {
    let encoded = marshal::marshal(node).expect("marshals");
    encoded[1..].to_vec()
}

fn raw_node_event(node: &Node) -> Arc<Event> {
    let owned = OwnedNodeRef::new(frame_of(node)).expect("decodes");
    Arc::new(Event::RawNode(Arc::new(owned)))
}

/// Run the handler over one event and return what the sink captured.
fn tap(event: Arc<Event>) -> Vec<Vec<u8>> {
    let captured: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let captured = Arc::clone(&captured);
        move |stanza: RawStanza<'_>| {
            captured
                .lock()
                .expect("sink lock")
                .push(stanza.frame.to_vec());
        }
    };
    let handler = RawNodeTap {
        sink: Arc::new(Mutex::new(sink)),
    };
    handler.handle_event(event);
    captured.lock().expect("sink lock").clone()
}

#[test]
fn a_raw_node_reaches_the_sink_byte_for_byte() {
    let node = message_node();
    let expected = frame_of(&node);

    let frames = tap(raw_node_event(&node));

    assert_eq!(frames.len(), 1, "exactly one stanza");
    assert_eq!(frames[0], expected, "the frame must cross verbatim");
}

#[test]
fn the_forwarded_frame_parses_with_the_wa_wire_codec() {
    // The claim that matters: what `whatsapp-rust` encodes, `wa-wire-codec`
    // reads. If the bundled token table ever drifts from the engine's, this is
    // where it shows.
    let node = message_node();
    let frames = tap(raw_node_event(&node));

    let parser = Parser::new(tokens::TABLE);
    let root = parser.parse(&frames[0]).expect("the codec parses it");

    assert!(root.tag().eq_str("message"));
    assert!(root.attr_eq("type", "text"));
    assert!(root.attr_eq("id", "ABCD1234"));
    assert!(root.attr_eq("from", "5511999998888@s.whatsapp.net"));

    let enc = root.child("enc").expect("the enc child");
    assert!(enc.attr_eq("v", "2"));
    assert_eq!(enc.content().as_bytes(), Some(&b"ciphertext-bytes"[..]));
}

#[test]
fn a_path_addresses_the_enc_the_way_an_envelope_would() {
    // The adapter cannot fill in plaintexts, but the paths a future one would
    // emit must already resolve against the frame it forwards today.
    let node = message_node();
    let frames = tap(raw_node_event(&node));
    let root = Parser::new(tokens::TABLE)
        .parse(&frames[0])
        .expect("parses");

    let mut path = wa_wire_adapter::NodePathBuf::new();
    path.push(0).expect("within the limit");

    let enc = root.at_path(path.iter()).expect("child 0 is the enc");
    assert!(enc.tag().eq_str("enc"));
    assert_eq!(enc.content().as_bytes(), Some(&b"ciphertext-bytes"[..]));
}

#[test]
fn the_frame_survives_an_envelope_round_trip() {
    let node = message_node();
    let expected = frame_of(&node);
    let frames = tap(raw_node_event(&node));

    let stanza = RawStanza::inbound(&frames[0]);
    let bytes = stanza.encode_to_vec().expect("encodes");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");

    assert_eq!(envelope.frame(), expected.as_slice());
    assert_eq!(envelope.flags().direction, Direction::Inbound);
    assert!(envelope.flags().is_verbatim());
    assert!(
        envelope.is_plaintext_empty(),
        "tap mode carries no plaintext table"
    );
}

#[test]
fn stanzas_of_every_shape_the_engine_produces_are_forwarded() {
    // Bare, attributes only, byte body, nested children, and the stream-control
    // node that the auth-phase capability claims to cover.
    let nodes = [
        NodeBuilder::new("ack").build(),
        NodeBuilder::new("receipt").attr("type", "read").build(),
        NodeBuilder::new("iq").bytes(b"payload".to_vec()).build(),
        message_node(),
        NodeBuilder::new("success")
            .attr("lid", "1234:5@lid")
            .build(),
        NodeBuilder::new("xmlstreamend").build(),
    ];

    for node in &nodes {
        let expected = frame_of(node);
        let frames = tap(raw_node_event(node));
        assert_eq!(frames.len(), 1, "{} was dropped", node.tag);
        assert_eq!(frames[0], expected, "{} was not verbatim", node.tag);

        Parser::new(tokens::TABLE)
            .parse(&frames[0])
            .unwrap_or_else(|error| panic!("{} did not parse: {error}", node.tag));
    }
}

#[test]
fn a_large_stanza_is_forwarded_without_truncation() {
    // Real captures peak around 433 KB.
    let node = NodeBuilder::new("iq")
        .attr("type", "result")
        .bytes(vec![0xAB; 500_000])
        .build();
    let frames = tap(raw_node_event(&node));

    assert_eq!(frames[0], frame_of(&node));
    let root = Parser::new(tokens::TABLE)
        .parse(&frames[0])
        .expect("parses");
    assert_eq!(root.content().as_bytes().map(<[u8]>::len), Some(500_000));
}

#[test]
fn events_other_than_raw_node_are_ignored() {
    // The handler narrows its interest, but the bus may still deliver more, so
    // the filter has to hold on its own.
    let frames = tap(Arc::new(Event::Connected(
        events::Connected::builder().build(),
    )));
    assert!(frames.is_empty(), "only RawNode is forwarded");
}

#[test]
fn a_poisoned_sink_drops_the_stanza_instead_of_unwinding() {
    // A consumer that panicked must not take the engine's receive path with it.
    let sink = Arc::new(Mutex::new(NullSink));
    let poison = Arc::clone(&sink);
    let _ = std::panic::catch_unwind(move || {
        let _guard = poison.lock().expect("first lock");
        panic!("consumer blew up");
    });
    assert!(sink.is_poisoned(), "fixture must actually poison the lock");

    let handler = RawNodeTap { sink };
    let node = message_node();
    handler.handle_event(raw_node_event(&node));
    // Reaching here without unwinding is the assertion.
}

#[test]
fn every_forwarded_stanza_satisfies_the_declaration() {
    let node = message_node();
    let frames = tap(raw_node_event(&node));
    assert_eq!(verify(&RawStanza::inbound(&frames[0])), Ok(()));
}

#[test]
fn the_declaration_matches_what_this_adapter_actually_does() {
    assert_eq!(INFO.id, PLUGIN_ID);
    assert_eq!(INFO.version, ADAPTER_VERSION);
    assert_eq!(INFO.engine_version, ENGINE_VERSION);

    // Claimed, and asserted elsewhere in this file.
    assert!(INFO.has(Capability::L0InboundTap));
    assert!(INFO.has(Capability::L0InboundAuthPhase));
    assert!(INFO.has(Capability::ZeroCopyFrame));

    // Not claimed, because the engine does not offer them at this point.
    assert!(
        !INFO.has(Capability::L0Plaintext),
        "RawNode fires before Signal has run"
    );
    assert!(
        !INFO.has(Capability::L0Outbound),
        "the engine has no raw outbound observer"
    );
    assert!(
        !INFO.has(Capability::Takeover),
        "RawNode observes; the pipeline runs regardless"
    );
    assert!(!INFO.has(Capability::L0Request));
    assert!(!INFO.has(Capability::DrainHook));
}

#[test]
fn an_undeclared_stanza_shape_is_rejected_by_verify() {
    // Guards the declaration itself: were this adapter ever to start
    // re-encoding, or to emit outbound stanzas, `verify` must catch it.
    assert_eq!(
        verify(&RawStanza::inbound(b"f").re_encoded()),
        Err(Violation::ReEncodedDespiteZeroCopy)
    );
    assert_eq!(
        verify(&RawStanza::outbound(b"f")),
        Err(Violation::OutboundWithoutCapability)
    );
}

#[test]
fn the_manifest_asks_only_for_what_it_needs() {
    // A tap observes. Messaging, IQ or task capabilities would let it send,
    // query or spawn — powers it has no business holding, and which the host
    // grants purely on the strength of this manifest.
    let actual = format!("{:?}", WaWirePlugin::new(NullSink).manifest());

    let observe_only = format!(
        "{:?}",
        PluginManifest::new(PLUGIN_ID, ADAPTER_VERSION)
            .with_capability(PluginCapability::CoreEvents)
    );
    assert_eq!(actual, observe_only);

    for extra in [
        PluginCapability::Messaging,
        PluginCapability::Iq,
        PluginCapability::Tasks,
        PluginCapability::PluginEvents,
    ] {
        let broader = format!(
            "{:?}",
            PluginManifest::new(PLUGIN_ID, ADAPTER_VERSION)
                .with_capability(PluginCapability::CoreEvents)
                .with_capability(extra)
        );
        assert_ne!(actual, broader, "manifest must not request {extra:?}");
    }
}

#[test]
fn the_shared_sink_is_readable_by_the_caller() {
    let plugin = WaWirePlugin::new(CountingSink::new());
    let sink = plugin.sink();

    let node = message_node();
    let handler = RawNodeTap {
        sink: Arc::clone(&sink),
    };
    handler.handle_event(raw_node_event(&node));
    handler.handle_event(raw_node_event(&node));

    let counts = *sink.lock().expect("sink lock");
    assert_eq!(counts.stanzas(), 2);
    assert_eq!(counts.plaintexts(), 0, "tap mode carries no plaintexts");
    assert_eq!(counts.re_encoded(), 0, "frames are verbatim");
    assert_eq!(counts.frame_bytes(), frame_of(&node).len() as u64 * 2);
}

#[test]
fn the_handler_declares_interest_in_raw_node_only() {
    // The interest hint is what makes the host take the forwarding lease; a
    // wider one would turn on machinery this adapter does not use.
    let handler = RawNodeTap {
        sink: Arc::new(Mutex::new(NullSink)),
    };
    let interest = handler.interest();
    assert!(interest.wants(EventKind::RawNode));
    assert!(!interest.wants(EventKind::Messages));
    assert!(!interest.wants(EventKind::Receipt));
    assert!(!interest.wants(EventKind::Connected));
}
