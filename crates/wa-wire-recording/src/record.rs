//! What a recording holds, one entry at a time.

use core::fmt;

/// What a record is.
///
/// A raw byte rather than an enum, for the same reason [`Tag`] is: a reader
/// must be able to walk past a kind a later writer invented, and an unknown
/// kind is an ordinary outcome rather than an error.
///
/// [`Tag`]: crate::meta::Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Kind(pub u8);

impl Kind {
    /// An RFC-008 envelope, verbatim.
    pub const ENVELOPE: Self = Self(0x00);
    /// An annotation about the traffic rather than part of it: a delta in
    /// microseconds from the recording's `created_at`, then a UTF-8 label.
    ///
    /// What a flight recorder writes when the thing worth investigating
    /// happens — "stream:error", "reconnect", "fault injected here".
    pub const MARK: Self = Self(0x01);
    /// The last record: how many came before it, and their checksum.
    pub const TRAILER: Self = Self(0xFF);

    /// Whether this build implements the kind.
    #[must_use]
    pub const fn is_known(self) -> bool {
        matches!(self, Self::ENVELOPE | Self::MARK | Self::TRAILER)
    }

    /// A stable name, for diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ENVELOPE => "envelope",
            Self::MARK => "mark",
            Self::TRAILER => "trailer",
            _ => "unknown",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_known() {
            f.write_str(self.name())
        } else {
            write!(f, "kind {:#04x}", self.0)
        }
    }
}

/// One record, borrowed from the recording's buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record<'a> {
    /// What it is.
    pub kind: Kind,
    /// Its bytes, unread.
    pub payload: &'a [u8],
}

impl<'a> Record<'a> {
    /// Read this record as a mark, if it is one.
    #[must_use]
    pub fn as_mark(&self) -> Option<Mark<'a>> {
        if self.kind != Kind::MARK {
            return None;
        }
        let bytes: [u8; 4] = self.payload.get(..4)?.try_into().ok()?;
        Some(Mark {
            delta_us: u32::from_le_bytes(bytes),
            label: core::str::from_utf8(self.payload.get(4..)?).ok()?,
        })
    }

    /// Read this record as an envelope's bytes, if it is one.
    #[must_use]
    pub const fn as_envelope(&self) -> Option<&'a [u8]> {
        if matches!(self.kind, Kind::ENVELOPE) {
            Some(self.payload)
        } else {
            None
        }
    }
}

/// An annotation placed between stanzas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark<'a> {
    /// Microseconds after the recording's `created_at`.
    pub delta_us: u32,
    /// What happened.
    pub label: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn kind_numbers_are_pinned() {
        // On the wire in files another language writes.
        assert_eq!(Kind::ENVELOPE.0, 0x00);
        assert_eq!(Kind::MARK.0, 0x01);
        assert_eq!(Kind::TRAILER.0, 0xFF);
    }

    #[test]
    fn every_known_kind_names_itself() {
        for kind in [Kind::ENVELOPE, Kind::MARK, Kind::TRAILER] {
            assert!(kind.is_known());
            assert_eq!(kind.to_string(), kind.name());
        }
        let unknown = Kind(0x42);
        assert!(!unknown.is_known());
        assert_eq!(unknown.name(), "unknown");
        assert!(unknown.to_string().contains("0x42"));
    }

    #[test]
    fn a_record_reads_as_what_it_is_and_not_as_what_it_is_not() {
        let envelope = Record {
            kind: Kind::ENVELOPE,
            payload: b"bytes",
        };
        assert_eq!(envelope.as_envelope(), Some(&b"bytes"[..]));
        assert_eq!(envelope.as_mark(), None);

        let mark = Record {
            kind: Kind::MARK,
            payload: b"\x10\x00\x00\x00stream:error",
        };
        assert_eq!(mark.as_envelope(), None);
        assert_eq!(
            mark.as_mark(),
            Some(Mark {
                delta_us: 16,
                label: "stream:error"
            })
        );
    }

    #[test]
    fn a_malformed_mark_reads_as_none_rather_than_panicking() {
        // Written by another language, so a short or non-UTF-8 payload is an
        // ordinary input.
        assert_eq!(
            Record {
                kind: Kind::MARK,
                payload: b"\x01\x02"
            }
            .as_mark(),
            None,
            "too short for the delta"
        );
        assert_eq!(
            Record {
                kind: Kind::MARK,
                payload: b"\x00\x00\x00\x00\xff\xfe"
            }
            .as_mark(),
            None,
            "label is not UTF-8"
        );
        // The delta alone, with an empty label, is well-formed.
        assert_eq!(
            Record {
                kind: Kind::MARK,
                payload: b"\x00\x00\x00\x00"
            }
            .as_mark(),
            Some(Mark {
                delta_us: 0,
                label: ""
            })
        );
    }
}
