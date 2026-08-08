# wa-wire-recording

The RFC-010 recording container: envelopes at rest.

[`wa-wire-contract`](../wa-wire-contract) specifies one stanza crossing the
boundary. This is a *sequence* of them in a file, plus the claims that decide
whether two such files may be compared at all: which adapter, which spec, which
dictionary, which traffic.

A container without those claims does not make them absent. It makes them
unverifiable, and a comparison runs and reports a verdict anyway.

## Truncation is a state, not a failure

The record count lives in a trailer rather than the header, so a writer never
has to know its own length before the first byte — which is what lets a ring
buffer be a writer at all. An interrupted recording therefore has no trailer,
and the reader reports `Integrity::Truncated`: every complete record is
readable, and the file is not comparable.

That is deliberate. The artifact a crash recorder exists to produce is, by
definition, the one that was interrupted, and a container that rejected it would
fail its most important use while passing every test written against
well-formed files.

## What it does not do

The trailer's CRC-32 detects damage, not tampering: anything able to rewrite the
records can rewrite the checksum, and nothing here is signed. Identity comes
from `input_digest`, which the container carries as opaque bytes and never
computes — so the hash function stays the responsibility of whoever produced
the traffic.

## Written twice

The format has a second implementation in
[`adapters/zapo/src/recording.ts`](../../adapters/zapo/src/recording.ts), for
the same reason the envelope does: an adapter has to run inside a JavaScript
engine, and two descriptions of one format that are only ever tested separately
are two formats waiting to diverge. Fixtures written by one are read by the
other in
[`cross_language.rs`](../wa-wire-conformance/tests/cross_language.rs).

## Testing

```sh
cargo test -p wa-wire-recording
```
