//! What one engine saw.
//!
//! A recording is a sequence of envelopes plus the adapter that produced them.
//! Capturing L0 rather than L1 is what makes replay possible at all: L0 is the
//! engine's output, so replaying it exercises everything downstream without
//! needing an account, a network, or the other party's data.

use wa_wire_adapter::AdapterInfo;
use wa_wire_contract::EnvelopeRef;

use crate::comparability::Comparability;

/// Envelopes captured from one engine, in arrival order.
#[derive(Debug, Clone, Copy)]
pub struct Recording<'a> {
    adapter: AdapterInfo<'a>,
    envelopes: &'a [&'a [u8]],
    comparability: Option<Comparability<'a>>,
}

impl<'a> Recording<'a> {
    /// Wrap captured envelopes.
    ///
    /// Carries no comparability declaration: a caller that assembled both sides
    /// in memory is vouching that they are of the same traffic. Compare that
    /// against a recording read from a container and the pair is refused, since
    /// half a checked claim leaves the pair unchecked.
    #[must_use]
    pub const fn new(adapter: AdapterInfo<'a>, envelopes: &'a [&'a [u8]]) -> Self {
        Self {
            adapter,
            envelopes,
            comparability: None,
        }
    }

    /// Declare what this recording is comparable to.
    #[must_use]
    pub const fn with_comparability(mut self, comparability: Comparability<'a>) -> Self {
        self.comparability = Some(comparability);
        self
    }

    /// What this recording declares about its own comparability.
    #[must_use]
    pub const fn comparability(&self) -> Option<Comparability<'a>> {
        self.comparability
    }

    /// Which adapter produced this.
    #[must_use]
    pub const fn adapter(&self) -> AdapterInfo<'a> {
        self.adapter
    }

    /// The adapter's id, for diagnostics.
    #[must_use]
    pub const fn id(&self) -> &'a str {
        self.adapter.id
    }

    /// How many stanzas were captured.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.envelopes.len()
    }

    /// Whether nothing was captured.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.envelopes.is_empty()
    }

    /// The raw envelope at `index`.
    #[must_use]
    pub fn envelope_bytes(&self, index: usize) -> Option<&'a [u8]> {
        self.envelopes.get(index).copied()
    }

    /// The decoded envelope at `index`, or `None` if it does not decode.
    #[must_use]
    pub fn envelope(&self, index: usize) -> Option<EnvelopeRef<'a>> {
        EnvelopeRef::decode(self.envelope_bytes(index)?).ok()
    }

    /// Every envelope in order, skipping nothing — a malformed one yields
    /// `None` in place rather than being dropped, so indices stay aligned
    /// between recordings.
    pub fn envelopes(&self) -> impl Iterator<Item = Option<EnvelopeRef<'a>>> + use<'a> {
        self.envelopes
            .iter()
            .map(|bytes| EnvelopeRef::decode(bytes).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;

    use wa_wire_adapter::{Capability, CapabilitySet, Flags, RawStanza};

    fn info() -> AdapterInfo<'static> {
        AdapterInfo::new(
            "test",
            "0.1.0",
            "1.0",
            CapabilitySet::NONE.with(Capability::L0InboundTap),
        )
    }

    fn envelope(frame: &[u8]) -> Vec<u8> {
        RawStanza::inbound(frame).encode_to_vec().expect("encodes")
    }

    #[test]
    fn a_recording_reports_what_it_holds() {
        let first = envelope(b"one");
        let second = envelope(b"two");
        let envelopes: [&[u8]; 2] = [&first, &second];
        let recording = Recording::new(info(), &envelopes);

        assert_eq!(recording.len(), 2);
        assert!(!recording.is_empty());
        assert_eq!(recording.id(), "test");
        assert_eq!(recording.adapter(), info());
    }

    #[test]
    fn an_empty_recording_is_reported_as_such() {
        let recording = Recording::new(info(), &[]);
        assert!(recording.is_empty());
        assert_eq!(recording.len(), 0);
        assert_eq!(recording.envelope(0), None);
        assert_eq!(recording.envelope_bytes(0), None);
        assert_eq!(recording.envelopes().count(), 0);
    }

    #[test]
    fn envelopes_decode_to_their_frames() {
        let first = envelope(b"one");
        let envelopes: [&[u8]; 1] = [&first];
        let recording = Recording::new(info(), &envelopes);

        let decoded = recording.envelope(0).expect("decodes");
        assert_eq!(decoded.frame(), b"one");
        assert_eq!(decoded.flags(), Flags::inbound());
        assert_eq!(recording.envelope_bytes(0), Some(&first[..]));
        assert_eq!(recording.envelope(1), None, "past the end");
    }

    #[test]
    fn a_malformed_envelope_holds_its_place() {
        // Dropping it would shift every later index and make two recordings
        // compare stanza N against stanza N+1.
        let good = envelope(b"ok");
        let envelopes: [&[u8]; 3] = [&good, b"not-an-envelope", &good];
        let recording = Recording::new(info(), &envelopes);

        assert_eq!(recording.len(), 3);
        let decoded: Vec<_> = recording.envelopes().collect();
        assert_eq!(decoded.len(), 3);
        assert!(decoded[0].is_some());
        assert!(decoded[1].is_none(), "malformed, but still an entry");
        assert!(decoded[2].is_some());
        assert_eq!(recording.envelope(1), None);
        assert_eq!(
            recording.envelope_bytes(1),
            Some(&b"not-an-envelope"[..]),
            "the bytes are still reachable"
        );
    }

    #[test]
    fn recordings_are_debuggable() {
        assert!(!alloc::format!("{:?}", Recording::new(info(), &[])).is_empty());
    }
}
