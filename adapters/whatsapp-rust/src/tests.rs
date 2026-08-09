//! Adapter tests.
//!
//! The event handler is exercised directly with synthetic `Event::RawNode`
//! values, so the whole forwarding path is covered without a connection. What
//! cannot be tested here is the engine dispatching those events in the first
//! place — that is the engine's own test surface.

use super::*;

use wa_wire_adapter::RequestError;

use crate::plaintext::PlaintextJoiner;

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
///
/// Flushes afterwards: a `<message>` is held for its plaintexts, and these
/// tests drive the handler with no decryption behind it. What they assert is
/// the frame, which the flush emits unchanged.
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
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::new(Mutex::new(sink)),
    };
    handler.handle_event(event);
    handler.flush();
    captured.lock().expect("sink lock").clone()
}

/// What a sink saw: which way each stanza went, and its frame.
type Seen = Arc<Mutex<Vec<(Direction, Vec<u8>)>>>;

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
    // The paths the adapter puts in a plaintext table must resolve against the
    // frame it forwards, or a payload lands on the wrong node.
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

    let handler = RawNodeTap {
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink,
    };
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

    assert!(
        INFO.has(Capability::L0Plaintext),
        "DecryptedPayload reports each one after Signal"
    );

    assert!(
        INFO.has(Capability::L0OutboundObserved),
        "SentFrame reports each stanza that went to the wire"
    );

    assert!(
        INFO.has(Capability::L0PlaintextCause),
        "EncDecryptFailed says why an <enc> produced nothing"
    );

    // Not claimed. Observing what was sent and being able to send are separate
    // capabilities on purpose, and this plugin only has the first.
    assert!(
        !INFO.has(Capability::L0Outbound),
        "this plugin observes and does not send"
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
    // re-encoding, `verify` must catch it.
    assert_eq!(
        verify(&RawStanza::inbound(b"f").re_encoded()),
        Err(Violation::ReEncodedDespiteZeroCopy)
    );
    // Outbound is declared now, so it is allowed — and the declaration is what
    // makes it allowed. An adapter that emitted one without claiming
    // `l0.outbound.observed` would still be caught.
    assert_eq!(verify(&RawStanza::outbound(b"f")), Ok(()));
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
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::clone(&sink),
    };
    handler.handle_event(raw_node_event(&node));
    handler.handle_event(raw_node_event(&node));
    // Both are `<message>`, so both wait; nothing decrypts behind this test.
    handler.flush();

    let counts = *sink.lock().expect("sink lock");
    assert_eq!(counts.stanzas(), 2);
    assert_eq!(
        counts.plaintexts(),
        2,
        "one unobserved entry per <enc>, naming the node without claiming a cause"
    );
    assert_eq!(counts.re_encoded(), 0, "frames are verbatim");
    assert_eq!(counts.frame_bytes(), frame_of(&node).len() as u64 * 2);
}

#[test]
fn the_handler_declares_interest_in_raw_node_only() {
    // The interest hint is what makes the host take the forwarding lease; a
    // wider one would turn on machinery this adapter does not use.
    let handler = RawNodeTap {
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::new(Mutex::new(NullSink)),
    };
    let interest = handler.interest();
    assert!(interest.wants(EventKind::RawNode));
    assert!(!interest.wants(EventKind::Messages));
    assert!(!interest.wants(EventKind::Receipt));
    assert!(!interest.wants(EventKind::Connected));
}

// --- the setup-time gate ----------------------------------------------------

#[test]
fn requiring_what_the_adapter_has_is_satisfied() {
    assert_eq!(
        INFO.require(
            CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0Plaintext)
                .with(Capability::ZeroCopyFrame)
        ),
        Ok(())
    );
}

#[test]
fn requiring_what_it_lacks_names_everything_missing_at_once() {
    // All of them, not the first: a caller fixes its setup in one pass rather
    // than one round trip per missing capability.
    let unmet = INFO
        .require(
            CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0Outbound)
                .with(Capability::L0Request),
        )
        .expect_err("this adapter does neither");

    assert_eq!(
        unmet.missing.iter().collect::<Vec<_>>(),
        [Capability::L0Outbound, Capability::L0Request],
        "the ones it has are not reported as missing"
    );
}

#[test]
fn takeover_and_tap_answer_the_same_requirement_differently() {
    // The difference the two capability sets exist to express: takeover cannot
    // see the auth phase. A consumer that needs it must be told at setup, not
    // by noticing that `success` never arrived.
    let needs_auth = CapabilitySet::NONE.with(Capability::L0InboundAuthPhase);

    assert_eq!(INFO.require(needs_auth), Ok(()));
    assert!(
        takeover::TAKEOVER_INFO.require(needs_auth).is_err(),
        "takeover is not offered connection-critical stanzas"
    );
}

