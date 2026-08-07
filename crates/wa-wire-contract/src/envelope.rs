//! The L0-plain envelope: frame bytes verbatim plus a plaintext side table.
//!
//! The node is never re-encoded here. The engine already decoded a buffer, and
//! that buffer is what travels; the plaintexts Signal produced travel beside it,
//! each addressed by the path of the node it belongs to. Parsing the frame is
//! the host's job, and only when something subscribed to L1.
//!
//! ```text
//! Envelope
//!   version      u16
//!   flags        u16
//!   frame_len    u32
//!   frame        u8[frame_len]
//!   pt_count     u16
//!   pt_entries   PlaintextEntry[pt_count]
//!
//! PlaintextEntry
//!   path_len     u8
//!   path         u16[path_len]      little-endian child indices from the root
//!   status       u8
//!   payload_len  u32
//!   payload      u8[payload_len]
//! ```
//!
//! Little-endian throughout. No padding: payloads are opaque byte strings, so
//! alignment would buy nothing and would cost the Go and TypeScript encoders
//! their simplicity.

use crate::error::{DecodeError, EncodeError, Field};
use crate::flags::Flags;
use crate::path::NodePath;
use crate::status::PlaintextStatus;
use crate::version::ContractVersion;

/// Bytes preceding the frame payload: version, flags, frame length.
pub const HEADER_LEN: usize = 8;

/// One decrypted payload, addressed by the path of the node it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaintextEntry<'a> {
    /// Which node inside the frame this plaintext belongs to.
    pub path: NodePath<'a>,
    /// Whether the payload holds usable plaintext, and if not, why.
    pub status: PlaintextStatus,
    /// The decrypted bytes. Empty unless `status` is
    /// [`PlaintextStatus::Ok`].
    pub payload: &'a [u8],
}

/// A decoded envelope borrowing from the buffer it was decoded from.
///
/// Decoding validates the whole envelope up front, so iterating
/// [`entries`](Self::entries) afterwards cannot fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeRef<'a> {
    version: ContractVersion,
    flags: Flags,
    frame: &'a [u8],
    table: &'a [u8],
    count: u16,
}

impl<'a> EnvelopeRef<'a> {
    /// The contract version the producer wrote.
    #[must_use]
    pub const fn version(&self) -> ContractVersion {
        self.version
    }

    /// Direction and frame provenance.
    #[must_use]
    pub const fn flags(&self) -> Flags {
        self.flags
    }

    /// The unpacked binary-node buffer, exactly as the engine's decoder
    /// consumed it.
    #[must_use]
    pub const fn frame(&self) -> &'a [u8] {
        self.frame
    }

    /// How many plaintext entries the envelope carries.
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.count as usize
    }

    /// Whether the envelope carries no plaintext at all — the case for every
    /// stanza that was never encrypted.
    #[must_use]
    pub const fn is_plaintext_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate the plaintext entries in the order the producer wrote them.
    pub fn entries(&self) -> impl Iterator<Item = PlaintextEntry<'a>> + use<'a> {
        let mut reader = Reader::new(self.table);
        // Bounded by the validated count, and `parse_entry` already succeeded
        // for each of them during `decode`.
        (0..self.count).map_while(move |_| parse_entry(&mut reader).ok())
    }

    /// The first entry addressing exactly `path`, if any.
    #[must_use]
    pub fn entry_at(&self, path: NodePath<'_>) -> Option<PlaintextEntry<'a>> {
        self.entries().find(|entry| entry.path == path)
    }

    /// Decode an envelope from `buf`.
    ///
    /// The whole envelope is validated: lengths must be consistent, reserved
    /// flag bits clear, statuses known, and no bytes may remain afterwards.
    pub fn decode(buf: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(buf);

        let version = ContractVersion::new(reader.u16(Field::Version)?);
        let flags = Flags::from_bits(reader.u16(Field::Flags)?)?;
        let frame_len = reader.u32(Field::FrameLen)?;
        let frame = reader.take_u32(frame_len, Field::Frame)?;
        let count = reader.u16(Field::PlaintextCount)?;

        let table_start = reader.position();
        for _ in 0..count {
            parse_entry(&mut reader)?;
        }
        let table = reader.consumed_since(table_start);

        let trailing = reader.remaining();
        if trailing != 0 {
            return Err(DecodeError::TrailingBytes(trailing));
        }

        Ok(Self {
            version,
            flags,
            frame,
            table,
            count,
        })
    }
}

