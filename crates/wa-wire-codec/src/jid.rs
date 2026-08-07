//! JIDs, kept in parts rather than joined.
//!
//! A JID renders as `user@server` or `user:device@server`, but no such string
//! exists in the frame — the parts arrive as separate tokens. Joining them
//! would mean allocating, so a [`Jid`] holds the parts and renders on demand.
//! Comparison and field access work without building anything.

use core::fmt;

use crate::packed::Packed;
use crate::token::{SERVER_INTEROP, SERVER_MSGR};

/// The user portion of a JID, in whichever form the frame carried it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum User<'a> {
    /// No user part; the JID is a bare server.
    None,
    /// A dictionary token.
    Token(&'a str),
    /// A packed digit run — the usual case for phone numbers.
    Packed(Packed<'a>),
    /// Raw bytes, to be read as UTF-8.
    Bytes(&'a [u8]),
}

impl<'a> User<'a> {
    /// Whether there is no user part.
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Whether two user parts name the same user, whatever form each arrived
    /// in.
    ///
    /// One engine may hand back a packed digit run where another hands back the
    /// same digits as bytes. They name the same user, and a conformance run
    /// that called them different would report a divergence that is not one.
    #[must_use]
    pub fn semantic_eq(self, other: User<'_>) -> bool {
        match (self, other) {
            (Self::None, User::None) => true,
            (Self::Packed(a), User::Packed(b)) => a.semantic_eq(b),
            // One rule, whichever side the packed run is on.
            (Self::Packed(packed), rendered) => packed_eq_text(packed, rendered.as_text()),
            #[allow(clippy::match_same_arms)]
            (rendered, User::Packed(packed)) => packed_eq_text(packed, rendered.as_text()),
            (a, b) => match (a.as_text(), b.as_text()) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            },
        }
    }

    /// The user as borrowed text, when it exists in that form.
    #[must_use]
    fn as_text(self) -> Option<&'a str> {
        match self {
            Self::None => Some(""),
            Self::Token(text) => Some(text),
            Self::Bytes(bytes) => core::str::from_utf8(bytes).ok(),
            Self::Packed(_) => None,
        }
    }

    /// Whether the user renders as exactly `other`.
    #[must_use]
    pub fn eq_str(self, other: &str) -> bool {
        match self {
            Self::None => other.is_empty(),
            Self::Token(token) => token == other,
            Self::Packed(packed) => packed.eq_str(other),
            Self::Bytes(bytes) => bytes == other.as_bytes(),
        }
    }
}

/// A packed run against text, when the other side had any.
///
/// An absent user is not the empty digit run: one names no user at all, the
/// other names a user whose digits happen to be none.
fn packed_eq_text(packed: Packed<'_>, text: Option<&str>) -> bool {
    text.is_some_and(|text| !text.is_empty() && packed.eq_str(text))
}

impl fmt::Display for User<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Token(token) => f.write_str(token),
            Self::Packed(packed) => write!(f, "{packed}"),
            // Invalid UTF-8 cannot be rendered, and silently dropping it would
            // be worse than showing the replacement character.
            Self::Bytes(bytes) => match core::str::from_utf8(bytes) {
                Ok(text) => f.write_str(text),
                Err(_) => f.write_str("\u{FFFD}"),
            },
        }
    }
}

/// A JID, held as parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jid<'a> {
    user: User<'a>,
    server: &'a str,
    device: u16,
    /// Only interop JIDs have one.
    integrator: Option<u16>,
}

impl<'a> Jid<'a> {
    /// A `user@server` JID.
    #[must_use]
    pub const fn pair(user: User<'a>, server: &'a str) -> Self {
        Self {
            user,
            server,
            device: 0,
            integrator: None,
        }
    }

    /// A user JID with an explicit device.
    #[must_use]
    pub const fn with_device(user: User<'a>, server: &'a str, device: u16) -> Self {
        Self {
            user,
            server,
            device,
            integrator: None,
        }
    }

    /// An interop JID, which carries an integrator alongside the device.
    #[must_use]
    pub const fn interop(user: User<'a>, device: u16, integrator: u16) -> Self {
        Self {
            user,
            server: SERVER_INTEROP,
            device,
            integrator: Some(integrator),
        }
    }

