# wa-wire adapter for hypermeow

Observes every decoded stanza, forwards its frame bytes verbatim, and joins the
plaintexts the engine decrypted onto it.

The third engine, and the first in a language the core cannot reach.

## Why this one is written in Go

The other two adapters hand their work to Rust: `whatsapp-rust` links it
natively, and `zapo` runs it through WebAssembly. Go can do neither — Rust in
Go means cgo, and cgo in the per-stanza hot path is the cost the whole boundary
exists to avoid.

So the boundary format is written out here for a third time, in Go. That is not
duplication for its own sake; it is the case the design was made for. An
adapter has to run inside its engine, and this proves the contract can be
implemented by someone who cannot use any of our code — which is the difference
between a specification and a library with three callers.

Three descriptions only ever tested separately are three formats waiting to
diverge, so the [fixtures](fixtures) this package writes are
[read back by the Rust side](../../crates/wa-wire-conformance/tests/cross_language_go.rs).
The Go encoder has no Rust to check itself against; that test is the check.

## Building

```console
cd adapters/hypermeow
go test ./...
go run ./cmd/emit-fixtures
```

The engine is a `replace` in `go.mod` rather than a version, pointing at a
`hypermeow` checkout beside `wa-wire`:

```
projects/
├── wa-wire/
└── hypermeow/
```

Deliberately a `replace`: the hooks this is built on live in
[polymorfa/hypermeow#5](https://github.com/polymorfa/hypermeow/pull/5) and
nowhere published yet, and a `replace` says that plainly instead of pinning a
commit that looks like a release. It disappears when the hooks land.

## Capabilities

Declared in `Info`, and checked against every envelope on the way out rather
than left as a comment.

| Capability | Status |
| --- | --- |
| `l0.inbound.tap` | yes — `RawNodeHandler` fires for every decoded stanza |
| `l0.inbound.auth-phase` | yes — the hook *is* the Noise frame callback, so `success` and `failure` reach it |
| `l0.zero-copy-frame` | yes — the engine hands over the buffer it decoded |
| `l0.plaintext` | yes — `DecryptedPayloadHandler` reports each `<enc>` after Signal |
| `l0.takeover` | in `TakeoverInfo`, not here — the hook's `drop` return is native takeover |
| `l0.outbound.observed` | **no** — the engine has no outbound observation point |
| `lifecycle.drain-hook` | **no** — nothing says when incoming handlers have finished |

A recording from this adapter therefore holds the inbound half of a session and
nothing the client replied. Only `whatsapp-rust` can report the other half
today.

## Joining plaintexts to their stanza

The frame arrives before Signal runs and the plaintexts after, so a stanza with
`<enc>` children is held until they catch up. Closing is by **counting, not by
clock**: the stanza says how many `<enc>` children it has, so the last payload
completes the table and the envelope goes at once. Giving up is measured in
**stanzas rather than milliseconds** — the receive path is ordered, and a count
is the same on every machine, which a duration is not.

The Rust and TypeScript adapters reach the same conclusions from the same
constraints. The three are deliberately alike: an adapter that decided
differently would produce recordings that differ for reasons the engines are
not responsible for.

### One thing this adapter does not have to do

Work out which node a payload belongs to. The other two engines report which
`<enc>` decrypted, counting `<enc>` nodes, and their adapters have to resolve
that to a child index — ambiguous the moment a stanza carries anything else,
and unresolvable for a fan-out `<message>`, where the copies for this device
are numbered apart from the direct children. Both of those adapters emit such
stanzas as L0-wire rather than risk attaching a plaintext to the wrong node.

`hypermeow` reports the child index directly, so the path is that number and
nothing is inferred. The hook was written against this need, which is the
advantage of contributing the observation point rather than working around one.

## Licensing

MIT, like the rest of the repository, and [`NOTICE.md`](NOTICE.md) explains why
— the design expected MPL-2.0 here and it turned out not to be needed, which is
worth stating rather than leaving a reader to infer.
