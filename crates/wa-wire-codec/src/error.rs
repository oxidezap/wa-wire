//! Parse failures.
//!
//! A frame arrives from the network by way of another process, so a malformed
//! one is ordinary input. Every variant names what was wrong specifically
//! enough to tell a protocol change apart from corruption — the distinction
//! that matters when a stanza suddenly stops parsing in production.

use core::fmt;

/// Why a frame could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The buffer ended mid-field.
    UnexpectedEof {
        /// Bytes the field required.
        needed: usize,
        /// Bytes actually left.
        available: usize,
    },
    /// A node did not begin with a list tag.
    ExpectedList {
        /// The tag byte found instead.
        found: u8,
    },
    /// A node's list length was zero. Every node has at least a tag.
    EmptyNode,
    /// A tag byte has no meaning in this position.
    UnexpectedTag {
        /// The offending byte.
        tag: u8,
    },
    /// A dictionary index points past the end of its dictionary. Usually means
    /// the token table is older than the stanza.
    UnknownToken {
        /// Which dictionary, or `None` for the single-byte table.
        dictionary: Option<u8>,
        /// The index that was out of range.
        index: u16,
    },
    /// A node's tag was not a string.
    NonStringTag,
    /// An attribute key or value was not a string.
    NonStringAttr,
    /// A packed nibble or hex run declared a length that cannot exist.
    InvalidPackedLength {
        /// The length byte as read.
        byte: u8,
    },
    /// Bytes remain after the root node. The caller passed more than one frame,
    /// or the frame is not what it claims.
    TrailingBytes(usize),
    /// Nesting went deeper than the parser permits.
    DepthLimitExceeded {
        /// The limit that was hit.
        limit: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "unexpected end of frame: needed {needed} byte(s), {available} available"
            ),
            Self::ExpectedList { found } => {
                write!(f, "expected a list tag, found {found:#04x}")
            }
            Self::EmptyNode => f.write_str("node list is empty; every node has at least a tag"),
            Self::UnexpectedTag { tag } => write!(f, "unexpected tag {tag:#04x}"),
            Self::UnknownToken { dictionary, index } => match dictionary {
                Some(dict) => write!(f, "no token {index} in dictionary {dict}"),
                None => write!(f, "no single-byte token {index}"),
            },
            Self::NonStringTag => f.write_str("node tag is not a string"),
            Self::NonStringAttr => f.write_str("attribute key or value is not a string"),
            Self::InvalidPackedLength { byte } => {
                write!(f, "invalid packed length byte {byte:#04x}")
            }
            Self::TrailingBytes(count) => {
                write!(f, "{count} trailing byte(s) after the root node")
            }
            Self::DepthLimitExceeded { limit } => {
                write!(f, "nesting deeper than the limit of {limit}")
            }
        }
    }
}

impl core::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn every_variant_renders_its_detail() {
        let cases: [(ParseError, &[&str]); 10] = [
            (
                ParseError::UnexpectedEof {
                    needed: 4,
                    available: 1,
                },
                &["4", "1"],
            ),
            (ParseError::ExpectedList { found: 0x07 }, &["0x07"]),
            (ParseError::EmptyNode, &["empty"]),
            (ParseError::UnexpectedTag { tag: 0xAB }, &["0xab"]),
            (
                ParseError::UnknownToken {
                    dictionary: Some(2),
                    index: 300,
                },
                &["2", "300"],
            ),
            (
                ParseError::UnknownToken {
                    dictionary: None,
                    index: 240,
                },
                &["240"],
            ),
            (ParseError::NonStringTag, &["tag"]),
            (ParseError::NonStringAttr, &["attribute"]),
            (ParseError::InvalidPackedLength { byte: 0x80 }, &["0x80"]),
            (ParseError::TrailingBytes(9), &["9"]),
        ];
        for (error, fragments) in cases {
            let text = error.to_string();
            assert!(!text.is_empty());
            for fragment in fragments {
                assert!(text.contains(fragment), "{text:?} lacks {fragment:?}");
            }
        }
        assert!(
            ParseError::DepthLimitExceeded { limit: 64 }
                .to_string()
                .contains("64")
        );
    }

    #[test]
    fn errors_are_std_errors_and_comparable() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&ParseError::EmptyNode);
        assert_eq!(ParseError::EmptyNode, ParseError::EmptyNode);
        assert_ne!(ParseError::EmptyNode, ParseError::NonStringTag);
        assert!(!alloc::format!("{:?}", ParseError::EmptyNode).is_empty());
    }
}
