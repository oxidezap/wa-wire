//! Capture live traffic into the conformance corpus.
//!
//! The corpus is otherwise hand-written, which is its weakness: it holds the
//! stanzas someone thought to write down. A server sends shapes nobody would
//! think to write down, and those are the ones where two engines are most
//! likely to disagree — which is the entire point of comparing them.
//!
//! This connects to a server, taps the inbound stream, and writes each decoded
//! stanza as a frame file. Nothing here knows which server: the endpoint is
//! configuration, so the same tool captures from a local test server or from
//! anywhere else that speaks the protocol.
//!
//! # Running
//!
//! ```sh
//! WA_WIRE_CAPTURE_URL=wss://127.0.0.1:8080/ws/chat \
//!   cargo run --example capture-corpus --features insecure-capture
//! ```
//!
//! | Variable | Meaning |
//! | --- | --- |
//! | `WA_WIRE_CAPTURE_URL` | Where to connect. Required. |
//! | `WA_WIRE_CAPTURE_OUT` | Where frames go. Defaults to `corpus/captured`. |
//! | `WA_WIRE_CAPTURE_SECONDS` | How long to stay connected. Defaults to 60. |
//! | `WA_WIRE_CAPTURE_STORE` | Session database. Defaults to an in-memory one, so each run pairs afresh. |
//! | `WA_WIRE_CAPTURE_PAIR_POST` | Optional. A URL the pairing code is `POST`ed to as `text/plain`. |
//! | `WA_WIRE_CAPTURE_VERSION` | Optional. `major.minor.patch` to use instead of looking the version up. |
//!
//! Pairing is normally a phone scanning the printed code. `WA_WIRE_CAPTURE_PAIR_POST`
//! covers the case where something else on the other end can accept the code
//! directly — the tool posts it and waits, and does not care what receives it.
//!
//! `WA_WIRE_CAPTURE_VERSION` matters more than it looks. By default the client
//! fetches the current web client version over the internet before connecting,
//! which makes a capture depend on a network the server has nothing to do with
//! — and pins it to whatever version happens to be live, which a server may not
//! accept. Setting this skips the lookup entirely.
//!
//! The `insecure-capture` feature turns off TLS and certificate-chain
//! verification, which a server using a self-signed certificate needs. It is a
//! feature rather than a runtime flag so that a build without it cannot be
//! talked into skipping verification.
//!
//! # Before committing what this captures
//!
//! **Capture from a test account.** Frames are stanzas as they arrived: JIDs,
//! phone numbers, message ids, push names. This tool does not scrub them, and
//! deliberately so — a scrubber that misses a field is worse than no scrubber,
//! because it invites trusting the output. Review what you commit.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use wa_wire_adapter::handoff::{Barrier, Detach, Gate};
use wa_wire_adapter::{Capability, RawStanza, StanzaSink};
use wa_wire_adapter_whatsapp_rust::{
    ADAPTER_VERSION, CAPABILITIES, Detacher, ENGINE_VERSION, PLUGIN_ID, WaWirePlugin,
};
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingWriter};
use whatsapp_rust::store::persistence_manager::PersistenceManager;
use whatsapp_rust::types::events::{Event, EventHandler, EventInterest, EventKind};
use whatsapp_rust::{Client, ClientBuilder};
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;

/// Everything the run was told, so a failure names the setting that caused it.
struct Settings {
    url: String,
    out: PathBuf,
    seconds: u64,
    store: Option<String>,
    pair_post: Option<String>,
    version: Option<(u32, u32, u32)>,
    recording: Option<PathBuf>,
}

