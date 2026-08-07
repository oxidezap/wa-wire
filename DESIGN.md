# wa-wire — Design Document

> **Status:** **IMPLEMENTING** — all nine RFCs accepted. Steps 1–6, 9 and 10 of
> §8 are done; steps 7–8 (Baileys, hypermeow) remain. Two engines are measured
> agreeing on derived events (rev 15); `whatsapp-rust` emits L0-plain, one
> adapter of the four the definition of done asks for.
> **Name:** `wa-wire` (D-018) · **License:** MIT, `adapters/hypermeow/` MPL-2.0 (D-022)
> **v1 scope:** L0 + L1, takeover included. No L2, no Layer 3 host.
> **Owner:** oxidezap
> **Last revised:** rev 13

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

| Feature | Location | Note |
| --- | --- | --- |
| `Event::RawNode` | `src/client/node_io.rs:457` | dispatched **before any early return**, so IQ responses and `xmlstreamend` are included |
| `RawNodeLease` | `src/client.rs:70-88` | atomic refcount; forwarding disables when the last lease drops |
| Idle cost avoidance | `src/client/node_io.rs:325-331` | an `ack` skips even the `Arc::new` when nothing observes |
| `send_node()` | `src/client/messaging.rs:109` | L0 out |
| `wait_for_node(NodeFilter)` | `src/client.rs` | raw request/response, zero-cost with no waiters |
| Plugin host | `src/plugins/mod.rs:38-92` | capability bitflags (`CoreEvents`/`Tasks`/`Messaging`/`Iq`/`PluginEvents`), install/callback/drain timeouts, lease acquired from *declared interest* (`mod.rs:507-536`) |
| ~60 typed event kinds | `wacore/src/types/events.rs:216-276` | close to a ready-made L1 vocabulary |
| Runtime abstraction | `wacore/src/runtime.rs` | `Runtime` trait, `Send` dropped on wasm32 |

**[VERIFIED] No takeover.** `Event::RawNode` is purely observational — the
native pipeline runs afterwards regardless (`node_io.rs:510-553`). The
`StanzaRouter` cannot be used to override a built-in handler either: it
**panics** on duplicate tag registration (`src/handlers/router.rs:30-35`).

**[VERIFIED] Observation is not free and not neutral.** Enabling raw forwarding
changes scheduling: `processes_inline()` returns `false` for `receipt` and `ack`
once forwarding is on (`node_io.rs:565-582`), moving them off the read loop into
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

### 3.4 `whatsmeow` / `hypermeow` is the only engine needing a patch

**[VERIFIED]**

- Node dispatch table is private: `cli.nodeHandlers` (`client.go:118`,
  populated `client.go:290-305`).
- Dispatch path: `client.go:844` → `handlerQueue` → `client.go:873`. A tap hook
  belongs immediately before the enqueue at `:844`. Estimated ~20 lines.
- L0 out exists only through the explicitly unstable
  `DangerousInternals().SendNode(ctx, node)` (`internals.go:170`),
  `SendNodeAndGetData` (`:166`), `HandleFrame` (`:158`), `DispatchEvent` (`:174`).
- Event surface is `AddEventHandler` (`client.go:769`) — typed events only, no
  raw node.
- No plugin system, no drain hook.

Since `hypermeow` is an oxidezap-adjacent fork keeping the `go.mau.fi/whatsmeow`
module path, the patch lands there first and can be proposed upstream later.

### 3.5 One connection per device — verified in three engines

**[VERIFIED]** WhatsApp terminates the previous connection when a second one
authenticates with the same keys:

- `whatsmeow`: `<conflict type="replaced"/>` → `events.StreamReplaced`
  (`connectionevents.go:48-51`); also treated as terminal in `request.go:35-37`.
  Doc: *"emitted when the client is disconnected by another client connecting
  with the same keys"* (`types/events/events.go:138`).
- `whatsapp-rust`: `Event::StreamReplaced` (`wacore/src/types/events.rs:263, 1419`),
  documented for `<conflict>`, 516 device removal, and 401 (`events.rs:1404`).
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

