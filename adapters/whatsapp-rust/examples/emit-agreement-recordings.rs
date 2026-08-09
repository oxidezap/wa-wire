//! Write one recording per engine, so the agreement can be checked without one.
//!
//! The four-engine comparison needs all four engines present at once, which is
//! why it has always been a manual run. What it *compares* needs none of them:
//! four byte streams and a token table. This freezes each engine's stream into
//! an RFC-010 recording so the comparison itself can live in the workspace and
//! run on every push.
//!
//! What CI then catches is our own derivation drifting — a change to
//! `wa-wire-l1` or the codec that makes four streams stop agreeing. What it
//! cannot catch is an *engine* changing, because a committed recording is a
//! photograph. That still needs this command re-run with every engine checked
//! out, which is the trade and is why the corpus digest travels inside each
//! file: a recording of a corpus that has since changed is reported as
//! incomparable rather than passing quietly.
//!
//! ```console
//! cd adapters/hypermeow && go run ./cmd/replay-corpus
//! cd adapters/baileys   && npx tsx scripts/replay-corpus.ts
//! cd adapters/zapo      && npx tsx scripts/emit-recording.ts
//! cd adapters/whatsapp-rust && cargo run --example emit-agreement-recordings
//! ```

use std::path::PathBuf;

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet, RawStanza};
use wa_wire_conformance::{Comparability, Recording, Tables, compare};
use wa_wire_recording::{ArtifactClass, Integrity, MetaBuilder, RecordingRef, RecordingWriter};
use whatsapp_rust::OwnedNodeRef;

fn workspace(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Every corpus frame, in the name order every reader walks.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = workspace("crates/wa-wire-conformance/corpus");
    let mut paths = Vec::new();
    for dir in [root.clone(), root.join("captured")] {
        let Ok(read) = std::fs::read_dir(&dir) else {
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

/// The checksum of the corpus, in the order every reader walks it.
fn corpus_digest() -> [u8; 4] {
    let mut joined = Vec::new();
    for (_, frame) in corpus() {
        joined.extend_from_slice(&frame);
    }
    wa_wire_recording::crc32(&joined).to_le_bytes()
}

/// What `whatsapp-rust`'s own encoder writes for each corpus stanza.
///
/// The re-encoded stream, not the forwarded one. Three of the four adapters
/// forward the corpus bytes untouched, so freezing what they forward would
/// freeze three identical copies of the input.
fn whatsapp_rust_reencoded() -> Vec<Vec<u8>> {
    corpus()
        .into_iter()
        .map(|(name, frame)| {
            let node = OwnedNodeRef::new(frame)
                .unwrap_or_else(|error| panic!("{name}: the engine must decode it: {error}"));
            let marshalled = whatsapp_rust::wacore_binary::marshal::marshal(&node.get().to_owned())
                .unwrap_or_else(|error| panic!("{name}: the engine must re-encode it: {error}"));
            // `marshal` writes the format byte the decoder strips.
            RawStanza::inbound(&marshalled[1..])
                .re_encoded()
                .encode_to_vec()
                .expect("envelope encodes")
        })
        .collect()
}

/// The envelopes an out-of-process adapter wrote, one file each.
fn replayed(adapter: &str, extension: &str) -> Vec<Vec<u8>> {
    let dir = workspace(&format!("adapters/{adapter}/replay"));
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nrun the adapter's replay-corpus command first",
                dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "{}: no envelopes", dir.display());

    paths
        .into_iter()
        .map(|path| std::fs::read(&path).expect("an envelope reads"))
        .collect()
}

/// `zapo` writes a container rather than loose files, so its stream is read
/// back out of one.
///
/// Checked before it is trusted. Re-stamping a damaged or partly-unread
/// recording with a fresh CRC would launder it into one that looks whole.
fn zapo_reencoded() -> Vec<Vec<u8>> {
    let path = workspace("adapters/zapo/recordings/zapo.recording");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nrun `npx tsx scripts/emit-recording.ts`",
            path.display()
        )
    });
    let recording = RecordingRef::decode(&bytes).expect("the committed recording reads");
    assert_eq!(
        recording.integrity(),
        Integrity::Complete,
        "{}: not whole; re-run the zapo replay",
        path.display()
    );
    assert_eq!(
        recording.skipped_records(),
        0,
        "{}: holds records this build does not read",
        path.display()
    );
    recording.envelopes().map(<[u8]>::to_vec).collect()
}

