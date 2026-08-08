//! The L1 derivation: typed canonical events from parsed stanzas.
//!
//! L0 is normative and this is the derived view. Nothing appears here that is
//! not derivable from a parsed stanza, and the derivation is **pure** — no key
//! material, no accumulated state — which is what lets it run host-side, once,
//! instead of being reimplemented inside every engine.
//!
//! ```
//! # #[cfg(feature = "testing")] {
//! use wa_wire_l1::{Event, derive, testing::{Fixture, parse}};
//!
//! let stanza = Fixture::node("receipt")
//!     .attr("id", "ABCD1234")
//!     .jid_attr("from", "5511999998888")
//!     .attr("type", "read")
//!     .build();
//!
//! let event = derive(&parse(&stanza)).expect("derives");
//! assert_eq!(event.tag(), "receipt");
//! # }
//! ```
//!
//! # Generated, not written
//!
//! Everything in [`generated`] comes from whatspec's `incoming` domain, which
//! describes how WhatsApp Web itself parses each stanza. The generator emits
//! *structure* — which extraction primitive to call, in what order, into which
//! field. The primitives live in [`extract`] and are written by hand, so a
//! protocol change moves shapes and calls rather than rules.
//!
//! The output is committed rather than built, so a protocol change arrives as a
//! reviewable diff. CI regenerates and requires no change.
//!
//! # What it cannot express yet
//!
//! [`generated::UNMODELLED_FIELDS`] names every field the generator could not
//! emit, rather than dropping it in silence. That distinction matters: a
//! derivation that quietly omitted a field would look complete and be wrong,
//! and no conformance run could tell — every engine would agree on the same
//! missing field.
//!
//! # Cost
//!
//! Deriving borrows from the frame. String fields stay as [`Value`], because
//! the text of a packed digit run or a JID exists nowhere in the buffer to
//! borrow; comparing and rendering them is allocation-free. Repeated children
//! are iterators, not collections, so a caller that wants the first does not
//! pay for the rest.
//!
//! [`Value`]: wa_wire_codec::Value

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

extern crate alloc;

pub mod content;
pub mod error;
pub mod extract;
pub mod generated;
pub mod provenance;
pub mod semantic;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use error::{DeriveError, Field};
pub use generated::{Event, KNOWN_TAGS, PROVENANCE, UNMODELLED_FIELDS, derive};
pub use provenance::Provenance;
