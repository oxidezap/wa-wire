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

## The live move

`attach-zapo.mjs` is the other half: it migrates a `whatsapp-rust` session into
`zapo`'s shape, seeds a fresh `zapo` store with it, and connects. The thing to
watch is not that traffic arrives — it is that **no QR is ever printed**. A run
that pairs has proved only that the mock server pairs anyone, so the script
fails if one appears, and the server's own log is the second opinion.

```console
node attach-zapo.mjs session.json ws://127.0.0.1:46002/ws/chat leg-b.wawr 6
```

It works. `zapo` reports `credentials ready { registered: true }`, completes the
handshake, receives `success`, and records 47 envelopes — against a server log
holding exactly one pairing, from the `whatsapp-rust` leg minutes earlier. The
session changed engines.

Seeding goes through `zapo`'s own store contracts and nothing else (D-007: the
host never owns the store), so `seed-zapo-store.mjs` is written against
documented methods and breaks at the call if one of them changes.

Two things cost a run each and are worth writing down. `WaClient` takes
`chatSocketUrls`, not `url`; passing the latter is silently ignored and the
client races its two production endpoints instead — a run against real WhatsApp
with credentials from a mock. And the certificate-chain check lives under
`dangerous`, which is the same reason the Rust side needs `insecure-capture`.

## Why the runner refuses to compare the two legs

It should. `wa-wire-gate --profile interop` reports `INCOMPARABLE (neither side
declares its input)` — D-079: a comparison means something only when both
recordings say what traffic they are a replay *of*. Two live legs are two
different windows of a server talking, so a stanza-by-stanza difference between
them is a fact about the server and not about the engines. Reporting "these
disagree" there is the error the container exists to prevent.

Which makes item 5's phrasing — "the events on either side compared by the
conformance runner" — ask for something the runner is built to decline. The
comparison the v1 machinery does make is two engines over *one* recorded input,
and that already runs (`engine_agreement`). What a live handoff can be checked
for is continuity, and that is a different assertion needing a different check.

## What is still missing

The return leg. Coming back needs `zapo`'s session read *out*, and its store
contracts do not offer that: `getPreKeysById`, `getSessionsBatch` and
`getRemoteIdentities` all take the keys you are asking about, and nothing
enumerates. Only `appState.exportData()` has a bulk read. So `zapo` can be
attached to from outside and not harvested from outside — which is not an
oversight, since those contracts are shaped for the engine's own lookups.

The way through is the one the store was built for: `createStore({backends,
providers})` takes a caller's backend, and a backend sees every write as it
happens. Harvesting becomes recording rather than reading. That is the next
piece, and it is host work.

There is also a route-blocking disagreement above this tool's level:
`registrationId`. `zapo` generates 1..16381 and Baileys masks to 14 bits, while
`whatsapp-rust` uses 1..2³¹−1 and whatsmeow a full `uint32`. `wa-store-migrate`
validates 14 bits, so a real `whatsapp-rust` session fails `validate: true` on
every route out. The wire carries a `uint32` — WA Web's own
`WhisperTextProtocol` declares it that way — so this is a local convention two
engines keep and two do not, and the migrator sides with the two.
