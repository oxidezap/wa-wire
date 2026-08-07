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
//! | `l0.plaintext` | **no** — `RawNode` fires before Signal has run |
//! | `l0.outbound` | **no** — the engine has no raw outbound observer |
//! | `l0.takeover` | in [`takeover`], not here — `RawNode` observes; the pipeline runs regardless |
//!
//! The plaintext gap is the interesting one. `Event::RawNode` is dispatched at
//! the point a stanza is decoded, which is necessarily *before* decryption. So
//! this adapter produces L0-wire, and an envelope it emits carries no plaintext
//! table. Reaching L0-plain needs a second observation point inside the engine,
//! after Signal — a patch, not a configuration.
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
                EventInterest::of(&[EventKind::RawNode]),
                Arc::new(RawNodeTap { sink }),
            )?;
            // The subscription lives as long as the plugin's resources do; the
            // host drops it on shutdown.
            core::mem::forget(subscription);

            Ok(Arc::new(WaWireApi { _private: () }))
        })
    }
}

struct RawNodeTap<S> {
    sink: Arc<Mutex<S>>,
}

impl<S> EventHandler for RawNodeTap<S>
where
    S: StanzaSink + Send + 'static,
{
    fn handle_event(&self, event: Arc<Event>) {
        let Event::RawNode(node) = &*event else {
            return;
        };
        // A refcount bump on the buffer the decoder already retained.
        let frame = node.backing_bytes();
        let stanza = RawStanza::inbound(&frame);
        debug_assert_eq!(
            INFO.verify(&stanza),
            Ok(()),
            "adapter emitted a stanza its own declaration forbids"
        );
        let Ok(mut sink) = self.sink.lock() else {
            // A poisoned sink means a consumer panicked. Dropping the stanza
            // beats propagating the panic into the engine's receive path.
            return;
        };
        sink.accept(stanza);
    }

    fn interest(&self) -> EventInterest {
        EventInterest::of(&[EventKind::RawNode])
    }
}

/// Check a stanza against [`INFO`].
///
/// Exposed so a host can assert the adapter's claims in its own tests, not only
/// in this crate's.
pub fn verify(stanza: &RawStanza<'_>) -> Result<(), Violation> {
    INFO.verify(stanza)
}

pub mod takeover;

#[cfg(test)]
mod tests;
