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

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet, RawStanza, StanzaSink, Violation};
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
}

impl<S> WaWirePlugin<S>
where
    S: StanzaSink + Send + 'static,
{
    /// Forward stanzas to `sink`.
    pub fn new(sink: S) -> Self {
        Self {
            sink: Arc::new(Mutex::new(sink)),
        }
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
        Box::pin(async move {
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
