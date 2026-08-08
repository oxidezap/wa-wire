# wa-wire

A runtime interoperability layer for WhatsApp Web client libraries.

`Baileys`, `whatsmeow`, `zapo` and `whatsapp-rust` all speak the same wire
protocol and all expose different APIs. A production system built on one of them
is structurally married to it: if the library stalls, breaks, or gets abandoned,
migrating means rewriting the integration — even though the protocol underneath
never changed.

`wa-wire` makes the thing they already share — the wire itself — the interface.

> **Status: early implementation.** The design is settled and recorded in
> [`DESIGN.md`](DESIGN.md), which carries ten accepted RFCs, a
> decision log, and a per-revision changelog. The v1 scope is L0 + L1; there is
> no Layer 3 host yet.

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
derivable from L0-plain. That derivation is a **pure** function — no keys and no
accumulated state — so it runs host-side, once, instead of being reimplemented
per engine.

**L1 has two halves, and both are generated from whatspec.** The stanza half
comes from its `incoming` domain, the payload half from the `WAProto.proto` it
extracts out of the WA Web bundle. `derive_content` reads a decrypted payload
and reports which kind of message it is and what it says, unwrapping the
envelopes WhatsApp puts around a message first. What stays hand-written is
which variants are worth naming and where each keeps its text, because no
schema says that.

That half is deliberately small. `waE2E.Message` has over a hundred variants and
this models a dozen; the rest cross as `Unmodelled` carrying their field number,
which is how the next one gets discovered.

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
| [`wa-wire-proto`](crates/wa-wire-proto) | parser for the protobuf wire format, over the payloads the boundary carries |
| [`wa-wire-l1`](crates/wa-wire-l1) | typed canonical events: the stanza from whatspec, the payload from `waE2E.proto` |
| [`wa-wire-recording`](crates/wa-wire-recording) | envelopes at rest, with the provenance that decides whether two files may be compared |
| [`wa-wire-conformance`](crates/wa-wire-conformance) | replays recordings through every engine and requires them to agree |
| [`wa-wire-gate`](crates/wa-wire-gate) | the command: two recordings in, a verdict out |

All of them are `no_std` with no dependencies beyond each other, and none of
them allocates while reading. `wa-wire-gate` is the exception and says so: it is
a tool rather than a library, which is why it is a separate crate.

| Adapter | Engine | Modes |
| --- | --- | --- |
| [`whatsapp-rust`](adapters/whatsapp-rust) | `whatsapp-rust` (Rust) | tap |
| [`zapo`](adapters/zapo) | `zapo` (TypeScript) | tap, takeover |

Adapters live outside the main workspace: each drags in a whole engine, and the
contract and codec stay dependency-free on purpose.

The boundary format is written twice — once in Rust, once in TypeScript, because
an adapter has to run inside a JavaScript engine. Two descriptions of one format
that are only ever tested separately are two formats waiting to diverge, so
[cross-language fixtures](crates/wa-wire-conformance/tests/cross_language.rs)
are written by one and read by the other.

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

## Conformance

The property that makes this more than a wrapper:

> Given the same traffic, every conforming engine must produce the same L1.

Four independent implementations reading one input find bugs that no single
implementation's own tests can, because a bug and its test are usually written
by the same person on the same afternoon. Divergence is the signal.

```rust
let report = compare(&engine_a, &engine_b, tokens::TABLE);
for divergence in report.faults() {
    eprintln!("{divergence}");
}
```

Two layers, and they fail differently. A **frame** difference is not on its own
a fault — the format has more than one way to say a thing, and two encodings of
one stanza are both valid. A **derivation** difference is: the derivation is a
pure function of the stanza, so two engines cannot both be right.

That split is what keeps the report readable. Reporting every byte difference
would bury the handful that matter.

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
python3 tools/generate-l1.py
```

Both print what they could **not** express rather than dropping it silently. A
derivation that quietly omitted a field would look complete and be wrong, and no
conformance run could tell — every engine would agree on the same missing
field.

## License

MIT — see [LICENSE](LICENSE).

The `hypermeow` adapter, when it lands, will be MPL-2.0: it patches
`whatsmeow`, which is MPL-2.0, and that license applies per file. It will live
in its own subdirectory with an explicit notice.
