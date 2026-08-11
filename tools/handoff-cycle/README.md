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

## What this does not do yet

The move is over the store only. Items 5 and 6 also want the traffic on either
side compared by the conformance runner, which needs `zapo` to attach with a
store it did not create — its backends are pluggable and writing one is host
work (D-007: the host never owns the store). The snapshot half is what was
blocked, and is not any more.

There is also a route-blocking disagreement above this tool's level:
`registrationId`. `zapo` generates 1..16381 and Baileys masks to 14 bits, while
`whatsapp-rust` uses 1..2³¹−1 and whatsmeow a full `uint32`. `wa-store-migrate`
validates 14 bits, so a real `whatsapp-rust` session fails `validate: true` on
every route out. The wire carries a `uint32` — WA Web's own
`WhisperTextProtocol` declares it that way — so this is a local convention two
engines keep and two do not, and the migrator sides with the two.
