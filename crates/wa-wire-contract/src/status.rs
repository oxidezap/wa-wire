//! Outcome of one decryption attempt.
//!
//! A stanza whose decryption failed still crosses the boundary. It carries a
//! non-`Ok` status and an empty payload rather than being dropped, so a
//! consumer can tell "this failed" apart from "this was never encrypted" —
//! the same rule `whatspec` and `wa-store-migrate` follow for unsupported
//! states.

use crate::error::DecodeError;

/// Whether a plaintext entry holds usable bytes, and if not, why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlaintextStatus {
    /// Decryption succeeded; the payload holds the plaintext.
    #[default]
    Ok,
    /// The engine attempted decryption and it failed. The payload is empty.
    DecryptFailed,
    /// The engine recognised the node but cannot decrypt this variant. The
    /// payload is empty.
    Unsupported,
}

impl PlaintextStatus {
    /// Pack into the on-wire status byte.
    #[must_use]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::DecryptFailed => 1,
            Self::Unsupported => 2,
        }
    }

    /// Unpack from the on-wire status byte.
    ///
    /// An unrecognised byte is rejected rather than mapped to a default: a
    /// future status must not be silently read as success.
    pub const fn from_byte(byte: u8) -> Result<Self, DecodeError> {
        match byte {
            0 => Ok(Self::Ok),
            1 => Ok(Self::DecryptFailed),
            2 => Ok(Self::Unsupported),
            other => Err(DecodeError::InvalidStatus(other)),
        }
    }

    /// Whether the entry's payload holds usable plaintext.
    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PlaintextStatus; 3] = [
        PlaintextStatus::Ok,
        PlaintextStatus::DecryptFailed,
        PlaintextStatus::Unsupported,
    ];

    #[test]
    fn every_status_round_trips() {
        for status in ALL {
            assert_eq!(PlaintextStatus::from_byte(status.to_byte()), Ok(status));
        }
    }

    #[test]
    fn byte_assignments_are_pinned() {
        // On the wire; changing one is a contract break.
        assert_eq!(PlaintextStatus::Ok.to_byte(), 0);
        assert_eq!(PlaintextStatus::DecryptFailed.to_byte(), 1);
        assert_eq!(PlaintextStatus::Unsupported.to_byte(), 2);
    }

    #[test]
    fn unknown_bytes_are_rejected_not_defaulted() {
        for byte in 3..=u8::MAX {
            assert_eq!(
                PlaintextStatus::from_byte(byte),
                Err(DecodeError::InvalidStatus(byte)),
                "byte {byte} must not decode"
            );
        }
    }

    #[test]
    fn only_ok_reports_usable_payload() {
        assert!(PlaintextStatus::Ok.is_ok());
        assert!(!PlaintextStatus::DecryptFailed.is_ok());
        assert!(!PlaintextStatus::Unsupported.is_ok());
    }

    #[test]
    fn default_is_ok() {
        assert_eq!(PlaintextStatus::default(), PlaintextStatus::Ok);
    }
}
