//! Zero-copy reader for the protobuf wire format.
//!
//! The sibling of [`wa-wire-codec`]. That one parses the stanza; this one
//! parses what the stanza's `<enc>` children decrypt to, which is where every
//! message body lives. Same rules: `no_std`, no dependencies, borrowing from
//! the buffer rather than copying out of it.
//!
//! ```
//! use wa_wire_proto::{Reader, Value};
//!
//! // field 1, length-delimited, "hi"
//! let buf = [0x0a, 0x02, b'h', b'i'];
//! let text = Reader::new(&buf)
//!     .find_last(1)?
//!     .and_then(Value::as_str);
//! assert_eq!(text, Some("hi"));
//! # Ok::<(), wa_wire_proto::Error>(())
//! ```
//!
//! # What it does not do
//!
//! It does not know any schema. A field is a number and some bytes, and what
//! those mean is the caller's business. That is deliberate: the payloads here
//! come from a protocol that adds fields without asking, and a reader that
//! refused an unknown field would fail on exactly the traffic worth looking at.
//!
//! # Totality
//!
//! Every byte sequence either yields fields or a reportable error, and no
//! sequence panics. Once a read fails the reader stops, so a caller looping
//! over [`Reader::next`] cannot spin on the same bad byte.
//!
//! [`wa-wire-codec`]: https://docs.rs/wa-wire-codec

#![no_std]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
    )
)]

use core::fmt;

/// Why a payload could not be read.
///
/// A payload arrives from another party's encoder, so a malformed one is an
/// ordinary input rather than a bug: reportable, never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A varint ran past the end of the buffer, or past ten bytes.
    ///
    /// Ten is the most a `u64` can need, so an eleventh continuation byte is a
    /// malformed input rather than a bigger number.
    MalformedVarint,
    /// The buffer ended in the middle of a field.
    UnexpectedEnd {
        /// Bytes the field needed.
        needed: usize,
        /// Bytes left.
        available: usize,
    },
    /// A tag named wire type 6 or 7, which the format does not define.
    UnknownWireType(u8),
    /// A tag named field number zero, which the format forbids.
    ZeroFieldNumber,
    /// A group was opened and never closed.
    UnterminatedGroup {
        /// The field number that opened it.
        number: u32,
    },
    /// A group ended that was never opened.
    UnexpectedGroupEnd {
        /// The field number that closed it.
        number: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedVarint => f.write_str("malformed varint"),
            Self::UnexpectedEnd { needed, available } => write!(
                f,
                "payload ended mid-field: needed {needed} byte(s), {available} available"
            ),
            Self::UnknownWireType(wire) => write!(f, "wire type {wire} is not defined"),
            Self::ZeroFieldNumber => f.write_str("field number 0 is not allowed"),
            Self::UnterminatedGroup { number } => {
                write!(f, "group {number} was opened and never closed")
            }
            Self::UnexpectedGroupEnd { number } => {
                write!(f, "group {number} was closed but never opened")
            }
        }
    }
}

impl core::error::Error for Error {}

/// One field's value, borrowed from the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    /// Wire type 0. Also carries `bool`, the `int`/`uint`/`sint` families and
    /// enums, which is why the interpretation is left to the caller.
    Varint(u64),
    /// Wire type 1: `fixed64`, `sfixed64`, `double`.
    Fixed64(u64),
    /// Wire type 2: `string`, `bytes`, embedded messages, packed repeats.
    Bytes(&'a [u8]),
    /// Wire type 3 and 4: everything between the start and the matching end.
    ///
    /// Deprecated in the format and unused by this protocol, but read rather
    /// than refused: a reader that stopped here would stop on a payload it
    /// could otherwise have handed over whole.
    Group(&'a [u8]),
    /// Wire type 5: `fixed32`, `sfixed32`, `float`.
    Fixed32(u32),
}

impl<'a> Value<'a> {
    /// The varint, if this is one.
    #[must_use]
    pub const fn as_u64(self) -> Option<u64> {
        match self {
            // A `fixed64` is a `u64` that chose a different encoding, so a
            // caller asking for the number should not have to care which.
            Self::Varint(value) | Self::Fixed64(value) => Some(value),
            _ => None,
        }
    }

    /// The varint narrowed to `u32`, if it fits.
    #[must_use]
    pub fn as_u32(self) -> Option<u32> {
        u32::try_from(self.as_u64()?).ok()
    }

    /// A `bool`, which the format encodes as a varint.
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        Some(self.as_u64()? != 0)
    }

    /// A zig-zag `sint64`.
    #[must_use]
    pub fn as_sint64(self) -> Option<i64> {
        let raw = self.as_u64()?;
        // The standard zig-zag decode, written without a cast: `raw >> 1`
        // always fits, and the negative branch is spelled out so a reader can
        // check it against the encoding rather than against a bit trick.
        let half = i64::try_from(raw >> 1).ok()?;
        Some(if raw & 1 == 0 {
            half
        } else {
            half.checked_neg()?.checked_sub(1)?
        })
    }

    /// The bytes, if this is a length-delimited field or a group.
    #[must_use]
    pub const fn as_bytes(self) -> Option<&'a [u8]> {
        match self {
            Self::Bytes(bytes) | Self::Group(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// The bytes as UTF-8.
    ///
    /// `None` for a field that is not length-delimited *and* for one whose
    /// bytes are not UTF-8, because a `string` field carrying something else
    /// is a payload disagreeing with its own schema.
    #[must_use]
    pub fn as_str(self) -> Option<&'a str> {
        core::str::from_utf8(self.as_bytes()?).ok()
    }

    /// Read this field as a nested message.
    #[must_use]
    pub fn as_message(self) -> Option<Reader<'a>> {
        Some(Reader::new(self.as_bytes()?))
    }
}

