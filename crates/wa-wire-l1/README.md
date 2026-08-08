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

## Three parts, all generated

| Part | Source | Generator |
| --- | --- | --- |
| inbound stanza | whatspec's `incoming` domain | `tools/generate-l1.py` |
| payload | whatspec's `WAProto.proto` | `tools/generate-content.py` |
| outbound stanza | whatspec's `stanza` and `iq` domains | `tools/generate-outgoing.py` |

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
| `UNMODELLED_FIELDS` | not modelled yet | 0 | it was the work |

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

### Mixin groups

The last four entries were union mixins: a field whose value is one of several
named alternatives parsed off the same node. Each becomes an enum, named after
its variants so that the same group under two shapes generates one type.

Alternatives are tried **richest-first**, and the order decides rather than
merely tidies. `NewsletterMessageAck`'s required fields are a subset of
`NewsletterQuestionResponseAck`'s, so the leaner one accepts every stanza the
richer one does — trying it first would claim them all and the richer variant
would never derive. That is D-041 one level down, and a hand-written test pins
it by failing when the order is reversed.

One group has an alternative with no guards and no required fields, so it
matches anything the others turn down. The generated test says so by name: a
group with a catch-all can never report that a stanza matched none of its
alternatives, which is a fact about that group and not a defect.

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

## What the client sends

`derive_outgoing` is a separate derivation, not the same one pointed the other
way. An outbound stanza wears the same tags as an inbound one and means the
opposite — an `<ack>` arriving is the server acknowledging our send, an `<ack>`
leaving is us acknowledging a delivery — so reading one with the inbound grammar
does not fail. It answers confidently and wrongly, which two engines can agree
on.

The sources describe **builders**: an attribute carries a `kind` saying how the
sender produces it (`const`, `dynamic`, a JID flavour) where `incoming` names
the accessor a reader calls. Read backwards a builder is still a shape — a
`const` is a value that must be there, everything else is a field the `required`
flag decides.

### The flavour of a JID is part of the shape

Three pairs of shapes differ in nothing else: an `<ack class="notification">`
addressed to a device is an identity change and to a user a device
notification; one spam report names a group and another a user. Reading all of
them as "a JID" makes one shape out of two and lets either claim the other's
stanzas, so `attr_user_jid`, `attr_device_jid` and `attr_group_jid` are
distinct. `g.us` is entry 45 of the token dictionary, not this crate's guess.

### Builders that describe one stanza

whatspec records a module per builder, and two modules can build the same
stanza while differing in something no reader can see: whether a value is
handed in or computed at build time, or whether one of them models an optional
attribute the other leaves out. Keeping both would be two types no stanza can
choose between, so they are folded and `MERGED_OUTGOING` names the pairs.

The fold is recomputed from the spec on every run, so a pair separates by itself
the day whatspec records something that tells them apart — which is what
[whatspec#43](https://github.com/oxidezap/whatspec/pull/43) does for one of
them, having found a literal the extractor was dropping.

`UNREACHABLE_OUTGOING` is empty and stays for the case the fold does not cover:
a shape *strictly* subsumed by another is still a different shape, merging it
would discard fields the survivor lacks, and a type nothing can reach is worth
naming rather than passing over.
