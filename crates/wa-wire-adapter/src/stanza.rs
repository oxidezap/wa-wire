//! What an engine hands over for one stanza.
//!
//! This is the pre-encoding shape of an envelope. An in-process consumer takes
//! it as it is and never pays for encoding; a sidecar consumer encodes it and
//! writes the bytes. Same value, two costs — which is why the sink receives
//! this rather than a finished buffer.

use wa_wire_contract::{
    Direction, EncodeError, EnvelopeBuilder, Flags, FrameOrigin, NodePath, PlaintextEntry,
    PlaintextStatus,
};

/// One decrypted payload the engine produced, addressed by node path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plaintext<'a> {
    /// Which node inside the frame this came from.
    pub path: NodePath<'a>,
    /// Whether the payload holds usable plaintext, and if not, why.
    pub status: PlaintextStatus,
    /// The decrypted bytes. Empty unless `status` is
    /// [`PlaintextStatus::Ok`].
    pub payload: &'a [u8],
}

impl<'a> Plaintext<'a> {
    /// A successful decryption.
    #[must_use]
    pub const fn ok(path: NodePath<'a>, payload: &'a [u8]) -> Self {
        Self {
            path,
            status: PlaintextStatus::Ok,
            payload,
        }
    }

    /// A decryption that was attempted and failed.
    ///
    /// The entry still travels, so a consumer can see *which* node failed
    /// rather than inferring it from a gap.
    #[must_use]
    pub const fn failed(path: NodePath<'a>) -> Self {
        Self {
            path,
            status: PlaintextStatus::DecryptFailed,
            payload: &[],
        }
    }

    /// A node the engine recognised but cannot decrypt.
    #[must_use]
    pub const fn unsupported(path: NodePath<'a>) -> Self {
        Self {
            path,
            status: PlaintextStatus::Unsupported,
            payload: &[],
        }
    }

    /// A node no plaintext ever arrived for.
    ///
    /// For an adapter that observes decryptions as they happen rather than
    /// being told the outcome of each one: it can say a node produced nothing,
    /// but not why. Claiming [`failed`](Self::failed) instead would put a cause
    /// it never verified into the record.
    #[must_use]
    pub const fn unobserved(path: NodePath<'a>) -> Self {
        Self {
            path,
            status: PlaintextStatus::Unobserved,
            payload: &[],
        }
    }
}

impl<'a> From<Plaintext<'a>> for PlaintextEntry<'a> {
    fn from(value: Plaintext<'a>) -> Self {
        Self {
            path: value.path,
            status: value.status,
            payload: value.payload,
        }
    }
}

/// One stanza as the engine observed it, before any encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawStanza<'a> {
    /// Which way the stanza was travelling.
    pub direction: Direction,
    /// Whether `frame` is the engine's own buffer or a re-encoding.
    pub frame_origin: FrameOrigin,
    /// The unpacked binary-node buffer, exactly as the engine's decoder
    /// consumed it.
    pub frame: &'a [u8],
    /// Payloads decrypted from nodes inside `frame`.
    pub plaintexts: &'a [Plaintext<'a>],
}

impl<'a> RawStanza<'a> {
    /// An inbound stanza carrying the engine's own frame bytes.
    #[must_use]
    pub const fn inbound(frame: &'a [u8]) -> Self {
        Self {
            direction: Direction::Inbound,
            frame_origin: FrameOrigin::Original,
            frame,
            plaintexts: &[],
        }
    }

    /// An outbound stanza carrying the engine's own frame bytes.
    #[must_use]
    pub const fn outbound(frame: &'a [u8]) -> Self {
        Self {
            direction: Direction::Outbound,
            frame_origin: FrameOrigin::Original,
            frame,
            plaintexts: &[],
        }
    }

