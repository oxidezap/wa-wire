//! The primitives a generated parser is built from.
//!
//! Written by hand; the generated code only chooses which of these to call and
//! in what order. That split is what keeps the generator emitting *structure*
//! rather than logic — a protocol change moves the shape file and the calls,
//! never the extraction rules.
//!
//! Nothing here allocates. A string field stays a [`Value`], because the text
//! of a packed run or a JID exists nowhere in the frame to borrow; comparing or
//! rendering it is the consumer's call, and both are allocation-free.

use wa_wire_codec::{Jid, NodeRef, Packed, User, Value};

use crate::error::{DeriveError, Field};

/// A required attribute.
pub fn attr_string<'a>(node: &NodeRef<'a>, key: &'static str) -> Result<Value<'a>, DeriveError> {
    node.attr(key).ok_or(DeriveError::MissingAttr { key })
}

/// An attribute that may be absent.
#[must_use]
pub fn maybe_attr_string<'a>(node: &NodeRef<'a>, key: &'static str) -> Option<Value<'a>> {
    node.attr(key)
}

/// A required integer attribute.
pub fn attr_int(node: &NodeRef<'_>, key: &'static str) -> Result<i64, DeriveError> {
    let value = attr_string(node, key)?;
    parse_int(value, key)
}

/// An integer attribute that may be absent.
///
/// An unparsable value is an error rather than `None`: "not a number" and "not
/// there" are different faults, and collapsing them hides a protocol change.
pub fn maybe_attr_int(node: &NodeRef<'_>, key: &'static str) -> Result<Option<i64>, DeriveError> {
    match node.attr(key) {
        Some(value) => parse_int(value, key).map(Some),
        None => Ok(None),
    }
}

/// A required timestamp attribute, in seconds.
pub fn attr_time(node: &NodeRef<'_>, key: &'static str) -> Result<i64, DeriveError> {
    attr_int(node, key)
}

/// A timestamp attribute that may be absent.
pub fn maybe_attr_time(node: &NodeRef<'_>, key: &'static str) -> Result<Option<i64>, DeriveError> {
    maybe_attr_int(node, key)
}

/// A required JID-valued attribute.
pub fn attr_jid<'a>(node: &NodeRef<'a>, key: &'static str) -> Result<Jid<'a>, DeriveError> {
    let value = attr_string(node, key)?;
    parse_jid(value, key)
}

/// A JID-valued attribute that may be absent.
pub fn maybe_attr_jid<'a>(
    node: &NodeRef<'a>,
    key: &'static str,
) -> Result<Option<Jid<'a>>, DeriveError> {
    match node.attr(key) {
        Some(value) => parse_jid(value, key).map(Some),
        None => Ok(None),
    }
}

/// A JID, however the encoder chose to write it.
///
/// The wire has a dedicated JID form, and an encoder may use it — or write the
/// same JID as a token or as bytes, which is just as valid and is what a second
/// implementation was observed doing for `s.whatsapp.net`. Reading only the
/// dedicated form meant one engine derived an event where another derived
/// nothing from the identical stanza: a conformance fault caused by the
/// derivation, not by either engine.
fn parse_jid<'a>(value: Value<'a>, key: &'static str) -> Result<Jid<'a>, DeriveError> {
    if let Some(jid) = value.as_jid() {
        return Ok(jid);
    }
    let text = value.as_str().ok_or(DeriveError::NotAJid { key })?;
    // A bare server is only read as one when the wire wrote it as a token.
    // Servers are dictionary entries, so a token is evidence; arbitrary bytes
    // with no `@` are just a string, and reading those as JIDs would turn the
    // field into "anything at all".
    let bare_server_allowed = matches!(value, Value::Token(_));
    jid_from_text(text, bare_server_allowed).ok_or(DeriveError::NotAJid { key })
}

/// Read `user@server`, `user:device@server`, or — when `bare_server_allowed` —
/// a lone `server`.
///
/// Rejects anything without a server part rather than inventing one: a field
/// the spec calls a JID holding something that is not one is a protocol change
/// worth reporting, not worth papering over.
fn jid_from_text(text: &str, bare_server_allowed: bool) -> Option<Jid<'_>> {
    let (user, server) = match text.split_once('@') {
        Some((user, server)) => (Some(user), server),
        None if bare_server_allowed => (None, text),
        None => return None,
    };
    if server.is_empty() || server.contains('@') {
        return None;
    }

    let Some(user) = user else {
        return Some(Jid::pair(User::None, server));
    };
    // `user:device` splits the device off; `user` alone is device zero.
    match user.split_once(':') {
        Some((user, device)) => {
            let device = device.parse::<u16>().ok()?;
            (!user.is_empty())
                .then(|| Jid::with_device(User::Bytes(user.as_bytes()), server, device))
        }
        None => (!user.is_empty()).then(|| Jid::pair(User::Bytes(user.as_bytes()), server)),
    }
}

