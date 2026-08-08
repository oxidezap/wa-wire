//! Why a recording could not be read or written.
//!
//! A recording comes from another machine, another language runtime, or a
//! process that was killed mid-write. Every one of those is an ordinary input
//! rather than a bug, so all of them are reportable and none of them panic.

use core::fmt;

/// Why a recording could not be read.
///
/// Truncation is deliberately absent: a recording cut short is a *state* the
/// reader reports, not a failure to read (D-076). The artifact a crash recorder
/// exists to produce is by definition the interrupted one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadError {
    /// The buffer does not start with `WAWR`.
    NotARecording,
    /// The buffer ended before a fixed-size header field could be read.
    ///
    /// Distinct from truncation, which is about records: a buffer too short to
    /// hold a header never was a recording.
    HeaderTooShort {
        /// Bytes the header needs.
        needed: usize,
        /// Bytes the buffer offers.
        available: usize,
    },
    /// The metadata block extends past the end of the buffer.
    MetaOutOfBounds {
        /// Bytes the metadata block claims.
        claimed: usize,
        /// Bytes left after the header.
        available: usize,
    },
    /// A metadata entry extends past the end of the metadata block.
    MalformedMeta {
        /// The tag that could not be read in full.
        tag: u16,
    },
    /// The container version is one this reader does not implement.
    ///
    /// Rejected rather than read on a best-effort basis: an older reader that
    /// guessed at a newer layout would produce a confident wrong answer, which
    /// is the failure the whole format exists to prevent.
    UnsupportedVersion(u16),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARecording => f.write_str("not a wa-wire recording: bad magic"),
            Self::HeaderTooShort { needed, available } => {
                write!(f, "header needs {needed} byte(s), {available} available")
            }
            Self::MetaOutOfBounds { claimed, available } => write!(
                f,
                "metadata claims {claimed} byte(s), {available} available"
            ),
            Self::MalformedMeta { tag } => {
                write!(f, "metadata entry {tag:#06x} runs past the block")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "container version {version} is not supported")
            }
        }
    }
}

impl core::error::Error for ReadError {}

/// Why a recording could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteError {
    /// A record payload is longer than the `u32` length prefix can describe.
    RecordTooLong(usize),
    /// The metadata block is longer than its `u32` length prefix can describe.
    MetaTooLong(usize),
    /// A length-prefixed string is longer than its `u16` prefix can describe.
    StringTooLong(usize),
    /// More records than the trailer's `u32` count can describe.
    TooManyRecords(u32),
    /// More capability identifiers than the `u16` count can describe.
    TooManyCapabilities(usize),
    /// A metadata tag was written twice.
    ///
    /// Refused rather than tolerated: a reader takes the first, so a duplicate
    /// is a value the writer believes it set and the reader will never see.
    DuplicateTag(u16),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordTooLong(len) => write!(f, "record of {len} byte(s) exceeds u32"),
            Self::MetaTooLong(len) => write!(f, "metadata of {len} byte(s) exceeds u32"),
            Self::StringTooLong(len) => write!(f, "string of {len} byte(s) exceeds u16"),
            Self::TooManyRecords(count) => write!(f, "{count} records exceed u32"),
            Self::TooManyCapabilities(count) => {
                write!(f, "{count} capability identifiers exceed u16")
            }
            Self::DuplicateTag(tag) => write!(f, "metadata tag {tag:#06x} written twice"),
        }
    }
}

impl core::error::Error for WriteError {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn read_errors_render_their_detail() {
        assert!(ReadError::NotARecording.to_string().contains("magic"));

        let short = ReadError::HeaderTooShort {
            needed: 10,
            available: 3,
        }
        .to_string();
        assert!(short.contains("10") && short.contains('3'), "{short}");

        let meta = ReadError::MetaOutOfBounds {
            claimed: 99,
            available: 4,
        }
        .to_string();
        assert!(meta.contains("99") && meta.contains('4'), "{meta}");

        assert!(
            ReadError::MalformedMeta { tag: 0x8001 }
                .to_string()
                .contains("0x8001")
        );
        assert!(ReadError::UnsupportedVersion(7).to_string().contains('7'));
    }

    #[test]
    fn write_errors_render_their_detail() {
        assert!(ReadError::NotARecording.to_string().contains("recording"));
        assert!(WriteError::RecordTooLong(5).to_string().contains('5'));
        assert!(WriteError::MetaTooLong(6).to_string().contains('6'));
        assert!(WriteError::StringTooLong(7).to_string().contains('7'));
        assert!(WriteError::TooManyRecords(8).to_string().contains('8'));
        assert!(WriteError::TooManyCapabilities(9).to_string().contains('9'));
        assert!(
            WriteError::DuplicateTag(0x0001)
                .to_string()
                .contains("0x0001")
        );
    }

    #[test]
    fn errors_are_std_errors_and_comparable() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&ReadError::NotARecording);
        assert_error(&WriteError::RecordTooLong(1));

        assert_eq!(ReadError::NotARecording, ReadError::NotARecording);
        assert_ne!(
            ReadError::UnsupportedVersion(1),
            ReadError::UnsupportedVersion(2)
        );
        assert!(!alloc::format!("{:?}", WriteError::MetaTooLong(1)).is_empty());
    }
}
