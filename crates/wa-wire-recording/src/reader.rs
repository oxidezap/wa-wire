//! Reading a recording.
//!
//! Borrowing throughout, like the envelope decoder it wraps: a recording is a
//! buffer someone already has, and the reader hands out views into it rather
//! than copies of it.

use wa_wire_contract::Provenance;

use crate::crc::Crc32;
use crate::error::ReadError;
use crate::meta::{AdapterMeta, ArtifactClass, CapabilityNames, MetaEntry, Tag};
use crate::record::{Kind, Record};

/// The bytes before the metadata block: magic, version, metadata length.
pub const HEADER_LEN: usize = 10;

/// The magic every recording starts with.
pub const MAGIC: [u8; 4] = *b"WAWR";

/// The container layout this build writes and reads.
pub const CONTAINER_VERSION: u16 = 1;

/// Whether a recording ends where it says it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrity {
    /// The trailer was found and its count and checksum both hold.
    Complete,
    /// The trailer was found and disagrees with the records before it.
    ///
    /// The records are still readable. What is not trustworthy is the claim
    /// that they are all of them, or that none was altered.
    Damaged {
        /// Records the trailer claimed.
        claimed: u32,
        /// Records actually found.
        found: u32,
        /// Whether the checksum matched.
        checksum_ok: bool,
    },
    /// The trailer was found and something follows it.
    ///
    /// Distinct from [`Damaged`] because nothing before the trailer is wrong:
    /// the count holds, the checksum holds, and the file still is not what it
    /// says it is. The checksum cannot cover this — it covers the bytes up to
    /// the trailer, which is everything the trailer knew about — so appended
    /// records read as a complete recording with traffic silently left out.
    ///
    /// What was appended is not read. It may be a second recording, a partial
    /// write, or padding, and guessing between them would be inventing the
    /// thing the trailer exists to state.
    ///
    /// [`Damaged`]: Self::Damaged
    TrailingBytes {
        /// Records the trailer accounted for.
        found: u32,
        /// Bytes after the trailer.
        trailing: usize,
    },
    /// No trailer: the writer was interrupted.
    ///
    /// Not an error (D-076). Every complete record before the cut is readable,
    /// and a crash recorder's most valuable artifact is by definition this one.
    Truncated {
        /// Records read before the cut.
        found: u32,
        /// Bytes left over that did not form a record.
        dangling: usize,
    },
}

impl Integrity {
    /// Whether the recording is whole.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// A recording borrowed from the buffer it was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordingRef<'a> {
    version: u16,
    meta: &'a [u8],
    body: &'a [u8],
    integrity: Integrity,
    /// Critical metadata tags this build does not implement.
    unknown_critical: u16,
    /// Record kinds this build does not implement.
    skipped_records: u32,
}

impl<'a> RecordingRef<'a> {
    /// Read a recording from `buf`.
    ///
    /// The header and the metadata block are validated up front, so every
    /// accessor afterwards is infallible. The records are walked to establish
    /// [`integrity`](Self::integrity) and are then re-walked lazily by
    /// [`records`](Self::records); walking twice costs one pass over a buffer
    /// the caller already holds and keeps the reader allocation-free.
    ///
    /// # Errors
    ///
    /// [`ReadError`] when the buffer is not a recording, is too short to hold a
    /// header, has a metadata block that runs past its end, or announces a
    /// container version this build does not implement. A recording whose
    /// *records* are cut short is not an error — see [`Integrity`].
    pub fn decode(buf: &'a [u8]) -> Result<Self, ReadError> {
        let (header, rest) = buf
            .split_at_checked(HEADER_LEN)
            .ok_or(ReadError::HeaderTooShort {
                needed: HEADER_LEN,
                available: buf.len(),
            })?;

        if header.get(..4) != Some(&MAGIC[..]) {
            return Err(ReadError::NotARecording);
        }
        let version = le_u16(header, 4).ok_or(ReadError::NotARecording)?;
        if version != CONTAINER_VERSION {
            return Err(ReadError::UnsupportedVersion(version));
        }
        let meta_len = le_u32(header, 6).ok_or(ReadError::NotARecording)? as usize;

        let (meta, body) = rest
            .split_at_checked(meta_len)
            .ok_or(ReadError::MetaOutOfBounds {
                claimed: meta_len,
                available: rest.len(),
            })?;

        let unknown_critical = validate_meta(meta)?;
        let (integrity, skipped_records) = walk(buf, meta_len, body);

        Ok(Self {
            version,
            meta,
            body,
            integrity,
            unknown_critical,
            skipped_records,
        })
    }

