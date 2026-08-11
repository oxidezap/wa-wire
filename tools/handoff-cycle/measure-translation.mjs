/**
 * What one translation costs, per domain, on real records.
 *
 * The other half of the per-access budget. `measure-store-access.mjs` counts how
 * often a live session reaches for the store; this measures what a translating
 * store would add to each of those calls.
 *
 * Measured on the committed fixture — a real paired session — rather than on a
 * synthetic record, because the cost scales with the record and a made-up
 * session is whatever size the person making it up chose.
 *
 * Three shapes, and the difference between them is the whole design question:
 *
 * - **Pass-through.** Both engines hold the same bytes, so the canonical form is
 *   those bytes and translation is a slice. This is the zero-copy case.
 * - **Field mapping.** Same values, different field names or framing. Cheap.
 * - **Codec.** One engine holds a protobuf and the other a decoded structure, so
 *   every access re-parses. This is the one that has to be affordable.
 *
 * Usage:
 *   node measure-translation.mjs [iterations]
 */

import { readFileSync } from 'node:fs'

import { decodeSignalSessionRecord, encodeSignalSessionRecord } from 'zapo-js/signal'

const ITERATIONS = Number(process.argv[2] ?? 2000)
const fixture = JSON.parse(
    readFileSync(new URL('./fixtures/whatsapp-rust-session.json', import.meta.url), 'utf-8')
)

const sessionBytes = new Uint8Array(Buffer.from(fixture.sessions[0].record, 'base64'))
const preKeyBytes = new Uint8Array(
    Buffer.from(fixture.preKeys[0].keyPair.pubKey, 'base64')
)

/** Median of `iterations` runs, in microseconds. */
function time(label, work) {
    // Warm up, so the first run's JIT does not become the answer.
    for (let i = 0; i < Math.min(200, ITERATIONS); i += 1) work()

    const samples = new Array(ITERATIONS)
    for (let i = 0; i < ITERATIONS; i += 1) {
        const started = process.hrtime.bigint()
        work()
        samples[i] = Number(process.hrtime.bigint() - started) / 1000
    }
    samples.sort((a, b) => a - b)
    const median = samples[Math.floor(samples.length / 2)]
    const p99 = samples[Math.floor(samples.length * 0.99)]
    console.log(`${label.padEnd(46)} ${median.toFixed(3).padStart(8)} µs   p99 ${p99.toFixed(3)} µs`)
    return median
}

console.log(`session record: ${sessionBytes.length} bytes, ${ITERATIONS} iterations\n`)
console.log(`${'translation'.padEnd(46)} ${'median'.padStart(8)}`)

// Pass-through: what a domain costs when both sides hold the same bytes. A
// subarray is a view, so this is the floor — the price of handing over a
// reference and nothing else.
const passthrough = time('pass-through (session bytes, no copy)', () => {
    const view = sessionBytes.subarray(0)
    if (view.length === 0) throw new Error('unreachable')
})

// A copy, for the case where the canonical store hands out owned buffers rather
// than views. Worth separating: it is the difference between zero-copy as a
// phrase and zero-copy as a fact.
time('copy (session bytes)', () => {
    const copy = sessionBytes.slice()
    if (copy.length === 0) throw new Error('unreachable')
})

time('copy (prekey public key, 32 bytes)', () => {
    const copy = preKeyBytes.slice()
    if (copy.length === 0) throw new Error('unreachable')
})

// The codec case: `zapo` holds a decoded session, `whatsapp-rust` holds the
// protobuf. Whichever way the canonical store leans, one of them pays this on
// every read and every write.
const decoded = decodeSignalSessionRecord(sessionBytes)
const decode = time('decode session record (proto -> zapo)', () => {
    decodeSignalSessionRecord(sessionBytes)
})
const encode = time('encode session record (zapo -> proto)', () => {
    encodeSignalSessionRecord(decoded)
})

console.log(
    `\npass-through is ${(decode / passthrough).toFixed(0)}x cheaper than decoding, ` +
        `and ${(decode + encode).toFixed(1)} µs is what a read-modify-write costs\n` +
        'when the canonical form is not the one an engine already holds.'
)
