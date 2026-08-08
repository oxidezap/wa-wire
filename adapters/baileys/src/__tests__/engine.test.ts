/**
 * The adapter against the engine, rather than against a shape of my own.
 *
 * Everything else in this package tests the adapter with hand-built nodes,
 * which proves it consistent with itself. What it cannot prove is that Baileys
 * hands over what this expects — and that is the half a mock always agrees
 * with. The `whatsapp-rust` adapter found a real off-by-one this way: its token
 * table resolved every token to its neighbour and nothing failed until a frame
 * the engine actually produced went through.
 */

import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { decodeBinaryNodeWithBuffer, encodeBinaryNode } from 'baileys'
import type { BinaryNode } from 'baileys'

import { PlaintextStatus, type Stanza } from '@oxidezap/wa-wire-ts'

import { PlaintextJoiner, type Node } from '../joiner.js'

const collector = () => {
    const seen: Stanza[] = []
    return { seen, sink: (stanza: Stanza) => void seen.push(stanza) }
}

describe('against Baileys itself', () => {
    it('forwards the bytes the engine decoded, not a re-encoding', async () => {
        const node: BinaryNode = {
            tag: 'receipt',
            attrs: { id: 'R1', from: '5511999998888@s.whatsapp.net', type: 'read' }
        }

        const { node: decoded, decompressed } = await decodeBinaryNodeWithBuffer(encodeBinaryNode(node))

        const { seen, sink } = collector()
        new PlaintextJoiner().acceptFrame(decoded as Node, decompressed, sink)

        assert.equal(seen.length, 1)
        // Byte for byte what the engine's own decoder consumed. A re-encoding
        // would mean the same and not be the same bytes, and anything comparing
        // two clients would read that as a difference between them.
        assert.deepEqual([...seen[0]!.frame], [...decompressed])
    })

    it('finds the <enc> children at the indices the engine reports', async () => {
        // The shape a real message has: `<enc>` is not the first child.
        const node: BinaryNode = {
            tag: 'message',
            attrs: { id: 'M1', from: '5511999998888@s.whatsapp.net', t: '1' },
            content: [
                { tag: 'participants', attrs: {} },
                { tag: 'enc', attrs: { v: '2', type: 'msg' }, content: new Uint8Array([1, 2, 3]) },
                { tag: 'enc', attrs: { v: '2', type: 'pkmsg' }, content: new Uint8Array([4, 5]) }
            ]
        }

        const { node: decoded, decompressed } = await decodeBinaryNodeWithBuffer(encodeBinaryNode(node))

        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()
        joiner.acceptFrame(decoded as Node, decompressed, sink)
        assert.equal(seen.length, 0, 'held for two payloads')

        // The indices Baileys reports are positions among *all* children, which
        // is what the adapter uses as the path. Counting `<enc>` nodes would
        // give 0 and 1 here, and address both plaintexts to the wrong nodes.
        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)
        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 2, plaintext: new Uint8Array([8]) }, sink)

        assert.equal(seen.length, 1)
        assert.deepEqual(
            seen[0]!.plaintexts?.map(entry => entry.path),
            [[1], [2]]
        )
        assert.equal(seen[0]!.plaintexts?.[0]?.status, PlaintextStatus.Ok)
    })

    it('hands over bytes that decode back to the same stanza', async () => {
        const node: BinaryNode = {
            tag: 'iq',
            attrs: { id: 'Q1', xmlns: 'w:p', type: 'get', to: 's.whatsapp.net' },
            content: [{ tag: 'ping', attrs: {} }]
        }

        const { node: decoded, decompressed } = await decodeBinaryNodeWithBuffer(encodeBinaryNode(node))
        const { seen, sink } = collector()
        new PlaintextJoiner().acceptFrame(decoded as Node, decompressed, sink)

        // What a consumer on the other side of the boundary does: parse the
        // frame it was handed. It has to arrive at the stanza the engine saw.
        const again = await decodeBinaryNodeWithBuffer(
            Buffer.concat([Buffer.of(0), Buffer.from(seen[0]!.frame)])
        )
        assert.equal(again.node.tag, 'iq')
        assert.equal(again.node.attrs.id, 'Q1')
        assert.deepEqual(again.node, decoded)
    })
})
