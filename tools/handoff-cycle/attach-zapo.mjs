/**
 * Attach `zapo` to a session another engine was holding, and record what it
 * hears.
 *
 * The claim under test is the one v2 exists for: a session can change engines
 * without the account re-pairing. So the thing to watch is not that traffic
 * arrives — it is that **no QR is ever printed**. A run that pairs has proved
 * nothing except that the mock server pairs anyone.
 *
 * Usage:
 *   node attach-zapo.mjs <rust-snapshot.json> <ws-url> <out.wawr> [seconds]
 */

import { readFileSync, writeFileSync } from 'node:fs'

import { migrate } from 'wa-store-migrate'
import { WaClient, createStore } from 'zapo-js'
import { decodeBinaryNode } from 'zapo-js/transport'
import {
    ArtifactClass,
    Mode,
    RecordKind,
    createDetacher,
    encodeRecording,
    encodeEnvelope,
    waWire,
} from '@oxidezap/wa-wire-adapter-zapo'

import { harvest } from './harvest-zapo-store.mjs'
import { seed } from './seed-zapo-store.mjs'

const [snapshotPath, url, out, harvested, seconds = '8'] = process.argv.slice(2)
if (!snapshotPath || !url || !out || !harvested) {
    throw new Error(
        'usage: attach-zapo.mjs <rust-snapshot.json> <ws-url> <out.wawr> <harvested.json> [seconds]'
    )
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

/** Bytes back out as base64, the same way the dumper wrote them in. */
function encodeBytes(_key, value) {
    if (value instanceof Uint8Array) return Buffer.from(value).toString('base64')
    if (value?.type === 'Buffer' && Array.isArray(value.data)) {
        return Buffer.from(value.data).toString('base64')
    }
    if (value instanceof Map) return Object.fromEntries(value)
    return value
}

const rust = decode(JSON.parse(readFileSync(snapshotPath, 'utf-8')), '')
// `validate: false` for one reason, recorded in the README: `whatsapp-rust`
// generates a registration id wider than the 14 bits the IR checks, and so does
// whatsmeow. Nothing else in this snapshot is in question.
const moved = migrate({ from: 'whatsapp-rust', to: 'zapo', data: rust, validate: false })

const SESSION = 'handoff'
const store = createStore({})
const written = await seed(store.session(SESSION), moved.data)
console.log('seeded', JSON.stringify(written))

const envelopes = []
// Every peer the leg hears from, so the harvest can ask about addresses that
// were not in what it seeded. `zapo`'s signal stores answer about addresses you
// name and list nothing, and the traffic is where a new name can come from.
const peers = new Map()

function noteAddresses(frame) {
    let node
    try {
        node = decodeBinaryNode(frame)
    } catch {
        // A frame this build cannot read is a finding elsewhere, not here.
        return
    }
    for (const attr of ['from', 'participant', 'sender']) {
        const jid = node.attrs?.[attr]
        if (typeof jid !== 'string' || !jid.includes('@')) continue
        const [left, server] = jid.split('@')
        const [user, device] = left.split(':')
        const address = { user, server, device: device ? Number(device) : 0 }
        peers.set(`${address.user}|${address.server}|${address.device}`, address)
    }
}

const plugin = waWire({
    mode: Mode.Tap,
    sink: (stanza) => {
        envelopes.push(encodeEnvelope(stanza))
        noteAddresses(stanza.frame)
    },
})

// `chatSocketUrls`, not `url`: the latter is silently ignored and the client
// races its two production endpoints instead — which is a run against real
// WhatsApp with credentials from a mock server, and it took one to notice.
const client = new WaClient({
    store,
    sessionId: SESSION,
    chatSocketUrls: [url],
    markOnlineOnConnect: false,
    // The mock server signs its certificate chain with its own root key, which
    // is the same reason the Rust side needs `insecure-capture`. Not a runtime
    // choice: it is here because this tool only ever points at a mock.
    dangerous: { disableNoiseCertificateChainVerification: true },
    plugins: [plugin],
})

let pairedHere = false
client.on('auth_qr', () => {
    // The whole point, failing loudly. A QR means the credentials did not carry
    // and this run is measuring a fresh pairing rather than a handoff.
    pairedHere = true
    console.error('a QR was printed — the session did not carry')
})

const open = Promise.withResolvers()
client.on('connection', (event) => {
    if (event.status === 'open') open.resolve()
    if (event.status === 'close' && !open.settled) {
        open.reject(new Error(`closed before opening: ${event.reason}`))
    }
})

client.connect().catch((error) => open.reject(error))
await open.promise
console.log('connected without pairing')

await new Promise((resolve) => setTimeout(resolve, Number(seconds) * 1000))

// Detach, not logout — the same distinction the adapter's `Detach` trait makes
// unreachable by construction on the Rust side.
await createDetacher(client).detach()

// Read the session back before the store goes away. Every peer the leg heard
// from is asked for on top of what was seeded, because the contracts answer
// about addresses you name and list nothing.
const peersInTraffic = [...peers.values()]
const taken = await harvest(store.session(SESSION), moved.data, peersInTraffic)
writeFileSync(harvested, JSON.stringify(taken, encodeBytes, 1))
console.log(
    'harvested',
    JSON.stringify({
        preKeys: taken.preKeys.length,
        sessions: taken.sessions.length,
        identities: taken.identities.length,
        appStateSyncKeys: taken.appState.keys.length,
        peersSeenInTraffic: peersInTraffic.length,
    })
)

await store.destroy?.()

writeFileSync(
    out,
    Buffer.from(
        encodeRecording(
            {
                adapter: {
                    id: 'zapo',
                    version: '0.1.0',
                    engineVersion: '1.7',
                    contractVersion: 1,
                    capabilities: [
                        'l0.inbound.tap',
                        'l0.plaintext',
                        'lifecycle.drain-hook',
                        'lifecycle.detach',
                    ],
                },
                artifactClass: ArtifactClass.Captured,
            },
            envelopes.map((envelope) => ({ kind: RecordKind.Envelope, envelope }))
        )
    )
)

console.log(`${envelopes.length} envelopes -> ${out}`)
if (pairedHere) {
    console.error('FAILED: the run paired, so it did not test a handoff')
    process.exit(1)
}
