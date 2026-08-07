//! Where an adapter delivers stanzas.
//!
//! A sink receives the pre-encoding [`RawStanza`], not a finished buffer. That
//! is deliberate: an in-process consumer reads the frame straight out of it and
//! never pays for encoding, while a sidecar consumer encodes and writes. Same
//! value, two costs, one adapter.

use crate::stanza::RawStanza;

/// Receives stanzas from an adapter.
///
/// Implementations must not block for long and must not panic: an adapter calls
/// this from the engine's receive path, where stalling reorders delivery and
/// panicking takes the connection with it.
pub trait StanzaSink {
    /// Accept one stanza.
    fn accept(&mut self, stanza: RawStanza<'_>);
}

impl<F> StanzaSink for F
where
    F: FnMut(RawStanza<'_>),
{
    fn accept(&mut self, stanza: RawStanza<'_>) {
        self(stanza);
    }
}

/// A sink that discards everything.
///
/// Useful for measuring what an adapter costs when nothing consumes it, which
/// is the case the interest-driven design is supposed to make free.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NullSink;

impl StanzaSink for NullSink {
    fn accept(&mut self, _stanza: RawStanza<'_>) {}
}

/// A sink that counts what passes through it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CountingSink {
    stanzas: u64,
    frame_bytes: u64,
    plaintexts: u64,
    re_encoded: u64,
}

impl CountingSink {
    /// A fresh counter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stanzas: 0,
            frame_bytes: 0,
            plaintexts: 0,
            re_encoded: 0,
        }
    }

    /// How many stanzas passed through.
    #[must_use]
    pub const fn stanzas(&self) -> u64 {
        self.stanzas
    }

    /// Total frame bytes seen.
    #[must_use]
    pub const fn frame_bytes(&self) -> u64 {
        self.frame_bytes
    }

    /// How many plaintext entries were carried.
    #[must_use]
    pub const fn plaintexts(&self) -> u64 {
        self.plaintexts
    }

    /// How many stanzas arrived re-encoded rather than verbatim.
    ///
    /// A non-zero count on an engine that claims zero-copy is a bug in that
    /// adapter, not a detail.
    #[must_use]
    pub const fn re_encoded(&self) -> u64 {
        self.re_encoded
    }
}

impl StanzaSink for CountingSink {
    fn accept(&mut self, stanza: RawStanza<'_>) {
        self.stanzas = self.stanzas.saturating_add(1);
        self.frame_bytes = self.frame_bytes.saturating_add(stanza.frame.len() as u64);
        self.plaintexts = self
            .plaintexts
            .saturating_add(stanza.plaintexts.len() as u64);
        if !stanza.is_verbatim() {
            self.re_encoded = self.re_encoded.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;

    use wa_wire_contract::Direction;

    use crate::path::NodePathBuf;
    use crate::stanza::Plaintext;

    #[test]
    fn a_closure_is_a_sink() {
        let mut seen: Vec<Direction> = Vec::new();
        {
            let mut sink = |stanza: RawStanza<'_>| seen.push(stanza.direction);
            sink.accept(RawStanza::inbound(b"a"));
            sink.accept(RawStanza::outbound(b"b"));
        }
        assert_eq!(seen, [Direction::Inbound, Direction::Outbound]);
    }

    #[test]
    fn the_null_sink_accepts_and_discards() {
        let mut sink = NullSink;
        sink.accept(RawStanza::inbound(b"anything"));
        assert_eq!(sink, NullSink);
    }

    #[test]
    fn the_counting_sink_tallies_every_dimension() {
        let mut path = NodePathBuf::new();
        path.push(0).unwrap();
        let plaintexts = [
            Plaintext::ok(path.as_path(), b"one"),
            Plaintext::failed(path.as_path()),
        ];

        let mut sink = CountingSink::new();
        assert_eq!(sink, CountingSink::default());

        sink.accept(RawStanza::inbound(b"12345"));
        sink.accept(RawStanza::outbound(b"678").with_plaintexts(&plaintexts));
        sink.accept(RawStanza::inbound(b"9").re_encoded());

        assert_eq!(sink.stanzas(), 3);
        assert_eq!(sink.frame_bytes(), 5 + 3 + 1);
        assert_eq!(sink.plaintexts(), 2);
        assert_eq!(sink.re_encoded(), 1);
    }

    #[test]
    fn a_fresh_counting_sink_is_zeroed() {
        let sink = CountingSink::new();
        assert_eq!(sink.stanzas(), 0);
        assert_eq!(sink.frame_bytes(), 0);
        assert_eq!(sink.plaintexts(), 0);
        assert_eq!(sink.re_encoded(), 0);
    }

    #[test]
    fn counting_a_verbatim_stanza_does_not_mark_it_re_encoded() {
        let mut sink = CountingSink::new();
        sink.accept(RawStanza::inbound(b"x"));
        assert_eq!(sink.re_encoded(), 0, "verbatim must not be miscounted");
    }

    #[test]
    fn sinks_are_debuggable() {
        assert!(!alloc::format!("{:?}", CountingSink::new()).is_empty());
        assert!(!alloc::format!("{NullSink:?}").is_empty());
    }
}
