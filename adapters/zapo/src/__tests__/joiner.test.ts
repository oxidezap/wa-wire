import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { BinaryNode } from 'zapo-js'

import { PlaintextStatus, type Stanza } from '../envelope.js'
import { PlaintextJoiner } from '../joiner.js'

/** A `<message>` whose children are the given `<enc>` types, in order. */
function message(id: string | undefined, encTypes: readonly string[]): BinaryNode {
    return {
        tag: 'message',
        attrs: id === undefined ? {} : { id, from: '5511999998888@s.whatsapp.net' },
        content: encTypes.map((type) => ({
            tag: 'enc',
            attrs: { type },
            content: new Uint8Array([1, 2, 3]),
        })),
    }
}

/** A frame stands in for the encoding; the joiner never looks inside it. */
function frame(marker: number): Uint8Array {
    return new Uint8Array([marker])
}

function collector(): { readonly stanzas: Stanza[]; readonly sink: (s: Stanza) => void } {
    const stanzas: Stanza[] = []
    return { stanzas, sink: (s) => stanzas.push(s) }
}

describe('stanzas that never wait', () => {
    it('passes a stanza with no enc straight through', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode({ tag: 'receipt', attrs: { id: 'R1' } }, frame(1), sink)

        assert.equal(stanzas.length, 1)
        assert.equal(joiner.waiting, 0)
        assert.equal(stanzas[0]?.plaintexts, undefined)
    })

    it('does not hold a message without an id', () => {
        // Nothing could match a payload back to it, so waiting would only delay
        // the stanza and then emit it unobserved.
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message(undefined, ['msg']), frame(1), sink)

        assert.equal(stanzas.length, 1)
        assert.equal(joiner.waiting, 0)
    })

    it('leaves a fan-out stanza as L0-wire', () => {
        // The engine numbers `<participants><to>` encs separately, and without
        // the device's own JID the index cannot be resolved to a node. Saying
        // nothing beats attaching a payload to the wrong `<enc>`.
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()
        const node: BinaryNode = {
            tag: 'message',
            attrs: { id: 'FAN1' },
            content: [
                { tag: 'enc', attrs: { type: 'skmsg' }, content: new Uint8Array([1]) },
                {
                    tag: 'participants',
                    attrs: {},
                    content: [
                        {
                            tag: 'to',
                            attrs: { jid: '5511999998888:1@s.whatsapp.net' },
                            content: [
                                { tag: 'enc', attrs: { type: 'pkmsg' }, content: new Uint8Array([2]) },
                            ],
                        },
                    ],
                },
            ],
        }

        joiner.acceptNode(node, frame(1), sink)

        assert.equal(stanzas.length, 1, 'emitted immediately')
        assert.equal(joiner.waiting, 0, 'and never waits')
        assert.equal(stanzas[0]?.plaintexts, undefined)
        assert.equal(joiner.abandoned, 0, 'not giving up — never started')
    })
})

describe('the common case', () => {
    it('waits for its plaintext and then emits once', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M1', ['msg']), frame(7), sink)
        assert.equal(stanzas.length, 0, 'nothing emitted while waiting')
        assert.equal(joiner.waiting, 1)

        joiner.acceptPlaintext(
            { messageId: 'M1', encIndex: 0, plaintext: new Uint8Array([9, 9]) },
            sink
        )

        assert.equal(stanzas.length, 1, 'one envelope, not two')
        assert.equal(joiner.waiting, 0)
        assert.deepEqual(stanzas[0]?.frame, frame(7), 'the frame it was holding')
        assert.deepEqual(stanzas[0]?.plaintexts, [
            { path: [0], status: PlaintextStatus.Ok, payload: new Uint8Array([9, 9]) },
        ])
        assert.equal(joiner.abandoned, 0)
    })

    it('releases a multi-enc message on its last plaintext', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M2', ['pkmsg', 'msg']), frame(1), sink)
        joiner.acceptPlaintext(
            { messageId: 'M2', encIndex: 1, plaintext: new Uint8Array([2]) },
            sink
        )
        assert.equal(stanzas.length, 0, 'still one short')

        joiner.acceptPlaintext(
            { messageId: 'M2', encIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )

        assert.equal(stanzas.length, 1)
        assert.deepEqual(
            stanzas[0]?.plaintexts?.map((p) => [p.path, [...p.payload]]),
            [
                [[0], [1]],
                [[1], [2]],
            ],
            'in stanza order, whatever order they arrived in'
        )
    })

    it('addresses the enc among all children, not among the encs', () => {
        // The engine counts `<enc>` nodes; the envelope addresses children. A
        // stanza carrying anything else first makes the two differ, and a
        // payload on the wrong node is a message from the wrong sender.
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()
        const node: BinaryNode = {
            tag: 'message',
            attrs: { id: 'M3' },
            content: [
                { tag: 'device-identity', attrs: {}, content: new Uint8Array([9]) },
                { tag: 'enc', attrs: { type: 'msg' }, content: new Uint8Array([1]) },
            ],
        }

        joiner.acceptNode(node, frame(1), sink)
        joiner.acceptPlaintext(
            { messageId: 'M3', encIndex: 0, plaintext: new Uint8Array([5]) },
            sink
        )

        assert.deepEqual(
            stanzas[0]?.plaintexts?.map((p) => p.path),
            [[1]],
            'child 1, though it is the stanza first <enc>'
        )
    })
})

