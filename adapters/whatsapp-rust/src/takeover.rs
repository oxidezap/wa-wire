//! Takeover: claiming a stanza instead of only watching it.
//!
//! Tap mode ([`WaWirePlugin`]) rides `Event::RawNode`, which is emitted before
//! any early return and therefore sees everything the engine decodes — but only
//! observes. The engine goes on to parse, dispatch and update its own state
//! regardless.
//!
//! Takeover rides `Client::add_stanza_interceptor` instead. A claimed stanza
//! skips the engine's built-in handler and is acknowledged all the same, so the
//! server does not redeliver. Under takeover the engine's own semantics stop
//! mattering, which is what makes engines interchangeable.
//!
//! [`WaWirePlugin`]: crate::WaWirePlugin
//!
//! # The two are not the same coverage
//!
//! They cannot be, and pretending otherwise would be the dishonest option:
//!
//! | | tap | takeover |
//! | --- | --- | --- |
//! | sees | every decoded stanza | what would have reached dispatch |
//! | `success`, `failure`, `stream:error`, `ack` | yes | **no** — the engine refuses to offer them |
//! | already-correlated IQ responses, `xmlstreamend` | yes | no |
//! | suppresses the engine | no | yes |
//!
//! The gap is deliberate on the engine's side: those four stanzas settle
//! authentication, shutdown and the waiters a send blocks on, and a consumer
//! that took one would leave the client authenticated-but-unaware or waiting
//! forever. That makes takeover's capability set genuinely different from
//! tap's, not a superset — hence [`TAKEOVER_CAPABILITIES`].
//!
//! # Running both
//!
//! Nothing stops it: a tap sees the whole stream while a takeover claims the
//! part it handles. The tap fires first, since `Event::RawNode` is dispatched
//! before interception.

use std::sync::{Arc, Mutex};

use wa_wire_adapter::{
    AdapterInfo, Capability, CapabilitySet, RawStanza, StanzaSink, UnmetCapabilities,
};
use whatsapp_rust::client::interceptor::{Interception, InterceptorHandle, StanzaInterceptor};
use whatsapp_rust::{Client, OwnedNodeRef};

use crate::{ADAPTER_VERSION, ENGINE_VERSION, PLUGIN_ID};

/// What this adapter can do in takeover mode.
///
/// `l0.inbound.auth-phase` is absent where tap has it: the engine does not
/// offer connection-critical stanzas to an interceptor, so takeover cannot see
/// the authentication exchange.
pub const TAKEOVER_CAPABILITIES: CapabilitySet = CapabilitySet::NONE
    .with(Capability::L0InboundTap)
    .with(Capability::Takeover)
    .with(Capability::ZeroCopyFrame);

/// This adapter's declaration in takeover mode.
pub const TAKEOVER_INFO: AdapterInfo<'static> = AdapterInfo::new(
    PLUGIN_ID,
    ADAPTER_VERSION,
    ENGINE_VERSION,
    TAKEOVER_CAPABILITIES,
);

/// Whether a claimed stanza should also be suppressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Claim {
    /// Forward it and let the engine carry on.
    ///
    /// Useful for watching the dispatch path specifically, without the wider
    /// reach — or the extra work — of a tap.
    #[default]
    Observe,
    /// Forward it and suppress the engine's own handler.
    Take,
}

impl Claim {
    const fn interception(self) -> Interception {
        match self {
            Self::Observe => Interception::Pass,
            Self::Take => Interception::Handled,
        }
    }
}

/// Decides, per stanza, whether to claim it.
///
/// Taking *every* stanza is rarely what anyone wants: an engine reduced to a
/// transport still has to be told what to do with the rest.
pub trait ClaimPolicy: Send + Sync + 'static {
    /// What to do with this stanza.
    fn claim(&self, node: &OwnedNodeRef) -> Claim;
}

impl<F> ClaimPolicy for F
where
    F: Fn(&OwnedNodeRef) -> Claim + Send + Sync + 'static,
{
    fn claim(&self, node: &OwnedNodeRef) -> Claim {
        self(node)
    }
}

/// Claims every stanza the engine offers.
///
/// Full takeover: the engine keeps doing Noise, decryption and acks, and stops
/// interpreting anything it is allowed to hand over.
#[derive(Debug, Clone, Copy, Default)]
pub struct TakeEverything;

