//! Takeover tests.
//!
//! The interceptor is driven directly with synthetic nodes, so the decision
//! logic is covered without a connection. What cannot be tested here is the
//! engine offering those nodes in the first place — that is the engine's own
//! test surface, and `whatsapp-rust` covers it.

use super::*;

use wa_wire_adapter::{CountingSink, NullSink};
use wa_wire_contract::EnvelopeRef;
use whatsapp_rust::wacore_binary::builder::NodeBuilder;
use whatsapp_rust::wacore_binary::marshal;
use whatsapp_rust::wacore_binary::node::Node;

fn frame_of(node: &Node) -> Vec<u8> {
    let encoded = marshal::marshal(node).expect("marshals");
    encoded[1..].to_vec()
}

fn owned(node: &Node) -> OwnedNodeRef {
    OwnedNodeRef::new(frame_of(node)).expect("decodes")
}

fn receipt() -> Node {
    NodeBuilder::new("receipt")
        .attr("id", "RCPT-1")
        .attr("from", "5511999998888@s.whatsapp.net")
        .build()
}

fn notification() -> Node {
    NodeBuilder::new("notification")
        .attr("type", "w:gp2")
        .build()
}

/// Run one stanza through an interceptor and return what it decided.
fn run<P: ClaimPolicy>(node: &Node, policy: P) -> (Interception, Vec<Vec<u8>>) {
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
    let interceptor = Interceptor {
        sink: Arc::new(Mutex::new(sink)),
        policy,
    };
    let decision = interceptor.intercept(&owned(node));
    let frames = captured.lock().expect("sink lock").clone();
    (decision, frames)
}

// --- forwarding ------------------------------------------------------------

#[test]
fn a_stanza_reaches_the_sink_byte_for_byte() {
    let node = receipt();
    let (_decision, frames) = run(&node, TakeEverything);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], frame_of(&node), "the frame crosses verbatim");
}

#[test]
fn forwarding_happens_before_the_decision() {
    // A claimed stanza must never be one the consumer did not receive: the
    // engine will not handle it either, so it would vanish.
    for policy_takes in [true, false] {
        let node = receipt();
        let (decision, frames) = if policy_takes {
            run(&node, TakeEverything)
        } else {
            run(&node, |_: &OwnedNodeRef| Claim::Observe)
        };
        assert_eq!(frames.len(), 1, "forwarded either way");
        assert_eq!(decision.is_handled(), policy_takes);
    }
}

#[test]
fn the_forwarded_frame_round_trips_through_an_envelope() {
    let node = receipt();
    let (_decision, frames) = run(&node, TakeEverything);

    let bytes = RawStanza::inbound(&frames[0])
        .encode_to_vec()
        .expect("encodes");
    let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
    assert_eq!(envelope.frame(), frame_of(&node).as_slice());
    assert!(
        envelope.flags().is_verbatim(),
        "zero-copy holds in takeover"
    );
}

// --- policies --------------------------------------------------------------

#[test]
fn take_everything_claims_every_stanza() {
    for node in [receipt(), notification()] {
        let (decision, _frames) = run(&node, TakeEverything);
        assert_eq!(decision, Interception::Handled, "{}", node.tag);
    }
}

#[test]
fn take_tags_claims_only_what_it_lists() {
    let policy = || TakeTags::new(["receipt", "call"]);

    let (claimed, _) = run(&receipt(), policy());
    assert_eq!(claimed, Interception::Handled);

    let (passed, frames) = run(&notification(), policy());
    assert_eq!(passed, Interception::Pass, "the engine still handles it");
    assert_eq!(frames.len(), 1, "and it was still forwarded");
}

#[test]
fn an_empty_tag_list_claims_nothing() {
    let (decision, frames) = run(&receipt(), TakeTags::new([]));
    assert_eq!(decision, Interception::Pass);
    assert_eq!(frames.len(), 1, "observing is not the same as ignoring");
}

#[test]
fn a_closure_is_a_policy() {
    let (decision, _frames) = run(&receipt(), |node: &OwnedNodeRef| {
        if node.get_attr("id").is_some() {
            Claim::Take
        } else {
            Claim::Observe
        }
    });
    assert_eq!(decision, Interception::Handled);
}

#[test]
fn observe_is_the_default_claim() {
    assert_eq!(Claim::default(), Claim::Observe);
    assert_eq!(Claim::Observe.interception(), Interception::Pass);
    assert_eq!(Claim::Take.interception(), Interception::Handled);
}

// --- failure ---------------------------------------------------------------

#[test]
fn a_poisoned_sink_passes_rather_than_claiming() {
    // Claiming a stanza the consumer never received would drop it entirely:
    // the engine skips it and nobody else has it. Passing is the safe side.
    let sink = Arc::new(Mutex::new(NullSink));
    let poison = Arc::clone(&sink);
    let _ = std::panic::catch_unwind(move || {
        let _guard = poison.lock().expect("first lock");
        panic!("consumer blew up");
    });
    assert!(sink.is_poisoned(), "fixture must actually poison the lock");

    let interceptor = Interceptor {
        sink,
        policy: TakeEverything,
    };
    assert_eq!(
        interceptor.intercept(&owned(&receipt())),
        Interception::Pass,
        "a stanza the consumer did not get must stay with the engine"
    );
}

// --- what takeover claims about itself -------------------------------------

#[test]
fn takeover_declares_what_it_actually_does() {
    assert!(TAKEOVER_INFO.has(Capability::Takeover));
    assert!(TAKEOVER_INFO.has(Capability::L0InboundTap));
    assert!(TAKEOVER_INFO.has(Capability::ZeroCopyFrame));
}

#[test]
fn takeover_does_not_claim_the_auth_phase_that_tap_has() {
    // The engine refuses to offer connection-critical stanzas to an
    // interceptor, so takeover cannot see the authentication exchange even
    // though tap can. The two capability sets differ in both directions —
    // takeover is not a superset of tap.
    assert!(
        !TAKEOVER_INFO.has(Capability::L0InboundAuthPhase),
        "success/failure/stream:error/ack are never offered"
    );
    assert!(
        crate::INFO.has(Capability::L0InboundAuthPhase),
        "while the tap does see them"
    );
    assert!(
        !crate::INFO.has(Capability::Takeover),
        "and the tap cannot suppress anything"
    );
}

#[test]
fn every_forwarded_stanza_satisfies_the_takeover_declaration() {
    let node = receipt();
    let (_decision, frames) = run(&node, TakeEverything);
    assert_eq!(
        TAKEOVER_INFO.verify(&RawStanza::inbound(&frames[0])),
        Ok(())
    );
}

#[test]
fn the_shared_sink_is_readable_by_the_caller() {
    let sink = Arc::new(Mutex::new(CountingSink::new()));
    let interceptor = Interceptor {
        sink: Arc::clone(&sink),
        policy: TakeEverything,
    };

    let node = receipt();
    interceptor.intercept(&owned(&node));
    interceptor.intercept(&owned(&node));

    let counts = *sink.lock().expect("sink lock");
    assert_eq!(counts.stanzas(), 2);
    assert_eq!(counts.re_encoded(), 0);
    assert_eq!(counts.frame_bytes(), frame_of(&node).len() as u64 * 2);
}
