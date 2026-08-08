//! Writing a recording.
//!
//! Append-only, and the trailer is written last. That ordering is the whole
//! reason the record count is not in the header: a writer that had to state its
//! length before the first byte could not be a ring buffer, and the
//! flight-recorder use is a ring buffer by definition (D-075).
//!
//! A consequence worth stating plainly: **a process killed before
//! [`finish`](RecordingWriter::finish) leaves a readable recording**. It has no
//! trailer, so it reads as [`Truncated`], every complete record in it is
//! usable, and it is not comparable. That is the intended outcome rather than a
//! tolerated one.
//!
//! [`Truncated`]: crate::Integrity::Truncated

extern crate alloc;
use alloc::vec::Vec;

use wa_wire_contract::Provenance;

use crate::crc::Crc32;
use crate::error::WriteError;
use crate::meta::{ArtifactClass, Tag};
use crate::reader::{CONTAINER_VERSION, MAGIC};
use crate::record::Kind;

/// Collects the metadata block before any record is written.
///
/// Separate from the writer because metadata comes first on the wire and
/// therefore must be complete before the first record: a builder that let you
/// add a tag afterwards would be a builder that cannot honour the request.
#[derive(Debug, Clone, Default)]
pub struct MetaBuilder {
    buf: Vec<u8>,
    tags: Vec<u16>,
}

impl MetaBuilder {
    /// An empty metadata block.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add a raw entry.
    ///
    /// # Errors
    ///
    /// [`WriteError::DuplicateTag`] if the tag was already written. A reader
    /// takes the first, so a duplicate is a value the writer believes it set
    /// and the reader will never see.
    pub fn raw(mut self, tag: Tag, value: &[u8]) -> Result<Self, WriteError> {
        if self.tags.contains(&tag.0) {
            return Err(WriteError::DuplicateTag(tag.0));
        }
        let len = meta_len_prefix(value.len())?;
        self.tags.push(tag.0);
        self.buf.extend_from_slice(&tag.0.to_le_bytes());
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(value);
        Ok(self)
    }

    /// Declare who produced the recording.
    ///
    /// Capabilities are written as their identifier strings (D-085).
    ///
    /// # Errors
    ///
    /// [`WriteError`] if a string exceeds its length prefix, or the tag repeats.
    pub fn adapter<'c, I>(
        self,
        id: &str,
        version: &str,
        engine_version: &str,
        contract_version: u16,
        capabilities: I,
    ) -> Result<Self, WriteError>
    where
        I: IntoIterator<Item = &'c str>,
    {
        let mut value = Vec::new();
        push_str(&mut value, id)?;
        push_str(&mut value, version)?;
        push_str(&mut value, engine_version)?;
        value.extend_from_slice(&contract_version.to_le_bytes());

        let mut names = Vec::new();
        let mut written: usize = 0;
        for capability in capabilities {
            push_str(&mut names, capability)?;
            written = written.saturating_add(1);
        }
        let count = capability_count_prefix(written)?;
        value.extend_from_slice(&count.to_le_bytes());
        value.extend_from_slice(&names);

        self.raw(Tag::ADAPTER, &value)
    }

    /// Declare which `whatspec` build the derivation came from.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if a string exceeds its length prefix, or the tag repeats.
    pub fn provenance(self, provenance: &Provenance<'_>) -> Result<Self, WriteError> {
        let mut value = Vec::new();
        push_str(&mut value, provenance.whatsapp_version)?;
        push_str(&mut value, provenance.manifest_hash)?;
        push_str(&mut value, provenance.generator_version)?;
        self.raw(Tag::PROVENANCE, &value)
    }

    /// Declare which token dictionary the frames were encoded against.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if the identity exceeds its length prefix, or the tag
    /// repeats.
    pub fn dictionary(self, identity: &str) -> Result<Self, WriteError> {
        let mut value = Vec::new();
        push_str(&mut value, identity)?;
        self.raw(Tag::DICTIONARY, &value)
    }

    /// Declare how this recording came to exist.
    ///
    /// # Errors
    ///
    /// [`WriteError::DuplicateTag`] if the tag repeats.
    pub fn artifact_class(self, class: ArtifactClass) -> Result<Self, WriteError> {
        self.raw(Tag::ARTIFACT_CLASS, &[class.to_byte()])
    }

    /// Declare the traffic this recording is a replay of.
    ///
    /// Never set by a live capture: a capture's input was the session, so
    /// nothing else can have seen it (D-079).
    ///
    /// # Errors
    ///
    /// [`WriteError::DuplicateTag`] if the tag repeats.
    pub fn input_digest(self, digest: &[u8]) -> Result<Self, WriteError> {
        self.raw(Tag::INPUT_DIGEST, digest)
    }

    /// Declare which transformation produced a sanitized recording.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if a string exceeds its length prefix, or the tag repeats.
    pub fn transform(self, identity: &str, config_digest: &str) -> Result<Self, WriteError> {
        let mut value = Vec::new();
        push_str(&mut value, identity)?;
        push_str(&mut value, config_digest)?;
        self.raw(Tag::TRANSFORM, &value)
    }

    /// Record the wall clock at the first record.
    ///
    /// # Errors
    ///
    /// [`WriteError::DuplicateTag`] if the tag repeats.
    pub fn created_at(self, millis_since_epoch: u64) -> Result<Self, WriteError> {
        self.raw(Tag::CREATED_AT, &millis_since_epoch.to_le_bytes())
    }

    /// Leave free text for a human.
    ///
    /// # Errors
    ///
    /// [`WriteError::DuplicateTag`] if the tag repeats.
    pub fn note(self, note: &str) -> Result<Self, WriteError> {
        self.raw(Tag::NOTE, note.as_bytes())
    }
}

