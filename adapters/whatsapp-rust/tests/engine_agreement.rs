//! Two engines, one corpus, and the claim the project rests on.
//!
//! > Given the same traffic, every conforming engine must produce the same L1.
//!
//! Everything else in `wa-wire` is plumbing for this test. Each corpus file is
//! a frame as an engine receives it; `whatsapp-rust` decodes it and forwards
//! the buffer verbatim, while `zapo` decodes it with its own decoder and
//! re-encodes with its own encoder. Two implementations of one wire format,
//! each entitled to encode a value its own way, feeding one derivation that has
//! to read them identically.
//!
//! # What a byte difference here actually measures
//!
//! Worth being precise about, because it is easy to read more into it. On a
//! hand-written frame both sides encode from the same source and their bytes
//! match, which compares the two *encoders*. On a **captured** frame they do
//! not: `whatsapp-rust` forwards the server's bytes untouched, so a difference
//! there is between whoever encoded that frame and `zapo` — not between the two
//! engines.
//!
//! Either way the useful half holds: two different encodings of one stanza
//! derive the same event. That is the property. What a captured difference does
//! *not* show is one engine encoding differently from the other.
//!
//! Known differences are named in `KNOWN_ENCODER_DIVERGENCES`, so a new one is
//! a deliberate decision rather than a counter going up.
//!
//! # Regenerating
//!
//! ```sh
//! cd adapters/whatsapp-rust && cargo run --example emit-corpus
//! cd adapters/zapo && npx tsx scripts/emit-recording.ts
//! ```

#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::sync::Arc;

use wa_wire_adapter::{AdapterInfo, RawStanza, StanzaSink};
use wa_wire_adapter_whatsapp_rust::INFO;
use wa_wire_conformance::{
    Comparability, ComparisonProfile, Layer, Recording, Tables, Verdict, compare,
};
use wa_wire_recording::{ArtifactClass, Integrity, RecordingRef};
use whatsapp_rust::OwnedNodeRef;

/// `zapo`'s declaration, as its own source states it.
///
/// Restated here rather than imported because the TypeScript adapter cannot be
/// linked into a Rust test. A mismatch is caught by the recording itself
/// failing to compare, not by this constant.
const ZAPO_INFO: AdapterInfo<'static> = AdapterInfo::new(
    "wa-wire-zapo",
    "0.1.0",
    "1.7",
    wa_wire_adapter::CapabilitySet::NONE
        .with(wa_wire_adapter::Capability::L0InboundTap)
        .with(wa_wire_adapter::Capability::Takeover)
        .with(wa_wire_adapter::Capability::DrainHook),
);

fn workspace(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Every corpus frame, in name order — the same order both readers use.
///
/// `captured/` holds frames recorded from a server rather than written by hand.
/// Both directories are read, and a capture is just more corpus: the comparison
/// does not care where a frame came from.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = workspace("crates/wa-wire-conformance/corpus");
    let mut entries = Vec::new();
    for dir in [root.clone(), root.join("captured")] {
        let Ok(read) = std::fs::read_dir(&dir) else {
            assert_ne!(
                dir,
                root,
                "{}: missing — run `cargo run --example emit-corpus`",
                dir.display()
            );
            continue;
        };
        entries.extend(
            read.filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "bin")),
        );
    }
    entries.sort();
    assert!(!entries.is_empty(), "the corpus must not be empty");

    entries
        .into_iter()
        .map(|path| {
            // Prefixed with the directory, so a captured frame and a written
            // one cannot collide on a name and cannot be confused in a report.
            let name = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("corpus file reads"))
        })
        .collect()
}

/// Run the corpus through this adapter and collect what it emitted.
///
/// Drives the sink directly rather than through the plugin host: the host's job
/// is delivering events, and this test is about what the adapter does with
/// them. Plugin installation is covered in the crate's own tests.
fn whatsapp_rust_envelopes() -> Vec<Vec<u8>> {
    #[derive(Default)]
    struct Capture(Vec<Vec<u8>>);

    impl StanzaSink for Capture {
        fn accept(&mut self, stanza: RawStanza<'_>) {
            self.0
                .push(stanza.encode_to_vec().expect("envelope encodes"));
        }
    }

    let mut capture = Capture::default();
    for (name, frame) in corpus() {
        let node = Arc::new(
            OwnedNodeRef::new(frame)
                .unwrap_or_else(|error| panic!("{name}: the engine must decode it: {error}")),
        );
        // The frame the engine received, forwarded as it stands.
        capture.accept(RawStanza::inbound(&node.backing_bytes()));
    }
    capture.0
}