impl Settings {
    fn from_env() -> Result<Self, String> {
        let url = std::env::var("WA_WIRE_CAPTURE_URL")
            .map_err(|_| "WA_WIRE_CAPTURE_URL is required".to_owned())?;
        let out = std::env::var("WA_WIRE_CAPTURE_OUT").map_or_else(
            |_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../crates/wa-wire-conformance/corpus/captured")
            },
            PathBuf::from,
        );
        let seconds = std::env::var("WA_WIRE_CAPTURE_SECONDS")
            .ok()
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| format!("WA_WIRE_CAPTURE_SECONDS is not a number: {value}"))
            })
            .transpose()?
            .unwrap_or(60);
        Ok(Self {
            url,
            out,
            seconds,
            store: std::env::var("WA_WIRE_CAPTURE_STORE").ok(),
            pair_post: std::env::var("WA_WIRE_CAPTURE_PAIR_POST").ok(),
            recording: std::env::var("WA_WIRE_CAPTURE_RECORDING")
                .ok()
                .map(PathBuf::from),
            version: std::env::var("WA_WIRE_CAPTURE_VERSION")
                .ok()
                .map(|value| parse_version(&value))
                .transpose()?,
        })
    }
}

/// Why the barrier could not be confirmed, in one phrase for the log.
///
/// Read from the declaration rather than written out, so a capability that
/// starts being true changes this line without anyone remembering to.
fn quiet_because() -> &'static str {
    if CAPABILITIES.contains(Capability::DrainHook) {
        "declares lifecycle.drain-hook"
    } else {
        "no lifecycle.drain-hook: the engine drains, nothing reports it"
    }
}

/// Parse `major.minor.patch`.
fn parse_version(value: &str) -> Result<(u32, u32, u32), String> {
    let mut parts = value.split('.').map(str::parse::<u32>);
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch)), None) => Ok((major, minor, patch)),
        _ => Err(format!(
            "WA_WIRE_CAPTURE_VERSION must be major.minor.patch, got {value}"
        )),
    }
}

/// Writes every frame it is handed, numbered in arrival order.
///
/// One file per stanza rather than one recording: the corpus is *input*, and
/// keeping each stanza addressable means a capture can be pruned by deleting
/// files rather than by re-running the whole session.
struct FrameWriter {
    dir: PathBuf,
    written: usize,
    failures: usize,
    /// Also assembled as one recording, when a caller asked for one.
    ///
    /// The directory and the recording answer different questions: the files
    /// are a corpus to replay one stanza at a time, and the recording is a
    /// session to compare against another engine's. A handoff needs the second
    /// — the claim is about what two engines heard, which is not a property any
    /// single stanza has.
    recording: Option<RecordingWriter>,
}

impl FrameWriter {
    fn new(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir)?;

        // Clear the `.bin` files a previous run left.
        //
        // The counter restarts but the names carry the stanza's tag too, so the
        // same position can produce a different filename — and the emitter
        // downstream sweeps every `.bin` in the directory. Left alone, a second
        // capture mixes into the first and the corpus becomes two sessions
        // nobody meant to compare.
        //
        // Only `.bin`, and only at the top level: this deletes files in a
        // directory the caller named, and deleting more than what this program
        // writes would be the kind of surprise a capture tool must not have.
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "bin") {
                std::fs::remove_file(&path)?;
            }
        }

        Ok(Self {
            dir,
            written: 0,
            failures: 0,
            recording: None,
        })
    }

    /// Assemble a recording alongside the files.
    ///
    /// The capability list is the adapter's own declaration rather than a
    /// literal, so a recording cannot come to claim something `INFO` stopped
    /// saying.
    fn recording(mut self) -> Result<Self, wa_wire_recording::WriteError> {
        let capabilities: Vec<&str> = CAPABILITIES.iter().map(Capability::identifier).collect();
        let meta = MetaBuilder::new()
            .adapter(
                PLUGIN_ID,
                ADAPTER_VERSION,
                ENGINE_VERSION,
                1,
                capabilities.iter().copied(),
            )?
            .artifact_class(ArtifactClass::Captured)?;
        self.recording = Some(RecordingWriter::new(meta)?);
        Ok(self)
    }

    /// A name that sorts in arrival order and says what it holds.
    fn name(&self, tag: &str) -> String {
        // Tags come from the wire; a hostile one must not escape the directory.
        let safe: String = tag
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        format!("{:04}-{safe}.bin", self.written)
    }
}