/// Builds a recording, one record at a time.
#[derive(Debug, Clone)]
pub struct RecordingWriter {
    buf: Vec<u8>,
    records: u32,
}

impl RecordingWriter {
    /// Start a recording with `meta` as its metadata block.
    ///
    /// # Errors
    ///
    /// [`WriteError::MetaTooLong`] if the block exceeds its `u32` prefix.
    pub fn new(meta: MetaBuilder) -> Result<Self, WriteError> {
        let mut buf = meta.buf;
        let len = meta_len_prefix(buf.len())?;

        // Destructured rather than indexed: the header's shape is checked by
        // the compiler, and splicing it onto the front consumes the builder's
        // buffer instead of copying the metadata into a second one.
        let [m0, m1, m2, m3] = MAGIC;
        let [v0, v1] = CONTAINER_VERSION.to_le_bytes();
        let [l0, l1, l2, l3] = len.to_le_bytes();
        buf.splice(0..0, [m0, m1, m2, m3, v0, v1, l0, l1, l2, l3]);

        Ok(Self { buf, records: 0 })
    }

    /// Append one envelope.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if the envelope or the record count exceeds its prefix.
    pub fn envelope(&mut self, envelope: &[u8]) -> Result<&mut Self, WriteError> {
        self.record(Kind::ENVELOPE, envelope)
    }

    /// Append a mark: what happened, and how long after the recording started.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if the record or the count exceeds its prefix.
    pub fn mark(&mut self, delta_us: u32, label: &str) -> Result<&mut Self, WriteError> {
        let mut payload = Vec::with_capacity(4usize.saturating_add(label.len()));
        payload.extend_from_slice(&delta_us.to_le_bytes());
        payload.extend_from_slice(label.as_bytes());
        self.record(Kind::MARK, &payload)
    }

    /// Append a record of any kind.
    ///
    /// # Errors
    ///
    /// [`WriteError`] if the payload or the count exceeds its prefix.
    pub fn record(&mut self, kind: Kind, payload: &[u8]) -> Result<&mut Self, WriteError> {
        let len = record_len_prefix(payload.len())?;
        let records = next_record_count(self.records)?;

        self.buf.push(kind.0);
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(payload);
        self.records = records;
        Ok(self)
    }