/// Refuse a stream that is not a replay of the corpus being stamped onto it.
///
/// A count alone does not settle this. A replay left over from a corpus that
/// changed without changing how many files it has would pass a length check,
/// then be written out carrying today's digest and a fresh CRC — which is
/// exactly the staleness the digest exists to catch, laundered by the tool that
/// was supposed to catch it.
///
/// So each stream is compared against the corpus itself: an engine's
/// re-encoding is entitled to differ in bytes and not in what it derives. If it
/// derives something else, it is a replay of something else.
fn check_replays_this_corpus(
    engine: &str,
    envelopes: &[Vec<u8>],
    corpus_refs: &[&[u8]],
    digest: &[u8],
) {
    let declared = Comparability::declared(digest, ArtifactClass::Replayed);
    let refs: Vec<&[u8]> = envelopes.iter().map(Vec::as_slice).collect();
    let corpus_info = AdapterInfo::new("corpus", "0.1.0", "0", CapabilitySet::NONE);
    let engine_info = AdapterInfo::new(engine, "0.1.0", "0", CapabilitySet::NONE);

    let report = compare(
        &Recording::new(corpus_info, corpus_refs).with_comparability(declared),
        &Recording::new(engine_info, &refs).with_comparability(declared),
        Tables::shared(wa_wire_codec::tokens::TABLE),
    );

    assert_eq!(
        report.incomparable(),
        None,
        "{engine}: cannot be checked against the corpus it claims to replay"
    );

    let faults: Vec<String> = report
        .faults()
        .map(|divergence| format!("  {divergence}"))
        .collect();
    assert!(
        faults.is_empty(),
        "{engine}: its replay derives something the corpus does not, so it is a replay of \
         other traffic. Re-run this engine's replay command before this one:\n{}",
        faults.join("\n")
    );
}

/// One engine's declaration, restated here because three of the four cannot be
/// linked into a Rust program.
struct Declared {
    id: &'static str,
    version: &'static str,
    engine_version: &'static str,
    capabilities: CapabilitySet,
    envelopes: Vec<Vec<u8>>,
}

fn main() {
    let digest = corpus_digest();
    let names: Vec<String> = corpus().into_iter().map(|(name, _)| name).collect();

    let engines: Vec<Declared> = vec![
        Declared {
            id: "whatsapp-rust",
            version: "0.1.0",
            engine_version: "0.7",
            capabilities: CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0InboundAuthPhase)
                .with(Capability::L0OutboundObserved)
                .with(Capability::L0Plaintext)
                .with(Capability::ZeroCopyFrame),
            envelopes: whatsapp_rust_reencoded(),
        },
        Declared {
            id: "zapo",
            version: "0.1.0",
            engine_version: "1.7",
            capabilities: CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0Plaintext)
                .with(Capability::Takeover)
                .with(Capability::DrainHook),
            envelopes: zapo_reencoded(),
        },
        Declared {
            id: "hypermeow",
            version: "0.1.0",
            engine_version: "0.0.0+frame-bytes-and-plaintext-hooks",
            capabilities: CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0InboundAuthPhase)
                .with(Capability::L0Plaintext)
                .with(Capability::ZeroCopyFrame),
            envelopes: replayed("hypermeow", "reencoded"),
        },
        Declared {
            id: "baileys",
            version: "0.1.0",
            engine_version: "7.0.0-rc14",
            capabilities: CapabilitySet::NONE
                .with(Capability::L0InboundTap)
                .with(Capability::L0InboundAuthPhase)
                .with(Capability::L0Plaintext)
                .with(Capability::ZeroCopyFrame),
            envelopes: replayed("baileys", "reencoded"),
        },
    ];

    let out = workspace("crates/wa-wire-conformance/recordings");
    std::fs::create_dir_all(&out).expect("the output directory is creatable");

    // The corpus itself, as envelopes, to check every stream against.
    let corpus_envelopes: Vec<Vec<u8>> = corpus()
        .into_iter()
        .map(|(_, frame)| {
            RawStanza::inbound(&frame)
                .encode_to_vec()
                .expect("envelope encodes")
        })
        .collect();
    let corpus_refs: Vec<&[u8]> = corpus_envelopes.iter().map(Vec::as_slice).collect();

    for engine in engines {
        assert_eq!(
            engine.envelopes.len(),
            names.len(),
            "{} re-encoded {} of {} corpus stanzas; regenerate its replay",
            engine.id,
            engine.envelopes.len(),
            names.len()
        );

        check_replays_this_corpus(engine.id, &engine.envelopes, &corpus_refs, &digest);

        let meta = MetaBuilder::new()
            .adapter(
                engine.id,
                engine.version,
                engine.engine_version,
                1,
                engine.capabilities.iter().map(Capability::identifier),
            )
            .expect("adapter")
            .artifact_class(ArtifactClass::Replayed)
            .expect("class")
            // The corpus this replays. Two recordings of different corpora are
            // refused rather than compared, which is what keeps a stale file
            // from reading as an engine regression.
            .input_digest(&digest)
            .expect("digest")
            // Frozen bytes read back against a table that has since moved would
            // put the same wrong tokens on both sides and agree. Naming the
            // table lets the reader refuse instead.
            .dictionary(wa_wire_codec::tokens::SOURCE_DIGEST)
            .expect("dictionary")
            .note("re-encoded corpus, written by emit-agreement-recordings")
            .expect("note");

        let mut writer = RecordingWriter::new(meta).expect("writer");
        for envelope in &engine.envelopes {
            writer.envelope(envelope).expect("envelope");
        }

        let path = out.join(format!("{}.wawr", engine.id));
        std::fs::write(&path, writer.finish()).expect("the recording is writable");
        println!(
            "{}: {} stanzas -> {}",
            engine.id,
            engine.envelopes.len(),
            path.display()
        );
    }
}
