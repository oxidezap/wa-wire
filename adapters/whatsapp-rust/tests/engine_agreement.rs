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
use wa_wire_conformance::{Layer, Recording, compare};
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
fn zapo_envelopes() -> Vec<Vec<u8>> {
    let path = workspace("adapters/zapo/recordings/zapo.recording");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun `npx tsx scripts/emit-recording.ts`",
            path.display()
        )
    });

    assert_eq!(&bytes[..4], b"WAWR", "recording magic");
    let count = u32::from_be_bytes(bytes[4..8].try_into().expect("4 bytes")) as usize;

    let mut envelopes = Vec::with_capacity(count);
    let mut offset = 8;
    for index in 0..count {
        let len = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .unwrap_or_else(|_| panic!("truncated length for envelope {index}")),
        ) as usize;
        offset += 4;
        envelopes.push(bytes[offset..offset + len].to_vec());
        offset += len;
    }
    assert_eq!(offset, bytes.len(), "trailing bytes in the recording");
    envelopes
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
    let report = compare(
        &Recording::new(INFO, &ours_refs),
        &Recording::new(ZAPO_INFO, &theirs_refs),
        table(),
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

    let report = compare(
        &Recording::new(INFO, &ours_refs),
        &Recording::new(ZAPO_INFO, &theirs_refs),
        table(),
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
    assert_eq!(
        report.divergences().count(),
        expected,
        "every difference is one of the known encoder choices"
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
