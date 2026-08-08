//! The envelope flag word.
//!
//! Two bits are assigned; the rest are reserved and must be zero. Rejecting a
//! set reserved bit is what lets a later contract version give that bit a
//! meaning without an older decoder silently misreading the envelope.

use crate::error::DecodeError;

/// Which way a stanza was travelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Received from the server.
    #[default]
    Inbound,
    /// Sent to the server.
    Outbound,
}

impl core::fmt::Display for Direction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        })
    }
}

/// Where the frame bytes in an envelope came from.
///
/// The contract's fast path is [`Original`]: the engine hands over the exact
/// buffer it decoded, so nothing is re-encoded. An engine that cannot reach
/// those bytes re-encodes the node and reports [`ReEncoded`], which still
/// conforms — the capability matrix is what surfaces the degradation.
///
/// [`Original`]: FrameOrigin::Original
/// [`ReEncoded`]: FrameOrigin::ReEncoded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FrameOrigin {
    /// The unpacked buffer the engine's own decoder consumed, verbatim.
    #[default]
    Original,
    /// Re-encoded from a decoded node, because the engine does not expose the
    /// original bytes.
    ReEncoded,
}

/// The decoded flag word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Flags {
    /// Travel direction of the stanza.
    pub direction: Direction,
    /// Provenance of the frame bytes.
    pub frame_origin: FrameOrigin,
}

impl Flags {
    const DIRECTION_BIT: u16 = 1 << 0;
    const FRAME_ORIGIN_BIT: u16 = 1 << 1;
    /// Bits with no assigned meaning in this contract version.
    pub const RESERVED_MASK: u16 = !(Self::DIRECTION_BIT | Self::FRAME_ORIGIN_BIT);

    /// An inbound envelope carrying original frame bytes — the common case.
    #[must_use]
    pub const fn inbound() -> Self {
        Self {
            direction: Direction::Inbound,
            frame_origin: FrameOrigin::Original,
        }
    }

    /// An outbound envelope carrying original frame bytes.
    #[must_use]
    pub const fn outbound() -> Self {
        Self {
            direction: Direction::Outbound,
            frame_origin: FrameOrigin::Original,
        }
    }

    /// Mark the frame as re-encoded rather than verbatim.
    #[must_use]
    pub const fn re_encoded(mut self) -> Self {
        self.frame_origin = FrameOrigin::ReEncoded;
        self
    }

    /// Pack into the on-wire flag word.
    #[must_use]
    pub const fn to_bits(self) -> u16 {
        let mut bits = 0u16;
        if matches!(self.direction, Direction::Outbound) {
            bits |= Self::DIRECTION_BIT;
        }
        if matches!(self.frame_origin, FrameOrigin::ReEncoded) {
            bits |= Self::FRAME_ORIGIN_BIT;
        }
        bits
    }

    /// Unpack from the on-wire flag word.
    ///
    /// Fails if any reserved bit is set.
    pub const fn from_bits(bits: u16) -> Result<Self, DecodeError> {
        if bits & Self::RESERVED_MASK != 0 {
            return Err(DecodeError::ReservedFlags(bits));
        }
        let direction = if bits & Self::DIRECTION_BIT != 0 {
            Direction::Outbound
        } else {
            Direction::Inbound
        };
        let frame_origin = if bits & Self::FRAME_ORIGIN_BIT != 0 {
            FrameOrigin::ReEncoded
        } else {
            FrameOrigin::Original
        };
        Ok(Self {
            direction,
            frame_origin,
        })
    }

    /// Whether the frame bytes are the engine's original buffer.
    #[must_use]
    pub const fn is_verbatim(self) -> bool {
        matches!(self.frame_origin, FrameOrigin::Original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_flags() -> [Flags; 4] {
        [
            Flags {
                direction: Direction::Inbound,
                frame_origin: FrameOrigin::Original,
            },
            Flags {
                direction: Direction::Inbound,
                frame_origin: FrameOrigin::ReEncoded,
            },
            Flags {
                direction: Direction::Outbound,
                frame_origin: FrameOrigin::Original,
            },
            Flags {
                direction: Direction::Outbound,
                frame_origin: FrameOrigin::ReEncoded,
            },
        ]
    }

    #[test]
    fn every_combination_round_trips() {
        for flags in all_flags() {
            let bits = flags.to_bits();
            assert_eq!(Flags::from_bits(bits), Ok(flags), "bits {bits:#06x}");
        }
    }

    #[test]
    fn bit_assignments_are_pinned() {
        // These values are on the wire; changing one is a contract break.
        assert_eq!(Flags::inbound().to_bits(), 0b00);
        assert_eq!(Flags::outbound().to_bits(), 0b01);
        assert_eq!(Flags::inbound().re_encoded().to_bits(), 0b10);
        assert_eq!(Flags::outbound().re_encoded().to_bits(), 0b11);
        assert_eq!(Flags::RESERVED_MASK, 0xFFFC);
    }

    #[test]
    fn reserved_bits_are_rejected() {
        for bit in 2..16u16 {
            let bits = 1u16 << bit;
            assert_eq!(
                Flags::from_bits(bits),
                Err(DecodeError::ReservedFlags(bits)),
                "bit {bit} must be reserved"
            );
        }
        // Reserved bits are rejected even alongside valid ones.
        assert_eq!(
            Flags::from_bits(0b101),
            Err(DecodeError::ReservedFlags(0b101))
        );
    }

    #[test]
    fn constructors_and_predicates_agree() {
        assert_eq!(Flags::inbound().direction, Direction::Inbound);
        assert_eq!(Flags::outbound().direction, Direction::Outbound);
        assert!(Flags::inbound().is_verbatim());
        assert!(Flags::outbound().is_verbatim());
        assert!(!Flags::inbound().re_encoded().is_verbatim());
        assert!(!Flags::outbound().re_encoded().is_verbatim());
    }

    #[test]
    fn default_is_the_common_case() {
        assert_eq!(Flags::default(), Flags::inbound());
        assert_eq!(Direction::default(), Direction::Inbound);
        assert_eq!(FrameOrigin::default(), FrameOrigin::Original);
    }

    #[test]
    fn re_encoded_is_idempotent() {
        let once = Flags::inbound().re_encoded();
        assert_eq!(once.re_encoded(), once);
    }
}
