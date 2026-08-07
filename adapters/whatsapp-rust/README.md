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

### Toolchain

Pinned to nightly here, and not by choice: `whatsapp-rust` enables
`wacore-binary/simd`, which needs `feature(portable_simd)`, and Cargo's feature
unification means this crate cannot opt out. The `wa-wire` crates themselves
build on stable — only an adapter is bound to its engine's toolchain.

## Engine patch

The adapter needs the buffer the engine decoded. `OwnedNodeRef` already retains
it as its yoke cart, but nothing exposed it, so this method was added upstream:

```rust
/// The whole buffer this node was decoded from, verbatim.
pub fn frame_bytes(&self) -> Bytes {
    self.inner.backing_cart().0.clone()
}
```

A refcount bump, not a copy — which is what makes `l0.zero-copy-frame` true
here rather than aspirational.

## Capabilities

| Capability | Status |
| --- | --- |
| `l0.inbound.tap` | yes — `Event::RawNode` fires before any early return |
| `l0.inbound.auth-phase` | yes — `success`, `failure`, `xmlstreamend` all reach it |
| `l0.zero-copy-frame` | yes — the decode buffer is already retained |
| `l0.plaintext` | **no** |
| `l0.outbound` | **no** |
| `l0.takeover` | **no** |
| `l0.request` | **no** |
| `lifecycle.drain-hook` | **no** |

Every row is asserted in `src/tests.rs`. A claim that stops being true fails a
test rather than quietly misleading a consumer.

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
