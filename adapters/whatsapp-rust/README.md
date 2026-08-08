# wa-wire adapter for whatsapp-rust

Observes every decoded stanza, forwards its frame bytes verbatim, joins the
plaintexts the engine decrypted onto it, and can send.

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
| `l0.plaintext` | yes | **no** |
| `l0.inbound.auth-phase` | yes | **no** |
| `l0.takeover` | **no** | yes |
| `l0.outbound` | on `Sender` | on `Sender` |
| `l0.request` | on `Sender` | on `Sender` |
| `lifecycle.drain-hook` | **no** | **no** |

Neither is a superset of the other, which is why they carry separate
declarations (`INFO` and `takeover::TAKEOVER_INFO`). Sending is a third
(`SENDING_INFO`, and `REQUESTING_INFO` when replies are correlated), because an
adapter built to observe genuinely cannot send and one set covering both would
be false for whichever the consumer actually holds. Every row is asserted in
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

### Requires two engine changes

Takeover needs `Client::add_stanza_interceptor`, added in
[oxidezap/whatsapp-rust#1239](https://github.com/oxidezap/whatsapp-rust/pull/1239).
Plaintexts need `Event::DecryptedPayload`, added in
[#1240](https://github.com/oxidezap/whatsapp-rust/pull/1240). A plain tap over
`Event::RawNode` works without either.

### How the plaintexts get there

`Event::RawNode` fires where a stanza is decoded, which is necessarily *before*
Signal runs, so the frame alone is **L0-wire**. `Event::DecryptedPayload`
carries each `<enc>`'s plaintext afterwards, and `plaintext.rs` joins the two:
a `<message>` is held until its payloads arrive, then crosses as one envelope
with its table filled in.

Closing by counting, not by clock. The stanza says how many `<enc>` children it
has, so the last payload completes the table and the envelope goes immediately.
What has no signal is an `<enc>` that will never produce one, so giving up is
measured in **stanzas rather than milliseconds**: the receive path is ordered,
and a count is the same on every machine, which a duration is not.

A fan-out `<message>` is the exception and crosses as L0-wire with no table.
The engine numbers the `<enc>` nodes under `<participants><to>` after the direct
ones and only for its own device, and reproducing that needs the device JID this
adapter does not have. A frame without payloads is a smaller claim than a
payload attached to the wrong `<enc>`, which would read as a message from the
wrong device.

Takeover does not claim `l0.plaintext`: interception happens before decryption
too, and a takeover consumer holds the stanza rather than waiting for payloads.

### Why no drain hook

Nothing in the engine says when incoming handlers have finished, so a consumer
cannot know its queue is quiet. Absent rather than approximated.

### What the engine can now do and this adapter does not

`Event::SentFrame` ([#1260](https://github.com/oxidezap/whatsapp-rust/pull/1260))
reports each marshaled stanza as it was handed to the Noise encryption — the
outbound counterpart of `Event::RawNode`, leased the same way.

This adapter does not surface it, because the contract has no capability for it:
`l0.outbound` means *can send*, not *reports what was sent*, and the eight
capabilities are a versioned surface rather than a list to append to. Naming a
ninth is a contract decision, recorded as D-102 and not yet taken.

Worth stating plainly rather than leaving as an absence: a recording made
through this adapter contains what the session received and nothing it replied,
and that is now a choice rather than a limit.

## Cost when nobody is listening

`Event::RawNode` and `Event::DecryptedPayload` each sit behind a lease the
plugin host takes only when a subscription declares interest in them. Without one the engine skips forwarding
entirely — it does not even wrap an `ack` in an `Arc`. Installing this plugin is
what turns that on; dropping it turns it back off.

Note that turning it on is not free in a second way: with forwarding enabled,
`receipt` and `ack` stop being processed inline and move to spawned tasks. The
contract requires an adapter to disclose that kind of change rather than let a
consumer discover it.
