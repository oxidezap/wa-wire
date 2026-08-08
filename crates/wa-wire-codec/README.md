# wa-wire-codec

A parser for WhatsApp's binary-node encoding, over pluggable token tables.

[`wa-wire-contract`](../wa-wire-contract) carries a stanza across the boundary
exactly as the engine decoded it — nothing is re-encoded, so the frame inside an
envelope is still in WhatsApp's own format. This crate is what makes that frame
navigable: host-side, once, and only if something asked.

## Nothing is copied

A `NodeRef` borrows the frame and re-walks it on demand. The encoding is
self-delimiting, so a node never needs to know where it ends, and there is
nothing to build up front. Tokens are borrowed from the table; raw payloads are
sub-slices of the frame.

Two forms have text that exists nowhere in the buffer — packed digit runs and
JIDs — and those stay **in parts** rather than being joined. They compare and
render on demand:

```rust
// No allocation: the comparison walks the parts.
let is_text = node.attr_eq("type", "text");
```

This is the difference between a parser you can run per stanza and one you
budget for. The read paths carry
[measured time and allocation budgets](../wa-wire-alloc-check) so the property
cannot quietly stop being true.

## The token table is a parameter

WhatsApp's dictionaries move with the client version. Under RFC-009 that is
*provenance*, not contract version, so the table is passed in rather than
compiled in:

```rust
let parser = Parser::new(tokens::TABLE);              // the bundled table
let custom = Parser::new(TokenTable::new(&[], &[]));  // or your own
```

The bundled table is generated from whatspec and committed
(`python3 tools/generate-tokens.py`), so a dictionary change arrives as a
reviewable diff rather than a build artifact. A host that supplies its own can
drop the `bundled-tokens` feature.

Two recordings parsed under different dictionaries are not comparable, and
saying so is [`wa-wire-recording`](../wa-wire-recording)'s job, not this crate's.

## Paths address nodes

The contract addresses each plaintext by the path of the node it came from; this
crate walks that path. If the two ever disagreed about what a path means, a
decrypted message would be attributed to the wrong recipient — so the agreement
is [asserted in a test](tests/envelope_integration.rs), not assumed.

## Scope

`no_std`, no dependencies. This crate reads the *stanza*; the protobuf inside an
`<enc>` is [`wa-wire-proto`](../wa-wire-proto). Both parse wire formats written
by somebody else, and both are held to the same rule about malformed input:
report it, never panic.