    /// Attach decrypted payloads.
    #[must_use]
    pub const fn with_plaintexts(mut self, plaintexts: &'a [Plaintext<'a>]) -> Self {
        self.plaintexts = plaintexts;
        self
    }

    /// Mark the frame as re-encoded rather than verbatim.
    ///
    /// Only for an engine that cannot reach its own decode buffer. Saying this
    /// when the bytes *are* verbatim would make a consumer distrust a frame it
    /// could have relied on.
    #[must_use]
    pub const fn re_encoded(mut self) -> Self {
        self.frame_origin = FrameOrigin::ReEncoded;
        self
    }

    /// The contract flags this stanza implies.
    #[must_use]
    pub const fn flags(&self) -> Flags {
        Flags {
            direction: self.direction,
            frame_origin: self.frame_origin,
        }
    }

    /// Whether the frame is the engine's own buffer.
    #[must_use]
    pub const fn is_verbatim(&self) -> bool {
        matches!(self.frame_origin, FrameOrigin::Original)
    }

    /// An envelope builder over this stanza.
    #[must_use]
    pub fn to_builder(
        &self,
    ) -> EnvelopeBuilder<'a, impl Iterator<Item = PlaintextEntry<'a>> + Clone + use<'a>> {
        EnvelopeBuilder::new(self.flags(), self.frame)
            .with_entries(self.plaintexts.iter().copied().map(PlaintextEntry::from))
    }

    /// The encoded size of this stanza's envelope.
    pub fn encoded_len(&self) -> Result<usize, EncodeError> {
        self.to_builder().encoded_len()
    }

    /// Encode into `dst`, returning how many bytes were written.
    pub fn encode_into_slice(&self, dst: &mut [u8]) -> Result<usize, EncodeError> {
        self.to_builder().encode_into_slice(dst)
    }

    /// Encode into a freshly allocated vector.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        self.to_builder().encode_to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec;

    use wa_wire_contract::EnvelopeRef;

    use crate::path::NodePathBuf;

    fn path(components: &[u16]) -> NodePathBuf {
        let mut path = NodePathBuf::new();
        for component in components {
            path.push(*component).unwrap();
        }
        path
    }

    #[test]
    fn an_inbound_stanza_defaults_to_verbatim_and_carries_nothing_extra() {
        let stanza = RawStanza::inbound(b"frame");
        assert_eq!(stanza.direction, Direction::Inbound);
        assert!(stanza.is_verbatim());
        assert!(stanza.plaintexts.is_empty());
        assert_eq!(stanza.flags(), Flags::inbound());
    }

    #[test]
    fn an_outbound_stanza_flips_only_the_direction() {
        let stanza = RawStanza::outbound(b"frame");
        assert_eq!(stanza.direction, Direction::Outbound);
        assert!(stanza.is_verbatim());
        assert_eq!(stanza.flags(), Flags::outbound());
    }

    #[test]
    fn re_encoding_is_declared_in_the_flags() {
        let stanza = RawStanza::inbound(b"frame").re_encoded();
        assert!(!stanza.is_verbatim());
        assert_eq!(stanza.flags(), Flags::inbound().re_encoded());
        assert!(!stanza.flags().is_verbatim());
    }

    #[test]
    fn a_stanza_round_trips_through_its_envelope() {
        let first = path(&[0]);
        let second = path(&[1]);
        let plaintexts = [
            Plaintext::ok(first.as_path(), b"one"),
            Plaintext::failed(second.as_path()),
        ];
        let stanza = RawStanza::inbound(b"the-frame").with_plaintexts(&plaintexts);

        let bytes = stanza.encode_to_vec().expect("encodes");
        assert_eq!(bytes.len(), stanza.encoded_len().expect("sizes"));

        let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
        assert_eq!(envelope.frame(), b"the-frame");
        assert_eq!(envelope.flags(), Flags::inbound());
        assert_eq!(envelope.entry_count(), 2);

        let entries: alloc::vec::Vec<_> = envelope.entries().collect();
        assert_eq!(entries[0].path, first.as_path());
        assert_eq!(entries[0].status, PlaintextStatus::Ok);
        assert_eq!(entries[0].payload, b"one");
        assert_eq!(entries[1].path, second.as_path());
        assert_eq!(entries[1].status, PlaintextStatus::DecryptFailed);
        assert!(entries[1].payload.is_empty());
    }

    #[test]
    fn each_plaintext_constructor_sets_its_status_and_clears_the_payload() {
        let path = path(&[3]);
        let ok = Plaintext::ok(path.as_path(), b"body");
        assert_eq!(ok.status, PlaintextStatus::Ok);
        assert_eq!(ok.payload, b"body");

        let failed = Plaintext::failed(path.as_path());
        assert_eq!(failed.status, PlaintextStatus::DecryptFailed);
        assert!(failed.payload.is_empty());

        let unsupported = Plaintext::unsupported(path.as_path());
        assert_eq!(unsupported.status, PlaintextStatus::Unsupported);
        assert!(unsupported.payload.is_empty());

        for plaintext in [ok, failed, unsupported] {
            let entry = PlaintextEntry::from(plaintext);
            assert_eq!(entry.path, plaintext.path);
            assert_eq!(entry.status, plaintext.status);
            assert_eq!(entry.payload, plaintext.payload);
        }
    }

    #[test]
    fn encoding_into_a_slice_matches_the_reported_size() {
        let stanza = RawStanza::outbound(b"abc");
        let needed = stanza.encoded_len().expect("sizes");
        let mut buffer = vec![0u8; needed];
        assert_eq!(stanza.encode_into_slice(&mut buffer), Ok(needed));

        let envelope = EnvelopeRef::decode(&buffer).expect("decodes");
        assert_eq!(envelope.frame(), b"abc");
        assert_eq!(envelope.flags().direction, Direction::Outbound);
    }

    #[test]
    fn encoding_into_a_short_slice_fails_rather_than_truncating() {
        let stanza = RawStanza::inbound(b"frame");
        let needed = stanza.encoded_len().expect("sizes");
        let mut small = vec![0u8; needed - 1];
        assert!(stanza.encode_into_slice(&mut small).is_err());
    }

    #[test]
    fn a_re_encoded_stanza_survives_the_round_trip_as_such() {
        let stanza = RawStanza::inbound(b"f").re_encoded();
        let bytes = stanza.encode_to_vec().expect("encodes");
        let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
        assert!(!envelope.flags().is_verbatim());
        assert_eq!(envelope.flags().frame_origin, FrameOrigin::ReEncoded);
    }

    #[test]
    fn an_empty_frame_is_representable() {
        // Not a stanza any engine emits, but the type must not special-case it.
        let stanza = RawStanza::inbound(&[]);
        let bytes = stanza.encode_to_vec().expect("encodes");
        let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
        assert!(envelope.frame().is_empty());
    }

    #[test]
    fn stanzas_are_comparable() {
        assert_eq!(RawStanza::inbound(b"a"), RawStanza::inbound(b"a"));
        assert_ne!(RawStanza::inbound(b"a"), RawStanza::inbound(b"b"));
        assert_ne!(RawStanza::inbound(b"a"), RawStanza::outbound(b"a"));
        assert!(!alloc::format!("{:?}", RawStanza::inbound(b"a")).is_empty());
    }
}
