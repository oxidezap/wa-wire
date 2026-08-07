//! Replaying recorded stanzas through every engine and requiring them to agree.
//!
//! This is the property that makes `wa-wire` more than a wrapper:
//!
//! > Given the same traffic, every conforming engine must produce the same L1.
//!
//! Four independent implementations reading one input find bugs that no single
//! implementation's own tests can, because a bug and its test are usually
//! written by the same person on the same afternoon. Divergence is the signal.
//!
//! # What is compared
//!
//! Two layers, and they fail differently:
//!
//! - **L0** — the frame bytes each engine forwarded. Byte-identical frames mean
//!   the engines saw the same stanza; different ones do not necessarily mean a
//!   bug, because two encodings of one stanza are both valid.
//! - **L1** — the events derived from those frames, compared by *meaning*. Two
//!   engines that encode a value differently and derive the same event agree.
//!   This is the layer where a divergence is a finding.
//!
//! That split matters: reporting every L0 difference would bury the L1 ones,
//! and L1 is where correctness lives.
//!
//! ```
//! use wa_wire_conformance::{Recording, compare};
//! use wa_wire_codec::TokenTable;
//!
//! # fn example(engine_a: Recording<'_>, engine_b: Recording<'_>, table: TokenTable<'_>) {
//! let report = compare(&engine_a, &engine_b, table);
//! if report.agrees() {
//!     // Same traffic, same events.
//! } else {
//!     for divergence in report.divergences() {
//!         eprintln!("{divergence}");
//!     }
//! }
//! # }
//! ```

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

extern crate alloc;

pub mod divergence;
pub mod recording;
pub mod report;

pub use divergence::{Divergence, Layer};
pub use recording::Recording;
pub use report::{Report, compare, replay};
