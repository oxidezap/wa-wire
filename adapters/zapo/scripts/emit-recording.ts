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
import { INFO } from '../src/capability.js'
import {
    ArtifactClass,
    RecordKind,
    crc32,
    encodeRecording,
    type RecordInput,
} from '../src/recording.js'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..')
const CORPUS = join(ROOT, '..', '..', 'crates', 'wa-wire-conformance', 'corpus')
const OUT = join(ROOT, 'recordings', 'zapo.recording')

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

const records: RecordInput[] = []
const frames: Uint8Array[] = []
for (const name of names) {
    const frame = new Uint8Array(readFileSync(join(CORPUS, name)))
    frames.push(frame)
    // `decodeBinaryNode` takes the frame without the leading format byte, which
    // is exactly what the corpus holds.
    const node = decodeBinaryNode(frame)
    const envelope = adapterEnvelope(node)
    records.push({ kind: RecordKind.Envelope, envelope })
    console.log(`${name}: <${node.tag}> -> ${envelope.length} bytes`)
}

/**
 * Which traffic this is a replay of.
 *
 * The checksum of the corpus, in name order — the same order the Rust side
 * walks it in, so the two arrive at the same value without coordinating. This
 * is what makes the comparison a checked claim rather than a convention: two
 * recordings of *different* corpora are refused instead of reported as an
 * engine regression.
 */
function corpusDigest(): Uint8Array {
    let total = 0
    for (const frame of frames) total += frame.length
    const joined = new Uint8Array(total)
    let at = 0
    for (const frame of frames) {
        joined.set(frame, at)
        at += frame.length
    }
    const out = new Uint8Array(4)
    new DataView(out.buffer).setUint32(0, crc32(joined), true)
    return out
}

const bytes = encodeRecording(
    {
        adapter: {
            id: INFO.id,
            version: INFO.version,
            engineVersion: INFO.engineVersion,
            contractVersion: INFO.contractVersion,
            capabilities: INFO.capabilities,
        },
        // Replayed, not captured: these envelopes came from feeding the corpus
        // through the engine, so something else can have seen the same input.
        artifactClass: ArtifactClass.Replayed,
        inputDigest: corpusDigest(),
        note: 'the conformance corpus, replayed through zapo',
    },
    records,
)

writeFileSync(OUT, bytes)
console.log(`\n${OUT}: ${names.length} envelopes, ${bytes.length} bytes`)
