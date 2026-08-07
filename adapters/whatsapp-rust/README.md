# wa-wire adapter for whatsapp-rust

Tap mode: observes every decoded stanza and forwards its frame bytes verbatim.

## Building

This crate is **not** part of the main `wa-wire` workspace. It pulls in the
whole engine — tokio, TLS, protobuf — and putting that in the main workspace
would make every `cargo test` there pay for it. The contract and codec stay
dependency-free on purpose.

It expects a `whatsapp-rust` checkout beside `wa-wire`:

```
projects/
├── wa-wire/
└── whatsapp-rust/
```

```console
cd adapters/whatsapp-rust
cargo test
```

### Dependencies

One engine dependency, not three. `whatsapp-rust` re-exports `wacore` and
`wacore_binary`, so naming them separately would add nothing except a way for
them to drift to a different version than the engine actually links:

```rust
use whatsapp_rust::plugins::{ClientPlugin, PluginContext};
use whatsapp_rust::types::events::{Event, EventHandler};
use whatsapp_rust::{NodeBuilder, OwnedNodeRef};
```

### Toolchain

Pinned to nightly here, and not by choice. `whatsapp-rust` enables
`wacore-binary/simd`, which needs `feature(portable_simd)`. This crate does not
name `wacore-binary` at all and still cannot escape it — a feature cannot be
turned off from a dependent.

The `wa-wire` crates themselves build on stable. Only an adapter is bound to its
engine's toolchain, which is part of what depending on an engine costs.

## Engine patch

The adapter needs the buffer the engine decoded. `OwnedNodeRef` already retains
it as its yoke cart, but nothing exposed it, so this method was added upstream:

```rust
/// The whole backing buffer, verbatim: exactly what `new` consumed.
pub fn backing_bytes(&self) -> Bytes {
    self.inner.backing_cart().0.clone()
}
```

A refcount bump, not a copy — which is what makes `l0.zero-copy-frame` true
here rather than aspirational.

## Two modes, two capability sets

Tap and takeover ride different engine hooks, and the difference in coverage is
real rather than cosmetic.

| Capability | tap (`WaWirePlugin`) | takeover (`takeover::attach`) |
| --- | --- | --- |
| `l0.inbound.tap` | yes | yes |
| `l0.zero-copy-frame` | yes | yes |
| `l0.inbound.auth-phase` | yes | **no** |
| `l0.takeover` | **no** | yes |
| `l0.plaintext` | **no** | **no** |
| `l0.outbound` | **no** | **no** |
| `l0.request` | **no** | **no** |
| `lifecycle.drain-hook` | **no** | **no** |

Neither is a superset of the other, which is why they carry separate
declarations (`INFO` and `takeover::TAKEOVER_INFO`). Every row is asserted in
this crate's tests.

**Tap** rides `Event::RawNode`, emitted before any early return, so it sees
everything the engine decodes — and only observes.

**Takeover** rides `Client::add_stanza_interceptor`. A claimed stanza skips the
engine's built-in handler and is acknowledged all the same, so the server does
not redeliver. It sees what would have reached dispatch, which is less: the
engine refuses to offer `success`, `failure`, `stream:error` and `ack` to an
interceptor, because a consumer that took one would leave the client
authenticated-but-unaware or waiting forever on a completed send.

Running both is fine — a tap watches the whole stream while a takeover claims
the part it handles. The tap fires first.

```rust
use wa_wire_adapter_whatsapp_rust::takeover::{TakeTags, attach};

// Handle receipts ourselves; leave the rest to the engine.
let handle = attach(&client, sink, TakeTags::new(["receipt"]));
```

### Requires the interceptor API

Takeover needs `Client::add_stanza_interceptor`, added in
[oxidezap/whatsapp-rust#1239](https://github.com/oxidezap/whatsapp-rust/pull/1239).
Tap works without it.

### Why no plaintexts

`Event::RawNode` is dispatched where a stanza is decoded, which is necessarily
*before* Signal runs. So this adapter emits **L0-wire**: envelopes it produces
carry a frame and an empty plaintext table.

That is honest rather than degraded — most stanzas (receipts, acks, presence,
IQ) never had anything encrypted. But a `<message>` crosses without its
plaintext, so L0-plain needs a second observation point inside the engine, after
decryption. That is a patch, not a configuration, and it is separate work.

### Why no takeover

`Event::RawNode` observes; the engine's pipeline runs regardless. Suppressing it
would mean overriding the `StanzaRouter`, which panics on duplicate tag
registration. Also separate work.

## Cost when nobody is listening

`Event::RawNode` sits behind a lease the plugin host takes only when a
subscription declares interest in it. Without one the engine skips forwarding
entirely — it does not even wrap an `ack` in an `Arc`. Installing this plugin is
what turns that on; dropping it turns it back off.

Note that turning it on is not free in a second way: with forwarding enabled,
`receipt` and `ack` stop being processed inline and move to spawned tasks. The
contract requires an adapter to disclose that kind of change rather than let a
consumer discover it.
