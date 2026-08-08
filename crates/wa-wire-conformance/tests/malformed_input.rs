//! Every decoder, fed bytes nobody meant it to see.
//!
//! Three crates here read buffers that came from somewhere else — another
//! process, another language runtime, another machine — and each documents the
//! same promise:
//!
//! > A malformed envelope is an ordinary input, not a bug. It must be
//! > reportable, never a panic.
//!
//! Nothing checked that. This does.
//!
//! # Why not `cargo-fuzz`
//!
//! It needs nightly and a separate crate outside the workspace, so it would run
//! on somebody's laptop when they remembered rather than on every commit. The
//! mutations here are **deterministic**: a fixed seed sequence, a hand-written
//! PRNG, no dependency, and a failure that reproduces exactly from the seed
//! printed in the assertion. Coverage-guided fuzzing explores further and is
//! worth adding later — this is the half that runs every time.
//!
//! # What is actually asserted
//!
//! Not just "it did not panic", which any `catch_unwind` gets. When a decoder
//! *accepts* a mutated buffer, the invariants it advertises still have to hold:
//! an envelope's entry count matches what it iterates, a non-`Ok` status still
//! carries no payload, a recording's records stay inside their buffer. A
//! decoder that survives by accepting nonsense has not survived.
//!
//! ```sh
//! cargo test -p wa-wire-conformance --test malformed_input
//! ```

// A mutator is arithmetic on indices by nature, and every one of them is
// already bounded by the buffer it walks. The lint earns its keep in the
// decoders under test, not in the thing shaking them.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use std::path::PathBuf;

use wa_wire_codec::{DEFAULT_MAX_DEPTH, NodeRef, Parser, TokenTable};
use wa_wire_contract::EnvelopeRef;
use wa_wire_l1::testing::{Fixture, FixtureBuilder};
use wa_wire_recording::{ArtifactClass, Integrity, MetaBuilder, RecordingRef, RecordingWriter};

/// How many mutations each seed input gets.
///
/// Small enough that the suite stays instant and large enough that every
/// mutation kind lands on every input. Raise it locally when hunting.
const ROUNDS: u64 = 400;

// --- the mutator -----------------------------------------------------------

/// xorshift64*, so a failing seed reproduces the exact buffer.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        // Zero is the one state xorshift cannot leave.
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        usize::try_from(self.next() % (n as u64)).unwrap_or(0)
    }
}

/// Damage `input` in one of the ways a real buffer gets damaged.
///
/// Length fields get their own mutation because they are where a decoder is
/// most likely to trust its input: flipping a bit somewhere in a payload is
/// caught by a checksum, while a length that claims more than exists is what
/// turns a read into an out-of-bounds one.
fn mutate(seed: u64, input: &[u8]) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut out = input.to_vec();
    if out.is_empty() {
        return out;
    }

    let rounds = 1 + rng.below(3);
    for _ in 0..rounds {
        if out.is_empty() {
            break;
        }
        match rng.below(6) {
            0 => {
                let at = rng.below(out.len());
                out[at] ^= 1 << rng.below(8);
            }
            1 => {
                let at = rng.below(out.len());
                out[at] = u8::try_from(rng.below(256)).unwrap_or(0xFF);
            }
            2 => {
                let keep = rng.below(out.len());
                out.truncate(keep);
            }
            3 => {
                let at = rng.below(out.len());
                out.insert(at, u8::try_from(rng.below(256)).unwrap_or(0));
            }
            4 => {
                // A length prefix that claims far more than is there.
                let at = rng.below(out.len().saturating_sub(4).max(1));
                for offset in 0..4 {
                    if let Some(slot) = out.get_mut(at + offset) {
                        *slot = 0xFF;
                    }
                }
            }
            _ => {
                let (a, b) = (rng.below(out.len()), rng.below(out.len()));
                out.swap(a, b);
            }
        }
    }
    out
}

/// How a sweep's mutations were answered.
///
/// A sweep where everything is refused proves only that the first length check
/// works, and it can become that silently — a stricter header, a changed magic,
/// and the mutator is suddenly testing nothing while staying green. Requiring
/// both outcomes is what keeps the sweep honest about what it covered.
#[derive(Default)]
struct Outcomes {
    accepted: u32,
    refused: u32,
}

