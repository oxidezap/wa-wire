# wa-wire-example-consumer

A consumer, written once, run against any engine.

This is the claim the project rests on, in code:

> Swap the engine underneath and the consumer does not change.

## The argument is in the dependency tree

What makes the claim true is what this crate does **not** depend on. No engine,
no runtime, no transport, no async — only the boundary types.

```console
cargo tree -p wa-wire-example-consumer
```

Four crates, and none of them is a WhatsApp client. Code that cannot name an
engine cannot be coupled to one, and that is a stronger statement than any test
of behaviour: it is checked by the compiler on every build.

## The logic is deliberately small

Count what arrived, remember the ids. The interesting part is not what a
consumer computes — it is that the same bytes produce the same answer no matter
who produced them.

```rust
let mut tally = Tally::default();
for envelope in envelopes {
    tally.accept(envelope, table)?;
}
```

A consumer with real logic would make the example about the logic. This one
makes it about the boundary.

## How it is used

[`wa-wire-conformance`](../wa-wire-conformance) runs this same `Tally` over
recordings from each adapter and requires the results to match. A difference
here means the engines did not deliver the same traffic — which is the whole
point of having a consumer that is provably identical on both sides.
