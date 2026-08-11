/**
 * Count and time every store call a live session makes.
 *
 * A translation layer pays its cost per access, so "is it fast enough" is not a
 * question about the codec alone — it is the codec times how often the engine
 * reaches for the store. That second number is the one nobody has, and it is
 * cheap to get: wrap the bundle `zapo` is handed and record what it asks for.
 *
 * The wrapper measures its own overhead, not the engine's work: `await`ing the
 * underlying call and timing the whole thing is what a translating store would
 * do, so the number here is the honest denominator for a per-access budget.
 *
 * Usage:
 *   node measure-store-access.mjs <rust-snapshot.json> <ws-url> [seconds]
 */

import { readFileSync } from 'node:fs'

import { migrate } from 'wa-store-migrate'
import { WaClient, createStore } from 'zapo-js'
import { Mode, createDetacher, waWire } from '@oxidezap/wa-wire-adapter-zapo'

import { seed } from './seed-zapo-store.mjs'

const [snapshotPath, url, seconds = '8'] = process.argv.slice(2)
if (!snapshotPath || !url) {
    throw new Error('usage: measure-store-access.mjs <rust-snapshot.json> <ws-url> [seconds]')
}

const BYTE_FIELDS = new Set([
    'privKey', 'pubKey', 'key', 'record', 'keyId', 'keyData', 'stateData',
    'indexMac', 'valueMac', 'token', 'signedPreKeySignature', 'advSecretKey',
    'account', 'edgeRoutingInfo', 'nctSalt', 'bytes',
])

function decode(value, field) {
    if (typeof value === 'string' && BYTE_FIELDS.has(field)) {
        return new Uint8Array(Buffer.from(value, 'base64'))
    }
    if (Array.isArray(value)) return value.map((entry) => decode(entry, field))
    if (value && typeof value === 'object') {
        return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, decode(v, k)]))
    }
    return value
}

/** Calls per `domain.method`, and the wall time spent inside them. */
const calls = new Map()

function record(name, nanos) {
    const seen = calls.get(name) ?? { count: 0, nanos: 0n }
    seen.count += 1
    seen.nanos += nanos
    calls.set(name, seen)
}

/** A domain store that counts and times what is asked of it. */
function watch(domain, target) {
    return new Proxy(target, {
        get(object, property, receiver) {
            const value = Reflect.get(object, property, receiver)
            if (typeof value !== 'function') return value
            const name = `${domain}.${String(property)}`
            return function measured(...args) {
                const started = process.hrtime.bigint()
                const result = value.apply(object, args)
                if (result && typeof result.then === 'function') {
                    return result.finally(() => record(name, process.hrtime.bigint() - started))
                }
                record(name, process.hrtime.bigint() - started)
                return result
            }
        },
    })
}

const DOMAINS = [
    'auth', 'signal', 'preKey', 'session', 'identity', 'senderKey', 'appState',
    'retry', 'groupMetadata', 'chatMetadata', 'deviceList', 'messages',
    'messageSecret', 'threads', 'contacts', 'privacyToken',
]

function watchBundle(bundle) {
    const wrapped = Object.create(null)
    for (const domain of DOMAINS) {
        const store = bundle[domain]
        wrapped[domain] = store && typeof store === 'object' ? watch(domain, store) : store
    }
    // Everything else on the bundle — `destroyCaches`, `destroy` — passes through.
    return new Proxy(bundle, {
        get: (object, property, receiver) =>
            property in wrapped ? wrapped[property] : Reflect.get(object, property, receiver),
    })
}

const rust = decode(JSON.parse(readFileSync(snapshotPath, 'utf-8')), '')
const moved = migrate({ from: 'whatsapp-rust', to: 'zapo', data: rust, validate: false })

const SESSION = 'measured'
const real = createStore({})
await seed(real.session(SESSION), moved.data)

// Counting starts after seeding: the seed is the host's own writing, and
// charging it to the engine would inflate every number below.
calls.clear()

const bundles = new Map()
const store = {
    session(id) {
        if (!bundles.has(id)) bundles.set(id, watchBundle(real.session(id)))
        return bundles.get(id)
    },
    destroyCaches: () => real.destroyCaches(),
    destroy: () => real.destroy(),
}

let stanzas = 0
const client = new WaClient({
    store,
    sessionId: SESSION,
    chatSocketUrls: [url],
    markOnlineOnConnect: false,
    dangerous: { disableNoiseCertificateChainVerification: true },
    plugins: [waWire({ mode: Mode.Tap, sink: () => { stanzas += 1 } })],
})

const open = Promise.withResolvers()
client.on('connection', (event) => {
    if (event.status === 'open') open.resolve()
})
client.connect().catch((error) => open.reject(error))
await open.promise

await new Promise((resolve) => setTimeout(resolve, Number(seconds) * 1000))
await createDetacher(client).detach()

const rows = [...calls].sort((a, b) => b[1].count - a[1].count)
const totals = rows.reduce(
    (sum, [, seen]) => ({ count: sum.count + seen.count, nanos: sum.nanos + seen.nanos }),
    { count: 0, nanos: 0n }
)

console.log(`\n${stanzas} stanza(s) over ${seconds}s\n`)
console.log('calls   µs total   µs each   store call')
for (const [name, seen] of rows) {
    const micros = Number(seen.nanos) / 1000
    console.log(
        `${String(seen.count).padStart(5)}   ${micros.toFixed(1).padStart(8)}   ` +
            `${(micros / seen.count).toFixed(2).padStart(7)}   ${name}`
    )
}

const perStanza = stanzas > 0 ? totals.count / stanzas : 0
console.log(
    `\n${totals.count} store calls, ${(Number(totals.nanos) / 1e6).toFixed(1)} ms total, ` +
        `${perStanza.toFixed(1)} calls per stanza`
)
console.log(
    'a translating store adds its own cost to each of those, so this is the ' +
        'denominator a per-access budget divides into'
)

await real.destroy?.()
