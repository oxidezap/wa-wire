//! A decoded scalar, in whatever form the frame carried it.
//!
//! Nothing here owns anything. A token is a `&'static str` from the table, raw
//! bytes borrow from the frame, and the two forms that have no string in the
//! buffer at all — packed digit runs and JIDs — stay in parts and render on
//! demand. That is what lets the parser touch a 433 KB stanza without
//! allocating.

use core::fmt;

use crate::jid::Jid;
use crate::packed::Packed;

/// A scalar read from a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    /// An absent value.
    Nil,
    /// A token resolved through the table.
    Token(&'a str),
    /// Raw bytes. Text in an attribute position, opaque in a content position.
    Bytes(&'a [u8]),
    /// A packed nibble or hex run.
    Packed(Packed<'a>),
    /// A JID, held as parts.
    Jid(Jid<'a>),
}

impl<'a> Value<'a> {
    /// Whether this is the absent value.
    #[must_use]
    pub const fn is_nil(self) -> bool {
        matches!(self, Self::Nil)
    }

    /// The value as a borrowed string, when one exists in the frame already.
    ///
    /// Returns `None` for packed runs and JIDs — not because they are not
    /// textual, but because their text exists nowhere to borrow from. Use
    /// [`eq_str`](Self::eq_str) to compare them, or `Display` to render them.
    #[must_use]
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            Self::Token(token) => Some(token),
            Self::Bytes(bytes) => core::str::from_utf8(bytes).ok(),
            Self::Nil | Self::Packed(_) | Self::Jid(_) => None,
        }
    }

    /// The value's bytes, when it is raw.
    #[must_use]
    pub const fn as_bytes(self) -> Option<&'a [u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// The JID, if this is one.
    #[must_use]
    pub const fn as_jid(self) -> Option<Jid<'a>> {
        match self {
            Self::Jid(jid) => Some(jid),
            _ => None,
        }
    }

    /// The packed run, if this is one.
    #[must_use]
    pub const fn as_packed(self) -> Option<Packed<'a>> {
        match self {
            Self::Packed(packed) => Some(packed),
            _ => None,
        }
    }

    /// Whether the value renders as exactly `other`, without building a string.
    ///
    /// This is the comparison L1 derivation actually needs — `attrs["type"] ==
    /// "text"` and the like — so it must not allocate.
    #[must_use]
    pub fn eq_str(self, other: &str) -> bool {
        match self {
            Self::Nil => false,
            Self::Token(token) => token == other,
            Self::Bytes(bytes) => bytes == other.as_bytes(),
            Self::Packed(packed) => packed.eq_str(other),
            Self::Jid(jid) => {
                let mut compare = StrEq::new(other);
                // Writing into the comparator cannot fail; a mismatch just
                // stops it matching.
                let _ = fmt::write(&mut compare, format_args!("{jid}"));
                compare.finished()
            }
        }
    }

    /// Whether two values mean the same thing, whatever form each arrived in.
    ///
    /// This is the comparison a conformance run needs. Two engines can encode
    /// one value differently and both be right — a token here, the same text as
    /// bytes there — and calling that a divergence would bury the real ones.
    #[must_use]
    pub fn semantic_eq(self, other: Value<'_>) -> bool {
        match (self, other) {
            (Self::Nil, Value::Nil) => true,
            (Self::Jid(a), Value::Jid(b)) => a.semantic_eq(b),
            (Self::Packed(a), Value::Packed(b)) => a.semantic_eq(b),
            (Self::Packed(packed), textual) => {
                textual.as_str().is_some_and(|text| packed.eq_str(text))
            }
            (textual, Value::Packed(packed)) => {
                textual.as_str().is_some_and(|text| packed.eq_str(text))
            }
            // A JID compared against text renders once and matches or does not;
            // `eq_str` walks the parts, so nothing is built.
            (Self::Jid(jid), textual) => textual
                .as_str()
                .is_some_and(|text| Value::Jid(jid).eq_str(text)),
            (textual, Value::Jid(jid)) => textual
                .as_str()
                .is_some_and(|text| Value::Jid(jid).eq_str(text)),
            (a, b) => match (a.as_str(), b.as_str()) {
                (Some(x), Some(y)) => x == y,
                // Two byte strings that are not valid text still compare, since
                // being unreadable does not make them unequal.
                _ => match (a.as_bytes(), b.as_bytes()) {
                    (Some(x), Some(y)) => x == y,
                    _ => false,
                },
            },
        }
    }

    /// Whether the value is textual in the sense the protocol means: anything
    /// that appears in an attribute position.
    #[must_use]
    pub const fn is_textual(self) -> bool {
        matches!(
            self,
            Self::Token(_) | Self::Bytes(_) | Self::Packed(_) | Self::Jid(_)
        )
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => Ok(()),
            Self::Token(token) => f.write_str(token),
            Self::Bytes(bytes) => match core::str::from_utf8(bytes) {
                Ok(text) => f.write_str(text),
                Err(_) => f.write_str("\u{FFFD}"),
            },
            Self::Packed(packed) => write!(f, "{packed}"),
            Self::Jid(jid) => write!(f, "{jid}"),
        }
    }
}

