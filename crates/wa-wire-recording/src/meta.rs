//! What a recording says about itself.
//!
//! Everything here exists to answer one question: may this recording be
//! compared against that one? A container that could not state which adapter,
//! which spec, which dictionary and which traffic produced it would not make
//! those claims absent — it would make them unverifiable, which is worse,
//! because a comparison would still run and still report a verdict.
//!
//! # Critical tags
//!
//! RFC-009 says unknown fields are preserved rather than dropped, and a reader
//! that meets an unknown tag here skips it for that reason. But some of these
//! fields *are* the basis on which two recordings may be compared, and silently
//! skipping one would produce a confident wrong verdict.
//!
//! So the high bit of a tag marks it critical (D-077). A reader that meets a
//! critical tag it does not understand may still show the recording to a human;
//! it may not call the recording comparable.

use core::fmt;

/// Marks a tag whose meaning a reader must understand to compare the recording.
pub const CRITICAL_BIT: u16 = 0x8000;

/// A metadata tag.
///
/// Held as a raw `u16` rather than an enum because an unknown tag is an
/// ordinary outcome, not an error: this reader must be able to carry a tag a
/// later writer invented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag(pub u16);

impl Tag {
    /// Who produced the recording: id, version, engine, contract, capabilities.
    pub const ADAPTER: Self = Self(CRITICAL_BIT | 0x0001);
    /// Which `whatspec` build the producer's derivation came from.
    pub const PROVENANCE: Self = Self(CRITICAL_BIT | 0x0002);
    /// Which token dictionary the frames were encoded against.
    pub const DICTIONARY: Self = Self(CRITICAL_BIT | 0x0003);
    /// Captured, replayed, sanitized or synthetic.
    pub const ARTIFACT_CLASS: Self = Self(CRITICAL_BIT | 0x0004);
    /// The traffic this recording is a replay *of*. Absent for a capture.
    pub const INPUT_DIGEST: Self = Self(CRITICAL_BIT | 0x0005);
    /// For a sanitized artifact: which transformation produced it.
    pub const TRANSFORM: Self = Self(CRITICAL_BIT | 0x0006);

    /// Wall clock at the first record, milliseconds since the Unix epoch.
    pub const CREATED_AT: Self = Self(0x0001);
    /// Free text for a human.
    pub const NOTE: Self = Self(0x0002);

    /// Whether a reader must understand this tag to compare the recording.
    #[must_use]
    pub const fn is_critical(self) -> bool {
        self.0 & CRITICAL_BIT != 0
    }

    /// Whether this reader implements the tag.
    #[must_use]
    pub const fn is_known(self) -> bool {
        matches!(
            self,
            Self::ADAPTER
                | Self::PROVENANCE
                | Self::DICTIONARY
                | Self::ARTIFACT_CLASS
                | Self::INPUT_DIGEST
                | Self::TRANSFORM
                | Self::CREATED_AT
                | Self::NOTE
        )
    }

    /// A stable name, for diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ADAPTER => "adapter",
            Self::PROVENANCE => "provenance",
            Self::DICTIONARY => "dictionary",
            Self::ARTIFACT_CLASS => "artifact_class",
            Self::INPUT_DIGEST => "input_digest",
            Self::TRANSFORM => "transform",
            Self::CREATED_AT => "created_at",
            Self::NOTE => "note",
            _ => "unknown",
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_known() {
            f.write_str(self.name())
        } else {
            write!(f, "tag {:#06x}", self.0)
        }
    }
}

/// How a recording came to exist.
///
/// The distinction is load-bearing rather than descriptive: a sanitized
/// recording has had its frames rewritten, so comparing one against a capture
/// compares a transformation's output with its input and calls the difference
/// an engine fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArtifactClass {
    /// Recorded from a live session. Its input was the session, so nothing
    /// else can have seen the same traffic (D-079).
    #[default]
    Captured,
    /// Produced by replaying another recording through an engine.
    Replayed,
    /// Derived from another recording by rewriting its frames.
    Sanitized,
    /// Written by hand or by a generator; no session behind it.
    Synthetic,
}

impl ArtifactClass {
    /// Pack into the on-wire byte.
    #[must_use]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::Captured => 0,
            Self::Replayed => 1,
            Self::Sanitized => 2,
            Self::Synthetic => 3,
        }
    }

    /// Unpack from the on-wire byte.
    ///
    /// An unrecognised class yields `None` rather than a default. The tag is
    /// critical, so a reader that cannot name the class must not treat the
    /// recording as comparable, and defaulting would hide exactly that.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Captured),
            1 => Some(Self::Replayed),
            2 => Some(Self::Sanitized),
            3 => Some(Self::Synthetic),
            _ => None,
        }
    }

    /// A stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::Replayed => "replayed",
            Self::Sanitized => "sanitized",
            Self::Synthetic => "synthetic",
        }
    }
}

impl fmt::Display for ArtifactClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One metadata entry, borrowed from the recording's own buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaEntry<'a> {
    /// Which field this is.
    pub tag: Tag,
    /// Its encoded value, unread.
    pub value: &'a [u8],
}

