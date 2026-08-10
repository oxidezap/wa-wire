# wa-wire

A runtime interoperability layer for WhatsApp Web client libraries.

`Baileys`, `whatsmeow`, `zapo` and `whatsapp-rust` all speak the same wire
protocol and all expose different APIs. A production system built on one of them
is structurally married to it: if the library stalls, breaks, or gets abandoned,
migrating means rewriting the integration — even though the protocol underneath
never changed.

`wa-wire` makes the thing they already share — the wire itself — the interface.

> **Status: v1 is published.** Seven crates are on crates.io and the contract's
> version 1 is frozen. The design is recorded in [`DESIGN.md`](DESIGN.md), which
> carries ten accepted RFCs, a decision log and a per-revision changelog. The
> v1 scope is L0 + L1: there is no Layer 3 host, and sending is the consumer's.
> [v2 is open](DESIGN.md#v2-scope--layer-3-the-host-and-moving-a-session-between-engines)
> and is Layer 3 — moving a session between engines without re-pairing.

```toml
[dependencies]
wa-wire-l1 = "0.1"                                      # stanza in, typed event out
wa-wire-codec = { version = "0.1", features = ["bundled-tokens"] }
```

```rust
let node = Parser::new(tokens::TABLE).parse(frame)?;
let event = derive(&node)?;   // the same event every conforming engine derives
```

Two adapters are ready to use, and two are built against engine changes still
in review — see [the matrix](DESIGN.md#rfc-002--capability-matrix) for which
capability each one has.

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

**L1 is generated from whatspec, in three parts.** The inbound stanza comes
from its `incoming` domain, the payload from the `WAProto.proto` it extracts out
of the WA Web bundle, and the *outbound* stanza from its `stanza` and `iq`
domains — which describe builders rather than parsers, and are a separate
derivation for that reason.

An outbound stanza wears the same tags as an inbound one and means the opposite:
an `<ack>` arriving is the server acknowledging our send, an `<ack>` leaving is
us acknowledging a delivery. Reading one with the other grammar does not fail,
it answers confidently and wrongly — so the direction picks the grammar, and a
recording says which way each stanza went. `derive_content` reads a decrypted payload
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

Seven are on crates.io; the version links there, the name links to the source.

| Crate | | What it is |
| --- | --- | --- |
| [`wa-wire-contract`](crates/wa-wire-contract) | [0.1.2](https://crates.io/crates/wa-wire-contract) | the normative envelope format and negotiation types — contract version 1, frozen |
| [`wa-wire-codec`](crates/wa-wire-codec) | [0.1.0](https://crates.io/crates/wa-wire-codec) | parser for WhatsApp's binary-node encoding, over pluggable token tables |
| [`wa-wire-adapter`](crates/wa-wire-adapter) | [0.1.0](https://crates.io/crates/wa-wire-adapter) | what an adapter must provide, and the plumbing every Rust adapter shares |
| [`wa-wire-proto`](crates/wa-wire-proto) | [0.1.0](https://crates.io/crates/wa-wire-proto) | parser for the protobuf wire format, over the payloads the boundary carries |
| [`wa-wire-l1`](crates/wa-wire-l1) | [0.1.0](https://crates.io/crates/wa-wire-l1) | typed canonical events: the stanza from whatspec, the payload from `waE2E.proto` |
| [`wa-wire-recording`](crates/wa-wire-recording) | [0.1.0](https://crates.io/crates/wa-wire-recording) | envelopes at rest, with the provenance that decides whether two files may be compared |
| [`wa-wire-conformance`](crates/wa-wire-conformance) | [0.1.0](https://crates.io/crates/wa-wire-conformance) | replays recordings through every engine and requires them to agree |
| [`wa-wire-gate`](crates/wa-wire-gate) | — | two commands: `wa-wire-gate` compares two recordings, `wa-wire-inspect` opens one |
| [`wa-wire-example-consumer`](crates/wa-wire-example-consumer) | — | a consumer written once and run against any engine, proving the boundary holds |
| [`wa-wire-alloc-check`](crates/wa-wire-alloc-check) | — | counts allocations, so the crates that claim not to allocate prove it |

The libraries are `no_std` with no dependencies beyond each other, and none of
them allocates while reading. The three with no version are not published and
say why in their own manifests: `wa-wire-gate` is a pair of tools rather than a
library, `wa-wire-example-consumer` exists to be read, and
`wa-wire-alloc-check` installs a global allocator, which is a `std` thing to do.

Two crates ship less than the repository holds. `wa-wire-l1` leaves out the 4MB
of vendored `whatspec` JSON, which is generator input — the derivation travels
as the generated Rust. `wa-wire-conformance` leaves out the corpus, the frozen
recordings and its integration tests, each of which reads a file that a
dependent has no use for.

| Adapter | Engine | Modes |
| --- | --- | --- |
| [`whatsapp-rust`](adapters/whatsapp-rust) | `whatsapp-rust` (Rust) | tap, takeover, sending |
| [`zapo`](adapters/zapo) | `zapo` (TypeScript) | tap, takeover, sending |
| [`hypermeow`](adapters/hypermeow) | `hypermeow` (Go) | tap, takeover |
| [`Baileys`](adapters/baileys) | `Baileys` (TypeScript) | tap |

Both emit L0-plain. What they cover differs, and the
[capability matrix](DESIGN.md#rfc-002--capability-matrix) is where that is
stated rather than discovered: only the Rust one reaches its engine's own buffer
and the auth phase, only the TypeScript one reports when handlers have drained.

Only the Rust one reports **what the session sent**, through
`l0.outbound.observed`. A recording from either of the others holds the inbound
half of a conversation and nothing the client replied.

Adapters live outside the main workspace: each drags in a whole engine, and the
contract and codec stay dependency-free on purpose.

The envelope is written **three times** — Rust, TypeScript, Go — for four
engines, because an adapter runs inside its engine and the engines are in three
languages. `zapo` and `Baileys` share
[one TypeScript writing](adapters/typescript); a fourth in a language that
already has one would be a description nobody checks against the others. The
first two hand their work to Rust, natively or through WebAssembly; Go can do
neither, since Rust in Go means cgo and cgo in the per-stanza hot path is the
cost the boundary exists to avoid.

That third writing is the case the design was made for. It proves the contract
can be implemented by someone who cannot use any of this code, which is the
difference between a specification and a library with three callers.

Descriptions only ever tested separately are formats waiting to diverge, so
cross-language fixtures are written by one and read by another —
[TypeScript's](crates/wa-wire-conformance/tests/cross_language.rs) and
[Go's](crates/wa-wire-conformance/tests/cross_language_go.rs).

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

**Four engines**, compared pairwise over one corpus — six comparisons, all
agreeing.

What is compared is each engine's own **re-encoding**, not what it forwards.
Three of the four adapters are zero-copy and hand the corpus bytes back
untouched, so comparing those compares three identical streams. Re-encoding is
where four implementations differ, and they do: two of them write different
bytes for five of the fourteen corpus stanzas, and derive the same events from
them.

Worth saying plainly: every finding so far has come from real captured traffic
meeting the derivation, and none from two engines disagreeing. That was the
argument for the third and fourth, and it has not paid off yet.

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
python3 tools/generate-l1.py        # inbound stanzas, from whatspec's `incoming`
python3 tools/generate-content.py   # payloads, from WAProto.proto
python3 tools/generate-outgoing.py  # outbound stanzas, from `stanza` and `iq`
```

They print what they could **not** express rather than dropping it silently. A
derivation that quietly omitted a field would look complete and be wrong, and no
conformance run could tell — every engine would agree on the same missing
field.

What they could not express is reported in three lists, because it is three
different things. `REQUEST_SCOPED_ASSERTIONS` are checks a pure derivation can
never make: a response's `from` matching the request's `to` needs the request,
and derivation sees one stanza. That list is a design limit and will not shrink.
`UNTYPED_FIELDS` is **empty**: its one entry was an enum whatspec declared with
no variants, which
[oxidezap/whatspec#42](https://github.com/oxidezap/whatspec/pull/42) fixed at
the source rather than here. `UNMODELLED_FIELDS` is the one that was actually work,
and it is empty: the last four entries were union mixins, which now generate an
enum apiece.

## License

MIT — see [LICENSE](LICENSE).

The `hypermeow` adapter was expected to be MPL-2.0, on the grounds that it
would carry patched `whatsmeow` files and that licence applies per file. It
carries none: the hooks it needs were contributed to `hypermeow` itself, where
they are MPL-2.0 as everything there is. What is here only *imports* the
engine, which MPL-2.0 §3.3 allows under other terms, so the adapter is MIT like
the rest. [`adapters/hypermeow/NOTICE.md`](adapters/hypermeow/NOTICE.md) says
so explicitly, and says what would change it.
