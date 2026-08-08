//! `wa-wire` adapter for the `whatsapp-rust` engine, in tap mode.
//!
//! Installs as a native plugin, subscribes to `Event::RawNode`, and forwards
//! each stanza's frame bytes to a [`StanzaSink`]. Nothing is re-encoded and
//! nothing is copied: `whatsapp-rust` already retains the buffer its decoder
//! consumed, so forwarding a stanza is a refcount bump.
//!
//! ```
//! use wa_wire_adapter::{CountingSink, RawStanza};
//! use wa_wire_adapter_whatsapp_rust::WaWirePlugin;
//!
//! // Any `StanzaSink` will do — a closure, a channel, a counter.
//! let plugin = WaWirePlugin::new(|stanza: RawStanza<'_>| {
//!     let _ = stanza.frame; // the engine's own buffer, not a copy
//! });
//!
//! // Or keep a handle on what the sink accumulated.
//! let counting = WaWirePlugin::new(CountingSink::new());
//! let sink = counting.sink();
//! assert_eq!(sink.lock().unwrap().stanzas(), 0);
//! ```
//!
//! Register it with the engine's plugin host at build time; the host installs
//! it and drops the subscription on shutdown.
//!
//! # What this adapter can and cannot do
//!
//! Declared in [`INFO`], and checked against every stanza in the tests rather
//! than left as a comment:
//!
//! | Capability | Status |
//! | --- | --- |
//! | `l0.inbound.tap` | yes — `Event::RawNode` fires before any early return |
//! | `l0.inbound.auth-phase` | yes — `success`, `failure` and `xmlstreamend` all reach it |
//! | `l0.zero-copy-frame` | yes — the decode buffer is already retained |
//! | `l0.plaintext` | yes — `Event::DecryptedPayload` reports each one after Signal |
//! | `l0.outbound` | **no** — the engine has no raw outbound observer |
//! | `l0.takeover` | in [`takeover`], not here — `RawNode` observes; the pipeline runs regardless |
//!
//! Plaintext is the one that needs two observation points rather than one.
//! `Event::RawNode` fires when a stanza is decoded, necessarily *before*
//! decryption; `Event::DecryptedPayload` reports each `<enc>` afterwards. The
//! adapter holds the frame until its payloads catch up and emits one envelope
//! carrying both — see [`plaintext`] for what happens when a payload never
//! comes.
//!
//! # Cost when nobody is listening
//!
//! `Event::RawNode` is gated behind a lease that the plugin host acquires only
//! when a subscription declares interest in it. With no such subscription the
//! engine skips forwarding entirely — it does not even wrap an `ack` in an
//! `Arc`. Installing this plugin is what turns that on, and dropping it turns it
//! back off.

use std::sync::{Arc, Mutex};

use wa_wire_adapter::{
    AdapterInfo, Capability, CapabilitySet, RawStanza, RequestError, RequestFuture, SendError,
    SendFuture, StanzaRequester, StanzaSender, StanzaSink, Violation,
};
use whatsapp_rust::OwnedNodeRef;
use whatsapp_rust::plugins::{
    ClientPlugin, PluginCapability, PluginContext, PluginFuture, PluginManifest,
};
use whatsapp_rust::types::events::{Event, EventHandler, EventInterest, EventKind};

use crate::plaintext::{DecryptedEnc, PlaintextJoiner};

/// The events this adapter needs to produce L0-plain.
///
/// Both are lease-gated in the engine, and the host takes each lease from this
/// interest — so declaring the pair here is also what turns them on.
fn interest() -> EventInterest {
    EventInterest::of(&[EventKind::RawNode, EventKind::DecryptedPayload])
}

/// The engine version this adapter was written against.
pub const ENGINE_VERSION: &str = "0.7";

/// This adapter's version.
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The plugin id, as it appears in the engine's manifest.
pub const PLUGIN_ID: &str = "wa-wire";