/// The envelopes `zapo` wrote, from its committed recording.
///
/// Read through the RFC-010 reader rather than by hand: the container used to
/// be four bytes of magic and a count, parsed with slice indexing inside this
/// test. A recording that travels between machines carries claims about which
/// engine, which spec and which traffic produced it, and a reader that panics
/// on anything unexpected cannot check them.
fn zapo_recording() -> Vec<u8> {
    let path = workspace("adapters/zapo/recordings/zapo.recording");
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun `npx tsx scripts/emit-recording.ts`",
            path.display()
        )
    })
}

/// The envelopes `zapo` wrote, owned so callers keep their existing shape.
fn zapo_envelopes() -> Vec<Vec<u8>> {
    let bytes = zapo_recording();
    let recording = RecordingRef::decode(&bytes).expect("the committed recording reads");
    assert_eq!(
        recording.integrity(),
        Integrity::Complete,
        "a committed recording must not be truncated or damaged"
    );
    assert_eq!(
        recording.unknown_critical_tags(),
        0,
        "a critical tag this build cannot read makes the pair incomparable"
    );
    recording.envelopes().map(<[u8]>::to_vec).collect()
}

/// The checksum of the corpus, in the order both readers walk it.
///
/// Both sides compute it independently and the comparison refuses the pair
/// unless they match, so two recordings of *different* corpora are reported as
/// incomparable rather than as an engine regression.
fn corpus_digest() -> [u8; 4] {
    let mut joined = Vec::new();
    for (_, frame) in corpus() {
        joined.extend_from_slice(&frame);
    }
    wa_wire_recording::crc32(&joined).to_le_bytes()
}

fn table() -> wa_wire_codec::TokenTable<'static> {
    wa_wire_codec::tokens::TABLE
}

