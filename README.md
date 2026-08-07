# wa-wire

A runtime interoperability layer for WhatsApp Web client libraries.

`Baileys`, `whatsmeow`, `zapo` and `whatsapp-rust` all speak the same wire
protocol and all expose different APIs. A production system built on one of them
is structurally married to it: if the library stalls, breaks, or gets abandoned,
migrating means rewriting the integration — even though the protocol underneath
never changed.

`wa-wire` makes the thing they already share — the wire itself — the interface.

> **Status: early implementation.** The design is settled and recorded in
> [`DESIGN.md`](DESIGN.md), which carries nine accepted RFCs, a decision log, and
> a per-revision changelog. The v1 scope is L0 + L1; there is no Layer 3 host
> yet.

## The idea

The ecosystem already had two intermediate representations and was missing a
third:

| Project | IR | Domain |
| --- | --- | --- |
| [`whatspec`](https://github.com/oxidezap/whatspec) | protocol surface | **spec** — static |
| [`wa-store-migrate`](https://github.com/vinikjkkj/wa-store-migrate) | `WaSnapshot` | **state** — at rest |
| **`wa-wire`** | envelope + L1 | **runtime** — in flight |

## Layers

- **L0-wire** — the stanza as it arrived, payload still encrypted.
- **L0-plain** — that frame plus the plaintexts the engine decrypted.
- **L1** — typed canonical events, derived from L0-plain.

L0 is normative and L1 is a derived view: nothing may appear in L1 that is not
derivable from L0-plain. That derivation is a **pure** function — protobuf
parsing and mapping, no keys and no accumulated state — so it runs host-side,
once, instead of being reimplemented per engine.

## What crosses the boundary

The frame bytes already exist inside every engine at the moment it decodes, and
the frame never contained the plaintext anyway: `<enc>` carries ciphertext, and
the plaintext arrives later from Signal.

So an envelope is **the frame verbatim plus a side table of plaintexts**, each
addressed by the path of the node it belongs to. Nothing is re-encoded, so there
is no encoding to choose — the boundary format *is* the wire format. The frame
is parsed exactly once, host-side, and only if something subscribed to L1.

## Crates

| Crate | What it is |
| --- | --- |
| [`wa-wire-contract`](crates/wa-wire-contract) | the normative envelope format and negotiation types |
| [`wa-wire-codec`](crates/wa-wire-codec) | parser for WhatsApp's binary-node encoding, over pluggable token tables |
| [`wa-wire-adapter`](crates/wa-wire-adapter) | what an adapter must provide, and the plumbing every Rust adapter shares |

All three are `no_std` with no dependencies beyond each other, and none of them
allocates while reading.

| Adapter | Engine | Mode |
| --- | --- | --- |
| [`whatsapp-rust`](adapters/whatsapp-rust) | `whatsapp-rust` | tap |

Adapters live outside the main workspace: each drags in a whole engine, and the
contract and codec stay dependency-free on purpose.

More arrive in the order set out in [`DESIGN.md` §8](DESIGN.md#8-implementation-plan).

### How they fit together

An envelope addresses each decrypted payload by the path of the node it came
from. The contract carries the path; the codec walks it:

```rust
let envelope = EnvelopeRef::decode(bytes)?;
let root = Parser::new(tokens::TABLE).parse(envelope.frame())?;

for entry in envelope.entries() {
    let node = root.at_path(entry.path.iter()).expect("addresses a node");
    // `node` is the <enc> that `entry.payload` decrypted from.
}
```

If the two ever disagreed about what a path means, a decrypted message would be
attributed to the wrong recipient — so the agreement is asserted in
[an integration test](crates/wa-wire-codec/tests/envelope_integration.rs), not
assumed.

## Development

```console
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
cargo llvm-cov --workspace --all-features --summary-only
```

Line coverage must stay at or above **95%**; CI enforces it. Portability is also
enforced: everything builds without an allocator and for
`wasm32-unknown-unknown`, since JS adapters consume the core through
WebAssembly.

The token dictionaries are generated and committed, so a protocol change arrives
as a reviewable diff rather than a build artifact. CI regenerates them and
requires no change:

```console
python3 tools/generate-tokens.py
```

## License

MIT — see [LICENSE](LICENSE).

The `hypermeow` adapter, when it lands, will be MPL-2.0: it patches
`whatsmeow`, which is MPL-2.0, and that license applies per file. It will live
in its own subdirectory with an explicit notice.