But **no single engine has all of it**: `whatsapp-rust` has full L0 coverage and
no takeover; `zapo` has takeover but does not cover the auth phase; `Baileys`
has the broadest raw coverage and neither a plugin system nor takeover.

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
│ Baileys: ws.on('frame')    hypermeow: ~20-line hook         │
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
- **[OQ-4](#oq-4-l2-in-v1)** — in or out of v1. Unresolved.

### Delivery modes (per subscription, per layer)

| Mode | Meaning | Engine support |
| --- | --- | --- |
| `tap` | observe; engine keeps processing natively | all four |
| `takeover` | suppress native processing; engine becomes transport + acks | `zapo` only today |

`takeover` is what makes an engine genuinely interchangeable — under takeover,
engine-specific semantics stop mattering because the engine stops interpreting.
It is also the mode that will require patching `whatsapp-rust`, whose
`StanzaRouter` panics on duplicate registration
(`src/handlers/router.rs:30-35`).

### Cost disclosure (normative)

An adapter **must** declare when enabling a subscription changes engine
behavior beyond adding a callback. Precedent:
`whatsapp-rust`'s `processes_inline()` reroutes `receipt`/`ack` off the read
loop once raw forwarding is enabled (`node_io.rs:565-582`). Silent scheduling
changes under observation are a conformance violation.

---

## RFC-002 — Capability matrix

**Status:** **ACCEPTED** (rev 7)

Following the cultural precedent already set by `wa-store-migrate`'s loss
reports and `whatspec`'s `dropsByReason`: **what cannot be done is declared
explicitly, never silently degraded.**

### Current state (verified 2026-08-07)

| Capability | whatsapp-rust | zapo | Baileys | whatsmeow / hypermeow |
| --- | --- | --- | --- | --- |
| L0 in, catch-all | ✅ `Event::RawNode` | ✅ stanza filter | ✅ `ws.on('frame')` | ❌ patch (~20 LOC) |
| L0 in covers auth/stream phase | ✅ | ❌ `success`/`failure` protected | ✅ incl. `Uint8Array` frames | — |
| L0 takeover | ❌ router panics | ✅ filter → `true` + auto-ack | ❌ observation only | ❌ |
| L0 out | ✅ `send_node` | ✅ `sendNode` | ✅ `sendNode` | ⚠️ `DangerousInternals` |
| Raw request/response | ✅ `wait_for_node` | ✅ `query` | ✅ `query` | ⚠️ `DangerousInternals` |
| Plugin host | ✅ capability bitflags | ✅ tuple-typed | ❌ | ❌ |
| Drain hook | ✅ `task_drain_timeout` | ✅ `registerDispose` | ❌ | ❌ |
| Zero-copy frame bytes | ✅ **already retained** — `Yoke<NodeRef, BytesCart>` + `slice_bytes()` | one-line patch at `decoder.ts:344` | one-line patch at `noise-handler.ts:196` | small patch at `client.go:824` |
| Runtime portability | native + wasm32 | node/bun/deno/browser | node (hard `ws`) | native |

**Zero-copy was re-assessed in rev 7** (see RFC-008). The rev 1 entry — "no
engine has it" — was wrong for `whatsapp-rust`: `OwnedNodeRef` is
`Yoke<NodeRef<'static>, BytesCart>` (`wacore/binary/src/node.rs:902-903`), so
the parsed node already borrows from a retained buffer and `slice_bytes()`
already hands out zero-copy sub-views. In the other three the bytes sit in a
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

**[UNKNOWN]** — [OQ-1](#oq-1-isolation-unit). Sessions-as-tasks in one
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
**Resolves:** [OQ-2](#oq-2--store-ownership)
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
- Limits: JS on the critical path ([OQ-3](#oq-3--wa-store-migrate-as-dependency-or-rust-port)),
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

**Status:** **ACCEPTED** (rev 7)

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
| whatsapp-rust | `src/client/node_io.rs:307` | `OwnedNodeRef::new(buffer)` — `Yoke<NodeRef<'static>, BytesCart>` (`wacore/binary/src/node.rs:902-903`) | **none — already retained**; expose via existing `slice_bytes()` (`:929-931`) |
| Baileys | `Utils/noise-handler.ts:196-198` | `const result = transport.decrypt(frame)` | pass `result` alongside `frame` into `onFrame` — one line |
| whatsmeow | `client.go:823-830` | `decompressed` from `waBinary.Unpack(data)` | pass `decompressed` with the node into `handlerQueue` |
| zapo | `transport/binary/decoder.ts:334-344` | `nodeBytes` in `decodeBinaryNodeStanza` | return/emit `nodeBytes` alongside the node |

**This substantially lowers D-016's cost.** In `whatsapp-rust` zero-copy is
already free — the parsed node borrows *from* the retained buffer, and
`slice_bytes()` already returns zero-copy sub-views. In the other three the
bytes sit in a local variable at the decode site; the patch is to propagate it.

**Normative payload definition** (verified consistent across all four): the
L0-wire payload is the **unpacked binary-node buffer** — after decompression,
without the leading format byte — i.e. exactly what each engine's decoder
consumes. `whatsapp-rust` documents this precisely at
`wacore/binary/src/node.rs:907-909`; `whatsmeow` performs the same `Unpack`
before `Unmarshal` (`client.go:824-830`).

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

**Status:** **ACCEPTED** (rev 7)

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

### OQ-7 — Handoff window — *provisional: no SLA claimed*
Unmeasured, per engine pair. **Provisional decision:** claim no availability SLA
for handoff until measured, and have Layer 3 refuse any route whose loss the
capability matrix reports as exceeding threshold (R6).

**Revision trigger:** Layer 3 work starting. `whatsapp-bench` already has the
pinned-source, hermetic, offline harness to run it.

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

1. `wa-wire-contract` published, with the RFC-008 format specified and frozen.
2. Four adapters emitting L0-plain: `whatsapp-rust`, `zapo`, `Baileys`,
   `hypermeow`.
3. L1 derivation generated from `whatspec`, host-side, single implementation.
4. Conformance suite (RFC-005) green: identical L0 in → identical L1 out across
   all four engines. **Green for two of them as of rev 15.**
5. Capability matrix machine-readable and enforced at setup.
6. Takeover working on at least `zapo` (native) and `whatsapp-rust` (patched).

**Explicitly out of v1:** L2 commands, Layer 3 host, session handoff, fencing,
multi-session pooling, media transfer, the `wa-store-migrate` port.

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
| ~~9~~ | ~~`whatsapp-rust` takeover patch (D-020)~~ | — | **done in rev 13** — a pre-dispatch interceptor, upstream at #1239 |
| ~~10~~ | ~~`whatsapp-rust` adapter, L0-plain~~ | — | **done in rev 14** — a per-`<enc>` plaintext event upstream at #1240, joined to its frame adapter-side |

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

---

## Changelog

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
- **Not yet exercised end to end.** Against a local test server the handshake
  completes and the server replies, but no stanza reaches the tap — most likely
  server-side setup (a pre-registered user, or a client version the server
  accepts) rather than the tool. The corpus is still the hand-written one, and
  the agreement result in rev 15 stands unchanged.

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
