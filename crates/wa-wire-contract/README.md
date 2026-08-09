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

## Two version axes that must not be confused

| Axis | Versions | Changes when |
| --- | --- | --- |
| **Contract version** | this envelope layout, these capability names | we change the boundary — rare, deliberate |
| **Spec provenance** | which whatspec build L1 derives from | WhatsApp changes — frequent, external |

A WhatsApp-side protocol change must **never** bump the contract version. If it
did, every adapter in the field would break whenever Meta shipped anything,
which would make the project worse than useless.

## Capabilities are declared, never inferred

`Capability::ALL` has ten members, and an adapter says which it has at setup.
A capability the consumer asked for and the adapter lacks is a setup error —
loud, at startup — not a runtime surprise or a silent degradation.

The set is a versioned surface rather than a list to append to: adding one after
publication is a version bump, so the vocabulary was audited before freezing.

Two of the ten have no provider. `l0.outbound.observed` has one engine that
could and an adapter that does not yet, and `l0.plaintext.cause` has none at
all — every adapter reports `Unobserved` for a missing payload, and the format
has carried `DecryptFailed` and `Unsupported` since it was written. A name
without a provider costs a line; the same name added later costs a version, and
until then a gate cannot tell a build that stopped decrypting from an adapter
that stopped watching.

## Scope

`no_std`, no dependencies, and nothing here parses a stanza — that is
[`wa-wire-codec`](../wa-wire-codec). This crate defines what crosses; the codec
understands what crossed.
