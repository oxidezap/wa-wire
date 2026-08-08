//! One value, several valid encodings, one derived event.
//!
//! An encoder chooses how to write a value: a JID as the wire's JID form or as
//! text, a run of digits packed or literal, a known string as a dictionary
//! token or as bytes. The format allows all of it, so two engines encoding the
//! same stanza can hand the derivation genuinely different bytes.
//!
//! Every test here is the same assertion: the choice must not change the event.
//! A failure means the derivation reads one encoder and not another — which
//! shows up to a consumer as two engines disagreeing about identical traffic.

#![allow(clippy::expect_used, clippy::panic)]

use wa_wire_l1::derive;
use wa_wire_l1::testing::{Fixture, parse};

#[test]
fn a_server_jid_derives_whether_written_as_a_jid_or_as_text() {
    // Found in captured traffic: one engine writes `from="s.whatsapp.net"` in
    // the wire's JID form, the other as a dictionary token. Both are valid.
    let as_jid = Fixture::node("receipt")
        .attr("id", "R1")
        .server_jid_attr("from")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();
    let as_text = Fixture::node("receipt")
        .attr("id", "R1")
        // The token, which is what the second engine was observed writing —
        // a server is a dictionary entry, so that is the natural encoding.
        .token_attr("from", "s.whatsapp.net")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let from_jid = derive(&parse(&as_jid)).expect("the JID form derives");
    let from_text =
        derive(&parse(&as_text)).expect("the text form must derive too — an encoder may choose it");
    assert!(
        from_jid.semantic_eq(&from_text),
        "the same receipt, encoded two ways, derived two different events"
    );
}

#[test]
fn a_user_jid_derives_whether_written_as_a_jid_or_as_text() {
    let as_jid = Fixture::node("receipt")
        .attr("id", "R2")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();
    let as_text = Fixture::node("receipt")
        .attr("id", "R2")
        .attr("from", "5511999998888@s.whatsapp.net")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let from_jid = derive(&parse(&as_jid)).expect("the JID form derives");
    let from_text = derive(&parse(&as_text)).expect("the text form derives");
    assert!(from_jid.semantic_eq(&from_text));
}

#[test]
fn a_timestamp_derives_whether_packed_or_literal() {
    // The bug this file was written after: the nibble alphabet exists to
    // compress runs of digits, so a real encoder packs a timestamp.
    let packed = Fixture::node("receipt")
        .attr("id", "R3")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .packed_attr("t", "1700000000")
        .build();
    let literal = Fixture::node("receipt")
        .attr("id", "R3")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let from_packed = derive(&parse(&packed)).expect("packed derives");
    let from_literal = derive(&parse(&literal)).expect("literal derives");
    assert!(from_packed.semantic_eq(&from_literal));
}

#[test]
fn an_id_derives_whether_packed_or_literal() {
    // Message ids are often digits, and an encoder packs those too.
    let packed = Fixture::node("receipt")
        .packed_attr("id", "1700000000")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();
    let literal = Fixture::node("receipt")
        .attr("id", "1700000000")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let from_packed = derive(&parse(&packed)).expect("packed derives");
    let from_literal = derive(&parse(&literal)).expect("literal derives");
    assert!(
        from_packed.semantic_eq(&from_literal),
        "a packed id and a literal one are the same id"
    );
}

#[test]
fn an_enum_derives_whether_tokenised_or_written_out() {
    // `read` is in the dictionary, so an encoder may write the token or the
    // bytes. `Value::eq_str` handles every form, which is why enums were never
    // affected by the bug the integer and JID readers had — asserted rather
    // than assumed, since a future enum reader could stop going through it.
    let tokenised = Fixture::node("receipt")
        .attr("id", "R4")
        .jid_attr("from", "5511999998888")
        .token_attr("type", "read")
        .attr("t", "1700000000")
        .build();
    let written_out = Fixture::node("receipt")
        .attr("id", "R4")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let from_token = derive(&parse(&tokenised)).expect("tokenised derives");
    let from_bytes = derive(&parse(&written_out)).expect("written out derives");
    assert!(from_token.semantic_eq(&from_bytes));
}

#[test]
fn a_jid_with_a_device_survives_being_written_as_text() {
    let as_text = Fixture::node("receipt")
        .attr("id", "R5")
        .attr("from", "5511999998888:3@s.whatsapp.net")
        .attr("type", "read")
        .attr("t", "1700000000")
        .build();

    let event = derive(&parse(&as_text)).expect("derives");
    assert!(
        event
            .node()
            .attr_eq("from", "5511999998888:3@s.whatsapp.net"),
        "the device survived the round trip"
    );
}

#[test]
fn text_that_is_not_a_jid_is_still_rejected() {
    // Accepting a JID written as text must not turn the field into "any string
    // at all": a spec field holding something that is not a JID is a protocol
    // change worth reporting. A lone word written as bytes is the case that
    // matters — it is only a server when the wire wrote it as a token.
    for not_a_jid in ["", "@", "user@", "@server", "a@b@c", "plain-text"] {
        let stanza = Fixture::node("receipt")
            .attr("id", "R6")
            .attr("from", not_a_jid)
            .attr("type", "read")
            .attr("t", "1700000000")
            .build();
        assert!(
            derive(&parse(&stanza)).is_err(),
            "{not_a_jid:?} must not read as a JID"
        );
    }
}
