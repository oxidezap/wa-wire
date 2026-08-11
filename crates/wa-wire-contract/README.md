# wa-wire-contract

The normative envelope format and the types negotiation is built from.

Everything else here is downstream of this crate. A stanza crossing the boundary
is an envelope; whether an adapter and a host may talk at all is decided by the
capability set and provenance in these types.

## The envelope is the wire format

An envelope is **the frame verbatim plus a side table of plaintexts**, each
addressed by the path of the node it belongs to.

Nothing is re-encoded, which means there was no encoding to choose. The frame
bytes already exist inside every engine at the moment it decodes, and the frame
never contained the plaintext anyway — `<enc>` carries ciphertext, and the
plaintext arrives later from Signal. So the boundary carries what the engine
already has, and the only invented part is the side table.

```rust
let envelope = EnvelopeRef::decode(bytes)?;
envelope.frame();                  // the stanza exactly as it arrived
for entry in envelope.entries() {
    entry.path;                    // which node this plaintext came from
    entry.payload;                 // the plaintext itself
}
```

Decoding never allocates and never copies: `EnvelopeRef` borrows from the buffer
it was handed. Encoding allocates exactly once, in `encode_to_vec`. Both claims
are measured rather than asserted — see
[`wa-wire-alloc-check`](../wa-wire-alloc-check).

## A path, not an index

An entry addresses its node by the path from the root, because the alternative —
counting `<enc>` children — is ambiguous the moment a stanza carries anything
else. Getting this wrong attributes a decrypted message to the wrong recipient,
which is why the agreement between this crate's path and the codec's walk is
[asserted in a test](../wa-wire-codec/tests/envelope_integration.rs) rather than
assumed.

## What publication froze

Contract version 1 is fixed as of 0.1.0: the envelope layout, the capability
identifiers named at the time, and the meaning of every field an envelope
carries.

Fixed rather than final. Additive change stays inside version 1 — new
capability names, new reserved flag bits, new metadata tags — because the
format was built to carry what a reader cannot resolve: a recording declares
capabilities by name and keeps the unknown ones, and a metadata tag says with
its critical bit whether skipping it is safe.

Version 2 is for the three things a reader cannot survive by ignoring: a field
that moved, a field that changed meaning, a field that went away.

## Two version axes that must not be confused

| Axis | Versions | Changes when |
| --- | --- | --- |
| **Contract version** | this envelope layout, these capability names | we change the boundary — rare, deliberate |
| **Spec provenance** | which whatspec build L1 derives from | WhatsApp changes — frequent, external |

A WhatsApp-side protocol change must **never** bump the contract version. If it
did, every adapter in the field would break whenever Meta shipped anything,
which would make the project worse than useless.

## Capabilities are declared, never inferred

An adapter says which of `Capability::ALL` it has at setup. A capability the
consumer asked for and the adapter lacks is a setup error — loud, at startup —
not a runtime surprise or a silent degradation.

Naming a new one is additive and stays inside contract version 1: a recording
declares capabilities by name and keeps names it does not recognise as bytes, so
a reader written before a name existed still round-trips a recording that claims
it. What would cost a version is the other direction — removing an identifier or
changing what one means, which turns an old recording's declaration into a lie.

Every one has a provider, which was not true at publication. `l0.plaintext.cause`
was the last to get one: the format had carried `DecryptFailed` and `Unsupported`
since it was written, and every adapter still reported `Unobserved` for a missing
payload because no engine said why one was missing.

The newest is `lifecycle.detach` — releasing a session so another engine can take
it, without unpairing the device. It is the one an engine can be flatly unable to
do, and one of them is.

Not every adapter has every one, and the matrix is the point rather than a
shortfall — a consumer asks for what it needs and finds out at setup.

## Scope

`no_std`, no dependencies, and nothing here parses a stanza — that is
[`wa-wire-codec`](../wa-wire-codec). This crate defines what crosses; the codec
understands what crossed.