#[test]
fn a_declared_requirement_is_carried_to_the_install() {
    // The builder is what a consumer actually calls, so the requirement has to
    // survive being put on it. Installing is covered by the host's own tests;
    // this checks the value is not dropped on the way.
    let plugin = WaWirePlugin::new(NullSink).requiring(
        CapabilitySet::NONE
            .with(Capability::L0Plaintext)
            .with(Capability::ZeroCopyFrame),
    );

    assert_eq!(
        plugin.required,
        CapabilitySet::NONE
            .with(Capability::L0Plaintext)
            .with(Capability::ZeroCopyFrame)
    );
    assert_eq!(
        INFO.require(plugin.required),
        Ok(()),
        "and this adapter satisfies it, so the install proceeds"
    );
}

#[test]
fn requiring_nothing_is_the_default_and_always_holds() {
    let plugin = WaWirePlugin::new(NullSink);
    assert_eq!(plugin.required, CapabilitySet::NONE);
    assert_eq!(INFO.require(CapabilitySet::NONE), Ok(()));
}

// --- outbound ---------------------------------------------------------------

#[test]
fn a_frame_round_trips_to_exactly_the_bytes_it_came_from() {
    // The property replay rests on: what an adapter forwards inbound is what it
    // can send back outbound, byte for byte. If these two ever diverge, a
    // recorded session stops being replayable and nothing would say so.
    for node in [
        message_node(),
        NodeBuilder::new("receipt").attr("id", "R1").build(),
        NodeBuilder::new("iq").bytes(vec![0xAB; 5000]).build(),
    ] {
        let marshalled = marshal::marshal(&node).expect("marshals");
        let frame = frame_of(&node);

        assert_eq!(
            to_marshalled(&frame),
            marshalled,
            "<{}> did not round-trip",
            node.tag
        );
    }
}

#[test]
fn an_empty_frame_still_produces_the_format_byte() {
    // Not a stanza the engine would accept, but the conversion must not lose
    // the byte that says how to read what follows.
    assert_eq!(to_marshalled(&[]), vec![0]);
}

#[test]
fn sending_is_declared_separately_from_observing() {
    // An adapter built to observe genuinely cannot send. One capability set
    // covering both would be false for whichever the consumer actually holds.
    assert!(!INFO.has(Capability::L0Outbound), "the tap does not send");
    assert!(SENDING_INFO.has(Capability::L0Outbound));

    // And sending adds to what the tap does rather than replacing it.
    for capability in CAPABILITIES.iter() {
        assert!(
            SENDING_INFO.has(capability),
            "{capability} was lost when sending was added"
        );
    }
}

#[test]
fn a_consumer_that_needs_to_send_is_refused_by_the_observing_declaration() {
    let needs_send = CapabilitySet::NONE.with(Capability::L0Outbound);

    assert!(
        INFO.require(needs_send).is_err(),
        "asking a tap to send is refused at setup, not at the first send"
    );
    assert_eq!(SENDING_INFO.require(needs_send), Ok(()));
}

#[test]
fn requesting_is_a_stronger_claim_than_sending() {
    // Writing to the socket and being handed the correlated answer are
    // different powers. An engine can offer the first without the second, so
    // one declaration covering both would be a claim neither has shown.
    assert!(!INFO.has(Capability::L0Request), "the tap does neither");
    assert!(
        !SENDING_INFO.has(Capability::L0Request),
        "sending alone does not correlate a reply"
    );
    assert!(REQUESTING_INFO.has(Capability::L0Request));
    assert!(
        REQUESTING_INFO.has(Capability::L0Outbound),
        "and requesting implies sending"
    );
}

#[test]
fn each_declaration_is_a_superset_of_the_last() {
    // The three are a ladder, not three unrelated sets: a consumer that
    // upgrades its requirement never loses something it already relied on.
    for capability in CAPABILITIES.iter() {
        assert!(SENDING_INFO.has(capability), "{capability} lost by sending");
    }
    for capability in SENDING_CAPABILITIES.iter() {
        assert!(
            REQUESTING_INFO.has(capability),
            "{capability} lost by requesting"
        );
    }
}

#[test]
fn a_consumer_that_needs_a_reply_is_refused_by_the_sending_declaration() {
    let needs_reply = CapabilitySet::NONE.with(Capability::L0Request);

    assert!(
        SENDING_INFO.require(needs_reply).is_err(),
        "found out at setup, not when the first reply never arrives"
    );
    assert_eq!(REQUESTING_INFO.require(needs_reply), Ok(()));
}

#[test]
fn a_rejection_this_engine_cannot_hand_over_says_so() {
    // The engine parses an error reply and keeps its code and text, not its
    // bytes — so `Rejected` carries `None` here. That is a real difference
    // between engines, and a consumer that needs the reply itself has to check
    // rather than find out at runtime.
    let rejected = RequestError::Rejected { frame: None };

    assert!(
        rejected.to_string().contains("did not hand over"),
        "the message says what is absent and why: {rejected}"
    );
    assert!(
        !matches!(rejected, RequestError::TimedOut),
        "and it is not confused with no reply at all"
    );
}

// --- the outbound half -------------------------------------------------------

