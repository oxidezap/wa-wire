//! Which of the engine's three ways to stop this adapter's detach is.
//!
//! `pause`, `disconnect` and `logout` all end a connection, and they differ in
//! what survives: a pause comes back on the caller's word, a disconnect is
//! terminal for that client, and a logout unpairs the device for good. A handoff
//! needs the first and is ruined by the other two, and reading the call site is
//! not proof — this checks the engine's own state afterwards.
//!
//! No server is involved. A client that never connected can still be paused,
//! which is the point: the state this asserts is the client's, not a socket's.

#![allow(clippy::expect_used)]

use std::sync::Arc;

use wa_wire_adapter::handoff::Detach;
use wa_wire_adapter::{Capability, CapabilitySet};
use wa_wire_adapter_whatsapp_rust::{
    CAPABILITIES, DETACHING_CAPABILITIES, DETACHING_INFO, Detacher,
};
use whatsapp_rust::store::persistence_manager::PersistenceManager;
use whatsapp_rust::{Client, ClientBuilder};
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;

/// A client with a store and a transport factory it never uses.
async fn offline_client(name: &str) -> Arc<Client> {
    let store = format!("file:wa-wire-detach-{name}?mode=memory&cache=shared");
    let backend = whatsapp_rust::store::SqliteStore::new(&store)
        .await
        .expect("an in-memory store");
    let persistence = Arc::new(
        PersistenceManager::new(Arc::new(backend))
            .await
            .expect("a persistence manager"),
    );

    ClientBuilder::new()
        .with_runtime(whatsapp_rust::runtime_impl::TokioRuntime)
        .with_persistence_manager(persistence)
        // Never dialled. A factory is required to build, and this test does not
        // connect.
        .with_transport_factory(TokioWebSocketTransportFactory::new().with_url("ws://127.0.0.1:1"))
        .with_http_client(whatsapp_rust::http::UreqHttpClient::new())
        .build()
        .await
        .expect("a client")
        .into_client()
}

#[tokio::test]
async fn detaching_pauses_the_session_rather_than_ending_it() {
    let client = offline_client("pauses").await;
    let detacher = Detacher::new(Arc::clone(&client));

    // Through the trait object, because that is how a host driving a handoff
    // holds it — and that surface has no way to log out.
    let releasing: &dyn Detach = &detacher;
    releasing.detach().await.expect("released");

    assert!(
        client.is_paused(),
        "a detach has to leave the session resumable; `disconnect()` would not have"
    );
    assert!(!client.is_connected());

    // And the session comes back on the caller's word, which is the property
    // that makes a handoff reversible: the engine is between connections, not
    // finished with them.
    client.resume();
    assert!(!client.is_paused());
}

#[tokio::test]
async fn detaching_twice_is_not_an_error() {
    // A host that crashed mid-handoff has to be able to start over.
    let client = offline_client("twice").await;
    let detacher = Detacher::new(Arc::clone(&client));

    detacher.detach().await.expect("released");
    detacher.detach().await.expect("released again");

    assert!(client.is_paused());
}

#[test]
fn only_the_detaching_declaration_claims_it() {
    // The tap has no client and so cannot release anything. A single set
    // covering both would be false for whichever the consumer actually holds.
    assert!(!CAPABILITIES.contains(Capability::Detach));
    assert!(DETACHING_CAPABILITIES.contains(Capability::Detach));
    assert_eq!(DETACHING_INFO.capabilities, DETACHING_CAPABILITIES);

    // Nothing else moved: detaching is one addition, not a different adapter.
    assert_eq!(
        DETACHING_CAPABILITIES.without(Capability::Detach),
        CAPABILITIES
    );
    let _: CapabilitySet = CAPABILITIES;
}
