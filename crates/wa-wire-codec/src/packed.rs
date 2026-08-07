//! Packed digit runs — nibble and hex.
//!
//! Two digits per byte, with a flag saying the last nibble is padding. Decoding
//! builds characters that exist nowhere in the buffer, so a borrowed `&str` is
//! impossible; the run stays packed and renders on demand instead. Comparisons
//! and iteration work without materialising anything, which is what keeps the
//! parser allocation-free.

use core::fmt;

use crate::error::ParseError;

/// Which digit set a packed run uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alphabet {
    /// Digits, `-` and `.` — used for phone numbers.
    Nibble,
    /// Uppercase hexadecimal.
    Hex,
}

impl Alphabet {
    /// The character for a nibble value.
    ///
    /// Computed rather than indexed: a four-bit input has no out-of-range case,
    /// and spelling that out leaves no unreachable arm for a table lookup to
    /// hide. [`crate::token::NIBBLE_ALPHABET`] and [`crate::token::HEX_ALPHABET`]
    /// describe the same mapping
    /// for callers that want it as data.
    #[must_use]
    pub const fn char_at(self, nibble: u8) -> char {
        let nibble = nibble & 0x0f;
        match self {
            Self::Hex => match nibble {
                0..=9 => b'0'.wrapping_add(nibble) as char,
                _ => b'A'.wrapping_add(nibble.wrapping_sub(10)) as char,
            },
            Self::Nibble => match nibble {
                0..=9 => b'0'.wrapping_add(nibble) as char,
                10 => '-',
                11 => '.',
                // 12..=15 are unassigned; every engine renders them as the
                // replacement character rather than failing.
                _ => '\u{FFFD}',
            },
        }
    }
}

/// A run of packed digits, still in its on-wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packed<'a> {
    alphabet: Alphabet,
    bytes: &'a [u8],
    /// Whether the final low nibble is padding rather than a digit.
    odd: bool,
}

impl<'a> Packed<'a> {
    /// Interpret `bytes` as a packed run.
    #[must_use]
    pub const fn new(alphabet: Alphabet, bytes: &'a [u8], odd: bool) -> Self {
        Self {
            alphabet,
            bytes,
            odd,
        }
    }

    /// Split the length byte into a byte count and the odd-length flag.
    ///
    /// The high bit marks an odd digit count; the low seven bits are the byte
    /// count. A zero count with the odd bit set would describe a run of minus
    /// one digits, so it is rejected.
    pub const fn split_length_byte(byte: u8) -> Result<(usize, bool), ParseError> {
        let odd = byte & 0x80 != 0;
        let count = (byte & 0x7f) as usize;
        if odd && count == 0 {
            return Err(ParseError::InvalidPackedLength { byte });
        }
        Ok((count, odd))
    }

    /// Which digit set this run uses.
    #[must_use]
    pub const fn alphabet(self) -> Alphabet {
        self.alphabet
    }

    /// The packed bytes.
    #[must_use]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// How many digits the run decodes to.
    #[must_use]
    pub const fn len(self) -> usize {
        let digits = self.bytes.len().saturating_mul(2);
        if self.odd {
            digits.saturating_sub(1)
        } else {
            digits
        }
    }

    /// Whether the run decodes to nothing.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The digits, in order.
    pub fn chars(self) -> impl Iterator<Item = char> + 'a {
        let alphabet = self.alphabet;
        let digits = self.len();
        self.bytes
            .iter()
            .flat_map(move |byte| [alphabet.char_at(byte >> 4), alphabet.char_at(byte & 0x0f)])
            .take(digits)
    }

    /// Whether the run decodes to exactly `other`, without building a string.
    #[must_use]
    pub fn eq_str(self, other: &str) -> bool {
        let mut expected = other.chars();
        let mut actual = self.chars();
        loop {
            match (actual.next(), expected.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) if a == b => {}
                _ => return false,
            }
        }
    }
}

impl fmt::Display for Packed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for digit in self.chars() {
            f.write_char(digit)?;
        }
        Ok(())
    }
}