#[test]
fn the_two_engines_derive_the_same_events_from_the_same_traffic() {
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();
    let names: Vec<_> = corpus().into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        ours.len(),
        theirs.len(),
        "both engines must have seen the whole corpus; regenerate the recording"
    );

    let ours_refs: Vec<&[u8]> = ours.iter().map(Vec::as_slice).collect();
    let theirs_refs: Vec<&[u8]> = theirs.iter().map(Vec::as_slice).collect();
    let digest = corpus_digest();
    let declared = Comparability::declared(&digest, ArtifactClass::Replayed);
    let report = compare(
        &Recording::new(INFO, &ours_refs).with_comparability(declared),
        &Recording::new(ZAPO_INFO, &theirs_refs).with_comparability(declared),
        Tables::shared(table()),
    );
    assert_eq!(
        report.incomparable(),
        None,
        "both sides must declare the same corpus before any verdict means anything"
    );

    let faults: Vec<_> = report.faults().collect();
    assert!(
        faults.is_empty(),
        "the engines derived different events across {} stanzas:\n{}",
        names.len(),
        faults
            .iter()
            .map(|divergence| format!("  {divergence}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(report.agrees());
}

/// Stanzas whose two frames legitimately differ.
///
/// Named rather than counted, so adding a divergence is a deliberate act with a
/// reviewed reason and not a number quietly going up.
const KNOWN_ENCODER_DIVERGENCES: &[(&str, &str)] = &[
    (
        "captured/0000-iq.bin",
        "from=s.whatsapp.net: whatsapp-rust writes it as a JID with no user, zapo as a dictionary token",
    ),
    (
        "captured/0001-iq.bin",
        "same server-JID-versus-token choice as 0000",
    ),
    (
        "captured/0012-iq.bin",
        "a childless <list>: whatsapp-rust writes an explicit empty body, zapo omits it entirely",
    ),
];

#[test]
fn frames_differ_only_where_they_are_known_to() {
    // On the hand-written corpus the bytes match, so this says the two encoders
    // agree. On captured frames it says less — see the module docs — but the
    // list is still what keeps a new difference from going unnoticed.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();
    let names: Vec<_> = corpus().into_iter().map(|(name, _)| name).collect();

    let mut unexpected = Vec::new();
    let mut seen = Vec::new();
    for (index, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        let a = wa_wire_contract::EnvelopeRef::decode(a).expect("ours decodes");
        let b = wa_wire_contract::EnvelopeRef::decode(b).expect("theirs decodes");
        if a.frame() == b.frame() {
            continue;
        }
        let name = names[index].as_str();
        match KNOWN_ENCODER_DIVERGENCES
            .iter()
            .find(|(known, _)| *known == name)
        {
            Some((_, reason)) => seen.push((name, *reason)),
            None => unexpected.push(format!(
                "  {name}: {} bytes against {}",
                a.frame().len(),
                b.frame().len()
            )),
        }
    }

    assert!(
        unexpected.is_empty(),
        "the encoders diverged somewhere new:\n{}\n\nIf the difference is a \
         legitimate encoding choice, add it to KNOWN_ENCODER_DIVERGENCES with \
         the reason. If it is not, one of the two encoders is wrong.",
        unexpected.join("\n")
    );
    // Only the listed stanzas actually present are expected to diverge: the
    // captured corpus is optional, and a checkout without it must still pass.
    let present: Vec<_> = KNOWN_ENCODER_DIVERGENCES
        .iter()
        .filter(|(known, _)| names.iter().any(|name| name == known))
        .collect();
    assert_eq!(
        seen.len(),
        present.len(),
        "a listed divergence stopped happening — remove it rather than leave it \
         claiming something that is no longer true"
    );
}

#[test]
fn the_envelopes_still_differ_in_what_they_declare() {
    // The frames match, but the engines are not interchangeable in what they
    // can claim about them: one forwards the buffer it decoded, the other
    // rebuilt it. A consumer that needs provably-original bytes has to read
    // `frame_origin`, and it has to be telling the truth.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();

    for (a, b) in ours.iter().zip(&theirs) {
        let a = wa_wire_contract::EnvelopeRef::decode(a).expect("ours decodes");
        let b = wa_wire_contract::EnvelopeRef::decode(b).expect("theirs decodes");
        assert!(a.flags().is_verbatim(), "whatsapp-rust forwards the buffer");
        assert!(!b.flags().is_verbatim(), "zapo rebuilt it");
    }
}

#[test]
fn every_divergence_is_at_l0_and_none_is_a_fault() {
    // The split the suite is built on, now carrying weight: frames differ,
    // derived events do not. A difference that reached L1 would mean one of the
    // two engines reads the wire wrongly — that is the finding this exists for.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();
    let ours_refs: Vec<&[u8]> = ours.iter().map(Vec::as_slice).collect();
    let theirs_refs: Vec<&[u8]> = theirs.iter().map(Vec::as_slice).collect();

    let digest = corpus_digest();
    let declared = Comparability::declared(&digest, ArtifactClass::Replayed);
    let report = compare(
        &Recording::new(INFO, &ours_refs).with_comparability(declared),
        &Recording::new(ZAPO_INFO, &theirs_refs).with_comparability(declared),
        Tables::shared(table()),
    );
    assert_eq!(
        report.incomparable(),
        None,
        "both sides must declare the same corpus before any verdict means anything"
    );

    let at_l1: Vec<_> = report
        .divergences()
        .filter(|divergence| divergence.layer() == Layer::L1)
        .map(|divergence| format!("  {divergence}"))
        .collect();
    assert!(
        at_l1.is_empty(),
        "the engines read the same bytes differently:\n{}",
        at_l1.join("\n")
    );
    assert!(report.agrees());
    assert_eq!(report.compared(), ours.len(), "every stanza was compared");
    let names: Vec<_> = corpus().into_iter().map(|(name, _)| name).collect();
    let expected = KNOWN_ENCODER_DIVERGENCES
        .iter()
        .filter(|(known, _)| names.iter().any(|name| name == known))
        .count();
    let frame_diffs = report
        .divergences()
        .filter(|divergence| matches!(divergence, wa_wire_conformance::Divergence::Frame { .. }))
        .count();
    assert_eq!(
        frame_diffs, expected,
        "every byte difference is one of the known encoder choices"
    );

    // One per stanza, and not a fault: this adapter hands over the buffer its
    // engine decoded, and `zapo` cannot reach its own, so it re-encodes and
    // says so. Recorded rather than suppressed, because between two *builds*
    // of one adapter the same finding means it stopped reaching its buffer.
    let origin_diffs = report
        .divergences()
        .filter(|divergence| {
            matches!(
                divergence,
                wa_wire_conformance::Divergence::FrameOrigin { .. }
            )
        })
        .count();
    assert_eq!(origin_diffs, ours.len());
    assert_eq!(
        report.divergences().count(),
        frame_diffs + origin_diffs,
        "and nothing else differs"
    );

    // The profiles are not interchangeable, and this is the corpus that shows
    // it: the same report that passes as interop fails as regression, because
    // "a different engine" and "the same engine, changed" are different
    // questions about the same evidence.
    assert_eq!(report.evaluate(ComparisonProfile::Interop), Verdict::Pass);
    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Fail,
        "these are two engines, not two builds of one"
    );
}

#[test]
fn the_corpus_covers_the_tags_the_derivation_models() {
    // A corpus that happened to contain only unmodelled stanzas would agree
    // trivially: both engines failing to derive anything is agreement.
    let mut derived = 0usize;
    for (name, frame) in corpus() {
        let node = wa_wire_codec::Parser::new(table())
            .parse(&frame)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        if wa_wire_l1::derive(&node).is_ok() {
            derived += 1;
        }
    }
    assert!(
        derived >= 8,
        "only {derived} corpus stanzas derive an event; the agreement would be \
         mostly vacuous"
    );
}

// --- the consumer -----------------------------------------------------------

/// One consumer, both engines, no branch on which.
///
/// `wa-wire-example-consumer` depends on the contract, the codec and the
/// derivation — and on no engine at all. It cannot tell which of these
/// recordings it is reading, because there is nothing in its dependency graph
/// that could tell it.
///
/// This is the claim the project is for. Everything else — the envelope format,
/// the adapters, the conformance suite — exists so that this function can be
/// written once and produce the same answer twice.
fn tally(envelopes: &[Vec<u8>]) -> wa_wire_example_consumer::Tally {
    let mut tally = wa_wire_example_consumer::Tally::default();
    for envelope in envelopes {
        tally.accept(envelope, table());
    }
    tally
}

#[test]
fn one_consumer_reads_both_engines_to_the_same_answer() {
    let ours = tally(&whatsapp_rust_envelopes());
    let theirs = tally(&zapo_envelopes());

    assert_eq!(
        ours, theirs,
        "the same consumer code produced different answers from the two engines"
    );
}

#[test]
fn the_consumer_actually_did_something() {
    // `Tally::default() == Tally::default()`, so the assertion above would hold
    // over two empty runs. It has to be reading real stanzas for the equality
    // to mean anything.
    let tally = tally(&whatsapp_rust_envelopes());

    assert_eq!(
        tally.stanzas,
        corpus().len(),
        "every corpus stanza reached the consumer"
    );
    assert_eq!(
        tally.derived,
        tally.stanzas - 1,
        "every stanza but the deliberately unmodelled <presence> derived an event"
    );
    assert_eq!(
        tally.inbound, tally.stanzas,
        "the corpus is inbound traffic"
    );
    // All four tags the derivation models. That matters for what the
    // two-engine equality is worth: agreement across the whole modelled
    // surface, not on one stanza kind repeated.
    assert_eq!(
        tally.by_tag.keys().copied().collect::<Vec<_>>(),
        ["ack", "call", "message", "receipt"],
        "a change here is a change in what the derivation covers"
    );
    assert!(
        !tally.ids.is_empty(),
        "and the consumer read attributes off them"
    );
}

#[test]
fn swapping_the_engine_is_not_swapping_the_bytes() {
    // The equality above would be trivial if both engines handed over identical
    // envelopes. They do not: the declared frame origin differs on every
    // stanza, because one engine forwards the buffer it decoded and the other
    // rebuilt it. The consumer is reading genuinely different input and
    // reaching the same conclusion.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();
    assert!(
        ours.iter().zip(&theirs).all(|(a, b)| a != b),
        "every envelope should differ in what it declares"
    );
}