/// Compares a `Display` rendering against a string as it is written, so a JID
/// can be matched without being built.
struct StrEq<'a> {
    expected: &'a str,
    matched: bool,
}

impl<'a> StrEq<'a> {
    const fn new(expected: &'a str) -> Self {
        Self {
            expected,
            matched: true,
        }
    }

    /// Whether everything written matched and nothing was left over.
    const fn finished(&self) -> bool {
        self.matched && self.expected.is_empty()
    }
}

impl fmt::Write for StrEq<'_> {
    fn write_str(&mut self, chunk: &str) -> fmt::Result {
        if !self.matched {
            return Ok(());
        }
        match self.expected.strip_prefix(chunk) {
            Some(rest) => self.expected = rest,
            None => self.matched = false,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    use crate::jid::User;
    use crate::packed::Alphabet;
    use crate::token::{SERVER_LID, SERVER_PN};

    fn packed() -> Packed<'static> {
        Packed::new(Alphabet::Nibble, &[0x12, 0x34], false)
    }

    fn jid() -> Jid<'static> {
        Jid::with_device(User::Token("5511"), SERVER_PN, 3)
    }

    #[test]
    fn nil_is_absent_and_matches_nothing() {
        let value = Value::Nil;
        assert!(value.is_nil());
        assert!(!value.is_textual());
        assert_eq!(value.as_str(), None);
        assert_eq!(value.as_bytes(), None);
        assert_eq!(value.as_jid(), None);
        assert_eq!(value.as_packed(), None);
        assert_eq!(value.to_string(), "");
        assert!(!value.eq_str(""), "nil is absent, not the empty string");
    }

    #[test]
    fn tokens_borrow_from_the_table() {
        let value = Value::Token("message");
        assert!(!value.is_nil());
        assert!(value.is_textual());
        assert_eq!(value.as_str(), Some("message"));
        assert_eq!(value.as_bytes(), None);
        assert_eq!(value.to_string(), "message");
        assert!(value.eq_str("message"));
        assert!(!value.eq_str("receipt"));
    }

    #[test]
    fn bytes_read_as_text_when_they_are_valid_utf8() {
        let value = Value::Bytes(b"hello");
        assert_eq!(value.as_str(), Some("hello"));
        assert_eq!(value.as_bytes(), Some(&b"hello"[..]));
        assert_eq!(value.to_string(), "hello");
        assert!(value.eq_str("hello"));
        assert!(!value.eq_str("hell"));
    }

    #[test]
    fn invalid_utf8_stays_readable_as_bytes() {
        let value = Value::Bytes(&[0xff, 0xfe]);
        assert_eq!(value.as_str(), None, "not valid text");
        assert_eq!(value.as_bytes(), Some(&[0xff, 0xfe][..]));
        assert_eq!(value.to_string(), "\u{FFFD}");
        assert!(value.is_textual());
    }

    #[test]
    fn packed_runs_have_no_borrowable_string_but_still_compare() {
        let value = Value::Packed(packed());
        assert_eq!(
            value.as_str(),
            None,
            "the digits exist nowhere in the buffer"
        );
        assert_eq!(value.as_packed(), Some(packed()));
        assert_eq!(value.to_string(), "1234");
        assert!(value.eq_str("1234"));
        assert!(!value.eq_str("123"));
        assert!(value.is_textual());
    }

    #[test]
    fn jids_compare_without_being_built() {
        let value = Value::Jid(jid());
        assert_eq!(value.as_str(), None);
        assert_eq!(value.as_jid(), Some(jid()));
        assert_eq!(value.to_string(), "5511:3@s.whatsapp.net");
        assert!(value.eq_str("5511:3@s.whatsapp.net"));
        assert!(!value.eq_str("5511@s.whatsapp.net"), "device differs");
        assert!(!value.eq_str("5511:3@lid"), "server differs");
        assert!(!value.eq_str("5511:3@s.whatsapp.ne"), "truncated");
        assert!(
            !value.eq_str("5511:3@s.whatsapp.nett"),
            "expected string left over"
        );
        assert!(!value.eq_str(""));
    }

    #[test]
    fn jid_comparison_handles_a_bare_server() {
        let value = Value::Jid(Jid::pair(User::None, SERVER_LID));
        assert!(value.eq_str("lid"));
        assert!(!value.eq_str("@lid"));
    }

    #[test]
    fn a_mismatch_early_in_a_jid_does_not_match_later() {
        // Once the comparator diverges it must stay diverged, even if a later
        // chunk would have lined up.
        let value = Value::Jid(jid());
        assert!(!value.eq_str("XXXX:3@s.whatsapp.net"));
    }

    #[test]
    fn accessors_reject_the_wrong_variant() {
        assert_eq!(Value::Token("t").as_jid(), None);
        assert_eq!(Value::Token("t").as_packed(), None);
        assert_eq!(Value::Jid(jid()).as_bytes(), None);
        assert_eq!(Value::Packed(packed()).as_bytes(), None);
        assert_eq!(Value::Packed(packed()).as_jid(), None);
        assert_eq!(Value::Jid(jid()).as_packed(), None);
    }

    #[test]
    fn semantic_equality_looks_past_the_encoding() {
        // One engine sends a token, another the same text as bytes. Both right.
        assert!(Value::Token("read").semantic_eq(Value::Bytes(b"read")));
        assert!(Value::Bytes(b"read").semantic_eq(Value::Token("read")));
        assert!(!Value::Token("read").semantic_eq(Value::Bytes(b"delivery")));

        // A packed digit run against the same digits as text.
        let digits = Packed::new(Alphabet::Nibble, &[0x12, 0x34], false);
        assert!(Value::Packed(digits).semantic_eq(Value::Token("1234")));
        assert!(Value::Token("1234").semantic_eq(Value::Packed(digits)));
        assert!(Value::Packed(digits).semantic_eq(Value::Bytes(b"1234")));
        assert!(!Value::Packed(digits).semantic_eq(Value::Token("1235")));

        // A JID against its rendering.
        assert!(Value::Jid(jid()).semantic_eq(Value::Bytes(b"5511:3@s.whatsapp.net")));
        assert!(Value::Bytes(b"5511:3@s.whatsapp.net").semantic_eq(Value::Jid(jid())));
        assert!(!Value::Jid(jid()).semantic_eq(Value::Bytes(b"5511@s.whatsapp.net")));
    }

    #[test]
    fn semantic_equality_is_reflexive_across_every_variant() {
        for value in [
            Value::Nil,
            Value::Token("t"),
            Value::Bytes(b"b"),
            Value::Packed(packed()),
            Value::Jid(jid()),
        ] {
            assert!(value.semantic_eq(value), "{value:?} differs from itself");
        }
    }

    #[test]
    fn nil_is_not_semantically_anything_else() {
        // Absent is not the empty string, and not an empty digit run either.
        for other in [
            Value::Token(""),
            Value::Bytes(b""),
            Value::Packed(Packed::new(Alphabet::Nibble, &[], false)),
            Value::Jid(jid()),
        ] {
            assert!(!Value::Nil.semantic_eq(other), "{other:?}");
            assert!(!other.semantic_eq(Value::Nil), "{other:?}");
        }
    }

    #[test]
    fn unreadable_bytes_still_compare_to_themselves() {
        // Not valid text, so `as_str` gives nothing — but identical bytes are
        // not a divergence.
        let invalid = Value::Bytes(&[0xff, 0xfe]);
        assert!(invalid.semantic_eq(Value::Bytes(&[0xff, 0xfe])));
        assert!(!invalid.semantic_eq(Value::Bytes(&[0xff, 0xfd])));
        assert!(!invalid.semantic_eq(Value::Token("x")));
        assert!(!invalid.semantic_eq(Value::Packed(packed())));
    }

    #[test]
    fn values_are_comparable() {
        assert_eq!(Value::Token("a"), Value::Token("a"));
        assert_ne!(Value::Token("a"), Value::Token("b"));
        assert_ne!(Value::Token("a"), Value::Bytes(b"a"));
        assert!(!alloc::format!("{:?}", Value::Nil).is_empty());
    }
}
