//! Every shape a payload can take, built from the numbers in `waE2E.proto`.

use super::*;
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;

/// A varint, as the format writes one.
///
/// Written out rather than assumed to fit a byte: the field numbers under test
/// go past 31, and a tag for those needs two bytes.
fn varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = u8::try_from(value & 0x7F).unwrap_or(0);
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

fn tag(number: u32, wire: u8) -> Vec<u8> {
    varint((u64::from(number) << 3) | u64::from(wire))
}

/// A length-delimited field: tag, length, bytes.
fn bytes_field(number: u32, value: &[u8]) -> Vec<u8> {
    let mut out = tag(number, 2);
    out.extend_from_slice(&varint(value.len() as u64));
    out.extend_from_slice(value);
    out
}

fn string_field(number: u32, value: &str) -> Vec<u8> {
    bytes_field(number, value.as_bytes())
}

/// A varint field, for the shapes that carry one.
fn varint_field(number: u32, value: u8) -> Vec<u8> {
    let mut out = tag(number, 0);
    out.extend_from_slice(&varint(u64::from(value)));
    out
}

// -- the shapes that speak ---------------------------------------------------

#[test]
fn a_plain_conversation_is_read_whole() {
    let payload = string_field(field::CONVERSATION, "hello there");
    let content = derive_content(&payload).expect("reads");

    assert_eq!(content.kind, MessageKind::Conversation);
    assert_eq!(content.text, Some("hello there"));
    assert_eq!(content.wrappers, 0);
}

#[test]
fn extended_text_carries_its_text_one_level_down() {
    let inner = string_field(field::EXTENDED_TEXT_TEXT, "a reply");
    let payload = bytes_field(field::EXTENDED_TEXT, &inner);
    let content = derive_content(&payload).expect("reads");

    assert_eq!(content.kind, MessageKind::ExtendedText);
    assert_eq!(content.text, Some("a reply"));
}

#[test]
fn a_caption_is_text() {
    // The caption fields sit at different numbers in each media message, so
    // this is where a copy-paste between them would show.
    for (variant, caption_field, kind) in [
        (field::IMAGE, field::IMAGE_TEXT, MessageKind::Image),
        (field::VIDEO, field::VIDEO_TEXT, MessageKind::Video),
        (field::DOCUMENT, field::DOCUMENT_TEXT, MessageKind::Document),
    ] {
        let inner = string_field(caption_field, "look at this");
        let payload = bytes_field(variant, &inner);
        let content = derive_content(&payload).expect("reads");

        assert_eq!(content.kind, kind, "{kind}");
        assert_eq!(content.text, Some("look at this"), "{kind}");
    }
}

#[test]
fn a_reaction_carries_the_emoji_it_is() {
    let inner = string_field(field::REACTION_TEXT, "👍");
    let payload = bytes_field(field::REACTION, &inner);
    let content = derive_content(&payload).expect("reads");

    assert_eq!(content.kind, MessageKind::Reaction);
    assert_eq!(content.text, Some("👍"));
}

#[test]
fn a_media_message_with_no_caption_says_nothing_rather_than_nothing_at_all() {
    // `None` and `Some("")` are different answers: one is "this kind does not
    // speak here", the other is "it spoke and said nothing".
    let inner = varint_field(1, 1);
    let payload = bytes_field(field::IMAGE, &inner);
    let content = derive_content(&payload).expect("reads");

    assert_eq!(content.kind, MessageKind::Image);
    assert_eq!(content.text, None);

    let empty = bytes_field(field::IMAGE, &string_field(field::IMAGE_TEXT, ""));
    assert_eq!(derive_content(&empty).expect("reads").text, Some(""));
}