/// What this adapter can do.
///
/// Every entry is asserted against real stanzas in this crate's tests, so a
/// capability cannot quietly stop being true.
pub const CAPABILITIES: CapabilitySet = CapabilitySet::NONE
    .with(Capability::L0InboundTap)
    .with(Capability::L0InboundAuthPhase)
    .with(Capability::L0Plaintext)
    .with(Capability::ZeroCopyFrame);

/// This adapter's declaration.
pub const INFO: AdapterInfo<'static> =
    AdapterInfo::new(PLUGIN_ID, ADAPTER_VERSION, ENGINE_VERSION, CAPABILITIES);

/// Forwards every decoded stanza to a sink.
///
/// The sink is shared behind a mutex because the engine dispatches events from
/// its receive path and may do so concurrently. Hold it briefly: a slow sink
/// stalls delivery.
pub struct WaWirePlugin<S> {
    sink: Arc<Mutex<S>>,
    required: CapabilitySet,
}

impl<S> WaWirePlugin<S>
where
    S: StanzaSink + Send + 'static,
{
    /// Forward stanzas to `sink`.
    pub fn new(sink: S) -> Self {
        Self {
            sink: Arc::new(Mutex::new(sink)),
            required: CapabilitySet::NONE,
        }
    }

    /// Refuse to install unless this adapter has every capability in `needed`.
    ///
    /// The setup-time gate. Without it a consumer discovers that its engine
    /// never emits plaintext, or re-encodes frames it meant to replay, as
    /// *missing traffic* — where the evidence of the problem is the thing that
    /// is absent. Naming the requirement turns that into a refused install.
    ///
    /// Cheap to state and worth stating even when it currently holds: the point
    /// is that it keeps holding when the engine moves underneath.
    ///
    /// ```no_run
    /// use wa_wire_adapter::{Capability, CapabilitySet, CountingSink};
    /// use wa_wire_adapter_whatsapp_rust::WaWirePlugin;
    ///
    /// let plugin = WaWirePlugin::new(CountingSink::new()).requiring(
    ///     CapabilitySet::NONE
    ///         .with(Capability::ZeroCopyFrame)
    ///         .with(Capability::L0Plaintext),
    /// );
    /// ```
    #[must_use]
    pub fn requiring(mut self, needed: CapabilitySet) -> Self {
        self.required = needed;
        self
    }

    /// The shared sink, for a caller that needs to read what it accumulated.
    #[must_use]
    pub fn sink(&self) -> Arc<Mutex<S>> {
        Arc::clone(&self.sink)
    }
}

/// The API this plugin publishes to the engine.
///
/// Deliberately empty: a tap has nothing to offer other plugins, and exposing
/// the sink here would let one plugin steal another's stanzas.
pub struct WaWireApi {
    _private: (),
}

impl<S> ClientPlugin for WaWirePlugin<S>
where
    S: StanzaSink + Send + 'static,
{
    type Api = WaWireApi;

    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, ADAPTER_VERSION)
            .with_capability(PluginCapability::CoreEvents)
    }

    fn install(&self, context: PluginContext) -> PluginFuture<'_, anyhow::Result<Arc<Self::Api>>> {
        let sink = Arc::clone(&self.sink);
        let required = self.required;
        Box::pin(async move {
            // Before anything is registered: a consumer that asked for what this
            // adapter cannot do should not get a half-working install.
            INFO.require(required)?;
            let events = context
                .core_events()
                .ok_or_else(|| anyhow::anyhow!("host did not grant events.core.observe"))?;

            // Declaring interest in RawNode is what makes the host take the
            // forwarding lease; dropping the subscription releases it.
            let subscription = events.subscribe(
                interest(),
                Arc::new(RawNodeTap {
                    sink,
                    joiner: Mutex::new(PlaintextJoiner::new()),
                }),
            )?;
            // The subscription lives as long as the plugin's resources do; the
            // host drops it on shutdown.
            core::mem::forget(subscription);

            Ok(Arc::new(WaWireApi { _private: () }))
        })
    }
}

