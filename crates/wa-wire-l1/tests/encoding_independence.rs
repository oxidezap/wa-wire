//! One stanza written two ways must derive one event.
//!
//! The wire lets an encoder choose: a value in a dictionary may be a token or
//! spelled out, a run of digits may be packed two per byte or left as text, a
//! `user@server` may be a JID or a string. Which it picks is its business —
//! [`derive`] must not be able to tell.
//!
//! This is the four-engine agreement property applied one level down. There it
//! is four implementations over one input; here it is one implementation over
//! the same value written every way the format allows, which is the reason
//! four engines *can* disagree in the first place.
//!
//! It exists because the class is real and recurring. `parse_int` once read
//! only `as_str`, so every packed timestamp in real traffic failed to derive.
//! A JID's server was once read as a dictionary token only, so `@newsletter` —
//! in none of the five dictionaries — could not be parsed at all. Both were
//! found by someone noticing, not by anything that runs.

#![allow(clippy::expect_used, clippy::panic)]

use wa_wire_l1::derive;
use wa_wire_l1::testing::{Fixture, FixtureBuilder, parse};

/// Both writings must derive, and derive the same thing.
///
/// Compared with `semantic_eq` rather than `Debug` or `PartialEq`: an event
/// borrows the frame it came from, and the whole point here is that the two
/// frames differ. What must not differ is every field read out of them.
fn agree(what: &str, one: FixtureBuilder, other: FixtureBuilder) {
    let (left, right) = (one.build(), other.build());
    let (left_node, right_node) = (parse(&left), parse(&right));

    let derived = |node| match derive(node) {
        Ok(event) => event,
        Err(error) => panic!("{what}: must derive, got {error:?}"),
    };
    let (left_event, right_event) = (derived(&left_node), derived(&right_node));

    assert!(
        left_event.semantic_eq(&right_event),
        "{what}\n  left:  {left_event:?}\n  right: {right_event:?}"
    );
}

#[test]
fn a_timestamp_packed_or_spelled_out_derives_the_same() {
    // A run of digits is exactly what the nibble alphabet exists to compress,
    // so any encoder that packs will pack a timestamp.
    let receipt = |t: fn(FixtureBuilder) -> FixtureBuilder| {
        t(Fixture::node("receipt")
            .attr("id", "R1")
            .jid_attr("from", "5511999998888")
            .attr("type", "read"))
    };

    agree(
        "a packed timestamp reads as the same instant",
        receipt(|node| node.attr("t", "1700000000")),
        receipt(|node| node.packed_attr("t", "1700000000")),
    );
}

#[test]
fn an_enum_as_a_token_or_as_text_derives_the_same() {
    // `read` is in the single-byte dictionary, so an encoder may write it as
    // one byte or as four. The enum matcher has to see through that.
    let receipt = |kind: fn(FixtureBuilder) -> FixtureBuilder| {
        kind(
            Fixture::node("receipt")
                .attr("id", "R1")
                .jid_attr("from", "5511999998888"),
        )
        .attr("t", "1700000000")
    };

    agree(
        "the receipt type is the same either way",
        receipt(|node| node.attr("type", "read")),
        receipt(|node| node.token_attr("type", "read")),
    );
}

#[test]
fn a_jid_as_parts_or_as_text_derives_the_same() {
    // `jid_attr` writes the JID form; `attr` writes the same thing as a string
    // and leaves the reader to split it.
    let receipt = |from: fn(FixtureBuilder) -> FixtureBuilder| {
        from(Fixture::node("receipt").attr("id", "R1"))
            .attr("type", "read")
            .attr("t", "1700000000")
    };

    agree(
        "a JID means the same in either form",
        receipt(|node| node.jid_attr("from", "5511999998888")),
        receipt(|node| node.attr("from", "5511999998888@s.whatsapp.net")),
    );
}

#[test]
fn a_group_jid_as_parts_or_as_text_derives_the_same() {
    let receipt = |from: fn(FixtureBuilder) -> FixtureBuilder| {
        from(Fixture::node("receipt").attr("id", "R1"))
            .jid_attr("participant", "5511999998888")
            .attr("type", "read")
            .attr("t", "1700000000")
    };

    agree(
        "a group JID means the same in either form",
        receipt(|node| node.group_jid_attr("from", "120363000000000000")),
        receipt(|node| node.attr("from", "120363000000000000@g.us")),
    );
}

#[test]
fn a_device_jid_as_parts_or_as_text_derives_the_same() {
    // The device part is what distinguishes two shapes from each other, so
    // losing it to a string would not merely change a field.
    let call = |creator: fn(FixtureBuilder) -> FixtureBuilder| {
        Fixture::node("call")
            .attr("id", "C1")
            .jid_attr("from", "5511999998888")
            .attr("t", "1700000000")
            .child(
                creator(Fixture::node("offer_notice"))
                    .attr("call-id", "CALLID")
                    .attr("type", "offer")
                    .attr("media", "audio"),
            )
    };

    agree(
        "a device JID keeps its device either way",
        call(|node| node.device_jid_attr("call-creator", "5511999998888", 1)),
        call(|node| node.attr("call-creator", "5511999998888:1@s.whatsapp.net")),
    );
}

#[test]
fn an_id_packed_as_hex_or_spelled_out_derives_the_same() {
    // Real message ids are uppercase hexadecimal, which the hex alphabet packs
    // two per byte.
    let receipt = |id: fn(FixtureBuilder) -> FixtureBuilder| {
        id(Fixture::node("receipt"))
            .jid_attr("from", "5511999998888")
            .attr("type", "read")
            .attr("t", "1700000000")
    };

    agree(
        "a hexadecimal id is the same id packed",
        receipt(|node| node.attr("id", "3EB0C767D26B8E1B")),
        receipt(|node| node.hex_attr("id", "3EB0C767D26B8E1B")),
    );
}

#[test]
fn a_server_in_no_dictionary_still_derives() {
    // `newsletter`, `bot`, `interop` and `hosted.lid` are in none of the five
    // dictionaries, so each arrives spelled out where `s.whatsapp.net` arrives
    // as one byte. A reader that only resolves tokens refuses them.
    let message = Fixture::node("message")
        .attr("id", "M1")
        .attr("from", "120363000000000000@newsletter")
        .attr("recipient", "5511777776666@s.whatsapp.net")
        .attr("type", "text")
        .attr("t", "1700000000")
        .child(
            Fixture::node("enc")
                .attr("v", "2")
                .attr("type", "msg")
                .bytes(&[1, 2, 3]),
        );

    let fixture = message.build();
    let node = parse(&fixture);
    derive(&node).expect("a @newsletter message derives");
}