    /// How many records have been written.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.records
    }

    /// Whether nothing has been written yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records == 0
    }

    /// The bytes so far, without a trailer.
    ///
    /// What a ring buffer hands over when it is frozen, and what a reader sees
    /// as [`Truncated`]: complete, usable records with no claim that they are
    /// all of them.
    ///
    /// [`Truncated`]: crate::Integrity::Truncated
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Close the recording and hand over its bytes.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        let mut payload = [0u8; 8];
        // Checksums everything written so far — header, metadata, records —
        // which is exactly what the reader recomputes before the trailer.
        let checksum = Crc32::new().update(&self.buf).finish();
        payload
            .get_mut(..4)
            .unwrap_or(&mut [])
            .copy_from_slice(&self.records.to_le_bytes());
        payload
            .get_mut(4..)
            .unwrap_or(&mut [])
            .copy_from_slice(&checksum.to_le_bytes());

        self.buf.push(Kind::TRAILER.0);
        self.buf
            .extend_from_slice(&u32::try_from(payload.len()).unwrap_or(8).to_le_bytes());
        self.buf.extend_from_slice(&payload);
        self.buf
    }
}

fn push_str(out: &mut Vec<u8>, value: &str) -> Result<(), WriteError> {
    let len = str_len_prefix(value.len())?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

// Each length prefix gets its own narrowing helper, as in the envelope encoder
// and for the same reason: the limits stay testable without materialising a
// 4 GiB buffer just to reach the branch.

fn meta_len_prefix(len: usize) -> Result<u32, WriteError> {
    u32::try_from(len).map_err(|_| WriteError::MetaTooLong(len))
}

fn record_len_prefix(len: usize) -> Result<u32, WriteError> {
    u32::try_from(len).map_err(|_| WriteError::RecordTooLong(len))
}

fn str_len_prefix(len: usize) -> Result<u16, WriteError> {
    u16::try_from(len).map_err(|_| WriteError::StringTooLong(len))
}

fn capability_count_prefix(count: usize) -> Result<u16, WriteError> {
    u16::try_from(count).map_err(|_| WriteError::TooManyCapabilities(count))
}

fn next_record_count(current: u32) -> Result<u32, WriteError> {
    current
        .checked_add(1)
        .ok_or(WriteError::TooManyRecords(current))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prefix_reports_the_value_that_did_not_fit() {
        // Reached here rather than through the writer: the smallest input that
        // overflows a u32 length is four gigabytes of it.
        assert_eq!(meta_len_prefix(4), Ok(4));
        assert_eq!(record_len_prefix(4), Ok(4));
        assert_eq!(str_len_prefix(4), Ok(4));
        assert_eq!(capability_count_prefix(4), Ok(4));
        assert_eq!(next_record_count(4), Ok(5));

        let huge = usize::MAX;
        assert_eq!(meta_len_prefix(huge), Err(WriteError::MetaTooLong(huge)));
        assert_eq!(
            record_len_prefix(huge),
            Err(WriteError::RecordTooLong(huge))
        );
        assert_eq!(str_len_prefix(huge), Err(WriteError::StringTooLong(huge)));
        assert_eq!(
            capability_count_prefix(huge),
            Err(WriteError::TooManyCapabilities(huge))
        );
        assert_eq!(
            next_record_count(u32::MAX),
            Err(WriteError::TooManyRecords(u32::MAX))
        );
    }

    #[test]
    fn the_boundary_value_still_fits() {
        assert_eq!(str_len_prefix(usize::from(u16::MAX)), Ok(u16::MAX));
        assert_eq!(
            str_len_prefix(usize::from(u16::MAX).saturating_add(1)),
            Err(WriteError::StringTooLong(0x1_0000))
        );
        assert_eq!(next_record_count(u32::MAX.saturating_sub(1)), Ok(u32::MAX));
    }
}