#[test]
fn a_shape_that_never_speaks_has_no_text() {
    for (variant, kind) in [
        (field::STICKER, MessageKind::Sticker),
        (field::AUDIO, MessageKind::Audio),
        (field::LOCATION, MessageKind::Location),
        (field::CONTACT, MessageKind::Contact),
        (field::PROTOCOL, MessageKind::Protocol),
        (field::CALL, MessageKind::Call),
    ] {
        let payload = bytes_field(variant, &varint_field(1, 1));
        let content = derive_content(&payload).expect("reads");
        assert_eq!(content.kind, kind);
        assert_eq!(content.text, None, "{kind}");
    }
}

// -- wrappers ----------------------------------------------------------------

#[test]
fn a_device_sent_copy_reports_the_message_rather_than_the_envelope() {
    // The failure this exists to prevent: a consumer asking "what did this
    // say" and being told "it was a device-sent wrapper".
    let real = string_field(field::CONVERSATION, "sent from my other device");
    let inner = bytes_field(field::DEVICE_SENT_INNER, &real);
    let payload = bytes_field(field::DEVICE_SENT, &inner);

    let content = derive_content(&payload).expect("reads");
    assert_eq!(content.kind, MessageKind::Conversation);
    assert_eq!(content.text, Some("sent from my other device"));
    assert_eq!(content.wrappers, 1);
}

#[test]
fn every_wrapper_the_spec_declares_unwraps_the_same_way() {
    // Twenty-nine of them, all `FutureProofMessage` holding `.message = 1`.
    // The list is generated by type, and this walks all of it: a hand-written
    // version of it had seven, so twenty-two classes of message would have
    // read as unmodelled instead of being unwrapped.
    for wrapper in field::WRAPPERS {
        if wrapper == field::DEVICE_SENT {
            continue;
        }
        let real = string_field(field::CONVERSATION, "inside");
        let inner = bytes_field(field::FUTURE_PROOF_INNER, &real);
        let payload = bytes_field(wrapper, &inner);

        let content = derive_content(&payload).expect("reads");
        assert_eq!(
            content.kind,
            MessageKind::Conversation,
            "wrapper {wrapper} was not unwrapped"
        );
        assert_eq!(content.text, Some("inside"));
        assert_eq!(content.wrappers, 1);
    }
}

#[test]
fn wrappers_nest_and_the_count_says_how_deep() {
    // A view-once inside a device-sent, which real traffic produces.
    let real = string_field(field::CONVERSATION, "twice wrapped");
    let once = bytes_field(
        field::VIEW_ONCE,
        &bytes_field(field::FUTURE_PROOF_INNER, &real),
    );
    let twice = bytes_field(
        field::DEVICE_SENT,
        &bytes_field(field::DEVICE_SENT_INNER, &once),
    );

    let content = derive_content(&twice).expect("reads");
    assert_eq!(content.kind, MessageKind::Conversation);
    assert_eq!(content.text, Some("twice wrapped"));
    assert_eq!(content.wrappers, 2);
}

#[test]
fn a_wrapper_chain_that_never_ends_is_refused_rather_than_walked() {
    let mut payload = string_field(field::CONVERSATION, "deep");
    for _ in 0..(MAX_WRAPPERS + 2) {
        payload = bytes_field(
            field::DEVICE_SENT,
            &bytes_field(field::DEVICE_SENT_INNER, &payload),
        );
    }
    assert_eq!(
        derive_content(&payload),
        Err(ContentError::TooDeeplyWrapped)
    );
}

#[test]
fn a_wrapper_with_nothing_inside_is_reported() {
    let payload = bytes_field(field::DEVICE_SENT, &varint_field(9, 1));
    assert_eq!(
        derive_content(&payload),
        Err(ContentError::EmptyWrapper {
            number: field::DEVICE_SENT
        })
    );
}

// -- totality ----------------------------------------------------------------

#[test]
fn a_variant_this_build_does_not_model_still_crosses() {
    // The rule L0 already follows: a protocol change narrows what is
    // understood, never what is delivered.
    let payload = bytes_field(80, &varint_field(1, 1));
    let content = derive_content(&payload).expect("reads");

    assert_eq!(content.kind, MessageKind::Unmodelled(80));
    assert_eq!(content.kind.number(), Some(80));
    assert_eq!(content.text, None);
    assert_eq!(content.kind.to_string(), "unmodelled(80)");
}