describe('giving up', () => {
    it('emits a stanza whose plaintext never comes as unobserved', () => {
        const joiner = new PlaintextJoiner(2)
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M5', ['msg']), frame(1), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(2), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(3), sink)
        assert.equal(joiner.waiting, 1, 'still waiting at the limit')

        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(4), sink)

        assert.equal(joiner.waiting, 0, 'one stanza past the limit gives up')
        assert.equal(joiner.abandoned, 1)
        const abandoned = stanzas.find((s) => s.plaintexts !== undefined)
        assert.deepEqual(abandoned?.plaintexts, [
            { path: [0], status: PlaintextStatus.Unobserved, payload: new Uint8Array() },
        ])
    })

    it('keeps the payloads a partly decrypted message did get', () => {
        const joiner = new PlaintextJoiner(1)
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M6', ['msg', 'skmsg']), frame(1), sink)
        joiner.acceptPlaintext(
            { messageId: 'M6', encIndex: 0, plaintext: new Uint8Array([7]) },
            sink
        )
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(2), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(3), sink)

        assert.equal(joiner.abandoned, 1)
        const abandoned = stanzas.find((s) => s.plaintexts?.length === 2)
        assert.deepEqual(
            abandoned?.plaintexts?.map((p) => p.status),
            [PlaintextStatus.Ok, PlaintextStatus.Unobserved]
        )
    })

    it('drops a late plaintext rather than attaching it to nothing', () => {
        const joiner = new PlaintextJoiner(1)
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M7', ['msg']), frame(1), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(2), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(3), sink)
        const emitted = stanzas.length

        joiner.acceptPlaintext(
            { messageId: 'M7', encIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )

        assert.equal(stanzas.length, emitted, 'no second envelope for an emitted stanza')
    })

    it('ignores an enc index the stanza does not have', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M8', ['msg']), frame(1), sink)
        joiner.acceptPlaintext(
            { messageId: 'M8', encIndex: 7, plaintext: new Uint8Array([1]) },
            sink
        )
        assert.equal(stanzas.length, 0, 'still waiting for its own enc')

        joiner.acceptPlaintext(
            { messageId: 'M8', encIndex: 0, plaintext: new Uint8Array([2]) },
            sink
        )
        assert.deepEqual(stanzas[0]?.plaintexts?.map((p) => p.status), [PlaintextStatus.Ok])
    })

    it('drops a plaintext for a stanza it never saw', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptPlaintext(
            { messageId: 'never-seen', encIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )

        assert.equal(stanzas.length, 0)
        assert.equal(joiner.waiting, 0)
    })
})

describe('ordering and shutdown', () => {
    it('lets stanzas that do not wait pass one that does', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M11', ['msg']), frame(1), sink)
        joiner.acceptNode({ tag: 'receipt', attrs: {} }, frame(2), sink)
        joiner.acceptNode({ tag: 'notification', attrs: {} }, frame(3), sink)

        assert.equal(stanzas.length, 2, 'both passed the waiting message')
        joiner.acceptPlaintext(
            { messageId: 'M11', encIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )
        assert.equal(stanzas.length, 3, 'and it arrives after them')
    })

    it('waits on two messages independently', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('A', ['msg']), frame(1), sink)
        joiner.acceptNode(message('B', ['msg']), frame(2), sink)
        assert.equal(joiner.waiting, 2)

        joiner.acceptPlaintext({ messageId: 'B', encIndex: 0, plaintext: new Uint8Array([2]) }, sink)
        assert.equal(joiner.waiting, 1, 'only B was released')
        assert.deepEqual(stanzas[0]?.frame, frame(2))
    })

    it('emits what is still waiting on flush', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.acceptNode(message('M9', ['msg']), frame(1), sink)
        joiner.acceptNode(message('M10', ['msg']), frame(2), sink)
        joiner.acceptPlaintext(
            { messageId: 'M10', encIndex: 0, plaintext: new Uint8Array([1]) },
            sink
        )
        assert.equal(joiner.waiting, 1)

        joiner.flush(sink)

        assert.equal(joiner.waiting, 0)
        assert.equal(stanzas.length, 2, 'the complete one and the flushed one')
        assert.equal(joiner.abandoned, 1, 'flushing an incomplete one counts')
    })

    it('does nothing when flushing an empty joiner', () => {
        const joiner = new PlaintextJoiner()
        const { stanzas, sink } = collector()

        joiner.flush(sink)

        assert.equal(stanzas.length, 0)
        assert.equal(joiner.abandoned, 0)
    })
})
