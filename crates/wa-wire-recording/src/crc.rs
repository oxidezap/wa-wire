//! CRC-32, for detecting damage to a recording.
//!
//! Not for detecting tampering: anything able to rewrite the records can
//! rewrite the checksum, and the container is not signed. A cryptographic
//! digest here would claim a guarantee the format does not provide, and would
//! have to be hand-written twice — once `no_std`, once for a browser (D-084).
//!
//! The standard reflected polynomial, so the values match every published
//! implementation and can be checked against known vectors rather than against
//! this code's own output.

/// Reflected form of the IEEE 802.3 polynomial.
const POLY: u32 = 0xEDB8_8320;

/// Running CRC-32 state.
///
/// Incremental because the writer checksums the header before the records
/// exist, and a reader checksums a buffer it is walking anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crc32(u32);

impl Crc32 {
    /// A fresh checksum.
    #[must_use]
    pub const fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    /// Fold `bytes` in.
    #[must_use]
    pub fn update(mut self, bytes: &[u8]) -> Self {
        // Bitwise rather than table-driven: a 1 KiB table would be the largest
        // thing in this crate, and a recording is checksummed once per read.
        for byte in bytes {
            self.0 ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = (self.0 & 1).wrapping_neg();
                self.0 = (self.0 >> 1) ^ (POLY & mask);
            }
        }
        self
    }

    /// The finished value.
    #[must_use]
    pub const fn finish(self) -> u32 {
        self.0 ^ 0xFFFF_FFFF
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksum a whole slice.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    Crc32::new().update(bytes).finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_published_vectors() {
        // From the CRC catalogue's CRC-32/ISO-HDLC entry. Checking against
        // published values rather than against this implementation's own
        // output is the whole point: a checksum that only agrees with itself
        // agrees with no other language.
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
        assert_eq!(crc32(b"abc"), 0x3524_41C2);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }

    #[test]
    fn folding_in_pieces_matches_folding_at_once() {
        // The writer checksums the header before the records exist, so the
        // incremental path has to reach the same answer as the whole-slice one.
        let whole = crc32(b"123456789");
        let pieces = Crc32::new()
            .update(b"1234")
            .update(b"")
            .update(b"56789")
            .finish();
        assert_eq!(pieces, whole);
    }

    #[test]
    fn a_single_flipped_bit_changes_the_value() {
        assert_ne!(crc32(b"abc"), crc32(b"abd"));
        assert_ne!(crc32(&[0x00]), crc32(&[0x80]));
    }

    #[test]
    fn the_default_is_a_fresh_checksum() {
        assert_eq!(Crc32::default(), Crc32::new());
        assert_eq!(Crc32::default().finish(), crc32(b""));
    }
}
