/**
 * Turn what `zapo` handed back into a `whatsapp-rust` snapshot.
 *
 * The second leg of the move. What makes it worth running rather than assuming
 * is that the session is not what it was: `zapo` consumed prekeys while it held
 * the account, and a `whatsapp-rust` that reattached with its own stale store
 * would offer the server keys it had already given away. That is the drift R1
 * is about, one step short of two writers.
 *
 * Usage:
 *   node back-to-rust.mjs <harvested-zapo.json> <out-rust.json>
 */

import { readFileSync, writeFileSync } from 'node:fs'

import { migrate } from 'wa-store-migrate'

const [from, out] = process.argv.slice(2)
if (!from || !out) throw new Error('usage: back-to-rust.mjs <harvested.json> <out.json>')

const BYTE_FIELDS = new Set([
    'privKey', 'pubKey', 'key', 'keyData', 'keyId', 'identityKey', 'record',
    'advSecretKey', 'signature', 'hash', 'accountSignatureKey', 'accountSignature',
    'deviceSignature', 'details', 'companionEncStatic', 'serverStaticKey',
    'routingInfo', 'nctSalt', 'token', 'secret',
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

function encodeBytes(_key, value) {
    if (value instanceof Uint8Array) return Buffer.from(value).toString('base64')
    if (value?.type === 'Buffer' && Array.isArray(value.data)) {
        return Buffer.from(value.data).toString('base64')
    }
    if (value instanceof Map) return Object.fromEntries(value)
    return value
}

const harvested = decode(JSON.parse(readFileSync(from, 'utf-8')), '')
const back = migrate({ from: 'zapo', to: 'whatsapp-rust', data: harvested, validate: false })

for (const loss of back.losses) {
    console.log(`declared ${loss.severity}: ${loss.domain} (${loss.count}) — ${loss.reason}`)
}
if (back.losses.length === 0) console.log('declared: nothing')

writeFileSync(out, JSON.stringify(back.data, encodeBytes, 1))
console.log(
    JSON.stringify({
        preKeys: back.data.preKeys?.length ?? 0,
        sessions: back.data.sessions?.length ?? 0,
        identities: back.data.identities?.length ?? 0,
        appStateKeys: back.data.appStateKeys?.length ?? 0,
    })
)