impl Outcomes {
    fn assert_both(&self, what: &str) {
        assert!(
            self.accepted > 0,
            "{what}: every mutation was refused, so the accept path went untested"
        );
        assert!(
            self.refused > 0,
            "{what}: every mutation was accepted, so the reject path went untested"
        );
    }
}

// --- the invariants --------------------------------------------------------

/// An accepted envelope has to mean what it says.
fn check_envelope(envelope: &EnvelopeRef<'_>, what: &str) {
    let entries: Vec<_> = envelope.entries().collect();
    assert_eq!(
        entries.len(),
        envelope.entry_count(),
        "{what}: iterating yielded a different number of entries than the count"
    );
    assert_eq!(
        envelope.is_plaintext_empty(),
        envelope.entry_count() == 0,
        "{what}: emptiness disagrees with the count"
    );
    for entry in entries {
        assert!(
            entry.status.is_ok() || entry.payload.is_empty(),
            "{what}: {} carries {} payload byte(s)",
            entry.status,
            entry.payload.len()
        );
        // Addressing has to be self-consistent, or a plaintext could be
        // attributed to a node it did not come from.
        assert_eq!(
            entry.path.as_le_bytes().len(),
            entry.path.len() * 2,
            "{what}: a path's bytes and component count disagree"
        );
        assert_eq!(entry.path.iter().count(), entry.path.len(), "{what}");
    }
}

/// An accepted recording has to stay inside its own buffer.
fn check_recording(recording: &RecordingRef<'_>, buf: &[u8], what: &str) {
    let mut total = 0usize;
    for record in recording.records() {
        total = total.saturating_add(record.payload.len());
        assert!(
            record.payload.len() <= buf.len(),
            "{what}: a record claims more bytes than the buffer holds"
        );
        // A mark either reads as one or does not; it must not do anything else.
        if record.kind == wa_wire_recording::Kind::MARK {
            let _ = record.as_mark();
        }
    }
    assert!(
        total <= buf.len(),
        "{what}: records total more than the whole recording"
    );

    match recording.integrity() {
        Integrity::Complete | Integrity::Damaged { .. } | Integrity::Truncated { .. } => {}
    }

    // Every accessor has to be callable on anything that decoded.
    let _ = recording.adapter();
    let _ = recording.provenance();
    let _ = recording.dictionary();
    let _ = recording.artifact_class();
    let _ = recording.input_digest();
    let _ = recording.transform();
    let _ = recording.created_at();
    let _ = recording.note();
    assert_eq!(
        recording.envelopes().count(),
        recording
            .records()
            .filter(|r| r.kind == wa_wire_recording::Kind::ENVELOPE)
            .count(),
        "{what}: the envelope view disagrees with the records"
    );
}

/// An accepted frame has to be walkable to the bottom.
fn check_node(node: &NodeRef<'_>, what: &str) {
    // Bounded so a cycle would fail loudly rather than hang. Real stanzas nest
    // to depth 9 at the extreme.
    fn walk(node: &NodeRef<'_>, depth: usize, what: &str) {
        assert!(depth < 128, "{what}: nesting past any real stanza");
        let _ = node.tag();
        for (_, value) in node.attrs() {
            let _ = value.as_str();
        }
        for child in node.children() {
            walk(&child, depth + 1, what);
        }
    }
    walk(node, 0, what);
}

// --- seed inputs -----------------------------------------------------------

fn table() -> TokenTable<'static> {
    wa_wire_codec::tokens::TABLE
}

fn receipt() -> FixtureBuilder {
    Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/zapo/fixtures")
}

/// Valid envelopes, including the ones another language wrote.
fn envelope_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = Vec::new();
    let built = receipt().build();
    seeds.push((
        "built".to_owned(),
        wa_wire_adapter::RawStanza::inbound(built.bytes())
            .encode_to_vec()
            .expect("encodes"),
    ));
    for name in [
        "receipt",
        "message-with-enc",
        "multi-device-with-plaintexts",
        "root-path-plaintext",
        "empty-frame",
    ] {
        if let Ok(bytes) = std::fs::read(fixture_dir().join(format!("{name}.bin"))) {
            seeds.push((name.to_owned(), bytes));
        }
    }
    assert!(
        seeds.len() > 1,
        "the cross-language fixtures must be present"
    );
    seeds
}

