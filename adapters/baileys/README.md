# wa-wire adapter for Baileys

Observes every decoded stanza, forwards its frame bytes verbatim, and joins the
plaintexts the engine decrypted onto it.

The fourth engine, and the one the definition of done was waiting on.

## The format is not written again

Three languages, three writings: Rust, TypeScript, Go. This adapter is the
second in TypeScript, so it shares the first one's — `@oxidezap/wa-wire-ts`,
extracted out of the `zapo` adapter for this. A fourth writing in a language
that already has one would be a description nobody checks against the others,
which is the opposite of what writing it three times is for.

What is specific to Baileys is the joining and the declaration. Both are small.

## Installing

Two config callbacks, because Baileys has no plugin host:

```ts
import makeWASocket from 'baileys'
import { toEnvelope, waWire } from '@oxidezap/wa-wire-adapter-baileys'

const wire = waWire(stanza => record(toEnvelope(stanza)))
const sock = makeWASocket({ ...config, ...wire.config })

// At shutdown, so a frame waiting on a payload that will never arrive is
// emitted unobserved rather than lost.
wire.flush()
```

Nothing to unregister: a caller that wants to stop stops passing the callbacks.

## What it needs from Baileys

Two observation points, both added for this and neither present upstream:

| Hook | What it carries | Why it did not exist |
| --- | --- | --- |
| `onFrameDecoded` | the node **and the buffer it was decoded from** | the buffer was in scope in `processData` and fell out of it |
| `onDecryptedPayload` | each `<enc>`'s plaintext, by child index | nothing carried a plaintext outside the parse that consumed it |

`decodeBinaryNodeWithBuffer` is the change that makes the first possible: the
same work `decodeBinaryNode` does, handing back the decompressed bytes as well.

The second fires **before the protobuf is parsed**, and fires for a payload
whose padding will not strip. The ratchet has advanced by then, so the plaintext
exists exactly once — throwing before handing it over loses it for good, and the
observer sees a message that arrived and never a reason it did not parse. The
`hypermeow` adapter found that same defect one layer deeper in its engine.

## Capabilities

Declared in `INFO`, and checked against every envelope on the way out rather
than left as a comment.

| Capability | Status |
| --- | --- |
| `l0.inbound.tap` | yes — the hook is in the frame loop, so nothing is filtered out of it |
| `l0.inbound.auth-phase` | yes — before anything decides what a stanza is, so `success` and `failure` reach it |
| `l0.zero-copy-frame` | yes — the buffer the decoder consumed, not a re-encoding |
| `l0.plaintext` | yes — one callback per `<enc>` |
| `l0.takeover` | **no** — the hook observes; the pipeline runs regardless |
| `l0.outbound.observed` | **no** — nothing reports what the client sent |
| `lifecycle.drain-hook` | **no** — nothing says when handlers have drained |

The auth phase is worth noting against `zapo`, which protects `success` and
`failure` from its stanza filters and so cannot see the login exchange. Being
inside the Noise loop rather than behind a dispatcher is what buys it here.

## Joining plaintexts to their stanza

The frame arrives before Signal runs and the plaintexts after, so a stanza with
`<enc>` children is held until they catch up. Closing is by **counting, not by
clock**: the stanza says how many `<enc>` children it has, so the last payload
completes the table. Giving up is measured in **stanzas rather than
milliseconds** — a count is the same on every machine, which a duration is not,
and this output is compared against other engines'.

### Stanzas leave in the order they arrived

A stanza waiting on payloads is not a licence to reorder the ones behind it. An
ack emitted the moment it arrives would overtake a message that came first, and
a recording compared position by position reports that as a divergence in
whichever engine happened to be slower.

All four adapters do this. Two did not until rev 42 — `zapo` and
`whatsapp-rust` emitted an unheld stanza the moment it arrived — and a test
reads all four joiners to keep them agreeing.

### One thing this adapter does not have to do

Work out which node a payload belongs to. Baileys reports the child index
directly — a decision taken when the hook was added, having seen the other
adapters resolve an `<enc>`-relative index and get it wrong for a fan-out
`<message>`, where the copies for this device are numbered apart from the direct
children.

## Building

```console
cd adapters/baileys
npm install
npm test
```

Expects a `Baileys` checkout beside `wa-wire`, built (`pnpm build`) so the
observation points are in its `lib/`. The engine is a `file:` dependency rather
than a version: the hooks are local changes and nowhere published.