// The bound lives on the struct only because `Drop` requires it to; every impl
// below repeats it anyway.
struct RawNodeTap<S: StanzaSink> {
    sink: Arc<Mutex<S>>,
    /// Held separately from the sink, and always locked first, so the two are
    /// acquired in one order everywhere.
    joiner: Mutex<PlaintextJoiner>,
}

impl<S: StanzaSink> RawNodeTap<S> {
    /// Run `work` with the joiner and sink both held.
    ///
    /// A poisoned lock means a consumer panicked. Dropping the stanza beats
    /// propagating the panic into the engine's receive path, and beats emitting
    /// past a joiner whose buffer may be half-updated.
    fn with_both(&self, work: impl FnOnce(&mut PlaintextJoiner, &mut S)) {
        let Ok(mut joiner) = self.joiner.lock() else {
            return;
        };
        let Ok(mut sink) = self.sink.lock() else {
            return;
        };
        work(&mut joiner, &mut sink);
    }

    /// Emit every frame still waiting for plaintexts.
    fn flush(&self) {
        self.with_both(|joiner, sink| joiner.flush(&mut VerifyingSink(sink)));
    }
}

impl<S: StanzaSink> Drop for RawNodeTap<S> {
    fn drop(&mut self) {
        // The host drops the subscription on shutdown, which drops this. A
        // frame still waiting for a plaintext that will now never arrive is
        // better emitted unobserved than lost: the stanza was real either way.
        self.flush();
    }
}

impl<S> EventHandler for RawNodeTap<S>
where
    S: StanzaSink + Send + 'static,
{
    fn handle_event(&self, event: Arc<Event>) {
        match &*event {
            // The frame is a refcount bump on the buffer the decoder retained.
            Event::RawNode(node) => self.with_both(|joiner, sink| {
                joiner.accept_frame(node, &mut VerifyingSink(sink));
            }),
            Event::DecryptedPayload(payload) => self.with_both(|joiner, sink| {
                joiner.accept_plaintext(
                    &DecryptedEnc {
                        message_id: payload.info.id.clone(),
                        enc_index: payload.enc_index,
                        payload: payload.payload.clone(),
                    },
                    &mut VerifyingSink(sink),
                );
            }),
            _ => {}
        }
    }

    fn interest(&self) -> EventInterest {
        interest()
    }
}

/// Checks each stanza against [`INFO`] on the way to the real sink.
///
/// In debug builds only: the declaration is what a consumer selects an engine
/// on, so a capability that stops being true should fail a test rather than
/// quietly mislead.
struct VerifyingSink<'a, S>(&'a mut S);

impl<S: StanzaSink> StanzaSink for VerifyingSink<'_, S> {
    fn accept(&mut self, stanza: RawStanza<'_>) {
        debug_assert_eq!(
            INFO.verify(&stanza),
            Ok(()),
            "adapter emitted a stanza its own declaration forbids"
        );
        self.0.accept(stanza);
    }
}

/// Check a stanza against [`INFO`].
///
/// Exposed so a host can assert the adapter's claims in its own tests, not only
/// in this crate's.
pub fn verify(stanza: &RawStanza<'_>) -> Result<(), Violation> {
    INFO.verify(stanza)
}

pub mod plaintext;
pub mod takeover;

#[cfg(test)]
mod tests;

/// Sending stanzas through a `whatsapp-rust` client.
///
/// The outbound half of the boundary. Frames go out exactly as they come in —
/// the buffer a decoder consumes — so a captured envelope can be sent back as it
/// stands.
///
/// ```no_run
/// use std::sync::Arc;
/// use wa_wire_adapter::StanzaSender;
/// use wa_wire_adapter_whatsapp_rust::Sender;
///
/// # async fn example(client: Arc<whatsapp_rust::Client>, frame: &[u8]) {
/// let sender = Sender::new(client);
/// let _ = sender.send_frame(frame).await;
/// # }
/// ```
pub struct Sender {
    client: Arc<whatsapp_rust::Client>,
}

impl Sender {
    /// Send through `client`.
    #[must_use]
    pub const fn new(client: Arc<whatsapp_rust::Client>) -> Self {
        Self { client }
    }
}

