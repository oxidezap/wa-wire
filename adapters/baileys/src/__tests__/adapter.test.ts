import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { Capability, Direction, FrameOrigin, PlaintextStatus, type Stanza } from '@oxidezap/wa-wire-ts'

import { INFO, has } from '../capability.js'
import { PlaintextJoiner, type Node } from '../joiner.js'
import { waWire } from '../adapter.js'

/**
 * A stanza with its `<enc>` somewhere other than first.
 *
 * The realistic shape, and the one that catches an adapter numbering `<enc>`
 * nodes rather than children: here the payload's child index is 1, and an
 * adapter counting `<enc>` nodes would address it as 0 — a plaintext attached
 * to the wrong node, which reads as a message from the wrong sender.
 */
const message = (id: string, encs = 1): Node => ({
    tag: 'message',
    attrs: { id, from: '5511999998888@s.whatsapp.net' },
    content: [
        { tag: 'participants', attrs: {} },
        ...Array.from({ length: encs }, () => ({ tag: 'enc', attrs: { type: 'msg' } }))
    ]
})

const ack = (id: string): Node => ({ tag: 'ack', attrs: { id }, content: undefined })

const collector = () => {
    const seen: Stanza[] = []
    return { seen, sink: (stanza: Stanza) => void seen.push(stanza) }
}

describe('joining a stanza to its plaintexts', () => {
    it('sends a stanza with nothing encrypted straight through', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()

        joiner.acceptFrame(ack('A1'), new Uint8Array([1, 2]), sink)

        assert.equal(seen.length, 1)
        assert.equal(seen[0]!.plaintexts, undefined)
        assert.equal(joiner.pending, 0)
    })

    it('addresses a payload by the child index it came from', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()

        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)
        assert.equal(seen.length, 0, 'a message with an <enc> waits for it')

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)

        assert.equal(seen.length, 1)
        assert.deepEqual(seen[0]!.plaintexts?.[0]?.path, [1])
        assert.equal(seen[0]!.plaintexts?.[0]?.status, PlaintextStatus.Ok)
    })

    it('closes on the last payload rather than on a clock', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()
        joiner.acceptFrame(message('M1', 2), new Uint8Array([1]), sink)

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([1]) }, sink)
        assert.equal(seen.length, 0, 'one of two is not the last')

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 2, plaintext: new Uint8Array([2]) }, sink)
        assert.equal(seen.length, 1)
        assert.equal(seen[0]!.plaintexts?.length, 2)
    })

    it('leaves stanzas in the order they arrived', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()

        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)
        joiner.acceptFrame(ack('A1'), new Uint8Array([2]), sink)
        assert.equal(seen.length, 0, 'the ack is behind a held message')

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)

        // Wire order, not completion order. A recording compared position by
        // position would otherwise report the interleaving as a divergence in
        // whichever engine happened to be slower.
        assert.deepEqual(
            seen.map(stanza => stanza.frame[0]),
            [1, 2]
        )
    })

    it('reports an <enc> that produced nothing, at the position it occupies', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner(1)

        joiner.acceptFrame(message('M1', 2), new Uint8Array([1]), sink)
        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)
        joiner.acceptFrame(ack('A1'), new Uint8Array([2]), sink)
        joiner.acceptFrame(ack('A2'), new Uint8Array([3]), sink)

        const held = seen.find(stanza => stanza.plaintexts?.length === 2)
        assert.ok(held, 'the held stanza was emitted')
        assert.equal(held.plaintexts?.[0]?.status, PlaintextStatus.Ok)
        assert.equal(held.plaintexts?.[1]?.status, PlaintextStatus.Unobserved)
        assert.deepEqual(held.plaintexts?.[1]?.path, [2])
    })

    it('ages a pending stanza on every later one, held or not', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner(1)

        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)
        // Acks cross straight through and still count: a receive path carrying
        // nothing else would otherwise hold a message for ever.
        joiner.acceptFrame(ack('A1'), new Uint8Array([2]), sink)
        joiner.acceptFrame(ack('A2'), new Uint8Array([3]), sink)

        assert.equal(joiner.pending, 0, 'given up on after two later stanzas')
        assert.equal(seen.length, 3)
    })

    it('drops a payload for a child the stanza has no <enc> at', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()
        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 9, plaintext: new Uint8Array([0]) }, sink)
        assert.equal(seen.length, 0, 'an unexpected index must not close the stanza')

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)
        assert.equal(seen[0]?.plaintexts?.[0]?.payload[0], 9)
    })

    it('keeps the first stanza when an id repeats', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()

        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)
        joiner.acceptFrame(message('M1'), new Uint8Array([2]), sink)

        // Both were real, and this adapter reports every inbound stanza. What
        // the first gives up is the chance of a payload.
        assert.equal(seen.length, 1)
        assert.equal(seen[0]!.frame[0], 1)
        assert.equal(seen[0]!.plaintexts?.[0]?.status, PlaintextStatus.Unobserved)

        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([9]) }, sink)
        assert.equal(seen[1]?.frame[0], 2)
        assert.equal(seen[1]?.plaintexts?.[0]?.status, PlaintextStatus.Ok)
    })

    it('copies the frame out of the engine buffer', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()

        const frame = new Uint8Array([1, 2, 3])
        joiner.acceptFrame(message('M1'), frame, sink)
        frame.set([9, 9, 9])
        joiner.acceptPlaintext({ messageId: 'M1', childIndex: 1, plaintext: new Uint8Array([0]) }, sink)

        assert.deepEqual([...seen[0]!.frame], [1, 2, 3])
    })

    it('emits what is still waiting on flush', () => {
        const { seen, sink } = collector()
        const joiner = new PlaintextJoiner()
        joiner.acceptFrame(message('M1'), new Uint8Array([1]), sink)

        joiner.flush(sink)

        assert.equal(seen.length, 1, 'the stanza was real either way')
        assert.equal(joiner.queued, 0)
    })

    it('drops a payload for a stanza nobody is holding', () => {
        const { seen, sink } = collector()
        new PlaintextJoiner().acceptPlaintext(
            { messageId: 'nobody', childIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )
        assert.equal(seen.length, 0)
    })
})

