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

## Capabilities

| Capability | Status |
| --- | --- |
| `l0.inbound.tap` | yes — the filter sees every inbound stanza |
| `l0.takeover` | yes — returning `true` drops it, and `zapo` still acks |
| `lifecycle.drain-hook` | yes — `registerDispose` runs after handlers drain |
| `l0.inbound.auth-phase` | **no** |
| `l0.zero-copy-frame` | **no** |
| `l0.plaintext` | **no** |
| `l0.outbound` | **no** |
| `l0.request` | **no** |

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

### Why no plaintexts

The filter runs before decryption, so a `<message>` crosses with its ciphertext
and an empty plaintext table. Most stanzas — receipts, acks, presence — never
had anything encrypted, so this is honest rather than degraded. Reaching
L0-plain needs a second observation point after Signal.

## Cross-language fixtures

`fixtures/*.bin` are envelopes written by this encoder and decoded by the Rust
one in `crates/wa-wire-conformance/tests/cross_language.rs`. The boundary format
is described in two languages, and two descriptions only ever tested separately
are two formats waiting to diverge.

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
