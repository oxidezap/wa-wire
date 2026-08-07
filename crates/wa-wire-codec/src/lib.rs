//! Zero-copy parsing of WhatsApp's binary-node encoding.
//!
//! `wa-wire` carries a stanza across the boundary exactly as the engine decoded
//! it — nothing is re-encoded, so the frame in an envelope is still in
//! WhatsApp's own format. This crate is what turns that frame into something
//! navigable, host-side, and only when something actually asked for it.
//!
//! # Nothing is copied
//!
//! A [`NodeRef`] borrows the frame and re-walks it on demand; the encoding is
//! self-delimiting, so a node never needs to know where it ends. Tokens are
//! borrowed from the table. Raw payloads are sub-slices of the frame.
//!
//! The two forms whose text exists nowhere in the buffer — packed digit runs
//! and JIDs — stay in parts rather than being joined, and compare and render
//! on demand:
//!
//! ```
//! # use wa_wire_codec::{Parser, tokens};
//! # let parser = Parser::new(tokens::TABLE);
//! # let frame = &[0xf8, 0x02, 0x05, 0x00][..];
//! # let node = parser.parse(frame).unwrap();
//! // No allocation: the comparison walks the parts.
//! let is_text = node.attr_eq("type", "text");
//! # let _ = is_text;
//! ```
//!
//! # The token table is a parameter
//!
//! WhatsApp's dictionaries move with the client version. Under RFC-009 that is
//! a matter of *provenance*, not of contract version, so the table is passed in
//! rather than compiled in. The bundled one is generated and committed, so a
//! protocol change arrives as a reviewable diff.
//!
//! ```
//! use wa_wire_codec::{Parser, TokenTable, tokens};
//!
//! let parser = Parser::new(tokens::TABLE);          // the bundled table
//! let custom = Parser::new(TokenTable::new(&[], &[])); // or your own
//! # let _ = (parser, custom);
//! ```
//!
//! # Paths line up with envelopes
//!
//! A plaintext entry in a `wa-wire-contract` envelope addresses its node by a
//! path of child indices. [`NodeRef::at_path`] walks exactly that path, which
//! is how a decrypted payload is matched back to the `<enc>` it came from.
//!
//! # Validation
//!
//! [`Parser::parse`] validates the entire tree, so every accessor on the result
//! is infallible. Malformed input is rejected up front and specifically enough
//! to tell a protocol change apart from corruption.

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

pub mod error;
pub mod jid;
pub mod node;
pub mod packed;
mod reader;
pub mod token;
pub mod value;

#[cfg(feature = "bundled-tokens")]
pub mod tokens;

pub use error::ParseError;
pub use jid::{Jid, User};
pub use node::{Attrs, Children, Content, DEFAULT_MAX_DEPTH, NodeRef, Parser};
pub use packed::{Alphabet, Packed};
pub use token::TokenTable;
pub use value::Value;
