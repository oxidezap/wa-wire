# handoff-cycle

Moves one session from an engine to another and back, and asks whether what came
back differs from what left in exactly the ways the route declared.

That question is items 5 and 6 of the v2 definition of done, and it is not the
same as "did the migration work". Counting rows answers the second: 807 prekeys
went out and 807 came back. This compares them byte for byte, because a round
trip that returns the right number of prekeys with the wrong bytes inside them
passes every count and has destroyed the session.

The route is `whatsapp-rust → zapo → whatsapp-rust`, which D-136 prefers because
nothing is lost moving into `zapo`. The snapshot step runs on
[`wa-store-migrate`](https://github.com/vinikjkkj/wa-store-migrate) (D-008),
taken from npm rather than from its repository — the repository does not build,
`src/adapters/wa-web` is imported and not committed, and the published package
ships it.

## Running it

```console
npm install
npm run check
```

against the committed fixture, which is a real session: paired against the
Barback mock server, captured with `capture-corpus`, then reduced to eight
prekeys and eight app-state keys so a reviewer can read it. Nothing in it belongs
to a person — the account is the mock server's `559980000001`.

To make your own, pair a client and dump its store:

```console
WA_WIRE_CAPTURE_URL=ws://127.0.0.1:45999/ws/chat \
WA_WIRE_CAPTURE_STORE=/tmp/session.db \
WA_WIRE_CAPTURE_PAIR_POST=http://127.0.0.1:45999/admin/mock-phone/scan-qr \
WA_WIRE_CAPTURE_VERSION=2.3000.1027934701 \
  cargo run --example capture-corpus --features insecure-capture

python3 dump-rust-store.py /tmp/session.db > session.json
node cycle.mjs session.json
```

## What it found

Three things, and each is why the tool compares what it compares.

**`appStateSyncKeys.timestamp` changes and nothing declares it.**
`whatsapp-rust` has no timestamp column for app-state keys, so the IR carries
none; `zapo`'s writer turns that into `0`
(`adapters/zapo/from-canonical.js:81`, `k.timestamp ?? 0`) and its reader hands
back `0` rather than absent. Every key comes home claiming 1970. `planLosses`
says nothing, so a host that trusts the declaration ships it. Small on this
route — `whatsapp-rust` does not read the field — and not small on a route whose
destination picks the newest key by it.

**`appStateVersions` is declared lossy in both directions and came back
identical.** Over-declaring is the safe direction, and worth knowing: a host
reading the matrix would refuse or warn about a move that costs nothing here.

**`tcTokens` carry a non-deterministic default.** `updatedAtMs: t.timestampMs ??
Date.now()` (`adapters/zapo/from-canonical.js:104`) makes the same migration
produce a different store each run. The fixture has no tokens, so this is read
from the source rather than observed — recorded here so it is not found twice.

## The read recipe in the upstream README is wrong for `prekeys`

`wa-store-migrate`'s `whatsapp-rust` recipe splits `prekeys.key` as a 64-byte
keypair. It is a libsignal `PreKeyRecordStructure` protobuf — field 1 the id,
field 2 the public key, field 3 the private key — written by
`new_pre_key_record(id, &kp).encode_to_vec()`. Splitting it in halves yields two
32-byte strings, neither of which is a key, and nothing downstream notices: they
are the right length and the migration reports success. `dump-rust-store.py`
parses the record.

The device columns *are* a raw pair, and in the other order from how they read:
`serialize_keypair` writes private first, then public.

## The whole move

```console
npm install
./run-cycle.sh
```

Three legs, one session, one pairing:

1. **`whatsapp-rust` pairs** against the Barback mock and holds the account.
2. **`zapo` takes it over** — the store is migrated into `zapo`'s shape, seeded
   into a fresh store, and connected. It reports `credentials ready {
   registered: true }`, handshakes, receives `success`, and records traffic.
3. **`whatsapp-rust` picks it back up**, with the store `zapo` handed back.

Leg 3 is run with no pairing endpoint configured at all. A leg that needed to
pair would have nowhere to send the code, so it would hang and record nothing —
the assertion made by leaving the means out rather than by checking afterwards.

`zapo` consumed five prekeys while it held the session: 807 went in and 802 came
back. That number is the reason the return leg carries state rather than letting
`whatsapp-rust` reattach with the store it still had — offering the server keys
it has already handed out is the drift R1 is about, one step short of two
writers.

Seeding and harvesting both go through `zapo`'s own store contracts and nothing
else (D-007: the host never owns the store), so a contract that changes breaks
at the call instead of yielding a store that loads and is subtly wrong.

Harvesting has one real limit, and it is bounded rather than hoped away.
`auth.load()` and `appState.exportData()` are bulk reads, but `getPreKeysById`,
`getSessionsBatch` and `getRemoteIdentities` each answer about keys you already
name, and nothing enumerates. So the harvest asks for everything it seeded, then
probes prekey ids past the highest until a run of misses, and asks about every
peer JID that appeared in the leg's own recorded traffic. What that leaves is a
session written for a peer that never appears in the traffic — which nothing in
the protocol does, since a session comes from a message, a retry or a prekey
fetch, and all three are stanzas. An argument, not a guarantee, and the
difference is worth the sentence.

Two API mistakes cost a run each and are worth writing down. `WaClient` takes
`chatSocketUrls`, not `url`; passing the latter is silently ignored and the
client races its two production endpoints instead — a run against real WhatsApp
with credentials from a mock. And the certificate-chain check lives under
`dangerous`, the same reason the Rust side needs `insecure-capture`.

## Why the runner is not what checks it

`wa-wire-gate --profile interop` reports `INCOMPARABLE (neither side declares
its input)` on two legs, and it is right to. D-079: a comparison means something
only when both recordings say what traffic they are a replay *of*. Two live legs
are two different windows of a server talking, so a stanza-by-stanza difference
between them is a fact about the server and not about the engines. Reporting
"these disagree" there is the error the container exists to prevent.

Item 5 names that comparison, and the runner declines it by design. What a live
move actually supports is continuity, and `continuity.mjs` is that check:

- **The server paired once.** Its log is the one witness that cannot be talked
  into agreeing, since it is the party that would have had to accept a second
  pairing.
- **The same account came back.** `success` carries the account's `lid` and its
  `companion_enc_static`, and a re-pair would have minted a new one of each.
- **Every leg carried a session**, not just an open socket — messages and
  receipts, not silence.

Verified by pointing it at two recordings from *different* pairings, where it
fails on all three counts.

One thing it reports rather than asserts: leg B has no `success` in its
recording. `zapo` does not declare `l0.inbound.auth-phase` — it protects
`success` and `failure` from its stanza filters — so its recording cannot carry
the login even though the login happened, and `zapo`'s own log shows it. A
silent skip there would have turned a capability gap into a passing check.

## Measuring what a translating store would cost

Two scripts, for the two halves of a per-access budget — the numbers behind
RFC-006's Option E amendment.

```console
node measure-store-access.mjs session.json ws://127.0.0.1:46020/ws/chat 8
node measure-translation.mjs 3000
```

`measure-store-access.mjs` wraps the bundle `zapo` is handed and times every
call. A live session made **68 store calls across 47 stanzas — 1.4 per stanza**.
Counting starts after seeding, because the seed is the host's own writing and
charging it to the engine would inflate every row.

`measure-translation.mjs` prices one translation on the committed fixture's real
1705-byte session record. Across five runs of 5000: a pass-through view is
0.041–0.047 µs, a copy 0.286–0.315 µs, and a decode-and-re-encode through
`zapo`'s codec 3.4–4.4 µs. Ranges, because a single run's median moved by a
third between the first two attempts. For scale, the same run
shows `zapo`'s own `messages.upsertBatch` at 624 µs per call against an
in-memory store.

So translation is not where the time goes — but only if the canonical form is
one an engine already holds. That is a per-domain question, and the amendment
has the table.

There is also a route-blocking disagreement above this tool's level:
`registrationId`. `zapo` generates 1..16381 and Baileys masks to 14 bits, while
`whatsapp-rust` uses 1..2³¹−1 and whatsmeow a full `uint32`. `wa-store-migrate`
validates 14 bits, so a real `whatsapp-rust` session fails `validate: true` on
every route out. The wire carries a `uint32` — WA Web's own
`WhisperTextProtocol` declares it that way — so this is a local convention two
engines keep and two do not, and the migrator sides with the two.
