# wa-wire — Design Document

> **Status:** **IMPLEMENTING** — ten RFCs accepted. Five of the six items in
> the definition of done are closed; publishing `wa-wire-contract` is the one
> that is not. Four engines are measured agreeing on derived events (rev 43),
> over six pairwise comparisons of one corpus. Every recording holds one half of a
> session: the inbound one. An engine can now report the other (rev 31).
> **Name:** `wa-wire` (D-018) · **License:** MIT, `adapters/hypermeow/` MPL-2.0 (D-022)
> **v1 scope:** L0 + L1, takeover included. No L2, no Layer 3 host.
> **Owner:** oxidezap
> **Last revised:** rev 61

This document is **incremental**. Every revision appends to the
[Changelog](#changelog) and the [Decision Log](#decision-log). Claims backed by
code carry a `file:line` reference so they can be checked, not trusted.

Confidence markers used throughout:

- **[VERIFIED]** — read in the source tree, reference given.
- **[INFERRED]** — reasoned from verified facts, not directly observed.
- **[UNKNOWN]** — open, needs work before it can be relied on.

---

## Table of contents

- [1. Problem](#1-problem)
- [2. Goals and non-goals](#2-goals-and-non-goals)
- [3. Findings](#3-findings)
- [4. Architecture](#4-architecture)
- [RFC-001 — Layer model (L0/L1/L2)](#rfc-001--layer-model-l0l1l2)
- [RFC-002 — Capability matrix](#rfc-002--capability-matrix)
- [RFC-003 — Session handoff protocol](#rfc-003--session-handoff-protocol)
- [RFC-004 — Multi-session host and resource sharing](#rfc-004--multi-session-host-and-resource-sharing)
- [RFC-005 — Conformance](#rfc-005--conformance)
- [RFC-006 — Store ownership](#rfc-006--store-ownership)
- [RFC-007 — Language and repository strategy](#rfc-007--language-and-repository-strategy)
- [RFC-008 — Boundary wire format](#rfc-008--boundary-wire-format)
- [RFC-009 — Contract versioning and provenance](#rfc-009--contract-versioning-and-provenance)
- [RFC-010 — Recording container](#rfc-010--recording-container)
- [RFC-005 amendment — comparison profiles](#rfc-005-amendment--comparison-profiles)
- [5. What this unlocks](#5-what-this-unlocks)
- [6. Risks and honest limitations](#6-risks-and-honest-limitations)
- [7. Open questions](#7-open-questions)
- [8. Implementation plan](#8-implementation-plan)
- [Decision log](#decision-log)
- [Changelog](#changelog)

---

## 1. Problem

Several mature WhatsApp Web client libraries exist: `Baileys`, `whatsmeow`
(and the `hypermeow` fork), `zapo`, `whatsapp-rust`, `baileyrs`. They all speak
the **same wire protocol** — same binary-node encoding, same `WAProto`
protobuf schemas, same stanza semantics — because they all talk to the same
server.

They expose **completely different APIs**.

The practical consequence, in the words of the conversation that started this:

> *"os cara já têm APIs, mudar em sistema é um pesadelo"*

A production system built on one library is structurally married to it. If the
library breaks, stalls, gets abandoned, or starts behaving badly after a
WhatsApp-side change, migrating means rewriting the integration layer — even
though the protocol underneath never changed.

The existing answer in this ecosystem is `multi-wa-api`: a REST API with two
swappable engines. That works, but it solves the problem at the **application**
layer by inventing a high-level abstraction. The cost of that approach is
well known: the abstraction is opinionated, always lags the libraries, and
becomes an `N+1`th API to maintain.

**The insight this project is built on:** the common denominator does not have
to be invented. It already exists, and it is the wire format itself.

### 1.1 The ecosystem already has two of the three IRs

| Project | Intermediate representation | Domain |
| --- | --- | --- |
| `oxidezap/whatspec` | protocol surface IR (`iq`, `proto`, `mex`, `appstate`, `abprops`, `enums`, `notif`, `tokens`) | **spec** — static |
| `vinikjkkj/wa-store-migrate` | `WaSnapshot` IR, 5 libraries, 20 migration routes, loss reporting | **state** — at rest |
| *(missing)* | — | **runtime** — in flight |

There is an IR for what the protocol *is*, and an IR for what a session *holds*.
There is none for what a session *does* while it is alive. That is the gap.

---

## 2. Goals and non-goals

### 2.1 Goals

- **G1** — Define a canonical, engine-neutral representation of WhatsApp runtime
  traffic (in and out), derived from `whatspec` rather than invented.
- **G2** — Make an integration written against the contract run on any conforming
  engine without source changes.
- **G3** — Near-zero overhead when idle. Nothing is materialized that nobody
  subscribed to.
- **G4** — Safe, atomic session handoff between engines, with declared and
  bounded loss.
- **G5** — Multi-session hosting in a single process with explicit,
  correctness-preserving resource sharing.
- **G6** — Adoptable by third parties one layer at a time. No all-or-nothing
  daemon.

### 2.2 Non-goals

- **NG1** — Not a high-level convenience API. That is `multi-wa-api`'s job, and
  it can be rebuilt on top of this.
- **NG2** — Not a ban-avoidance or detection-evasion tool. Orthogonal concern;
  the project neither helps nor hurts there.
- **NG3** — Not a replacement for any engine. Engines stay independent and
  competitive; this describes what they already have in common.
- **NG4** — Not an attempt to make every engine equivalent. Divergence is
  **reported** through capabilities, never papered over.

---

## 3. Findings

Everything below was read in local checkouts on 2026-08-07.

### 3.1 `whatsapp-rust` already implements most of the proposed contract

Line references are as of rev 31. This engine moves; where a claim depends on
*where* something is rather than *that* it exists, the file is named so it can
be re-checked rather than trusted.

| Feature | Location | Note |
| --- | --- | --- |
| `Event::RawNode` | `src/client/node_io.rs:490` | dispatched **before any early return**, so IQ responses and `xmlstreamend` are included |
| `Event::SentFrame` | `wacore/src/types/events.rs` | the outbound counterpart, added in upstream #1260 — see below |
| `RawNodeLease` | `src/client.rs:93` | atomic refcount; forwarding disables when the last lease drops |
| Idle cost avoidance | `src/client/node_io.rs` | an `ack` skips even the `Arc::new` when nothing observes |
| `send_node()` | `src/client/messaging.rs:116` | L0 out |
| `wait_for_node(NodeFilter)` | `src/client.rs` | raw request/response, zero-cost with no waiters |
| `StanzaInterceptor` | `src/client/interceptor.rs` | takeover, added in upstream #1239 — see below |
| Plugin host | `src/plugins/mod.rs` | capability bitflags (`CoreEvents`/`Tasks`/`Messaging`/`Iq`/`PluginEvents`), install/callback/drain timeouts, lease acquired from *declared interest* |
| ~60 typed event kinds | `wacore/src/types/events.rs:216` | close to a ready-made L1 vocabulary |
| Runtime abstraction | `wacore/src/runtime.rs` | `Runtime` trait, `Send` dropped on wasm32 |

**[VERIFIED, reversed in rev 31] Takeover exists.** Revisions 1-30 said it did
not: `Event::RawNode` is purely observational, the native pipeline runs
afterwards regardless, and `StanzaRouter::register` **panics** on duplicate tag
registration (`src/handlers/router.rs:30-35`), so a built-in handler could not
be replaced.

The panic is still there. It stopped being the obstacle. `StanzaInterceptor`
(upstream #1239) runs where dispatch would have and either steps aside or claims
the stanza, skipping the built-in handler without going through the router.
Claiming also turns the `<nack>` the client owed into an `<ack>`, since somebody
did handle it.

Five things are never offered: `success`, `failure`, `stream:error`, `ack`, and
a server-initiated `<iq>` ping. See
[Delivery modes](#delivery-modes-per-subscription-per-layer) for why that
exclusion is structural rather than an oversight.

**[VERIFIED, rev 31] The send side is observable.** `Event::SentFrame` carries
one marshaled stanza exactly as it was handed to the Noise encryption, emitted
once the transport accepted the write — the single point every send crosses.
Frames that failed to encrypt or to write never appear, and neither do pre-Noise
handshake frames.

It is leased like `RawNode` (`acquire_sent_frame_forwarding()`), so nothing is
cloned while nothing is listening. Before it, the only thing watching outbound
traffic was a filtered one-shot waiter for a single expected stanza, and the
paths that never build a `Node` at all — acks, delivery receipts, direct-encoded
IQs — were invisible even to that.

This matters here more than its size suggests: **it is the first engine-side
support for recording both halves of a session**, which is what
[RFC-010](#rfc-010--recording-container) would need in order to compare a
candidate's *replies* rather than only what it was told.

**[VERIFIED] Observation is not free and not neutral.** Enabling raw forwarding
changes scheduling: `processes_inline()` returns `false` for `receipt` and `ack`
once forwarding is on (`node_io.rs:648`), moving them off the read loop into
spawned tasks. Observing therefore alters the execution path. This must be
stated in the contract.

### 3.2 `zapo` has both a plugin system and a low-level API

**Plugin system** — `src/client/plugins/define.ts`, exported publicly at
`src/index.ts:2`:

```ts
defineWaClientPlugin({ id, exposeAs?, setup(ctx), dispose? })
```

The context (`src/client/plugins/types.ts`) carries `client`, `options`,
`logger`, `stores` (`WaStore['session']`), the full `deps: WaClientDependencies`
coordinator graph, `emit`/`on`/`off`/`once`, `queryWithContext`,
`registerIncomingHandler`, `registerIncomingStanzaFilter`, and
`registerDispose` — the latter documented as running *"on `WaClient.disconnect`
after incoming handlers drain"*.

Typing is by tuple inference (`WaClientExposedFromPlugins`) with plugin-declared
events carried on a `__pluginEvents` phantom marker. No global augmentation: a
`voip_*` event only exists when the voip plugin is installed.

**Low-level API** — `WaLowLevelCoordinator` (`src/client/coordinators/WaLowLevelCoordinator.ts:14-42`),
reachable as `client.lowlevel`:

```
sendNode(node)
query(node, timeoutMs?, opts?)
registerIncomingHandler({ tag, subtype?, handler, prepend? })
unregisterIncomingHandler(reg)
registerIncomingStanzaFilter(node => boolean | Promise<boolean>)
```

**[VERIFIED] `registerIncomingStanzaFilter` enables true takeover.** Implementation
at `src/client/coordinators/WaIncomingNodeCoordinator.ts:189-217`:

- runs against **every** inbound stanza before any handler;
- filters are awaited **strictly in series** in an index loop (`:194-197`) — no
  reordering hazard;
- a filter that throws is logged and skipped, not fatal (`:198-203`);
- returning `true` **drops** the stanza and still emits the correct ack via
  `buildInboundAck` + `sendSafeAck` (`:210-214`), so the server stops
  redelivering.

So `zapo` supports both **tap** (`return false`) and **takeover**
(`return true`, engine becomes pure Noise transport + acks) **today, with no
fork**.

**[VERIFIED] Protected tags are narrower than documented.** The doc comment
mentions "stream-control nodes and the connection-critical success/failure
tags", but the actual set is `FILTER_PROTECTED_TAGS = { success, failure }`
(`WaIncomingNodeCoordinator.ts:126`). L0 coverage of the auth/stream phase is
therefore incomplete in `zapo`, unlike `whatsapp-rust`.

**[VERIFIED] `zapo` has zero hard runtime dependencies.** `dependencies` is
absent from `package.json`; `argo-codec`, `pino`, `pino-pretty` and `ws` are
**peerDependencies** only. `WaWebSocket.ts:67-70` resolves
`globalThis.WebSocket` first, falling back to an optional dynamic import of
`ws`. Granular subpath exports (`./client`, `./auth`, `./crypto`, `./media`,
`./message`, `./appstate`). This makes `zapo` the most runtime-portable engine
of the set — relevant to Bun/Deno/browser/worker targets.

**[VERIFIED] `zapo` carries its own spec directory** — `spec/{abprops,appstate,mex,proto,version}`,
re-exported through `src/*-spec.ts`. Same domain decomposition as `whatspec`,
independently arrived at.

### 3.3 `Baileys` already has a catch-all frame hook

**[VERIFIED]** `ws.emit('frame', frame)` at
`packages/baileys/src/Socket/socket.ts:749`, inside `onMessageReceived`'s
`noise.decodeFrame` callback — fired for **every** frame, before any tag-based
dispatch, including non-binary-node `Uint8Array` frames (handshake phase). It is
already consumed internally at `socket.ts:539`.

So `ws.on('frame', …)` gives L0 in with **no patch**, and with *better*
coverage than `zapo` (handshake frames included). This corrects an earlier
assumption in this design that Baileys would need a patch.

Also verified: `sendNode` (`socket.ts:163`), `query` (`socket.ts:260`),
tag-scoped `CB:*` handlers (`socket.ts:1026-1208`), `connectionReplaced = 440`
(`src/Types/index.ts:31`). No takeover mechanism, no plugin system, no drain
hook.

**[VERIFIED] The oxidezap Baileys fork already depends on
`whatsapp-rust-bridge`** (`packages/baileys/package.json`, `workspace:^`).
Convergence at the core layer has already started independently of this project.

### 3.4 `hypermeow` needs less patching than this document assumed

**[VERIFIED, revised in rev 31]** The estimate below was written against
upstream `whatsmeow`. The `hypermeow` fork has moved since, and most of what
this section called for already exists there.

`hypermeow`'s `main` has moved again since rev 26 — `events.UndecryptedMessage`
now dispatches from the receive goroutine, and the privacy cache was fixed —
but nothing there touches what this section claims.

**What is already in the fork:**

- `RawNodeHandler` (`client.go:888`, fired inside `handleFrame`) runs for every
  inbound stanza after Noise decryption and binary decoding, before dispatch.
  It returns `(modified, drop)`, so it is not only a tap: dropping is
  **takeover, natively**, with no patch at all.
- `handleFrame` is the Noise layer's own frame callback (`handshake.go:126`),
  so the hook sees `<success>` and `<failure>` too. That is
  `l0.inbound.auth-phase`, which neither existing adapter has *alongside*
  takeover.
- `DisabledFeatures.Signal` lets an external system own the Signal session,
  with `events.UndecryptedMessage` carrying the envelope verbatim.
- L0 out is still `DangerousInternals().SendNode` (`internals.go:170`).

**What is still missing on `main`, and is proposed in
[polymorfa/hypermeow#5](https://github.com/polymorfa/hypermeow/pull/5) — open
as of rev 31, so this is a branch and not yet a fact about the fork:**

- **The frame bytes.** `RawNodeHandler` received the decoded node only, so an
  adapter had to re-encode. `handleFrame` had the decompressed buffer in scope
  and let it fall out right after `Unmarshal`; the PR hands it over. This is
  `l0.zero-copy-frame` for the cost of one argument.
- **The plaintexts.** Nothing carried the per-`<enc>` plaintext, the same gap
  `whatsapp-rust` had before #1240. The PR adds `DecryptedPayloadHandler`,
  firing before the protobuf unmarshal, because a payload that fails to
  unmarshal was being dropped behind a warning after the ratchet had already
  advanced.

  Review found the same defect one layer deeper: `unpadMessage` runs inside
  `decryptDM`/`decryptGroupMsg`, *after* the ratchet, so a padding failure lost
  the plaintext too — the exact loss the hook exists to prevent. Both now return
  the raw output alongside the unpad error. `RawNode` is also passed by value
  rather than by pointer, which a benchmark showed cost one heap allocation per
  stanza.

So the capability shape of a `hypermeow` adapter, with that PR, would be tap,
auth phase, takeover, zero-copy, plaintext and outbound sending. Without it, tap
and takeover only, L0-wire and re-encoded.

Rev 26 called that "the widest of the three". That is no longer true: it cannot
observe what it sends, which `whatsapp-rust` gained in upstream #1260. The
widest reading of the engines is now a moving target rather than a ranking, and
[RFC-002](#rfc-002--capability-matrix) is where it is stated per capability
instead of per engine.

Still absent either way: no plugin system, no drain hook, and the node dispatch
table stays private (`cli.nodeHandlers`).

### 3.5 One connection per device — verified in three engines

**[VERIFIED]** WhatsApp terminates the previous connection when a second one
authenticates with the same keys:

- `whatsmeow`: `<conflict type="replaced"/>` → `events.StreamReplaced`
  (`connectionevents.go:48-51`); also treated as terminal in `request.go:35-37`.
  Doc: *"emitted when the client is disconnected by another client connecting
  with the same keys"* (`types/events/events.go:138`).
- `whatsapp-rust`: `Event::StreamReplaced` (`wacore/src/types/events.rs:263, 943`),
  documented for `<conflict>`, 516 device removal, and 401.
- `Baileys`: `connectionReplaced = 440` (`src/Types/index.ts:31`).

**This is the single most design-defining fact in the document.** Blue/green
handoff with connection overlap is physically impossible. See
[RFC-003](#rfc-003--session-handoff-protocol).

### 3.6 Summary of the surprise

Two engines independently converged on the same architecture — plugin host,
lifecycle with drain, interest-driven raw node access, raw send, raw
request/response — without coordination. The contract this project needs to
define is, to a large extent, **already written twice**. What is missing is the
name, the neutral spec, and the conformance proof.

But **no single engine has all of it** — though `whatsapp-rust` came close
during rev 31. It now has takeover as well (`StanzaInterceptor`) and is the only
engine that can observe what it *sends* (`Event::SentFrame`); what it still
lacks is nothing this document has found. `zapo` has takeover but does not cover
the auth phase and cannot observe outbound frames. `Baileys` has the broadest
raw inbound coverage and neither a plugin system nor takeover. `hypermeow` with
PR #5 covers inbound, auth phase, takeover, zero-copy and plaintexts, and has no
plugin host, no drain hook and no outbound observation.

**The conclusion this section drew still holds, for a changed reason.** It used
to hold because every engine was missing something structural. It now holds
because the engines are converging on the same surface at different speeds, and
a contract pinned to whichever is furthest ahead this month is a contract that
excludes the rest. Two of the three gaps closed in the two days before rev 31.

---

## 4. Architecture

Three layers. Each is independently useful and independently adoptable. This is
the core of [G6](#21-goals).

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 3 — HOST (optional)                                   │
│ multi-session supervisor · resource pooling · hot-swap      │
│ fencing · canary routing                                    │
└─────────────────────────────────────────────────────────────┘
                            ▲ consumes the contract
┌─────────────────────────────────────────────────────────────┐
│ Layer 2 — ADAPTERS (one per engine)                         │
│ zapo: plugin, no patch     whatsapp-rust: plugin, no patch  │
│ Baileys: ws.on('frame')    hypermeow: hook, no patch        │
└─────────────────────────────────────────────────────────────┘
                            ▲ implements
┌─────────────────────────────────────────────────────────────┐
│ Layer 1 — CONTRACT (`wa-wire`)                              │
│ L0/L1/L2 spec · capability matrix · conformance suite       │
│ generated from whatspec                                     │
└─────────────────────────────────────────────────────────────┘
```

**Key architectural decision (see [D-002](#decision-log)):** `wa-wire` is
primarily a **plugin**, not a host. Adoption is one line
(`.use(waWire())` / `client.plugin(WaWire::new())`), it runs in-process with no
IPC boundary in the common case, and the supervisor is opt-in.

An earlier draft of this document had it backwards — a daemon that *controls*
engines. That was rejected: it imposes an IPC cost on everyone, it demands
all-or-nothing adoption, and it makes third-party engine support impossible
without the daemon's blessing.

---

## RFC-001 — Layer model (L0/L1/L2)

**Status:** **ACCEPTED** (rev 7)

### Normativity

> **L0 is normative. L1 is a derived view. L2 is a convenience surface.**

**Hard rule:** no information may appear in L1 that is not derivable from
`L0 + session state`. This is what makes the layers non-competing and makes
conformance mechanically checkable — given identical L0 input and identical
session state, every conforming engine must produce byte-identical L1.
Divergence is a bug, not a difference of opinion between libraries.

This resolves the false dichotomy between "raw" and "typed": they are the same
object in two projections.

### L0 — raw stanza

- **Payload:** binary node, post-Noise, pre-semantics.
- **Direction:** in and out.
- **Source of truth:** the wire.
- **Escape hatch:** anything the contract has not modeled yet is still reachable
  here. This is what prevents the contract from becoming a cage — the failure
  mode that kills every abstraction layer in this space.

### L1 — canonical events

- **Payload:** decrypted `waE2E.Message` / `WebMessageInfo` plus typed
  receipts, notifications, presence, app-state mutations.
- **Derivation:** generated from `whatspec` (`notif`, `enums`, `iq`, `appstate`
  domains), not hand-written.
- **Vocabulary baseline:** `whatsapp-rust`'s `EventKind`
  (`wacore/src/types/events.rs:216-276`) is the most complete existing candidate.

**Two honest caveats that must be in the design from day one:**

1. **L1 is not a pure function of a single stanza.** Decryption depends on the
   Signal ratchet; app-state depends on accumulated patches; polls and reactions
   depend on message secrets. The real signature is
   `(stanza, session_state) → L1`. Deterministic replay of L1 therefore requires
   capturing state, not just wire traffic. This directly constrains
   [RFC-005](#rfc-005--conformance).
2. **Media does not traverse the socket.** Upload/download is HTTP against
   MMS/CDN. It belongs to neither L0 nor L1 — it is a separate domain with its
   own capability.

### L2 — commands

- **Payload:** `send`, `edit`, `react`, `revoke`, `presence`, group operations,
  media transfer.
- **Status:** where 70% of the work and 100% of the semantic arguments live.
- **[OQ-4](#oq-4--l2-in-v1--resolved-in-rev-6)** — in or out of v1. Unresolved.

### Delivery modes (per subscription, per layer)

| Mode | Meaning | Engine support |
| --- | --- | --- |
| `tap` | observe; engine keeps processing natively | all four |
| `takeover` | suppress native processing; engine becomes transport + acks | `zapo`, `whatsapp-rust`, `hypermeow` |

`takeover` is what makes an engine genuinely interchangeable — under takeover,
engine-specific semantics stop mattering because the engine stops interpreting.

**It no longer requires patching `whatsapp-rust`** (revised in rev 31). This
document said it would, because `StanzaRouter::register` panics on a duplicate
tag (`src/handlers/router.rs:30-35`) and there was no other way in. The panic is
still there; it stopped being the obstacle. `StanzaInterceptor` (upstream #1239,
`src/client/interceptor.rs`) runs where dispatch would have and either steps
aside or claims the stanza, skipping the built-in handler without touching the
router at all.

Four tags are **never offered** to an interceptor — `success`, `failure`,
`stream:error`, `ack` — and neither is a server-initiated `<iq>` ping. Each
settles connection state: authentication, shutdown, reconnection, and the
waiters a send blocks on. A claimed ping is a pong never sent, and the server
drops the connection over it.

That exclusion is the same line [RFC-003](#rfc-003--session-handoff-protocol)
draws, arrived at independently, and it is why takeover here is **partial by
construction rather than by omission**. An engine cannot hand over the stanzas
that keep it connected and still be the transport. So the capability is not
"takeover" but "takeover of everything except what the connection is made of",
and D-103 records that the matrix must say which.

### Cost disclosure (normative)

An adapter **must** declare when enabling a subscription changes engine
behavior beyond adding a callback. Precedent:
`whatsapp-rust`'s `processes_inline()` reroutes `receipt`/`ack` off the read
loop once raw forwarding is enabled (`node_io.rs:648`). Silent scheduling
changes under observation are a conformance violation.

---

## RFC-002 — Capability matrix

**Status:** **ACCEPTED** (rev 7)

Following the cultural precedent already set by `wa-store-migrate`'s loss
reports and `whatspec`'s `dropsByReason`: **what cannot be done is declared
explicitly, never silently degraded.**

### Current state (verified 2026-08-08)

Two engines are read at an open PR rather than at a released version.
`hypermeow` at [#5](https://github.com/polymorfa/hypermeow/pull/5): its `main`
has the raw-node hook (upstream #3) but neither the frame bytes nor the
plaintexts. `Baileys` at
[WhiskeySockets/Baileys#2762](https://github.com/WhiskeySockets/Baileys/pull/2762):
its `develop` has neither. Rows marked with a PR number are unavailable
without it.

Line numbers into an engine are checked by `tools/check-docs.py` against
whatever is checked out beside this repository, which is how the `Unmarshal`
citation below was found pointing sixty lines short.

| Capability | whatsapp-rust | zapo | Baileys | hypermeow |
| --- | --- | --- | --- | --- |
| L0 in, catch-all | ✅ `Event::RawNode` | ✅ stanza filter | ✅ `ws.on('frame')` | ✅ `RawNodeHandler` |
| L0 in covers auth/stream phase | ✅ | ❌ `success`/`failure` protected | ✅ incl. `Uint8Array` frames | ✅ hook is the Noise frame callback |
| L0 takeover | ⚠️ `StanzaInterceptor`, minus the five below | ✅ filter → `true` + auto-ack | ❌ observation only | ⚠️ `drop` return, whole-stanza |
| L0 **out**, observed | ✅ `Event::SentFrame` (leased) | ❌ | ❌ | ❌ |
| L0 out, sent | ✅ `send_node` | ✅ `sendNode` | ✅ `sendNode` | ⚠️ `DangerousInternals` |
| Raw request/response | ✅ `wait_for_node` | ✅ `query` | ✅ `query` | ⚠️ `DangerousInternals` |
| Per-`<enc>` plaintext | ✅ upstream #1240 | ❌ | ✅ *(PR #2762)* | ✅ *(PR #5)* |
| Why a payload is missing | ❌ | ❌ | ❌ | ❌ |
| Plugin host | ✅ capability bitflags | ✅ tuple-typed | ❌ | ❌ |
| Drain hook | ✅ `task_drain_timeout` | ✅ `registerDispose` | ❌ | ❌ |
| Zero-copy frame bytes | ✅ **already retained** — `Yoke<NodeRef, BytesCart>` + `backing_bytes()` | one-line patch at `decoder.ts:344` | ✅ *(PR #2762)* | ✅ *(PR #5)* |
| Runtime portability | native + wasm32 | node/bun/deno/browser | node (hard `ws`) | native |

**Two rows have no capability identifier, deliberately.** *Plugin host* is how
an adapter installs and *runtime portability* is where its engine runs: both
matter to whoever is choosing an engine and neither is something a consumer can
require of the boundary at setup. A capability is a promise about what crosses;
these are facts about what is on the other side of it.

**That row is no longer `❌` everywhere.** `whatsapp-rust` provides it as of
rev 55, through `Event::EncDecryptFailed` (upstream #1261) — the per-`<enc>`
counterpart of `Event::DecryptedPayload`, numbered by the same enumeration, so
the two events index one stanza and not two. The other three still report
`Unobserved` for everything.

**The original note, kept because the reasoning is what named the capability.** No adapter reports *why* an
`<enc>` produced nothing, so every entry says `Unobserved`. `PlaintextStatus`
has carried `DecryptFailed` and `Unsupported` since the format was written,
which means the format anticipated a distinction the vocabulary did not name.
`l0.plaintext.cause` names it (D-130), before publication rather than after.

**It is not that the engines are silent.** `whatsapp-rust` dispatches
`Event::UndecryptableMessage`, and reading it closely is what settled this
(D-133). It carries the message, `is_unavailable`, an `unavailable_type` and a
`decrypt_fail_mode` — and `decrypt_fail_mode` is the server's `show`/`hide`
display hint, not a reason. Two properties rule it out for this row: it is
dispatched **per message rather than per `<enc>`**, so a fan-out stanza cannot
have its failures attributed; and it is **deduplicated per `(chat, id)` and
suppressed on resends**, so the second time a stanza arrives undecryptable
there is no event at all. An adapter wiring it into a plaintext entry would
produce a `DecryptFailed` that is right sometimes and quietly missing others,
which is worse than the `Unobserved` it reports today — a gate can act on a
status that is always honest and cannot act on one that is usually honest.

What would close it is the symmetric counterpart of `Event::DecryptedPayload`
(upstream #1240): a failure event carrying `enc_index` and a cause, dispatched
from the same loop, which already has `enc_index` in scope. Every failure
branch in that loop needs one, so it is an engine change with its own review
rather than an adapter change.

**Takeover is `⚠️` rather than `✅` where it is partial, and the partiality is
the honest reading, not a shortfall.** `whatsapp-rust` never offers `success`,
`failure`, `stream:error`, `ack`, or a server-initiated `<iq>` ping, because
each settles connection state. An engine that handed those over would stop
being able to stay connected. `zapo`'s filter can claim anything, which is
broader and correspondingly easier to hang the connection with.

**`L0 out, observed` is a distinct row from `L0 out, sent`** (D-102). Sending is
what an adapter does; observing what left is what a *recording* needs, and until
upstream #1260 no engine had it. A recording that captures only the inbound side
holds one half of a conversation — see [RFC-010](#rfc-010--recording-container),
which today records exactly that half.

**The contract names it** as `l0.outbound.observed` (D-102), the ninth of the
identifiers frozen at 0.1.0. The row stays about
engines rather than about us: it records what an engine *could* provide, which
was the input to that decision and not the decision.

**Zero-copy was re-assessed in rev 7** (see RFC-008). The rev 1 entry — "no
engine has it" — was wrong for `whatsapp-rust`: `OwnedNodeRef` is
`Yoke<NodeRef<'static>, BytesCart>` (`wacore/binary/src/node.rs:903`), so the
parsed node already borrows from a retained buffer. Getting the *whole* buffer
back needed one upstream method — `backing_bytes()`
(`wacore/binary/src/node.rs:948`), added in rev 10 — because `slice_bytes()`
takes a slice that already points inside the buffer and so cannot produce the
buffer itself. In the other three the bytes sit in a
local variable at the decode site. This is what made D-016 affordable enough to
put in v1.

### Rules

- A capability is either **present**, **absent**, or **degraded with a stated
  cost**. There is no fourth state.
- Consumers negotiate: a subscription requesting an absent capability fails
  loudly at setup, never silently at runtime.
- The matrix is machine-readable and is the input to
  [RFC-005](#rfc-005--conformance).

---

## RFC-003 — Session handoff protocol

**Status:** **ACCEPTED** (rev 7)
**Depends on:** [finding 3.5](#35-one-connection-per-device--verified-in-three-engines)

### The physical constraint

One device, one connection. A second connection with the same keys causes the
server to kill the first (`conflict type="replaced"`). **Overlapping
blue/green handoff is impossible.** Handoff is necessarily stop-the-world per
session.

### Phases

```
1. quiesce   stop accepting new commands; mark session draining
2. barrier   drain in-flight sends, pending acks, active retries
             (zapo: registerDispose; whatsapp-rust: task_drain_timeout)
3. detach    clean disconnect — MUST NOT be a logout
4. snapshot  export state via the wa-store-migrate IR
5. attach    target engine imports, connects, completes handshake
6. resume    release the queued command backlog
```

### State that must never be split or concurrently written

Each of these is a distinct, permanent corruption mode:

| State | Failure if duplicated |
| --- | --- |
| Signal double-ratchet | every message advances it; two writers → unrecoverable `bad MAC` on that session |
| Pre-keys | one-shot consumption; reuse → peer-side session failure |
| App-state LTHash + version | incremental hash; divergence → rejected patch or expensive full resync |
| Retry counters / message IDs | duplicate or dropped delivery |

### Requirements

- **R1 — Exclusive ownership with a fencing token.** A per-session lock is not
  sufficient across multiple hosts: after a GC pause or network partition two
  hosts can both believe they own a session. A persisted monotonic fencing
  token is required. This is classic distributed-lock discipline and cannot be
  retrofitted cheaply.
- **R2 — Handoff is not lossless for every route.** `wa-store-migrate` already
  documents this: Baileys drops skip message keys; `whatsapp-rust`'s app-state
  encoding loses sub-second precision. "Safe hot-swap" must mean *"loss is
  known, declared, and accepted"* — never *"lossless"*. The 20-route matrix
  carries per-route cost.
- **R3 — Duplicate-safe L1.** The unavailability window is handshake + resync.
  Messages arriving in it stay queued server-side (offline delivery), but
  **in-flight acks can duplicate**. Deduplication by message ID is mandatory in
  L1, not optional.
- **R4 — `detach` must be type-level distinct from `logout`.** A bug here
  unpairs the customer's device. Enforced by types, not by convention.

### Open

**[UNKNOWN]** Measured duration of the unavailability window per engine pair.
Needs an experiment — `whatsapp-bench` already has the pinned-source harness to
run it.

---

## RFC-004 — Multi-session host and resource sharing

**Status:** **ACCEPTED** (rev 7)

### Shareable across sessions in one process

- binary-protocol token tables (static, from `whatspec`)
- protobuf descriptors
- HTTP/TLS connection pool for MMS/CDN
- executor / thread pool / DNS resolver

### Never shareable

- any key material, Signal store, app-state, socket
- device-list and LID↔PN caches are **per-account** — sharing them globally is
  a cross-tenant leak, not an optimization

### Idle-cost principle (normative)

The pattern is already proven in `whatsapp-rust` and becomes a contract
requirement: **interest-driven materialization**. A subscription declares what
it wants; the engine materializes nothing else. No L0 subscriber → no
serialization. No L1 subscriber → no protobuf decode. Reference implementation:
`node_io.rs:325-331`, where an unobserved `ack` avoids even its `Arc`
allocation.

### Isolation unit

**[UNKNOWN]** — [OQ-1](#oq-1--isolation-unit--provisional-process-per-engine). Sessions-as-tasks in one
multi-tenant process is cheap but one engine panic takes down neighbours;
process-per-engine with N sessions inside bounds the blast radius at the cost of
IPC. Current lean: **process per engine, N sessions inside**.

### JS runtime portability

Relevant because engines are not all Rust and the target runtimes differ.

| Runtime | Baileys | zapo | Note |
| --- | --- | --- | --- |
| Node ≥20.9 | ✅ | ✅ | baseline |
| Bun / Deno | likely ✅ | ✅ | zapo prefers `globalThis.WebSocket` (`WaWebSocket.ts:67-70`) |
| Browser / Worker | ❌ hard `ws` dep | plausible | zapo has zero hard deps |
| QuickJS | ❌ | ❌ | no sockets, no native crypto |

QuickJS is only viable if the **host** supplies socket and crypto through
bindings, leaving JS with protocol logic only. That is real, but it drags the
design toward the "fat host" pole — see [§4](#4-architecture) and
[D-002](#decision-log). Classified as **research, not roadmap**.

---

## RFC-005 — Conformance

**Status:** **ACCEPTED** (rev 7)

The property that makes the whole thing more than a wrapper:

> Given identical L0 input and identical session state, every conforming engine
> must produce identical L1 output.

### Method

1. **Capture.** Record L0 + state snapshots from a real session (any engine).
2. **Replay.** Feed the recording to every engine offline, no account needed.
3. **Diff.** Compare L1 output. Any divergence is a bug in exactly one engine —
   and the suite says which, since `whatspec` is the oracle.

This is a strictly stronger tool than what exists today: `whatsapp-bench`
measures *performance* across engines; nothing measures *correctness* across
engines.

### Constraint inherited from RFC-001

Because L1 is `(stanza, session_state) → L1` and not a pure function of the
stanza, recordings must capture state transitions, not just wire traffic. This
makes the recording format a first-class artifact rather than a debug dump.

### Reuse

`whatsapp-bench` already solves source pinning, checksum verification,
hermetic builds, offline execution, and per-engine adapters in three languages.
The conformance runner should reuse that machinery rather than duplicate it.

---

## RFC-006 — Store ownership

**Status:** **ACCEPTED** (rev 7) — recommendation stands; the snapshot measurement refines Layer 3, it does not gate v1
**Resolves:** [OQ-2](#oq-2--store-ownership--resolved-in-rev-7)
**Blocks:** [RFC-003](#rfc-003--session-handoff-protocol) only — not Layers 1 or 2

### Why this is the expensive question

Store ownership simultaneously determines handoff cost and atomicity, how much
patching each engine needs, whether third parties can plug in unaided, and
whether the project stays neutral or collapses into "whatsapp-rust with
facades" (risk R4). A wrong answer here is not fixed by refactoring — it
redefines the product.

### The asymmetry that decides it

Finding 3.6 recorded that **event** models converged independently across
engines. **Store** models did the opposite — they diverged completely.

| Engine | Model | Shape |
| --- | --- | --- |
| Baileys | `SignalKeyStore.get(type, ids)` / `set(dataSet)` over a 10-entry `SignalDataTypeMap`, plus `transactWith(scope)` with deterministically ordered locks (`Types/Auth.ts:74-133`) | **generic typed KV** |
| zapo | 16 domain stores — 11 persistent + 5 cache — with per-domain backends and compile-time refusal to route an unimplemented domain (`src/store/types.ts:21-125`) | **domain-oriented, granular** |
| whatsmeow | ~9 Go interfaces: `IdentityStore`, `SessionStore`, `PreKeyStore`, `SenderKeyStore`, `AppStateSyncKeyStore`, `AppStateStore`, `ContactStore`, `ChatSettingsStore`, `DeviceContainer` (`store/store.go:23-121`) | **domain-oriented** |
| whatsapp-rust | `Backend: SignalStore + AppSyncStore + ProtocolStore + MsgSecretStore + DeviceStore` (`wacore/src/store/traits.rs:936`), with libsignal traits kept separate (`wacore/libsignal/src/protocol/storage/traits.rs`) | **domain + libsignal split** |

### [VERIFIED] The store is not a byte bag

Several store operations carry **logic**, not just persistence:

- `GetOrGenPreKeys(ctx, count)` — *generates* keys (`store/store.go:42`)
- `MarkPreKeysAsUploaded(ctx, upToID)` — range semantics (`:46`)
- `GetManySessions` / `PutManySessions` — batch paths that exist for
  performance (`:33-35`)
- `transactWith(scope)` — Baileys orders locks by sorted `RecordRef` to make
  overlapping transactions deadlock-free (`Types/Auth.ts:119-129`)
- zapo separates *persistent* from *cache* domains with different teardown
  semantics (`src/store/types.ts:38-60`)

**Consequence:** a host that owns the store must decide who generates pre-keys,
in which ID range, under which transaction. At that point it is no longer
abstracting persistence — it is reimplementing four engines' session logic.
This is what makes option C below a trap rather than a trade-off.

### Options, in recommended order

#### Option A — Native snapshot contract, capability-negotiated **(recommended)**

The host does not own the store. The contract instead requires the engine to
**export and import the `WaSnapshot` IR natively**, from the inside, with access
to its own stores.

- Work: moderate per engine, incremental, optional.
- Handoff: fast — no cross-language translation, nobody reading another
  engine's database from outside.
- Neutrality: preserved. The engine implements what it already knows how to do
  (read its own stores), not an imposed interface.
- Fidelity is declared per engine: `full` (lossless round-trip), `lossy` (with
  the loss named), or `none` (falls back to Option B). Same culture as
  `wa-store-migrate`'s loss reports and D-004.

This is the correct form of "thin host with a fast path": the host asks for an
**artifact**, it does not control the **mechanism**.

#### Option B — Engine owns, external export/import via `wa-store-migrate` **(ship this first)**

What already works today, with zero engine changes: 5 libraries, 20 routes,
loss reporting.

- Work: none on engines.
- Handoff: more expensive (read the store externally, translate, write) and
  lossy per the documented matrix.
- Neutrality: maximum.
- Limits: JS on the critical path ([OQ-3](#oq-3--wa-store-migrate-port--decided-technically-open-on-governance)),
  and the translator must track each engine's internal schema, which changes
  without notice.

**Plan of record:** ship Layer 3 v1 on Option B, while designing the Option A
contract from the start. They are not alternatives — Option A is precisely
"the engine implements the same IR from the inside".

#### Option C — Host owns the store **(rejected unless measurement forces it)**

Requires deep patches in every engine, discards each engine's batch
optimizations, and runs into the unanswerable question of who generates
pre-keys. In practice only oxidezap's own forks would comply, which is exactly
risk R4.

#### Option D — Shared physical backend, namespaced per engine **(rejected)**

Sharing the disk does not share the semantics. Internal formats diverge.
Recorded only so it stays explicitly discarded.

### Sequencing — this does not block the start

Layers 1 and 2 (contract, adapters, conformance) touch **no store at all**.
Only Layer 3 does. So the project can start while this stays open, provided the
contract is not painted into a corner.

### [UNKNOWN] The measurement that settles it

Snapshot size and duration for a **mature** real session — many Signal
sessions, populated app-state, months of use. The local `whatsapp.db` in the
`whatsapp-rust` checkout is 0 bytes, so no real figure is available yet.

- ~50 ms → Option B suffices; Option A is polish; Option C is pure
  overengineering.
- ~8 s → the unavailability window becomes unacceptable and Option A becomes a
  requirement.

`whatsapp-bench` already provides the pinned-source, hermetic, offline harness
needed to run this.

---

## RFC-007 — Language and repository strategy

**Status:** **ACCEPTED** (rev 7)
**Refines:** [RFC-001](#rfc-001--layer-model-l0l1l2) (adds the L0-wire / L0-plain split)

### Decision

Rust for the core: contract, codec, L1 derivation, snapshot IR, host,
conformance runner. Cargo workspace monorepo. Rationale is the same as G3 —
predictable low overhead, no GC in the stanza path, and `wacore` already
compiles to `wasm32`, which is how JS adapters will consume the core.

### The constraint that shapes it

Adapters run **inside** the engines, so Rust does not reach uniformly:

| Engine | Adapter language | Rust reach |
| --- | --- | --- |
| whatsapp-rust | Rust crate | native |
| zapo, Baileys | TypeScript plugin | via WASM — `whatsapp-rust-bridge` already ships this, and the oxidezap Baileys fork already depends on it (finding 3.3) |
| hypermeow / whatsmeow | Go hook | **none** — Rust in Go means cgo, and cgo in the per-stanza hot path defeats G3 |

"Written in Rust" is therefore true of the core and false of the Go adapter.
This is not a compromise; it forces a refinement that improves the design.

### L0-wire / L0-plain

If the Go adapter had to derive L1, there would be N diverging L1
implementations — the exact problem this project exists to remove. The split:

| Sub-layer | Content | Produced by |
| --- | --- | --- |
| **L0-wire** | the stanza as received, payload still encrypted | engine |
| **L0-plain** | the stanza plus the plaintext the engine decrypted | engine |
| **L1** | typed canonical events | **host, in Rust, once** |

`L0-plain → L1` is a **pure** function — protobuf parse and mapping, no keys,
no ratchet, no accumulated state. It therefore runs in the host and needs no
per-language reimplementation.

> **Both halves exist as of rev 27, and both are generated.** The stanza half
> comes from whatspec's `incoming` domain, the payload half from its
> `WAProto.proto` (D-093), and provenance carries a digest for each because the
> two can move apart (D-095). What stays hand-written is the *rules*: which
> variants are worth naming, what unwrapping means, which field of a variant is
> its text. None of that is in a schema. The payload half is deliberately
> partial and total anyway: every variant it does not model crosses by number
> rather than being dropped.

This resolves the caveat recorded in RFC-001 §L1 ("L1 is not a pure function of
a single stanza"): it is not pure over **L0-wire**, but it *is* pure over
**L0-plain**. The state dependency lives entirely on the engine side of the
boundary, which is where the keys already are.

Responsibility splits on the correct line:

> The engine does what only it can do — Noise, Signal, app-state.
> The host does what can be standardized — parse, mapping, typing.

Corollary: adapters should be **deliberately dumb**. Emit L0-plain and stop.
A dumb adapter has nothing to diverge and little to break, which also
mitigates R1.

### Workspace layout (proposed)

```
wa-wire/
├── Cargo.toml                      # workspace root
├── crates/
│   ├── wa-wire-contract/           # L0/L1 types, capability matrix — no I/O, no deps
│   ├── wa-wire-codec/              # binary-node codec, tokens from whatspec
│   ├── wa-wire-l1/                 # L0-plain → L1 derivation, generated from whatspec
│   ├── wa-wire-snapshot/           # WaSnapshot IR core (port, see below)
│   ├── wa-wire-snapshot-baileys/   # per-engine store schema adapters
│   ├── wa-wire-snapshot-zapo/
│   ├── wa-wire-snapshot-whatsmeow/
│   ├── wa-wire-snapshot-warust/
│   ├── wa-wire-host/               # Layer 3: supervisor, fencing, handoff, pooling
│   ├── wa-wire-conformance/        # replay + diff runner
│   └── wa-wire-wasm/               # wasm-bindgen surface consumed by JS adapters
├── adapters/
│   ├── whatsapp-rust/              # Rust plugin
│   ├── zapo/                       # TS plugin (thin, over wa-wire-wasm)
│   ├── baileys/                    # TS plugin (thin, over wa-wire-wasm)
│   └── hypermeow/                  # Go hook + adapter (pure Go)
├── spec/                           # language-neutral IR + JSON Schema
└── DESIGN.md
```

**Dependency rule:** `wa-wire-contract` depends on nothing. Adapters depend only
on the contract (plus the WASM surface for JS). `wa-wire-host` may depend on
everything. Any edge that violates this is a design bug — it means engine
specifics leaked into the contract (risk R4).

### FFI binding choice

**Recommendation on record; one measurement outstanding.**

The local `wasm-ffi-bench` project already measured this properly — identical
compilation flags, identical `wasm-opt`, isolated processes, auto-calibrated
iterations, checksum-verified correctness. Its numbers are used directly here.

#### Two distinct boundaries

| | Direction | Analogous benchmark | What it costs |
| --- | --- | --- | --- |
| **A — ingress** | JS → WASM: adapter hands a `BinaryNode` to the core | `step_particles` | `wbgen-flat` 61 µs vs `wasm-bindgen`+serde 991 µs — **16x** |
| **B — egress** | WASM → JS: each L1 event reaches the consumer | scenario 2, string op | `sledgehammer` 29 ns vs `wasm-bindgen` 217 ns — **7x** |

Boundary B matters more than it first appears: L1 events are string-dense
(JIDs, message IDs, push names, message bodies) and WhatsApp delivers in
batches — offline sync flushes hundreds at once.

#### Decisions

1. **Base: `wasm-bindgen` without serde, flat marshalling** — the `wbgen-flat`
   engine, with glue emitted by the `synth` proc-macro rather than hand-written.
   It wins 4 of 5 scenario-1 workloads while simultaneously being the smallest
   (21.9 KB shipped total, 8x under boltffi), the lowest RSS (62 MB), and the
   only one whose linear memory does **not** grow.
2. **`boltffi` rejected for this application** — not on merit generally, but on
   two properties that are disqualifying here: a 128 KB npm runtime dependency
   (unacceptable for a library third parties install into their own Baileys),
   and linear memory growing to 20.7 MB vs 2.6 MB (this process runs for months
   with N sessions; growing linmem is the worst possible trait for that
   profile). It does win string marshalling (`count_words`, ~38%) — not our
   payload shape.
3. **Batch egress, sledgehammer-style** — as a *pattern*, not a dependency.
   Emitting L1 events one-per-call is the 217 ns/op path; batching the flush is
   the 29 ns/op path. WhatsApp already delivers in batches, so batching is
   natural rather than imposed.
4. **`napi` out of the WASM path**, but legitimate for a native Node addon.
   If `whatsapp-rust-bridge` already covers that axis, do not duplicate it.

#### Measured `BinaryNode` shape — real capture

No `wasm-ffi-bench` workload is recursive with variable depth, which is what a
`BinaryNode` is. Rather than guess, shape was measured from
`whatsapp-rust/docs/real-whatsapp-log.json` (6 MB capture of a real session);
214 inbound/outbound stanzas parsed.

Node definition for reference (`wacore/binary/src/node.rs:640-651`):
`Node { tag, attrs: Attrs, content: Option<Bytes | String | Nodes> }` — the JS
side is structurally identical.

| | nodes | attrs | depth | XML bytes |
| --- | --- | --- | --- | --- |
| **median** | 2 | 5 | 2 | 122 |
| mean | 59.6 | 80.9 | 2.5 | 4 877 |
| **p90** | 13 | 15 | 4 | 667 |
| **max** | **4 528** | **9 457** | **9** | **433 KB** |

Per tag:

| tag | n | nodes (med) | attrs (med) | nodes (max) | max depth |
| --- | --- | --- | --- | --- | --- |
| `iq` | 77 | 2 | 5 | **4 528** | 9 |
| `message` | 59 | 5 | 12 | 7 | 3 |
| `chatstate` | 42 | — | — | — | — |
| `ack` | 26 | — | — | — | — |

**Extremely heavy-tailed: median 2 nodes, max 4 528 — a ~2000x span.** The
attrs-to-nodes ratio is ~2:1, so a large `iq` pushes roughly **19 000 strings**
across the boundary in a single crossing.

#### [VERIFIED BY DATA] Two regimes, and the binding decides neither

**Common regime** (median and p90 — `message`, `chatstate`, `ack`, `receipt`):
2–13 nodes, 5–15 attrs. Cost is dominated by **fixed per-call overhead**, where
every tool sits between 4.6 and 6.1 ns. The difference is noise. For ~90% of
traffic the binding choice is performance-irrelevant.

**Tail regime** (large `iq` — usync, history sync, group metadata): 4 528 nodes
and ~19 000 strings in one crossing. No field-by-field traversal is acceptable
here, in any tool. The only good answer is **not to cross the boundary with the
object at all**.

Consequences:

1. **D-013 stands, on different grounds.** `wbgen-flat` remains the choice, but
   demoted from a performance decision to a **size, memory and dependency**
   decision — where the evidence is strong *and* independent of payload shape
   (21.9 KB shipped, zero npm dep, linmem that does not grow). D-014's grounds
   for rejecting boltffi never depended on performance.
2. **OQ-8 resolved: zero-copy moves into v1; D-005 is reversed** (see D-016).
   Not from speculation — a 433 KB stanza with 9 457 attrs exists in the real
   capture. Since the node *already came from a buffer* the engine decoded,
   traversing the object redoes work already done.

#### W6 — respecified

W6 measures **boundary strategy, not tooling**, with fixtures extracted from the
real capture.

| Fixture | Source | nodes / attrs / depth |
| --- | --- | --- |
| `F1-narrow` | real median (`chatstate`/`receipt`) | 2 / 5 / 2 |
| `F2-message` | `<message>` with nested `<enc>` | 5 / 12 / 3 |
| `F3-p90` | real p90 | 13 / 15 / 4 |
| `F4-tail` | the largest `iq` in the capture | 4 528 / 9 457 / 9 |

| Strategy | Mechanism | Expected cost |
| --- | --- | --- |
| `S1-traverse` | JS object crosses field by field | scales with nodes+attrs |
| `S2-reserialize` | JS flattens to a buffer, one crossing | one memcpy + Rust parse, but redoes work |
| `S3-bytes` | original frame bytes (requires the patch) | one memcpy, no double parse |

**The deciding metric is not ns/stanza — it is the slope of `S1` against
nodes+attrs.** If that slope is steep (and the tail says it will be), `S3` stops
being an optimization and becomes an architectural requirement, putting the
raw-byte-access patch into v1 for all four engines.

#### Sampling caveat

The 214 stanzas come from **one** session, and a pairing session at that —
`pair-success` present, many setup `iq`s, few `message`s. Not steady-state
production traffic.

This weakens the sample in one direction and **reinforces the conclusion** in
another: steady state is even more dominated by small `message`/`receipt`/`ack`
stanzas, which strengthens the common-regime finding. The tail does not
disappear either — usync, group metadata and on-demand history sync keep
producing it.

### Porting `wa-store-migrate`

**Recommendation: port, but as a differentially verified port — not a rewrite.**

Verified upstream facts: TypeScript, MIT, 11 commits, focused scope, adapter
pattern over a `StoreAdapter` interface, and — significantly — a published
`docs/IR.md`. The IR is **specified**, not merely implemented.

Reasoning:

- **The asset is not the code, it is the schema knowledge of 5 libraries.**
  Porting means re-verifying 20 routes. The code is the easy part.
- **Two diverging oracles is the real risk.** If the Rust side advances and the
  TS side stalls, which one is correct? Mitigation, cheap and consistent with
  D-004: **differential conformance** — both implementations over shared
  fixtures, output required to be identical. The port becomes proven rather
  than asserted.
- **Governance precedes engineering.** The project belongs to vinikjkkj, who
  also authors `zapo` — one of the engines. A coordinated, announced port is
  worth considerably more than a silent fork. This is a conversation to have,
  not an architectural decision to make.

**Proposed framing:** `docs/IR.md` is promoted to the normative spec; the TS
package and the Rust crate are both implementations of it. Then there is no
original and no copy — there is a spec and two implementers.

---

## RFC-008 — Boundary wire format

**Status:** **ACCEPTED** (rev 7), **published and frozen** in rev 45 as
contract version 1, shipped in
[`wa-wire-contract` 0.1.1](https://crates.io/crates/wa-wire-contract). What is
fixed is the envelope layout, the capability identifiers named at the time, and
what every field means. Additive change stays inside version 1 — a recording
declares capabilities by name and keeps names it does not recognise as bytes, so
naming a new one costs nothing a reader has to know about. Moving a field,
changing what one means, or removing one needs version 2 (D-132).

### The observation that decides the whole RFC

The original framing of this question — "binary-node encoding versus a
purpose-built flat encoding" — was the wrong question. It presupposes that the
node gets **serialized** at the boundary. It does not need to be.

Two facts settle it:

1. **The frame bytes already exist, in every engine, at the moment of decode.**
   Verified in all four (see table below).
2. **The frame does not contain the plaintext.** `<enc>` carries ciphertext;
   the plaintext arrives later, from Signal. So an L0-plain envelope was never
   going to be "the frame" — it is necessarily *the frame plus something*.

Therefore the envelope carries **the original frame bytes verbatim, plus a side
table of decrypted payloads**. Nothing is re-encoded, so there is no encoding to
choose. The node is parsed exactly once, host-side, and only if someone
subscribed to L1 — which is precisely the interest-driven principle of RFC-004.

The founding thesis holds literally: the boundary format *is* the wire format,
because the boundary never re-encodes.

### [VERIFIED] Where the bytes are, per engine

| Engine | Location | Variable | Patch |
| --- | --- | --- | --- |
| whatsapp-rust | `src/client/node_io.rs:337` | `OwnedNodeRef::new(buffer)` — `Yoke<NodeRef<'static>, BytesCart>` (`wacore/binary/src/node.rs:903`) | **none — already retained**; `backing_bytes()` (`:948`) returns the whole buffer as a refcount bump |
| Baileys | `Utils/noise-handler.ts:196-198` | `const result = transport.decrypt(frame)` | pass `result` alongside `frame` into `onFrame` — one line |
| whatsmeow | `client.go:823-830` | `decompressed` from `waBinary.Unpack(data)` | pass `decompressed` with the node into `handlerQueue` |
| zapo | `transport/binary/decoder.ts:334-344` | `nodeBytes` in `decodeBinaryNodeStanza` | return/emit `nodeBytes` alongside the node |

**This substantially lowers D-016's cost.** In `whatsapp-rust` zero-copy is
already free — the parsed node borrows *from* the retained buffer, and
`backing_bytes()` returns the whole buffer without copying it. (Earlier
revisions of this table named `slice_bytes()` here; that was corrected in rev
10, and the table itself kept the stale name until rev 31.) In the other three the
bytes sit in a local variable at the decode site; the patch is to propagate it.

**Normative payload definition** (verified consistent across all four): the
L0-wire payload is the **unpacked binary-node buffer** — after decompression,
without the leading format byte — i.e. exactly what each engine's decoder
consumes. `whatsapp-rust` documents this precisely at
`wacore/binary/src/node.rs:907-909`; `whatsmeow` performs the same `Unpack`
before `Unmarshal` (`client.go:876-882`).

### Envelope layout

Little-endian throughout — native on every target platform and on WASM.
No padding: payloads are opaque byte strings, so alignment buys nothing.

```
Envelope
  version      u16     contract major (see RFC-009)
  flags        u16     bit0  direction      0 = inbound, 1 = outbound
                       bit1  frame_origin   0 = original, 1 = re-encoded
                       bit2..15 reserved, must be zero
  frame_len    u32
  frame        u8[frame_len]      unpacked binary-node buffer, verbatim
  pt_count     u16
  pt_entries   PlaintextEntry[pt_count]

PlaintextEntry
  path_len     u8
  path         u16[path_len]      child indices from the root node
  status       u8                 0 = ok, 1 = decrypt_failed, 2 = unsupported,
                                  3 = unobserved (D-054)
  payload_len  u32
  payload      u8[payload_len]    decrypted protobuf bytes
```

Fixed header is 8 bytes. Against the measured median stanza (122 bytes) that is
~6%; against the tail (433 KB) it is nothing. In the common regime, cost is
dominated by fixed per-call overhead anyway (RFC-007), where 8 bytes of memcpy
does not register.

### Design notes

**Plaintext correlation is by node path**, not by ordinal or buffer offset.
A path is the list of child indices from the root. Rationale:

- **Ordinal** (the *i*-th `<enc>`) is fragile — it silently breaks if an engine
  reorders or filters children.
- **Buffer offset** is the most rigorous and would suit `whatsapp-rust`'s
  `slice_bytes()` perfectly, but requires the decoder to expose offsets. Go and
  TypeScript decoders do not, and D-011 says adapters stay dumb.
- **Path** is trivially produced during traversal in all four languages,
  unambiguous, and generalizes beyond `<enc>` — `<device-identity>`, media
  payloads, and future nested content need no format change.

**`status` makes failure explicit.** A stanza whose decryption failed still
crosses the boundary with `status = decrypt_failed` and an empty payload. It is
not dropped and not silently zero-valued — consistent with D-004 and with the
`whatspec`/`wa-store-migrate` house rule that unsupported states stay
distinguishable.

**`frame_origin` handles engines without byte access.** An engine that cannot
supply the original bytes re-encodes the node and sets bit1. The contract still
works; the capability matrix reports the degradation. This keeps third-party
engines viable without a patch (G6) while letting patched engines take the fast
path.

**Attrs never cross as a structure.** The question of "map versus flat parallel
arrays" — which RFC-007's measurements made look load-bearing — dissolves:
attrs live inside `frame`, in binary-node encoding, and are decoded once
host-side if and only if L1 was requested. The ~19 000 strings of the tail
stanza never cross as strings at all.

**One encoder serves both topologies.** In-process passes the envelope as a
slice or `Uint8Array`; the sidecar prefixes it with a `u32` length and a
handshake carrying the magic and contract version. The per-stanza bytes are
identical, so there is exactly one encoder and one decoder.

### Consequences for the parser

The host needs the binary-node token dictionaries to parse `frame`. Those come
from `whatspec` (`tokens/index.json`), which is already deterministic and
pinned. This makes `wa-wire-codec` a `whatspec` consumer rather than a
reimplementation — the same relationship `zapo` already has with its `spec/`
directory (finding 3.2).

**As implemented (rev 9):** the table is a *parameter* (`TokenTable`), not a
constant, so a host generated from a different `whatspec` build supplies its own
without needing a different parser (D-031). A generated table ships bundled
behind a default feature, with the source table's SHA-256 recorded alongside it.

Two properties of the encoding turned out to carry the design:

- **It is self-delimiting.** A node can be represented by the slice starting at
  its own list tag, running to the end of the buffer; the parse knows where to
  stop. No offset arithmetic is needed to slice a subtree out, which is what
  makes the whole tree navigable with no allocation and no index bookkeeping.
- **Some values have no string in the frame.** Packed digit runs and JIDs are
  assembled from parts, so there is nothing to borrow. They stay in parts and
  compare through `eq_str` (D-033) — which is the comparison L1 derivation
  actually performs, so nothing is lost by not materialising them.

---

## RFC-009 — Contract versioning and provenance

**Status:** **ACCEPTED** (rev 7), **in force** since rev 45: contract version 1
is frozen and the crate is at 0.1.1. The two axes this RFC separates are now
separate in public — a WhatsApp-side change moves provenance and never the
contract version.

The substrate changes without notice and third parties will depend on the
contract. Without a compatibility rule agreed *before* v1, the first breaking
change breaks every consumer — the one failure an interoperability layer cannot
survive.

### Two independent version axes

Conflating these is the usual mistake, so they are separated explicitly:

| Axis | What it versions | Where it appears | Changes when |
| --- | --- | --- | --- |
| **Contract version** | envelope layout, capability names, negotiation | `version: u16` in the envelope (RFC-008) | we change the boundary — rare, deliberate |
| **Spec provenance** | which `whatspec` manifest L1 derives from | reported at setup, not per stanza | WhatsApp changes — frequent, external |

A WhatsApp-side protocol change must **never** bump the contract version. If it
does, every adapter in the field breaks whenever Meta ships anything, which
would make the project worse than useless. The contract versions *our* boundary;
provenance tracks *their* protocol.

### Compatibility rules

- **L0 is always total.** No contract version may fail to represent a stanza.
  An unmodelled stanza crosses intact, in full, always. This is the escape hatch
  of RFC-001, and it is what prevents the contract from becoming a cage. It is
  also why a WhatsApp change cannot break the boundary: at L0 there is nothing
  to break.
- **L1 is additive-only within a major.** New event kinds and new fields may
  appear; existing ones never change meaning or disappear.
- **Unknown L1 fields are preserved, never dropped.** A consumer on an older
  generated L1 that receives a newer field keeps it as opaque data rather than
  discarding it. Dropping would make round-trips lossy and silently corrupt
  record/replay (RFC-005).
- **Deprecation before removal**, and removal only at a major.

### Negotiation

At setup, the adapter declares `{ contract_version, capabilities, provenance }`.
The host validates and **fails loudly**, refusing to start. Never at runtime,
never degraded silently — the same rule already fixed for capabilities
(RFC-002).

A capability the consumer requested but the adapter lacks is a setup error, not
a runtime surprise. A provenance mismatch between adapter and host is a
**warning**, not an error: it means they were generated from different WhatsApp
versions, which is expected during rollout and is exactly the kind of thing the
conformance suite exists to detect.

### Provenance

The generated L1 records the `whatspec` manifest it derives from — WhatsApp
version, domain hashes, generator version — and exposes it at runtime.

This is not bookkeeping. It is what makes RFC-005 meaningful: when two engines
disagree on L1 output, the first question is whether they were generated from
the same spec. Without provenance that question is unanswerable and every
conformance failure becomes ambiguous.

`whatspec` is already deterministic with inputs pinned by SHA-256 in
`bundles.lock.json`. That guarantee propagates rather than being re-invented.

### Codegen strategy — **generated and committed**

Chosen over build-script generation:

- **A protocol change becomes a reviewable diff.** This is the decisive
  argument. The project exists to track WhatsApp's protocol; seeing exactly what
  changed, in a pull request, is a core feature — not a build artifact.
- **Builds need no extra tooling**, so third parties can contribute to an
  adapter without running the extractor.
- **CI enforces freshness**: regenerating must produce no diff. This gets
  build-script generation's only real advantage — the impossibility of drift —
  without its costs.
- **It matches the house pattern.** `whatspec` and `wabench` are both pinned,
  reproducible and offline-capable. A third approach would be gratuitous.

---

## RFC-010 — Recording container

**Status:** **ACCEPTED** (rev 23), **implemented** in rev 24

RFC-008 specifies one stanza crossing the boundary. Nothing specifies a
*sequence* of them at rest, and every use the project has beyond its own test
suite starts by writing or reading one.

### The position this reverses

A container exists already. `adapters/zapo/scripts/emit-recording.ts:33` writes
`WAWR`, a `u32` count, then each envelope length-prefixed, and
`adapters/whatsapp-rust/tests/engine_agreement.rs:146` reads it back by hand.
Its own documentation explains why it was left unspecified:

> Deliberately trivial. The envelope format is the contract; this is only a way
> to put several of them in one file, and a reader that needs a spec for the
> container is a reader spending attention in the wrong place.

That was right while the only writer and the only reader lived in this
repository and ran in the same CI job. It stops being right the moment a
recording outlives the process that wrote it, moves between machines, or is
compared against a recording made by different code. At that point the container
is carrying claims — *which engine, which spec, which dictionary, which traffic*
— and a format that cannot state them makes those claims unverifiable rather
than absent.

### What the ad hoc format cannot express

Each of these is a defect only under the new use, which is why none of them was
wrong before:

| Gap | Consequence |
| --- | --- |
| Big-endian, while RFC-008 is little-endian throughout | Two byte orders in one file, for no reason |
| Count in the header | The writer must know the total before the first byte, so a ring buffer and a streaming writer are both excluded |
| No adapter, engine, spec or dictionary identity | A comparison cannot tell whether it is comparing like with like |
| No artifact class | A sanitized recording and a captured one are indistinguishable |
| No identity for the traffic that produced it | Two recordings of *different* input read as a regression |
| Truncation only detected when it lands mid-record | A file cut on a record boundary reads as complete |
| The reader indexes slices and panics | Every use outside a test needs a reader that reports instead |

### Layout

Little-endian throughout, matching RFC-008 (D-074). Three parts: a header, a
sequence of records, and a trailer.

| Field | Type | Notes |
| --- | --- | --- |
| `magic` | `u8[4]` | `WAWR`, unchanged — files carrying it already exist |
| `container_version` | `u16` | this layout; **not** the contract version |
| `meta_len` | `u32` | bytes of metadata that follow |
| `meta` | `u8[meta_len]` | TLV, below |
| `records` | … | until the trailer |

**On the third version number.** RFC-009 separates two axes and warns against
conflating them, so a third needs justifying. It is not a third axis: the
container version and the contract version are both *our* boundary, and RFC-009's
rule is about keeping WhatsApp's protocol off that axis entirely. They are split
because they move independently — a metadata tag can be added without the
envelope layout changing, and a recording written today must stay readable when
it does. Spec provenance remains the other axis, unchanged and per recording.

A **record** is `kind: u8`, `len: u32`, `payload: u8[len]`.

| Kind | Payload |
| --- | --- |
| `0x00` Envelope | an RFC-008 envelope, verbatim |
| `0x01` Mark | `delta_us: u32`, then a UTF-8 label — "stream:error", "reconnect", "fault injected here" |
| `0xFF` Trailer | `record_count: u32`, then `crc32: u32` over every preceding byte |

Further kinds are additive. A reader skips one it does not know and **counts**
it; a recording with skipped records is not comparable (D-078), because what was
skipped might have been load-bearing.

### The trailer detects damage; it does not establish identity

An earlier draft of this RFC put a 32-byte cryptographic digest in the trailer.
That does not survive contact with the constraints (D-084): every crate here is
dependency-free and `no_std`, and the TypeScript writer has to run in a browser
and a worker, so a SHA-256 would have to be hand-written twice — and a
hand-rolled hash is exactly the kind of code that is subtly wrong in a way tests
written by its author do not catch.

It would also have claimed more than it delivers. A digest in an unsigned file
detects accidental damage and nothing else, since anything that can rewrite the
records can rewrite the digest. **The container is not a tamper-evident
format**, and CRC-32 says so honestly while being fifteen lines and pinnable
against published vectors in both languages.

Identity comes from `input_digest` instead, which the container carries as
opaque bytes and never computes. That keeps identity the responsibility of
whoever produced the traffic, where the hash function is already chosen.

### Why the count moved to the end

A recorder that must state its length before writing anything cannot be a ring
buffer, and the flight-recorder use is a ring buffer by definition. Putting the
count in a trailer also makes truncation detectable in the case that matters:
a file cut on a record boundary is missing its trailer, which the header form
cannot express at all.

### A truncated recording is readable

**The most valuable artifact a crash recorder produces is, by definition, the
one that was interrupted.** So the absence of a trailer is a *state*, not a
parse error (D-076):

- every complete record before the cut is readable and usable;
- a partial record at the end is dropped with its bytes, not reported as
  corruption;
- the recording is marked truncated, and is `Incomparable` for any gate.

A format that rejected these would fail its most important use while passing
every test written against well-formed files.

### Metadata, and which of it is load-bearing

TLV: `tag: u16`, `len: u32`, `value`. Unknown tags are skipped, following
RFC-009's rule that unknown fields are preserved rather than dropped.

Skipping is not always safe, though. Some of these fields are the entire basis
on which two recordings may be compared, and a reader that silently ignored one
would produce a confident, wrong verdict. So **the high bit of the tag marks it
critical** (D-077): a reader that meets a critical tag it does not understand
may still inspect the recording, and may not call it comparable.

| Tag | Critical | Value |
| --- | --- | --- |
| `adapter` | yes | id, version, engine version, contract version, capability set |
| `provenance` | yes | the whatspec manifest RFC-009 already defines |
| `dictionary` | yes | identity and digest of the token table the frames were encoded against |
| `artifact_class` | yes | `captured`, `replayed`, `sanitized` or `synthetic` |
| `input_digest` | yes | the traffic this recording is a replay *of*; absent for a capture |
| `transform` | yes | for a sanitized artifact: the transformation's identity and configuration digest |
| `created_at` | no | wall clock at the first record |
| `note` | no | free text for a human |

### Comparability is declared, not assumed

`compare` today documents a precondition it cannot check:

> Both recordings must be of the *same* stanzas, in the same order.

For a gate that runs unattended, a precondition in a doc comment is a
precondition nobody enforces. Two recordings are comparable only when all of
these hold, and the comparison reports `Incomparable` otherwise (D-078):

- same `input_digest`, and both declare one;
- same `artifact_class`, and for `sanitized`, the same `transform`;
- compatible `dictionary` — see below;
- matching `provenance`, or the L1 half of the comparison is void;
- neither is truncated;
- no critical metadata tag and no record kind was skipped.

**A live capture declares no `input_digest`** and is therefore never
gate-comparable (D-079). This is not a limitation to work around: a capture is a
session that happened once, so nothing else can have seen the same input. A
capture is an *input* to the gate, not a result from it.

### The dictionary belongs to the recording

`compare(left, right, table)` takes one `TokenTable` for both sides. That holds
only while both recordings were encoded against the same dictionary, and the
whole point of an upgrade gate is that the two sides are different builds —
which is exactly when the dictionary may have moved, since D-031 already makes
it a parameter that travels with the WhatsApp client version.

So the table is resolved **per recording**, from its `dictionary` tag (D-082),
and a comparison whose tables are unavailable is `Incomparable` rather than
attempted. This matters most for a re-encoded frame: two builds may write
different token indices for the same value and be semantically identical, which
is a difference the L0 comparison should attribute to the dictionary rather than
to an engine.

### Sanitization: the constraint, not the algorithm

The algorithm is out of scope. Two constraints on it are not, because they are
properties of the format:

**A sanitized frame is necessarily re-encoded.** A JID cannot be replaced inside
a frame without rewriting the frame, and rewriting it forfeits
`FrameOrigin::Original` — the property that makes a recording faithful. Nothing
avoids this, so the format states it instead: sanitization always yields
`ReEncoded` frames, and an artifact class that says so.

**A sanitizer must preserve the shape of what it replaces, not only its type.**
A pseudonymous JID with a different digit count changes the packed-nibble
encoding. The two real bugs this project's conformance run has found so far
(D-062, D-063) were both encoding-shape bugs, visible only because captured
traffic contained those shapes. A sanitizer that normalizes them erases exactly
the class of defect the corpus exists to catch.

### Cross-language

The container is written by one language and read by another for the same reason
the envelope is, so it inherits the same rule: fixtures written by the TypeScript
writer and read by the Rust reader, and the reverse, as
`crates/wa-wire-conformance/tests/cross_language.rs` already does for RFC-008. A
Go writer follows when the fourth adapter does.

### Deliberately not decided: per-envelope timestamps

A flight recorder wants to answer "what happened in the last thirty seconds",
which needs a time on every record, and this RFC does not give it one. The
reasons to wait: a timestamp must never take part in a comparison, since two
replays of one input differ on it by construction; a sanitizer will want to
blur or drop it; and four bytes on every record is a real cost to pay before
any reader needs it.

`Mark` covers the case that motivated the question — "the error happened
here" — at no cost to a recording that does not use it. A timestamped envelope
kind is additive, so choosing later costs nothing. This is the part of the RFC
most likely to move on review, and it is stated rather than left to be noticed.

### Explicitly out of scope

- the sanitization algorithm;
- protobuf parsing of plaintexts;
- any performance budget;
- CLI, report rendering, storage policy, retention.

---

## RFC-005 amendment — comparison profiles

**Status:** **ACCEPTED** (rev 23), **implemented** in rev 24. Amends
[RFC-005](#rfc-005--conformance).

RFC-005 was written for one question: do two engines agree? The container makes
a second question mechanical — did this version regress against that one? — and
the two want opposite answers from the same evidence.

### Two engines and two versions are not the same comparison

| Finding | Two engines | Two versions |
| --- | --- | --- |
| frame bytes differ | not a fault: two encodings of one stanza are both valid | a fault: the same encoder changed its output |
| coverage lost by the candidate | not a fault: how much an adapter observes is a property of the adapter (D-055) | a fault: the same adapter observes less than it did |
| coverage gained by the candidate | not a fault | an improvement, reported and passing |
| frame origin degraded | not a fault: adapters differ by design | a fault: the same adapter stopped reaching its own buffer |
| length, direction, plaintext, L1 | a fault | a fault |
| provenance, input, class or dictionary mismatch | incomparable | incomparable |

So `Divergence::is_fault()` cannot stay a property of the divergence. The
comparator's job is to **record facts**; deciding which are faults is the
profile's (D-080):

```rust
let report = compare(&baseline, &candidate);
let verdict = report.evaluate(ComparisonProfile::Regression);
```

### The verdict is three-valued

`Pass`, `Fail`, `Incomparable`. Today a provenance mismatch is a divergence that
`agrees()` ignores, which means *"this comparison was between unlike things"*
renders as *"they agree"* — the worst available default, since it is a green
result produced by a comparison that never happened. An improvement folds into
`Pass` and is reported, so the verdict stays decidable.

### Regression is directional

Under `Interop` the two sides are symmetric. Under `Regression` they are not:
`left` is the baseline and `right` is the candidate, and the direction is what
separates a regression from an improvement. `Divergence::PlaintextCoverage`
already carries `only_left` and `only_right` separately, so the format needs
nothing; the policy reads the fields.

### Two facts the comparator currently suppresses

Both are correct suppressions under `Interop` and both are needed under
`Regression`, so they must be recorded and left unjudged rather than dropped
at the source:

- **frame origin changing.** Not compared today, deliberately: it differs by
  design between an engine that exposes its decode buffer and one that
  re-encodes, and reporting it per stanza would bury real findings.
- **a status changing between two non-`Ok` values**, such as `DecryptFailed`
  becoming `Unobserved`. Invisible today, because only `Ok` entries are
  compared and coverage counts only `Ok` ones. Between engines it says nothing;
  between versions it says an adapter stopped knowing why a payload was
  missing.

---

## 5. What this unlocks

Being honest about which of these are *real* and which are *speculative*.

### Solid — these follow directly from the design

1. **Migrating libraries without rewriting the system.** The originating
   complaint. The integration targets the contract; swapping engines is
   configuration.
2. **Uniform protocol observability.** One log/trace/metric format for any
   engine. Today, cross-library debugging is effectively impossible because
   every library reports differently. This alone may justify the project.
3. **Record and replay.** Capture L0 from production, replay offline against
   another engine or another version. Field bugs become reproducible without an
   account and without the customer's data being live. This is a genuinely
   large quality-of-life change for anyone operating at scale.
4. **Differential bug discovery.** Running four independent implementations on
   identical input surfaces bugs no single implementation's tests can find.
   Historically, this is the highest-yield property of a conformance suite.
5. **Canary and A/B in production.** 5% of sessions on one engine, comparing
   real metrics — feeding directly into `whatsapp-bench`.
6. **Engine failover.** WhatsApp changes something, one library starts taking
   `stream:error`s. Drain those sessions to another engine without re-pairing
   and without redeploying the application.
7. **Zero-downtime library upgrades.** Same engine, new version, sessions
   handed off rather than reconnected.

### Plausible — enabled, but needing real work

8. **A mock/test WhatsApp server.** With a canonical L0, writing a server-side
   simulator becomes tractable. This would unblock CI without a real account —
   `whatsapp-bench` currently depends on the private Barback for this.
9. **A protocol inspector.** Effectively Wireshark for WhatsApp Web, engine-agnostic.
10. **Protocol fuzzing.** Feed malformed L0 to every engine and compare failure
    modes. Security-relevant.
11. **Per-tier engine selection in SaaS.** Premium tenants on the more robust
    engine; commodity tenants on the cheaper one.
12. **Third-party engines.** Someone writes a Zig or Elixir client and it plugs
    into existing tooling on day one. This is what turns the project from a
    product into a platform.

### Honest counterweights

- It does **not** remove the need to understand the protocol — it exposes more
  of it, not less.
- It does **not** protect against bans. Orthogonal.
- It does **not** make a bad engine good. It makes a bad engine *measurably* bad,
  which is different and arguably more useful.
- It does **not** eliminate per-engine operational knowledge. Failure modes,
  reconnect behavior, and memory profiles still differ.

---

## 6. Risks and honest limitations

| # | Risk | Severity | Mitigation |
| --- | --- | --- | --- |
| R1 | N adapters against fast-moving upstreams (Baileys breaks APIs across minors) | high | generate from `whatspec`; conformance CI on pinned versions via `whatsapp-bench` |
| R2 | WhatsApp changes the protocol → L1 must regenerate | medium | already `whatspec`'s job; keep the dependency, do not fork the knowledge |
| R3 | oxidezap ends up the only maintainer of yet another project | high | Layer 1 must be usable standalone so third parties have a reason to contribute |
| R4 | The contract is perceived as "whatsapp-rust with facades" | **high** | thin-host decision ([D-002](#decision-log)); the contract must never assume oxidezap internals |
| R5 | `takeover` requires patching engines, including our own | medium | **updated rev 7:** D-020 puts takeover in v1, so `whatsapp-rust` is patched at step 9. `tap` stays mandatory and works unpatched on 3 of 4 engines, so takeover slipping does not block v1 — it degrades to a capability the matrix reports as absent |
| R6 | Handoff corrupts a production session | **critical** | RFC-003 R1–R4; refuse handoff when the route's declared loss exceeds threshold. **Out of v1** — Layer 3 |
| R7 | L2 becomes an unbounded semantic swamp | medium | **resolved for v1:** D-019 defers L2 entirely to v2 |
| R8 | Observation changes engine behavior in ways consumers do not expect | medium | mandatory cost disclosure (RFC-001); precedent is `whatsapp-rust`'s `processes_inline()` rerouting under raw forwarding |
| R9 | The envelope's verbatim-frame design fails on an engine that cannot supply bytes | low | `frame_origin` flag (D-026); re-encode path keeps such an engine conforming, with the matrix reporting the degradation |

### The central tension

There is a single axis that determines the character of the project:

**Thin host** — each engine runs whole and autonomous; the contract only
observes and translates. Faithful multi-engine, honest, easy third-party
adoption. Costs: higher overhead, more expensive handoff, larger loss matrix.

**Fat host** — the host owns socket, crypto, and store; engines degrade to logic
plugins. Minimal overhead, cheap handoff, easy atomicity. Costs: requires deep
patches in every library, so in practice only oxidezap's own forks comply — and
the project becomes "whatsapp-rust with facades", which destroys the neutrality
pitch.

**Recommendation:** thin-host contract with an *optional*, capability-negotiated
fast path. The contract assumes nothing. Engines that implement the fast path
get zero-copy and cheap handoff; the rest work with more overhead and fewer
capabilities.

---

## 7. Open questions

> **None of the remaining questions block v1.** All four belong to Layer 3,
> which is out of v1 scope (§8). Each carries a provisional decision plus the
> explicit trigger that would revise it, so nothing is left merely open.

### OQ-1 — Isolation unit — *provisional: process per engine*
Session-as-task in a shared multi-tenant process, or process-per-engine hosting
N sessions? Scale pushes toward the first; handoff safety pushes toward the
second.

**Provisional decision:** process per engine, N sessions inside. It bounds the
blast radius of an engine panic, and RFC-004 already establishes that the
genuinely shareable resources (token tables, protobuf descriptors, HTTP pool,
executor) are shareable *within* a process — so the sharing win is kept.

**Revision trigger:** a target of sessions-per-host high enough that per-process
memory, not per-session state, dominates the footprint. Measure before
revisiting.

### ~~OQ-2 — Store ownership~~ — **RESOLVED in rev 7**
The host never owns the store (D-007); Layer 3 v1 ships on external
`wa-store-migrate` (D-008). The snapshot-cost measurement refines *which*
Layer 3 option is used, not *whether* the host owns the store — that part is
decided. See [RFC-006](#rfc-006--store-ownership).

### OQ-3 — `wa-store-migrate` port — *decided technically, open on governance*
The technical answer is D-012: a differentially verified port, with `docs/IR.md`
promoted to normative spec and TS and Rust as co-implementations.

**What remains is not architecture.** The project belongs to vinikjkkj, who also
authors `zapo` — one of the engines. A coordinated port is worth more than a
silent fork. This is a conversation, and it does not gate v1, which contains no
Layer 3 at all.

### ~~OQ-4 — L2 in v1~~ — **RESOLVED in rev 6**
v1 is **L0 + L1 only**. Consumers send raw stanzas. L2 deferred to v2 (D-019).

### ~~OQ-5 — Name~~ — **RESOLVED in rev 6**
**`wa-wire`** (D-018).

### ~~OQ-6 — Takeover in `whatsapp-rust`~~ — **RESOLVED in rev 6**
Takeover **is** in v1, and `whatsapp-rust` gets the patch (D-020). Scope
clarified by D-021: takeover suppresses *dispatch*, never *crypto*.

### ~~OQ-7 — Handoff window~~ — **MEASURED in rev 58**
The engine-side floor is between 31 ms and 273 ms, and the four engines differ
by 8.7×. Measured with `wabench`'s `ready` scenario, seven runs each against
Barback:

| Engine | Median | Spread |
| --- | --- | --- |
| `hypermeow` | 31.2 ms | 3.2 ms |
| `zapo` | 52.9 ms | 3.4 ms |
| `Baileys` | 156.3 ms | 22.7 ms |
| `whatsapp-rust` | 273.3 ms | 3.2 ms |

**`ready` is the right scenario and `reconnect` is not** (D-135). `ready` times
the interval from workload start to the engine's own connected event, which is
when Layer 3 could release a queued backlog — the definition the window needs.
`reconnect` looks closer to a handoff and is not comparable across engines: each
adapter decides what "back" means, and `whatsmeow`'s returns when the socket is
up where Baileys waits for the open event. The same measurement reads 2.5 ms and
16.5 ms for that reason, and neither is the window.

**A floor, not an SLA.** Barback runs locally, so these exclude every network
round trip; a real handoff adds them to each of the handshake's legs. The number
bounds what the engine costs, which is the part a route can be chosen on.

**Resync does not differentiate.** `offline-sync` takes about 0.55 s on all
three engines that complete it, because the backlog is paced by the server
rather than by the client.

### ~~OQ-8 — Zero-copy priority~~ — **RESOLVED in rev 5**
Settled by the measured stanza shape in RFC-007, not by W6: the real tail
(4 528 nodes / 9 457 attrs / 433 KB) makes field-by-field traversal untenable in
any binding. Zero-copy is in v1; D-005 reversed by D-016. W6 remains worth
running to size the patch's payoff, but is no longer a blocker.

---

## 8. Implementation plan

**v1 scope (locked in rev 6):** `wa-wire`, L0 + L1, takeover included, MIT.
No L2. No Layer 3 host.

### Definition of done for v1

**All six are met, and as of rev 57 the result is installable rather than only
green.** Seven crates are on crates.io, all seven built on docs.rs, and a crate
depending on nothing but the registry compiles against them.

What that does *not* mean is on the criterion below it: two of the four adapters
still need an engine change that is in review, so the four-engine agreement is
reproducible here and not yet by a stranger.

1. `wa-wire-contract` published, with the RFC-008 format specified and frozen.
   **Done in rev 45**, at 0.1.0 — [on crates.io](https://crates.io/crates/wa-wire-contract),
   0.1.2 as of rev 56, when the last capability without a provider got one. The
   other six followed in rev 57.
2. Four adapters emitting L0-plain: `whatsapp-rust`, `zapo`, `Baileys`,
   `hypermeow`. **Done as of rev 41.** Two are built against engine changes
   still in review: [polymorfa/hypermeow#5](https://github.com/polymorfa/hypermeow/pull/5)
   and [WhiskeySockets/Baileys#2762](https://github.com/WhiskeySockets/Baileys/pull/2762).
3. L1 derivation generated from `whatspec`, host-side, single implementation.
   **Done.** Inbound stanzas in rev 11, payloads in rev 27 — generated rather
   than written since rev 28 corrected where the numbers come from — and
   outbound stanzas in rev 33. Nothing the generators cannot express is left
   unreported: `UNMODELLED_FIELDS` and `UNTYPED_FIELDS` are empty, and the nine
   `REQUEST_SCOPED_ASSERTIONS` are a design limit rather than a backlog.
4. Conformance suite (RFC-005) green: identical L0 in → identical L1 out across
   all four engines. **Green for all four as of rev 43**, over six pairwise
   comparisons of one corpus, and **in CI since rev 49** over one committed
   recording per engine.
5. Capability matrix machine-readable and enforced at setup. **Done in rev 20.**
   All five upgrade-gate criteria are measured as of rev 29: stanzas not lost,
   frames still parsing, the same L1, plaintext coverage held, and a
   performance budget per read path.
6. Takeover working on at least `zapo` (native) and `whatsapp-rust`. **Done.**
   No patch in the end: `StanzaInterceptor` landed upstream as #1239, so both
   are native — partial on the Rust side, and the matrix says which five
   stanzas it never offers.

**Explicitly out of v1:** L2 commands, Layer 3 host, session handoff, fencing,
multi-session pooling, media transfer, the `wa-store-migrate` port.

### v2 scope — Layer 3: the host, and moving a session between engines

**Opened in rev 57.** v1 made four engines produce the same events from the same
traffic. v2 is what that was for: a session running on one engine, moved to
another, without re-pairing.

The design is not the open part. [RFC-003](#rfc-003--session-handoff-protocol),
[RFC-004](#rfc-004--multi-session-host-and-resource-sharing) and
[RFC-006](#rfc-006--store-ownership) were accepted in rev 7 and nothing since
has disturbed them. What v2 adds is implementation, and two numbers nobody has.

#### What it inherits as settled

- **Handoff is stop-the-world per session** (RFC-003). One device, one
  connection: a second with the same keys makes the server kill the first, so
  there is no blue/green. The six phases — quiesce, barrier, detach, snapshot,
  attach, resume — follow from that and are not a choice.
- **The host never owns the store** (D-007), and Layer 3 ships on external
  `wa-store-migrate` (D-008). Owning it would mean reimplementing four engines'
  session logic.
- **`detach` is type-level distinct from `logout`** (R3-R4). A bug there unpairs
  the customer's device, so it is enforced by types rather than by care.
- **Sharing is per-process and never per-account** (RFC-004). Token tables,
  protobuf descriptors, the HTTP pool and the executor are shared; key material,
  Signal state, sockets, and the device-list and LID caches are not — the last
  two because sharing them is a cross-tenant leak wearing the clothes of an
  optimisation.

#### What has to be measured before anything is claimed

Both are marked `[UNKNOWN]` in the RFCs that need them, and both are the kind of
number this project refuses to guess.

- **The unavailability window, per engine pair** (RFC-003, OQ-7). Handshake plus
  resync, and no one has timed it. Until it is timed there is no SLA to offer
  and no threshold for Layer 3 to refuse a route by.
- ~~**The loss, per route** (R2).~~ **Measured in rev 58.** Twelve routes across
  the four engines, computed by `wa-store-migrate`'s own `planLosses` rather
  than read off its README:

| Route | Lost | Degraded |
| --- | --- | --- |
| `whatsmeow` → `zapo` | nothing | nothing |
| `whatsapp-rust` → `zapo` | nothing | `appStateVersions` |
| `Baileys` → `zapo` | nothing | `sessions`, `senderKeys`, `privacyTokens` |
| `zapo` → `whatsmeow` | `deviceLists` | nothing |
| `whatsapp-rust` → `whatsmeow` | `deviceLists` | `appStateVersions` |
| `Baileys` → `whatsmeow` | `deviceLists` | `sessions`, `senderKeys`, `privacyTokens` |
| → `whatsapp-rust`, from `zapo` or `whatsmeow` | `contacts`, `messageSecrets` | `appStateVersions` |
| → `Baileys`, from `zapo` or `whatsmeow` | `contacts`, `messageSecrets` | `sessions`, `senderKeys`, `privacyTokens` |
| `Baileys` ↔ `whatsapp-rust` | `contacts`, `messageSecrets` | all four of the above |

  **Loss is a property of the destination**, and degradation of both ends.
  Nothing is lost moving *into* `zapo`, which makes it the safe target and
  `Baileys ↔ whatsapp-rust` the pair to refuse by default.

  One domain in the IR, `senderKeyDistributions`, is neither read nor written by
  any adapter. Forcing it into a snapshot makes every route look like it loses
  something; no real source can produce it, so it is dead weight in the IR
  rather than a route cost. Worth deleting upstream, not worth modelling.

**So v2 started with an experiment, not a host**, and both unknowns are now
numbers. A route whose loss is unmeasured is a route the gate cannot judge, and
a host that moves a session across it is claiming something no one checked.

#### Two engines cannot do a phase RFC-003 requires

Found by trying to measure, and neither is in the loss matrix because neither is
about state.

**Neither is a blocker any more.** The paragraph below described two engines;
one was fixed upstream and the other was never broken.

- ~~**`zapo` cannot drop its transport.**~~ **Withdrawn in rev 63.** The claim
  came from a harness failure — *"zapo does not support dropping its transport,
  which reconnect requires"* — which reports on the `whatsapp-bench` client, not
  on the engine: `clients/zapo/benchmark.mjs` simply never called
  `registerTransportDrop`. `WaClient.disconnect()` has been there all along, and
  its own doc says it closes the transport gracefully, does not clear stored
  credentials, and that `connect()` again resumes the same session; it emits
  `isLogout: false`, and `zapo` has no auto-reconnect, so nothing reopens the
  socket unless a caller asks. Given the hook, five `reconnect` cycles complete
  against the mock server with no re-pairing.
- ~~**`whatsapp-rust` does not come back**~~ — **fixed upstream in rev 60.**
  `Client::pause()` and `Client::resume()` landed as
  [#1265](https://github.com/oxidezap/whatsapp-rust/pull/1265), with the
  misleading log line fixed separately as
  [#1264](https://github.com/oxidezap/whatsapp-rust/pull/1264). `pause` is a
  detach that is not a stop: no `Disconnected` is dispatched because the
  application ended the connection, the account stays registered, and other
  devices see nothing. `resume` tells the run loop to reconnect with no backoff
  owed for an offline window the application chose.

  **Measured at 43 ms**, twenty cycles, p50 42.99 and p95 44.78, timed from
  `pause` to a dispatched `Connected`. That is the phase 3 → phase 5 cost, and
  it is six times cheaper than the same engine's cold `ready` because a resume
  skips what pairing and first sync do.

  Its doc states the loss RFC-003 needs stated: *"Nothing is carried across the
  pause that a network drop would not also have taken."*

  What rev 59 described is what was fixed, and the paragraph is kept below
  because the shape of the gap is the same one `zapo` still has. `disconnect()`
  writes `is_running = false` and fires the shutdown notifier: it is a terminal
  stop, not a pause. The run loop then prints *"Expected disconnect (e.g., 515),
  reconnecting immediately…"* and, on the next line, *"Client run loop has shut
  down."* A `connect()` after that completes the Noise handshake and the server
  sends its `<success>`, but the loop that would decode it has exited, so
  nothing is read, no `Connected` is dispatched, and eighteen seconds later the
  keepalive discovers `NotConnected`.

  So phase 5 is not slow or flaky here — **the library has no detach that is not
  a stop**, which is a different problem from the one the symptom suggested. It
  can be a **source** and not a target.

Together these removed four of the twelve routes. Two are back as of rev 60,
leaving the three that need `zapo` as a source. Both were engine work rather
than host work, and neither was visible from reading the RFCs.

#### Definition of done for v2

1. The unavailability window measured for every engine pair, and the per-route
   loss matrix filled in from measurement rather than from documentation.
2. ~~A fencing token: persisted, monotonic, and proven to serialise two hosts
   that both believe they own one session (R1).~~ **Done in rev 62**:
   `wa_wire_adapter::handoff::Fence`. A `u64` a host persists and an `admit`
   that refuses a token older than the newest seen, naming both — the host that
   comes back from a pause is told it *lost* the session, not that something
   failed.
3. ~~`detach` and `logout` distinct at the type level, with a test that the
   distinction cannot be crossed by accident (R4).~~ **Done in rev 62**:
   `wa_wire_adapter::handoff::Detach` is a trait with one method, so a host
   driving a handoff has no `logout` to reach (D-138). The test is a
   `compile_fail` doctest paired with an identical passing one, and the engine
   side is `lifecycle.detach` — declared by `whatsapp-rust` via `Client::pause`
   and by `zapo` via `WaClient.disconnect` (rev 63).
4. ~~Deduplication by message id in L1 (R3).~~ **Done in rev 61**:
   `wa_wire_l1::dedup::SeenStanzas`, a bounded window a caller drives. It is
   beside `derive` rather than inside it, because telling a redelivery from an
   arrival is exactly the knowledge one stanza does not carry, and a stateful
   `derive` would stop being the thing four engines can be compared on (D-010).
5. One session moved between two engines and back, with the events on either
   side compared by the conformance runner — the v1 machinery pointed at the v2
   claim. **Half done in rev 64**: the *store* moves,
   `whatsapp-rust → zapo → whatsapp-rust` on a real paired session
   (`tools/handoff-cycle`). What is left is the traffic half, which needs `zapo`
   to attach with a store it did not create — its backends are pluggable and
   writing one is host work (D-007).
6. ~~The loss the route declared, and no more, observed in that move.~~ **Done
   in rev 64**, and the answer is *no*: everything came back byte-identical
   except `appStateSyncKeys.timestamp`, which the route does not declare and
   which `zapo`'s writer turns from absent into `0`. `appStateVersions` is
   declared lossy in both directions and came back identical. The check exists
   now, and it is the check that found both (D-142).

**Explicitly out of v2:** L2 commands, media transfer, QuickJS or any
fat-host binding, and multi-host scheduling. The first is a project of its own
(D-019, R7); the last is what a fencing token exists to make possible later, not
something v2 builds.

#### What v2 depends on that this project does not own

`wa-store-migrate` belongs to vinikjkkj, who also authors `zapo`. D-012 settled
the technical question — a differentially verified port with `docs/IR.md`
promoted to normative — and [OQ-3](#oq-3--wa-store-migrate-port--decided-technically-open-on-governance)
records that what remains is a conversation rather than a design. v2's core
step, `snapshot`, runs on that tool. **That is the single largest risk in this
scope**, and unlike v1's two open engine PRs it is a dependency at the centre
rather than at the edges.

### Ordering

Dependencies, not a schedule.

All design blockers are cleared as of rev 7. RFC-008 and RFC-009 are accepted,
so step 0 is done and implementation can begin.

| Step | Work | Blocks | Notes |
| --- | --- | --- | --- |
| ~~0~~ | ~~RFC-008 boundary format, RFC-009 versioning~~ | — | **done in rev 7** |
| ~~1~~ | ~~`wa-wire-contract` — envelope encode/decode, capability + provenance types~~ | — | **done in rev 8** — `no_std`, zero dependencies, allocation-free decoding; 99.7% line coverage |
| ~~2~~ | ~~`wa-wire-codec` — binary-node parse over `whatspec` tokens~~ | — | **done in rev 9** — token table is a parameter, not a constant; 99.8% line coverage |
| ~~3~~ | ~~`whatsapp-rust` adapter, tap mode~~ | — | **done in rev 10** — plus `wa-wire-adapter`, the SDK every Rust adapter shares |
| ~~4~~ | ~~`wa-wire-l1` — derivation generated from `whatspec`~~ | — | **done in rev 11** — generated from the `incoming` domain, tests generated alongside |
| ~~5~~ | ~~`zapo` adapter~~ | — | **done in rev 12** — plugin + stanza filter, takeover native, cross-language fixtures |
| ~~6~~ | ~~**Conformance runner**~~ | — | **done in rev 11** — the central claim is now a test result |
| 7 | `Baileys` adapter | third engine | `ws.on('frame')` for tap; one-line patch for bytes |
| 8 | `hypermeow` adapter + Go hook | fourth engine | hook at `client.go:844`, bytes at `:824`; **MPL-2.0 subdirectory with NOTICE** |
| ~~9~~ | ~~`whatsapp-rust` takeover patch (D-020)~~ | — | **done in rev 13** — a pre-dispatch interceptor, merged upstream as #1239 |
| ~~10~~ | ~~`whatsapp-rust` adapter, L0-plain~~ | — | **done in rev 14** — a per-`<enc>` plaintext event merged upstream as #1240, joined to its frame adapter-side |
| ~~11~~ | ~~`wa-wire-recording` — the RFC-010 container, plus comparison profiles~~ | — | **done in rev 24** — the ad hoc `WAWR` is a contract read by both languages, `is_fault` is a profile, and comparability is declared in the file rather than assumed by the runner |
| ~~12~~ | ~~`wa-wire-gate` — the command, and a fuzz sweep over every decoder~~ | — | **done in rev 25** — the first thing here anyone can run; three-valued exit codes; every decoder now proves the "reportable, never a panic" claim it makes |
| ~~13~~ | ~~`wa-wire-proto` and the payload half of L1~~ | — | **done in rev 27** — the boundary carried decrypted payloads that nothing read; the numbers are generated from whatspec's `WAProto.proto` after rev 28 corrected where they came from |

**Step 6 is the milestone that matters.** Everything before it is plumbing;
step 6 is where "four engines produce identical L1" stops being a claim and
becomes a test result. If the project is going to fail, it fails there — so
reaching it early is the point of this ordering.

Steps 1–2 are pure and dependency-free, so they can be built and unit-tested
with no engine present. Steps 3 and 5 are the two engines needing no patch for
tap mode, which is why they come before the ones that do.

### Quality bar

Line coverage must stay at or above **95%**, enforced in CI. Two habits carry
most of the weight:

- **Delete unreachable defensive code instead of leaving it uncovered.** A
  branch no test can reach is a branch no reviewer can trust. The `Reader`
  tracks its unread tail rather than an index precisely so every read is a
  `split_*_checked` whose single failure arm is the real short-read case.
- **Extract limits into pure helpers so they stay testable.** The frame and
  payload length prefixes cap at `u32`; reaching those through the builder
  would need a 4 GiB buffer, so the narrowing lives in its own function and is
  checked directly.

Portability is enforced too: the contract builds with no allocator and for
`wasm32-unknown-unknown`, since JS adapters reach the core through WebAssembly.

### Byte-access patch sites (all verified, RFC-008)

| Engine | Site | Change |
| --- | --- | --- |
| whatsapp-rust | `node_io.rs:307` | **corrected in rev 10:** `slice_bytes()` needs a slice already inside the buffer, so it cannot hand over the whole thing. One method added upstream — `OwnedNodeRef::backing_bytes()`, a clone of the yoke's cart, so a refcount bump rather than a copy |
| zapo | `transport/binary/decoder.ts:344` | emit `nodeBytes` alongside the node |
| Baileys | `Utils/noise-handler.ts:196-198` | pass `result` into `onFrame` |
| whatsmeow | `client.go:824-830` | carry `decompressed` with the node |

### Deliberately deferred

- W6 benchmark (RFC-007) — worth running to size D-016's payoff, not a blocker.
- Snapshot cost measurement (RFC-006) — needed before Layer 3, not before v1.
- Handoff window measurement (OQ-7) — same.
- Isolation unit (OQ-1) — a Layer 3 decision.
- `wa-store-migrate` port governance (OQ-3) — conversation with vinikjkkj,
  independent of v1.

---

## Decision log

| ID | Decision | Rationale | Rev |
| --- | --- | --- | --- |
| D-001 | L0 and L1 are not alternatives; L0 is normative and L1 is a derived view | Removes the "which canonical format" argument entirely and makes conformance mechanically checkable | 1 |
| D-002 | `wa-wire` is a plugin first, a host second | No IPC in the common case; one-line adoption; third-party engines possible without our daemon | 1 |
| D-003 | Thin-host contract with optional capability-negotiated fast path | Preserves neutrality (R4) without giving up performance for our own forks | 1 |
| D-004 | Divergence between engines is reported via capabilities, never hidden | Matches existing culture in `whatspec` (`dropsByReason`) and `wa-store-migrate` (loss reports) | 1 |
| D-005 | Zero-copy frame bytes excluded from v1 | No engine has it; it is an optimization, not a prerequisite | 1 |
| D-006 | Handoff is stop-the-world per session | Forced by the server: one connection per device (finding 3.5) | 1 |
| D-007 | The host never owns the store | Store operations carry logic, not just persistence (RFC-006); owning them means reimplementing four engines' session logic | 2 |
| D-008 | Layer 3 v1 ships on external `wa-store-migrate`; native snapshot contract designed in parallel | Option B works today with zero engine changes; Option A is the same IR moved inside, so neither invalidates the other | 2 |
| D-009 | Rust core, Cargo workspace monorepo | Predictable low overhead (G3), no GC in the stanza path, `wacore` already targets wasm32 which is how JS adapters consume it | 3 |
| D-010 | L0 splits into L0-wire and L0-plain; `L0-plain → L1` is pure and runs host-side in Rust, once | Prevents N diverging L1 implementations; the Go adapter cannot host Rust, so derivation must not live in adapters | 3 |
| D-011 | Adapters are deliberately dumb — emit L0-plain and stop | Nothing to diverge, little to break; also mitigates R1 | 3 |
| D-012 | `wa-store-migrate` is ported as a differentially verified port, with `docs/IR.md` promoted to normative spec | The asset is schema knowledge, not code; differential conformance prevents two diverging oracles | 3 |

| D-013 | FFI base is `wasm-bindgen` without serde, flat marshalling, glue emitted by proc-macro | Wins speed, size and memory simultaneously; zero npm runtime dep; linear memory does not grow — decisive for a long-lived multi-session process | 4 |
| D-014 | `boltffi` rejected for this application | 128 KB runtime dep and linmem growing to 20.7 MB are disqualifying for a library third parties install and a process that runs for months | 4 |
| D-015 | L1 egress is batched, sledgehammer-style — as a pattern, not a dependency | 217 ns/op unbatched vs 29 ns/op batched on string-dense events; WhatsApp already delivers in batches | 4 |

| D-016 | **Reverses D-005.** Zero-copy raw frame bytes is a v1 requirement, not a post-v1 optimization | Real capture shows a heavy tail (4 528 nodes / 9 457 attrs / 433 KB); field-by-field traversal is untenable there in any binding, and the node already came from a buffer the engine decoded | 5 |
| D-017 | D-013 stands, but as a size/memory/dependency decision rather than a performance one | Measured shape shows ~90% of traffic is dominated by fixed per-call overhead, where all tools are within noise | 5 |

| D-018 | Name is **`wa-wire`** | The contract *is* the wire format — the founding thesis; sits naturally beside `whatspec`, `wabench`, `wa-store-migrate` | 6 |
| D-019 | v1 is **L0 + L1 only**; L2 deferred to v2 | L2 is most of the work and all of the semantic arguments (R7); consumers can send raw stanzas meanwhile | 6 |
| D-020 | **Takeover is in v1**, and `whatsapp-rust` gets the patch | Takeover is what makes engines genuinely interchangeable — under it, engine-specific semantics stop mattering. Owner's call, against the earlier recommendation to defer | 6 |
| D-021 | Takeover suppresses **dispatch**, never **crypto** | L0-plain requires the engine's decryption; a takeover that disabled crypto would make L0-plain unproducible and silently degrade the contract | 6 |
| D-022 | License **MIT**, with `adapters/hypermeow/` isolated as MPL-2.0 | Aligns with whatsapp-rust, zapo, Baileys, wa-store-migrate; whatsmeow is MPL-2.0 so patched files inherit it per-file. Isolation must be explicit and carry a NOTICE | 6 |

| D-023 | The boundary carries **original frame bytes verbatim + a plaintext side table**; the node is never re-encoded | The frame bytes already exist in all four engines, and the frame never contained the plaintext anyway — so there is no encoding to choose. Parse happens once, host-side, only if L1 was subscribed | 7 |
| D-024 | Plaintext correlates to its node by **path** (child indices from root) | Ordinal breaks under reordering; buffer offsets require decoders to expose offsets, which Go and TS do not and D-011 forbids demanding. Path is trivial in all four languages and generalizes beyond `<enc>` | 7 |
| D-025 | Failed decryption crosses the boundary with `status = decrypt_failed`, never dropped or zero-valued | Consistent with D-004 and the house rule that unsupported states stay distinguishable from zero | 7 |
| D-026 | `frame_origin` flag lets an engine without byte access re-encode and still conform | Keeps third-party engines viable unpatched (G6) while patched engines take the fast path | 7 |
| D-027 | Contract version and spec provenance are **separate axes**; a WhatsApp change never bumps the contract version | Otherwise every adapter in the field breaks whenever Meta ships anything. L0 totality is what makes this possible: at L0 there is nothing for a protocol change to break | 7 |
| D-028 | L1 codegen is **generated and committed**, with CI enforcing that regeneration is a no-op | A protocol change becomes a reviewable diff — a core feature for a project whose purpose is tracking protocol change, not a build artifact | 7 |

| D-029 | Line coverage floor of 95%, enforced in CI | The contract is the product; unreviewable code is untrustworthy code | 8 |
| D-030 | Unreachable defensive branches are removed, not left uncovered | A branch no test can reach is a branch no reviewer can trust. Concretely: the `Reader` tracks its unread tail instead of an index, so every read is a `split_*_checked` with one real failure arm | 8 |
| D-031 | The token table is a **parameter**, not a constant | Dictionaries move with the WhatsApp client version, which RFC-009 makes a matter of provenance rather than contract version. A host generated from a different `whatspec` build supplies its own table instead of needing a different parser | 9 |
| D-032 | Parsing validates the whole tree up front, so every accessor afterwards is infallible | Pushing `Result` into `attrs()`, `children()` and `at_path()` would make the common path noisy to serve an error that validation already ruled out | 9 |
| D-033 | Packed runs and JIDs stay in parts and compare/render on demand | Their text exists nowhere in the frame, so borrowing is impossible and joining would allocate. `eq_str` is the comparison L1 derivation actually needs, and it walks the parts | 9 |
| D-034 | The codec parses only; re-encoding stays with the engine | An engine that cannot supply original bytes re-encodes in its own language and sets `frame_origin` (D-026). A host-side encoder would duplicate that with no caller | 9 |
| D-035 | `l0.plaintext` is a capability of its own, separate from the inbound tap | The frame is available the moment a stanza is decoded; a plaintext only exists after Signal has run. An engine can offer one and not the other — `whatsapp-rust` does exactly that | 10 |
| D-036 | An adapter's capability claims are **checked against its stanzas**, not merely declared | A declaration nobody verifies drifts from the code the first time an engine moves underneath. `AdapterInfo::verify` turns "this adapter is zero-copy" into a failing test | 10 |
| D-037 | A sink receives the pre-encoding `RawStanza`, not a finished buffer | An in-process consumer then never pays for encoding, while a sidecar consumer encodes and writes. Same value, two costs, one adapter | 10 |
| D-038 | Adapters live outside the main workspace | Each drags in a whole engine — tokio, TLS, protobuf — and the contract and codec are dependency-free on purpose. An adapter also inherits its engine's toolchain, which must not become the project's | 10 |
| D-039 | L1 is generated from whatspec's `incoming` domain, not written | That domain records how WhatsApp Web itself parses each stanza. Writing the derivation by hand would mean guessing at what the spec states | 11 |
| D-040 | The most specific reading of a field wins; obligation is taken at its weakest | whatspec records every call site, so one field appears several times with different readers. One site using the always-present reader does not make a field required on the wire — trusting it made generated shapes reject valid stanzas | 11 |
| D-041 | Shapes of one tag are tried richest-first, behind whatever assertions the spec gives | Spec order let the most permissive shape win every time: a call receipt claimed every message receipt, its required fields being a subset | 11 |
| D-042 | Tests for generated code are generated from the same source | Sixteen fixtures kept in step with sixteen shapes by hand would drift; deriving both from one source is the only way they cannot | 11 |
| D-043 | Conformance compares L1 by **meaning**, not by bytes | Two engines can encode one value differently and both be right. Reporting that as a divergence would bury the ones that are real | 11 |
| D-044 | A frame difference is reported but is not a fault; a derivation difference is | The format has more than one valid encoding, so L0 differences are context. The derivation is pure, so two engines cannot both be right at L1 | 11 |
| D-045 | Two engines failing the same way is agreement, not a finding | Being consistently silent about a stanza neither models is exactly the consistency conformance is checking for | 11 |
| D-046 | The boundary format is implemented per adapter language, verified by committed cross-language fixtures | An adapter runs inside its engine, so a JS engine needs a JS encoder. Two descriptions of one format tested only separately are two formats waiting to diverge | 12 |
| D-047 | An adapter that cannot reach the engine's buffer re-encodes and says so | `frame_origin` exists for exactly this. Claiming verbatim bytes that are not verbatim would make a consumer trust a frame it should not | 12 |
| D-048 | Takeover is a pre-dispatch interceptor, not a router override | `StanzaRouter::register` panics on duplicate tags, and overriding a handler would still leave no way to reach a tag the engine does not model. A gate before dispatch covers both | 13 |
| D-049 | Connection-critical stanzas are never offered for takeover | `success`, `failure`, `stream:error` and `ack` settle auth, shutdown and send waiters. Taking one would leave a client authenticated-but-unaware or waiting forever — breaking it, not extending it. `zapo` protects the same auth tags | 13 |
| D-050 | A claimed stanza is always answered, ack replacing the nack it would have got | Answering nothing leaves it in the offline queue and recycles the stream — a failure `whatsapp-rust` had already been bitten by once | 13 |
| D-051 | Tap and takeover carry separate capability sets; neither is a superset | Tap sees the auth phase and cannot suppress; takeover suppresses and cannot see it. One declaration for both would be false in one direction | 13 |
| D-052 | An adapter holds a frame until its plaintexts arrive, emitting one envelope per stanza | The frame exists when a stanza is decoded, a plaintext only after Signal. Emitting twice would make a consumer correlate what the adapter already knows, and "one stanza, one envelope" is what replay and conformance compare | 14 |
| D-053 | Waiting is bounded in stanzas, not milliseconds | No per-`<enc>` signal exists for one that will never decrypt, so something must give up. A stanza count is identical on every machine; a duration is not, and this output is meant to be compared across engines | 14 |
| D-054 | `PlaintextStatus::Unobserved` is added to the format | The three existing statuses each assert a cause. An adapter that watches plaintexts appear knows a node produced nothing but not why, and guessing would put an unverified cause into the record | 14 |
| D-055 | A fan-out stanza is emitted as L0-wire, with no plaintext table | The engine numbers `<participants><to>` encs after the direct ones and only for its own device; reproducing that needs the device JID, which a plugin-installed adapter does not have. A frame without payloads is a smaller claim than a payload on the wrong `<enc>` | 14 |
| D-056 | The conformance corpus is frames, not envelopes, and is committed | An envelope is what an adapter produced; a frame is what an engine received. Feeding one frame to two engines is the only way their outputs are comparable, and committing it makes a corpus change a reviewed change | 15 |
| D-057 | Engine agreement is asserted at `is_identical`, not `agrees` | The suite tolerates an L0 difference because two encodings are both valid — but on this corpus there is nothing to tolerate. Asserting the weaker property would let a future encoder change start producing different bytes silently | 15 |
| D-058 | Capture takes its endpoint as configuration and knows nothing about the server | A tool that names one server becomes a dependency on it. An endpoint, an optional pairing hook and a TLS-verification feature cover a local test server and a real one with the same code | 16 |
| D-059 | Captured frames are not scrubbed | A scrubber that misses a field is worse than none, because it invites trusting the output. Capture from a test account and review what gets committed | 16 |
| D-060 | Encoder divergences are listed by name, not counted | A count going up says nothing about whether the new difference is a valid encoding choice or one engine being wrong. A named list makes each one a reviewed decision | 16 |
| D-061 | The example consumer's dependency graph is enforced by a test | "Swap the engine and the consumer does not change" erodes with one convenience dependency, and nothing would fail to say so. The test is what says so | 17 |
| D-062 | A packed run is read as an integer where one is expected | The nibble alphabet exists to compress runs of digits, so any real encoder packs timestamps. Reading only strings meant every packed integer failed to derive | 17 |
| D-063 | A JID is read whatever form the encoder chose | The wire has a dedicated JID form and an encoder may use it or write text; both are valid. Reading only the dedicated form made one engine derive where another derived nothing from identical traffic | 17 |
| D-064 | A bare server is only read as a JID when the wire wrote it as a token | Servers are dictionary entries, so a token is evidence. Accepting any word without an `@` would turn a JID field into "any string at all" | 17 |
| D-065 | A spec defect is fixed in the spec, never worked around here | The derivation is generated from whatspec so that it says what WA Web says. Softening a required field locally would make it quietly disagree, which is the failure this project exists to detect | 17 |
| D-066 | A requirement is refused at install, not reported per stanza | An unmet capability shows up as *missing traffic*, where the evidence of the problem is the thing that is absent. Refusing to start names it while there is still something to name | 20 |
| D-067 | An unmet requirement names every missing capability, not the first | A caller fixes its setup in one pass rather than one round trip per capability | 20 |
| D-068 | An outbound frame is the same bytes as an inbound one | Record and replay stop being separate features: a captured envelope can be sent back as it stands. Each adapter converts to whatever its engine wants, so a consumer never learns which | 21 |
| D-069 | Sending is declared separately from observing | An adapter built to observe genuinely cannot send. One capability set covering both would be false for whichever the consumer actually holds | 21 |
| D-070 | `l0.outbound` does not imply `l0.request` | Writing to the socket and being handed the correlated answer are different powers; an engine may offer one without the other | 21 |
| D-071 | A rejection carries the reply's frame only where the engine hands it over | `whatsapp-rust` parses an error reply and keeps its code and text, not its bytes. Naming the absence lets a consumer check; pretending uniformity would make it find out at runtime | 22 |
| D-072 | The three declarations are a ladder, each a superset of the last | A consumer raising its requirement from observing to sending to requesting never loses something it already relied on | 22 |
| D-073 | The recording container is specified, reversing "deliberately trivial" | It was a way to put envelopes in one file while the only reader lived in the same CI job. A recording that outlives its process carries claims about engine, spec, dictionary and traffic, and a format that cannot state them makes those claims unverifiable rather than absent | 23 |
| D-074 | The container is little-endian, matching RFC-008 | The ad hoc format is big-endian while the envelopes inside it are not. Two byte orders in one file is a defect waiting for the first reader written from the wrong half | 23 |
| D-075 | The record count lives in a trailer, not the header | A writer that must state its length before the first byte cannot be a ring buffer, and the flight-recorder use is a ring buffer by definition | 23 |
| D-076 | A recording without its trailer is truncated, not invalid: readable, and not comparable | The most valuable artifact a crash recorder produces is by definition the one that was interrupted. A format that rejected it would fail its most important use while passing every test written against well-formed files | 23 |
| D-077 | Metadata tags carry a critical bit; an unknown critical tag forbids comparison, not inspection | RFC-009 says unknown fields are preserved rather than dropped, but some of these fields *are* the basis on which two recordings may be compared. Skipping one silently would produce a confident wrong verdict | 23 |
| D-078 | Comparability is declared in the file, never assumed by the runner | `compare` documents "the same stanzas, in the same order" as a precondition it cannot check. For a gate that runs unattended, a precondition in a doc comment is one nobody enforces | 23 |
| D-079 | A live capture declares no input digest and is therefore never gate-comparable | A capture is a session that happened once, so nothing else can have seen the same input. It is an input to the gate, not a result from it | 23 |
| D-080 | `is_fault` becomes a comparison profile: the comparator records facts, the profile judges them | Between two engines a frame difference is two valid encodings; between two versions of one engine it is the encoder changing. The same evidence, opposite verdicts | 23 |
| D-081 | The comparison verdict is three-valued: pass, fail, incomparable | Today a provenance mismatch is ignored by `agrees()`, so "this comparison was between unlike things" renders as "they agree" — a green result from a comparison that never happened | 23 |
| D-082 | The token dictionary is resolved per recording, not per comparison | D-031 already makes the table travel with the WhatsApp client version, and an upgrade gate compares exactly the builds where it may have moved. Two builds writing different indices for one value is a dictionary difference, not an engine one | 23 |
| D-083 | A sanitizer must preserve the encoding shape of what it replaces, not only its type | Both conformance findings so far (D-062, D-063) were encoding-shape bugs, visible only because captured traffic held those shapes. Normalizing them erases the defect class the corpus exists to catch | 23 |
| D-084 | The trailer carries CRC-32, not a cryptographic digest | A digest in an unsigned file detects accidental damage and nothing more, so SHA-256 would claim tamper-evidence the format does not provide — and would have to be hand-written twice, `no_std` and browser-safe. Identity lives in `input_digest`, which the container carries and never computes | 23 |
| D-085 | Capabilities travel as their identifier strings, not as the bitset | `CapabilitySet` is a `u8` whose bit assignment is internal to the Rust crate, while `Capability::identifier` is stable and is literally what the TypeScript enum holds. A container read by three languages must not depend on two of them agreeing about bit order | 23 |
| D-086 | The gate's verdict reaches a pipeline as three distinct exit codes, not two | A CI step that folded "incomparable" into failure sends someone hunting a bug that is not there; one that folded it into success ships on no evidence. The distinction only pays if it survives the process boundary | 25 |
| D-087 | Dictionary *resolution* is the host's job, not the comparator's | `Comparability::check` cannot know which token tables exist where it runs. The host does, so it reports `UnresolvableDictionary` — and says whether a dictionary was declared or assumed, because an assumption a reader cannot see is one nobody checked | 25 |
| D-088 | The malformed-input sweep asserts invariants, not just the absence of a panic | A decoder that survives by accepting nonsense has not survived. It also asserts that mutations land on both sides, since a sweep where everything is refused proves only that the first length check works — and can become that silently | 25 |
| D-089 | The payload reader is its own crate, sibling to the codec | One parses the stanza, the other parses what its `<enc>` children decrypt to. Both are wire formats read from buffers somebody else wrote, and keeping them apart is what lets the fuzz sweep and the allocation counter hold each to the same rule without either knowing the other | 27 |
| ~~D-090~~ | ~~The payload half of L1 is **written**, not generated~~ | **Reversed by D-093.** The stated reason was false: whatspec does extract the protobuf | 27 |
| D-093 | **Reverses D-090.** The payload's field numbers are generated from whatspec's `WAProto.proto`, and only the rules stay written | whatspec's `wa-proto` extracts the schema from the bundle's `internalSpec` modules and pins it by SHA-256 in its manifest, so the oracle D-090 said did not exist was there all along. Hand-writing the numbers cost 22 of the 29 wrappers the spec declares | 28 |
| D-094 | Wrappers are collected from the spec **by type**, never by name | A list of names is a second place to add one and therefore a place to forget one. Collecting every `Message` field whose type is `FutureProofMessage` is what turned seven into twenty-nine | 28 |
| D-096 | The gate reports what the payloads turned out to be, per side | A count of stanzas is the half the boundary had before it could read a payload at all. Counting per side rather than merging is what makes a candidate that read fewer messages visible instead of hidden behind a sum | 29 |
| D-097 | Each read path carries a time budget, asserted rather than plotted | A benchmark whose only output is a number tells the next person nothing: they see 900 ns and cannot say whether it is fine. The budgets sit several times above the measured cost, because what is worth catching is a copy where there was a borrow, not a slower runner | 29 |
| D-095 | Provenance carries a digest per domain, not one for the build | WhatsApp can renumber a protobuf field without touching how a stanza parses. One digest would call two builds the same spec when only half of it matched | 28 |
| D-091 | A payload whose first field is unmodelled crosses as `Unmodelled(n)`, never as empty | Without the whole schema, an unknown variant and a metadata field are indistinguishable, so reporting the number seen is the most that can be claimed. Reporting nothing would make a protocol change look like an empty message, which is the one reading that is certainly wrong | 27 |
| D-092 | Wrappers are unwrapped before the kind is reported, and the depth is reported | A consumer asking what a message said does not mean "it was a device-sent copy of an ephemeral wrapper". The count is kept because a nested payload is a fact worth seeing rather than one to hide | 27 |
| D-098 | A field is read by its `wireName`, and the test that says so is written by hand | The spec records two names per field and they differ for fifty of them. A generated test cannot catch the generator reading the wrong one: the fixture is built from the same spec by the same rule, so the pair agrees with each other and with no stanza a server sends | 30 |
| D-099 | What the generator cannot express is reported in three lists, not one | One list read as one backlog. Nine of its fifteen entries were assertions no pure derivation can ever make, and burying them next to six real gaps made the real ones look like the same kind of thing. Separated, one list is a design limit that will never shrink and the others are work | 30 |
| D-100 | A response-to-request assertion is a design limit, not a gap | `derive` is a pure function of one stanza (D-010), so `from` matching the request's `to` is not something it can check — the request is not in scope and giving it one would end the purity that lets derivation run once, host-side. The assertions are emitted as text for the caller that does hold the request | 30 |
| D-101 | Two spec fields differing only by category are both emitted, the second aliased | `verified_name` arrives as a child on one shape and an attribute on another. Dropping either would silently lose a reading; renaming the collision by its category keeps both and says which is which | 30 |
| D-102 | Observing what an engine *sends* is a capability of its own, listed apart from being able to send | An adapter sends; a recording needs to know what left. Until upstream #1260 no engine could tell us, and folding the two into one row would have made the gap invisible — the matrix would have read `✅` for both while half of every session went unrecorded | 31 |
| D-103 | Where takeover is partial, the matrix says which stanzas are excluded rather than scoring it | `whatsapp-rust` never offers the five tags that settle connection state, and that is the correct design — an engine that handed them over could not stay connected. A bare `✅` would claim more than is true and a bare `❌` would deny something real, so the cell carries the exclusion | 31 |
| D-104 | An engine claim names its file, and only pins a line where the line is the claim | Three of this document's line references had drifted by rev 31 while every statement they supported was still true, which trains a reader to stop checking. The file is stable enough to find the thing; the line number was decoration that decayed | 31 |
| D-105 | Observing what an engine sends is its own capability, `l0.outbound.observed` | `l0.outbound` is the ability to send, which every adapter with a `Sender` has and which says nothing about whether the engine reports what left. `AdapterInfo::verify` already gated outbound envelopes on a capability documented as "observe the outbound path" and tested the one for sending, so the distinction existed in the prose before it existed in the code | 32 |
| ~~D-106~~ | ~~Outbound stanzas are compared at L0 and never derived~~ | The derivation comes from whatspec's `incoming` domain, which records how WA Web parses what the *server* sends. An outbound `<ack>` satisfies those shapes and means the opposite, so it does not fail to derive — it derives confidently and wrongly, and two engines agreeing on a wrong reading reports as agreement. Deriving them needs the request-side domains, which nothing here generates from | 32 |
| D-107 | A mixin group's alternatives are tried richest-first, as ordering and not preference | `NewsletterMessageAck`'s required fields are a subset of `NewsletterQuestionResponseAck`'s, so the leaner alternative accepts every stanza the richer one does and trying it first would claim them all. This is D-041 one level down, arrived at from the same evidence | 32 |
| D-108 | Read budgets are reported but not enforced under coverage instrumentation | `cargo llvm-cov` costs about 4x on the walk it counts, so a ceiling measured against it measures the instrumentation. The recording walk sat inside its ceiling on CI's runner and outside it on a developer's — passing on the margin rather than the merits. `cargo test` still enforces them, and the self-test asserts the exemption rather than being silently disabled by it | 32 |
| D-109 | **Reverses D-106.** Outbound stanzas are derived, from a second generator over whatspec's `stanza` and `iq` domains | D-106 said the request-side domains were not consumed and left the bytes as the claim. `stanza/index.json` is 179 records every one of which is `direction: outgoing`, and `iq`'s `request` half describes 137 more. The gap was that nothing here read them, not that nothing described them | 33 |
| D-110 | The outbound derivation is a separate generator, not a flag on the existing one | The two domains answer different questions. `incoming` records how WA Web *parses*, naming the accessor a reader calls; `stanza` and `iq` record how it *builds*, naming how the sender produces each value. Read backwards a builder is still a shape, but the vocabularies do not meet, and one generator serving both would be two generators sharing a file | 33 |
| D-111 | A JID's flavour is enforced, not collapsed into "a JID" | Two `<ack class="notification">` builders differ in nothing but `to` being a device JID in one and a user JID in the other, and two spam reports in nothing but a group JID against a user JID. Reading all three as one type made one shape out of two and let either claim the other's stanzas | 33 |
| ~~D-112~~ | ~~Shapes the dispatch can never reach are named, not reordered around~~ | Four differ from an earlier shape in nothing a reader can see: whatspec separates an attribute the caller supplies from one the builder computes, which is a fact about the builder and not about the stanza, and an extra optional attribute discriminates nothing. Reordering moves the problem to the other shape; **Superseded by D-113.** The four were not four shapes; they were two, twice | 33 |
| D-113 | **Supersedes D-112.** Builders producing one stanza are folded into one shape, and the fold is reported | Three of the four were two modules building the same stanza, differing only in whether a value is handed in or computed and whether one bothers to model an optional attribute. Keeping both is two types no stanza can choose between. Mutual indistinguishability, not one-way: a shape strictly subsumed by another is still a different shape and merging it would discard fields the survivor lacks | 34 |
| D-114 | The fold is recomputed from the spec each run rather than listed | It un-merges by itself the day whatspec records something that separates a pair — which happened to one of them the same afternoon, when the literal inside `CUSTOM_STRING` stopped being dropped upstream | 34 |
| D-115 | A field is read from the node its `sourcePath` names | whatspec records the path when a field is not on the node the shape names — an ack's paid-conversation data hangs off `<biz>`, its pricing off `<biz><pricing>`. Reading them off the root found nothing and found it silently, since the fixture was built the same wrong way. The third instance of one failure: a generator and its generated tests walking one spec by one rule cannot catch the rule | 36 |
| D-116 | An optional mixin's children are optional | A `sameNode` mixin says the whole group may be absent, and inlining it without carrying that across promoted every child to required. `NewsletterMessageAck` gained a mandatory `edit` and two mandatory byte bodies, so an ack carrying only `class` and `t` — which the spec allows — did not derive | 36 |
| D-117 | A mixin group about a child is absent when that child is | A variant whose fields all hang off one descendant is *about* it. One has a single optional field under `<biz><pricing>`, so its derivation succeeded on any node at all and, being last, turned every ack into a paid conversation with no data in it — an absence reported as a presence | 36 |
| D-118 | Each direction is compared as its own sequence | An engine dispatches what it received from the read path and what it sent from the send path, with no ordering between them. One merged sequence aligned by position would call a different interleaving a direction divergence and a frame divergence on every stanza after it. Within a direction the order is the engine's own and is stable | 36 |
| D-119 | Whether a missing direction is a fault depends on who was watching | Two adapters that both observe the outbound half and disagree on how much there was have found something; one that cannot see it at all has not. Counting the second as missing stanzas blames an engine for its observer, which is the failure `l0.plaintext` coverage already avoids one layer in | 36 |
| D-120 | A recording carrying a direction its manifest does not claim is a fault under every profile | Inconsistent with itself rather than with the other recording: nothing downstream can tell whether those records are real or an artefact of how the file was assembled | 36 |
| D-121 | The envelope is written a third time, in Go, and cross-checked rather than shared | Go cannot host the Rust core — cgo in the per-stanza hot path is the cost the boundary exists to avoid — so the format is implemented by someone who cannot use any of this code. That is the case the design was made for, and the difference between a specification and a library with three callers. The Go encoder has no Rust to check itself against, so its fixtures are read by the Rust side | 38 |
| D-122 | ~~D-022's MPL-2.0 subdirectory is not needed~~, and the reason is kept | The adapter carries no `whatsmeow` file: the hooks went upstream, where they are MPL-2.0 already, and what is here only imports the engine — which §3.3 allows under other terms. D-022 stays because copying or vendoring one file would make it true again | 38 |
| D-123 | The boundary is implemented once per *language*, not once per adapter | Baileys is the second TypeScript engine, and a fourth writing of the format in a language that already has one would be a description nobody checks against the others. `@oxidezap/wa-wire-ts` was extracted out of the `zapo` adapter for it; the Go one stayed separate because Go genuinely cannot use any of it | 41 |
| D-124 | A shared package holds the vocabulary and no adapter's declaration | The extracted module carried `zapo`'s `INFO` and a `has()` closing over it, which read as "the adapter" while there was one and was wrong the moment there were two. What an adapter *has* lives with the adapter | 41 |
| D-125 | Baileys reports a plaintext's *child* index, chosen having watched two adapters resolve an `<enc>`-relative one | Both of the earlier engines report which `<enc>` decrypted, counting `<enc>` nodes, and their adapters must work out which child that is — ambiguous the moment a stanza carries anything else, and unresolvable for a fan-out `<message>`, so both give up and emit L0-wire. Designing the hook against a need already understood cost nothing and removed the case | 41 |
| D-126 | Every joiner emits in arrival order, and a stanza waiting on payloads holds up the ones behind it | Emitting an unheld stanza the moment it arrives puts it ahead of a held one that came first. The comparison aligns by position, so the reordering reads as a divergence in whichever engine happened to be slower — a finding about timing wearing the clothes of a finding about behaviour. Holding the queue costs latency in the adapter and buys a recording whose order is the wire's | 42 |
| D-127 | `pending` counts what waits on payloads; `queued` counts what has not left | Once stanzas leave in order the two stop being the same number, and a caller asking "is anything outstanding?" means the first while a caller asking "has everything drained?" means the second. One name for both would answer whichever question the reader did not ask | 42 |
| D-128 | Conformance compares what each engine **re-encodes**, not what it forwards | Three of the four adapters are zero-copy and forward the corpus bytes untouched, so comparing those compares nothing: three identical streams agree by construction and the run is green while proving that a copy is a copy. Re-encoding is where four implementations can differ, and on this corpus two of them differ on five stanzas of fourteen | 43 |
| D-129 | Each engine replays the corpus in its own process and writes envelopes as files | A container would carry the claims a gate needs — which traffic, which adapter, whether the file is whole — and the comparison supplies all of them itself. `zapo` goes through one only because it had one | 43 |
| D-130 | `l0.plaintext.cause` is named before publication, though nothing provides it yet | `PlaintextStatus` has carried `DecryptFailed` and `Unsupported` since the format was written and no adapter has ever emitted either — the format anticipated a distinction the vocabulary did not name. Under `Unobserved`, a build whose messages stopped decrypting is indistinguishable from one whose adapter stopped observing, which is the failure mode this project keeps finding. Adding it after publication is a contract version bump; adding it now costs a line | 44 |
| D-131 | A capability is a promise about what crosses the boundary, not a fact about the engine behind it | *Plugin host* and *runtime portability* are rows in the matrix and are not capabilities: they matter to whoever is choosing an engine, and a consumer cannot require either of them at setup. Naming them would put things in the vocabulary that `require` could never usefully check | 44 |
| D-132 | Publication freezes contract version 1 as *fixed*, not as *final* | New capability identifiers, new reserved flag bits and new metadata tags may still appear: a reader that meets one keeps it rather than refusing, because the format was built to carry what it cannot resolve. What needs version 2 is moving a field, changing what one means, or removing anything — the three a reader cannot survive by ignoring | 45 |
| D-133 | `l0.plaintext.cause` stays without a provider rather than being wired to `Event::UndecryptableMessage` | That event is per message, deduplicated per `(chat, id)` and suppressed on resends, and its `decrypt_fail_mode` is a display hint rather than a cause. A status that is right sometimes and absent otherwise is worse than one that is uniformly `Unobserved`: a gate can act on the second and not on the first | 50 |
| D-134 | v2 is Layer 3 — the host and moving a session between engines — and it opens with an experiment rather than a host | The three RFCs it needs were accepted in rev 7 and have not moved, so the design is not the open part. What is open is two numbers: the unavailability window per engine pair and the loss per route. R2's promise is "loss is known, declared and accepted", and nothing is known — which makes the promise unsayable, not merely unproven. A route whose loss is unmeasured is one the gate cannot judge | 57 |
| D-135 | The handoff window is measured by `ready`, never by `reconnect` | `ready` times workload start to the engine's own connected event, which is when a host could release a queued backlog. `reconnect` is not comparable across engines: each adapter decides what "back" means, and whatsmeow's returns on socket-up where Baileys waits for the open event — 2.5 ms against 16.5 ms for the same nominal thing | 58 |
| D-136 | Layer 3 prefers `zapo` as a handoff target and refuses `Baileys ↔ whatsapp-rust` by default | Loss is a property of the destination: nothing is lost moving into `zapo`, while both directions between `Baileys` and `whatsapp-rust` lose `contacts` and `messageSecrets` and degrade four more domains. A default that has to be overridden is how a measured matrix becomes a policy | 58 |
| D-137 | Deduplication keys on tag *and* id, holds a bounded window, and reports a third answer for what it cannot identify | An `<ack>` and a `<receipt>` for one message carry that message's id, so keying on the id alone would report the second as a redelivery of the first and drop it. The window is bounded because the alternative is unbounded state in a `no_std` crate that allocates nowhere else; the trade is that an evicted id reads as new, which loses nothing. And a stanza with no id is `Untracked` rather than `New`, so a caller counting duplicates cannot read "could not tell" as "there were none" | 61 |
| D-138 | `Detach` is a trait with one method, not a variant of an enum that also names `logout` | The two acts look alike from one line away and differ irreversibly: a detach hands the session on, a logout unpairs the customer's device. `enum End { Detach, Logout }` is the shape a bug flips and still compiles. A host driving a handoff holds `&dyn Detach` and cannot log out because there is no method to call — proved by a `compile_fail` doctest paired with an identical one that compiles, so it cannot start passing for an unrelated reason | 62 |
| D-139 | The fencing token lives in `wa-wire-adapter` beside `Detach`, and an engine never sees it | The fence is the host's bookkeeping and the detach is the engine's act. An engine has no way to know what else in the fleet believes it owns this session, so a token threaded through the adapter API would be a parameter no implementation could check. Keeping them adjacent and separate is what lets a host run them in order | 62 |
| D-140 | An engine's capability is read from the engine, never from a harness failure | `zapo` was recorded as unable to drop its transport on the strength of a `whatsapp-bench` message that says exactly that. The message reports on the benchmark client, which never registered a drop hook, and `WaClient.disconnect()` had been there the whole time. A harness failure is evidence about the harness until someone reads the engine | 63 |
| D-141 | The handoff cycle takes `wa-store-migrate` from npm, not from its repository | The repository does not build — `src/adapters/wa-web` is imported by the registry and by five test files and is not committed — and the published package ships it. Waiting for the repository to be fixed would have kept the one dependency at the centre of v2 blocking on someone else's commit, when the artefact that runs was already available | 64 |
| D-142 | A move is checked by comparing the snapshots byte for byte, never by counting rows | 807 prekeys going out and 807 coming back is the answer to "did it run", and a round trip that returns the right number of prekeys with the wrong bytes inside them passes it. Counting also missed the one real finding: every app-state key came home with a timestamp it did not leave with, and the count never moved | 64 |

---

## Changelog

### rev 64 — 2026-08-11

- **The snapshot step runs.** `tools/handoff-cycle` moves one session
  `whatsapp-rust → zapo → whatsapp-rust` and compares what came back with what
  left. `wa-store-migrate` comes from npm rather than from its repository, which
  still does not build (D-141) — the dependency at the centre of v2 is no longer
  waiting on someone else's commit.
- **On a real session, not a synthetic one.** Paired against Barback with
  `capture-corpus`, 807 prekeys, 70 app-state keys, one session, one identity.
  The committed fixture is that store reduced to eight of each so a reviewer can
  read it; the account is the mock server's.
- **Byte for byte, not by count** (D-142). Prekeys, sessions, identities,
  `signedPreKey`, `deviceLists` and `appStateVersions` all came back identical.
- **One undeclared difference, which is exactly what item 6 asks about.**
  `appStateSyncKeys.timestamp` is absent in the IR — `whatsapp-rust` has no such
  column — and `zapo`'s writer turns absent into `0`
  (`adapters/zapo/from-canonical.js:81`), its reader handing back `0` rather
  than absent. Every key comes home claiming 1970 and `planLosses` says nothing.
  Harmless on this route, since `whatsapp-rust` does not read the field. Not
  harmless on a route whose destination picks the newest key by it.
- **And one over-declaration.** `appStateVersions` is called lossy in both
  directions and came back byte-identical. Over-declaring is the safe direction;
  worth knowing, because a host reading the matrix would warn about a move that
  costs nothing.
- **A third, read rather than observed.** `updatedAtMs: t.timestampMs ??
  Date.now()` (`adapters/zapo/from-canonical.js:104`) makes the same migration
  produce a different store each run. The fixture has no `tcTokens`, so this is
  recorded to keep it from being found twice.
- **The upstream read recipe corrupts every prekey.** `wa-store-migrate`'s
  README splits `prekeys.key` as a 64-byte keypair; it is a libsignal
  `PreKeyRecordStructure` protobuf. Splitting it yields two 32-byte strings that
  are the right length and neither of which is a key, so nothing downstream
  notices. `dump-rust-store.py` parses the record. The device columns *are* a
  raw pair, private first.
- **`registrationId` divides the four engines**, and blocks the route above this
  tool. `zapo` generates 1..16381 and Baileys masks to 14 bits; `whatsapp-rust`
  uses 1..2³¹−1 and whatsmeow a full `uint32`. `wa-store-migrate` validates 14
  bits, so a real `whatsapp-rust` session fails `validate: true` on every route
  out. The wire carries a `uint32` — WA Web's `WhisperTextProtocol` declares it
  so — which makes this a local convention two engines keep and two do not.
- **All four adapters now declare `lifecycle.detach`.** `Baileys` via
  `end(undefined)`, which closes without the `loggedOut` status code every
  consumer branches on to decide whether to wipe its auth state; `hypermeow` via
  `Disconnect()`, which calls `expectDisconnect` first so the automatic
  reconnect does not fire. Both take the one engine method they use rather than
  a whole client, so what they depend on is in the type.

### rev 63 — 2026-08-11

- **`zapo` can release its session, and could all along.** rev 58 recorded that
  it could not, citing the harness verbatim: *"zapo does not support dropping
  its transport, which reconnect requires"*. That message is emitted by
  `whatsapp-bench`'s SDK when a client never calls `registerTransportDrop`, and
  `clients/zapo/benchmark.mjs` never did. It says nothing about the engine
  (D-140).
- **What the engine actually offers.** `WaClient.disconnect()` closes the
  transport gracefully, does not clear stored credentials — its own doc says
  `connect()` again resumes the same session — and emits `isLogout: false`.
  `zapo` has no built-in auto-reconnect, so nothing reopens the socket unless a
  caller asks. Every clause a detach needs.
- **Measured, not reasoned.** Registering the hook in the bench client and
  running `reconnect --client zapo` gives 5/5 cycles valid against the mock
  server, p50 3.9 ms, with no pairing or QR traffic in the server log. The
  earlier finding was withdrawn on evidence, not on a second reading.
- **`createDetacher` in the `zapo` adapter**, and `DETACHING_INFO` beside
  `SENDING_INFO`. Not on the sending ladder: releasing the session and
  correlating a reply are unrelated powers, and stacking them would make a host
  ask for `l0.request` to be allowed to hand a session on.
- **Two of the four adapters now declare `lifecycle.detach`.** `whatsapp-rust`
  via `Client::pause`, `zapo` via `WaClient.disconnect`. `Baileys` and
  `hypermeow` can — ending the socket without marking it closed, and
  `Disconnect()` then `Connect()` — and their adapters do not expose it yet.
- **This unblocks the preferred route.** D-136 prefers `zapo` as a handoff
  target because nothing is lost moving into it; a round trip also needs it as a
  source, which is what was thought impossible. Items 5 and 6 of the v2
  definition of done now wait on `wa-store-migrate` alone.

### rev 62 — 2026-08-11

- **The fencing token and the type-level detach**, items 2 and 3 of the v2
  definition of done, in `wa_wire_adapter::handoff`. They ship together because
  they are the two halves of one question — who may end a session, and how.
- **`Fence` refuses the host that came back.** R1 is the case a per-session lock
  cannot cover: after a GC pause or a partition, two hosts each believe they own
  a session, and the Signal ratchet does not survive two writers. `Fence::admit`
  takes a monotonic `FencingToken` and refuses one older than the newest it has
  seen, naming both — a host has to be able to report that it *lost* the session
  rather than that something failed, because the two call for opposite
  responses. A refusal does not move the fence, and `resumed_at` is how a
  restarted host comes back knowing what it had already admitted.
- **`FencingToken::next` stops rather than wraps.** A wrapped token compares
  below every token in the field, which is exactly the state fencing exists to
  make impossible, and it would look like it worked.
- **`Detach` offers detaching and nothing else** (D-138). Not a `logout` on the
  same trait, not an enum with two variants — the distinction R4 asks for is
  enforced by what exists. The `compile_fail` doctest is paired with an
  identical passing one; verified by flipping the failing line to the passing
  call and watching rustdoc report that it compiled.
- **`lifecycle.detach` is the eleventh capability**, with a provider on the day
  it was named: `whatsapp-rust`'s `Client::pause` (#1265) closes the socket,
  refuses `connect()` including an attempt in flight, and leaves the account
  paired. `Detacher` is the adapter's, and `tests/detach.rs` asserts the client
  is left *resumable* — which is what tells `pause` apart from `disconnect` and
  `logout` after the fact. Swapping the call to `disconnect()` fails two of the
  three tests.
- **The trait promises "will not open another", not "holds no socket".** An
  engine can have a connection attempt in flight that a detach cannot reach
  into; `pause` documents exactly that. What every implementation must guarantee
  is that no such attempt becomes a live session.
- **Adding a capability does not bump the contract version**, which the code has
  said since the ninth and two READMEs still denied. A recording declares
  capabilities by name and keeps names it does not recognise as bytes. What
  costs a version is removing an identifier or changing what one means.
- **`check-docs.py` now reads every README**, not only `DESIGN.md`. The stale
  prose above was outside its reach, and so was the published claim it was
  written for. Sixteen documents, vocabulary checks only.
- **Two narrow reads in the cross-language test.** It scanned TypeScript for
  `": 'l0."` and so was blind to the whole `lifecycle.` family, and it never
  read Go at all — a Go-only or TypeScript-only capability would have passed.
  Now both files, bounded to the block that holds the vocabulary, with a count
  assertion so a read that matches nothing cannot agree with everything.
  Verified by deleting `lifecycle.detach` from the Go source.

### rev 61 — 2026-08-10

- **R3 is implemented**: `wa_wire_l1::dedup::SeenStanzas`. A handoff is
  stop-the-world, inbound survives it because the server queues it, and acks in
  flight do not — the server resends what it could not know was read. R3 calls
  deduplicating those mandatory, and the fourth item of the v2 definition of
  done is now done.
- **Beside `derive`, not inside it.** Telling a redelivery from a first arrival
  is the one thing a single stanza does not carry, and D-010 makes `derive` pure
  precisely so four engines can be compared on it. A stateful `derive` would
  answer differently depending on what it had seen, and nothing could be
  replayed. A caller that does not need deduplication pays nothing.
- **Three answers, not two** (D-137). `New` and `Duplicate`, and `Untracked` for
  a stanza with no `id` — which is most of them, since only `ack`, `receipt` and
  `call` shapes model one. Folding that into `New` would let a caller counting
  duplicates read "could not tell" as "there were none", the same mistake the
  gate refuses when it reports `incomparable`.
- **The key is tag and id together.** An `<ack>` and a `<receipt>` for one
  message both carry that message's id; keying on the id alone would report the
  second as a redelivery of the first and drop it.
- No allocation and no growth: a fixed ring of inline ids, `no_std`, about six
  kilobytes at the default window. An id longer than a slot is `Untracked`
  rather than truncated, because a truncated id collides with every id sharing
  its prefix, and that collision is the one error here that loses a message.
- A duplicate is not re-remembered. Re-inserting it would evict one older entry
  per redelivery, so a burst would empty the window of everything it was there
  to recognise — tested by sending fifty and requiring the four originals to
  survive.

### rev 60 — 2026-08-10

- **`whatsapp-rust` can be a handoff target.** `Client::pause()` and
  `Client::resume()` merged upstream as #1265, and the log line that announced a
  reconnect the shutdown forbade as #1264 — the two halves rev 59 said the
  question would split into, with the reading it could not choose between
  resolved as (A): `disconnect()` is terminal by design, the log was the defect,
  and the missing thing was a detach.
- **Measured rather than assumed: 43 ms**, twenty cycles, p50 42.99 and p95
  44.78, timed from `pause` to a dispatched `Connected`. Six times cheaper than
  the same engine's cold `ready` of 273 ms, which is what a resume skipping
  pairing and first sync should look like.
- `pause`'s doc states the loss in the terms RFC-003 asks for: nothing is
  carried across it that a network drop would not also have taken. That is a
  route cost declared by the engine rather than inferred by us.
- **Two of the four blocked routes are back**, and `zapo` is now the only engine
  that cannot perform a phase — it still has no way to drop its transport, so it
  cannot `detach`. Three routes need it as a source and remain unavailable.

### rev 59 — 2026-08-10

- **Rev 58 said `whatsapp-rust` cannot `attach`. It is more specific than that,
  and the specificity changes what v2 needs.** `disconnect()` writes
  `is_running = false` and fires the shutdown notifier — it is a terminal stop.
  The run loop announces *"reconnecting immediately"* and then announces that it
  has shut down; a later `connect()` completes the handshake and the server
  sends its `<success>`, and nothing decodes it because the reader is gone.
- **What the library lacks is a detach that is not a stop**, which RFC-003's
  phase 3 requires by name and R4 requires to be type-level distinct from
  `logout`. That is a capability to add, not a bug to fix. *(Added upstream as
  `Client::pause`/`resume` in #1265; the sentence that followed here paired it
  with `zapo`'s supposed inability to drop its transport, which rev 63
  withdrew.)*
- Three explanations were eliminated before that one survived, each with
  evidence: the `expected_disconnect` flag left dirty (the symptom persists on
  the commit that clears it), the `Connection` guard dropped undriven (the
  pinned API returns no guard, and driving it on the newer API changes nothing),
  and the caller racing the run loop (removing the caller's `connect()` leaves
  the same two log lines and no new connection).
- `agent_prompts/detach-and-reattach-a-session.md` in `whatsapp-rust` carries
  the reproduction, the eliminated hypotheses, and the one question that decides
  the work — whether `disconnect()` is terminal by design, in which case the log
  line is the defect and the missing capability is the finding. The prompt does
  not answer it, because answering it needs the intent behind an API this
  repository does not own.

### rev 58 — 2026-08-10

v2 opened on the claim that its two unknowns had to be measured before anything
was built. They are measured, and measuring them found three things that
reading could not.

- **The handoff window's engine-side floor is 31 ms to 273 ms**, an 8.7× spread:
  `hypermeow` 31.2, `zapo` 52.9, `Baileys` 156.3, `whatsapp-rust` 273.3, seven
  runs each, within-engine spread about 3 ms. A floor rather than an SLA —
  Barback is local, so every network round trip is excluded.
- **`reconnect` cannot answer that question** (D-135), which is the finding
  behind the finding. It is the scenario that looks like a handoff, and it is
  not comparable: each adapter decides what "back" means, so whatsmeow returns
  when the socket is up and Baileys waits for the open event. Two engines, one
  nominal measurement, 2.5 ms and 16.5 ms. `ready` is the comparable one because
  it times what a host would wait for.
- **The loss matrix is filled in** for all twelve routes, computed by
  `wa-store-migrate`'s own `planLosses` instead of read off its README. Loss is
  a property of the destination: nothing is lost moving into `zapo`, and
  `Baileys ↔ whatsapp-rust` loses two domains and degrades four. That is a
  policy now (D-136), not a table.
- **Two engines cannot perform a phase RFC-003 requires.** `zapo` cannot drop
  its transport, so it cannot `detach`; `whatsapp-rust`'s disconnect-then-connect
  never returns in thirty seconds, so it cannot `attach`. Four of the twelve
  routes are unavailable until one of them changes. Neither was visible from the
  RFCs, and both are engine work rather than host work.
- One correction to my own reading. `senderKeyDistributions` appeared to drop on
  every route; it is neither read nor written by any adapter, so no real source
  can produce it. The universal drop was an artifact of forcing it into a
  synthetic snapshot — it is dead weight in the IR, not a route cost.
- **`wa-store-migrate` does not build from its repository.** `src/adapters/wa-web`
  is imported by the registry and by five test files and is not committed; five
  suites fail on the missing module. The npm package ships it, so a consumer is
  fine and a contributor is not — which matters because D-012 plans a
  differentially verified port of exactly that code.

### rev 57 — 2026-08-09

- **v2 is open, and it is Layer 3** (D-134): the host, and moving a session from
  one engine to another without re-pairing. v1 made four engines agree about
  what arrives; this is what that agreement was for.
- **The design is not the open part.** RFC-003, RFC-004 and RFC-006 were
  accepted in rev 7 and nothing in fifty revisions has disturbed them. Handoff
  is stop-the-world because one device gets one connection; the host never owns
  the store; `detach` is not `logout`, at the type level. v2 implements what
  those already decided.
- **It opens with an experiment.** Two numbers are marked `[UNKNOWN]` in the
  RFCs that need them — the unavailability window per engine pair (OQ-7) and the
  loss per route (R2) — and both are the kind this project will not guess. R2's
  promise is "loss is known, declared and accepted"; today nothing is known,
  which makes the promise unsayable rather than merely unproven. `whatsapp-bench`
  has the harness already.
- **The largest risk is named where it can be seen.** v2's `snapshot` phase runs
  on `wa-store-migrate`, which belongs to someone else. v1's two open engine PRs
  are at the edges; this one is at the centre, and OQ-3 records that what remains
  there is a conversation rather than a design.

- **The seven crates are published.** `wa-wire-contract` 0.1.2 and the other six
  at 0.1.0, all seven built on docs.rs. Verified the way a stranger would: a new
  crate with no `path` and no workspace, depending on the registry alone,
  resolves and compiles. That is the difference the definition of done could not
  state before — every criterion was met while the thing itself was a repository.
- The README carried the old world: one crate marked published and the rest not,
  and no way in. It now lists each version against the registry, opens with the
  two lines that get a stanza to a typed event, and says which two adapters need
  an engine change first.
- **What publication did not settle** is worth keeping in view rather than
  celebrating past. `hypermeow` and Baileys still read their hooks from an open
  PR, so the central claim — four engines, one corpus, the same events — is
  reproducible here and nowhere else. Nothing in this repository can close that.

### rev 56 — 2026-08-09

- **The other six crates are prepared for publication.** Contract 0.1.2 goes
  with them: its README said one of the ten capabilities had no provider, and
  that stopped being true in rev 55. All ten have one now, though no adapter has
  all ten — which is the matrix rather than a shortfall.
- **Two packages were shipping things nobody installing them wants.**
  `wa-wire-l1` carried 4MB of vendored `whatspec` JSON, which is generator input:
  the derivation ships as the generated Rust beside it, and nothing at build or
  run time opens the spec. `wa-wire-conformance` carried the corpus, the frozen
  recordings and every integration test — and each of those tests reads a file,
  including fixtures from adapters that are not in the package at all. Shipping
  the tests without their data would publish a crate whose `cargo test` fails.
  Both are excluded, and `wa-wire-conformance` went from 54 files to 12.
- **The last two cannot be verified before the first four are up.** `cargo
  package` resolves dependencies through the index, so `wa-wire-l1` cannot be
  checked until `wa-wire-codec` exists there, and `wa-wire-conformance` until
  `wa-wire-l1` does. Inherent to publishing a graph; the order handles it, and
  each `cargo publish` verifies itself when its turn comes.

### rev 55 — 2026-08-09

- **`l0.plaintext.cause` has a provider.** It was the one capability frozen into
  contract version 1 with no implementation anywhere, and the published README
  says so. `Event::EncDecryptFailed` landed upstream, so the `whatsapp-rust`
  adapter now reports why an `<enc>` produced nothing instead of only that it
  did not.
- **Fifteen reasons into three statuses, and the narrowing is where it is
  interesting.** Attempted-and-failed is `DecryptFailed`; recognised-but-
  undecryptable is `Unsupported`; and `NotAttempted` — the engine could have
  tried and chose not to — has no frozen status that says it, so it reports as
  `Unobserved`. That is true and says less than the engine knew, and naming it
  properly would be a version 2 change. A test walks all fifteen and requires
  each side of the engine's own `decryption_was_attempted` line to land on the
  matching side here, so a reason added upstream cannot drift into the wrong
  half through the catch-all arm.
- **A cause never displaces a plaintext.** The engine reports both for one
  `<enc>` when the bytes existed and would not parse, and the bytes are the more
  useful half: a consumer holding them can decide for itself.
- **The narrow-read sweep found two more.** Reading a value while assuming one
  encoding is the class that produced the `@newsletter` defect, and the sweep is
  now a test rather than a reading: the same stanza written every way the format
  allows must derive one event, compared with `semantic_eq` because the frames
  necessarily differ.
- `Fixture::device_jid_attr` wrote `SERVER_PN_TAG` as the domain-type byte.
  That constant is the *token index* for `s.whatsapp.net`, correct three lines
  above where the pair form takes a token, and domain type 1 is `lid` — so every
  fixture built with it carried a `@lid` device JID while its test read a phone
  number. No assertion depended on the server, which is why 180 tests passed
  over it.
- The builder could not write a hex-packed value at all: `packed_attr` filters
  to the nibble alphabet and **drops** what it cannot encode, so a hexadecimal
  message id came out as the digits it happened to contain. `hex_attr` writes
  the form WhatsApp actually uses for ids, and both now share one packer.

### rev 54 — 2026-08-09

- **Rev 53 said three engines misparse an interop JID. They do not, and the
  claim is withdrawn.** It rested on reading the client's *writer*
  (`WA/Wap.js`, `ne()`), which emits `tag, user, u16 device, u16 integrator`
  and stops. The client's *reader* is `be()` in the same file, and it takes
  user, device, integrator and then a string it discards. Both are the client;
  the two directions of this JID genuinely differ, and `whatsapp-rust`'s own
  marshal tests say so in as many words. For inbound traffic the reader is the
  authority, so the trailing server token is there and consuming it is correct.
  `whatsmeow`, `zapo`, Baileys, `whatsapp-rust` and rev 52's `wa-wire-codec`
  all agree; rev 53 broke ours to match a writer it should not have been
  reading. Reverted.
- The `hypermeow` failure said `expected "interop", got "type"` — the decoder
  *validating* the token it had just read. That was the answer, and it was
  read as the symptom.
- **Chasing it did find a real defect, ours alone.** A JID's user is read with
  the general value reader; its **server** was read as a dictionary token only.
  `newsletter`, `bot`, `interop` and `hosted.lid` are in none of the five
  dictionaries, so each arrives spelled out — and a `@newsletter` JID, which is
  ordinary current traffic, could not be parsed at all. The client reads that
  position with `decodeString`, which takes either, and so does every other
  engine.
- **A test had that backwards and locked it in.** `a_jid_server_must_be_a_token`
  used a spelled-out server as the *counterexample* and asserted the parse
  fails. It now asserts `@newsletter` parses, with a second test for a server
  that is not text at all.
- The corpus carries a `@newsletter` message rather than an interop one. An
  interop frame cannot represent inbound traffic here, because it is generated
  by an engine's writer and no writer emits the inbound form — which is the
  same asymmetry, met from the other side.

### rev 53 — 2026-08-09

- **Rev 52 widened the corpus by what the derivation reads; this widens it by
  what an encoder writes.** Agreement between four engines is only informative
  where they could disagree, and three of the four forward the corpus bytes
  untouched — so what the comparison reads is each engine's *re-encoding*, and
  an encoder only chooses where the protocol admits a choice. A corpus of plain
  ASCII attributes asks nothing.
- `tests/corpus_encodings.rs` counts **encodings rather than stanzas**: token
  against spelled-out, packed nibbles, packed hexadecimal, the four JID forms,
  a body past the one-byte length and a child list past the one-byte count.
  Six were already present; four were not.
- **Writing one of them found a decoder defect in three of the four engines.**
  An interop JID is `tag, user, u16 device, u16 integrator` and stops
  (`WA/Wap.js`, the `JID_INTEROP` arm; the Messenger arm beside it is the one
  that writes a trailing server token). `wa-wire-codec` consumed a server token
  that is not there, and so do `whatsmeow` and `zapo`; Baileys reads one
  speculatively and rewinds only if it throws, which did not save it here
  because the next byte was a valid token. All four swallow the following
  attribute's key and desynchronise the rest of the frame.
- **Ours is fixed, with the client cited.** The test that covered it had been
  written against the same wrong layout — a frame ending at the JID, where
  reading one byte too many costs nothing. It now carries an attribute after
  the JID, so the same mistake is a parse failure.
- **The frame is kept, in `corpus/blocked/`.** A replayed corpus needs every
  engine to read every frame; this one three cannot. The replays walk the
  corpus root and `captured/` and skip directories, so it stays out of the
  agreement run and in the repository, where a fix has something to be checked
  against. `corpus_encodings` reads both directories: the encoding is still one
  this project must handle.

### rev 52 — 2026-08-09

- **The four engines had only ever been asked to agree about five of the
  sixteen shapes the derivation models.** Agreement is as wide as the corpus,
  and the corpus reached `IncomingMsgParser`, `IncomingMsgReceiptParser`,
  `CallParser`, `SendMsgAckSyncParser` and `ParsePublishViewResponseSuccess`.
  The other eleven were exercised only by generated unit tests, which are
  written against one implementation and so cannot disagree with anything.
- **Twelve stanzas were added and fifteen shapes are now reached**, each the
  leanest stanza that falls to its shape rather than to a richer sibling.
  `tests/corpus_coverage.rs` asserts it, so a spec refresh that adds a shape
  arrives as a failure naming it.
- **Widening the corpus found an ordering defect, which is the better find.**
  D-041 orders a tag's shapes most-specific first, and the shape-level sort key
  was `(guards, total fields)` — total, not required. `CallParser` demands two
  attributes and mentions seven; `CallOfferNoticeParser` demands four. The
  wide-but-lax shape sorted first and took every `<call>`, so
  `CallOfferNoticeParser` could never derive. The mixin-variant sort right
  beside it (D-107) had the rule right — `(guards, required, total)` — and the
  shape sort was missing the middle term. Fixed, and the two now agree.
- **One shape is unreachable and is now declared** rather than silently missing.
  `ParseNewsletterResponseSuccess` demands `t` through both alternatives of its
  union, and `SendMsgAckSyncParser` demands `t` and nothing else, so it is tried
  first and always matches. The outgoing generator has had `UNREACHABLE_OUTGOING`
  since rev 33; the incoming side had no equivalent.
- **Declared rather than computed, on purpose.** A subset test over required
  fields would also flag `CallReceiptParser`, which shares its required pair
  with `IncomingMsgReceiptParser` and is reachable anyway: a `type` outside the
  message-receipt enum makes the earlier shape reject. Reachability turns on
  what a field accepts, not only on whether it is demanded, and a heuristic that
  missed that would excuse a real gap.
- **A change that was reverted, because the fix was worse than the gap.**
  Hoisting literals pinned by every alternative of a required union into the
  shape's guards would have made `ParseNewsletterResponseSuccess` reachable —
  and would also have reclassified every ordinary `<ack class="message" t="…">`
  as a newsletter response. The real client separates those by which request the
  response answers, which `derive` cannot see (D-010). Reaching one obscure
  shape is not worth misreading the most common ack.

### rev 51 — 2026-08-09

- **The freshness guard rev 49 introduced could be walked past.** The emitter
  checked that each engine's replay had the right *number* of stanzas and then
  stamped today's corpus digest onto it. A replay left over from a corpus that
  changed without changing its file count passed that check and was written out
  looking current — the staleness the digest exists to catch, laundered by the
  tool meant to catch it. Each stream is now compared against the corpus itself
  before it is written: an engine's re-encoding may differ in bytes and not in
  what it derives, so a stream that derives something else is a replay of
  something else. Verified by swapping two of `hypermeow`'s envelopes and
  watching the emitter refuse.
- **The frozen recordings named no dictionary**, so the comparison read them
  with whatever table this build carries. A token table that moves after a
  freeze would put the same wrong tokens on both sides and agree. Each recording
  now records the table it was written against, and a build carrying a different
  one refuses rather than compares.
- **Comparability is read out of each file rather than asserted over both.**
  `Comparability::declared` states that a recording is whole, carries no unknown
  critical tag and skipped no record — three things a file says about itself.
  A future recording with an unrecognised record kind would have had those
  records dropped and the comparison would have passed on the part it did read.
- **The inspector was quiet about what it did not read.** A recording holding
  only an unknown record kind reported "complete, 0 envelopes", which is what an
  empty one reports. Skipped records and unknown critical tags are now counted
  in the summary.
- **`adapter` is a critical tag, and the code said it was not.** A present but
  unparseable declaration reported as "undeclared", which is a different finding
  and not a corrupt file. The two are now separate.
- **A recording is not trusted, which is the point of opening one.** Every value
  printed from a file is escaped: a stanza id carrying a newline forged a line
  of the report, and one carrying `ESC [` handed the terminal a command rather
  than a character. Both are valid UTF-8, and a tool for looking at hostile
  files was rendering them verbatim.

### rev 50 — 2026-08-09

- **The tenth capability's justification was wrong, and it was mine.** The
  matrix said no adapter reports why an `<enc>` produced nothing because the
  engines never say. `whatsapp-rust` does say: `Event::UndecryptableMessage`
  has existed all along.
- **It still cannot serve** (D-133), for reasons worth writing down so nobody
  investigates this twice. It is dispatched per message rather than per `<enc>`,
  so a fan-out stanza's failures cannot be attributed. It is deduplicated per
  `(chat, id)` and suppressed on resends, so the second arrival produces no
  event. And `decrypt_fail_mode` is the server's `show`/`hide` display hint, not
  a cause. Wired into a plaintext entry it would give a `DecryptFailed` that is
  right sometimes and silently missing otherwise, which is worse than the
  uniform `Unobserved` today — a gate can act on a status that is always honest.
- **What would close it** is the symmetric counterpart of
  `Event::DecryptedPayload`: a failure event carrying `enc_index` and a cause,
  from the same loop, which already has `enc_index` in scope. Every failure
  branch needs one, so it is an engine change with its own review rather than
  something an adapter can arrange.

### rev 49 — 2026-08-09

- **`wa-wire-inspect`**, a second binary beside the gate. The format is
  published and frozen, and until now the only way to open a `.wawr` was to
  write Rust against `wa-wire-recording`. A format nobody can open is a format
  nobody can check, which is the argument that put the gate beside the
  comparator in the first place. It reports what the file *says*, including
  where that disagrees with itself: a trailer whose count does not match, bytes
  appended after it, a dictionary this build does not carry, a capability
  identifier from a newer adapter. A reader opens a recording precisely when
  something is wrong with it, so resolving those away would hide the reason
  they looked.
- **The four-engine agreement now runs in CI.** Producing four streams needs
  four engines checked out at once, which is why it was manual; *comparing*
  them needs four byte streams and a token table. Each engine's re-encoded
  stream is frozen into a recording (16KB for all four) and
  `wa-wire-conformance` compares all six pairs on every push.
- **What that catches is our half.** A change to `wa-wire-l1` or the codec that
  makes four engines stop agreeing was, until today, caught by nobody until
  someone ran the manual command. What it cannot catch is an engine moving,
  since a committed recording is a photograph — that still needs the live run.
  The corpus digest travels inside each file, so a recording of traffic that has
  since changed is refused rather than passed. Verified by adding a corpus file
  and watching the run fail with the command to fix it.
- **A green comparison is not evidence that the comparison could fail.** One
  test hands the comparator a stream with two stanzas out of order and requires
  it to object, on this corpus, with this data. Without it, four identical
  streams would satisfy every other assertion in the file.

### rev 48 — 2026-08-09

- **`tools/check-docs.py`**, wired into CI. The same defect appeared three times
  in one day in three different files — a count of capabilities the code had
  moved past — and one of them reached crates.io, where a published README
  cannot be edited. Every instance was true when written and every one was
  caught by a person re-reading. The checks are the subset of that prose a
  machine can settle: capability names and counts against `capability.rs`, RFC
  cross-references, internal anchors, `path:line` citations, and whether the
  published version is mentioned at all. Each was verified by breaking the
  document and watching it fail.
- **It immediately found what it was written for.** RFC-002 still said
  `Capability::ALL` has eight members and that the contract "does not name this
  capability yet" — it names it, as `l0.outbound.observed`, and the freeze made
  that ten. The matrix still called the Baileys hooks local; they are
  [#2762](https://github.com/WhiskeySockets/Baileys/pull/2762). RFC-008 and
  RFC-009 did not say they had been published, which for the two RFCs the
  freeze is about is the fact most worth stating.
- **Citations into an engine are checked against that engine's release branch**,
  not the working tree. The first run read our own PR branch and called two
  correct citations stale. One was genuinely stale: `Unmarshal` in `hypermeow`
  moved to `client.go:876-882` while the document still said 824-830.
- Three heuristics were wrong before they were right, and the failures are the
  interesting part. Pairing every backticked snippet on a line with every
  citation on it read one engine's evidence against another's file; a citation
  belongs to the snippet immediately before it. Matching a snippet verbatim
  missed `backing_bytes()` against `fn backing_bytes(&self)`; identifiers work.
  And naming one default branch per repository failed on Baileys, whose
  `origin/HEAD` still points at a `master` from before the monorepo layout.
- The check also had to learn that a changelog is allowed to be wrong. Quoting
  the old count in the entry above tripped it, so what a revision asserts is
  now read separately from what it records having fixed.

### rev 47 — 2026-08-08

- **The Baileys hooks are upstream** as
  [WhiskeySockets/Baileys#2762](https://github.com/WhiskeySockets/Baileys/pull/2762),
  against `develop`. Both engine-side dependencies are now open PRs rather than
  one PR and one working tree, which is the difference between a fourth adapter
  someone else can build and one only this machine can.
- **Reviewing the patch for upstreaming found a copy I had added.**
  `transport.decrypt` already returns a `Buffer`, so the `Buffer.from(result)`
  wrapping it copied every inbound frame, whether or not anyone was observing.
  Measured at +15% and +2KB per frame; removed. `decodeBinaryNodeWithBuffer`
  itself is within 1% of `decodeBinaryNode`, which is what it should be, since
  it does the same work and returns an intermediate that already existed.
- The first benchmark said the new function was 55% *faster*, which is not
  something a function doing identical work can be. It was V8 tier-up
  penalising whichever case ran first. Warming every case before measuring any,
  then taking a median over alternating rounds, gave -0.7%. Worth remembering
  the next time a number here looks good.
- The patch went from 153 lines to 95: a 28-line helper folded into its call
  site, a duplicated type signature replaced by an import, and the commentary
  cut to what a reader of that file would need.

### rev 46 — 2026-08-08

- **The published README said `l0.outbound.observed` had no provider. It has
  one.** `adapters/whatsapp-rust` declares it and has since the capability was
  named — `Event::SentFrame` is subscribed, unpacked and forwarded. Only
  `l0.plaintext.cause` is a name without an implementation.
- The claim survived because **nothing tested it**. `CAPABILITIES` documents
  itself as asserted entry by entry, and the declaration test checked five of
  the six: the one added most recently was the one nobody wrote an assertion
  for. Now asserted, which is what should have caught this instead of a reading.
- Two more statements had gone stale against the code they describe. The
  `whatsapp-rust` README still said the contract had eight capabilities and that
  naming a ninth for `SentFrame` was "not yet taken" — taken as D-102, and the
  adapter's own capability table was missing the row. The Baileys joiner still
  said `zapo` and `whatsapp-rust` reorder, which rev 42 fixed.
- The shape is the one this project keeps finding: **prose about a capability is
  not a test of it.** Every one of the three was a sentence describing code that
  had moved underneath it, and the fix for the first was an assertion.

### rev 45 — 2026-08-08

- **`wa-wire-contract` 0.1.0 is published**, and the definition of done is
  closed. Contract version 1 is fixed: the envelope layout, the ten capability
  identifiers, and what every field an envelope carries means.
- **Fixed is not final** (D-132), and the difference is worth stating where
  someone depending on this will read it. Additive change stays inside version
  1 — new capability names, new reserved flag bits, new metadata tags — because
  the format was built to carry what a reader cannot resolve: a recording
  declares capabilities by name and preserves the unknown ones, and a metadata
  tag says with its critical bit whether skipping it is safe. Version 2 is for
  the three things a reader cannot survive by ignoring: a field that moved, a
  field that changed meaning, a field that went away.
- The crate gained the manifest a listing needs — keywords, categories, a
  README link — and nothing else. It has no dependencies, and packaging it
  builds it outside the workspace, which is the only real check that a consumer
  can use it at all.

### rev 44 — 2026-08-08

Audit of the capability vocabulary, publication being the point after which
adding one is a contract version bump.

- **One capability was missing, and the format already knew it** (D-130).
  `PlaintextStatus` has carried `DecryptFailed` and `Unsupported` since RFC-008
  and no adapter has ever emitted either: all four watch payloads appear and
  are never told why one did not, so every entry says `Unobserved`.
  - That absence is the failure mode this project keeps running into. Under
    `Unobserved`, a candidate build whose messages stopped decrypting looks
    exactly like one whose adapter stopped observing — the failure and the
    blind spot are the same silence, and the gate cannot separate them.
  - `l0.plaintext.cause` names it, and `verify` refuses a cause from an adapter
    that has not declared it. Nothing provides it yet, which is the point:
    naming it now costs a line and naming it later costs a version.
- **Two matrix rows are deliberately not capabilities** (D-131). *Plugin host*
  and *runtime portability* are facts about an engine rather than promises
  about what crosses, and `require` could never usefully check either.
- **The matrix's Baileys column was two revisions stale**: per-`<enc>`
  plaintext and zero-copy frames both landed with the adapter in rev 41.
- Nothing else came up. Takeover partiality is not a second capability — the
  five stanzas `whatsapp-rust` withholds are the ones no engine can safely hand
  over, so "complete takeover" would name something nobody should ask for.
  Media, session handoff and the Layer 3 host are out of v1 by RFC.

### rev 43 — 2026-08-08

- **The central claim is a four-engine test result.** One corpus, replayed
  through every engine in its own process, compared pairwise across all six
  pairs. Item 4 of the definition of done closes, leaving only publishing
  `wa-wire-contract`.
- **It compares what each engine re-encodes** (D-128), which is the difference
  between a result and a formality. Three of the four adapters are zero-copy
  and forward the corpus bytes untouched: comparing those is comparing three
  identical byte streams, which agree by construction. Each engine's own
  encoder is where four implementations genuinely differ — `hypermeow` and
  Baileys write different bytes for five of the fourteen corpus stanzas — and
  that the derivation matches anyway is the property.
  - Checked by corrupting one engine's re-encoding, which the test catches and
    names the disagreeing pair for.
- Two new replay commands, one per out-of-process engine, writing envelopes as
  files rather than containers (D-129).
- **What four engines have not yet found.** Every finding so far has come from
  real captured traffic meeting the derivation, and not one from two engines
  disagreeing. That was the argument for a third and fourth; the argument is
  not yet paid off, and saying so is more useful than the number four.

### rev 42 — 2026-08-08

- **The four joiners now agree about ordering** (D-126). `zapo` and
  `whatsapp-rust` emitted an unheld stanza the moment it arrived, so an ack
  overtook a message that came first; the Go and Baileys ones queued. All four
  queue now and drain from the front.
  - This was reported in rev 41 and not fixed, on the grounds that changing two
    working adapters is its own change. It is, and this is it.
  - The cost is real and worth naming: a held message delays every stanza
    behind it, so an adapter's output is later than it was. What it buys is a
    recording whose order is the wire's rather than the engine's timing —
    without which the comparison reports a divergence whenever two engines
    interleave differently, which is a finding about scheduling dressed as a
    finding about behaviour.
- **`pending` and `queued` are separate counts** (D-127), since they stopped
  being the same number.
- **A test reads all four joiners and requires each to state the rule** and to
  have something that drains a queue. Crude, and the only check that spans four
  languages without running four engines. What it catches is the rule being
  dropped in a rewrite, which is how the two came to disagree: one was written
  before the other understood the problem.
- A smaller thing the change surfaced: `zapo` was emitting `plaintexts: []` for
  a stanza with no `<enc>` where it used to emit nothing. An empty table and no
  table are different claims — "nothing decrypted" against "nothing was
  encrypted" — and a reader should not have to infer which.

### rev 41 — 2026-08-08

- **The fourth engine.** `Baileys` needed two observation points and had
  neither: the buffer a node was decoded from fell out of scope in
  `processData`, and nothing carried a plaintext outside the parse that
  consumed it. Both are proposed in
  [WhiskeySockets/Baileys#2762](https://github.com/WhiskeySockets/Baileys/pull/2762),
  open as of rev 47.
  - `decodeBinaryNodeWithBuffer` is the whole of the first: the same work
    `decodeBinaryNode` does, handing back the decompressed bytes as well.
  - The second fires **before the protobuf is parsed**, and fires for a payload
    whose padding will not strip — the defect the `hypermeow` review found one
    layer deeper in its engine, avoided here by having seen it there.
  - The hook reports the payload's **child** index (D-125), which the other two
    engines do not, so this adapter has no fan-out case to give up on.
- **The format was not written a fourth time** (D-123). Baileys is the second
  TypeScript engine, so `@oxidezap/wa-wire-ts` was extracted out of the `zapo`
  adapter and both now share it. Three writings is three *languages*, which is
  the number an adapter's home imposes; a fourth in a language that already has
  one would be a description nobody checks against the others.
  - Extracting it surfaced that the module carried `zapo`'s own declaration and
    a `has()` closing over it (D-124) — a shape that reads correctly while
    there is one adapter and silently means the wrong thing once there are two.
  - It also surfaced that the TypeScript side had no `verify`, which the Rust
    and Go adapters both have. Added to the shared package, so both TypeScript
    adapters now check an envelope against their own declaration.
- **The three joiners do not agree on ordering, and this is the second one to
  notice.** `zapo` and `whatsapp-rust` emit an unheld stanza the moment it
  arrives, which puts an ack ahead of a message that came first; the Go adapter
  and this one queue and emit in arrival order. A recording compared position by
  position reports the difference as a divergence in whichever engine was
  slower. Recorded rather than fixed here: changing two working adapters is its
  own change.

### rev 40 — 2026-08-08

Review of the last two revisions. Almost all of it was real, and the gate could
return the wrong verdict five ways.

- **A recording could lose its entire inbound half and pass.** The exemption
  that keeps an observer's limited reach from being blamed on its engine was
  written for the outbound sequence and applied to both, so a candidate with no
  inbound stanzas at all produced no divergences. Introduced in rev 36 and
  fixed here.
- **Bytes after the trailer read as `Complete`.** The checksum covers what
  precedes the trailer, which is everything the trailer knew about, so appended
  records leave the count right, the checksum right and the file wrong. A new
  `Integrity::TrailingBytes` names it, and makes such a recording incomparable
  since `whole` already requires `Complete`.
- **Three more ways a comparison ran on nothing:** a recording written under a
  later contract version was read as this one, because `AdapterInfo` is rebuilt
  at the current version and nothing consulted the file's; a skipped record —
  the container's own escape hatch for a kind a reader does not know — was
  counted and then ignored; and two recordings that both declared *no* artifact
  class were compared as though two absences were an agreement, as were two
  `Sanitized` ones naming no transform.
- **Two in the protobuf reader.** The tenth byte of a varint carries bit 63 and
  no more, so nine `0x80`s and a `0x02` were accepted as *zero* — a malformed
  varint read as a value. And an end-group was checked only at depth zero, so
  `group 1 { group 2 { end 3 } end 1 }` balanced with the mismatch unremarked.
- **Four in the Go adapter**, three of them from the two hooks running on
  different goroutines — something the other two adapters do not have to
  contend with:
  - Deliveries to the sink could overlap, since both hooks reached it after
    releasing the joiner's lock.
  - The lookahead aged by frames received while payloads arrive from behind a
    256-deep queue, so a message could be given up on while its plaintext was
    still queued. The default is now larger than that queue, and says why.
  - A stanza waiting on payloads let the ones behind it overtake it. The queue
    now drains in arrival order, which is also the order a recording is
    compared in.
  - `TakeoverInfo` promised `l0.plaintext`, and a claimed stanza is never
    decrypted — `Require(Takeover, L0Plaintext)` succeeded for a combination
    the engine cannot produce.
- **The format moved into its own Go package.** The fixtures are the only check
  on that encoder, and CI could not regenerate them while the command pulled in
  the engine. `wire/` imports nothing but the standard library, so CI now
  regenerates and requires no diff — reading committed files proves the reader
  agrees with what was committed, not that the writer still produces it.
- Smaller: `derive_all` handed outbound envelopes back as inbound events; an
  interop JID written as text lost its integrator, so the two spellings of one
  identity derived differently; and `capture-corpus` left a previous run's
  files in the directory for the emitter to sweep up.

### rev 39 — 2026-08-08

- **Rev 38 claimed a defect in the shared design, and there was none.** Writing
  the Go joiner, the lookahead did not count stanzas that crossed straight
  through, so a receive path carrying nothing but acks would have held a
  message for ever. I fixed it and wrote that the third implementation had
  exposed a flaw the other two would share.
  - It had not. Both age their pending stanzas *before* deciding whether the
    new one is holdable — the TypeScript one says so in as many words — and
    both have a test that ages with `<receipt>` stanzas, which is precisely the
    case, so either would have failed had it been wrong.
  - The defect was mine, in the new implementation, and my own test caught it.
    That test caught it because it was written in the shape of the Rust one:
    the value came from copying an existing test's design, not from a third
    implementation revealing anything.
  - Left as a correction rather than deleted, because the claim sent a reader
    to audit two adapters that are fine, and credited an exercise with a
    finding it did not make. A third implementation is worth having for the
    reasons in D-121; this was not one of them.

### rev 38 — 2026-08-08

- **The third engine, and the first the core cannot reach.** `whatsapp-rust`
  links the Rust natively and `zapo` runs it through WebAssembly; Go can do
  neither, so the boundary format is written out a third time, in Go (D-121).
  - That is not duplication for its own sake — it is the case the design was
    made for. An adapter runs inside its engine, and this one is written by
    someone who could not have used our code even if they wanted to.
  - The Go encoder has no Rust to check itself against, so it emits fixtures
    that `cargo test -p wa-wire-conformance` reads. The check found nothing on
    the first run and catches an endianness inversion when one is introduced.
- **The engine gives this adapter something the other two lack.** Both of them
  are told which `<enc>` of a stanza decrypted, counting `<enc>` nodes, and
  have to resolve that to a child index — ambiguous the moment a stanza carries
  anything else, and unresolvable for a fan-out `<message>`, so both emit those
  as L0-wire rather than risk attaching a plaintext to the wrong node.
  `hypermeow` reports the child index directly and nothing is inferred. The
  hook was written against that need, which is the advantage of contributing
  the observation point rather than working around one.
- **A defect in the third implementation, corrected in rev 39.** The claim
  first written here — that it exposed something in the shared design — was
  wrong; see rev 39.
- **D-022 turned out not to be needed** (D-122). The adapter was set aside as
  an MPL-2.0 subdirectory on the expectation of carrying patched `whatsmeow`
  files; it carries none, since the hooks went upstream where they are MPL-2.0
  already. `NOTICE.md` says so and says what would change it.

### rev 37 — 2026-08-08

- **The definition of done was two items behind the work.** Item 3 still said
  the payload derivation was written "because whatspec has no oracle for it",
  which rev 28 reversed; item 6 still called `whatsapp-rust` takeover a patch,
  which rev 31 recorded as upstream. Both are done, and the list now says so —
  a checklist that undercounts what is finished is as misleading as one that
  overcounts.
- Of the six, **two remain**: publishing `wa-wire-contract`, and the third and
  fourth engines. They are the same item twice over, since a frozen contract is
  what a new adapter is written against.

### rev 36 — 2026-08-08

Review of the two previous revisions, and most of what it found was real.

- **Three bugs in the derivation, all in the mixin work and all of a kind.**
  - **`sourcePath` was ignored** (D-115), by the whole generator rather than
    only the new part — it simply had no fields using it until the mixins
    arrived. Thirty-four do, and every one of them read off the wrong node: a
    real ack carrying `<biz paid_convo_id=…>` derived as an *empty* paid group
    conversation and lost the data. The fixtures were built the same wrong way
    and agreed, which is the third time that pairing has hidden a defect — the
    other two were `wireName` and the enum-without-variants. The test is
    hand-written for that reason.
  - **An optional mixin made its children required** (D-116). Six of them sit
    in `NewsletterMessageAck`, so an ack carrying only `class` and `t` did not
    derive.
  - **A mixin group about a child was never absent** (D-117), so every ack
    carried an empty paid-conversation record. An absence reported as a
    presence is the reading a consumer cannot recover from.
- **The comparator aligned two directions by one index** (D-118). `SentFrame`
  comes from the send path and `RawNode` from the read path, with no ordering
  between them, so the same traffic replayed twice interleaves differently and
  the comparison would report a divergence per stanza. Each direction is now
  its own sequence.
- **A recording without an outbound half was being read as one missing
  stanzas** (D-119). Only one engine can report what it sent; the declared
  capability decides whether a difference is the engines' or the observers'.
  And a recording carrying a direction its manifest does not claim is now a
  fault in itself (D-120).
- **Smaller, and all real:** `derive_all` handed back outbound stanzas as
  inbound events; `content_string` accepted bytes that are not text and let
  them render later with replacements; `capture-corpus` would have written the
  client's own sends into a corpus replayed as though the server had said them;
  the TypeScript capability vocabulary had eight of nine, and nothing tied the
  two lists together until now; and the violation message named the capability
  for *sending* where the check wants the one for *observing*.

### rev 35 — 2026-08-08

- **The vendored spec was refreshed and a fold came undone by itself.**
  [whatspec#43](https://github.com/oxidezap/whatspec/pull/43) merged, so
  `type: CUSTOM_STRING("offer_notice")` is a pinned value rather than "a string
  attribute", and the two `<ack class="call">` builders are two shapes again:
  207 where there were 206, three folds where there were four.
  - Nothing here changed to make that happen, which is what D-114 was for. The
    generator recomputes the fold from the spec, so the answer tracks what the
    spec can express rather than what it could express when someone last wrote
    a list down.

### rev 34 — 2026-08-08

- **The four unreachable shapes were not four shapes.** Rev 33 recorded them as
  a fact about the spec: pairs differing in nothing a reader can see, named
  rather than reordered around. Reading the bundle rather than the IR showed
  two different things wearing one description.
  - **Three are one stanza described twice** (D-113). `WAWebHandleGrowthNotification`
    and `WAWebHandleBotProfileNotification` both build
    `<ack class="notification" id to type>`; one passes `type` through
    `CUSTOM_STRING`, the other does not, and neither difference reaches the
    wire. `WAWebReceiptAck` models an optional `participant` that
    `WAWebHandleVoipCallReceipt` leaves out. They are folded, richer one
    surviving, and `MERGED_OUTGOING` says which folded into which.
  - **One was a loss in whatspec.** `WAWebHandleVoipOfferNotice` writes
    `type: CUSTOM_STRING("offer_notice")` — a literal, in the bundle, dropped by
    the extractor because a literal reaching the builder through the wire helper
    fell through to "a string attribute". A bare literal in the same position
    was already a pin, and `CUSTOM_STRING(enum)` already resolved.
    [whatspec#43](https://github.com/oxidezap/whatspec/pull/43) closes it;
    twelve attributes across two domains gain their value.
- **The fold is computed, not listed** (D-114), so it undoes itself when the
  spec grows a discriminator. That is not hypothetical: the `offer_notice` pair
  separates the moment whatspec#43 lands and the vendored spec is refreshed.
- **`UNREACHABLE_OUTGOING` is empty**, and stays as a constant because the
  situation it names is possible. A shape strictly subsumed by another — rather
  than mutually indistinguishable — would be a type nothing can reach, and
  silence about that is worse than an empty list.
- Worth recording about the upstream loss: **nothing counted it**. The extractor
  believed it had read the attribute, so `dropsByReason` was unmoved and the
  floor guards saw nothing. Unlike the enum gap in rev 30, which its own
  diagnostics led me to, this one needed a consumer downstream noticing two
  shapes it could not tell apart.

### rev 33 — 2026-08-08

- **D-106 is reversed, one revision after it was written.** It said outbound
  stanzas could not be derived because whatspec's request-side domains were not
  consumed by any generator here. True, and the wrong conclusion to draw: the
  domains describe 210 outbound shapes between them, and not reading a
  description is not the same as not having one.
  - `stanza/index.json` is 179 records, every one `direction: outgoing` —
    acks, receipts, messages, presence, the high-volume traffic. `iq`'s
    `request` half describes 137 more once duplicates are folded.
  - `srvreq` looked like the third source and is not: those are stanzas the
    *server* initiates and the client answers, which is more inbound traffic
    wearing a name that suggests otherwise.
- **A second generator, deliberately** (D-110). `tools/generate-outgoing.py`
  reads builders where `generate-l1.py` reads parsers, and the two vocabularies
  do not meet: a builder's attribute says how the sender *produces* the value —
  `const`, `dynamic`, `generated_id` — where a parser's says which accessor a
  reader calls.
- **Four things the work found, none of which were the thing it set out to do:**
  - A `const` on a *child* is a discriminator dispatch cannot see, because
    dispatch guards on the stanza's own attributes. Two `abt get` requests
    differ only in what their `<props>` child pins, and the shape that ignored
    the pin claimed the other's stanzas. Child structs enforce their own pins;
    top-level ones do not, because there dispatch has already tested them and
    the check would be five hundred branches only a bug could reach (D-030).
  - **JID flavours were collapsed** (D-111). Three pairs of shapes differ in
    nothing else. `attr_user_jid`, `attr_device_jid` and `attr_group_jid` now
    hold them apart — a device JID carries a device part, a group JID lives on
    `g.us`, which is entry 45 of the token dictionary rather than this crate's
    guess.
  - **Ordering had to become tree-aware.** `SetReadReceiptJob` is
    `SetPrivacyJob` with `category/@name` pinned two levels down, and a
    specificity that counted only the top level saw two shapes with one child
    each and picked the wrong one.
  - **Four shapes can never be derived as themselves** (D-112), and the
    generator works that out rather than being told. It simulates its own
    dispatch over each shape's fixture and reports which earlier shape claims
    it.
- **The generated tests are table-driven, and that is a coverage decision
  rather than a style one.** An assertion's failure path is a region a passing
  test never enters, so 210 tests carrying a formatted message each contribute
  some 550 regions reachable only by breaking the build. Three loops over one
  table have three.

### rev 32 — 2026-08-08

- **The vendored spec was refreshed and `UNTYPED_FIELDS` emptied without a line
  changing here.** Its one entry was an `attrEnum` whatspec declared with no
  variants; [whatspec#42](https://github.com/oxidezap/whatspec/pull/42) found
  the cause upstream — the extractor refused a numeric property key, and
  `{0:"0",1:"1",7:"7"}` is how that enum is written — and recovered 33 more
  constraints in its other domains on the way. Reporting what a generator could
  not express, rather than dropping it, is what led there.
- **A recording can hold both halves of a session** (D-105). The envelope
  already carried `Direction::Outbound` and the container already took records
  of both kinds; what was missing was a name for the capability, an adapter that
  populates it, and a rule for judging it.
  - The two observation points hand over different shapes. `RawNode` gives the
    buffer the decoder consumed, which `unpack` has stripped; `SentFrame` gives
    what went to the Noise encryption, format byte still attached. Forwarding it
    untouched would write frames every reader misparses by one byte.
  - **Outbound stanzas are compared at L0 and never derived** (D-106). A test
    pins that an inbound pair still diverges at L1, so the guard cannot pass by
    having switched the comparison off.
  - The gate reports the split per side: a candidate that stopped observing its
    own sends loses records, and absent records are what a total cannot show.
- **`UNMODELLED_FIELDS` is empty.** The last four entries were union mixins,
  which now generate an enum apiece, named after their alternatives so the same
  group under two shapes becomes one type. Alternatives are tried richest-first
  (D-107).
  - Descending into them surfaced a fifth thing: `contentString`, a method the
    generator had never met because nothing above the mixin used it. Adding the
    primitive was three lines; finding it took modelling the mixin.
  - Tests for the alternatives are generated, per D-042. The shape fixtures
    reach only the leanest alternative by construction — a fixture satisfying a
    richer one satisfies the leaner one too — so each is built from its own
    fields, which is also the only thing that exercises the ordering.
  - One group has an alternative requiring nothing, so it accepts anything the
    others turn down. The generated test names that rather than asserting the
    group can report a miss, which it cannot.

### rev 31 — 2026-08-08

- **`whatsapp-rust` has takeover, and this document said it did not — while
  also saying it did.** §3.1 and the RFC-002 matrix rested on
  `StanzaRouter::register` panicking on a duplicate tag, so a built-in handler
  could not be replaced. The panic is still there; upstream #1239 added
  `StanzaInterceptor`, which runs where dispatch would have and never goes
  through the router.
  - **Step 9 of the implementation plan has recorded that since rev 13**
    ("a pre-dispatch interceptor, merged upstream as #1239"), and the adapter's
    own README documents the interceptor it rides. So this was not a fact the
    document lacked — it was a fact recorded in one section and contradicted in
    two others for eighteen revisions. A changelog that appends is good at
    acquiring facts and bad at retiring them, and nothing here was checking the
    older sections against the newer ones.
  - Takeover there is partial, and the partiality is right: `success`,
    `failure`, `stream:error`, `ack` and a server-initiated `<iq>` ping are
    never offered, because each settles connection state. That is the same line
    RFC-003 draws, reached independently (D-103).
- **The send side became observable** (D-102). Upstream #1260 added
  `Event::SentFrame` — one marshaled stanza as handed to the Noise encryption,
  emitted at the single point every send crosses, leased so nothing is cloned
  while nothing listens. The matrix gained a row for it, separate from `send_node`:
  sending is what an adapter does, observing what left is what a recording needs.
  - This is the first engine-side support for recording **both halves** of a
    session. RFC-010 records the inbound half, which is all any engine could
    give it until now. Nothing in the container changes yet — noted because the
    constraint that shaped it has lifted, not because the format has.
- **The capability matrix now reads `hypermeow` rather than `whatsmeow /
  hypermeow`**, at PR #5 rather than at `main`, and says so. The fork's `main`
  has the raw-node hook; the frame bytes and plaintexts are still a branch, and
  a matrix that blurs the two overstates what an adapter could be built on today.
- **Rev 26's "widest of the three" for `hypermeow` is retired.** It cannot
  observe what it sends. The engines are converging on one surface at different
  speeds, and a per-engine ranking goes stale faster than a per-capability
  table; §3.6 now says that instead.
- **Line references audited** (D-104). `events.rs:1419` → `943`,
  `node_io.rs:307` → `337`, `node_io.rs:457` → `490`, `client.rs:70-88` → `93`,
  `messaging.rs:109` → `116`. Every statement they supported was still true,
  which is the problem: references that rot while their claims hold teach a
  reader to stop checking them. Ranges that were decoration are now file-only.
- **RFC-008's patch table still told the reader to use `slice_bytes()`** for the
  whole buffer. Rev 10 already recorded that this cannot work — `slice_bytes`
  takes a slice that already points inside the buffer — and added
  `backing_bytes()` upstream. The correction had been written in the changelog
  and never applied to the table, so the document contradicted itself for
  twenty-one revisions.
- **Every crate has a README.** Seven of ten had none.

### rev 30 — 2026-08-08

- **The generator was reading the wrong name for fifty fields.** whatspec
  records both a bundle-side name and the name that travels on the wire, and
  they differ for fifty of them. The emitter used the bundle's. Two shapes were
  live-wrong — `applicationError` for `application_error`, `serverId` for
  `server_id` — and nothing failed, because the fixture builder walked the same
  spec by the same rule and produced stanzas that were wrong the same way. The
  pair agreed with each other and with no stanza a server sends (D-098). The
  test that pins this is hand-written for exactly that reason, and it was
  checked against the old behaviour before being kept.
- **`UNMODELLED_FIELDS` was three different things under one name** (D-099). It
  held fifteen entries and read as fifteen pieces of work:
  - **Nine were assertions no pure derivation can make** — `from` must match the
    request's `to`, and equivalents for `id` and `type`. Derivation sees one
    stanza and no request (D-010), so these are a design limit rather than a
    backlog, and they now say so in their own list for the caller that does hold
    the request (D-100).
  - **One was an enum the spec declares with no variants**, whose values live on
    sibling shapes as literal guards. It reads as text, which is what the wire
    carries anyway, and is listed as untyped rather than missing.
  - **One was a name collision**: `verified_name` genuinely arrives as a child
    on one shape and as an attribute on another. Both are emitted now, the
    second aliased by its category (D-101).
  - **Four are real and remain** — union mixins with two or three variants each,
    each variant a named shape carrying its own fields and assertions, and
    nesting further mixins inside. Modelling them means recursive in-struct
    dispatch, which is a piece of its own rather than the tail of this one. The
    look was the deliverable here; the build is not promised.

### rev 29 — 2026-08-08

- **The gate reports content.** It could say "14 stanzas compared" and not what
  any of them were, which is the half the boundary already had before L1 could
  read a payload. It now counts message kinds per side and marks the ones that
  differ (D-096). Counting per side is the point: a merged total would hide a
  candidate that read fewer messages than the baseline, which is the finding.
- **The fifth gate criterion is measured** (D-097). Four of the five had tests
  behind them and performance had nothing. Each read path now carries a budget
  and the assertion is against the budget, not against last week's run:
  envelope decode 210 ns, frame parse 1.2 µs, stanza derive 11.6 µs, payload
  derive 521 ns, walking a 32-record recording 87 µs, all in a debug build.
  Ceilings sit around four times those, because what is worth catching is a
  borrow becoming a copy and not a loaded CI runner.
  - The budgets live in `tests/` rather than `benches/`, since a criterion that
    runs only when somebody remembers is not a criterion. A test asserts that a
    blown budget fails the run, so the mechanism cannot rot into printing.
  - `stanza derive` at nine times the parse it contains is not a defect: the
    generated derivation tries shapes richest-first until one matches, so a
    receipt walks several that do not. Recorded because it is the first time
    anybody measured it.
- **A cross-language fixture was making a claim it could not back.** It declared
  a token dictionary, the gate correctly refused to compare recordings whose
  dictionary it does not have, and the fixture stopped testing anything else.
  The claim is gone: a dictionary tag says a reader holding that table can parse
  these frames, and that fixture exists to exercise the container.

### rev 28 — 2026-08-08

- **D-090 was wrong, and D-093 reverses it.** It claimed the payload half of L1
  had to be written because whatspec records nothing about the protobuf inside
  an `<enc>`. It does: `wa-proto` extracts the schema from the WA Web bundle's
  `internalSpec` modules, emits `WAProto.proto`, and pins it by SHA-256 in the
  manifest. The oracle was there and this project did not look for it, so the
  field numbers came from a copy checked into another repository instead.
- **The cost was 22 of 29 wrappers.** The hand-written list had the seven
  `FutureProofMessage` envelopes somebody thought of; the spec declares
  twenty-nine. Poll, status, spoiler, newsletter and bot messages would all have
  read as unmodelled rather than being unwrapped to the message inside. The
  generator collects them **by type** now (D-094), so the next one arrives
  without anyone remembering it.
- **Provenance gained a second digest** (D-095). The two halves come from two
  domains and can move apart: WhatsApp can renumber a protobuf field without
  changing how a stanza parses. One digest would call two builds the same spec
  when only half of it matched.
- **Two bugs in the generator itself**, both caught by its own refusals rather
  than by producing wrong numbers:
  - The spec nests most of `waE2E` inside `Message`, so a scan that only saw
    top-level blocks found `Message` and nothing it points at.
  - Two different `ExtendedTextMessage` types exist under different parents and
    disagree about what their fields hold, so lookups had to become qualified
    by path. They happen to agree that `text = 1`, which is exactly the kind of
    luck a hand-written number relies on.
  - The generator now checks that the file's braces balance before trusting its
    own scan, because a block it opened and never closed would misqualify every
    name after it silently. Two empty one-line messages (`message Signal {}`)
    were doing precisely that.

### rev 27 — 2026-08-08

- **L1 reads the plaintexts.** Until now the boundary carried decrypted
  payloads that nothing read: the derivation covered `receipt`, `ack`, `call`
  and the *shell* of `message`, and message content not at all. The payloads
  crossed, were compared between engines, and went nowhere.
- **`wa-wire-proto`**, a protobuf wire-format reader (D-089): `no_std`, no
  dependencies, borrowing from the payload. Total over the format, including
  the deprecated groups, because a reader that stopped at one would stop on a
  payload it could otherwise have handed over whole. It inherits the two
  disciplines already in place for free: the mutation sweep now covers it, and
  the allocation counter measures it at zero.
- **`wa_wire_l1::content`**, written rather than generated (D-090). D-039 says
  L1 is generated because whatspec records how WhatsApp Web parses a stanza and
  writing that by hand would be guessing at the spec. That reason does not
  reach the protobuf inside an `<enc>`: whatspec says nothing about it, so
  there is no oracle to generate from. The oracle here is `waE2E.proto` and
  every field number sits next to the line it came from.
- **Deliberately partial, and total anyway.** `waE2E.Message` has over a
  hundred variants; this models twelve and answers two questions for every
  payload: which kind is this, and what does it say. A variant it does not
  model crosses as `Unmodelled` carrying its field number (D-091), which is how
  the next one gets found.
- **Wrappers are unwrapped first** (D-092). A real message often arrives inside
  `deviceSentMessage` or one of seven `FutureProofMessage` envelopes, and a
  reader that reported the envelope would answer "what did this say" with "it
  was a wrapper". The depth is reported rather than hidden.
- **Two findings from writing the tests**, both mine:
  - The wrapper list was hand-built and missing one of the seven, which would
    have made a whole class of message read as unmodelled. Replaced by a
    predicate, so there is no second list to forget.
  - An unmodelled field read as `Empty`, indistinguishable from a payload with
    no fields at all. That is the failure the totality rule exists to prevent,
    so the reader now reports the number it saw and says plainly that it cannot
    tell an unknown variant from metadata without the whole schema.
- **The example consumer reads message content**, which is what makes the claim
  checkable from outside: it tallies messages by kind and collects their text,
  and the test drives a real L0-plain envelope through it end to end.

### rev 25 — 2026-08-08

- **The gate is a command.** `wa-wire-gate` takes two recordings and prints a
  verdict. Nothing in it is new logic; what is new is that any of it can be run.
  Until now the container, the comparator and the profiles were reachable only
  from tests, which is the same gap the example consumer closed for the
  boundary — and that one **found a real bug** the moment it became the first
  code to use the crates from outside (D-062).
- **Three exit codes, because a pipeline branches on them** (D-086). `0` pass,
  `1` fail, `2` incomparable, plus `64` for bad arguments and `66` for a
  recording that could not be read. A CI step that collapsed `2` into failure
  would send someone hunting a bug that is not there; one that collapsed it into
  success would ship on no evidence at all. The distinction only pays if it
  survives the process boundary, so the exit codes are tested by running the
  binary rather than the library.
- **The gate is where dictionary resolution actually happens** (D-087). RFC-010
  says a comparison whose tables are unavailable is incomparable rather than
  attempted; `Comparability::check` cannot enforce that, because it does not
  know what tables exist where it runs. The host does, so
  `Incomparable::UnresolvableDictionary` is reported by the host, and the report
  says whether a dictionary was declared or assumed — an assumption a reader
  cannot see is an assumption nobody checked.
- **No silent caps.** Long finding lists are trimmed and say what they trimmed,
  per the same rule that made encoder divergences a named list rather than a
  counter (D-060).
- **Every decoder now proves the claim it makes.** Three crates read buffers
  written elsewhere and all three documented that a malformed one "must be
  reportable, never a panic". Nothing checked it. `malformed_input.rs` sweeps
  deterministic mutations across the envelope decoder, the container reader and
  the frame parser, and asserts more than the absence of a panic: when a decoder
  *accepts* a mutated buffer, the invariants it advertises still have to hold
  (D-088).
- **The sweep refuses to become vacuous.** It asserts that mutations land on
  both sides — some accepted, some refused — because a sweep where everything is
  rejected proves only that the first length check works, and it can become that
  silently after a stricter header. Measured: about a quarter of mutated
  envelopes still decode, so the accept path is genuinely exercised.
- **A finding from writing it**: the frame parser already bounds nesting at 64
  and refuses deeper input with an error that names the limit. The test was
  written expecting to *discover* whether a 2 000-deep frame would take the
  stack with it; it found the defence already there, so it now pins both sides
  of the limit instead — refused past it, fully walkable inside it.
- **Deterministic rather than coverage-guided**, deliberately: `cargo-fuzz`
  needs nightly and a crate outside the workspace, so it would run when someone
  remembered. This runs on every commit with no dependency, and a failure
  reproduces exactly from the seed in the assertion. Coverage-guided fuzzing is
  worth adding on top, not instead.

### rev 24 — 2026-08-08

- **Step 11 done: the container is a contract.** `wa-wire-recording` implements
  RFC-010 in Rust and `adapters/zapo/src/recording.ts` implements it in
  TypeScript, with fixtures written by one and read by the other — the same
  arrangement RFC-008 already had, for the same reason. The ad hoc `WAWR` that
  `engine_agreement.rs` parsed by hand is gone; that test now reads through the
  contract, and refuses a recording that is truncated, damaged, or carries a
  critical tag this build cannot interpret.
- **Two amendments the implementation forced**, both recorded rather than
  quietly applied:
  - **CRC-32, not SHA-256** (D-084). The draft put a cryptographic digest in the
    trailer. Every crate here is dependency-free and `no_std`, and the
    TypeScript writer has to run in a browser, so it would have been
    hand-written twice — and it would have claimed tamper-evidence an unsigned
    file does not have. Identity stays in `input_digest`, which the container
    carries and never computes.
  - **Capabilities travel as identifier strings** (D-085). `CapabilitySet` is a
    `u8` whose bit assignment is internal to one crate; `Capability::identifier`
    is stable and is literally what the TypeScript enum holds. A format read by
    three languages must not depend on two of them agreeing about bit order.
- **A property the tests pinned down**, which the design had not stated: the
  checksum covers everything *before* the trailer, so it cannot cover the count
  the trailer carries. Every field has exactly one detector — the body by the
  checksum, the count by disagreeing with what was found, the checksum by
  itself — so neither check is redundant and neither is missing.
- **`is_fault` is a profile now** (D-080), which is the change that turns the
  conformance suite into a second product. `report.evaluate(profile)` returns
  `Pass`, `Fail` or `Incomparable`, and the same corpus that passes as interop
  fails as regression — asserted in `engine_agreement.rs`, because two engines
  are not two builds of one.
- **Two facts the comparator used to suppress are now recorded**: frame origin
  changing, and a status moving between two non-`Ok` values. Both suppressions
  were right between engines and would have made the regression profile blind.
  Recording them surfaced a real consequence immediately: the two adapters
  differ on frame origin for *every* stanza, which the corpus test now states
  outright rather than having hidden.
- **Comparability is declared, not assumed** (D-078). `Recording::new` is a
  caller vouching for both sides; a recording read from a container carries the
  claim and it is checked. Mixing the two is refused, because half a checked
  claim leaves the pair unchecked. The corpus test now has both engines compute
  the same corpus checksum independently, so two recordings of different traffic
  report `Incomparable` instead of reading as an engine regression.
- **Coverage**: the workspace is at 96.3% lines, and every file is above 95%
  except the generated derivation, whose tests are generated with it. Two
  pre-existing gaps closed on the way past: `wa-wire-adapter/src/send.rs` (72% →
  99%) and `wa-wire-example-consumer` (82% → 100%), both low because their only
  exercise lived in the adapter workspace, where this crate's coverage is not
  measured.

### rev 23 — 2026-08-08

- **RFC-010 proposed: the recording container.** A container already existed —
  `WAWR`, a count, length-prefixed envelopes — written by a script and read by
  hand inside one test. Its own comment argued against specifying it, and that
  argument was right for as long as the writer and the reader were the same CI
  job. It stops being right when a recording travels: at that point the file is
  carrying claims about which engine, which spec, which dictionary and which
  traffic produced it, and a format with nowhere to put them does not make those
  claims absent, it makes them unverifiable (D-073).
- **What the ad hoc format cannot express**, each of them a defect only under the
  new use: big-endian inside a little-endian contract (D-074); a count in the
  header, which excludes a ring buffer and therefore the entire flight-recorder
  use (D-075); no adapter, spec, dictionary or artifact class; no identity for
  the input, so two recordings of *different* traffic read as a regression; and
  truncation that goes undetected whenever the cut lands on a record boundary.
- **A truncated recording stays readable** (D-076). The artifact a crash
  recorder exists to produce is, by definition, the one that was interrupted. A
  container that rejected it would fail its most important use while passing
  every test written against well-formed files. Missing trailer means truncated
  and not comparable, never unparseable.
- **Comparability moves into the file** (D-078). `compare` documents "the same
  stanzas, in the same order" as a precondition, and cannot check it. That is
  fine for a test with both sides in view and useless for a gate running
  unattended, so the recordings declare it: same input digest, same artifact
  class, compatible dictionary, matching provenance, neither truncated. A live
  capture declares no input digest and is therefore never gate-comparable
  (D-079) — it is an input, not a result.
- **`is_fault` becomes a profile** (D-080), which is the change that turns the
  conformance suite into a second product. Between two engines, differing frame
  bytes are two valid encodings of one stanza; between two versions of one
  engine, they are the encoder changing under you. Same evidence, opposite
  verdicts, so the comparator records facts and the profile judges them. The
  verdict gains a third value (D-081): today a provenance mismatch is ignored by
  `agrees()`, so "these were unlike things" reports as "they agree" — a green
  result from a comparison that never ran.
- **Two facts the comparator suppresses today** have to start being recorded and
  left unjudged: frame origin changing, and a status moving between two non-`Ok`
  values. Both suppressions are correct between engines and wrong between
  versions.
- **Sanitization gets constraints, not an algorithm.** A sanitized frame is
  necessarily re-encoded, because a JID cannot be replaced without rewriting the
  frame — so it forfeits `FrameOrigin::Original` and must say so. And a
  sanitizer has to preserve the *encoding shape* of what it replaces, not only
  its type (D-083): both conformance findings so far were encoding-shape bugs,
  visible only because captured traffic held those shapes.

### rev 22.1 — the review pass

Eleven findings against rev 22, all acted on. Five were defects rather than
polish:

- **Takeover was killing decryption.** `zapo` decrypts inside the dispatch that
  takeover suppresses, so every encrypted stanza timed out in the joiner and
  crossed as `Unobserved`: the mode contradicted D-021, which exists precisely
  to forbid that. The filter now suppresses everything *except* the stanzas the
  joiner is holding, which is the smallest carve-out that keeps L0-plain
  producible.
- **A lost stanza did not fail the conformance run.** `Divergence::Length` was
  classified not-a-fault, so two recordings of the same traffic with different
  counts reported agreement. Length, direction and plaintext are faults now.
- **The comparison ignored everything but the frame**, including the plaintext
  table — the entire difference between L0-wire and L0-plain. Payloads both
  sides call usable must now match; differing *coverage* is reported and is not
  a fault, because that is a limit on the adapter (D-055) rather than an engine
  being wrong.
- **An odd-length path corrupted the envelope.** `NodePath::from_le_bytes`
  truncated the component *count* but not the bytes, so the encoder wrote
  `path_len = 1` followed by three bytes and the decoder read the third as the
  status. It passed the size assertion.
- **A failed status could carry a payload**, in both languages. Now refused on
  the way out and on the way in, since each side is the other's only guard.

Also: the plugin subscription was `mem::forget`ed and now lives in the API the
host holds; the capability gate checks the mode the instance was installed in
rather than what the adapter can do; `onError` carries the stanza that failed
instead of a fabricated node; `Buffer` left the send path, which was breaking
every non-Node runtime; and path dependencies gained versions.

One finding was refused. Making L1 consume the plaintexts would mean adding a
parameter nothing reads: `wa-wire-l1` is generated from whatspec's `incoming`
domain, which describes stanza parsing, and the protobuf reader that would use
a payload is not written. The gap is now stated in the README, in RFC-001's
sub-layer section and in the definition of done, rather than papered over with
an unused argument.

### rev 17 — 2026-08-07

- **The thesis is now a passing test.** `wa-wire-example-consumer` is a consumer
  written once and run against both engines, and
  `one_consumer_reads_both_engines_to_the_same_answer` shows the same code
  reaching the same answer from each. Until now the project had a contract, a
  codec, a derivation, two adapters and a conformance suite — and not one line
  showing anybody *using* them.
- **What makes it hold is the absence** (D-061): the consumer depends on
  `wa-wire-contract`, `wa-wire-codec` and `wa-wire-l1`, and on no engine,
  runtime, transport or async. Code that cannot name an engine cannot be coupled
  to one. Two tests enforce that graph rather than trusting it, because one
  convenience dependency would end it silently.
- **The equality is not vacuous**, and the tests say why: every corpus stanza
  reaches the consumer, most derive an event, and the two engines' envelopes
  differ on every single stanza — one declares the frame verbatim, the other
  re-encoded. Different input, same conclusion.
- **It surfaced a real bug, which is what an example is for** (D-062). Only
  `ack` and `receipt` derived; `<message>` and `<call>` did not. Two causes, one
  trivial and one not:
  - the corpus was missing attributes the shapes require (`recipient` on a
    message, the four on `<offer-notice>`) — a corpus fault;
  - and `parse_int` read only strings. The nibble alphabet exists to compress
    runs of digits, so **every encoder packs timestamps** — which meant every
    packed `t` in real traffic failed to derive, taking `<message>` and
    `<call>` with it. The two most important stanza kinds did not derive at
    all, and nothing had noticed.

  Fixed by reading the digits straight out of the packed run, without
  materialising a string (`no_std`). A packed run that is not numeric — the
  alphabet also carries `-` and `.` — is still reported as not-an-int rather
  than guessed at. All four modelled tags now derive, and the consumer test
  asserts the full set.

### rev 17.1 — pulling the thread

The packed-integer bug was not alone. Auditing every extractor against the
encodings real traffic uses found a second of the same kind, and cleared the
rest.

- **A JID written as text did not derive** (D-063). `attr_jid` accepted only the
  wire's dedicated JID form, and a second engine was observed writing
  `from="s.whatsapp.net"` as a dictionary token instead — both valid. The
  consequence was worse than the integer bug: one engine derived an event where
  the other derived nothing **from identical traffic**, which is a conformance
  fault caused by the derivation rather than by either engine. Every `receipt`
  or `message` from a bare server was invisible to L1.
- **Accepting text does not mean accepting anything** (D-064). A lone word with
  no `@` is read as a bare server only when the wire wrote it as a token, since
  servers are dictionary entries. Bytes stay rejected, so a JID field holding
  something that is not a JID is still reported.
- **The rest of the extractors are clear**, checked rather than assumed:
  - enums go through `Value::eq_str`, which handles all five encodings —
    now asserted by a test rather than left as a property of the code;
  - `Value::semantic_eq` covers the cross-encoding comparisons;
  - `content_bytes` is only used for blobs (ciphertext, protobuf, identity),
    which no encoder tokenises.
- **`tests/encoding_shapes.rs`** is where this now lives: one value, several
  valid encodings, one derived event. Seven cases, and the place to add the
  next one.
- **The last finding was whatspec's, and it was a real extraction bug** (D-065).
  All five captured `<message>` stanzas failed because `<meta>` declared a
  required `content` attribute real stanzas do not carry. Tracing it into
  `wa-forge` found the cause: an attribute read whose name is not a string
  literal — `e.attrString(k)` — fell back to naming the field `content`, and
  since `attrString` is not a `maybe` spelling it was published as **required**.
  The generated parser then rejected every element that correctly lacked it.

  Three fields were invented that way, each obligatory and each unsatisfiable:
  `incoming /content` (`attrJidWithType`), `incoming /meta/content`
  (`attrString`), `notif /content` (`attrFromJid`).

  Fixed upstream in `wa-forge`: a read whose name cannot be resolved is reported
  through the existing `dropsByReason` channel — `attributeNameNotLiteral` —
  rather than guessed at. Dropping a read says "this scanner could not follow
  it"; inventing one says something about the protocol that is not true, and the
  generated parser enforces it.

  With the regenerated spec, captured `<message>` derivation goes from **0 of 5
  to 5 of 5**, and the captured corpus from 31 to 36 derived stanzas.

### rev 16 — 2026-08-07

- **A capture tool, so the corpus can stop being hand-written.**
  `adapters/whatsapp-rust/examples/capture-corpus.rs` connects to a server, taps
  the inbound stream and writes each stanza as a frame. The corpus's weakness is
  that it holds the stanzas someone thought to write down; a server sends shapes
  nobody would think to write down, and those are where two engines are most
  likely to disagree.
- **The tool knows nothing about any particular server** (D-058). The endpoint
  is `WA_WIRE_CAPTURE_URL`, pairing can optionally be forwarded to
  `WA_WIRE_CAPTURE_PAIR_POST`, and skipping TLS/certificate-chain verification
  is the `insecure-capture` **feature** rather than a runtime flag — a build
  without it cannot be talked into skipping either.
- **Frames are not scrubbed** (D-059), deliberately. Capture from a test
  account; review before committing.
- **Version lookup is skippable** via `WA_WIRE_CAPTURE_VERSION`. By default the
  client fetches the live web-client version over the internet before
  connecting, which makes a capture depend on a network the server has nothing
  to do with and pins it to whatever is live at the time. `with_version_override`
  skips the lookup entirely.
- **The blocker was mine, not the server's.** `Client::connect()` completes the
  handshake and returns; the loop that reads frames off the socket lives inside
  `Client::run()`. A capture built on `connect` alone watches the server's bytes
  reach the transport and never get decoded — which is exactly the silence
  observed. Switching to `run` captured 67 stanzas immediately.
- **Captured traffic produced the first L0 differences** — and they measure
  something narrower than they first appeared to (D-060). Across 67 captured
  stanzas the frames differ three times: `from="s.whatsapp.net"` encoded as a
  user-less JID against a dictionary token (twice), and a childless node written
  with an explicit empty body against none.

  The correction: on a **captured** frame `whatsapp-rust` forwards the server's
  bytes untouched, so the difference is between whoever encoded that frame and
  `zapo` — *not* between the two engines. On the hand-written corpus, where both
  sides encode from the same source, their bytes match. So the two engines'
  encoders have not been shown to differ at all.

  What holds either way, and is the property that matters: **two different
  encodings of one stanza derive the same event**, now with encodings that
  really did differ rather than a tolerance nothing exercised.
- **The captured frames are not committed.** They came from a mock, which does
  not reproduce the real server, so committing them would buy CI coverage of one
  mock's encoder while reading as "we test against real traffic". They also
  carry whatever the server sent. Capture stays an investigation tool; what it
  finds gets distilled into written fixtures when it can be.

### rev 15 — 2026-08-07

- **The central claim is now a measurement.** `whatsapp-rust` and `zapo` both
  read a committed corpus of frames and their output is compared:
  `adapters/whatsapp-rust/tests/engine_agreement.rs`. This is what §8 called the
  milestone that matters — *"if the project is going to fail, it fails there"* —
  and until now the conformance crate only had tests of the **comparator**,
  driven by envelopes Rust built for itself. Two engines had never actually been
  put side by side.
- **They agree, and by more than the design allows for** (D-057). The suite was
  built to tolerate an L0 difference, since two encodings of one stanza are both
  valid and only L1 has to match. On this corpus there is nothing to tolerate:
  `zapo` re-encodes from a decoded node, `whatsapp-rust` forwards the buffer it
  received, and the frames come out **byte-identical** across all 13 stanzas.
  The test asserts `is_identical()` rather than `agrees()` so that stops being
  true loudly rather than quietly.
- **What the result does not cover, stated plainly.** Because the encoders agree,
  the *interesting* path — different bytes, same derived event — is exercised
  only by the comparator's own unit tests, not by two real engines. A third
  engine less faithful to the format is what would exercise it. The corpus also
  covers the four tags the derivation models (13 stanzas, ≥8 deriving an event),
  which is enough to make agreement non-vacuous but is not real traffic.
- **The corpus is frames, committed** (D-056), regenerated by
  `cargo run --example emit-corpus` on the Rust side and read by
  `npx tsx scripts/emit-recording.ts` on the TypeScript side.

### rev 14 — 2026-08-07

- **Step 10 done: `whatsapp-rust` emits L0-plain.** The first of the four
  adapters the definition of done asks for. Needed a second observation point
  in the engine, sent upstream as
  [#1240](https://github.com/oxidezap/whatsapp-rust/pull/1240): a plaintext that
  decrypts but does not decode was being logged and dropped, and since the
  decryption had already advanced the ratchet, those bytes were gone for good.
  The PR argues that on its own terms and never mentions `wa-wire`.
- **The adapter joins frame to plaintexts itself** (D-052). They arrive at
  different times and cannot be made to arrive together — the frame when a
  stanza is decoded, a plaintext only after Signal. Holding the frame until its
  table fills keeps "one stanza, one envelope", which is the unit replay and
  conformance compare.
- **Waiting is counted in stanzas, not milliseconds** (D-053). Investigating
  the engine first is what settled this: `UndecryptableMessage` is per *message*
  and deduplicated by id, so it cannot close a per-node table, and several
  paths drop an `<enc>` with no event at all. Nothing can close the table by
  event alone, so something has to give up — and a stanza count gives up
  identically on every machine, which is the property this project's output
  needs. The frame itself supplies the count of `<enc>` nodes, so the common
  case still closes immediately, with no clock involved.
- **`PlaintextStatus::Unobserved` added to RFC-008** (D-054). The three existing
  statuses each assert a cause; an adapter that watches plaintexts appear knows
  a node produced nothing but not why. Both encoders carry it, and the
  cross-language fixture now exercises all four.
- **Fan-out stanzas stay L0-wire** (D-055), a limitation found by writing the
  end-to-end test rather than by reading the code: the engine's `<enc>`
  enumeration concatenates direct children with the ones addressed to its own
  device under `<participants>`, so an index cannot be resolved to a node
  without the device's JID.
- **Three upstream review findings were real and fixed**: the bot-secret
  (`msmsg`) path decoded its payload without emitting the event, and its secret
  is single-use, so a dropped payload there is as unrecoverable as a ratcheted
  one; `enc_index` was counted per decryption bucket rather than per
  stanza — the common group shape (pkmsg + skmsg) reported the same index for
  both; and splitting a published `wacore` function changed its arity, which
  `cargo semver-checks` correctly called a breaking change.

### rev 13 — 2026-08-07

- **Step 9 done: takeover on `whatsapp-rust`.** Needed an upstream change, sent
  as [#1239](https://github.com/oxidezap/whatsapp-rust/pull/1239) and written to
  stand on its own: the engine gains a pre-dispatch interceptor so a consumer
  can act on a stanza it does not model, instead of watching it get nacked. The
  PR never mentions `wa-wire`.
- **Both engines now have takeover.** `zapo` had it natively; this closes the
  gap D-020 opened.
- **Review caught two real faults in the first cut of the patch** (D-050,
  D-049), both about the same thing — a takeover that answers nothing or takes
  what it should not leaves a *worse* client, not an extended one:
  - a claimed stanza outside `should_ack`'s tags went unanswered, exactly the
    failure `nack_unrecognized_stanza` exists to prevent;
  - `success`, `failure`, `stream:error` and `ack` could be claimed, which
    would strand authentication, reconnection or a pending send.
- **Tap and takeover declare different capabilities** (D-051). Takeover cannot
  see the auth phase, because the engine refuses to offer those stanzas to an
  interceptor; tap can, and cannot suppress. Neither is a superset, so a single
  declaration would be false in one direction.

### rev 12 — 2026-08-07

- **Step 5 implemented: the `zapo` adapter**, in TypeScript. 99.5% line
  coverage; the whole adapter is one `registerIncomingStanzaFilter` call.
- **Takeover works with no fork**, as finding 3.2 predicted: returning `true`
  from the filter drops the stanza from `zapo`'s pipeline while `zapo` still
  acks it, so the server does not redeliver. This is the first adapter to have
  it.
- **Cross-language fixtures** (D-046). Envelopes written by the TypeScript
  encoder are decoded by the Rust one in
  `crates/wa-wire-conformance/tests/cross_language.rs`, including the
  multi-device case where a path landing on the wrong `<enc>` would attribute a
  decrypted message to the wrong recipient. Committed, regenerated by CI,
  checked from both sides.
- **The frame is re-encoded and says so** (D-047). `zapo`'s filter receives a
  decoded node, so `frameOrigin = ReEncoded`. The README records the one-line
  change upstream (`decoder.ts:344`) that would make `l0.zero-copy-frame` true.
- Capability gaps confirmed against the engine rather than assumed: no
  auth-phase coverage (`success`/`failure` are protected from filters), no
  plaintexts (the filter runs before decryption).

### rev 11 — 2026-08-07

- **Steps 4 and 6 implemented: `wa-wire-l1` and `wa-wire-conformance`.**
  95.6% line coverage and 100% function coverage across the workspace; clippy
  clean.
- **L1 is generated from whatspec's `incoming` domain** (D-039). Three things
  the spec forced out that a hand-written parser would have guessed wrong:
  - the same field is read several ways at different call sites — `t` as a
    string, an int and a timestamp — so the most specific reading wins (D-040);
  - a call site using the always-present reader does not make a field required
    on the wire, and trusting it made shapes reject valid stanzas (D-040);
  - shapes of one tag must be tried richest-first, or the most permissive one
    claims everything — a call receipt swallowed every message receipt, its
    required fields being a subset (D-041).
- **Tests for generated code are generated too** (D-042), from the same shapes.
- **`semantic_eq` added at every level** — `Value`, `Jid`, `Packed`, and each
  generated shape. Two engines can encode one value differently and both be
  right, and a comparison that called that a divergence would bury the real
  ones (D-043).
- **Conformance separates context from faults** (D-044). A frame difference is
  reported and is not a fault; a derivation difference is. Two engines failing
  the same way is agreement (D-045).
- Provenance mismatch is reported *first*, because it changes how every L1
  difference after it reads.

### rev 10 — 2026-08-07

- **Step 3 implemented: `wa-wire-adapter` (SDK) and the `whatsapp-rust` tap
  adapter.** 100% line and function coverage across the workspace; clippy clean.
- **Correction to RFC-008's patch table.** It recorded "none — expose existing
  `slice_bytes()`" for `whatsapp-rust`. Wrong: `slice_bytes` takes a slice that
  already points inside the buffer, so it cannot produce the buffer itself. One
  method was added upstream — `OwnedNodeRef::backing_bytes()` — cloning the yoke's
  cart, so still a refcount bump rather than a copy. The zero-copy claim holds;
  the route to it did not.
- **Bug found by testing against the real engine.** `TokenTable::single_byte`
  applied a `-1`, treating tags as one-indexed. `whatsapp-rust`'s
  `get_single_token` indexes by the tag byte directly — its table carries a
  placeholder in slot 0 rather than shifting. The off-by-one parsed *cleanly*
  and resolved every token to its neighbour, so nothing failed until a frame the
  engine actually produced was fed through. Fixed, and pinned by a test against
  the engine's own tag numbers.
- **`l0.plaintext` added as a distinct capability (D-035).** `Event::RawNode`
  fires where a stanza is decoded, necessarily before Signal runs, so this
  adapter emits L0-wire and its envelopes carry an empty plaintext table.
  Honest rather than degraded — most stanzas never had anything encrypted — but
  a `<message>` crosses without its plaintext, and closing that needs a second
  observation point inside the engine.
- Capability claims are now **verified against stanzas** (D-036), so a claim
  that stops being true fails a test.
- Recorded D-035 through D-038.
- Adapter dependencies reduced to one. `whatsapp-rust` re-exports `wacore` and
  `wacore_binary`, so naming them separately only added a way for them to drift
  to a different version than the engine actually links.

### rev 9 — 2026-08-07

- **Step 2 implemented: `wa-wire-codec`.** `no_std`, zero runtime dependencies,
  `unsafe` forbidden, nothing allocated while parsing.
  - Covers the whole value grammar: single-byte and dictionary tokens, all three
    binary widths, packed nibble and hex runs, and all four JID forms
    (pair, user-with-domain-type, interop, Messenger).
  - `NodeRef` holds the slice starting at its own list tag and re-walks on
    demand — the encoding is self-delimiting, so a node never needs to know
    where it ends, and no offset arithmetic is required to slice one out.
  - 100 unit tests plus 6 integration tests; **99.83% line coverage across the
    workspace, 100% function coverage**; clippy clean at `pedantic`.
- **Design refinements recorded as D-031…D-034.** The one worth naming: packed
  runs and JIDs have no string anywhere in the frame, so they stay in parts and
  compare through `eq_str` rather than being joined. That is what keeps a 433 KB
  stanza allocation-free.
- **Integration test asserts the two crates agree about paths.** If the
  contract's `NodePath` and the codec's `at_path` ever diverged, a decrypted
  message would be attributed to the wrong recipient — too important to leave
  as a shared assumption between two crates.
- **Token dictionaries generated and committed** (`tools/generate-tokens.py`),
  with the source table's SHA-256 recorded in the generated module. CI
  regenerates and requires no diff, which is RFC-009's codegen rule (D-028)
  in practice.
- Bug found and fixed during testing: `skip_node_body` walked child bodies twice,
  because `Children`'s iterator already advances past each child. Caught by the
  path-navigation tests, which is why they exist.
- CI extended: codec without bundled tokens, whole-workspace wasm32 build, and a
  generated-code freshness gate.

### rev 8 — 2026-08-07

- **Step 1 implemented: `wa-wire-contract`.**
  - `no_std`, zero dependencies, `unsafe` forbidden.
  - Decoding borrows from the input buffer — no allocation, no copy. Encoding
    writes once into a caller-supplied slice, or allocates exactly once behind
    the optional `alloc` feature.
  - Modules: `envelope`, `path`, `flags`, `status`, `error`, `version`,
    `capability`, `provenance`.
  - 86 tests plus a doctest; **99.70% line coverage, 100% function coverage**;
    clippy clean at `pedantic` with `arithmetic_side_effects`,
    `indexing_slicing`, `panic`, `unwrap_used` and `expect_used` denied in the
    library.
  - Verified building with `--no-default-features` and for
    `wasm32-unknown-unknown`.
- Envelope layout implemented exactly as RFC-008 specifies, with a test pinning
  the byte layout so a change to the wire is a change to that test.
- Decode validates the whole envelope up front, so iterating entries afterwards
  cannot fail. Truncation is rejected at *every* offset — a test cuts the buffer
  at each byte and requires an error.
- Recorded D-029 and D-030.
- Added CI: fmt, clippy, tests, docs, coverage gate, and the two portability
  builds.

### rev 7 — 2026-08-07

- **RFC-008 and RFC-009 accepted. All nine RFCs now `ACCEPTED`. No design
  blocker remains; implementation can start at step 1.**
- **RFC-008 resolved by reframing the question.** "Binary-node versus a
  purpose-built flat encoding" presupposed that the node gets serialized at the
  boundary. It does not: the frame bytes already exist in every engine, and the
  frame never contained the plaintext, so the envelope is
  *frame verbatim + plaintext side table*. Nothing is re-encoded, so there is no
  encoding to choose. The founding thesis holds literally.
- **Verified byte-access sites in all four engines** — `node_io.rs:307`,
  `decoder.ts:344`, `noise-handler.ts:196`, `client.go:824`.
- **Correction to rev 1's capability matrix:** zero-copy was recorded as absent
  in every engine. Wrong for `whatsapp-rust` — `OwnedNodeRef` is
  `Yoke<NodeRef<'static>, BytesCart>`, so the node already borrows from a
  retained buffer and `slice_bytes()` already returns zero-copy sub-views. This
  is what made D-016 cheap enough to sit in v1.
- Normative L0-wire payload pinned: the **unpacked** binary-node buffer, no
  format byte — verified identical across all four decoders.
- Envelope layout specified: 8-byte header, verbatim frame, path-addressed
  plaintext table with explicit `status`.
- **RFC-009 separates contract version from spec provenance** (D-027). Merging
  them would break every deployed adapter on every WhatsApp change; L0 totality
  is what makes the separation safe.
- Codegen decided: generated and committed, CI-enforced (D-028).
- Recorded D-023 through D-028.
- §8 updated: step 0 closed, ordering re-based, byte-access patch sites listed.
- **Consistency pass.** Fixed stale entries left behind by rev 5–6 decisions:
  R5 still assumed takeover was deferred (D-020 reversed that); R7 still pointed
  at the resolved OQ-4; the RFC-002 zero-copy row still read "no engine has it".
  Added R9 for the verbatim-frame design's failure mode.
- **All remaining open questions given provisional decisions with explicit
  revision triggers.** OQ-2 promoted to resolved. OQ-1, OQ-3 and OQ-7 are all
  Layer 3, out of v1 scope — nothing is left merely open.

### rev 6 — 2026-08-07

- **v1 scope locked.** Resolved OQ-4, OQ-5, OQ-6 by owner decision.
- Recorded D-018 through D-022.
- **D-021 is a clarification the owner's takeover decision forced out:** takeover
  had two possible readings — suppressing dispatch, or suppressing all engine
  processing including crypto. The second is incoherent, because L0-plain
  depends on the engine having decrypted. Fixed before it could become contract
  ambiguity.
- **Four unregistered gaps found in the readiness inventory**, none of which had
  an open question:
  - the L0-plain boundary format was never specified → **RFC-008**, now the
    hard blocker on all implementation;
  - `whatspec` → L1 codegen strategy undefined → RFC-009;
  - no contract versioning or compatibility policy → **RFC-009**;
  - licensing unexamined → verified whatsmeow/hypermeow are **MPL-2.0** while
    every other engine is MIT → D-022.
- **Added §8 Implementation plan** — the document had matured as a design with
  no execution layer: all seven RFCs sat at `proposed`, with no ordering and no
  definition of done.
- Ordering places the **conformance runner at step 5**, deliberately early: it
  is where the central claim becomes a test result rather than an assertion.

### rev 5 — 2026-08-07

- Measured real `BinaryNode` shape from `whatsapp-rust/docs/real-whatsapp-log.json`
  (214 parsed stanzas) instead of assuming it.
- **Key finding:** the distribution is extremely heavy-tailed — median 2 nodes /
  5 attrs / depth 2, max 4 528 nodes / 9 457 attrs / depth 9 / 433 KB. A ~2000x
  span. Attrs-to-nodes ratio ~2:1, so a large `iq` crosses with ~19 000 strings.
- **Two regimes identified, and the binding choice decides neither.** Common
  regime is fixed-call-overhead bound (all tools within noise); tail regime is
  untenable for field-by-field traversal in every tool.
- **OQ-8 resolved without needing W6.** D-005 reversed by D-016: zero-copy is a
  v1 requirement.
- D-013 re-grounded (D-017): kept for size/memory/zero-dependency, not speed.
- **W6 respecified** — measures boundary strategy (`S1-traverse` /
  `S2-reserialize` / `S3-bytes`) across four real fixtures, not tooling. The
  deciding metric is the slope of `S1` against nodes+attrs.
- Sampling caveat recorded: single pairing session, not steady state.

### rev 4 — 2026-08-07

- Resolved the FFI binding question in RFC-007 using `wasm-ffi-bench` figures.
- **Correction to rev 3:** the earlier "workload-dependent, undecided" framing
  undersold the data. `wbgen-flat` wins 4 of 5 scenario-1 workloads *and* is
  smallest *and* lowest RSS *and* the only one whose linmem does not grow.
  boltffi's single win is string marshalling — not our payload shape.
- Identified **two distinct boundaries** rather than one: ingress (JS→WASM,
  16x spread) and egress (WASM→JS, 7x spread on string-dense events). Egress
  was nearly missed and matters because L1 events are string-dense and arrive
  in batches.
- Recorded D-013, D-014, D-015.
- **[UNKNOWN] recorded:** no existing benchmark workload is recursive with
  variable depth, which is what a `BinaryNode` is. Proposed a W6 workload.
- Added **OQ-8** — zero-copy may outrank binding choice, since it removes
  boundary A instead of optimizing it. D-005 to be revisited after W6.

### rev 3 — 2026-08-07

- Added **RFC-007 — Language and repository strategy**.
- Rust core confirmed; Cargo workspace monorepo layout proposed with an explicit
  dependency rule (`wa-wire-contract` depends on nothing).
- **Constraint recorded:** Rust does not reach the Go adapter. cgo in the
  per-stanza hot path defeats G3, so the hypermeow adapter stays pure Go.
- **Design refinement:** L0 splits into L0-wire and L0-plain. This resolves the
  RFC-001 caveat that "L1 is not a pure function of a single stanza" — it is not
  pure over L0-wire, but it *is* pure over L0-plain, so L1 derivation moves
  host-side and exists once.
- `wa-store-migrate` port recommended as a differentially verified port rather
  than a rewrite; `docs/IR.md` (confirmed to exist upstream) becomes the
  normative spec with TS and Rust as co-implementations.
- FFI binding choice left open — `wasm-ffi-bench` shows it is workload-dependent
  and should be measured against the real L0-plain payload shape.
- Recorded D-009 through D-012.

### rev 2 — 2026-08-07

- Added **RFC-006 — Store ownership**, resolving the analysis half of OQ-2.
- Surveyed all four store models (`Types/Auth.ts:74-133`, `src/store/types.ts:21-125`,
  `store/store.go:23-121`, `wacore/src/store/traits.rs:936`).
- **Key finding:** event models converged across engines; store models diverged
  completely. Baileys is a generic typed KV; the other three are
  domain-oriented at three different granularities.
- **Key finding:** store operations carry logic, not just persistence
  (`GetOrGenPreKeys` generates keys; `MarkPreKeysAsUploaded` has range
  semantics; `transactWith` orders locks). A host owning the store would have
  to reimplement session logic — this is what rejects the fat-host option.
- Recorded D-007 and D-008.
- Noted that OQ-2 blocks only RFC-003, not Layers 1 and 2 — the project can
  start with it open.
- Identified the measurement that settles it: snapshot size/duration for a
  mature session. Local `whatsapp.db` is 0 bytes, so no figure yet.

### rev 1 — 2026-08-07

- Initial document.
- Surveyed `whatsapp-rust`, `zapo`, `Baileys`, `whatsmeow` local checkouts;
  all capability claims carry `file:line` references.
- **Correction vs. earlier discussion:** Baileys does *not* need a patch for L0
  in — `ws.emit('frame', frame)` (`socket.ts:749`) is already catch-all and
  covers more than `zapo`'s filter (handshake frames included).
- **Correction vs. earlier discussion:** `whatsapp-rust` does *not* support
  takeover; `StanzaRouter::register` panics on duplicate tags
  (`router.rs:30-35`).
- Confirmed `zapo` stanza filters run strictly in series — no reordering hazard
  (`WaIncomingNodeCoordinator.ts:194-197`).
- Confirmed `zapo` has zero hard runtime dependencies and prefers
  `globalThis.WebSocket`, making it the most runtime-portable engine.
- Confirmed one-connection-per-device across three engines; recorded as the
  binding constraint on RFC-003.
- Recorded D-001 through D-006.
