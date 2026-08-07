//! What a `wa-wire` adapter must provide, and the plumbing every Rust adapter
//! shares.
//!
//! An adapter is the thin piece that lives inside an engine, observes stanzas,
//! and hands them on. It is deliberately dumb: emit what the engine saw and
//! stop. Everything that could be interpreted differently between engines —
//! parsing, L1 derivation — happens host-side, once, so an adapter has nothing
//! to diverge on and little to break when the engine moves underneath it.
//!
//! # What crosses
//!
//! A [`RawStanza`] is the pre-encoding shape of an envelope: the frame bytes
//! the engine decoded, plus any payloads it decrypted, each addressed by the
//! path of the node it came from.
//!
//! ```
//! use wa_wire_adapter::{NodePathBuf, Plaintext, RawStanza, StanzaSink};
//!
//! # fn example(frame: &[u8], decrypted: &[u8]) {
//! // The adapter walked to child 0 and found an <enc> it decrypted.
//! let mut path = NodePathBuf::new();
//! path.push(0).expect("within the depth limit");
//! let plaintexts = [Plaintext::ok(path.as_path(), decrypted)];
//!
//! let stanza = RawStanza::inbound(frame).with_plaintexts(&plaintexts);
//!
//! let mut sink = |stanza: RawStanza<'_>| {
//!     // In-process: read the frame straight out, no encoding at all.
//!     let _ = stanza.frame;
//! };
//! sink.accept(stanza);
//! # }
//! ```
//!
//! A sink receives the stanza rather than a finished buffer, so an in-process
//! consumer never pays for encoding. A sidecar consumer calls
//! [`RawStanza::encode_to_vec`] or [`RawStanza::encode_into_slice`] itself.
//!
//! # Declared, then checked
//!
//! No engine can do everything, and the gaps are real: one covers the auth
//! phase and cannot take over dispatch, another takes over but skips
//! `success`/`failure`. [`AdapterInfo`] carries what an adapter claims, and
//! [`AdapterInfo::verify`] checks stanzas against those claims — so a
//! capability that stops being true fails a test instead of quietly misleading
//! a consumer.
//!
//! # Cost
//!
//! Nothing here allocates. Paths are built in a fixed-capacity buffer, frames
//! and payloads are borrowed from the engine, and encoding — when it happens at
//! all — writes once into a caller-supplied slice.

#![no_std]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::cast_possible_truncation,
    )
)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod info;
pub mod path;
pub mod sink;
pub mod stanza;

pub use info::{AdapterInfo, Violation};
pub use path::{MAX_DEPTH, NodePathBuf, PathTooDeep};
pub use sink::{CountingSink, NullSink, StanzaSink};
pub use stanza::{Plaintext, RawStanza};

// Re-exported so an adapter needs one dependency, not two.
pub use wa_wire_contract::{
    Capability, CapabilitySet, ContractVersion, Direction, FrameOrigin, NodePath, PlaintextStatus,
    Provenance,
};