/// One field: its number and its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field<'a> {
    /// The field number the encoder wrote.
    pub number: u32,
    /// What it carried.
    pub value: Value<'a>,
}

/// Walks a payload, one field at a time.
#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    rest: &'a [u8],
    failed: bool,
}

impl<'a> Reader<'a> {
    /// Read `payload`.
    #[must_use]
    pub const fn new(payload: &'a [u8]) -> Self {
        Self {
            rest: payload,
            failed: false,
        }
    }

    /// Bytes not yet read.
    #[must_use]
    pub const fn remaining(&self) -> &'a [u8] {
        self.rest
    }

    /// Whether a read has failed, after which nothing more is produced.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// The next field, or `None` at the end.
    ///
    /// Fuses on error: a caller looping over this cannot spin on one bad byte.
    #[allow(clippy::should_implement_trait, reason = "fallible and fusing")]
    pub fn next(&mut self) -> Option<Result<Field<'a>, Error>> {
        if self.failed || self.rest.is_empty() {
            return None;
        }
        match self.read_field() {
            Ok(field) => Some(Ok(field)),
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }

    /// The last field numbered `number`, if any.
    ///
    /// The *last*, because the format says a repeated scalar resolves to the
    /// one written last, and a payload that repeats a field is legal.
    ///
    /// # Errors
    ///
    /// [`Error`] if the payload is malformed before the search ends.
    pub fn find_last(mut self, number: u32) -> Result<Option<Value<'a>>, Error> {
        let mut found = None;
        while let Some(field) = self.next() {
            let field = field?;
            if field.number == number {
                found = Some(field.value);
            }
        }
        Ok(found)
    }

    fn read_field(&mut self) -> Result<Field<'a>, Error> {
        let tag = self.read_varint()?;
        let number = u32::try_from(tag >> 3).map_err(|_| Error::MalformedVarint)?;
        if number == 0 {
            return Err(Error::ZeroFieldNumber);
        }
        let wire = u8::try_from(tag & 7).unwrap_or(u8::MAX);

        let value = match wire {
            0 => Value::Varint(self.read_varint()?),
            1 => Value::Fixed64(u64::from_le_bytes(self.read_array::<8>()?)),
            2 => {
                let len =
                    usize::try_from(self.read_varint()?).map_err(|_| Error::UnexpectedEnd {
                        needed: usize::MAX,
                        available: self.rest.len(),
                    })?;
                Value::Bytes(self.take(len)?)
            }
            3 => Value::Group(self.read_group(number)?),
            4 => return Err(Error::UnexpectedGroupEnd { number }),
            5 => Value::Fixed32(u32::from_le_bytes(self.read_array::<4>()?)),
            other => return Err(Error::UnknownWireType(other)),
        };
        Ok(Field { number, value })
    }

    /// Everything between a start-group tag and its matching end.
    fn read_group(&mut self, number: u32) -> Result<&'a [u8], Error> {
        let body = self.rest;
        let mut depth = 1usize;
        loop {
            let before = self.rest.len();
            let tag = self.read_varint()?;
            let inner = u32::try_from(tag >> 3).map_err(|_| Error::MalformedVarint)?;
            if inner == 0 {
                return Err(Error::ZeroFieldNumber);
            }
            match tag & 7 {
                0 => {
                    self.read_varint()?;
                }
                1 => {
                    self.read_array::<8>()?;
                }
                2 => {
                    let len = payload_len(self.read_varint()?, self.rest.len())?;
                    self.take(len)?;
                }
                3 => depth = depth.saturating_add(1),
                4 => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        if inner != number {
                            return Err(Error::UnexpectedGroupEnd { number: inner });
                        }
                        // Everything before the tag that closed it.
                        let end = body.len().saturating_sub(before);
                        return body.get(..end).ok_or(Error::UnterminatedGroup { number });
                    }
                }
                5 => {
                    self.read_array::<4>()?;
                }
                other => {
                    return Err(Error::UnknownWireType(
                        u8::try_from(other).unwrap_or(u8::MAX),
                    ));
                }
            }
            if self.rest.is_empty() {
                return Err(Error::UnterminatedGroup { number });
            }
        }
    }

    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut value = 0u64;
        for index in 0u32..10 {
            let byte = *self.rest.first().ok_or(Error::MalformedVarint)?;
            self.rest = self.rest.get(1..).unwrap_or(&[]);
            value |= u64::from(byte & 0x7F) << index.saturating_mul(7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error::MalformedVarint)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let bytes = self.take(N)?;
        <[u8; N]>::try_from(bytes).map_err(|_| Error::UnexpectedEnd {
            needed: N,
            available: 0,
        })
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let (head, tail) = self.rest.split_at_checked(n).ok_or(Error::UnexpectedEnd {
            needed: n,
            available: self.rest.len(),
        })?;
        self.rest = tail;
        Ok(head)
    }
}

/// Narrow a length prefix to this target's `usize`.
///
/// A free function so the limit stays testable: `usize` is 32-bit on wasm32 and
/// on the ESP32 targets this ecosystem builds for, and a length written by
/// somebody else can exceed it there. Reaching the branch through the reader
/// would need a four-gigabyte payload on a 64-bit host.
fn payload_len(raw: u64, available: usize) -> Result<usize, Error> {
    usize::try_from(raw).map_err(|_| Error::UnexpectedEnd {
        needed: usize::MAX,
        available,
    })
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