/// What this adapter can do when it is also sending.
///
/// A separate declaration rather than a flag on [`CAPABILITIES`]: an adapter
/// built for observation alone genuinely cannot send, and one set covering both
/// would be false for whichever the consumer actually has.
pub const SENDING_CAPABILITIES: CapabilitySet = CAPABILITIES.with(Capability::L0Outbound);

/// This adapter's declaration when it is also sending.
pub const SENDING_INFO: AdapterInfo<'static> = AdapterInfo::new(
    PLUGIN_ID,
    ADAPTER_VERSION,
    ENGINE_VERSION,
    SENDING_CAPABILITIES,
);

/// The marshalled stanza a frame came from.
///
/// A frame is the decoder's buffer: the marshalled stanza minus its leading
/// format byte. Putting the byte back is the exact inverse of how the frame was
/// taken on the way in, which is what lets a captured envelope be sent back
/// unchanged — record and replay are the same bytes, not two encodings that
/// happen to agree.
fn to_marshalled(frame: &[u8]) -> Vec<u8> {
    let mut marshalled = Vec::with_capacity(frame.len().saturating_add(1));
    // Zero is what `Encoder::new_vec` writes there.
    marshalled.push(0);
    marshalled.extend_from_slice(frame);
    marshalled
}

impl StanzaSender for Sender {
    fn send_frame<'a>(&'a self, frame: &'a [u8]) -> SendFuture<'a> {
        let marshalled = to_marshalled(frame);
        Box::pin(async move {
            self.client
                .send_raw_bytes(marshalled)
                .await
                .map_err(|error| match error {
                    // The one a consumer can act on without knowing the engine.
                    whatsapp_rust::ClientError::NotConnected => SendError::NotConnected,
                    other => SendError::Engine(Box::new(other)),
                })
        })
    }
}

/// This adapter's declaration when it also sends and correlates replies.
pub const REQUESTING_INFO: AdapterInfo<'static> = AdapterInfo::new(
    PLUGIN_ID,
    ADAPTER_VERSION,
    ENGINE_VERSION,
    SENDING_CAPABILITIES.with(Capability::L0Request),
);

impl StanzaRequester for Sender {
    fn request_frame<'a>(&'a self, frame: &'a [u8]) -> RequestFuture<'a> {
        Box::pin(async move {
            // The engine correlates by the stanza's own id, so the frame has to
            // become a node for it to read one. This is the one place outbound
            // is not opaque bytes, and it is the engine's requirement rather
            // than the boundary's.
            let node = OwnedNodeRef::new(frame.to_vec())
                .map_err(|error| {
                    RequestError::Send(SendError::Engine(Box::new(FrameNotDecodable(error))))
                })?
                .get()
                .to_owned();

            match self.client.send_iq_node(node, None).await {
                Ok(reply) => Ok(reply.backing_bytes().to_vec()),
                Err(whatsapp_rust::IqError::Timeout) => Err(RequestError::TimedOut),
                Err(whatsapp_rust::IqError::NotConnected) => {
                    Err(RequestError::Send(SendError::NotConnected))
                }
                // The engine parses the error reply and keeps the code and text,
                // not the bytes — so there is nothing to hand over here. The
                // `None` says exactly that rather than implying no reply came.
                Err(error @ whatsapp_rust::IqError::ServerError { .. }) => {
                    let _ = error;
                    Err(RequestError::Rejected { frame: None })
                }
                Err(other) => Err(RequestError::Send(SendError::Engine(Box::new(other)))),
            }
        })
    }
}

/// A frame the engine's own decoder could not read.
///
/// Its own type so the failure names itself in a report, instead of arriving as
/// a bare string inside an engine error it did not come from.
#[derive(Debug)]
struct FrameNotDecodable(whatsapp_rust::wacore_binary::BinaryError);

impl core::fmt::Display for FrameNotDecodable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "frame is not a decodable stanza: {}", self.0)
    }
}

impl core::error::Error for FrameNotDecodable {}