/// Assembles an envelope.
///
/// Entries are supplied by an iterator so the frame and payload bytes are only
/// ever borrowed; nothing is copied until [`encode_into_slice`] or
/// [`encode_to_vec`] writes the output.
///
/// [`encode_into_slice`]: EnvelopeBuilder::encode_into_slice
/// [`encode_to_vec`]: EnvelopeBuilder::encode_to_vec
#[derive(Debug, Clone)]
pub struct EnvelopeBuilder<'a, I> {
    version: ContractVersion,
    flags: Flags,
    frame: &'a [u8],
    entries: I,
}

impl<'a> EnvelopeBuilder<'a, core::iter::Empty<PlaintextEntry<'a>>> {
    /// An envelope carrying only a frame, at the current contract version.
    #[must_use]
    pub fn new(flags: Flags, frame: &'a [u8]) -> Self {
        Self {
            version: ContractVersion::CURRENT,
            flags,
            frame,
            entries: core::iter::empty(),
        }
    }
}

impl<'a, I> EnvelopeBuilder<'a, I>
where
    I: Iterator<Item = PlaintextEntry<'a>> + Clone,
{
    /// Attach the plaintext entries.
    ///
    /// The iterator must be `Clone` because the encoder walks it twice: once to
    /// size the output, once to write it. That is what keeps encoding
    /// allocation-free.
    pub fn with_entries<J>(self, entries: J) -> EnvelopeBuilder<'a, J>
    where
        J: Iterator<Item = PlaintextEntry<'a>> + Clone,
    {
        EnvelopeBuilder {
            version: self.version,
            flags: self.flags,
            frame: self.frame,
            entries,
        }
    }

    /// Override the contract version. Only useful for compatibility tests;
    /// producers should emit [`ContractVersion::CURRENT`].
    #[must_use]
    pub const fn with_version(mut self, version: ContractVersion) -> Self {
        self.version = version;
        self
    }

    /// Exact encoded size in bytes.
    pub fn encoded_len(&self) -> Result<usize, EncodeError> {
        frame_len_prefix(self.frame.len())?;

        let mut total = HEADER_LEN
            .checked_add(self.frame.len())
            .and_then(|n| n.checked_add(2))
            .ok_or(EncodeError::LengthOverflow)?;

        let mut count: usize = 0;
        for entry in self.entries.clone() {
            count = count.checked_add(1).ok_or(EncodeError::LengthOverflow)?;
            total = total
                .checked_add(entry_len(&entry)?)
                .ok_or(EncodeError::LengthOverflow)?;
        }
        entry_count_prefix(count)?;

        Ok(total)
    }

    /// Write the envelope into `dst`, returning how many bytes were written.
    pub fn encode_into_slice(&self, dst: &mut [u8]) -> Result<usize, EncodeError> {
        let needed = self.encoded_len()?;
        if dst.len() < needed {
            return Err(EncodeError::BufferTooSmall {
                needed,
                available: dst.len(),
            });
        }

        // Every prefix below was already proved to fit by `encoded_len`. They
        // go through the same checked helpers rather than a cast, so a future
        // change to the sizing logic cannot start truncating on the wire.
        let frame_len = frame_len_prefix(self.frame.len())?;
        let count = entry_count_prefix(self.entries.clone().count())?;

        let mut writer = Writer::new(dst);
        writer.u16(self.version.get());
        writer.u16(self.flags.to_bits());
        writer.u32(frame_len);
        writer.bytes(self.frame);
        writer.u16(count);

        for entry in self.entries.clone() {
            let components = path_len_prefix(entry.path.len())?;
            let payload_len = payload_len_prefix(entry.payload.len())?;

            writer.u8(components);
            writer.bytes(entry.path.as_le_bytes());
            writer.u8(entry.status.to_byte());
            writer.u32(payload_len);
            writer.bytes(entry.payload);
        }

        debug_assert_eq!(writer.position(), needed);
        Ok(needed)
    }

    /// Encode into a freshly allocated vector.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let needed = self.encoded_len()?;
        let mut out = alloc::vec![0u8; needed];
        let written = self.encode_into_slice(&mut out)?;
        out.truncate(written);
        Ok(out)
    }
}