    /// The container layout the writer used.
    #[must_use]
    pub const fn container_version(&self) -> u16 {
        self.version
    }

    /// Whether the recording ends where it says it does.
    #[must_use]
    pub const fn integrity(&self) -> Integrity {
        self.integrity
    }

    /// How many critical metadata tags this build could not interpret.
    ///
    /// Any at all means the recording must not be treated as comparable
    /// (D-077); it may still be read and shown.
    #[must_use]
    pub const fn unknown_critical_tags(&self) -> u16 {
        self.unknown_critical
    }

    /// How many records this build skipped because it does not know the kind.
    #[must_use]
    pub const fn skipped_records(&self) -> u32 {
        self.skipped_records
    }

    /// Every metadata entry, in the order written.
    pub fn meta(&self) -> impl Iterator<Item = MetaEntry<'a>> + use<'a> {
        let mut rest = self.meta;
        core::iter::from_fn(move || {
            let (tag, value, tail) = read_entry(rest)?;
            rest = tail;
            Some(MetaEntry { tag, value })
        })
    }

    /// The value of `tag`, if the recording carries it.
    ///
    /// The first wins. A writer here refuses to emit a duplicate, so a second
    /// one can only come from a writer that does not, and taking the first is
    /// the reading every implementation can agree on without coordination.
    #[must_use]
    pub fn value(&self, tag: Tag) -> Option<&'a [u8]> {
        self.meta()
            .find(|entry| entry.tag == tag)
            .map(|entry| entry.value)
    }

    /// Who produced this recording.
    #[must_use]
    pub fn adapter(&self) -> Option<AdapterMeta<'a>> {
        let value = self.value(Tag::ADAPTER)?;
        let (id, rest) = read_str(value)?;
        let (version, rest) = read_str(rest)?;
        let (engine_version, rest) = read_str(rest)?;
        let contract_version = le_u16(rest, 0)?;
        let count = le_u16(rest, 2)?;
        Some(AdapterMeta {
            id,
            version,
            engine_version,
            contract_version,
            capabilities: CapabilityNames {
                bytes: rest.get(4..)?,
                count,
            },
        })
    }

    /// Which `whatspec` build the producer's derivation came from.
    #[must_use]
    pub fn provenance(&self) -> Option<Provenance<'a>> {
        let value = self.value(Tag::PROVENANCE)?;
        let (whatsapp_version, rest) = read_str(value)?;
        let (manifest_hash, rest) = read_str(rest)?;
        let (generator_version, _) = read_str(rest)?;
        Some(Provenance::new(
            whatsapp_version,
            manifest_hash,
            generator_version,
        ))
    }

    /// Which token dictionary the frames were encoded against.
    #[must_use]
    pub fn dictionary(&self) -> Option<&'a str> {
        read_str(self.value(Tag::DICTIONARY)?).map(|(value, _)| value)
    }

    /// How this recording came to exist.
    ///
    /// `None` when the tag is absent *or* names a class this build does not
    /// know. Both mean the same thing to a caller: the class cannot be
    /// established, so the recording is not comparable.
    #[must_use]
    pub fn artifact_class(&self) -> Option<ArtifactClass> {
        ArtifactClass::from_byte(*self.value(Tag::ARTIFACT_CLASS)?.first()?)
    }

    /// The traffic this recording is a replay of.
    ///
    /// Absent for a capture, which is what makes a capture an input to a
    /// comparison rather than a result from one (D-079).
    #[must_use]
    pub fn input_digest(&self) -> Option<&'a [u8]> {
        self.value(Tag::INPUT_DIGEST)
    }

    /// For a sanitized recording: the transformation's identity and the digest
    /// of its configuration.
    #[must_use]
    pub fn transform(&self) -> Option<(&'a str, &'a str)> {
        let value = self.value(Tag::TRANSFORM)?;
        let (identity, rest) = read_str(value)?;
        let (config_digest, _) = read_str(rest)?;
        Some((identity, config_digest))
    }

    /// Wall clock at the first record, milliseconds since the Unix epoch.
    #[must_use]
    pub fn created_at(&self) -> Option<u64> {
        let value = self.value(Tag::CREATED_AT)?;
        let bytes: [u8; 8] = value.get(..8)?.try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }

    /// Free text the writer left for a human.
    #[must_use]
    pub fn note(&self) -> Option<&'a str> {
        core::str::from_utf8(self.value(Tag::NOTE)?).ok()
    }

    /// Every record, in order, stopping at the trailer or at the cut.
    pub fn records(&self) -> impl Iterator<Item = Record<'a>> + use<'a> {
        let mut rest = self.body;
        core::iter::from_fn(move || {
            let (record, tail) = read_record(rest)?;
            if record.kind == Kind::TRAILER {
                return None;
            }
            rest = tail;
            Some(record)
        })
    }

    /// Every envelope, in order.
    ///
    /// Records of other kinds are passed over: a consumer comparing traffic
    /// wants the stanzas, and a mark is an annotation about them rather than
    /// one of them.
    pub fn envelopes(&self) -> impl Iterator<Item = &'a [u8]> + use<'a> {
        self.records()
            .filter(|record| record.kind == Kind::ENVELOPE)
            .map(|record| record.payload)
    }

    /// How many envelopes it holds.
    #[must_use]
    pub fn envelope_count(&self) -> usize {
        self.envelopes().count()
    }
}