describe('what the adapter declares', () => {
    it('claims what Baileys provides', () => {
        for (const capability of [
            Capability.L0InboundTap,
            // The frame hook is inside the Noise loop, before anything decides
            // what a stanza is, so the authentication exchange reaches it.
            Capability.L0InboundAuthPhase,
            Capability.L0Plaintext,
            // The hook carries the buffer the node was decoded from, so nothing
            // is re-encoded.
            Capability.ZeroCopyFrame
        ]) {
            assert.ok(has(capability), `INFO lacks ${capability}`)
        }
    })

    it('does not claim what Baileys does not offer', () => {
        for (const capability of [
            // Nothing reports what the client sent.
            Capability.L0OutboundObserved,
            // Nothing says when handlers have drained.
            Capability.DrainHook,
            // The frame hook observes; the pipeline runs regardless.
            Capability.Takeover
        ]) {
            assert.ok(!has(capability), `INFO claims ${capability}`)
        }
    })

    it('names the contract version it writes', () => {
        assert.equal(INFO.contractVersion, 1)
        assert.equal(INFO.id, 'baileys')
    })
})

describe('the installed adapter', () => {
    it('forwards a stanza through the two callbacks', () => {
        const { seen, sink } = collector()
        const wire = waWire(sink)

        wire.config.onFrameDecoded(message('M1'), new Uint8Array([1, 2]))
        wire.config.onDecryptedPayload({
            stanza: message('M1'),
            childIndex: 1,
            encType: 'msg',
            plaintext: new Uint8Array([7]),
            unpadded: true
        })

        assert.equal(seen.length, 1)
        assert.equal(seen[0]!.direction, Direction.Inbound)
        assert.equal(seen[0]!.frameOrigin, FrameOrigin.Original)
        assert.deepEqual([...seen[0]!.frame], [1, 2])
        assert.equal(seen[0]!.plaintexts?.[0]?.payload[0], 7)
    })

    it('ignores a frame from before the transport is up', () => {
        const { seen, sink } = collector()
        const wire = waWire(sink)

        // The handshake exchange: bytes, not a node, and nothing to pair with.
        wire.config.onFrameDecoded(new Uint8Array([1, 2, 3]), undefined)

        assert.equal(seen.length, 0)
        assert.equal(wire.pending, 0)
    })

    it('empties what is held on flush', () => {
        const { seen, sink } = collector()
        const wire = waWire(sink)

        wire.config.onFrameDecoded(message('M1'), new Uint8Array([1]))
        assert.equal(wire.pending, 1)

        wire.flush()
        assert.equal(seen.length, 1)
        assert.equal(wire.pending, 0)
    })
})
