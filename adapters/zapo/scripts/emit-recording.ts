/**
 * Replay the conformance corpus through `zapo` and write what this adapter
 * produced.
 *
 * Each corpus file is a frame as an engine receives it. `zapo` decodes it with
 * its own decoder, and the adapter re-encodes it with its own encoder — so the
 * envelopes written here are bytes `whatsapp-rust` would never produce for the
 * same input. That is the point: two encodings of one stanza have to derive the
 * same L1, and this is the half of that comparison written in TypeScript.
 *
 * ```sh
 * npx tsx scripts/emit-recording.ts
 * ```
 *
 * Committed output, regenerated in CI with no diff allowed.
 */

import { readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { decodeBinaryNode, encodeBinaryNode } from 'zapo-js/transport'
import type { BinaryNode } from 'zapo-js'

import { Direction, FrameOrigin, encodeEnvelope, type Stanza } from '../src/envelope.js'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..')
const CORPUS = join(ROOT, '..', '..', 'crates', 'wa-wire-conformance', 'corpus')
const OUT = join(ROOT, 'recordings', 'zapo.recording')

/**
 * A recording file: `WAWR`, a u32 count, then each envelope length-prefixed.
 *
 * Deliberately trivial. The envelope format is the contract; this is only a way
 * to put several of them in one file, and a reader that needs a spec for the
 * container is a reader spending attention in the wrong place.
 */
function encodeRecording(envelopes: readonly Uint8Array[]): Uint8Array {
    const total =
        4 + 4 + envelopes.reduce((sum, envelope) => sum + 4 + envelope.length, 0)
    const out = new Uint8Array(total)
    const view = new DataView(out.buffer)
    out.set(new TextEncoder().encode('WAWR'), 0)
    view.setUint32(4, envelopes.length, false)

    let offset = 8
    for (const envelope of envelopes) {
        view.setUint32(offset, envelope.length, false)
        offset += 4
        out.set(envelope, offset)
        offset += envelope.length
    }
    return out
}

/** What the adapter does with one stanza, minus the plugin plumbing. */
function adapterEnvelope(node: BinaryNode): Uint8Array {
    const stanza: Stanza = {
        direction: Direction.Inbound,
        // The engine hands the filter a decoded node, not the buffer it came
        // from, so the frame has to be re-encoded — and the envelope says so.
        frameOrigin: FrameOrigin.ReEncoded,
        frame: new Uint8Array(encodeBinaryNode(node)),
    }
    return encodeEnvelope(stanza)
}

/** Both the written corpus and anything captured, in one sorted list. */
function corpusFiles(): string[] {
    const files: string[] = []
    for (const dir of ['', 'captured']) {
        const full = dir ? join(CORPUS, dir) : CORPUS
        let entries: string[]
        try {
            entries = readdirSync(full)
        } catch {
            continue
        }
        for (const name of entries) {
            if (name.endsWith('.bin')) files.push(dir ? join(dir, name) : name)
        }
    }
    return files.sort()
}

const names = corpusFiles()
if (names.length === 0) {
    throw new Error(`no corpus in ${CORPUS} — run \`cargo run --example emit-corpus\``)
}

const envelopes: Uint8Array[] = []
for (const name of names) {
    const frame = new Uint8Array(readFileSync(join(CORPUS, name)))
    // `decodeBinaryNode` takes the frame without the leading format byte, which
    // is exactly what the corpus holds.
    const node = decodeBinaryNode(Buffer.from(frame))
    envelopes.push(adapterEnvelope(node))
    console.log(`${name}: <${node.tag}> -> ${envelopes[envelopes.length - 1]?.length} bytes`)
}

writeFileSync(OUT, encodeRecording(envelopes))
console.log(`\n${OUT}: ${names.length} envelopes`)
