# wa-wire-l1

Typed canonical events, derived from a parsed stanza and its plaintexts.

L0 is normative and this is the derived view: nothing appears here that is not
derivable from L0-plain. The derivation is **pure** — no key material, no
accumulated state — which is what lets it run host-side, once, instead of being
reimplemented inside every engine.

That purity is also the boundary of what this crate can ever do. See
[Assertions no derivation can make](#assertions-no-derivation-can-make).

```rust
let stanza = Fixture::node("receipt")
    .attr("id", "ABCD1234")
    .jid_attr("from", "5511999998888")
    .attr("type", "read")
    .build();

let event = derive(&parse(&stanza)).expect("derives");
assert_eq!(event.tag(), "receipt");
```

## Two halves, both generated

| Half | Source | Generator |
| --- | --- | --- |
| stanza | whatspec's `incoming` domain | `tools/generate-l1.py` |
| payload | whatspec's `WAProto.proto` | `tools/generate-content.py` |

The `incoming` domain records how WhatsApp Web itself parses each stanza, so the
generator emits *structure* — which extraction primitive to call, in what order,
into which field. The primitives live in `extract.rs` and are hand-written, so a
protocol change moves shapes and calls rather than rules.

Tests for generated code are generated too, from the same shapes. Output is
committed, and CI regenerates and requires no change.

What stays hand-written on the payload side is which variants are worth naming
and where each keeps its text, because no schema says that. `waE2E.Message` has
over a hundred variants and this models a dozen; the rest cross as
`Unmodelled(n)` carrying the field number they were seen under, which is how the
next one gets discovered.

## Three lists of what could not be expressed

A generator that quietly dropped a field would look complete and be wrong, and
no conformance run could catch it — every engine would agree on the same missing
field. So the generators print what they could not express. It is three lists
because it is three different things:

| Constant | What it means | Count | Will it shrink? |
| --- | --- | --- | --- |
| `REQUEST_SCOPED_ASSERTIONS` | checks a pure derivation cannot make | 9 | **no** — a design limit |
| `UNTYPED_FIELDS` | crosses, but below its declared type | 0 | fixed upstream |
| `UNMODELLED_FIELDS` | not modelled yet | 4 | yes — this is the work |

`UNTYPED_FIELDS` emptied without a line changing here. Its one entry was an
`attrEnum` whatspec declared with no variants — the values lived on sibling
shapes as literal guards — so the field read as text.
[oxidezap/whatspec#42](https://github.com/oxidezap/whatspec/pull/42) found the
cause: the extractor refused a numeric property key, and `{0:"0",1:"1",7:"7"}`
is how that enum is written. Fixing it upstream turned this into a typed enum
and recovered 33 more constraints in whatspec's other domains, which is the
argument for reporting what a generator could not express instead of dropping
it.

### Assertions no derivation can make

Nine entries are of the form *"a response's `from` must match the request's
`to`"*. `derive` sees one stanza and no request, so these are unmodellable here
by construction — and giving the derivation a request would end the purity that
lets it run once, host-side. They are emitted as text for the caller that does
hold the request.

`UNMODELLED_FIELDS` is down to four, all union mixins, which need recursive
in-struct dispatch.

## A field is read by its wire name

whatspec records two names per field — the one the bundle uses and the one that
travels — and they differ for fifty of them. This generator read the wrong one
until rev 30, and no generated test could catch it: the fixture builder walked
the same spec by the same rule, so the pair agreed with each other and with
nothing a server sends.

The test pinning this is [hand-written](tests/derive.rs) for exactly that
reason, and was checked against the old behaviour before being kept.

## Provenance travels with the derivation

Two engines disagreeing on L1 raises one question first: did they derive from
the same spec? `Provenance` carries a digest **per domain**, because WhatsApp
can renumber a protobuf field without touching how a stanza parses, and one
digest would call two builds the same spec when only half of it matched.
