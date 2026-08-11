/**
 * What every domain costs to translate, not just the session record.
 *
 * The session measurement said translation is cheap and that the canonical
 * format decides more than the language does. Both claims were made on one
 * domain, and the four engines disagree domain by domain — so this prices the
 * other three on real records from a real group session: a sender key from an
 * actual SKDM, the bincode `HashState` `whatsapp-rust` writes for app-state, and
 * the device record, which is not a blob at all.
 *
 * The device is the interesting one. There is no codec: it is a struct of
 * key pairs and scalars, and the translation is field mapping plus splitting
 * each 64-byte column into its two halves. So it prices a different *kind* of
 * work, and a design that assumed every domain was a protobuf would have missed
 * that the cheapest domain is cheap for a different reason.
 *
 * Usage:
 *   node measure-domains.mjs [iterations]
 */

import { readFileSync } from 'node:fs'

import {
    decodeSenderKeyRecord,
    encodeSenderKeyRecord,
} from 'zapo-js/signal'

const ITERATIONS = Number(process.argv[2] ?? 5000)
const fixture = JSON.parse(
    readFileSync(new URL('./fixtures/group-session.json', import.meta.url), 'utf-8')
)

const bytes = (value) => new Uint8Array(Buffer.from(value, 'base64'))

function time(label, work) {
    for (let i = 0; i < Math.min(500, ITERATIONS); i += 1) work()
    const samples = new Array(ITERATIONS)
    for (let i = 0; i < ITERATIONS; i += 1) {
        const started = process.hrtime.bigint()
        work()
        samples[i] = Number(process.hrtime.bigint() - started) / 1000
    }
    samples.sort((a, b) => a - b)
    const median = samples[Math.floor(samples.length / 2)]
    console.log(`${label.padEnd(48)} ${median.toFixed(4).padStart(8)} µs`)
    return median
}

// --- sender key ---------------------------------------------------------------

const senderKey = fixture.senderKeys[0]
const senderKeyBytes = bytes(senderKey.record)
const groupId = senderKey.address.split(':')[0]
const sender = (() => {
    const [, right] = senderKey.address.split(':')
    const [user, rest] = right.split('@')
    const [server, device] = rest.split('.')
    return { user, server, device: Number(device ?? 0) }
})()

console.log(`sender key record: ${senderKeyBytes.length} bytes`)
console.log(`app-state HashState: ${bytes(fixture.appStateVersions[0].stateData).length} bytes`)
console.log(`${ITERATIONS} iterations\n`)

time('sender key: pass-through (view)', () => {
    const view = senderKeyBytes.subarray(0)
    if (view.length === 0) throw new Error('unreachable')
})

const decoded = decodeSenderKeyRecord(senderKeyBytes, groupId, sender)
time('sender key: decode (proto -> zapo)', () => {
    decodeSenderKeyRecord(senderKeyBytes, groupId, sender)
})
time('sender key: encode (zapo -> proto)', () => {
    encodeSenderKeyRecord(decoded)
})

// --- app-state ----------------------------------------------------------------

// `whatsapp-rust` writes `bincode::serde::encode(&HashState, standard())`; zapo
// holds `{version, hash, indexValueMap}`. wa-store-migrate carries the codec
// because no engine's own library speaks the other's framing here.
// By file URL: the package does not export this subpath, and the codec is
// internal to it. Reaching in is deliberate and worth saying out loud — the
// measurement is of that codec, not of an API anyone should call.
const { decodeRustHashState, encodeRustHashState } = await import(
    new URL('./node_modules/wa-store-migrate/dist/esm/adapters/whatsapp-rust/bincode.js', import.meta.url)
)

const stateData = bytes(fixture.appStateVersions[0].stateData)
time('app-state version: pass-through (view)', () => {
    const view = stateData.subarray(0)
    if (view.length === 0) throw new Error('unreachable')
})
const hashState = decodeRustHashState(stateData)
time('app-state version: decode bincode HashState', () => {
    decodeRustHashState(stateData)
})
time('app-state version: encode bincode HashState', () => {
    encodeRustHashState(hashState)
})

// App-state sync keys are `{keyId, keyData}` on both sides — the translation is
// handing over two buffers, which is the shape the whole proposal wants.
const syncKey = fixture.appStateKeys[0]
const keyId = bytes(syncKey.keyId)
const keyData = bytes(syncKey.keyData)
time('app-state sync key: pass-through (two views)', () => {
    const a = keyId.subarray(0)
    const b = keyData.subarray(0)
    if (a.length + b.length === 0) throw new Error('unreachable')
})

// --- device -------------------------------------------------------------------

// No codec. `whatsapp-rust` stores each key pair as 64 bytes, private half
// first; zapo wants `{pubKey, privKey}`. So the translation is a rename and two
// subarrays per pair — and whether those are views or copies is the entire
// difference between the two rows below.
const device = fixture.device
const noise = bytes(device.noiseKey.privKey)
const joined = new Uint8Array(64)
joined.set(bytes(device.noiseKey.privKey), 0)
joined.set(bytes(device.noiseKey.pubKey), 32)

time('device: split one key pair (views)', () => {
    const pair = { privKey: joined.subarray(0, 32), pubKey: joined.subarray(32, 64) }
    if (pair.privKey.length === 0) throw new Error('unreachable')
})
time('device: split one key pair (copies)', () => {
    const pair = { privKey: joined.slice(0, 32), pubKey: joined.slice(32, 64) }
    if (pair.privKey.length === 0) throw new Error('unreachable')
})
time('device: whole record, three pairs + scalars', () => {
    const out = {
        registrationId: device.registrationId,
        noiseKey: { privKey: joined.subarray(0, 32), pubKey: joined.subarray(32, 64) },
        identityKey: { privKey: joined.subarray(0, 32), pubKey: joined.subarray(32, 64) },
        signedPreKey: { privKey: joined.subarray(0, 32), pubKey: joined.subarray(32, 64) },
        signedPreKeyId: device.signedPreKeyId,
        advSecretKey: noise.subarray(0),
        meJid: device.pn,
        meLid: device.lid,
    }
    if (out.registrationId === undefined) throw new Error('unreachable')
})