/// Walk the records once, to establish integrity and count skipped kinds.
fn walk(buf: &[u8], meta_len: usize, body: &[u8]) -> (Integrity, u32) {
    let mut rest = body;
    let mut found: u32 = 0;
    let mut skipped: u32 = 0;

    loop {
        let Some((record, tail)) = read_record(rest) else {
            // Nothing more that forms a whole record. Whatever is left is the
            // tail of a write that stopped part way.
            return (
                Integrity::Truncated {
                    found,
                    dangling: rest.len(),
                },
                skipped,
            );
        };

        if record.kind == Kind::TRAILER {
            let claimed = le_u32(record.payload, 0).unwrap_or(0);
            let stated = le_u32(record.payload, 4).unwrap_or(0);
            // Everything before the trailer record itself: header, metadata,
            // and every record it counts.
            let upto = HEADER_LEN
                .saturating_add(meta_len)
                .saturating_add(body.len().saturating_sub(rest.len()));
            let actual = Crc32::new().update(buf.get(..upto).unwrap_or(&[])).finish();
            let checksum_ok = actual == stated;

            // `tail` is what follows the trailer; `rest` still holds the
            // trailer itself.
            let integrity = if !tail.is_empty() {
                // The trailer says the recording ends here and it does not.
                // Reported before the checksum, which cannot see this: it
                // covers the bytes up to the trailer, so appended records
                // leave every earlier claim holding and the file still wrong.
                Integrity::TrailingBytes {
                    found,
                    trailing: tail.len(),
                }
            } else if checksum_ok && claimed == found {
                Integrity::Complete
            } else {
                Integrity::Damaged {
                    claimed,
                    found,
                    checksum_ok,
                }
            };
            return (integrity, skipped);
        }

        if !record.kind.is_known() {
            skipped = skipped.saturating_add(1);
        }
        found = found.saturating_add(1);
        rest = tail;
    }
}

/// Check every metadata entry parses, and count critical tags we do not know.
fn validate_meta(meta: &[u8]) -> Result<u16, ReadError> {
    let mut rest = meta;
    let mut unknown_critical: u16 = 0;
    while !rest.is_empty() {
        // A partial entry is a header fault rather than truncation: the
        // metadata block declared its own length, so it cannot end early.
        let Some((tag, _, tail)) = read_entry(rest) else {
            let tag = le_u16(rest, 0).unwrap_or(0);
            return Err(ReadError::MalformedMeta { tag });
        };
        if tag.is_critical() && !tag.is_known() {
            unknown_critical = unknown_critical.saturating_add(1);
        }
        rest = tail;
    }
    Ok(unknown_critical)
}

fn read_entry(rest: &[u8]) -> Option<(Tag, &[u8], &[u8])> {
    let tag = Tag(le_u16(rest, 0)?);
    let len = le_u32(rest, 2)? as usize;
    let (value, tail) = rest.get(6..)?.split_at_checked(len)?;
    Some((tag, value, tail))
}

fn read_record(rest: &[u8]) -> Option<(Record<'_>, &[u8])> {
    let kind = Kind(*rest.first()?);
    let len = le_u32(rest, 1)? as usize;
    let (payload, tail) = rest.get(5..)?.split_at_checked(len)?;
    Some((Record { kind, payload }, tail))
}

fn read_str(rest: &[u8]) -> Option<(&str, &[u8])> {
    let len = usize::from(le_u16(rest, 0)?);
    let (value, tail) = rest.get(2..)?.split_at_checked(len)?;
    Some((core::str::from_utf8(value).ok()?, tail))
}

fn le_u16(buf: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    let bytes: [u8; 2] = buf.get(at..end)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn le_u32(buf: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    let bytes: [u8; 4] = buf.get(at..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}