impl ClaimPolicy for TakeEverything {
    fn claim(&self, _node: &OwnedNodeRef) -> Claim {
        Claim::Take
    }
}

/// Claims only the stanzas whose tag is in a list.
#[derive(Debug, Clone)]
pub struct TakeTags {
    tags: Vec<&'static str>,
}

impl TakeTags {
    /// Claim these tags and pass the rest.
    #[must_use]
    pub fn new(tags: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            tags: tags.into_iter().collect(),
        }
    }
}

impl ClaimPolicy for TakeTags {
    fn claim(&self, node: &OwnedNodeRef) -> Claim {
        if self.tags.iter().any(|tag| *tag == node.tag()) {
            Claim::Take
        } else {
            Claim::Observe
        }
    }
}

/// Forward stanzas to `sink`, claiming the ones `policy` selects.
///
/// The returned handle keeps the interceptor registered; dropping it removes
/// it and the engine goes back to handling everything itself.
///
/// ```no_run
/// use std::sync::Arc;
/// use wa_wire_adapter::CountingSink;
/// use wa_wire_adapter_whatsapp_rust::takeover::{TakeTags, attach};
///
/// # fn example(client: &Arc<whatsapp_rust::Client>) {
/// // Handle receipts ourselves; leave the rest to the engine.
/// let handle = attach(client, CountingSink::new(), TakeTags::new(["receipt"]));
/// # let _ = handle;
/// # }
/// ```
pub fn attach<S, P>(client: &Arc<Client>, sink: S, policy: P) -> InterceptorHandle
where
    S: StanzaSink + Send + 'static,
    P: ClaimPolicy,
{
    client.add_stanza_interceptor(Arc::new(Interceptor {
        sink: Arc::new(Mutex::new(sink)),
        policy,
    }))
}

/// Like [`attach`], but refuses unless takeover mode has every capability in
/// `needed`.
///
/// Takeover's capability set is genuinely different from the tap's — it cannot
/// see the auth phase — so "the adapter supports it" is not one question here.
/// Stating what a consumer relies on is how that difference stops being a
/// footnote someone has to have read.
///
/// # Errors
///
/// [`UnmetCapabilities`] naming everything takeover mode lacks, before anything
/// is registered.
pub fn attach_requiring<S, P>(
    client: &Arc<Client>,
    sink: S,
    policy: P,
    needed: CapabilitySet,
) -> Result<InterceptorHandle, UnmetCapabilities>
where
    S: StanzaSink + Send + 'static,
    P: ClaimPolicy,
{
    TAKEOVER_INFO.require(needed)?;
    Ok(attach(client, sink, policy))
}

/// Like [`attach`], but keeps a handle on the sink.
pub fn attach_shared<S, P>(
    client: &Arc<Client>,
    sink: Arc<Mutex<S>>,
    policy: P,
) -> InterceptorHandle
where
    S: StanzaSink + Send + 'static,
    P: ClaimPolicy,
{
    client.add_stanza_interceptor(Arc::new(Interceptor { sink, policy }))
}

pub(crate) struct Interceptor<S, P> {
    pub(crate) sink: Arc<Mutex<S>>,
    pub(crate) policy: P,
}

impl<S, P> StanzaInterceptor for Interceptor<S, P>
where
    S: StanzaSink + Send + 'static,
    P: ClaimPolicy,
{
    fn intercept(&self, node: &OwnedNodeRef) -> Interception {
        // Forwarded before the decision, so a claimed stanza is never one the
        // consumer did not receive.
        let frame = node.backing_bytes();
        let stanza = RawStanza::inbound(&frame);
        debug_assert_eq!(
            TAKEOVER_INFO.verify(&stanza),
            Ok(()),
            "adapter emitted a stanza its own declaration forbids"
        );

        let Ok(mut sink) = self.sink.lock() else {
            // A poisoned sink means a consumer panicked. Claiming a stanza it
            // never received would drop it silently, so pass instead and let
            // the engine handle it.
            return Interception::Pass;
        };
        sink.accept(stanza);
        drop(sink);

        self.policy.claim(node).interception()
    }
}

#[cfg(test)]
#[path = "takeover_tests.rs"]
mod tests;
