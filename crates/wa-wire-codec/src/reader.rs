//! Bounds-checked cursor over a frame.
//!
//! Big-endian, unlike the envelope: this is WhatsApp's encoding, and the
//! contract's job is to carry it unchanged rather than normalise it.
//!
//! The cursor tracks its unread tail rather than an index, so every read is a
//! `split_*_checked` whose one failure arm is the real short-read case. An
//! index-based cursor would need bounds checks the invariants already rule out
//! — arms no test could reach.

use crate::error::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.rest.len()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// What has not been read yet.
    pub(crate) const fn tail(&self) -> &'a [u8] {
        self.rest
    }

    const fn eof(&self, needed: usize) -> ParseError {
        ParseError::UnexpectedEof {
            needed,
            available: self.rest.len(),
        }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        let (head, tail) = self.rest.split_at_checked(n).ok_or_else(|| self.eof(n))?;
        self.rest = tail;
        Ok(head)
    }

    fn take_array<const N: usize>(&mut self) -> Result<&'a [u8; N], ParseError> {
        let (head, tail) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or_else(|| self.eof(N))?;
        self.rest = tail;
        Ok(head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ParseError> {
        let [byte] = *self.take_array::<1>()?;
        Ok(byte)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, ParseError> {
        Ok(u16::from_be_bytes(*self.take_array::<2>()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, ParseError> {
        Ok(u32::from_be_bytes(*self.take_array::<4>()?))
    }

    /// A 20-bit length: the low nibble of the first byte, then two more bytes.
    pub(crate) fn u20(&mut self) -> Result<u32, ParseError> {
        let [high, mid, low] = *self.take_array::<3>()?;
        Ok((u32::from(high & 0x0f) << 16) | (u32::from(mid) << 8) | u32::from(low))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_are_big_endian() {
        let mut reader = Reader::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
        assert_eq!(reader.u8(), Ok(0x01));
        assert_eq!(reader.u16(), Ok(0x0203));
        assert_eq!(reader.u32(), Ok(0x0405_0607));
        assert!(reader.is_empty());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn u20_masks_the_high_nibble() {
        // Only the low nibble of the first byte counts, so 0xF1 reads as 0x1.
        let mut reader = Reader::new(&[0xF1, 0x23, 0x45]);
        assert_eq!(reader.u20(), Ok(0x0001_2345));
        let mut reader = Reader::new(&[0x00, 0x00, 0x00]);
        assert_eq!(reader.u20(), Ok(0));
        let mut reader = Reader::new(&[0x0F, 0xFF, 0xFF]);
        assert_eq!(reader.u20(), Ok(0x000F_FFFF));
    }

    #[test]
    fn take_yields_exact_slices_and_advances() {
        let mut reader = Reader::new(&[1, 2, 3, 4]);
        assert_eq!(reader.take(0), Ok(&[][..]));
        assert_eq!(reader.take(3), Ok(&[1u8, 2, 3][..]));
        assert_eq!(reader.tail(), &[4]);
        assert_eq!(reader.take(1), Ok(&[4u8][..]));
        assert!(reader.is_empty());
    }

    #[test]
    fn short_reads_report_what_was_missing_and_do_not_advance() {
        let mut reader = Reader::new(&[1, 2]);
        assert_eq!(
            reader.take(3),
            Err(ParseError::UnexpectedEof {
                needed: 3,
                available: 2
            })
        );
        assert_eq!(reader.remaining(), 2, "a failed read must not consume");

        assert_eq!(
            reader.u32(),
            Err(ParseError::UnexpectedEof {
                needed: 4,
                available: 2
            })
        );
        assert_eq!(
            reader.u20(),
            Err(ParseError::UnexpectedEof {
                needed: 3,
                available: 2
            })
        );
        assert_eq!(reader.u16(), Ok(0x0102));
        assert_eq!(
            reader.u8(),
            Err(ParseError::UnexpectedEof {
                needed: 1,
                available: 0
            })
        );
    }

    #[test]
    fn an_enormous_take_is_a_short_read_not_an_overflow() {
        let mut reader = Reader::new(&[1]);
        assert!(reader.take(usize::MAX).is_err());
        assert_eq!(reader.remaining(), 1);
    }

    #[test]
    fn an_empty_reader_reports_empty() {
        let reader = Reader::new(&[]);
        assert!(reader.is_empty());
        assert_eq!(reader.remaining(), 0);
        assert_eq!(reader.tail(), &[] as &[u8]);
    }
}
