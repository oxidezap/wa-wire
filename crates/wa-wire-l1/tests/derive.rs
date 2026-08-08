//! Deriving real stanza shapes.
//!
//! The generated code is exercised through the public surface, the way a host
//! uses it. What is asserted here is not that the generator ran — the build
//! proves that — but that the shapes it emitted actually match the stanzas
//! WhatsApp sends.

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use wa_wire_l1::generated::ParseNewsletterResponseNegative;
use wa_wire_l1::testing::{Fixture, parse};
use wa_wire_l1::{DeriveError, Event, KNOWN_TAGS, PROVENANCE, UNMODELLED_FIELDS, derive};

// --- dispatch --------------------------------------------------------------

#[test]
fn a_read_receipt_derives() {
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let event = derive(&parse(&stanza)).expect("derives");
    assert_eq!(event.tag(), "receipt");
    assert!(event.node().attr_eq("id", "ABCD1234"));
}

#[test]
fn a_message_derives() {
    // The full shape: whatspec marks `recipient` and the typed `t` required,
    // so a stanza without them is a different shape, not a broken one.
    let stanza = Fixture::node("message")
        .attr("id", "MSG1")
        .jid_attr("from", "5511999998888")
        .jid_attr("recipient", "5511888887777")
        .attr("t", "1700000000")
        .attr("type", "text")
        .child(Fixture::node("enc").attr("type", "msg").bytes(b"cipher"))
        .build();

    let event = derive(&parse(&stanza)).expect("derives");
    assert_eq!(event.tag(), "message");
}

#[test]
fn an_ack_derives() {
    let stanza = Fixture::node("ack")
        .attr("id", "ACK1")
        .attr("class", "message")
        .jid_attr("from", "5511999998888")
        .jid_attr("content", "5511888887777")
        .attr("t", "1700000000")
        .build();

    let event = derive(&parse(&stanza)).expect("derives");
    assert_eq!(event.tag(), "ack");
}

#[test]
fn an_unknown_tag_is_reported_as_such() {
    // Not an error in the frame — a stanza this build has no shape for. L0
    // still carries it, which is the whole point of L0 being total.
    let stanza = Fixture::node("presence").attr("type", "available").build();
    assert_eq!(derive(&parse(&stanza)), Err(DeriveError::UnknownStanza));
}

#[test]
fn a_known_tag_with_no_matching_shape_is_reported_apart_from_an_unknown_tag() {
    // `receipt` has shapes, but none matches a stanza missing every required
    // field. Telling this from `UnknownStanza` is what distinguishes "we do not
    // model this" from "this changed".
    let stanza = Fixture::node("receipt").build();
    assert_eq!(
        derive(&parse(&stanza)),
        Err(DeriveError::NoMatchingShape { tag: "receipt" })
    );
}

#[test]
fn every_known_tag_is_distinct_and_non_empty() {
    for (index, tag) in KNOWN_TAGS.iter().enumerate() {
        assert!(!tag.is_empty());
        for other in KNOWN_TAGS.iter().skip(index + 1) {
            assert_ne!(tag, other, "duplicate tag in KNOWN_TAGS");
        }
    }
    assert!(KNOWN_TAGS.contains(&"message"));
    assert!(KNOWN_TAGS.contains(&"receipt"));
    assert!(KNOWN_TAGS.contains(&"ack"));
    assert!(KNOWN_TAGS.contains(&"call"));
}

#[test]
fn a_derived_event_points_back_at_its_node() {
    let stanza = Fixture::node("receipt")
        .attr("id", "X")
        .jid_attr("from", "u")
        .build();
    let node = parse(&stanza);
    let event = derive(&node).expect("derives");
    assert_eq!(event.node().tag(), node.tag());
}

// --- fields ----------------------------------------------------------------

