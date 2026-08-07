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

use wa_wire_adapter::{RawStanza, StanzaSink};
use wa_wire_adapter_whatsapp_rust::WaWirePlugin;
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
            version: std::env::var("WA_WIRE_CAPTURE_VERSION")
                .ok()
                .map(|value| parse_version(&value))
                .transpose()?,
        })
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
}

impl FrameWriter {
    fn new(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            written: 0,
            failures: 0,
        })
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
        // The tag is only for the filename, so a frame that will not parse is
        // still worth keeping — an engine disagreeing about it is a finding.
        let tag = wa_wire_codec::Parser::new(wa_wire_codec::tokens::TABLE)
            .parse(stanza.frame)
            .map_or_else(|_| "unparsed".to_owned(), |node| node.tag().to_string());
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
                    let posted = ureq::post(&url)
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

    let plugin = WaWirePlugin::new(FrameWriter::new(settings.out.clone())?);
    let sink = plugin.sink();
    let client = build(&settings, plugin).await?;
    client
        .subscribe_handler(Arc::new(Pairing {
            post_to: settings.pair_post.clone(),
        }))
        .detach();
    client.connect().await?;

    tokio::time::sleep(Duration::from_secs(settings.seconds)).await;
    client.disconnect().await;

    let sink = sink.lock().expect("sink lock");
    println!(
        "\n{} frames written, {} failed",
        sink.written, sink.failures
    );
    if sink.written == 0 {
        anyhow::bail!("no stanzas captured — is the session paired and the server sending?");
    }
    Ok(())
}
