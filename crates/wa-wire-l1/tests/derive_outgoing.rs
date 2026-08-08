//! What the client sends, derived.
//!
//! The generated tests cover that each shape derives from its own fields. What
//! is asserted here is what no generator can check about itself: that the
//! outbound derivation is a *different* reading from the inbound one, and that
//! it is the one an outbound stanza deserves.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use wa_wire_l1::testing::{Fixture, parse};
use wa_wire_l1::{
    DeriveError, Event, MERGED_OUTGOING, OUTGOING_TAGS, UNMODELLED_OUTGOING, UNREACHABLE_OUTGOING,
    derive, derive_outgoing,
};

/// The same stanza reads differently depending on which way it travelled.
///
/// This is the whole reason the outbound derivation exists. An `<ack>` inbound
/// is the server acknowledging our send; outbound it is us acknowledging a
/// delivery. Both derivations accept it, and only one of them is right — which
/// is why the caller has to know the direction and the comparator does.
#[test]
fn one_stanza_two_readings_and_the_direction_decides() {
    let ack = Fixture::node("ack")
        .attr("id", "ABCD1234")
        .attr("class", "receipt")
        .jid_attr("to", "5511999998888")
        .build();
    let node = parse(&ack);

    let inbound = derive(&node).expect("the inbound grammar accepts it");
    let outbound = derive_outgoing(&node).expect("so does the outbound one");

    assert_eq!(inbound.tag(), "ack");
    assert_eq!(outbound.tag(), "ack");
    // Different types entirely: there is no comparison between them, and that
    // is the point. Feeding an outbound stanza to `derive` yields an event
    // named for what the server would have meant by it.
    assert!(matches!(inbound, Event::Ack(_)));
}

/// A namespace and type select the `<iq>` a client built.
#[test]
fn an_iq_is_selected_by_its_namespace_and_type() {
    let stanza = Fixture::node("iq")
        .attr("xmlns", "abt")
        .attr("type", "get")
        .attr("to", "s.whatsapp.net")
        .attr("id", "1")
        .child(Fixture::node("props").attr("protocol", "1"))
        .build();

    let derived = derive_outgoing(&parse(&stanza));
    assert!(derived.is_ok(), "{:?}", derived.err());
    assert_eq!(derived.expect("derives").tag(), "iq");
}

/// A namespace no builder uses is unknown, not the nearest match.
///
/// The spec covers what WA Web builds. An engine is entitled to send something
/// it does not, and reporting that plainly is worth more than picking whichever
/// shape happened to fit.
#[test]
fn an_unbuilt_iq_is_reported_as_unknown() {
    let stanza = Fixture::node("iq")
        .attr("xmlns", "no:such:namespace")
        .attr("type", "get")
        .attr("id", "1")
        .build();
    assert_eq!(
        derive_outgoing(&parse(&stanza)),
        Err(DeriveError::UnknownStanza)
    );
}

/// A tag no builder produces is unknown too.
#[test]
fn an_unmodelled_tag_is_unknown() {
    let stanza = Fixture::node("nothing-whatsapp-builds").build();
    assert_eq!(
        derive_outgoing(&parse(&stanza)),
        Err(DeriveError::UnknownStanza)
    );
}

/// Derivation is pure, so two runs over one node agree.
#[test]
fn deriving_twice_gives_the_same_answer() {
    let stanza = Fixture::node("ack")
        .attr("id", "ABCD1234")
        .attr("class", "receipt")
        .build();
    let node = parse(&stanza);
    let (a, b) = (derive_outgoing(&node), derive_outgoing(&node));
    assert_eq!(a, b);
    if let (Ok(a), Ok(b)) = (a, b) {
        assert!(a.semantic_eq(&b));
    }
}

/// Two stanzas of different shapes never mean the same thing.
#[test]
fn different_shapes_never_compare_equal() {
    let ack = Fixture::node("ack")
        .attr("id", "ABCD1234")
        .attr("class", "receipt")
        .build();
    let presence = Fixture::node("presence").attr("type", "available").build();

    let (Ok(a), Ok(b)) = (
        derive_outgoing(&parse(&ack)),
        derive_outgoing(&parse(&presence)),
    ) else {
        panic!("both derive");
    };
    assert!(!a.semantic_eq(&b));
    assert!(a.semantic_eq(&a));
}

#[test]
fn every_outgoing_tag_is_distinct_and_non_empty() {
    let mut seen = OUTGOING_TAGS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), before, "duplicate tag in OUTGOING_TAGS");
    assert!(OUTGOING_TAGS.iter().all(|tag| !tag.is_empty()));
    // The tags a client sends are not the tags it receives.
    assert!(OUTGOING_TAGS.contains(&"iq"));
    assert!(OUTGOING_TAGS.contains(&"presence"));
}

#[test]
fn the_generator_expressed_every_builder_attribute() {
    assert!(
        UNMODELLED_OUTGOING.is_empty(),
        "unmodelled: {UNMODELLED_OUTGOING:?}"
    );
}

/// Nothing is left that no stanza can derive as.
///
/// The list exists because the situation is possible, not because it is
/// expected: a shape strictly subsumed by another would be a type nothing can
/// ever reach, and silence about it would be worse than the list.
#[test]
fn no_shape_is_left_unreachable() {
    assert!(
        UNREACHABLE_OUTGOING.is_empty(),
        "unreachable: {UNREACHABLE_OUTGOING:?}"
    );
}

/// Four builders describe a stanza another already describes, and are folded.
///
/// whatspec records a module per builder, and two modules can build one stanza
/// while differing in something no reader can see: whether a value is handed
/// in or computed at build time, or whether one of them models an optional
/// attribute the other leaves out. Keeping both would be two types no stanza
/// can choose between.
///
/// The list is recomputed from the spec on every run, so a pair separates by
/// itself the day whatspec records something that tells them apart — and this
/// test then says the list wants updating rather than deleting.
#[test]
fn builders_describing_one_stanza_are_folded_and_named() {
    assert!(
        !MERGED_OUTGOING.is_empty(),
        "if this is empty the spec grew a discriminator, which is good news \
         and wants the list updated rather than removed"
    );
    for (folded, survivor) in MERGED_OUTGOING {
        assert_ne!(folded, survivor, "a shape cannot fold into itself");
        assert!(!folded.is_empty() && !survivor.is_empty());
    }
}