fn recording_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = Vec::new();

    let meta = MetaBuilder::new()
        .adapter("engine", "0.1.0", "1.0", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class")
        .input_digest(b"seed")
        .expect("input")
        .note("a seed for the mutator")
        .expect("note");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    for (_, envelope) in envelope_seeds() {
        writer.envelope(&envelope).expect("envelope");
    }
    writer.mark(42, "stream:error").expect("mark");
    seeds.push(("built".to_owned(), writer.finish()));

    for name in ["recording", "recording-truncated"] {
        if let Ok(bytes) = std::fs::read(fixture_dir().join(format!("{name}.wawr"))) {
            seeds.push((name.to_owned(), bytes));
        }
    }
    seeds
}

fn frame_seeds() -> Vec<(String, Vec<u8>)> {
    let mut seeds = vec![("receipt".to_owned(), receipt().build().bytes().to_vec())];
    let nested = Fixture::node("iq")
        .attr("id", "1")
        .child(Fixture::node("a").child(Fixture::node("b").child(Fixture::node("c"))))
        .build();
    seeds.push(("nested".to_owned(), nested.bytes().to_vec()));
    seeds
}

// --- the sweeps ------------------------------------------------------------

#[test]
fn no_mutation_of_an_envelope_escapes_the_decoder() {
    let mut seen = Outcomes::default();
    for (name, seed) in envelope_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let what = format!("envelope `{name}` seed {round}");
            match EnvelopeRef::decode(&bytes) {
                Ok(envelope) => {
                    seen.accepted += 1;
                    check_envelope(&envelope, &what);
                }
                Err(error) => {
                    seen.refused += 1;
                    assert!(!error.to_string().is_empty(), "{what}: unreportable");
                }
            }
        }
    }
    seen.assert_both("envelope");
}

#[test]
fn no_mutation_of_a_recording_escapes_the_reader() {
    let mut seen = Outcomes::default();
    for (name, seed) in recording_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let what = format!("recording `{name}` seed {round}");
            match RecordingRef::decode(&bytes) {
                Ok(recording) => {
                    seen.accepted += 1;
                    check_recording(&recording, &bytes, &what);
                }
                Err(error) => {
                    seen.refused += 1;
                    assert!(!error.to_string().is_empty(), "{what}: unreportable");
                }
            }
        }
    }
    seen.assert_both("recording");
}

#[test]
fn no_mutation_of_a_frame_escapes_the_parser() {
    let parser = Parser::new(table());
    let mut seen = Outcomes::default();
    for (name, seed) in frame_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let what = format!("frame `{name}` seed {round}");
            match parser.parse(&bytes) {
                Ok(node) => {
                    seen.accepted += 1;
                    check_node(&node, &what);
                }
                Err(error) => {
                    seen.refused += 1;
                    assert!(!error.to_string().is_empty(), "{what}: unreportable");
                }
            }
        }
    }
    seen.assert_both("frame");
}

#[test]
fn a_mutated_recording_is_still_only_read_through_the_envelope_decoder() {
    // The composition the gate actually performs: read a container, then decode
    // each payload. A recording that survives its own reader must not hand out
    // an envelope that then blows up.
    for (name, seed) in recording_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let Ok(recording) = RecordingRef::decode(&bytes) else {
                continue;
            };
            for (index, envelope) in recording.envelopes().enumerate() {
                let what = format!("recording `{name}` seed {round} envelope {index}");
                match EnvelopeRef::decode(envelope) {
                    Ok(decoded) => check_envelope(&decoded, &what),
                    Err(error) => assert!(!error.to_string().is_empty(), "{what}"),
                }
            }
        }
    }
}

#[test]
fn no_mutation_of_a_payload_escapes_the_protobuf_reader() {
    // The newest of the three decoders, and the one reading bytes that came
    // out of somebody else's encoder by way of Signal.
    let mut seen = Outcomes::default();
    for (name, seed) in payload_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let what = format!("payload `{name}` seed {round}");
            let mut reader = wa_wire_proto::Reader::new(&bytes);
            let mut ok = true;
            while let Some(field) = reader.next() {
                match field {
                    Ok(field) => {
                        // Every accessor has to be callable on anything read.
                        let _ = field.value.as_u64();
                        let _ = field.value.as_str();
                        let _ = field.value.as_sint64();
                        if let Some(bytes) = field.value.as_bytes() {
                            assert!(bytes.len() <= 4096, "{what}: a field outgrew its buffer");
                        }
                    }
                    Err(error) => {
                        ok = false;
                        assert!(!error.to_string().is_empty(), "{what}: unreportable");
                    }
                }
            }
            if ok {
                seen.accepted += 1;
            } else {
                seen.refused += 1;
            }
        }
    }
    seen.assert_both("payload");
}