/// The buffer the engine hands over on a send: marshalled, format byte **kept**.
///
/// This is where the two observation points differ. `RawNode` gives the buffer
/// its decoder consumed, which `unpack` has already stripped; `SentFrame` gives
/// what went to the Noise encryption, which still carries the byte.
fn sent_frame_event(node: &Node) -> Arc<Event> {
    let packed = marshal::marshal(node).expect("marshals");
    Arc::new(Event::SentFrame(
        events::SentFrame::builder().plaintext(packed.into()).build(),
    ))
}

/// A sent stanza crosses as an outbound envelope carrying the unpacked frame.
///
/// The assertion is against `frame_of`, which strips the format byte, so a
/// build that forwarded `SentFrame.plaintext` untouched fails here rather than
/// writing a recording every reader misparses by one byte.
#[test]
fn a_sent_frame_crosses_unpacked_and_outbound() {
    let node = message_node();
    let captured: Seen = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let captured = Arc::clone(&captured);
        move |stanza: RawStanza<'_>| {
            captured
                .lock()
                .expect("sink lock")
                .push((stanza.direction, stanza.frame.to_vec()));
        }
    };
    let handler = RawNodeTap {
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::new(Mutex::new(sink)),
    };
    handler.handle_event(sent_frame_event(&node));

    let seen = captured.lock().expect("sink lock").clone();
    assert_eq!(seen.len(), 1, "one send, one envelope");
    assert_eq!(seen[0].0, Direction::Outbound);
    assert_eq!(
        seen[0].1,
        frame_of(&node),
        "the format byte must be stripped, as it is on the inbound side"
    );

    // And what crossed is parseable as a stanza, which is the point of
    // stripping it: a reader walks this the same way it walks an inbound frame.
    let parsed = Parser::new(tokens::TABLE)
        .parse(&seen[0].1)
        .expect("the outbound frame parses");
    assert!(parsed.tag().eq_str("message"));
}

/// An outbound stanza waits for nothing.
///
/// A `<message>` inbound is held until its plaintexts arrive. The same tag
/// outbound has no `<enc>` this adapter decrypted, so holding it would delay a
/// stanza for a payload that is never coming. It is emitted without a flush.
#[test]
fn an_outbound_message_is_not_held_for_plaintexts() {
    let captured: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let sink = {
        let captured = Arc::clone(&captured);
        move |_: RawStanza<'_>| {
            *captured.lock().expect("sink lock") += 1;
        }
    };
    let handler = RawNodeTap {
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::new(Mutex::new(sink)),
    };

    handler.handle_event(sent_frame_event(&message_node()));
    assert_eq!(
        *captured.lock().expect("sink lock"),
        1,
        "emitted before any flush"
    );
}

/// A frame the engine wrote that will not unpack is dropped, not forwarded.
#[test]
fn an_unpackable_sent_frame_is_dropped() {
    let event = Arc::new(Event::SentFrame(
        events::SentFrame::builder()
            .plaintext(Vec::new().into())
            .build(),
    ));
    let captured: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let sink = {
        let captured = Arc::clone(&captured);
        move |_: RawStanza<'_>| {
            *captured.lock().expect("sink lock") += 1;
        }
    };
    let handler = RawNodeTap {
        joiner: Mutex::new(PlaintextJoiner::new()),
        sink: Arc::new(Mutex::new(sink)),
    };
    handler.handle_event(event);
    assert_eq!(*captured.lock().expect("sink lock"), 0);
}

/// Every reason the engine can give maps somewhere, and the two halves of the
/// engine's own `decryption_was_attempted` line up with the two halves here.
///
/// The mapping ends in a catch-all so a reason added upstream still resolves
/// rather than failing to compile — which means nothing would notice it landing
/// in the wrong half. This is what notices.
#[test]
fn every_failure_reason_maps_the_way_the_engine_reads_it() {
    use whatsapp_rust::types::events::EncDecryptFailureReason as Reason;

    const EVERY: [Reason; 15] = [
        Reason::MalformedNode,
        Reason::UnsupportedEncType,
        Reason::MalformedCiphertext,
        Reason::NoSession,
        Reason::NoSenderKey,
        Reason::UnknownPreKey,
        Reason::UntrustedIdentity,
        Reason::BadMac,
        Reason::InvalidMessage,
        Reason::NoMessageSecret,
        Reason::LocalCryptoFailure,
        Reason::SignalError,
        Reason::StorageFailure,
        Reason::PlaintextUnusable,
        Reason::NotAttempted,
    ];

    for reason in EVERY {
        let status = crate::status_for(reason);
        if reason.decryption_was_attempted() {
            assert_eq!(
                status,
                PlaintextStatus::DecryptFailed,
                "{reason:?} reached a cipher, so the status must claim it did"
            );
        } else {
            assert_ne!(
                status,
                PlaintextStatus::DecryptFailed,
                "{reason:?} never reached a cipher, so the status must not claim it did"
            );
        }
    }

    // The one the frozen vocabulary cannot carry: the engine could have tried
    // and chose not to, and no status says that.
    assert_eq!(
        crate::status_for(Reason::NotAttempted),
        PlaintextStatus::Unobserved
    );
    assert_eq!(
        crate::status_for(Reason::UnsupportedEncType),
        PlaintextStatus::Unsupported
    );
}
