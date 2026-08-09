# wa-wire-conformance

Replay recorded stanzas through every engine and require them to agree.

This is the property that makes `wa-wire` more than a wrapper:

> Given the same traffic, every conforming engine must produce the same L1.

Independent implementations reading one input find bugs that no single
implementation's own tests can, because a bug and its test are usually written
by the same person on the same afternoon. **Divergence is the signal.**

## Four engines, frozen

`recordings/*.wawr` holds one engine's re-encoded corpus stream each, and
[`tests/engine_agreement.rs`](tests/engine_agreement.rs) compares all six pairs
on every push. No engine is needed to run it: comparing needs four byte streams
and a token table.

Producing them does need all four, so they are refreshed by
`cargo run --example emit-agreement-recordings` in the `whatsapp-rust` adapter.
Each file carries the corpus digest, so a recording of traffic that has since
changed is refused rather than compared.

What this catches is our own side moving — a derivation or codec change that
makes four engines stop agreeing. What it cannot catch is an engine moving,
since a committed recording is a photograph of one.

## Two layers that fail differently

- **L0** — the frame bytes each engine forwarded. Byte-identical frames mean the
  engines saw the same stanza. Different ones are *not* necessarily a bug: two
  encodings of one stanza are both valid.
- **L1** — the events derived from those frames, compared by **meaning**. Two
  engines that encode a value differently and derive the same event agree.

Reporting every L0 difference would bury the L1 ones, and L1 is where
correctness lives. So the comparator records both and separates *context* from
*faults*.

## The direction picks the grammar

An outbound stanza is derived with the outbound derivation and an inbound one
with the inbound derivation, and picking wrong is worse than not deriving at
all. The two grammars accept the same tags and disagree about what they mean, so
a stanza read the wrong way does not fail — it produces a confident wrong
answer, and two engines agreeing on a wrong reading reports as agreement.

A pair whose envelopes disagree on direction is a finding in its own right, and
it is raised before either is derived.

## Each direction is its own sequence

An engine dispatches what it received from the read path and what it sent from
the send path, and there is no ordering between the two. One merged sequence
compared by position would call a different interleaving a divergence on every
stanza after it, so the inbound halves are compared against each other and the
outbound halves against each other. Within a direction the order is the
engine's own and is stable.

Whether a *missing* direction is a fault depends on who was watching. Two
adapters that both observe the outbound half and disagree on how much of it
there was have found something; one that cannot see it at all has not, and
counting that as missing stanzas would blame an engine for its observer. The
declared capability decides. A recording carrying a direction its own manifest
does not claim is a fault under every profile — nothing downstream can tell
whether those records are real.

## One body of evidence, two questions

| Profile | Asks | A frame difference is |
| --- | --- | --- |
| `Interop` | do these two engines agree? | two valid encodings — context |
| `Regression` | did this build change? | the encoder moving under you — a fault |

Same facts, opposite verdicts. That is why the comparator records facts and a
*profile* judges them, rather than baking one question's answer into the
comparison.

## The verdict is three-valued

`Pass`, `Fail`, and **`Incomparable`** — which is not a pass.

A comparison between unlike things is not a disagreement. Recordings declare
what traffic they replay, which adapter produced them, which spec and which
dictionary; a pair that cannot establish comparability reports why instead of
returning a green result from a comparison that never ran.

```rust
match report.evaluate(ComparisonProfile::Interop) {
    Verdict::Pass => {}
    Verdict::Fail => report.failures(ComparisonProfile::Interop).for_each(|d| eprintln!("{d}")),
    Verdict::Incomparable(why) => eprintln!("nothing was established: {why}"),
}
```

Comparability is **declared, not assumed**: two recordings that both decline to
say what they replay are vouched for by neither, and a live capture declares no
input digest and is never gate-comparable.

## Also here

- **[The malformed-input sweep](tests/malformed_input.rs)** — deterministic
  mutations across every decoder in the workspace, asserting invariants rather
  than merely the absence of a panic. A decoder that survives by accepting
  nonsense has not survived. It also asserts that mutations land on *both*
  sides, since a sweep where everything is refused proves only that the first
  length check works, and can become that silently.
- **[Cross-language fixtures](tests/cross_language.rs)** — the envelope and the
  recording container are written twice, in Rust and in TypeScript, because an
  adapter has to run inside a JavaScript engine. Two descriptions of one format
  that are only ever tested separately are two formats waiting to diverge, so
  each is written by one and read by the other.

## Status

**Two engines today**, of the four the definition of done asks for. Two agreeing
is weaker evidence than four: they can be wrong the same way. Every finding so
far has come from real captured traffic meeting the derivation rather than from
the two disagreeing — which is exactly what a third engine would change.

[`wa-wire-gate`](../wa-wire-gate) is this as a command, with an exit code per
verdict.
