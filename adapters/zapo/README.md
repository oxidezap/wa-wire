# wa-wire adapter for zapo

Observes every inbound stanza and hands it on. Optionally suppresses `zapo`'s
own dispatch, which is takeover — without a fork.

## How it works

`zapo` exposes `registerIncomingStanzaFilter`, which runs against **every**
inbound stanza before any handler. That one hook is the whole adapter:

```ts
import { WaClient } from 'zapo-js'
import { Mode, waWire } from '@oxidezap/wa-wire-adapter-zapo'

const client = new WaClient({
    plugins: [waWire({ sink: (stanza) => queue.push(stanza) })],
})
```

Returning `true` from the filter drops the stanza from the engine's pipeline
while `zapo` still emits the ack, so the server does not redeliver. That is
`Mode.Takeover`: the engine keeps doing Noise and acks and stops interpreting.
Under takeover an engine's own semantics stop mattering, which is what makes
engines interchangeable.

It never suppresses decryption. L0-plain depends on the engine having
decrypted, so a takeover that disabled crypto would silently degrade the
contract rather than extend it.

In `zapo` decryption happens *inside* the dispatch takeover suppresses, so a
stanza carrying ciphertext is passed on and only everything else is dropped.
That exception is the whole reason this mode is still L0-plain.

## Capabilities

| Capability | Status |
| --- | --- |
| `l0.inbound.tap` | yes — the filter sees every inbound stanza |
| `l0.plaintext` | yes — payloads joined onto the stanza they came from |
| `lifecycle.drain-hook` | yes — `registerDispose` runs after handlers drain |
| `l0.takeover` | only when installed as `Mode.Takeover` |
| `l0.outbound` | on `createSender` |
| `l0.request` | on `createRequester` |
| `l0.inbound.auth-phase` | **no** |
| `lifecycle.detach` | on `createDetacher` — `disconnect()` closes the transport and keeps the credentials |
| `l0.zero-copy-frame` | **no** |

The first three are what any instance provides. Takeover is not: installed as a
tap this adapter suppresses nothing, whatever it is capable of, so the
capability is checked against the mode rather than against the adapter
(`TAP_CAPABILITIES` and `TAKEOVER_CAPABILITIES`). Sending and requesting carry
their own declarations for the same reason.

Every row is asserted in `src/__tests__/adapter.test.ts`. A claim that stops
being true fails a test rather than quietly misleading a consumer.

### Why no auth phase

`zapo` protects `success` and `failure` from stanza filters
(`FILTER_PROTECTED_TAGS` in `WaIncomingNodeCoordinator.ts`). That is deliberate
on `zapo`'s side — a filter that dropped `success` would break the login flow —
so the tap does not see the authentication exchange.

### Why the frame is re-encoded

The filter receives a decoded `BinaryNode`, not the buffer it came from, so this
adapter re-encodes and sets `frameOrigin = ReEncoded`. A consumer reads that
flag and knows not to expect the bytes to be identical to what arrived.

Closing it is one line upstream, at `src/transport/binary/decoder.ts:344`:
`decodeBinaryNodeStanza` already holds `nodeBytes` and drops them on return.
Emitting them alongside the node would make `l0.zero-copy-frame` true here, the
same way `OwnedNodeRef::backing_bytes` did for `whatsapp-rust`.

### How the plaintexts get there

The filter runs before decryption, so the frame alone is L0-wire. `zapo` emits
`debug_decrypted_payload` per `<enc>` afterwards, and `joiner.ts` holds a
`<message>` until its payloads arrive so a consumer sees one envelope per stanza
rather than a frame and then a stream of payloads to correlate itself.

It closes by counting the stanza's `<enc>` children, so the common case has no
clock in it. Giving up on one that never arrives is measured in **stanzas
rather than milliseconds**: the receive path is ordered, and a count reads the
same on every machine, which a duration does not.

A fan-out `<message>` crosses as L0-wire with no table. Its `<enc>` nodes under
`<participants><to>` are numbered after the direct ones and only for this
device, and reproducing that needs a device JID this adapter does not have. A
frame without payloads is a smaller claim than a payload on the wrong `<enc>`.

## Cross-language fixtures

Two formats are written here and read by Rust in
`crates/wa-wire-conformance/tests/cross_language.rs`: `fixtures/*.bin` are
RFC-008 envelopes, and `fixtures/*.wawr` are RFC-010 recordings, including one
frozen with no trailer so the other side has to read an interrupted file.

Each format is described in two languages, and two descriptions only ever
tested separately are two formats waiting to diverge.

```console
npx tsx scripts/emit-fixtures.ts
```

They are committed, and both sides check them: this package asserts they
regenerate byte-identically, and the Rust side asserts it can read them.

## Development

```console
npm install
npm test
npm run test:coverage   # gate at 95% lines
npm run typecheck
```

Expects a `zapo` checkout beside `wa-wire`:

```
projects/
├── wa-wire/
└── zapo/
```

Like the `whatsapp-rust` adapter, this is outside the main workspace and its own
CI concern: building it needs an engine checkout, and the `wa-wire` crates stay
dependency-free.
