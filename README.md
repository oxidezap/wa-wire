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
| [`wa-wire-example-consumer`](crates/wa-wire-example-consumer) | a consumer written once and run against any engine, proving the boundary holds |
| [`wa-wire-alloc-check`](crates/wa-wire-alloc-check) | counts allocations, so the crates that claim not to allocate prove it |

The libraries are `no_std` with no dependencies beyond each other, and none of
them allocates while reading. Two are deliberately not: `wa-wire-gate` is a tool
rather than a library, and `wa-wire-alloc-check` installs a global allocator,
which is a `std` thing to do. Both are `publish = false` and say why in their
own manifests.

| Adapter | Engine | Modes |
| --- | --- | --- |
| [`whatsapp-rust`](adapters/whatsapp-rust) | `whatsapp-rust` (Rust) | tap, takeover, sending |
| [`zapo`](adapters/zapo) | `zapo` (TypeScript) | tap, takeover, sending |

Both emit L0-plain. What they cover differs, and the
[capability matrix](DESIGN.md#rfc-002--capability-matrix) is where that is
stated rather than discovered: only the Rust one reaches its engine's own buffer
and the auth phase, only the TypeScript one reports when handlers have drained.

Adapters live outside the main workspace: each drags in a whole engine, and the
contract and codec stay dependency-free on purpose.

Both boundary formats — the envelope and the recording container that holds
several of them — are written twice, once in Rust and once in TypeScript,
because an adapter has to run inside a JavaScript engine. Two descriptions of
one format that are only ever tested separately are two formats waiting to
diverge, so [cross-language fixtures](crates/wa-wire-conformance/tests/cross_language.rs)
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

Independent implementations reading one input find bugs that no single
implementation's own tests can, because a bug and its test are usually written
by the same person on the same afternoon. Divergence is the signal.

**Two engines today**, of the four the definition of done asks for. Two
agreeing is weaker evidence than four: they can be wrong the same way. Every
finding so far has come from real captured traffic meeting the derivation
rather than from the two disagreeing, which is exactly what a third engine
would change.

```rust
let report = compare(&engine_a, &engine_b, Tables::shared(tokens::TABLE));
match report.evaluate(ComparisonProfile::Interop) {
    Verdict::Pass => {}
    Verdict::Fail => report.failures(ComparisonProfile::Interop).for_each(|d| eprintln!("{d}")),
    Verdict::Incomparable(why) => eprintln!("nothing was established: {why}"),
}
```

**The same evidence answers two different questions.** Between two engines, a
frame difference is two valid encodings of one stanza; between two builds of one
engine, it is the encoder changing under you. So the comparator records facts and
a [profile](DESIGN.md#rfc-005-amendment--comparison-profiles) judges them.

The verdict is three-valued because a comparison between unlike things is not a
disagreement. Recordings say what traffic they replay, and a pair that cannot
establish it reports `Incomparable` rather than a green result from a comparison
that never ran.

[`wa-wire-gate`](crates/wa-wire-gate) is that as a command, with an exit code per
answer.

## Development

```console
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo llvm-cov --workspace --all-features --summary-only
```

`cargo doc` is on that list because `cargo test` does not build documentation,
so a broken intra-doc link passes every other check and fails CI.

Line coverage must stay at or above **95%**; CI enforces it. Portability is also
enforced: everything builds without an allocator and for
`wasm32-unknown-unknown`, since JS adapters consume the core through
WebAssembly.

Three claims that used to be documentation are now tests. Malformed input is
swept with deterministic mutations across every decoder, since three crates
promise it is reportable and never a panic. Allocation counts are measured,
since five places claim not to allocate while reading. Each read path carries a
time budget, since a benchmark whose only output is a number leaves the reader
unable to say whether it is fine.

### Generated code

Everything generated is committed, so a protocol change arrives as a reviewable
diff rather than a build artifact. CI regenerates and requires no change:

```console
python3 tools/generate-tokens.py    # the token dictionaries
python3 tools/generate-l1.py        # the stanza half of L1, from whatspec
python3 tools/generate-content.py   # the payload half, from WAProto.proto
```

They print what they could **not** express rather than dropping it silently. A
derivation that quietly omitted a field would look complete and be wrong, and no
conformance run could tell — every engine would agree on the same missing
field.

## License

MIT — see [LICENSE](LICENSE).

The `hypermeow` adapter, when it lands, will be MPL-2.0: it patches
`whatsmeow`, which is MPL-2.0, and that license applies per file. It will live
in its own subdirectory with an explicit notice.
