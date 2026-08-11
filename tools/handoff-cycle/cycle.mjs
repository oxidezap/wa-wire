/**
 * Move one session out to another engine and back, and check that what came
 * back differs from what left in exactly the ways the route declared.
 *
 * The route is `whatsapp-rust → zapo → whatsapp-rust`, which D-136 prefers
 * because nothing is lost moving into zapo. The interesting half is the return
 * leg: whatsapp-rust cannot write `contacts` or `messageSecrets`, and both
 * adapters call `appStateVersions` lossy.
 *
 * The comparison is against the canonical snapshot the first leg produced,
 * **byte for byte**, and the question it answers is not "did the round trip
 * work" — counting rows would answer that, and would pass a trip that returned
 * 807 prekeys with the wrong bytes in them. The question is whether the
 * difference is exactly the one `planLosses` named.
 */

import { readFileSync } from 'node:fs'
import { migrate, planLosses, ALL_DOMAINS } from 'wa-store-migrate'

const path = process.argv[2]
if (!path) throw new Error('usage: cycle.mjs <rust-snapshot.json>')

const BYTE_FIELDS = new Set([
    'privKey', 'pubKey', 'key', 'record', 'keyId', 'keyData', 'stateData',
    'indexMac', 'valueMac', 'token', 'signedPreKeySignature', 'advSecretKey',
    'account', 'edgeRoutingInfo', 'nctSalt', 'bytes',
])

/** Base64 back to bytes, wherever the dumper put a string. */
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

const original = decode(JSON.parse(readFileSync(path, 'utf-8')), '')

const out = migrate({ from: 'whatsapp-rust', to: 'zapo', data: original, validate: false })
const back = migrate({ from: 'zapo', to: 'whatsapp-rust', data: out.data, validate: false })

for (const [leg, result] of [['whatsapp-rust -> zapo', out], ['zapo -> whatsapp-rust', back]]) {
    console.log(`leg  ${leg}`)
    if (result.losses.length === 0) console.log('     declared: nothing')
    for (const loss of result.losses) {
        console.log(`     declared ${loss.severity}: ${loss.domain} (${loss.count})`)
    }
}

// --- compare the two canonical snapshots, all the way down -------------------

/** A stable, comparable rendering of anything the IR holds. */
function normalise(value) {
    if (value instanceof Uint8Array) return `b64:${Buffer.from(value).toString('base64')}`
    if (value instanceof Map) {
        return Object.fromEntries(
            [...value.entries()].sort(([a], [b]) => String(a).localeCompare(String(b)))
                .map(([k, v]) => [String(k), normalise(v)])
        )
    }
    if (Array.isArray(value)) return value.map(normalise)
    if (value && typeof value === 'object') {
        return Object.fromEntries(
            Object.entries(value).sort(([a], [b]) => a.localeCompare(b))
                .map(([k, v]) => [k, normalise(v)])
        )
    }
    return value === undefined ? null : value
}

/** Every path at which `left` and `right` differ. */
function differences(left, right, at = '') {
    const a = normalise(left)
    const b = normalise(right)
    if (JSON.stringify(a) === JSON.stringify(b)) return []
    if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') {
        return [{ at, left: preview(a), right: preview(b) }]
    }
    if (Array.isArray(a) !== Array.isArray(b)) return [{ at, left: 'array', right: 'object' }]
    if (Array.isArray(a)) {
        if (a.length !== b.length) return [{ at, left: `${a.length} entries`, right: `${b.length} entries` }]
        return a.flatMap((entry, index) => differences(entry, b[index], `${at}[${index}]`))
    }
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort()
    return keys.flatMap((key) => differences(a[key], b[key], at ? `${at}.${key}` : key))
}

function preview(value) {
    const text = typeof value === 'string' ? value : JSON.stringify(value)
    return text && text.length > 48 ? `${text.slice(0, 48)}…` : text
}

const declared = new Map()
for (const loss of [...out.losses, ...back.losses]) declared.set(loss.domain, loss.severity)

console.log('\ndomain                    entries   verdict')
let undeclared = 0
for (const domain of ALL_DOMAINS) {
    const before = out.snapshot[domain]
    const after = back.snapshot[domain]
    const size = (value) =>
        value === undefined || value === null ? 0
            : Array.isArray(value) ? value.length
                : value instanceof Map ? value.size : 1
    if (size(before) === 0 && size(after) === 0) continue

    const diff = differences(before, after, domain)
    const claim = declared.get(domain)
    let verdict
    if (diff.length === 0) {
        verdict = claim ? `identical (declared ${claim} anyway)` : 'identical'
    } else if (claim) {
        verdict = `differs — declared ${claim}`
    } else {
        verdict = `DIFFERS, UNDECLARED (${diff.length})`
        undeclared += 1
        for (const entry of diff.slice(0, 4)) {
            console.log(`   ${entry.at}: ${entry.left} -> ${entry.right}`)
        }
    }
    console.log(`${domain.padEnd(24)} ${String(size(before)).padStart(6)}   ${verdict}`)
}

if (undeclared > 0) {
    console.log(`\n${undeclared} domain(s) changed that the route did not declare`)
    process.exit(1)
}
console.log('\nevery byte that changed was on a domain the route declared')
