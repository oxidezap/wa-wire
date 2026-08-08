//! Counts allocations, so the crates that claim not to allocate have to prove
//! it.
//!
//! Five places say some version of the same thing:
//!
//! - `README.md`: "none of them allocates while reading"
//! - `wa-wire-contract`: "Decoding never allocates and never copies", and
//!   "allocates exactly once with `encode_to_vec`"
//! - `wa-wire-recording`: "Reading never allocates"
//! - `wa-wire-codec`: "No allocation: the comparison walks the parts"
//! - `wa-wire-l1`: "comparing and rendering them is allocation-free"
//!
//! Nothing checked any of them. A claim repeated in five files and verified in
//! none is a claim that stops being true quietly, and the cost of it stopping
//! is paid per stanza by a process that runs for months.
//!
//! # Why this is a crate of its own
//!
//! Counting allocations means installing a [`GlobalAlloc`], which cannot be
//! written without `unsafe`. Every other crate here sets
//! `unsafe_code = "forbid"`, which an `allow` cannot lift, and weakening that
//! for a whole package to fit a test in would trade a real guarantee for a
//! measurement. So the exception lives here, in the one crate that exists for
//! the measurement and that nothing depends on.
//!
//! [`GlobalAlloc`]: std::alloc::GlobalAlloc
//!
//! ```sh
//! cargo test -p wa-wire-alloc-check
//! ```

#![cfg_attr(not(test), no_std)]

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
