/**
 * Emit the cross-language fixtures.
 *
 * These are envelopes written by this TypeScript encoder and decoded by the
 * Rust one, which is the only way to know the two agree. A format described in
 * two languages and tested in one is a format with an untested half.
 *
 *     npx tsx scripts/emit-fixtures.ts
 *
 * The output is committed. CI regenerates and requires no diff, so a change to
 * either encoder shows up as a failing check rather than as traffic nobody can
 * read.
 */

import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { encodeBinaryNode } from 'zapo-js/transport'
import type { BinaryNode } from 'zapo-js'

import {
    Direction,
    FrameOrigin,
    PlaintextStatus,
    encodeEnvelope,
    type Stanza,
} from '../src/envelope.js'

const HERE = dirname(fileURLToPath(import.meta.url))
const OUT = join(HERE, '..', 'fixtures')

function frame(node: BinaryNode): Uint8Array {
    return encodeBinaryNode(node)
}

const receipt: BinaryNode = {
    tag: 'receipt',
    attrs: { id: 'ABCD1234', from: '5511999998888@s.whatsapp.net', type: 'read' },
}

const messageWithEnc: BinaryNode = {
    tag: 'message',
    attrs: { id: 'MSG1', from: '5511999998888@s.whatsapp.net', t: '1700000000' },
    content: [
        {
            tag: 'enc',
            attrs: { v: '2', type: 'msg' },
            content: new TextEncoder().encode('ciphertext-bytes'),
        },
    ],
}

const multiDevice: BinaryNode = {
    tag: 'message',
    attrs: { id: 'MSG2', from: '5511999998888@s.whatsapp.net' },
    content: [
        { tag: 'enc', attrs: { type: 'msg' }, content: new TextEncoder().encode('one') },
        { tag: 'enc', attrs: { type: 'pkmsg' }, content: new TextEncoder().encode('two') },
        { tag: 'enc', attrs: { type: 'skmsg' }, content: new TextEncoder().encode('three') },
    ],
}

const fixtures: ReadonlyArray<readonly [string, Stanza]> = [
    [
        'receipt',
        { direction: Direction.Inbound, frameOrigin: FrameOrigin.ReEncoded, frame: frame(receipt) },
    ],
    [
        'message-with-enc',
        {
            direction: Direction.Inbound,
            frameOrigin: FrameOrigin.ReEncoded,
            frame: frame(messageWithEnc),
        },
    ],
    [
        'outbound-verbatim',
        {
            direction: Direction.Outbound,
            frameOrigin: FrameOrigin.Original,
            frame: frame(receipt),
        },
    ],
    [
        // The shape path addressing exists for: one plaintext per <enc>.
        'multi-device-with-plaintexts',
        {
            direction: Direction.Inbound,
            frameOrigin: FrameOrigin.Original,
            frame: frame(multiDevice),
            plaintexts: [
                { path: [0], status: PlaintextStatus.Ok, payload: new TextEncoder().encode('plain-one') },
                { path: [1], status: PlaintextStatus.DecryptFailed, payload: new Uint8Array() },
                { path: [2], status: PlaintextStatus.Unsupported, payload: new Uint8Array() },
            ],
        },
    ],
    [
        // The root path addresses the stanza itself.
        'root-path-plaintext',
        {
            direction: Direction.Inbound,
            frameOrigin: FrameOrigin.Original,
            frame: frame(receipt),
            plaintexts: [
                { path: [], status: PlaintextStatus.Ok, payload: new TextEncoder().encode('whole') },
            ],
        },
    ],
    [
        'empty-frame',
        {
            direction: Direction.Inbound,
            frameOrigin: FrameOrigin.Original,
            frame: new Uint8Array(),
        },
    ],
]

mkdirSync(OUT, { recursive: true })
for (const [name, stanza] of fixtures) {
    const bytes = encodeEnvelope(stanza)
    writeFileSync(join(OUT, `${name}.bin`), bytes)
    console.log(`${name}.bin: ${bytes.length} bytes`)
}
