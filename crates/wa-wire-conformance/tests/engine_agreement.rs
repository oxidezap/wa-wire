//! Four engines, one corpus, the same events — checked without an engine.
//!
//! This is the property the project rests on:
//!
//! > Given the same traffic, every conforming engine must produce the same L1.
//!
//! It has always been true and has always been checked by hand, because
//! *producing* four streams needs four engines checked out at once and each
//! brings its own toolchain. What *comparing* them needs is four byte streams
//! and a token table, so the streams are frozen into recordings
//! (`recordings/*.wawr`, written by the `whatsapp-rust` adapter's
//! `emit-agreement-recordings`) and the comparison runs here, on every push.
//!
//! # What this catches, and what it cannot
//!
//! It catches **our** side moving: a change to `wa-wire-l1` or the codec that
//! makes four streams stop deriving the same events. That is the half nobody
//! was watching, and it is the half changed most often.
//!
//! It cannot catch an **engine** moving, because a committed recording is a
//! photograph of one. That still needs the four checked out and the emitter
//! re-run, which is inherent and is why each recording carries the corpus
//! digest: a recording of traffic that has since changed is reported as
//! incomparable rather than passing quietly.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet};
use wa_wire_conformance::{Comparability, Recording, Tables, compare};
use wa_wire_recording::{ArtifactClass, Integrity, RecordingRef};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every corpus frame, in the name order every reader walks.
///
/// The same walk the emitter does, so the digest computed here is the digest it
/// wrote. A different walk would make every recording look stale.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = crate_dir().join("corpus");
    let mut paths = Vec::new();
    for dir in [root.clone(), root.join("captured")] {
        let Ok(read) = std::fs::read_dir(&dir) else {
            assert_ne!(dir, root, "{}: the corpus is missing", dir.display());
            continue;
        };
        paths.extend(
            read.filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "bin")),
        );
    }
    paths.sort();
    assert!(!paths.is_empty(), "the corpus must not be empty");

    paths
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("a corpus file reads"))
        })
        .collect()
}

fn corpus_digest() -> [u8; 4] {
    let mut joined = Vec::new();
    for (_, frame) in corpus() {
        joined.extend_from_slice(&frame);
    }
    wa_wire_recording::crc32(&joined).to_le_bytes()
}

/// Every engine's committed stream, in a fixed order.
const ENGINES: &[&str] = &["whatsapp-rust", "zapo", "hypermeow", "baileys"];

fn recording_path(engine: &str) -> PathBuf {
    crate_dir()
        .join("recordings")
        .join(format!("{engine}.wawr"))
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun `cargo run --example emit-agreement-recordings` in \
             adapters/whatsapp-rust, with all four engines checked out",
            path.display()
        )
    })
}

/// What one engine's recording holds, once it has been checked over.
struct Replay {
    engine: &'static str,
    bytes: Vec<u8>,
}

impl Replay {
    fn envelopes(&self) -> Vec<&[u8]> {
        self.decoded().envelopes().collect()
    }

    fn decoded(&self) -> RecordingRef<'_> {
        RecordingRef::decode(&self.bytes).expect("a committed recording decodes")
    }

    /// The declaration the recording carries, rebuilt.
    ///
    /// Read out of the file rather than restated here: three of the four
    /// adapters cannot be linked into a Rust test, and a constant copied from
    /// each would be a fifth place for them to drift.
    fn info(&self) -> AdapterInfo<'_> {
        let recording = self.decoded();
        let meta = recording
            .adapter()
            .expect("a recording declares its adapter");
        let capabilities = meta
            .capabilities
            .iter()
            .filter_map(Capability::from_identifier)
            .fold(CapabilitySet::NONE, CapabilitySet::with);

        AdapterInfo::new(meta.id, meta.version, meta.engine_version, capabilities)
    }

    /// What the file declares about whether it may be compared at all.
    fn comparability(&self) -> Comparability<'_> {
        Comparability::of(&self.decoded())
    }
}

fn every_engine() -> Vec<Replay> {
    ENGINES
        .iter()
        .map(|engine| Replay {
            engine,
            bytes: read(&recording_path(engine)),
        })
        .collect()
}

#[test]
fn every_committed_recording_is_whole_and_replays_the_current_corpus() {
    let digest = corpus_digest();
    let expected = corpus().len();

    for replay in every_engine() {
        let recording = replay.decoded();
        let engine = replay.engine;

        assert_eq!(
            recording.integrity(),
            Integrity::Complete,
            "{engine}: a committed recording must not be truncated or damaged"
        );
        assert_eq!(
            recording.unknown_critical_tags(),
            0,
            "{engine}: a critical tag this build cannot read makes it incomparable"
        );
        assert_eq!(
            recording.skipped_records(),
            0,
            "{engine}: a record of a kind this build does not read would leave the \
             comparison passing on the part it did read"
        );
        // Frozen bytes decoded against a table that has since moved would put
        // the same wrong tokens on both sides and agree. The recording names
        // the dictionary it was written against; this build has to be carrying
        // that one.
        assert_eq!(
            recording.dictionary(),
            Some(wa_wire_codec::tokens::SOURCE_DIGEST),
            "{engine}: written against a different token dictionary than this build \
             carries; re-run emit-agreement-recordings"
        );
        assert_eq!(
            recording.envelope_count(),
            expected,
            "{engine}: {} stanzas against a corpus of {expected}; re-run \
             emit-agreement-recordings",
            recording.envelope_count()
        );
        assert_eq!(
            recording.input_digest(),
            Some(digest.as_slice()),
            "{engine}: recorded from a different corpus than the one committed here; \
             re-run emit-agreement-recordings with all four engines checked out"
        );
    }
}

