//! The wa-wire recording container: envelopes at rest.
//!
//! RFC-008 specifies one stanza crossing the boundary. This is a *sequence* of
//! them in a file, plus the claims that decide whether two such files may be
//! compared at all: which adapter, which spec, which dictionary, which traffic.
//!
//! A container without those claims does not make them absent. It makes them
//! unverifiable, and a comparison runs and reports a verdict anyway.
//!
//! ```
//! use wa_wire_recording::{ArtifactClass, Integrity, MetaBuilder, RecordingRef, RecordingWriter};
//!
//! let meta = MetaBuilder::new()
//!     .adapter("zapo", "0.1.0", "1.7", 1, ["l0.inbound.tap"])?
//!     .artifact_class(ArtifactClass::Synthetic)?;
//!
//! let mut writer = RecordingWriter::new(meta)?;
//! writer.envelope(b"first")?;
//! writer.mark(1_500, "stream:error")?;
//! writer.envelope(b"second")?;
//! let bytes = writer.finish();
//!
//! let recording = RecordingRef::decode(&bytes)?;
//! assert_eq!(recording.integrity(), Integrity::Complete);
//! assert_eq!(
//!     recording.envelopes().collect::<Vec<_>>(),
//!     [b"first".as_slice(), b"second"]
//! );
//! assert_eq!(recording.adapter().map(|a| a.id), Some("zapo"));
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```
//!
//! # Truncation is a state, not a failure
//!
//! The record count lives in a trailer rather than the header, so a writer does
//! not have to know its own length before the first byte — which is what lets a
//! ring buffer be a writer at all. The consequence is that an interrupted
//! recording has no trailer, and this reader treats that as
//! [`Integrity::Truncated`]: every complete record is readable and the file is
//! not comparable.
//!
//! That is deliberate. The artifact a crash recorder exists to produce is, by
//! definition, the one that was interrupted, and a container that rejected it
//! would fail its most important use while passing every test written against
//! well-formed files.
//!
//! # What it does not do
//!
//! The trailer's checksum detects damage, not tampering: anything able to
//! rewrite the records can rewrite the checksum, and nothing here is signed.
//! Identity comes from [`input_digest`], which the container carries as opaque
//! bytes and never computes — so the hash function stays the responsibility of
//! whoever produced the traffic.
//!
//! [`input_digest`]: RecordingRef::input_digest

#![no_std]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
    )
)]

pub mod crc;
pub mod error;
pub mod meta;
pub mod reader;
pub mod record;
#[cfg(feature = "alloc")]
pub mod writer;

pub use crc::{Crc32, crc32};
pub use error::{ReadError, WriteError};
pub use meta::{AdapterMeta, ArtifactClass, CapabilityNames, MetaEntry, Tag};
pub use reader::{CONTAINER_VERSION, HEADER_LEN, Integrity, MAGIC, RecordingRef};
pub use record::{Kind, Mark, Record};
#[cfg(feature = "alloc")]
pub use writer::{MetaBuilder, RecordingWriter};

#[cfg(all(test, feature = "alloc"))]
#[path = "lib_tests.rs"]
mod tests;
