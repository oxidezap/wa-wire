//! Why a stanza could not be derived into an event.
//!
//! Each variant names the field, so a failure tells you which part of the shape
//! stopped matching. That is the distinction that matters in production: a
//! protocol change looks like one specific field going missing, while
//! corruption looks like the frame failing to parse at all — and the two want
//! very different responses.

use core::fmt;

/// Why derivation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeriveError {
    /// The stanza's tag matches no shape this build knows.
    UnknownStanza,
    /// A shape matched the tag but its assertions did not hold, and no other
    /// shape for that tag matched either.
    NoMatchingShape {
        /// The tag that was tried.
        tag: &'static str,
    },
    /// A required attribute is absent.
    MissingAttr {
        /// The attribute name.
        key: &'static str,
    },
    /// An attribute that should hold an integer does not.
    NotAnInt {
        /// The attribute name.
        key: &'static str,
    },
    /// An attribute that should hold a JID does not.
    NotAJid {
        /// The attribute name.
        key: &'static str,
    },
    /// An enum-valued attribute carries a value this build does not know.
    UnknownEnumValue {
        /// The attribute name.
        key: &'static str,
    },
    /// A required child node is absent.
    MissingChild {
        /// The child's tag.
        tag: &'static str,
    },
    /// A node that should carry a body does not.
    MissingContent {
        /// What kind of body was expected.
        field: Field,
    },
    /// An unsigned body is wider than 64 bits.
    ContentTooWide {
        /// How many bytes it carried.
        len: usize,
    },
}

/// What kind of body a node was expected to carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Field {
    /// Raw bytes.
    Bytes,
    /// An unsigned integer.
    Uint,
}

impl Field {
    /// A stable name for diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::Uint => "uint",
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for DeriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStanza => f.write_str("no shape is defined for this stanza's tag"),
            Self::NoMatchingShape { tag } => {
                write!(f, "no <{tag}> shape matched this stanza")
            }
            Self::MissingAttr { key } => write!(f, "missing required attribute `{key}`"),
            Self::NotAnInt { key } => write!(f, "attribute `{key}` is not an integer"),
            Self::NotAJid { key } => write!(f, "attribute `{key}` is not a JID"),
            Self::UnknownEnumValue { key } => {
                write!(f, "attribute `{key}` carries an unknown value")
            }
            Self::MissingChild { tag } => write!(f, "missing required child <{tag}>"),
            Self::MissingContent { field } => write!(f, "missing {field} body"),
            Self::ContentTooWide { len } => {
                write!(f, "unsigned body of {len} byte(s) exceeds 64 bits")
            }
        }
    }
}

impl core::error::Error for DeriveError {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn every_variant_names_what_went_wrong() {
        let cases: [(DeriveError, &str); 9] = [
            (DeriveError::UnknownStanza, "tag"),
            (DeriveError::NoMatchingShape { tag: "receipt" }, "receipt"),
            (DeriveError::MissingAttr { key: "id" }, "id"),
            (DeriveError::NotAnInt { key: "t" }, "t"),
            (DeriveError::NotAJid { key: "from" }, "from"),
            (DeriveError::UnknownEnumValue { key: "type" }, "type"),
            (DeriveError::MissingChild { tag: "enc" }, "enc"),
            (
                DeriveError::MissingContent {
                    field: Field::Bytes,
                },
                "bytes",
            ),
            (DeriveError::ContentTooWide { len: 9 }, "9"),
        ];
        for (error, fragment) in cases {
            let text = error.to_string();
            assert!(text.contains(fragment), "{text:?} lacks {fragment:?}");
        }
    }

    #[test]
    fn field_names_are_stable_and_distinct() {
        assert_eq!(Field::Bytes.name(), "bytes");
        assert_eq!(Field::Uint.name(), "uint");
        assert_ne!(Field::Bytes.name(), Field::Uint.name());
        assert_eq!(Field::Uint.to_string(), "uint");
    }

    #[test]
    fn errors_are_std_errors_and_comparable() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&DeriveError::UnknownStanza);
        assert_eq!(DeriveError::UnknownStanza, DeriveError::UnknownStanza);
        assert_ne!(
            DeriveError::MissingAttr { key: "a" },
            DeriveError::MissingAttr { key: "b" }
        );
        assert!(!alloc::format!("{:?}", DeriveError::UnknownStanza).is_empty());
    }
}