#[test]
fn no_mutation_of_a_payload_escapes_the_content_derivation() {
    let mut seen = Outcomes::default();
    for (name, seed) in payload_seeds() {
        for round in 0..ROUNDS {
            let bytes = mutate(round, &seed);
            let what = format!("content `{name}` seed {round}");
            match wa_wire_l1::content::derive_content(&bytes) {
                Ok(content) => {
                    seen.accepted += 1;
                    // A kind is either named or numbered; nothing else.
                    let _ = content.kind.number();
                    assert!(!content.kind.name().is_empty(), "{what}");
                    assert!(content.wrappers <= 8, "{what}: unwrapped past the bound");
                }
                Err(error) => {
                    seen.refused += 1;
                    assert!(!error.to_string().is_empty(), "{what}: unreportable");
                }
            }
        }
    }
    seen.assert_both("content");
}

/// Valid `waE2E.Message` payloads, in the shapes real traffic carries.
fn payload_seeds() -> Vec<(String, Vec<u8>)> {
    let conversation = b"\x0a\x0bhello there".to_vec();
    // deviceSentMessage { message { conversation: "hi" } }
    let wrapped = alloc_vec(&[
        &[0xfa, 0x01, 0x08][..],
        &[0x12, 0x06][..],
        &[0x0a, 0x04][..],
        b"hihi",
    ]);
    // imageMessage { caption: "look" }
    let captioned = alloc_vec(&[&[0x1a, 0x08][..], &[0x1a, 0x06][..], b"look!!"]);
    vec![
        ("conversation".to_owned(), conversation),
        ("device-sent".to_owned(), wrapped),
        ("captioned-image".to_owned(), captioned),
    ]
}

fn alloc_vec(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

// --- inputs nobody would generate by mutation ------------------------------

#[test]
fn hand_built_hostile_buffers_are_refused_rather_than_trusted() {
    // Mutation rarely produces a maximal length prefix in exactly the right
    // place, and that is the value a decoder is most likely to trust.
    let hostile: [(&str, Vec<u8>); 6] = [
        ("empty", Vec::new()),
        ("one byte", vec![0]),
        ("all ones, short", vec![0xFF; 9]),
        ("all ones, long", vec![0xFF; 4096]),
        ("all zeroes", vec![0; 4096]),
        ("magic then nothing", b"WAWR".to_vec()),
    ];

    for (name, bytes) in hostile {
        if let Ok(envelope) = EnvelopeRef::decode(&bytes) {
            check_envelope(&envelope, name);
        }
        if let Ok(recording) = RecordingRef::decode(&bytes) {
            check_recording(&recording, &bytes, name);
        }
        if let Ok(node) = Parser::new(table()).parse(&bytes) {
            check_node(&node, name);
        }
    }
}

#[test]
fn nesting_is_bounded_before_it_reaches_the_stack() {
    // A parser that recurses per level turns a 20 KB frame into a stack
    // overflow, which is an abort rather than an error — the one failure mode
    // no caller can handle. This one refuses first, and the test pins both
    // sides of that limit so it cannot quietly move.
    let nest = |levels: usize| {
        let mut deep = Fixture::node("leaf");
        for _ in 0..levels {
            deep = Fixture::node("wrap").child(deep);
        }
        deep.build()
    };
    let parser = Parser::new(table());

    let too_deep = nest(2_000);
    let error = parser
        .parse(too_deep.bytes())
        .expect_err("nesting past the limit must be refused");
    let text = error.to_string();
    assert!(
        text.contains(&DEFAULT_MAX_DEPTH.to_string()),
        "the error has to name the limit it enforced: {text}"
    );

    // And the limit is not so tight that real traffic hits it: the deepest
    // stanza the spec models nests to 9.
    let within = nest(DEFAULT_MAX_DEPTH - 2);
    let node = parser
        .parse(within.bytes())
        .expect("nesting inside the limit must parse");

    // Walked iteratively: the claim is about the parser, and a recursive
    // checker would be the thing that overflowed.
    let mut stack = vec![node];
    let mut depth = 0usize;
    while let Some(current) = stack.pop() {
        depth = depth.saturating_add(1);
        stack.extend(current.children());
    }
    assert_eq!(
        depth,
        DEFAULT_MAX_DEPTH - 1,
        "every level has to be reachable"
    );
}
