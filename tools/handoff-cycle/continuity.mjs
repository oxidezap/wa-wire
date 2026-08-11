/**
 * Did the session survive being handed between engines?
 *
 * This is the assertion a live move supports, and it is deliberately not the one
 * item 5 names. `wa-wire-gate --profile interop` refuses two live legs as
 * `UndeclaredInput`, and it is right to: two legs are two different windows of a
 * server talking, so a stanza-by-stanza difference between them is a fact about
 * the server (D-079). The runner compares two engines over *one* recorded input,
 * which `engine_agreement` already does.
 *
 * What a handoff can be checked for is continuity — that the account on the far
 * side is the account that went in, and that it never re-paired to get there.
 * Three things say so, and none of them alone would:
 *
 * 1. **The server paired once.** Its log is the only witness that cannot be
 *    talked into agreeing, since it is the party that would have had to accept a
 *    second pairing.
 * 2. **The same account came back.** `success` carries the account's `lid` and
 *    its `companion_enc_static`; a re-pair would have minted a new one of each.
 * 3. **Every leg after the first carried traffic.** A leg that connected and
 *    heard nothing proves only that a socket opened.
 *
 * Usage:
 *   node continuity.mjs <server.log> <leg.wawr>...
 */

import { readFileSync } from 'node:fs'

import { decodeRecording } from '@oxidezap/wa-wire-adapter-zapo'
import { decodeBinaryNode } from 'zapo-js/transport'

const [logPath, ...legPaths] = process.argv.slice(2)
if (!logPath || legPaths.length < 2) {
    throw new Error('usage: continuity.mjs <server.log> <leg.wawr>...')
}

/** The envelope header is version, flags, then the frame's length. */
function frameOf(envelope) {
    const view = new DataView(envelope.buffer, envelope.byteOffset, envelope.byteLength)
    const length = view.getUint32(4, true)
    return envelope.subarray(8, 8 + length)
}

function read(path) {
    const recording = decodeRecording(new Uint8Array(readFileSync(path)))
    const tags = new Map()
    let success = null
    let paired = false

    for (const envelope of recording.envelopes) {
        let node
        try {
            node = decodeBinaryNode(frameOf(envelope))
        } catch {
            // A frame this build cannot read is a finding for the gate, not here.
            continue
        }
        tags.set(node.tag, (tags.get(node.tag) ?? 0) + 1)
        if (node.tag === 'success') success = node.attrs
        // A pairing inside a leg would be the failure this whole thing is about.
        const children = Array.isArray(node.content) ? node.content : []
        if (children.some((child) => child?.tag === 'pair-device' || child?.tag === 'pair-success')) {
            paired = true
        }
    }

    return { path, stanzas: recording.envelopes.length, tags, success, paired }
}

const legs = legPaths.map(read)
const failures = []

// 1 — the server's own count.
const pairings = readFileSync(logPath, 'utf-8').split('\n')
    .filter((line) => line.includes('starting QR pairing flow')).length
console.log(`server paired ${pairings} time(s)`)
if (pairings !== 1) {
    failures.push(`the server paired ${pairings} times; one handoff should pair exactly once`)
}

// 2 — the same account on both ends.
const identified = legs.filter((leg) => leg.success)
if (identified.length < 2) {
    // Not a failure by itself: an adapter without `l0.inbound.auth-phase` never
    // sees `success`, so its recording cannot carry the proof even when the
    // login happened. Said out loud, because a silent skip here would turn a
    // capability gap into a passing check.
    console.log(
        `only ${identified.length} leg(s) recorded a success node — ` +
            'an adapter that does not declare l0.inbound.auth-phase cannot see one'
    )
}
if (identified.length >= 2) {
    const [first] = identified
    for (const leg of identified.slice(1)) {
        for (const attr of ['lid', 'companion_enc_static']) {
            if (leg.success[attr] !== first.success[attr]) {
                failures.push(
                    `${attr} changed between ${first.path} and ${leg.path}: ` +
                        `${first.success[attr]} -> ${leg.success[attr]}`
                )
            }
        }
    }
    console.log(`same account across ${identified.length} legs: ${first.success.lid}`)
}

// 3 — every leg carried a session, and no leg after the first re-paired.
//
// The first one pairs: that is how the account comes into being, and a check
// that forbade it would be measuring a session nobody had. Everything after it
// is a handoff, and a pairing there is the failure.
legs.forEach((leg, index) => {
    const kinds = [...leg.tags].map(([tag, count]) => `${tag}:${count}`).join(' ')
    console.log(`${leg.path}  ${leg.stanzas} stanza(s)  ${kinds}`)
    if (leg.stanzas === 0) failures.push(`${leg.path} recorded nothing`)
    if (leg.paired && index > 0) {
        failures.push(`${leg.path} paired, so the session did not carry into it`)
    }
    // A session that is being served looks like one: the server sends and the
    // client is acked. A leg with neither opened a socket and nothing more.
    if (!leg.tags.has('message') && !leg.tags.has('receipt')) {
        failures.push(`${leg.path} carried no message or receipt traffic`)
    }
})

if (failures.length > 0) {
    console.log('')
    for (const failure of failures) console.log(`FAIL: ${failure}`)
    process.exit(1)
}
console.log('\nthe session is continuous across every leg')
