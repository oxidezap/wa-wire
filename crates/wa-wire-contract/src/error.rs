//! Decode failures.
//!
//! Every variant names what was expected and what was found. An envelope comes
//! from another process or another language runtime, so a malformed one is an
//! ordinary input, not a bug — it must be reportable, never a panic.

use core::fmt;

use crate::status::PlaintextStatus;

/// Why an envelope could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// The buffer ended before a field could be read in full.
    UnexpectedEof {
        /// Bytes the field required.
        needed: usize,
        /// Bytes actually left in the buffer.
        available: usize,
        /// Which field ran out.
        field: Field,
    },
    /// A reserved flag bit was set. Reserved bits must be zero so that a future
    /// contract version can assign meaning to them without silently changing
    /// how an older decoder behaves.
    ReservedFlags(u16),
    /// The status byte of a plaintext entry is not a value this contract
    /// version defines.
    InvalidStatus(u8),
    /// A plaintext entry carries a payload under a status that defines none.
    ///
    /// Only [`PlaintextStatus::Ok`] describes usable bytes. Bytes under any
    /// other status have no defined meaning, and reading them as plaintext
    /// would be reading whatever the producer happened to leave there.
    ///
    /// [`PlaintextStatus::Ok`]: crate::status::PlaintextStatus::Ok
    PayloadOnFailedStatus {
        /// The status the entry declared.
        status: PlaintextStatus,
        /// How many bytes it carried anyway.
        len: usize,
    },
    /// The envelope decoded correctly but bytes remain. A trailing tail means
    /// the producer and consumer disagree about the layout, which is a fault
    /// even though the prefix parsed.
    TrailingBytes(usize),
}

/// The envelope field a decode failure refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Field {
    /// The contract version.
    Version,
    /// The flag word.
    Flags,
    /// The frame length prefix.
    FrameLen,
    /// The frame payload.
    Frame,
    /// The plaintext entry count.
    PlaintextCount,
    /// A plaintext entry's path length prefix.
    PathLen,
    /// A plaintext entry's path.
    Path,
    /// A plaintext entry's status byte.
    Status,
    /// A plaintext entry's payload length prefix.
    PayloadLen,
    /// A plaintext entry's payload.
    Payload,
}

impl Field {
    /// A stable, human-readable name for this field.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::Flags => "flags",
            Self::FrameLen => "frame_len",
            Self::Frame => "frame",
            Self::PlaintextCount => "pt_count",
            Self::PathLen => "path_len",
            Self::Path => "path",
            Self::Status => "status",
            Self::PayloadLen => "payload_len",
            Self::Payload => "payload",
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                needed,
                available,
                field,
            } => write!(
                f,
                "unexpected end of input reading {field}: needed {needed} byte(s), {available} available"
            ),
            Self::ReservedFlags(bits) => {
                write!(f, "reserved flag bits set: {bits:#06x}")
            }
            Self::InvalidStatus(byte) => write!(f, "invalid plaintext status: {byte}"),
            Self::PayloadOnFailedStatus { status, len } => write!(
                f,
                "status {status} carries {len} payload byte(s); only ok may carry any"
            ),
            Self::TrailingBytes(count) => {
                write!(f, "{count} trailing byte(s) after the envelope")
            }
        }
    }
}

impl core::error::Error for DecodeError {}

