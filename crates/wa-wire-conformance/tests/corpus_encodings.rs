//! The corpus must contain values that can be written more than one way.
//!
//! Agreement between four engines is only informative where they could
//! disagree. Three of the four adapters forward the corpus bytes untouched, so
//! what the comparison actually reads is each engine's **re-encoding** — and an
//! encoder only has a choice to make where the protocol admits one.
//!
//! `5511999998888@s.whatsapp.net` admits several: a `user@server` pair, a user
//! JID with an explicit domain byte, or a raw string. `3EB0C767D26B8E1B` can be
//! a packed hexadecimal run or twelve bytes of text. `1700000000` can be packed
//! nibbles or ten. A corpus of plain ASCII attributes asks none of those
//! questions, and the engines agree by having nothing to differ about.
//!
//! So this counts encodings rather than stanzas. A corpus that grows without
//! reaching a new form has not widened what the agreement covers.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use wa_wire_codec::packed::Alphabet;
use wa_wire_codec::{NodeRef, Parser, Value};

/// A way the binary protocol can carry a value.
///
/// Named for what an encoder chooses, not for the tag byte: `Binary20` is "a
/// body too long for a one-byte length", which is the decision, and the tag is
/// how the decision is written down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Form {
    /// Resolved through a token dictionary rather than spelled out.
    Token,
    /// Spelled out.
    Text,
    /// Digits, `-` and `.`, two per byte.
    PackedNibble,
    /// Uppercase hexadecimal, two per byte.
    PackedHex,
    /// `user@server`, no device.
    JidPair,
    /// A JID addressed to one device of a user.
    JidWithDevice,
    /// A JID on the `lid` domain rather than the phone-number one.
    JidLid,
    /// A JID whose server is in no dictionary, so it is spelled out.
    JidSpelledOutServer,
    /// A body whose length needs more than one byte.
    LongBody,
    /// A node with more children than a one-byte count can hold.
    LongChildList,
}

/// Forms the corpus must contain, each with why an encoder may choose
/// otherwise.
const REQUIRED: [(Form, &str); 10] = [
    (
        Form::Token,
        "a value in a dictionary may be spelled out instead",
    ),
    (
        Form::Text,
        "the ordinary case, and the baseline the rest differ from",
    ),
    (
        Form::PackedNibble,
        "a run of digits fits two per byte, and an encoder may leave it as text",
    ),
    (
        Form::PackedHex,
        "an uppercase hexadecimal id fits two per byte, same choice",
    ),
    (Form::JidPair, "`user@server` with no device"),
    (
        Form::JidWithDevice,
        "a device JID needs the form that carries one; text would lose it",
    ),
    (
        Form::JidLid,
        "the domain byte distinguishes `lid` from a phone number, and an \
         encoder writing `user@lid` as text moves that into the string",
    ),
    (
        Form::JidSpelledOutServer,
        "`newsletter`, `bot`, `interop` and `hosted.lid` are in none of the five \
         dictionaries, so a reader that only resolves tokens refuses them",
    ),
    (
        Form::LongBody,
        "past 255 bytes the length field widens, and where the boundary sits \
         is the encoder's reading of it",
    ),
    (
        Form::LongChildList,
        "past 255 children the list header widens, same boundary question",
    ),
];

fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the corpus directory reads")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("a corpus file reads"))
        })
        .collect()
}

fn note(found: &mut BTreeSet<Form>, value: Value<'_>) {
    match value {
        Value::Token(_) => {
            found.insert(Form::Token);
        }
        Value::Bytes(bytes) => {
            found.insert(Form::Text);
            if bytes.len() > 255 {
                found.insert(Form::LongBody);
            }
        }
        Value::Packed(packed) => {
            found.insert(match packed.alphabet() {
                Alphabet::Nibble => Form::PackedNibble,
                Alphabet::Hex => Form::PackedHex,
            });
        }
        Value::Jid(jid) => {
            // Asked of the table rather than listed here, so the set follows
            // the dictionaries instead of a copy of them.
            let in_dictionary = (0..=u8::MAX)
                .filter_map(|tag| wa_wire_codec::tokens::TABLE.single_byte(tag))
                .any(|token| token == jid.server());
            if !in_dictionary {
                found.insert(Form::JidSpelledOutServer);
            }

            if jid.server() == "lid" {
                found.insert(Form::JidLid);
            } else if jid.device() > 0 {
                found.insert(Form::JidWithDevice);
            } else {
                found.insert(Form::JidPair);
            }
        }
        Value::Nil => {}
    }
}

fn walk(found: &mut BTreeSet<Form>, node: &NodeRef<'_>) {
    note(found, node.tag());
    for (_, value) in node.attrs() {
        note(found, value);
    }

    match node.content() {
        // A body is a value like any other, so the same classifier reads it —
        // which is how a long binary body and a long attribute both count.
        wa_wire_codec::Content::Value(value) => note(found, value),
        wa_wire_codec::Content::Children(children) => {
            let mut count = 0usize;
            for child in children {
                count = count.saturating_add(1);
                walk(found, &child);
            }
            if count > 255 {
                found.insert(Form::LongChildList);
            }
        }
        wa_wire_codec::Content::None => {}
    }
}

#[test]
fn the_corpus_contains_every_encoding_an_engine_could_choose_differently() {
    let parser = Parser::new(wa_wire_codec::tokens::TABLE);
    let mut found: BTreeSet<Form> = BTreeSet::new();

    for (name, frame) in corpus() {
        let node = parser
            .parse(&frame)
            .unwrap_or_else(|error| panic!("{name}: the corpus must parse: {error:?}"));
        walk(&mut found, &node);
    }

    let missing: Vec<&(Form, &str)> = REQUIRED
        .iter()
        .filter(|(form, _)| !found.contains(form))
        .collect();

    assert!(
        missing.is_empty(),
        "no corpus frame carries {}, so no engine was ever asked to choose there:\n{}",
        if missing.len() == 1 {
            "a form"
        } else {
            "these forms"
        },
        missing
            .iter()
            .map(|(form, why)| format!("  {form:?} — {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