impl StanzaSink for FrameWriter {
    fn accept(&mut self, stanza: RawStanza<'_>) {
        // Inbound only. These files are replayed as traffic the server sent,
        // and the adapter now also reports what the client sent — writing both
        // to one directory would feed a replay stanzas it never received, as
        // though the server had said them.
        if stanza.direction != wa_wire_adapter::Direction::Inbound {
            return;
        }

        // The tag is only for the filename, so a frame that will not parse is
        // still worth keeping — an engine disagreeing about it is a finding.
        let tag = wa_wire_codec::Parser::new(wa_wire_codec::tokens::TABLE)
            .parse(stanza.frame)
            .map_or_else(|_| "unparsed".to_owned(), |node| node.tag().to_string());
        // Into the recording before the file: the recording is the session and
        // an envelope missing from it is a divergence a comparison would report,
        // while a `.bin` that failed to write is one stanza less in a corpus.
        if let Some(writer) = self.recording.as_mut() {
            match stanza.encode_to_vec() {
                Ok(envelope) => {
                    if let Err(error) = writer.envelope(&envelope) {
                        self.failures = self.failures.saturating_add(1);
                        eprintln!("  recording: {error}");
                    }
                }
                Err(error) => {
                    self.failures = self.failures.saturating_add(1);
                    eprintln!("  recording: {error}");
                }
            }
        }

        let path = self.dir.join(self.name(&tag));
        match std::fs::write(&path, stanza.frame) {
            Ok(()) => {
                self.written = self.written.saturating_add(1);
                println!("  {} ({} bytes)", path.display(), stanza.frame.len());
            }
            Err(error) => {
                self.failures = self.failures.saturating_add(1);
                eprintln!("  {}: {error}", path.display());
            }
        }
    }
}

/// Surfaces the pairing code, and hands it on when there is somewhere to hand
/// it to.
struct Pairing {
    post_to: Option<String>,
}

impl EventHandler for Pairing {
    fn handle_event(&self, event: Arc<Event>) {
        match &*event {
            Event::PairingQrCode(qr) => {
                println!("\nScan to pair:\n{}\n", qr.code);
                let Some(url) = self.post_to.clone() else {
                    return;
                };
                let code = qr.code.clone();
                // Blocking, on a thread of its own: this runs on the event
                // dispatch path, and holding it up delays the next stanza.
                std::thread::spawn(move || {
                    // Same reasoning as the transport's: a server worth pointing
                    // this at during development presents its own certificate,
                    // and this whole example is behind `insecure-capture`.
                    let agent: ureq::Agent = ureq::Agent::config_builder()
                        .tls_config(
                            ureq::tls::TlsConfig::builder()
                                .disable_verification(true)
                                .build(),
                        )
                        .build()
                        .into();
                    let posted = agent
                        .post(&url)
                        .header("content-type", "text/plain")
                        .send(code.as_bytes());
                    match posted {
                        Ok(_) => println!("pairing code posted to {url}"),
                        Err(error) => {
                            eprintln!("posting the pairing code to {url} failed: {error}");
                        }
                    }
                });
            }
            Event::PairSuccess(_) => println!("paired"),
            Event::Connected(_) => println!("connected — capturing"),
            _ => {}
        }
    }

    fn interest(&self) -> EventInterest {
        EventInterest::of(&[
            EventKind::PairingQrCode,
            EventKind::PairSuccess,
            EventKind::Connected,
        ])
    }
}