/// Why an envelope could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// The frame is longer than the `u32` length prefix can describe.
    FrameTooLong(usize),
    /// A plaintext payload is longer than the `u32` length prefix can describe.
    PayloadTooLong(usize),
    /// A node path has more components than the `u8` length prefix can
    /// describe. Real stanzas nest to depth 9 at the extreme, so hitting this
    /// means the path is wrong, not that the limit is tight.
    PathTooLong(usize),
    /// More plaintext entries than the `u16` count can describe.
    TooManyEntries(usize),
    /// A plaintext entry carries a payload under a status that defines none.
    ///
    /// Refused at the producer rather than tolerated, so an envelope whose
    /// status and payload contradict each other never reaches a consumer that
    /// has to guess which of the two to believe.
    PayloadOnFailedStatus {
        /// The status the entry declared.
        status: PlaintextStatus,
        /// How many bytes it carried anyway.
        len: usize,
    },
    /// The destination slice is smaller than [`encoded_len`] reported.
    ///
    /// [`encoded_len`]: crate::envelope::EnvelopeBuilder::encoded_len
    BufferTooSmall {
        /// Bytes the envelope needs.
        needed: usize,
        /// Bytes the destination offers.
        available: usize,
    },
    /// The envelope's total size does not fit in `usize` on this target.
    LengthOverflow,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLong(len) => write!(f, "frame of {len} byte(s) exceeds u32"),
            Self::PayloadTooLong(len) => write!(f, "payload of {len} byte(s) exceeds u32"),
            Self::PathTooLong(len) => write!(f, "path of {len} component(s) exceeds u8"),
            Self::TooManyEntries(count) => write!(f, "{count} plaintext entries exceed u16"),
            Self::PayloadOnFailedStatus { status, len } => write!(
                f,
                "status {status} carries {len} payload byte(s); only ok may carry any"
            ),
            Self::BufferTooSmall { needed, available } => write!(
                f,
                "destination too small: needed {needed} byte(s), {available} available"
            ),
            Self::LengthOverflow => {
                f.write_str("encoded length exceeds this target's address space")
            }
        }
    }
}

impl core::error::Error for EncodeError {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn field_names_are_stable_and_unique() {
        let all = [
            Field::Version,
            Field::Flags,
            Field::FrameLen,
            Field::Frame,
            Field::PlaintextCount,
            Field::PathLen,
            Field::Path,
            Field::Status,
            Field::PayloadLen,
            Field::Payload,
        ];
        for (i, a) in all.iter().enumerate() {
            assert!(!a.name().is_empty());
            assert_eq!(a.to_string(), a.name());
            for b in all.iter().skip(i.saturating_add(1)) {
                assert_ne!(a.name(), b.name(), "field names must be unique");
            }
        }
    }

    #[test]
    fn decode_errors_render_their_detail() {
        let eof = DecodeError::UnexpectedEof {
            needed: 4,
            available: 1,
            field: Field::FrameLen,
        };
        let text = eof.to_string();
        assert!(text.contains("frame_len"), "{text}");
        assert!(text.contains('4') && text.contains('1'), "{text}");

        assert!(
            DecodeError::ReservedFlags(0xFFFC)
                .to_string()
                .contains("0xfffc")
        );
        assert!(DecodeError::InvalidStatus(9).to_string().contains('9'));
        assert!(DecodeError::TrailingBytes(3).to_string().contains('3'));

        let paired = DecodeError::PayloadOnFailedStatus {
            status: PlaintextStatus::Unobserved,
            len: 12,
        }
        .to_string();
        assert!(
            paired.contains("unobserved") && paired.contains("12"),
            "{paired}"
        );
    }

    #[test]
    fn encode_errors_render_their_detail() {
        assert!(EncodeError::FrameTooLong(5).to_string().contains('5'));
        assert!(EncodeError::PayloadTooLong(6).to_string().contains('6'));
        assert!(EncodeError::PathTooLong(7).to_string().contains('7'));
        assert!(EncodeError::TooManyEntries(8).to_string().contains('8'));
        let paired = EncodeError::PayloadOnFailedStatus {
            status: PlaintextStatus::DecryptFailed,
            len: 9,
        }
        .to_string();
        assert!(
            paired.contains("decrypt-failed") && paired.contains('9'),
            "{paired}"
        );
        let small = EncodeError::BufferTooSmall {
            needed: 10,
            available: 2,
        };
        assert!(small.to_string().contains("10") && small.to_string().contains('2'));
        assert!(
            EncodeError::LengthOverflow
                .to_string()
                .contains("address space")
        );
    }

    #[test]
    fn errors_are_std_errors() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&DecodeError::TrailingBytes(1));
        assert_error(&EncodeError::LengthOverflow);
    }

    #[test]
    fn errors_are_debug_and_comparable() {
        assert_eq!(DecodeError::InvalidStatus(1), DecodeError::InvalidStatus(1));
        assert_ne!(DecodeError::InvalidStatus(1), DecodeError::InvalidStatus(2));
        assert_eq!(EncodeError::PathTooLong(1), EncodeError::PathTooLong(1));
        assert_ne!(EncodeError::PathTooLong(1), EncodeError::FrameTooLong(1));
        assert!(!alloc::format!("{:?}", Field::Path).is_empty());
    }
}
