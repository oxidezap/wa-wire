/**
 * Run the conformance corpus through this engine.
 *
 * The corpus is frames as an engine receives them: decompressed node bytes,
 * without the format byte. Each is decoded by Baileys' own decoder and
 * forwarded as this adapter would forward it, and the envelopes are written for
 * the Rust side to compare against the other three engines'.
 *
 * Envelopes as files rather than a recording container. The container carries
 * the claims a *gate* needs — which traffic, which adapter, whether the file is
 * whole — and none is in question here: the comparison is driven from the Rust
 * side, which builds a recording of its own around these.
 *
 *     npx tsx scripts/replay-corpus.ts
 */

import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { decodeBinaryNodeWithBuffer, encodeBinaryNode } from 'baileys'

import { Direction, FrameOrigin, encodeEnvelope, verify } from '@oxidezap/wa-wire-ts'

import { INFO } from '../src/capability.js'

const root = process.argv[2] ?? '../../crates/wa-wire-conformance/corpus'
const out = process.argv[3] ?? 'replay'

/**
 * Every corpus frame, in name order.
 *
 * The same order every replay uses, since the comparison aligns by position.
 */
const corpus = (): { name: string; bytes: Buffer }[] => {
    const frames: { name: string; bytes: Buffer }[] = []
    for (const dir of [root, join(root, 'captured')]) {
        let entries: string[]
        try {
            entries = readdirSync(dir)
        } catch (error) {
            if (dir === root) {
                throw error
            }

            continue
        }

        for (const entry of entries) {
            if (!entry.endsWith('.bin')) {
                continue
            }

            const path = join(dir, entry)
            // Prefixed with the directory, so a captured frame and a written
            // one cannot collide on a name.
            frames.push({ name: path.slice(root.length + 1), bytes: readFileSync(path) })
        }
    }

    return frames.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
}

const frames = corpus()
if (frames.length === 0) {
    throw new Error(`${root}: no corpus frames`)
}

rmSync(out, { recursive: true, force: true })
mkdirSync(out, { recursive: true })

let written = 0
for (const frame of frames) {
    // The engine's own decoder, which is the point: a frame this adapter
    // forwards is one Baileys agreed was a stanza.
    //
    // Prefixed with a zero format byte, since the corpus holds what the decoder
    // consumes and the entry point takes what the transport delivers. What
    // comes back out is the corpus frame again, which is asserted below rather
    // than assumed.
    const { node, decompressed } = await decodeBinaryNodeWithBuffer(
        Buffer.concat([Buffer.of(0), frame.bytes])
    )
    if (!decompressed.equals(frame.bytes)) {
        throw new Error(`${frame.name}: the decoder consumed different bytes than the corpus holds`)
    }

    // Forwarded as it stands, like the other replays: the frame path is what
    // this compares, and a plaintext table needs Signal to have run.
    const stanza = {
        direction: Direction.Inbound,
        frameOrigin: FrameOrigin.Original,
        frame: new Uint8Array(frame.bytes)
    }
    const violation = verify(INFO, stanza)
    if (violation) {
        throw new Error(`${frame.name}: ${violation}`)
    }

    writeFileSync(join(out, `${String(written).padStart(4, '0')}.envelope`), encodeEnvelope(stanza))

    // And the engine's own *encoder*, written alongside.
    //
    // The adapter forwards verbatim, so its envelopes are the corpus bytes and
    // every zero-copy engine's are identical — which makes comparing them a
    // comparison of nothing. Re-encoding is where four engines genuinely
    // differ: each is entitled to write a value its own way, and the property
    // under test is that all four still derive the same event.
    //
    // `encodeBinaryNode` writes the format byte the decoder strips.
    const reEncoded = encodeBinaryNode(node).subarray(1)
    writeFileSync(
        join(out, `${String(written).padStart(4, '0')}.reencoded`),
        encodeEnvelope({
            direction: Direction.Inbound,
            frameOrigin: FrameOrigin.ReEncoded,
            frame: new Uint8Array(reEncoded)
        })
    )
    written += 1
}

console.log(`${out}: ${written} envelope(s) from ${frames.length} corpus frame(s)`)