#[test]
fn only_a_payload_with_no_fields_at_all_is_empty() {
    // `messageContextInfo = 35` is metadata rather than a variant, and this
    // reader cannot tell that from an unknown variant without the whole
    // schema. So it reports the number it saw rather than pretending to know
    // which of the two it was.
    let payload = bytes_field(35, &varint_field(1, 1));
    let content = derive_content(&payload).expect("reads");
    assert_eq!(content.kind, MessageKind::Unmodelled(35));
    assert_eq!(content.text, None);

    assert_eq!(
        derive_content(&[]).expect("an empty payload reads"),
        MessageContent {
            kind: MessageKind::Empty,
            text: None,
            wrappers: 0
        }
    );
}

#[test]
fn the_first_recognised_variant_wins() {
    // `waE2E.Message` declares no `oneof`, so a payload could carry two. The
    // first is the message; taking the last would report a wrapper's sibling.
    let mut payload = string_field(field::CONVERSATION, "first");
    payload.extend_from_slice(&bytes_field(
        field::EXTENDED_TEXT,
        &string_field(field::EXTENDED_TEXT_TEXT, "second"),
    ));

    let content = derive_content(&payload).expect("reads");
    assert_eq!(content.kind, MessageKind::Conversation);
    assert_eq!(content.text, Some("first"));
}

#[test]
fn a_malformed_payload_is_reported_rather_than_guessed_at() {
    // Truncated length prefix.
    let payload = [0x0a, 0x7f, 0x01];
    let error = derive_content(&payload).expect_err("must not read");
    assert!(matches!(error, ContentError::Malformed(_)));
    assert!(!error.to_string().is_empty());
}

#[test]
fn truncating_a_real_payload_anywhere_reports_rather_than_panics() {
    let real = string_field(field::CONVERSATION, "a message worth cutting");
    let payload = bytes_field(
        field::DEVICE_SENT,
        &bytes_field(field::DEVICE_SENT_INNER, &real),
    );
    for cut in 0..payload.len() {
        if let Err(error) = derive_content(&payload[..cut]) {
            assert!(!error.to_string().is_empty(), "cut {cut}");
        }
    }
    assert!(derive_content(&payload).is_ok());
}

#[test]
fn every_kind_names_itself_distinctly() {
    let all = [
        MessageKind::Conversation,
        MessageKind::ExtendedText,
        MessageKind::Image,
        MessageKind::Video,
        MessageKind::Document,
        MessageKind::Audio,
        MessageKind::Sticker,
        MessageKind::Contact,
        MessageKind::Location,
        MessageKind::Reaction,
        MessageKind::Protocol,
        MessageKind::Call,
        MessageKind::Empty,
    ];
    for (i, a) in all.iter().enumerate() {
        assert!(!a.name().is_empty());
        assert_eq!(a.to_string(), a.name());
        for b in all.iter().skip(i.saturating_add(1)) {
            assert_ne!(a.name(), b.name());
            assert_ne!(a, b);
            if let (Some(x), Some(y)) = (a.number(), b.number()) {
                assert_ne!(x, y, "{a} and {b} claim the same field number");
            }
        }
    }
}

#[test]
fn errors_are_std_errors_and_render() {
    fn assert_error<E: core::error::Error>(_: &E) {}
    assert_error(&ContentError::TooDeeplyWrapped);
    for error in [
        ContentError::TooDeeplyWrapped,
        ContentError::EmptyWrapper { number: 31 },
        ContentError::Malformed(wa_wire_proto::Error::MalformedVarint),
    ] {
        assert!(!error.to_string().is_empty());
    }
    assert!(!alloc::format!("{:?}", MessageKind::Empty).is_empty());
}
