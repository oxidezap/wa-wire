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
//! # The result
//!
//! They agree at L1, and — not what this was written expecting — their frames
//! are byte-identical too. The suite is built to tolerate an L0 difference
//! because two encodings of one stanza are both valid; on this corpus there is
//! nothing to tolerate. The tests assert the stronger property, so a future
//! change that starts producing different bytes shows up as a result rather
//! than passing silently under the weaker one.
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
use wa_wire_conformance::{Recording, compare};
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
fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = workspace("crates/wa-wire-conformance/corpus");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nrun `cargo run --example emit-corpus`",
                dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "the corpus must not be empty");

    entries
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .expect("corpus file has a name")
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

#[test]
fn the_two_encoders_agree_byte_for_byte() {
    // Not what this test was written expecting. `zapo` re-encodes from a
    // decoded node while `whatsapp-rust` forwards the buffer it received, so
    // the two had every opportunity to differ — and produce identical bytes for
    // the whole corpus.
    //
    // That is a stronger result than the one the conformance suite is built to
    // tolerate, and it is worth pinning: if a future change to either encoder
    // starts producing different bytes, that is a real event about the two
    // implementations, even though the comparison would still pass on meaning.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();

    for (index, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        let a = wa_wire_contract::EnvelopeRef::decode(a).expect("ours decodes");
        let b = wa_wire_contract::EnvelopeRef::decode(b).expect("theirs decodes");
        assert_eq!(
            a.frame(),
            b.frame(),
            "stanza {index}: the encoders diverged — if this is intended, the \
             conformance claim moves from `is_identical` to `agrees`"
        );
    }
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
fn nothing_diverges_at_either_layer() {
    // `agrees()` tolerates an L0 difference; `is_identical()` does not. The
    // stronger one holds here, so assert the stronger one — a report that
    // quietly starts carrying frame differences would still `agree`, and this
    // is what notices.
    let ours = whatsapp_rust_envelopes();
    let theirs = zapo_envelopes();
    let ours_refs: Vec<&[u8]> = ours.iter().map(Vec::as_slice).collect();
    let theirs_refs: Vec<&[u8]> = theirs.iter().map(Vec::as_slice).collect();

    let report = compare(
        &Recording::new(INFO, &ours_refs),
        &Recording::new(ZAPO_INFO, &theirs_refs),
        table(),
    );

    let found: Vec<_> = report
        .divergences()
        .map(|divergence| format!("  [{:?}] {divergence}", divergence.layer()))
        .collect();
    assert!(
        report.is_identical(),
        "expected nothing at all:\n{}",
        found.join("\n")
    );
    assert_eq!(report.compared(), ours.len(), "every stanza was compared");
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