async fn build(
    settings: &Settings,
    plugin: WaWirePlugin<FrameWriter>,
) -> anyhow::Result<Arc<Client>> {
    let store = settings
        .store
        .clone()
        .unwrap_or_else(|| "file:wa-wire-capture?mode=memory&cache=shared".to_owned());
    let backend = whatsapp_rust::store::SqliteStore::new(&store).await?;
    let persistence = Arc::new(PersistenceManager::new(Arc::new(backend)).await?);

    let mut builder = ClientBuilder::new()
        .with_runtime(whatsapp_rust::runtime_impl::TokioRuntime)
        .with_persistence_manager(persistence)
        .with_transport_factory(TokioWebSocketTransportFactory::new().with_url(&settings.url))
        .with_http_client(whatsapp_rust::http::UreqHttpClient::new())
        .with_plugin(plugin);
    if let Some(version) = settings.version {
        builder = builder.with_version_override(version);
    }
    Ok(builder.build().await?.into_client())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `RUST_LOG=whatsapp_rust=debug` when a capture comes back empty and the
    // reason is on the connection rather than in this tool.
    env_logger::init();
    let settings = Settings::from_env().map_err(|error| anyhow::anyhow!(error))?;
    println!(
        "capturing from {} for {}s into {}",
        settings.url,
        settings.seconds,
        settings.out.display()
    );

    let mut frames = FrameWriter::new(settings.out.clone())?;
    if settings.recording.is_some() {
        frames = frames.recording()?;
    }
    let plugin = WaWirePlugin::new(frames);
    let sink = plugin.sink();
    let client = build(&settings, plugin).await?;
    client
        .subscribe_handler(Arc::new(Pairing {
            post_to: settings.pair_post.clone(),
        }))
        .detach();
    // `run`, not `connect`. `connect` completes the handshake and returns;
    // the loop that reads frames off the socket lives inside `run`, so a
    // capture built on `connect` alone sees the server's bytes arrive at the
    // transport and never get decoded.
    let running = Arc::clone(&client);
    let task = tokio::spawn(async move { running.run().await });

    tokio::time::sleep(Duration::from_secs(settings.seconds)).await;

    // The handoff, in the order RFC-003 gives. A capture that just disconnected
    // would be measuring the shape of a session ending, not of one being handed
    // over — and the two differ in exactly the phases below.
    //
    // Phase 1, quiesce. Nothing here issues commands, so the gate holds nothing;
    // it runs anyway, because the point of the phase is that the application
    // cannot add to what phase 2 drains, and a step skipped for being empty is
    // one nobody notices is missing later.
    let mut gate: Gate<Vec<u8>, 32> = Gate::new();
    gate.quiesce();

    // Phase 2, barrier. This adapter does not declare `lifecycle.drain-hook`, so
    // nothing will ever call `drained` and the answer is `Unconfirmed`.
    //
    // Which is not the same as "nothing drained". `Client::pause` flushes
    // inbound commits, offline receipts and the outbound scope before it closes
    // the socket — the engine does drain. What it does not do is tell a plugin,
    // so the honest report is that the host could not confirm it, and that is
    // the distinction `Quiet` exists to keep.
    let barrier = Barrier::new();

    // Phase 3, detach — `pause`, not `disconnect`. The second is terminal and
    // the session would not come back; this is a handoff.
    Detacher::new(Arc::clone(&client)).detach().await?;
    task.abort();

    println!("barrier: {} ({})", barrier.state(), quiet_because());

    // Phase 6, resume.
    let released = gate.resume(|_| unreachable!("nothing was queued"));
    println!("resumed: {released} command(s) released");

    let (written, failures, recording) = {
        let mut sink = sink.lock().expect("sink lock");
        // Taken rather than borrowed: finishing a recording consumes the
        // writer, and the plugin still holds the sink.
        let recording = sink.recording.take().map(RecordingWriter::finish);
        (sink.written, sink.failures, recording)
    };
    println!("\n{written} frames written, {failures} failed");

    if let (Some(path), Some(bytes)) = (settings.recording.as_ref(), recording) {
        std::fs::write(path, &bytes)?;
        println!("recording: {} ({} bytes)", path.display(), bytes.len());
    }

    if written == 0 {
        anyhow::bail!("no stanzas captured — is the session paired and the server sending?");
    }
    Ok(())
}