/// A required enum-valued attribute, resolved by the caller's `from_wire`.
pub fn attr_enum<'a, T>(
    node: &NodeRef<'a>,
    key: &'static str,
    from_wire: fn(Value<'a>) -> Option<T>,
) -> Result<T, DeriveError> {
    let value = attr_string(node, key)?;
    from_wire(value).ok_or(DeriveError::UnknownEnumValue { key })
}

/// An enum-valued attribute that may be absent.
pub fn maybe_attr_enum<'a, T>(
    node: &NodeRef<'a>,
    key: &'static str,
    from_wire: fn(Value<'a>) -> Option<T>,
) -> Result<Option<T>, DeriveError> {
    match node.attr(key) {
        Some(value) => from_wire(value)
            .ok_or(DeriveError::UnknownEnumValue { key })
            .map(Some),
        None => Ok(None),
    }
}

/// An enum-valued attribute whose unknown values are dropped rather than
/// rejected — the wire has variants this build does not know, by design.
#[must_use]
pub fn attr_enum_or_none<'a, T>(
    node: &NodeRef<'a>,
    key: &'static str,
    from_wire: fn(Value<'a>) -> Option<T>,
) -> Option<T> {
    node.attr(key).and_then(from_wire)
}

/// A required child node.
pub fn child<'a>(node: &NodeRef<'a>, tag: &'static str) -> Result<NodeRef<'a>, DeriveError> {
    node.child(tag).ok_or(DeriveError::MissingChild { tag })
}

/// A child node that may be absent.
#[must_use]
pub fn maybe_child<'a>(node: &NodeRef<'a>, tag: &'static str) -> Option<NodeRef<'a>> {
    node.child(tag)
}

/// Every child carrying `tag`, in document order.
pub fn children_with_tag<'a>(
    node: &NodeRef<'a>,
    tag: &'static str,
) -> impl Iterator<Item = NodeRef<'a>> + use<'a> {
    node.children().filter(move |child| child.tag().eq_str(tag))
}

/// A node's raw byte body.
pub fn content_bytes<'a>(node: &NodeRef<'a>) -> Result<&'a [u8], DeriveError> {
    node.content()
        .as_bytes()
        .ok_or(DeriveError::MissingContent {
            field: Field::Bytes,
        })
}

/// A node's body read as an unsigned integer.
pub fn content_uint(node: &NodeRef<'_>) -> Result<u64, DeriveError> {
    let bytes = content_bytes(node)?;
    // WhatsApp writes these as big-endian, shortest-form integers.
    let mut value: u64 = 0;
    for byte in bytes.iter().take(8) {
        value = (value << 8) | u64::from(*byte);
    }
    if bytes.len() > 8 {
        return Err(DeriveError::ContentTooWide { len: bytes.len() });
    }
    Ok(value)
}

fn parse_int(value: Value<'_>, key: &'static str) -> Result<i64, DeriveError> {
    if let Some(text) = value.as_str() {
        return text
            .parse::<i64>()
            .map_err(|_| DeriveError::NotAnInt { key });
    }
    // A run of digits is exactly what the nibble alphabet exists to compress,
    // so any encoder that packs will pack a timestamp — and reading only
    // `as_str` here meant every packed integer in real traffic failed to
    // derive. Read the digits out rather than materialising a string: this
    // crate is `no_std` and the value is at most twenty of them.
    if let Some(packed) = value.as_packed() {
        return packed_int(packed).ok_or(DeriveError::NotAnInt { key });
    }
    Err(DeriveError::NotAnInt { key })
}