/// Who produced a recording.
///
/// Capabilities travel as their identifier strings rather than as the bitset
/// (D-085): the bit assignment is internal to one crate, while the identifiers
/// are stable and are literally what the TypeScript side holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterMeta<'a> {
    /// Stable adapter identifier.
    pub id: &'a str,
    /// The adapter's own version.
    pub version: &'a str,
    /// Which engine version it was built against.
    pub engine_version: &'a str,
    /// The contract version its envelopes speak.
    pub contract_version: u16,
    /// Capability identifiers, in the order written.
    pub capabilities: CapabilityNames<'a>,
}

/// The capability identifiers a recording declares.
///
/// Kept as the unread bytes so that a capability this build does not know
/// still round-trips and still shows up in a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityNames<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) count: u16,
}

impl<'a> CapabilityNames<'a> {
    /// How many were declared.
    #[must_use]
    pub const fn len(self) -> usize {
        self.count as usize
    }

    /// Whether none were declared.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.count == 0
    }

    /// Each identifier, in the order written.
    pub fn iter(self) -> impl Iterator<Item = &'a str> {
        let mut rest = self.bytes;
        (0..self.count).map_while(move |_| {
            let (len, tail) = rest.split_at_checked(2)?;
            let len = usize::from(u16::from_le_bytes([*len.first()?, *len.get(1)?]));
            let (value, tail) = tail.split_at_checked(len)?;
            rest = tail;
            core::str::from_utf8(value).ok()
        })
    }

    /// Whether `identifier` was declared.
    #[must_use]
    pub fn contains(self, identifier: &str) -> bool {
        self.iter().any(|name| name == identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    const ALL_TAGS: [Tag; 8] = [
        Tag::ADAPTER,
        Tag::PROVENANCE,
        Tag::DICTIONARY,
        Tag::ARTIFACT_CLASS,
        Tag::INPUT_DIGEST,
        Tag::TRANSFORM,
        Tag::CREATED_AT,
        Tag::NOTE,
    ];

    #[test]
    fn the_comparability_tags_are_critical_and_the_rest_are_not() {
        // This is the whole point of the bit: skipping one of the first six
        // silently would let a reader call two unlike recordings comparable.
        for tag in [
            Tag::ADAPTER,
            Tag::PROVENANCE,
            Tag::DICTIONARY,
            Tag::ARTIFACT_CLASS,
            Tag::INPUT_DIGEST,
            Tag::TRANSFORM,
        ] {
            assert!(tag.is_critical(), "{tag} decides comparability");
        }
        for tag in [Tag::CREATED_AT, Tag::NOTE] {
            assert!(!tag.is_critical(), "{tag} is decoration");
        }
    }

    #[test]
    fn tag_numbers_are_pinned() {
        // On the wire in files another language writes; changing one is a
        // contract break.
        assert_eq!(Tag::ADAPTER.0, 0x8001);
        assert_eq!(Tag::PROVENANCE.0, 0x8002);
        assert_eq!(Tag::DICTIONARY.0, 0x8003);
        assert_eq!(Tag::ARTIFACT_CLASS.0, 0x8004);
        assert_eq!(Tag::INPUT_DIGEST.0, 0x8005);
        assert_eq!(Tag::TRANSFORM.0, 0x8006);
        assert_eq!(Tag::CREATED_AT.0, 0x0001);
        assert_eq!(Tag::NOTE.0, 0x0002);
    }

    #[test]
    fn every_tag_has_a_distinct_name() {
        for (i, a) in ALL_TAGS.iter().enumerate() {
            assert!(a.is_known());
            assert_eq!(a.to_string(), a.name());
            for b in ALL_TAGS.iter().skip(i.saturating_add(1)) {
                assert_ne!(a.name(), b.name());
                assert_ne!(a.0, b.0);
            }
        }
    }

    #[test]
    fn an_unknown_tag_names_itself_by_number() {
        let unknown = Tag(0x00FF);
        assert!(!unknown.is_known());
        assert!(!unknown.is_critical());
        assert_eq!(unknown.name(), "unknown");
        assert!(unknown.to_string().contains("0x00ff"));

        let unknown_critical = Tag(CRITICAL_BIT | 0x00FF);
        assert!(!unknown_critical.is_known());
        assert!(unknown_critical.is_critical());
    }

    #[test]
    fn artifact_classes_round_trip_and_reject_the_unknown() {
        for class in [
            ArtifactClass::Captured,
            ArtifactClass::Replayed,
            ArtifactClass::Sanitized,
            ArtifactClass::Synthetic,
        ] {
            assert_eq!(ArtifactClass::from_byte(class.to_byte()), Some(class));
            assert!(!class.name().is_empty());
            assert_eq!(class.to_string(), class.name());
        }
        assert_eq!(ArtifactClass::default(), ArtifactClass::Captured);
        for byte in 4..=u8::MAX {
            assert_eq!(
                ArtifactClass::from_byte(byte),
                None,
                "byte {byte} must not decode to a class"
            );
        }
    }

    #[test]
    fn byte_assignments_are_pinned() {
        assert_eq!(ArtifactClass::Captured.to_byte(), 0);
        assert_eq!(ArtifactClass::Replayed.to_byte(), 1);
        assert_eq!(ArtifactClass::Sanitized.to_byte(), 2);
        assert_eq!(ArtifactClass::Synthetic.to_byte(), 3);
    }
}