use core::fmt::Write as _;

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    #[test]
    fn a_length_byte_splits_into_count_and_parity() {
        assert_eq!(Packed::split_length_byte(0x00), Ok((0, false)));
        assert_eq!(Packed::split_length_byte(0x05), Ok((5, false)));
        assert_eq!(Packed::split_length_byte(0x7f), Ok((127, false)));
        assert_eq!(Packed::split_length_byte(0x85), Ok((5, true)));
        assert_eq!(Packed::split_length_byte(0xff), Ok((127, true)));
    }

    #[test]
    fn an_odd_run_of_zero_bytes_is_rejected() {
        // It would describe minus one digits.
        assert_eq!(
            Packed::split_length_byte(0x80),
            Err(ParseError::InvalidPackedLength { byte: 0x80 })
        );
    }

    #[test]
    fn even_nibble_runs_decode_both_halves() {
        let packed = Packed::new(Alphabet::Nibble, &[0x12, 0x34], false);
        assert_eq!(packed.len(), 4);
        assert!(!packed.is_empty());
        assert_eq!(packed.to_string(), "1234");
        assert_eq!(packed.chars().collect::<String>(), "1234");
        assert_eq!(packed.as_bytes(), &[0x12, 0x34]);
        assert_eq!(packed.alphabet(), Alphabet::Nibble);
    }

    #[test]
    fn odd_runs_drop_the_padding_nibble() {
        let packed = Packed::new(Alphabet::Nibble, &[0x12, 0x3f], true);
        assert_eq!(packed.len(), 3);
        assert_eq!(packed.to_string(), "123");
    }

    #[test]
    fn hex_runs_use_uppercase_digits() {
        let packed = Packed::new(Alphabet::Hex, &[0xAB, 0xCD], false);
        assert_eq!(packed.to_string(), "ABCD");
        let odd = Packed::new(Alphabet::Hex, &[0xEF, 0x00], true);
        assert_eq!(odd.to_string(), "EF0");
    }

    #[test]
    fn the_nibble_alphabet_covers_phone_number_punctuation() {
        // 10 is '-', 11 is '.', and 12..16 are unassigned.
        let packed = Packed::new(Alphabet::Nibble, &[0xAB, 0xCD], false);
        assert_eq!(packed.to_string(), "-.\u{FFFD}\u{FFFD}");
        let real = Packed::new(Alphabet::Nibble, &[0x55, 0x11, 0x99, 0x88], false);
        assert_eq!(real.to_string(), "55119988");
    }

    #[test]
    fn an_empty_run_decodes_to_nothing() {
        let packed = Packed::new(Alphabet::Nibble, &[], false);
        assert_eq!(packed.len(), 0);
        assert!(packed.is_empty());
        assert_eq!(packed.to_string(), "");
        assert_eq!(packed.chars().count(), 0);
        assert!(packed.eq_str(""));
    }

    #[test]
    fn an_odd_flag_on_empty_bytes_saturates_rather_than_wrapping() {
        // Not reachable through the parser, which rejects the length byte, but
        // the type must stay total for a hand-built value.
        let packed = Packed::new(Alphabet::Nibble, &[], true);
        assert_eq!(packed.len(), 0);
        assert_eq!(packed.chars().count(), 0);
    }

    #[test]
    fn eq_str_compares_without_allocating() {
        let packed = Packed::new(Alphabet::Nibble, &[0x12, 0x34], false);
        assert!(packed.eq_str("1234"));
        assert!(!packed.eq_str("123"), "shorter must not match");
        assert!(!packed.eq_str("12345"), "longer must not match");
        assert!(!packed.eq_str("1235"), "same length, different digit");
        assert!(!packed.eq_str(""));

        let odd = Packed::new(Alphabet::Hex, &[0xAB, 0xC0], true);
        assert!(odd.eq_str("ABC"));
        assert!(!odd.eq_str("ABC0"));
    }

    #[test]
    fn char_at_agrees_with_the_published_tables() {
        // `char_at` computes what NIBBLE_ALPHABET and HEX_ALPHABET describe;
        // the two must not drift apart.
        use crate::token::{HEX_ALPHABET, NIBBLE_ALPHABET};
        for nibble in 0..16u8 {
            let index = nibble as usize;
            assert_eq!(Alphabet::Nibble.char_at(nibble), NIBBLE_ALPHABET[index]);
            assert_eq!(Alphabet::Hex.char_at(nibble), HEX_ALPHABET[index]);
        }
    }

    #[test]
    fn char_at_masks_to_four_bits() {
        assert_eq!(Alphabet::Hex.char_at(0x00), '0');
        assert_eq!(Alphabet::Hex.char_at(0x0f), 'F');
        assert_eq!(Alphabet::Hex.char_at(0xff), 'F', "high bits are ignored");
        assert_eq!(Alphabet::Nibble.char_at(0x0a), '-');
    }

    #[test]
    fn every_nibble_value_decodes_for_both_alphabets() {
        for value in 0..16u8 {
            let byte = [value << 4];
            assert_eq!(Packed::new(Alphabet::Hex, &byte, true).chars().count(), 1);
            assert_eq!(
                Packed::new(Alphabet::Nibble, &byte, true).chars().count(),
                1
            );
        }
    }

    #[test]
    fn a_long_run_decodes_every_digit() {
        let bytes: Vec<u8> = (0..127u8).collect();
        let packed = Packed::new(Alphabet::Hex, &bytes, false);
        assert_eq!(packed.len(), 254);
        assert_eq!(packed.chars().count(), 254);
    }

    #[test]
    fn packed_values_are_comparable() {
        let a = Packed::new(Alphabet::Nibble, &[0x12], false);
        let b = Packed::new(Alphabet::Nibble, &[0x12], false);
        let c = Packed::new(Alphabet::Hex, &[0x12], false);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(!alloc::format!("{a:?}").is_empty());
    }
}