/// A packed run read as a decimal integer, or `None` if it is not one.
///
/// The nibble alphabet also carries `-`, `.` and a couple of others, so a
/// packed run is not necessarily a number — a phone number with a leading `+`
/// is the obvious case.
fn packed_int(packed: Packed<'_>) -> Option<i64> {
    let mut value: i64 = 0;
    let mut digits = 0u32;
    for character in packed.chars() {
        let digit = character.to_digit(10)?;
        value = value.checked_mul(10)?.checked_add(i64::from(digit))?;
        digits = digits.saturating_add(1);
    }
    (digits > 0).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;

    use crate::testing::{Fixture, parse};

    fn node(fixture: &Fixture) -> NodeRef<'_> {
        parse(fixture)
    }

    #[test]
    fn a_packed_integer_reads_as_a_number() {
        // The bug this covers: an encoder packs a run of digits into nibbles —
        // which is what the alphabet is for — and the extractor only knew how
        // to read strings. Every packed timestamp in real traffic failed to
        // derive, which took `<message>` and `<call>` with it.
        let fixture = Fixture::node("message")
            .packed_attr("t", "1700000010")
            .build();
        let node = node(&fixture);

        assert_eq!(attr_int(&node, "t"), Ok(1_700_000_010));
        assert_eq!(attr_time(&node, "t"), Ok(1_700_000_010));
    }

    #[test]
    fn a_packed_run_that_is_not_a_number_is_reported_as_such() {
        // The nibble alphabet carries `-` and `.` too, so packed does not imply
        // numeric. Saying "not an int" is right; guessing a number is not.
        let fixture = Fixture::node("message")
            .packed_attr("t", "55-11-999")
            .build();
        let node = node(&fixture);

        assert_eq!(
            attr_int(&node, "t"),
            Err(DeriveError::NotAnInt { key: "t" })
        );
    }

    #[test]
    fn an_unparsable_string_is_still_an_error_not_a_zero() {
        let fixture = Fixture::node("message").attr("t", "later").build();
        let node = node(&fixture);

        assert_eq!(
            attr_int(&node, "t"),
            Err(DeriveError::NotAnInt { key: "t" })
        );
    }

    #[test]
    fn required_attributes_are_read_and_missing_ones_reported() {
        let fixture = Fixture::node("receipt").attr("id", "ABC").build();
        let node = node(&fixture);

        assert_eq!(attr_string(&node, "id").map(Value::as_str), Ok(Some("ABC")));
        assert_eq!(
            attr_string(&node, "type"),
            Err(DeriveError::MissingAttr { key: "type" })
        );
    }

    #[test]
    fn optional_attributes_report_absence_without_failing() {
        let fixture = Fixture::node("receipt").attr("id", "ABC").build();
        let node = node(&fixture);

        assert!(maybe_attr_string(&node, "id").is_some());
        assert_eq!(maybe_attr_string(&node, "nope"), None);
    }

    #[test]
    fn integers_parse_and_report_both_failure_kinds_apart() {
        let fixture = Fixture::node("receipt")
            .attr("t", "1700000000")
            .attr("neg", "-5")
            .attr("bad", "not-a-number")
            .build();
        let node = node(&fixture);

        assert_eq!(attr_int(&node, "t"), Ok(1_700_000_000));
        assert_eq!(attr_int(&node, "neg"), Ok(-5));
        assert_eq!(attr_time(&node, "t"), Ok(1_700_000_000));

        // Absent and unparsable are different faults.
        assert_eq!(maybe_attr_int(&node, "nope"), Ok(None));
        assert_eq!(
            maybe_attr_int(&node, "bad"),
            Err(DeriveError::NotAnInt { key: "bad" })
        );
        assert_eq!(
            attr_int(&node, "bad"),
            Err(DeriveError::NotAnInt { key: "bad" })
        );
        assert_eq!(
            attr_int(&node, "nope"),
            Err(DeriveError::MissingAttr { key: "nope" })
        );
        assert_eq!(maybe_attr_time(&node, "t"), Ok(Some(1_700_000_000)));
    }

    #[test]
    fn jid_attributes_are_read_as_jids() {
        let fixture = Fixture::node("receipt").jid_attr("from", "user").build();
        let node = node(&fixture);

        let jid = attr_jid(&node, "from").expect("a jid");
        assert_eq!(jid.server(), "s.whatsapp.net");
        assert!(maybe_attr_jid(&node, "from").expect("ok").is_some());
        assert_eq!(maybe_attr_jid(&node, "nope"), Ok(None));
        assert_eq!(
            attr_jid(&node, "nope"),
            Err(DeriveError::MissingAttr { key: "nope" })
        );
    }

    #[test]
    fn a_non_jid_in_a_jid_slot_is_rejected() {
        let fixture = Fixture::node("receipt").attr("from", "plain-text").build();
        let node = node(&fixture);

        assert_eq!(
            attr_jid(&node, "from"),
            Err(DeriveError::NotAJid { key: "from" })
        );
        assert_eq!(
            maybe_attr_jid(&node, "from"),
            Err(DeriveError::NotAJid { key: "from" })
        );
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Kind {
        Read,
        Delivery,
    }

    fn kind_from_wire(value: Value<'_>) -> Option<Kind> {
        if value.eq_str("read") {
            Some(Kind::Read)
        } else if value.eq_str("delivery") {
            Some(Kind::Delivery)
        } else {
            None
        }
    }

    #[test]
    fn enums_resolve_and_unknown_values_are_rejected() {
        let fixture = Fixture::node("receipt")
            .attr("type", "read")
            .attr("other", "invented")
            .build();
        let node = node(&fixture);

        assert_eq!(attr_enum(&node, "type", kind_from_wire), Ok(Kind::Read));
        assert_eq!(
            maybe_attr_enum(&node, "type", kind_from_wire),
            Ok(Some(Kind::Read))
        );
        assert_eq!(maybe_attr_enum(&node, "nope", kind_from_wire), Ok(None));

        assert_eq!(
            attr_enum(&node, "other", kind_from_wire),
            Err(DeriveError::UnknownEnumValue { key: "other" })
        );
        assert_eq!(
            maybe_attr_enum(&node, "other", kind_from_wire),
            Err(DeriveError::UnknownEnumValue { key: "other" })
        );
        assert_eq!(
            attr_enum(&node, "nope", kind_from_wire),
            Err(DeriveError::MissingAttr { key: "nope" })
        );
    }

    #[test]
    fn the_lenient_enum_reader_drops_what_it_does_not_know() {
        // Some slots carry variants a build predates; rejecting there would
        // fail a whole stanza over a field nobody reads.
        let fixture = Fixture::node("receipt")
            .attr("type", "read")
            .attr("other", "invented")
            .build();
        let node = node(&fixture);

        assert_eq!(
            attr_enum_or_none(&node, "type", kind_from_wire),
            Some(Kind::Read)
        );
        assert_eq!(attr_enum_or_none(&node, "other", kind_from_wire), None);
        assert_eq!(attr_enum_or_none(&node, "nope", kind_from_wire), None);
    }

    #[test]
    fn children_are_found_by_tag() {
        let fixture = Fixture::node("receipt")
            .child(Fixture::node("error").attr("code", "403"))
            .child(Fixture::node("user").attr("n", "1"))
            .child(Fixture::node("user").attr("n", "2"))
            .build();
        let node = node(&fixture);

        assert!(child(&node, "error").is_ok());
        assert!(maybe_child(&node, "error").is_some());
        assert_eq!(maybe_child(&node, "nope"), None);
        assert_eq!(
            child(&node, "nope"),
            Err(DeriveError::MissingChild { tag: "nope" })
        );

        let users: Vec<_> = children_with_tag(&node, "user").collect();
        assert_eq!(users.len(), 2);
        assert!(users[0].attr_eq("n", "1"));
        assert!(users[1].attr_eq("n", "2"));
        assert_eq!(children_with_tag(&node, "nope").count(), 0);
    }

    #[test]
    fn byte_bodies_are_borrowed_and_absent_ones_reported() {
        let with_body = Fixture::node("enc").bytes(b"cipher").build();
        assert_eq!(content_bytes(&node(&with_body)), Ok(&b"cipher"[..]));

        let without = Fixture::node("enc").build();
        assert_eq!(
            content_bytes(&node(&without)),
            Err(DeriveError::MissingContent {
                field: Field::Bytes
            })
        );
    }

    #[test]
    fn unsigned_bodies_decode_big_endian() {
        for (bytes, expected) in [
            (&[0x01][..], 1u64),
            (&[0x01, 0x00], 256),
            (&[0xFF, 0xFF], 65_535),
            (&[0x00, 0x00, 0x00, 0x01], 1),
        ] {
            let fixture = Fixture::node("count").bytes(bytes).build();
            assert_eq!(content_uint(&node(&fixture)), Ok(expected), "{bytes:?}");
        }

        let empty = Fixture::node("count").bytes(b"").build();
        assert_eq!(content_uint(&node(&empty)), Ok(0));
    }

    #[test]
    fn an_oversized_unsigned_body_is_rejected_rather_than_truncated() {
        let fixture = Fixture::node("count").bytes(&[0xFF; 9]).build();
        assert_eq!(
            content_uint(&node(&fixture)),
            Err(DeriveError::ContentTooWide { len: 9 })
        );
    }
}
