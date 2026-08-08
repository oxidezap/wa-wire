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
} from '@oxidezap/wa-wire-ts'
import {
    ArtifactClass,
    RecordKind,
    encodeRecording,
    type RecordInput,
} from '@oxidezap/wa-wire-ts'

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
        { tag: 'enc', attrs: { type: 'msg' }, content: new TextEncoder().encode('four') },
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
                { path: [3], status: PlaintextStatus.Unobserved, payload: new Uint8Array() },
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

/**
 * A container written here and read by the Rust crate.
 *
 * Every part the two implementations could disagree on is present: metadata of
 * each kind, a mark between two envelopes, an unknown ancillary tag, an unknown
 * record kind, and a trailer.
 */
const containerRecords: RecordInput[] = [
    { kind: RecordKind.Envelope, envelope: encodeEnvelope(fixtures[0]![1]) },
    { kind: RecordKind.Mark, deltaUs: 1_500, label: 'stream:error' },
    { kind: 0x7e, payload: new TextEncoder().encode('from a later writer') },
    { kind: RecordKind.Envelope, envelope: encodeEnvelope(fixtures[1]![1]) },
]

const container = encodeRecording(
    {
        adapter: {
            id: 'zapo',
            version: '0.1.0',
            engineVersion: '1.7',
            contractVersion: 1,
            capabilities: ['l0.inbound.tap', 'l0.plaintext', 'lifecycle.drain-hook'],
        },
        provenance: {
            whatsappVersion: '2.3000.1044659339',
            manifestHash: 'sha256:fixture',
            generatorVersion: '0.1.0',
        },
        // No dictionary tag on purpose. Declaring one is a claim that a
        // reader holding that table can parse these frames, and this fixture
        // exists to exercise the container rather than to make that claim.
        // A reader that met an identity it does not have would refuse, which
        // is right, and would stop this fixture testing anything else.
        artifactClass: ArtifactClass.Synthetic,
        inputDigest: new TextEncoder().encode('cross-language-fixture'),
        createdAt: 1_754_000_000_000n,
        note: 'written by emit-fixtures.ts, read by cross_language.rs',
        extra: [{ tag: 0x0042, value: new TextEncoder().encode('ancillary') }],
    },
    containerRecords,
)
writeFileSync(join(OUT, 'recording.wawr'), container)
console.log(`recording.wawr: ${container.length} bytes`)

// The same records with no trailer: what a ring buffer hands over when it is
// frozen, and what the Rust reader must still read.
const frozen = container.subarray(0, container.length - 13)
writeFileSync(join(OUT, 'recording-truncated.wawr'), frozen)
console.log(`recording-truncated.wawr: ${frozen.length} bytes`)
