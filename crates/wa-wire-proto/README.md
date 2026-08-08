# wa-wire-proto

A reader for the protobuf wire format, over the payloads the boundary carries.

The sibling of [`wa-wire-codec`](../wa-wire-codec). That one parses the stanza;
this one parses what the stanza's `<enc>` children decrypt to, which is where
every message body lives.

Same rules, deliberately: `no_std`, no dependencies, borrowing from the buffer
rather than copying out of it. They are kept apart so that the malformed-input
sweep and the allocation counter can hold each to the same standard without
either knowing the other exists.

```rust
// field 1, length-delimited, "hi"
let buf = [0x0a, 0x02, b'h', b'i'];
let text = Reader::new(&buf).find_last(1)?.and_then(Value::as_str);
assert_eq!(text, Some("hi"));
```

## It knows no schema

A field is a number and some bytes; what those mean is the caller's business.

That is not minimalism, it is a requirement. These payloads come from a protocol
that adds fields without asking, and a reader that refused an unknown field
would fail on exactly the traffic worth looking at. The schema-aware half lives
in [`wa-wire-l1`](../wa-wire-l1), where the field numbers are *generated* from
whatspec's `WAProto.proto` rather than written down.

## Last wins

`find_last` rather than `find_first`, because protobuf says a repeated scalar
field takes its last value. A reader that stopped at the first match would
disagree with every real encoder on a message that set a field twice.

## Totality

Every input either parses or reports an error, and no input panics. On a
malformed field the reader **fuses**: it reports the error and yields nothing
further, rather than resynchronising on what might be the middle of a length
prefix and inventing fields that were never sent.

Groups are supported, though nothing in current WhatsApp traffic uses them —
they are still legal on the wire, and a reader that treated a legal encoding as
corrupt would be wrong about the protocol rather than about the message.

These properties are swept with
[deterministic mutations](../wa-wire-conformance/tests/malformed_input.rs)
across every decoder here, since three crates promise them.