#[test]
fn receipt_fields_are_extracted_with_their_declared_types() {
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let Event::IncomingMsgReceiptParser(receipt) = derive(&parse(&stanza)).expect("derives") else {
        panic!("expected the receipt shape");
    };

    assert!(receipt.id.eq_str("ABCD1234"));
    assert_eq!(receipt.from.server(), "s.whatsapp.net");
    assert!(receipt.from.user().eq_str("5511999998888"));
    assert!(receipt.r#type.is_some(), "the enum resolved");
}

#[test]
fn an_unknown_enum_value_falls_through_to_a_shape_that_does_not_type_it() {
    // `type` is enum-valued in the message-receipt shape, so a value from a
    // future release fails it — and the tag's next shape, which reads `type` as
    // plain text, takes over.
    //
    // That fallthrough is the design working, not a hole: a stanza carrying one
    // unfamiliar enum value still derives into *something*, and the consumer
    // sees which shape matched rather than losing the stanza. What it must not
    // do is silently invent a variant, and it does not.
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD")
        .jid_attr("from", "u")
        .attr("type", "invented-by-a-future-release")
        .build();

    let event = derive(&parse(&stanza)).expect("a shape still matches");
    assert_eq!(event.tag(), "receipt");
    assert!(
        !matches!(event, Event::IncomingMsgReceiptParser(_)),
        "the shape that types `type` must not claim an unknown value"
    );

    // The strict shape rejects it on its own, which is what forced the
    // fallthrough.
    assert_eq!(
        wa_wire_l1::generated::IncomingMsgReceiptParser::derive(&parse(&stanza)),
        Err(DeriveError::UnknownEnumValue { key: "type" })
    );
}

#[test]
fn repeated_children_derive_lazily_and_in_order() {
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD")
        .jid_attr("from", "u")
        .child(
            Fixture::node("participants")
                .child(
                    Fixture::node("user")
                        .jid_attr("jid", "first")
                        .attr("t", "1700000001"),
                )
                .child(
                    Fixture::node("user")
                        .jid_attr("jid", "second")
                        .attr("t", "1700000002"),
                ),
        )
        .build();

    let Event::IncomingMsgReceiptParser(receipt) = derive(&parse(&stanza)).expect("derives") else {
        panic!("expected the receipt shape");
    };

    let participants = receipt.participants.expect("the participants child");
    let users: Vec<_> = participants
        .user()
        .collect::<Result<_, _>>()
        .expect("every user derives");

    assert_eq!(users.len(), 2);
    assert!(users[0].jid.user().eq_str("first"));
    assert_eq!(users[0].t, 1_700_000_001);
    assert!(users[1].jid.user().eq_str("second"));
    assert_eq!(users[1].t, 1_700_000_002);
}

#[test]
fn a_malformed_repeated_child_surfaces_per_item() {
    // The iterator yields a Result per child, so one bad entry does not
    // invalidate the ones around it.
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD")
        .jid_attr("from", "u")
        .child(
            Fixture::node("participants")
                .child(
                    Fixture::node("user")
                        .jid_attr("jid", "ok")
                        .attr("t", "1700000001"),
                )
                .child(Fixture::node("user").jid_attr("jid", "broken")),
        )
        .build();

    let Event::IncomingMsgReceiptParser(receipt) = derive(&parse(&stanza)).expect("derives") else {
        panic!("expected the receipt shape");
    };

    let results: Vec<_> = receipt.participants.expect("participants").user().collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert_eq!(results[1], Err(DeriveError::MissingAttr { key: "t" }));
}

#[test]
fn an_optional_child_is_absent_rather_than_failing() {
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD")
        .jid_attr("from", "u")
        .build();

    let Event::IncomingMsgReceiptParser(receipt) = derive(&parse(&stanza)).expect("derives") else {
        panic!("expected the receipt shape");
    };
    assert!(receipt.participants.is_none());
    assert!(receipt.error.is_none());
}

// --- purity ----------------------------------------------------------------

#[test]
fn deriving_the_same_stanza_twice_gives_the_same_event() {
    // The property the whole layering rests on: derivation is a function of the
    // stanza alone. If it were not, running it host-side would not be
    // equivalent to running it in the engine.
    let stanza = Fixture::node("receipt")
        .attr("id", "ABCD")
        .jid_attr("from", "u")
        .attr("type", "delivery")
        .build();
    let node = parse(&stanza);

    let first = derive(&node).expect("derives");
    let second = derive(&node).expect("derives");
    assert_eq!(first, second);
}

#[test]
fn two_identical_stanzas_derive_identically() {
    let build = || {
        Fixture::node("receipt")
            .attr("id", "ABCD")
            .jid_attr("from", "u")
            .attr("t", "1700000000")
            .build()
    };
    let (a, b) = (build(), build());
    assert_eq!(
        derive(&parse(&a)).expect("derives"),
        derive(&parse(&b)).expect("derives")
    );
}

// --- provenance and honesty ------------------------------------------------

#[test]
fn provenance_is_recorded_in_full() {
    assert!(PROVENANCE.is_complete());
    assert!(PROVENANCE.whatsapp_version.starts_with("2."));
    assert!(PROVENANCE.incoming_digest.starts_with("sha256:"));
    assert!(PROVENANCE.matches(&PROVENANCE));
}

#[test]
fn unmodelled_fields_are_named_rather_than_dropped_in_silence() {
    // A derivation that quietly omitted a field would look complete and be
    // wrong — and no conformance run could tell, because every engine would
    // agree on the same missing field. So the gaps are part of the API.
    for entry in UNMODELLED_FIELDS {
        assert!(!entry.is_empty());
        assert!(
            entry.contains(':'),
            "an entry must name what was dropped and why: {entry:?}"
        );
    }
}

/// A field is read by the name the wire uses, not the name the bundle uses.
///
/// The spec records both and they differ for fifty fields. The generator read
/// the wrong one, and no generated test could tell: a fixture built from the
/// same spec by the same rule was wrong in exactly the same way, so the pair
/// agreed with each other and with no real stanza.
///
/// Written by hand for that reason. The attribute spelled here is the one a
/// server sends, taken from the spec's wire-facing side rather than from the
/// generator that was reading the other one.
#[test]
fn an_attribute_is_read_by_its_name_on_the_wire() {
    let with_wire_name = |key: &str| {
        Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .bytes(b"x")
            .attr(key, "7")
            .attr("backoff", "1")
            .build()
    };

    let stanza = with_wire_name("application_error");
    let node = parse(&stanza);
    let shape = ParseNewsletterResponseNegative::derive(&node)
        .expect("the wire spelling satisfies the shape");
    assert_eq!(shape.application_error, 7);

    // The bundle's spelling is not what the wire carries, so a stanza using it
    // is missing the field. Reading by that name would have accepted this one.
    let wrong = with_wire_name("applicationError");
    assert!(
        ParseNewsletterResponseNegative::derive(&parse(&wrong)).is_err(),
        "camelCase is the bundle's name for this field, not the wire's"
    );
}

/// The derivation is for stanzas that *arrive*, and says so by being wrong
/// about one that leaves.
///
/// whatspec's `incoming` domain records how WA Web parses what the server
/// sends. An `<ack>` the client sends looks similar and means something else:
/// inbound it is the server acknowledging our send, outbound it is us
/// acknowledging a delivery. Same tag, same attributes, opposite speaker.
///
/// Nothing here can tell those apart, because nothing here is told which
/// direction a stanza travelled — so an outbound stanza does not merely fail
/// to derive, it derives *confidently and wrongly*. That is why the conformance
/// comparator refuses to derive outbound envelopes rather than trusting this.
#[test]
fn an_outbound_stanza_derives_under_the_inbound_grammar() {
    // The shape of an ack a client sends: id, to, class.
    let ours = Fixture::node("ack")
        .attr("id", "ABCD1234")
        .jid_attr("to", "5511999998888")
        .attr("class", "receipt")
        .build();

    let derived = derive(&parse(&ours));
    assert!(
        derived.is_ok(),
        "an outbound ack is accepted by an inbound shape: {:?}",
        derived.err()
    );
    // And it is reported as the inbound event, which is the wrong reading.
    assert!(matches!(derived.expect("derives"), Event::Ack(_)));
}

// --- mixin groups ------------------------------------------------------------

/// A mixin group's variants are tried richest-first, and the order decides.
///
/// `NewsletterMessageAck`'s required fields are a subset of
/// `NewsletterQuestionResponseAck`'s — the latter also needs
/// `response_server_id`. So the leaner variant accepts every stanza the richer
/// one does, and trying it first would claim them all: a question response
/// would be reported as an ordinary message ack, silently, forever.
///
/// This is D-041 one level down. The rule that orders shapes of a tag orders
/// variants of a mixin for exactly the same reason, and neither is a
/// preference.
#[test]
fn a_mixin_group_picks_the_richest_variant_that_fits() {
    use wa_wire_l1::generated::NewsletterQuestionResponseAckOrNewsletterMessageAck as Group;

    let with_server_id = Fixture::node("ack")
        .attr("class", "message")
        .attr("t", "1")
        .attr("edit", "1")
        .attr("response_server_id", "77")
        .bytes(b"franking")
        .build();
    let node = parse(&with_server_id);
    assert!(
        matches!(
            Group::derive(&node).expect("derives"),
            Group::NewsletterQuestionResponseAck(_)
        ),
        "the field only the richer variant requires must select it"
    );

    // Without it, the same stanza is the leaner variant — not an error.
    // Everything the richer variant needs except the one field only it
    // requires — so the difference under test is that field and nothing else.
    let without = Fixture::node("ack")
        .attr("class", "message")
        .attr("t", "1")
        .attr("edit", "1")
        .bytes(b"franking")
        .build();
    assert!(matches!(
        Group::derive(&parse(&without)).expect("derives"),
        Group::NewsletterMessageAck(_)
    ));
}

/// A variant guarded by a literal value is selected by that value.
///
/// The status mixin's three alternatives differ only in `edit`: `1` is an
/// edit, `7` a revoke, `8` an admin revoke. Nothing else tells them apart, so
/// dropping the guard would make all three the first one.
#[test]
fn a_guarded_variant_is_selected_by_its_literal() {
    use wa_wire_l1::generated::StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke as Group;

    for (value, expected) in [
        ("1", "StatusAckEdit"),
        ("7", "StatusAckRevoke"),
        ("8", "StatusAckAdminRevoke"),
    ] {
        let stanza = Fixture::node("ack").attr("edit", value).build();
        let derived = Group::derive(&parse(&stanza)).expect("derives");
        let name = match derived {
            Group::StatusAckEdit(_) => "StatusAckEdit",
            Group::StatusAckRevoke(_) => "StatusAckRevoke",
            Group::StatusAckAdminRevoke(_) => "StatusAckAdminRevoke",
            // `#[non_exhaustive]`: a variant added upstream reaches here, and
            // naming it rather than panicking keeps the failure readable.
            _ => "an alternative this test predates",
        };
        assert_eq!(name, expected, "edit={value}");
    }

    // A value no variant claims is none of them, rather than the first.
    let unknown = Fixture::node("ack").attr("edit", "3").build();
    assert!(Group::maybe_derive(&parse(&unknown)).is_none());
}

/// Nothing in `UNMODELLED_FIELDS`, and the constant still exists to say so.
#[test]
fn the_generator_expressed_every_field() {
    assert!(
        UNMODELLED_FIELDS.is_empty(),
        "still unmodelled: {UNMODELLED_FIELDS:?}"
    );
}