/// The property the project rests on, on every push.
///
/// Pairwise rather than each against a reference. Agreement ought to be
/// transitive, and a pair is what a report can name: "these two disagreed" is
/// actionable where "someone disagreed with the reference" is a second
/// investigation.
///
/// The **re-encoded** stream is what the recordings hold, and that matters.
/// Three of the four adapters are zero-copy and forward the corpus bytes
/// untouched, so comparing what they forward compares nothing. Re-encoding is
/// where four independent implementations genuinely differ — `hypermeow` and
/// Baileys write different bytes for five of these stanzas — and that the
/// derivation matches anyway is the property.
#[test]
fn every_engine_derives_the_same_events_from_the_same_traffic() {
    let engines = every_engine();
    assert_eq!(engines.len(), 4, "the definition of done asks for four");

    let names: Vec<String> = corpus().into_iter().map(|(name, _)| name).collect();

    for (index, left) in engines.iter().enumerate() {
        for right in engines.iter().skip(index.saturating_add(1)) {
            let (left_envelopes, right_envelopes) = (left.envelopes(), right.envelopes());
            let (left_info, right_info) = (left.info(), right.info());
            // Read out of each file rather than asserted over both. Declaring
            // comparability here would state that the recordings are whole,
            // carry no unknown critical tag and skipped no record — three
            // things a file says about itself and a caller cannot know.
            let (left_declared, right_declared) = (left.comparability(), right.comparability());

            let report = compare(
                &Recording::new(left_info, &left_envelopes).with_comparability(left_declared),
                &Recording::new(right_info, &right_envelopes).with_comparability(right_declared),
                Tables::shared(wa_wire_codec::tokens::TABLE),
            );

            assert_eq!(
                report.incomparable(),
                None,
                "{} against {}: both sides must declare the same corpus before any \
                 verdict means anything",
                left.engine,
                right.engine
            );

            let faults: Vec<_> = report.faults().collect();
            assert!(
                faults.is_empty(),
                "{} and {} derived different events across {} stanzas:\n{}",
                left.engine,
                right.engine,
                names.len(),
                faults
                    .iter()
                    .map(|divergence| format!("  {divergence}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            assert!(report.agrees(), "{} and {}", left.engine, right.engine);
        }
    }
}

/// The comparison would notice, on exactly this data.
///
/// A green run means four streams agreed. It does not, on its own, mean the
/// comparison could ever have said otherwise — and a comparison that cannot
/// fail is a comparison that proves nothing. So one engine's stream is given a
/// stanza from the wrong position and the report is required to object.
#[test]
fn a_diverging_engine_would_be_caught_on_this_corpus() {
    let engines = every_engine();
    let digest = corpus_digest();
    let declared = Comparability::declared(&digest, ArtifactClass::Replayed);

    let mut pair = engines.iter();
    let (Some(left), Some(right)) = (pair.next(), pair.next()) else {
        panic!("two engines are needed to compare two");
    };
    let mut tampered = right.envelopes();
    assert!(tampered.len() > 1, "the corpus needs two stanzas to swap");
    tampered.swap(0, 1);

    let untouched = left.envelopes();
    let report = compare(
        &Recording::new(left.info(), &untouched).with_comparability(declared),
        &Recording::new(right.info(), &tampered).with_comparability(declared),
        Tables::shared(wa_wire_codec::tokens::TABLE),
    );

    assert_eq!(report.incomparable(), None, "the pair is still comparable");
    assert!(
        report.faults().next().is_some(),
        "two stanzas out of order derived the same events, so this comparison          cannot distinguish engines on this corpus"
    );
}

/// The engines really do write different bytes, so the test above has content.
///
/// Without this, four identical streams would satisfy every assertion in this
/// file and the run would be green while proving that a copy is a copy.
#[test]
fn the_streams_being_compared_are_not_all_the_same_bytes() {
    let engines = every_engine();
    let streams: Vec<Vec<&[u8]>> = engines.iter().map(Replay::envelopes).collect();

    let differing = streams
        .iter()
        .enumerate()
        .flat_map(|(index, left)| {
            streams
                .iter()
                .skip(index.saturating_add(1))
                .filter(move |right| *right != left)
        })
        .count();

    assert!(
        differing > 0,
        "every engine wrote identical bytes, so agreement is a tautology here"
    );
}