// Each length prefix gets its own narrowing helper. Keeping them as free
// functions means the limits stay testable without materialising a 4 GiB
// buffer just to reach the branch.

fn frame_len_prefix(len: usize) -> Result<u32, EncodeError> {
    u32::try_from(len).map_err(|_| EncodeError::FrameTooLong(len))
}

fn payload_len_prefix(len: usize) -> Result<u32, EncodeError> {
    u32::try_from(len).map_err(|_| EncodeError::PayloadTooLong(len))
}

fn path_len_prefix(components: usize) -> Result<u8, EncodeError> {
    u8::try_from(components).map_err(|_| EncodeError::PathTooLong(components))
}

fn entry_count_prefix(count: usize) -> Result<u16, EncodeError> {
    u16::try_from(count).map_err(|_| EncodeError::TooManyEntries(count))
}

fn entry_len(entry: &PlaintextEntry<'_>) -> Result<usize, EncodeError> {
    path_len_prefix(entry.path.len())?;
    payload_len_prefix(entry.payload.len())?;
    // path_len(1) + path + status(1) + payload_len(4) + payload
    entry
        .path
        .as_le_bytes()
        .len()
        .checked_add(6)
        .and_then(|n| n.checked_add(entry.payload.len()))
        .ok_or(EncodeError::LengthOverflow)
}

fn parse_entry<'a>(reader: &mut Reader<'a>) -> Result<PlaintextEntry<'a>, DecodeError> {
    let components = reader.u8(Field::PathLen)?;
    let path_bytes = reader.take(usize::from(components) * 2, Field::Path)?;
    let status = PlaintextStatus::from_byte(reader.u8(Field::Status)?)?;
    let payload_len = reader.u32(Field::PayloadLen)?;
    let payload = reader.take_u32(payload_len, Field::Payload)?;
    Ok(PlaintextEntry {
        path: NodePath::from_le_bytes(path_bytes),
        status,
        payload,
    })
}

// ---------------------------------------------------------------------------

/// Bounds-checked forward cursor. Every read either yields the exact bytes
/// requested or reports how far it fell short.
/// Tracking the unread tail rather than an index lets every read be a
/// `split_*_checked`, whose single failure branch is the real short-read case.
/// An index-based cursor would need bounds checks the invariants already rule
/// out — defensive arms that can never run and can never be tested.
struct Reader<'a> {
    buf: &'a [u8],
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, rest: buf }
    }

    fn position(&self) -> usize {
        self.buf.len().saturating_sub(self.rest.len())
    }

    const fn remaining(&self) -> usize {
        self.rest.len()
    }

    fn consumed_since(&self, start: usize) -> &'a [u8] {
        self.buf.get(start..self.position()).unwrap_or(&[])
    }

    fn eof(&self, needed: usize, field: Field) -> DecodeError {
        DecodeError::UnexpectedEof {
            needed,
            available: self.remaining(),
            field,
        }
    }

    fn take(&mut self, n: usize, field: Field) -> Result<&'a [u8], DecodeError> {
        let (head, tail) = self
            .rest
            .split_at_checked(n)
            .ok_or_else(|| self.eof(n, field))?;
        self.rest = tail;
        Ok(head)
    }

    /// Take a `u32`-declared length.
    ///
    /// The widening is lossless on every target this crate supports: `usize` is
    /// 32 or 64 bits and a `u32` fits in both, so there is no fallible
    /// conversion to test here.
    fn take_u32(&mut self, n: u32, field: Field) -> Result<&'a [u8], DecodeError> {
        self.take(n as usize, field)
    }

    fn take_array<const N: usize>(&mut self, field: Field) -> Result<&'a [u8; N], DecodeError> {
        let (head, tail) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or_else(|| self.eof(N, field))?;
        self.rest = tail;
        Ok(head)
    }

    fn u8(&mut self, field: Field) -> Result<u8, DecodeError> {
        let [byte] = *self.take_array::<1>(field)?;
        Ok(byte)
    }

    fn u16(&mut self, field: Field) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(*self.take_array::<2>(field)?))
    }

    fn u32(&mut self, field: Field) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(*self.take_array::<4>(field)?))
    }
}