    /// A Messenger JID.
    #[must_use]
    pub const fn messenger(user: User<'a>, device: u16) -> Self {
        Self {
            user,
            server: SERVER_MSGR,
            device,
            integrator: None,
        }
    }

    /// The user part.
    #[must_use]
    pub const fn user(self) -> User<'a> {
        self.user
    }

    /// The server part.
    #[must_use]
    pub const fn server(self) -> &'a str {
        self.server
    }

    /// The device number; zero means the primary device.
    #[must_use]
    pub const fn device(self) -> u16 {
        self.device
    }

    /// The integrator, for interop JIDs.
    #[must_use]
    pub const fn integrator(self) -> Option<u16> {
        self.integrator
    }

    /// Whether this JID names a server with no user.
    #[must_use]
    pub const fn is_server_only(self) -> bool {
        self.user.is_none()
    }

    /// Whether two JIDs name the same address, whatever form each part arrived
    /// in.
    #[must_use]
    pub fn semantic_eq(self, other: Jid<'_>) -> bool {
        self.server == other.server
            && self.device == other.device
            && self.integrator == other.integrator
            && self.user.semantic_eq(other.user)
    }
}

impl fmt::Display for Jid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A bare server is written alone: `s.whatsapp.net`, not `@s.whatsapp.net`.
        if self.user.is_none() && self.integrator.is_none() {
            return f.write_str(self.server);
        }
        if let Some(integrator) = self.integrator {
            write!(f, "{integrator}-")?;
        }
        write!(f, "{}", self.user)?;
        if self.device != 0 || self.integrator.is_some() {
            write!(f, ":{}", self.device)?;
        }
        write!(f, "@{}", self.server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    use crate::packed::Alphabet;
    use crate::token::{SERVER_LID, SERVER_PN};

    #[test]
    fn a_bare_server_renders_without_an_at_sign() {
        let jid = Jid::pair(User::None, SERVER_PN);
        assert_eq!(jid.to_string(), "s.whatsapp.net");
        assert!(jid.is_server_only());
        assert!(jid.user().is_none());
        assert_eq!(jid.device(), 0);
        assert_eq!(jid.integrator(), None);
    }

    #[test]
    fn a_user_pair_renders_with_the_server() {
        let jid = Jid::pair(User::Token("5511999998888"), SERVER_PN);
        assert_eq!(jid.to_string(), "5511999998888@s.whatsapp.net");
        assert!(!jid.is_server_only());
        assert_eq!(jid.server(), SERVER_PN);
    }

    #[test]
    fn a_packed_user_renders_through_its_digits() {
        let packed = Packed::new(Alphabet::Nibble, &[0x55, 0x11], false);
        let jid = Jid::pair(User::Packed(packed), SERVER_LID);
        assert_eq!(jid.to_string(), "5511@lid");
    }

    #[test]
    fn a_device_is_written_only_when_present() {
        let primary = Jid::with_device(User::Token("55119"), SERVER_PN, 0);
        assert_eq!(primary.to_string(), "55119@s.whatsapp.net");

        let companion = Jid::with_device(User::Token("55119"), SERVER_PN, 33);
        assert_eq!(companion.to_string(), "55119:33@s.whatsapp.net");
        assert_eq!(companion.device(), 33);
    }

    #[test]
    fn interop_jids_carry_an_integrator_and_always_show_the_device() {
        let jid = Jid::interop(User::Token("user"), 0, 42);
        assert_eq!(jid.to_string(), "42-user:0@interop");
        assert_eq!(jid.integrator(), Some(42));
        assert_eq!(jid.server(), SERVER_INTEROP);

        let with_device = Jid::interop(User::Token("user"), 7, 42);
        assert_eq!(with_device.to_string(), "42-user:7@interop");
    }

    #[test]
    fn messenger_jids_use_the_msgr_server() {
        let jid = Jid::messenger(User::Token("abc"), 3);
        assert_eq!(jid.to_string(), "abc:3@msgr");
        assert_eq!(jid.server(), SERVER_MSGR);

        let primary = Jid::messenger(User::Token("abc"), 0);
        assert_eq!(primary.to_string(), "abc@msgr");
    }

    #[test]
    fn user_variants_compare_against_strings() {
        assert!(User::None.eq_str(""));
        assert!(!User::None.eq_str("x"));

        assert!(User::Token("abc").eq_str("abc"));
        assert!(!User::Token("abc").eq_str("abd"));

        let packed = Packed::new(Alphabet::Nibble, &[0x12], false);
        assert!(User::Packed(packed).eq_str("12"));
        assert!(!User::Packed(packed).eq_str("13"));

        assert!(User::Bytes(b"raw").eq_str("raw"));
        assert!(!User::Bytes(b"raw").eq_str("other"));
    }

    #[test]
    fn user_variants_render() {
        assert_eq!(User::None.to_string(), "");
        assert_eq!(User::Token("t").to_string(), "t");
        assert_eq!(User::Bytes(b"bytes").to_string(), "bytes");
        let packed = Packed::new(Alphabet::Hex, &[0xAB], false);
        assert_eq!(User::Packed(packed).to_string(), "AB");
    }

    #[test]
    fn invalid_utf8_renders_as_the_replacement_character() {
        // Dropping it silently would hide a real protocol problem.
        let user = User::Bytes(&[0xff, 0xfe]);
        assert_eq!(user.to_string(), "\u{FFFD}");
        assert!(!user.eq_str("\u{FFFD}"), "comparison stays byte-exact");
    }

    #[test]
    fn semantic_equality_looks_past_how_a_user_was_encoded() {
        // The digits 5511 as a packed run and as raw bytes name one user.
        let packed = Packed::new(Alphabet::Nibble, &[0x55, 0x11], false);
        let as_packed = Jid::pair(User::Packed(packed), SERVER_PN);
        let as_bytes = Jid::pair(User::Bytes(b"5511"), SERVER_PN);
        let as_token = Jid::pair(User::Token("5511"), SERVER_PN);

        assert!(as_packed.semantic_eq(as_bytes));
        assert!(as_bytes.semantic_eq(as_packed), "and the other way round");
        assert!(as_packed.semantic_eq(as_token));
        assert!(as_bytes.semantic_eq(as_token));
        assert_ne!(as_packed, as_bytes, "while not being byte-equal");
    }

    #[test]
    fn semantic_equality_still_separates_different_addresses() {
        let base = Jid::with_device(User::Token("u"), SERVER_PN, 1);
        assert!(base.semantic_eq(base));

        for different in [
            Jid::with_device(User::Token("v"), SERVER_PN, 1),
            Jid::with_device(User::Token("u"), SERVER_LID, 1),
            Jid::with_device(User::Token("u"), SERVER_PN, 2),
            Jid::interop(User::Token("u"), 1, 7),
            Jid::pair(User::None, SERVER_PN),
        ] {
            assert!(!base.semantic_eq(different), "{different} must differ");
        }
    }

    #[test]
    fn a_packed_user_never_equals_an_absent_one() {
        let packed = Packed::new(Alphabet::Nibble, &[], false);
        assert!(!User::Packed(packed).semantic_eq(User::None));
        assert!(!User::None.semantic_eq(User::Packed(packed)));
    }

    #[test]
    fn invalid_utf8_is_never_semantically_equal_to_text() {
        let invalid = User::Bytes(&[0xff, 0xfe]);
        assert!(!invalid.semantic_eq(User::Token("x")));
        assert!(!User::Token("x").semantic_eq(invalid));
        let packed = Packed::new(Alphabet::Nibble, &[0x12], false);
        assert!(!invalid.semantic_eq(User::Packed(packed)));
    }

    #[test]
    fn jids_are_comparable() {
        let a = Jid::pair(User::Token("x"), SERVER_PN);
        let b = Jid::pair(User::Token("x"), SERVER_PN);
        let c = Jid::pair(User::Token("x"), SERVER_LID);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(!alloc::format!("{a:?}").is_empty());
    }
}
