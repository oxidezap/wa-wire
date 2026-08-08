# wa-wire-adapter

What an adapter must provide, and the plumbing every Rust adapter shares.

An adapter is the thin piece that lives inside an engine, observes stanzas, and
hands them on. It is deliberately dumb: emit what the engine saw and stop.

That is the whole design argument. Everything that could be interpreted
differently between engines — parsing, L1 derivation — happens host-side, once.
An adapter with no interpretation in it has nothing to diverge on, and little to
break when the engine moves underneath it. There are four engines and one
derivation, not four derivations.

## What crosses

A `RawStanza` is the pre-encoding shape of an envelope: the frame bytes the
engine decoded, plus any payloads it decrypted, each addressed by the path of
the node it came from.

```rust
let mut path = NodePathBuf::new();
path.push(0).expect("within the depth limit");
let plaintexts = [Plaintext::ok(path.as_path(), decrypted)];

let stanza = RawStanza::inbound(frame).with_plaintexts(&plaintexts);
sink.accept(stanza);
```

## A sink takes the stanza, not a buffer

A sink receives the `RawStanza` rather than a finished byte buffer, so an
in-process consumer can read the frame straight out and never encode anything.
Encoding is what you pay to cross a process or a language boundary; it should
not be what you pay to cross a function call.

## Declaring what you have

An adapter publishes an `AdapterInfo`: its capability set and its provenance. A
capability the consumer asked for and the adapter lacks is a **setup error**,
raised at startup, never a runtime surprise.

Modes carry separate declarations, because they are genuinely different. In the
`whatsapp-rust` adapter, tap sees the auth phase and the plaintexts while
takeover sees neither, and takeover can claim a stanza while tap cannot. Neither
is a superset of the other, so one set covering both would be false for whichever
the consumer is actually holding.

The same rule covers **cost disclosure**: if enabling a subscription changes
engine behaviour beyond adding a callback — rescheduling work off the read loop,
say — the adapter must say so. Silent scheduling changes under observation are a
conformance violation.

## Paths are bounded

`NodePathBuf` has a depth limit and `push` returns a result. A stanza is
attacker-adjacent input, and an unbounded path is a way to make a reader
allocate on someone else's say-so.

## Scope

`no_std`, no dependencies beyond [`wa-wire-contract`](../wa-wire-contract).
Adapters themselves live outside this workspace, under `adapters/`, because each
drags in a whole engine.
