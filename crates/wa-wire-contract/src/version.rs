//! Contract versioning.
//!
//! This versions *our* boundary, never WhatsApp's protocol. A protocol change
//! must not bump it — otherwise every deployed adapter breaks whenever Meta
//! ships anything. What makes that separation safe is L0 totality: at L0 there
//! is nothing for a protocol change to break, because the frame crosses
//! verbatim. Which WhatsApp version a build derives from is tracked separately,
//! by [`Provenance`](crate::provenance::Provenance).

use core::fmt;

/// The major version of the boundary layout and negotiation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractVersion(u16);

impl ContractVersion {
    /// The version this build produces and understands.
    pub const CURRENT: Self = Self(1);

    /// Wrap a raw version number.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// The raw version number as it appears on the wire.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Whether this build can interpret envelopes at `self`.
    ///
    /// Equality, not a range: a differing major means the layout may differ,
    /// and guessing is how silent corruption starts.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.0 == Self::CURRENT.0
    }

    /// Check compatibility against this build, reporting the mismatch.
    ///
    /// This is the setup-time gate. Negotiation fails loudly here so nothing
    /// can fail quietly later, per the same rule capabilities follow.
    pub const fn check(self) -> Result<(), VersionMismatch> {
        if self.is_supported() {
            Ok(())
        } else {
            Err(VersionMismatch {
                expected: Self::CURRENT,
                found: self,
            })
        }
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl From<u16> for ContractVersion {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<ContractVersion> for u16 {
    fn from(value: ContractVersion) -> Self {
        value.0
    }
}

/// A peer speaks a contract version this build does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionMismatch {
    /// What this build speaks.
    pub expected: ContractVersion,
    /// What the peer announced.
    pub found: ContractVersion,
}

impl fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "contract version mismatch: this build speaks {}, peer announced {}",
            self.expected, self.found
        )
    }
}

impl core::error::Error for VersionMismatch {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn current_is_pinned() {
        // On the wire; bumping this is a deliberate contract break.
        assert_eq!(ContractVersion::CURRENT.get(), 1);
    }

    #[test]
    fn raw_values_round_trip() {
        for raw in [0u16, 1, 2, 999, u16::MAX] {
            let version = ContractVersion::new(raw);
            assert_eq!(version.get(), raw);
            assert_eq!(u16::from(version), raw);
            assert_eq!(ContractVersion::from(raw), version);
        }
    }

    #[test]
    fn only_the_current_version_is_supported() {
        assert!(ContractVersion::CURRENT.is_supported());
        assert_eq!(ContractVersion::CURRENT.check(), Ok(()));

        for raw in [0u16, 2, 999, u16::MAX] {
            let version = ContractVersion::new(raw);
            assert!(!version.is_supported(), "{raw} must not be supported");
            assert_eq!(
                version.check(),
                Err(VersionMismatch {
                    expected: ContractVersion::CURRENT,
                    found: version,
                })
            );
        }
    }

    #[test]
    fn versions_order_numerically() {
        assert!(ContractVersion::new(1) < ContractVersion::new(2));
        assert_eq!(ContractVersion::new(3), ContractVersion::new(3));
    }

    #[test]
    fn display_is_readable() {
        assert_eq!(ContractVersion::new(7).to_string(), "v7");
        let mismatch = VersionMismatch {
            expected: ContractVersion::new(1),
            found: ContractVersion::new(2),
        };
        let text = mismatch.to_string();
        assert!(text.contains("v1") && text.contains("v2"), "{text}");
    }

    #[test]
    fn mismatch_is_a_std_error() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&VersionMismatch {
            expected: ContractVersion::CURRENT,
            found: ContractVersion::new(2),
        });
    }
}
