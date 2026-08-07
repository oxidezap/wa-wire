//! The normative `wa-wire` boundary format and negotiation types.
//!
//! Every WhatsApp Web client library speaks the same wire protocol and exposes
//! a different API. `wa-wire` makes the thing they already share — the wire
//! itself — the interface, so an integration written once runs on any
//! conforming engine.
//!
//! # The layer model
//!
//! - **L0-wire** — the stanza as it arrived, payload still encrypted.
//! - **L0-plain** — that frame plus the plaintexts the engine decrypted.
//! - **L1** — typed canonical events, derived from L0-plain.
//!
//! L0 is normative and L1 is a derived view: nothing may appear in L1 that is
//! not derivable from L0-plain. `L0-plain → L1` is a pure function — protobuf
//! parsing and mapping, no keys and no accumulated state — which is why it runs
//! host-side, once, instead of being reimplemented per engine.
//!
//! # What crosses the boundary
//!
//! The frame bytes already exist inside every engine at the moment it decodes,
//! and the frame never contained the plaintext anyway — `<enc>` carries
//! ciphertext, and the plaintext arrives later from Signal. So an envelope is
//! **the frame verbatim plus a side table of plaintexts**, each addressed by
//! the path of the node it belongs to.
//!
//! Nothing is re-encoded, so there is no encoding to choose. The frame is
//! parsed exactly once, host-side, and only if something subscribed to L1.
//!
//! ```
//! use wa_wire_contract::{EnvelopeBuilder, EnvelopeRef, Flags, NodePath,
//!                        PlaintextEntry, PlaintextStatus};
//!
//! // A <message> whose single <enc> child decrypted successfully.
//! let frame = b"\xf8\x03...";                 // unpacked binary-node bytes
//! let path = 0u16.to_le_bytes();              // child 0 of the root
//! let entries = [PlaintextEntry {
//!     path: NodePath::from_le_bytes(&path),
//!     status: PlaintextStatus::Ok,
//!     payload: b"decrypted protobuf",
//! }];
//!
//! let bytes = EnvelopeBuilder::new(Flags::inbound(), frame)
//!     .with_entries(entries.iter().copied())
//!     .encode_to_vec()?;
//!
//! let envelope = EnvelopeRef::decode(&bytes)?;
//! assert_eq!(envelope.frame(), frame);
//! let entry = envelope.entries().next().expect("one plaintext");
//! assert!(entry.status.is_ok());
//! assert_eq!(entry.payload, b"decrypted protobuf");
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```
//!
//! # Two version axes
//!
//! [`ContractVersion`] versions *this* boundary; [`Provenance`] records which
//! `whatspec` build an L1 derivation came from. A WhatsApp protocol change
//! moves provenance and leaves the contract version alone — otherwise every
//! deployed adapter would break whenever the protocol shifted. L0 totality is
//! what makes that safe: the frame crosses verbatim, so there is nothing at L0
//! for a protocol change to break.
//!
//! # Cost
//!
//! Decoding never allocates and never copies: an [`EnvelopeRef`] borrows from
//! the buffer it was decoded from. Encoding writes once into a caller-supplied
//! slice, or allocates exactly once with [`encode_to_vec`].
//!
//! [`encode_to_vec`]: EnvelopeBuilder::encode_to_vec

#![no_std]
// Tests assert on known-good fixtures, so panicking on a broken one is the
// point. The library itself stays free of these.
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

pub mod capability;
pub mod envelope;
pub mod error;
pub mod flags;
pub mod path;
pub mod provenance;
pub mod status;
pub mod version;

pub use capability::{Capability, CapabilitySet, UnmetCapabilities};
pub use envelope::{EnvelopeBuilder, EnvelopeRef, HEADER_LEN, PlaintextEntry};
pub use error::{DecodeError, EncodeError, Field};
pub use flags::{Direction, Flags, FrameOrigin};
pub use path::NodePath;
pub use provenance::{Provenance, ProvenanceCheck};
pub use status::PlaintextStatus;
pub use version::{ContractVersion, VersionMismatch};