/// Forward cursor over a destination proved large enough by `encoded_len`.
struct Writer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Writer<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    const fn position(&self) -> usize {
        self.pos
    }

    fn bytes(&mut self, src: &[u8]) {
        let end = self.pos.saturating_add(src.len());
        if let Some(dst) = self.buf.get_mut(self.pos..end) {
            dst.copy_from_slice(src);
            self.pos = end;
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    fn path(components: &[u16]) -> Vec<u8> {
        components.iter().flat_map(|c| c.to_le_bytes()).collect()
    }

    fn entry<'a>(p: &'a [u8], status: PlaintextStatus, payload: &'a [u8]) -> PlaintextEntry<'a> {
        PlaintextEntry {
            path: NodePath::from_le_bytes(p),
            status,
            payload,
        }
    }

    fn encode<'a>(flags: Flags, frame: &'a [u8], entries: &'a [PlaintextEntry<'a>]) -> Vec<u8> {
        EnvelopeBuilder::new(flags, frame)
            .with_entries(entries.iter().copied())
            .encode_to_vec()
            .expect("fixture must encode")
    }

    // -- round trips --------------------------------------------------------

    #[test]
    fn frame_only_round_trips() {
        let frame = b"\xf8\x03\x01\x02\x03";
        let bytes = encode(Flags::inbound(), frame, &[]);

        assert_eq!(bytes.len(), HEADER_LEN + frame.len() + 2);

        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.version(), ContractVersion::CURRENT);
        assert_eq!(env.flags(), Flags::inbound());
        assert_eq!(env.frame(), frame);
        assert_eq!(env.entry_count(), 0);
        assert!(env.is_plaintext_empty());
        assert_eq!(env.entries().count(), 0);
    }

    #[test]
    fn empty_frame_round_trips() {
        let bytes = encode(Flags::outbound(), &[], &[]);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.frame(), b"");
        assert_eq!(env.flags(), Flags::outbound());
    }

    #[test]
    fn entries_round_trip_in_order() {
        let p0 = path(&[0]);
        let p1 = path(&[1, 2]);
        let p2 = path(&[]);
        let entries = [
            entry(&p0, PlaintextStatus::Ok, b"first"),
            entry(&p1, PlaintextStatus::DecryptFailed, b""),
            entry(&p2, PlaintextStatus::Unsupported, b""),
        ];
        let bytes = encode(Flags::inbound(), b"frame", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");

        assert_eq!(env.entry_count(), 3);
        assert!(!env.is_plaintext_empty());
        let decoded = env.entries().collect::<Vec<_>>();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], entries[0]);
        assert_eq!(decoded[1], entries[1]);
        assert_eq!(decoded[2], entries[2]);
    }

    #[test]
    fn entries_can_be_iterated_repeatedly() {
        let p = path(&[3]);
        let entries = [entry(&p, PlaintextStatus::Ok, b"x")];
        let bytes = encode(Flags::inbound(), b"f", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");

        assert_eq!(env.entries().count(), 1);
        assert_eq!(env.entries().count(), 1, "iteration must not consume");
    }

    #[test]
    fn entry_at_finds_by_path_and_misses_cleanly() {
        let p1 = path(&[1]);
        let p2 = path(&[2]);
        let entries = [
            entry(&p1, PlaintextStatus::Ok, b"one"),
            entry(&p2, PlaintextStatus::Ok, b"two"),
        ];
        let bytes = encode(Flags::inbound(), b"f", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");

        let found = env.entry_at(NodePath::from_le_bytes(&p2)).expect("present");
        assert_eq!(found.payload, b"two");
        let missing = path(&[9]);
        assert_eq!(env.entry_at(NodePath::from_le_bytes(&missing)), None);
    }

    #[test]
    fn all_flag_combinations_round_trip() {
        for flags in [
            Flags::inbound(),
            Flags::outbound(),
            Flags::inbound().re_encoded(),
            Flags::outbound().re_encoded(),
        ] {
            let bytes = encode(flags, b"n", &[]);
            let env = EnvelopeRef::decode(&bytes).expect("decodes");
            assert_eq!(env.flags(), flags);
        }
    }

    #[test]
    fn a_deep_path_at_the_measured_maximum_round_trips() {
        // Real captures reach depth 9; the u8 prefix leaves ample headroom.
        let deep = path(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let entries = [entry(&deep, PlaintextStatus::Ok, b"deep")];
        let bytes = encode(Flags::inbound(), b"f", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        let got = env.entries().next().expect("one entry");
        assert_eq!(got.path.len(), 9);
        assert_eq!(
            got.path.iter().collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
    }

    #[test]
    fn a_large_frame_round_trips() {
        // Real captures peak around 433 KB.
        let frame = vec![0xABu8; 500_000];
        let bytes = encode(Flags::inbound(), &frame, &[]);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.frame().len(), frame.len());
        assert_eq!(env.frame(), frame.as_slice());
    }

    #[test]
    fn many_entries_round_trip() {
        let p = path(&[7]);
        let entries: Vec<_> = (0..1000)
            .map(|_| entry(&p, PlaintextStatus::Ok, b"payload"))
            .collect();
        let bytes = encode(Flags::inbound(), b"f", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.entry_count(), 1000);
        assert_eq!(env.entries().count(), 1000);
    }

    // -- wire layout is pinned ---------------------------------------------

    #[test]
    fn byte_layout_is_exact() {
        let p = path(&[258]);
        let entries = [entry(&p, PlaintextStatus::DecryptFailed, b"ab")];
        let bytes = encode(Flags::outbound(), b"\x01\x02", &entries);

        #[rustfmt::skip]
        let expected: Vec<u8> = vec![
            0x01, 0x00,             // version = 1
            0x01, 0x00,             // flags = outbound
            0x02, 0x00, 0x00, 0x00, // frame_len = 2
            0x01, 0x02,             // frame
            0x01, 0x00,             // pt_count = 1
            0x01,                   // path_len = 1 component
            0x02, 0x01,             // path[0] = 258 little-endian
            0x01,                   // status = DecryptFailed
            0x02, 0x00, 0x00, 0x00, // payload_len = 2
            b'a', b'b',             // payload
        ];
        assert_eq!(bytes, expected);
        assert_eq!(HEADER_LEN, 8);
    }

    // -- decode rejects malformed input ------------------------------------

    #[test]
    fn truncation_at_every_offset_is_rejected() {
        let p = path(&[1, 2]);
        let entries = [entry(&p, PlaintextStatus::Ok, b"payload")];
        let bytes = encode(Flags::inbound(), b"frame-bytes", &entries);

        for cut in 0..bytes.len() {
            let err =
                EnvelopeRef::decode(&bytes[..cut]).expect_err("truncated input must not decode");
            assert!(
                matches!(err, DecodeError::UnexpectedEof { .. }),
                "cut {cut} gave {err:?}"
            );
        }
        assert!(EnvelopeRef::decode(&bytes).is_ok());
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut bytes = encode(Flags::inbound(), b"f", &[]);
        bytes.push(0xFF);
        assert_eq!(
            EnvelopeRef::decode(&bytes),
            Err(DecodeError::TrailingBytes(1))
        );

        bytes.extend_from_slice(&[0, 0, 0]);
        assert_eq!(
            EnvelopeRef::decode(&bytes),
            Err(DecodeError::TrailingBytes(4))
        );
    }

    #[test]
    fn reserved_flag_bits_are_rejected() {
        let mut bytes = encode(Flags::inbound(), b"f", &[]);
        bytes[2] = 0x04; // first reserved bit
        assert_eq!(
            EnvelopeRef::decode(&bytes),
            Err(DecodeError::ReservedFlags(0x0004))
        );
    }

    #[test]
    fn unknown_status_is_rejected() {
        let p = path(&[0]);
        let entries = [entry(&p, PlaintextStatus::Ok, b"")];
        let mut bytes = encode(Flags::inbound(), b"f", &entries);
        // header(8) + frame(1) + count(2) + path_len(1) + path(2) = 14
        bytes[14] = 3;
        assert_eq!(
            EnvelopeRef::decode(&bytes),
            Err(DecodeError::InvalidStatus(3))
        );
    }

    #[test]
    fn an_overlong_declared_frame_is_rejected_not_allocated() {
        let mut bytes = encode(Flags::inbound(), b"f", &[]);
        bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        let err = EnvelopeRef::decode(&bytes).expect_err("must not decode");
        assert!(matches!(
            err,
            DecodeError::UnexpectedEof {
                field: Field::Frame,
                ..
            }
        ));
    }

    #[test]
    fn an_overlong_declared_payload_is_rejected() {
        let p = path(&[0]);
        let entries = [entry(&p, PlaintextStatus::Ok, b"xy")];
        let mut bytes = encode(Flags::inbound(), b"f", &entries);
        let len = bytes.len();
        bytes[len - 6..len - 2].copy_from_slice(&u32::MAX.to_le_bytes());
        let err = EnvelopeRef::decode(&bytes).expect_err("must not decode");
        assert!(matches!(
            err,
            DecodeError::UnexpectedEof {
                field: Field::Payload,
                ..
            }
        ));
    }

    #[test]
    fn an_overstated_entry_count_is_rejected() {
        let mut bytes = encode(Flags::inbound(), b"f", &[]);
        let count_at = HEADER_LEN + 1;
        bytes[count_at..count_at + 2].copy_from_slice(&7u16.to_le_bytes());
        let err = EnvelopeRef::decode(&bytes).expect_err("must not decode");
        assert!(matches!(err, DecodeError::UnexpectedEof { .. }), "{err:?}");
    }

    #[test]
    fn an_understated_entry_count_leaves_trailing_bytes() {
        let p = path(&[1]);
        let entries = [
            entry(&p, PlaintextStatus::Ok, b"a"),
            entry(&p, PlaintextStatus::Ok, b"b"),
        ];
        let mut bytes = encode(Flags::inbound(), b"f", &entries);
        let count_at = HEADER_LEN + 1;
        bytes[count_at..count_at + 2].copy_from_slice(&1u16.to_le_bytes());
        let err = EnvelopeRef::decode(&bytes).expect_err("must not decode");
        assert!(matches!(err, DecodeError::TrailingBytes(_)), "{err:?}");
    }

    #[test]
    fn an_empty_buffer_is_rejected() {
        let err = EnvelopeRef::decode(&[]).expect_err("must not decode");
        assert!(matches!(
            err,
            DecodeError::UnexpectedEof {
                field: Field::Version,
                ..
            }
        ));
    }

    // -- unknown versions still decode -------------------------------------

    #[test]
    fn a_future_version_decodes_so_the_host_can_report_it() {
        // Rejecting here would hide the mismatch behind a parse error; the
        // version check belongs to negotiation, which needs the value.
        let future = ContractVersion::new(999);
        let bytes = EnvelopeBuilder::new(Flags::inbound(), b"f")
            .with_version(future)
            .encode_to_vec()
            .expect("encodes");
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.version(), future);
    }

    // -- encode errors ------------------------------------------------------

    #[test]
    fn encoding_into_a_short_buffer_reports_the_shortfall() {
        let builder = EnvelopeBuilder::new(Flags::inbound(), b"frame");
        let needed = builder.encoded_len().expect("sizes");
        let mut small = vec![0u8; needed - 1];
        assert_eq!(
            builder.encode_into_slice(&mut small),
            Err(EncodeError::BufferTooSmall {
                needed,
                available: needed - 1,
            })
        );

        let mut exact = vec![0u8; needed];
        assert_eq!(builder.encode_into_slice(&mut exact), Ok(needed));

        let mut roomy = vec![0u8; needed + 16];
        assert_eq!(builder.encode_into_slice(&mut roomy), Ok(needed));
        assert!(EnvelopeRef::decode(&roomy[..needed]).is_ok());
    }

    #[test]
    fn an_overlong_path_is_rejected() {
        let long = path(&vec![1u16; 256]);
        let entries = [entry(&long, PlaintextStatus::Ok, b"")];
        let builder =
            EnvelopeBuilder::new(Flags::inbound(), b"f").with_entries(entries.iter().copied());
        assert_eq!(builder.encoded_len(), Err(EncodeError::PathTooLong(256)));
        assert_eq!(
            builder.encode_to_vec().unwrap_err(),
            EncodeError::PathTooLong(256)
        );
    }

    #[test]
    fn a_path_at_the_prefix_limit_is_accepted() {
        let max = path(&vec![1u16; 255]);
        let entries = [entry(&max, PlaintextStatus::Ok, b"")];
        let bytes = encode(Flags::inbound(), b"f", &entries);
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.entries().next().expect("entry").path.len(), 255);
    }

    #[test]
    fn too_many_entries_are_rejected() {
        // The count prefix is u16, so one entry past it must not silently wrap.
        let p = path(&[0]);
        let one = entry(&p, PlaintextStatus::Ok, b"");
        let over = usize::from(u16::MAX) + 1;

        let builder = EnvelopeBuilder::new(Flags::inbound(), b"f")
            .with_entries(core::iter::repeat_n(one, over));
        assert_eq!(
            builder.encoded_len(),
            Err(EncodeError::TooManyEntries(over))
        );
        assert_eq!(
            builder.encode_to_vec().unwrap_err(),
            EncodeError::TooManyEntries(over)
        );

        // Exactly u16::MAX still encodes.
        let at_limit = EnvelopeBuilder::new(Flags::inbound(), b"f")
            .with_entries(core::iter::repeat_n(one, usize::from(u16::MAX)));
        let bytes = at_limit.encode_to_vec().expect("encodes at the limit");
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.entry_count(), usize::from(u16::MAX));
    }

    #[test]
    fn length_prefixes_reject_what_they_cannot_represent() {
        // The frame and payload limits need a 4 GiB buffer to reach through the
        // builder, so the narrowing itself is checked directly.
        const OVER_U32: usize = u32::MAX as usize + 1;

        assert_eq!(frame_len_prefix(0), Ok(0));
        assert_eq!(frame_len_prefix(u32::MAX as usize), Ok(u32::MAX));
        assert_eq!(
            frame_len_prefix(OVER_U32),
            Err(EncodeError::FrameTooLong(OVER_U32))
        );

        assert_eq!(payload_len_prefix(7), Ok(7));
        assert_eq!(payload_len_prefix(u32::MAX as usize), Ok(u32::MAX));
        assert_eq!(
            payload_len_prefix(OVER_U32),
            Err(EncodeError::PayloadTooLong(OVER_U32))
        );

        assert_eq!(path_len_prefix(9), Ok(9));
        assert_eq!(path_len_prefix(255), Ok(255));
        assert_eq!(path_len_prefix(256), Err(EncodeError::PathTooLong(256)));

        assert_eq!(entry_count_prefix(0), Ok(0));
        assert_eq!(entry_count_prefix(65_535), Ok(65_535));
        assert_eq!(
            entry_count_prefix(65_536),
            Err(EncodeError::TooManyEntries(65_536))
        );
    }

    #[test]
    fn encoded_len_matches_what_encoding_writes() {
        let p = path(&[1, 2, 3]);
        let entries = [
            entry(&p, PlaintextStatus::Ok, b"alpha"),
            entry(&p, PlaintextStatus::Unsupported, b""),
        ];
        let builder =
            EnvelopeBuilder::new(Flags::inbound(), b"frame").with_entries(entries.iter().copied());
        let needed = builder.encoded_len().expect("sizes");
        let bytes = builder.encode_to_vec().expect("encodes");
        assert_eq!(bytes.len(), needed);
    }

    #[test]
    fn builder_defaults_to_the_current_version() {
        let builder = EnvelopeBuilder::new(Flags::inbound(), b"f");
        let bytes = builder.encode_to_vec().expect("encodes");
        let env = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(env.version(), ContractVersion::CURRENT);
    }

    // -- reader internals ---------------------------------------------------
    //
    // `parse_entry` is unreachable-by-construction from `entries()` once
    // `decode` has validated, so its failure paths are exercised directly.

    #[test]
    fn parse_entry_reports_each_truncation_point() {
        let cases: [(&[u8], Field); 5] = [
            (&[], Field::PathLen),
            (&[1], Field::Path),
            (&[1, 0, 0], Field::Status),
            (&[0, 0], Field::PayloadLen),
            (&[0, 0, 1, 0, 0, 0], Field::Payload),
        ];
        for (input, field) in cases {
            let mut reader = Reader::new(input);
            let err = parse_entry(&mut reader).expect_err("must fail");
            assert!(
                matches!(err, DecodeError::UnexpectedEof { field: got, .. } if got == field),
                "input {input:?} gave {err:?}"
            );
        }
    }

    #[test]
    fn parse_entry_rejects_an_unknown_status() {
        let mut reader = Reader::new(&[0, 200, 0, 0, 0, 0]);
        assert_eq!(
            parse_entry(&mut reader),
            Err(DecodeError::InvalidStatus(200))
        );
    }

    #[test]
    fn reader_take_is_bounds_checked() {
        let mut reader = Reader::new(&[1, 2, 3]);
        assert_eq!(reader.remaining(), 3);
        assert_eq!(reader.take(2, Field::Frame), Ok(&[1u8, 2][..]));
        assert_eq!(reader.position(), 2);
        assert_eq!(reader.remaining(), 1);
        assert_eq!(
            reader.take(2, Field::Frame),
            Err(DecodeError::UnexpectedEof {
                needed: 2,
                available: 1,
                field: Field::Frame,
            })
        );
        // A failed read must not advance the cursor.
        assert_eq!(reader.position(), 2);
        assert_eq!(reader.take(0, Field::Frame), Ok(&[][..]));
    }

    #[test]
    fn reader_rejects_an_offset_that_would_overflow() {
        let mut reader = Reader::new(&[1, 2, 3]);
        let err = reader
            .take(usize::MAX, Field::Frame)
            .expect_err("overflows");
        assert!(matches!(err, DecodeError::UnexpectedEof { .. }));
    }

    #[test]
    fn reader_scalars_are_little_endian() {
        let mut reader = Reader::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
        assert_eq!(reader.u8(Field::Status), Ok(0x01));
        assert_eq!(reader.u16(Field::Version), Ok(0x0302));
        assert_eq!(reader.u32(Field::FrameLen), Ok(0x0706_0504));
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn reader_scalars_report_eof() {
        assert!(Reader::new(&[]).u8(Field::Status).is_err());
        assert!(Reader::new(&[1]).u16(Field::Version).is_err());
        assert!(Reader::new(&[1, 2, 3]).u32(Field::FrameLen).is_err());
    }

    #[test]
    fn reader_consumed_since_returns_the_span() {
        let mut reader = Reader::new(&[1, 2, 3, 4]);
        let start = reader.position();
        reader.take(3, Field::Frame).expect("takes");
        assert_eq!(reader.consumed_since(start), &[1, 2, 3]);
        // A start beyond the cursor yields an empty span rather than panicking.
        assert_eq!(reader.consumed_since(99), &[] as &[u8]);
    }

    #[test]
    fn take_u32_handles_the_full_range() {
        let mut reader = Reader::new(&[7, 8]);
        assert_eq!(reader.take_u32(2, Field::Frame), Ok(&[7u8, 8][..]));
        let mut reader = Reader::new(&[7, 8]);
        assert!(reader.take_u32(u32::MAX, Field::Frame).is_err());
    }

    #[test]
    fn writer_writes_little_endian_and_tracks_position() {
        let mut buf = [0u8; 7];
        let mut writer = Writer::new(&mut buf);
        writer.u8(0x01);
        writer.u16(0x0302);
        writer.u32(0x0706_0504);
        assert_eq!(writer.position(), 7);
        assert_eq!(buf, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
    }

    #[test]
    fn writer_ignores_writes_past_the_end() {
        // Unreachable through the builder, which sizes first; kept total anyway.
        let mut buf = [0u8; 2];
        let mut writer = Writer::new(&mut buf);
        writer.u32(0xDEAD_BEEF);
        assert_eq!(writer.position(), 0);
        assert_eq!(buf, [0, 0]);
    }
}
