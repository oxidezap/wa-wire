import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { BinaryNode } from 'zapo-js'
import { encodeBinaryNode } from 'zapo-js/transport'

import { Capability, INFO, SENDING_INFO, declares } from '../capability.js'
import { NotConnectedError, SendError, createSender } from '../send.js'

function receipt(): BinaryNode {
    return { tag: 'receipt', attrs: { id: 'R1', from: '5511999998888@s.whatsapp.net' } }
}

describe('sending a stanza', () => {
    it('hands the engine the stanza the frame holds', async () => {
        const sent: BinaryNode[] = []
        const sender = createSender({
            sendNode: async (node) => {
                sent.push(node)
            },
        })

        await sender.sendFrame(new Uint8Array(encodeBinaryNode(receipt())))

        assert.equal(sent.length, 1)
        assert.equal(sent[0]?.tag, 'receipt')
        assert.equal(sent[0]?.attrs.id, 'R1')
    })

    it('round-trips a frame it forwarded inbound', async () => {
        // The property replay rests on: what the adapter hands a consumer is
        // what the consumer can hand back. A frame that only survives one
        // direction makes a recorded session unreplayable, and nothing else
        // would say so.
        const original = receipt()
        const frame = new Uint8Array(encodeBinaryNode(original))

        const sent: BinaryNode[] = []
        const sender = createSender({
            sendNode: async (node) => {
                sent.push(node)
            },
        })
        await sender.sendFrame(frame)

        assert.deepEqual(
            new Uint8Array(encodeBinaryNode(sent[0]!)),
            frame,
            'the stanza that went out encodes back to the frame that came in'
        )
    })

    it('reports a frame it cannot read without touching the socket', async () => {
        // Distinguished from an engine refusal on purpose: nothing was sent, and
        // the fix is in what the caller passed.
        let called = false
        const sender = createSender({
            sendNode: async () => {
                called = true
            },
        })

        await assert.rejects(
            () => sender.sendFrame(new Uint8Array([0xff, 0xff, 0xff])),
            (error: unknown) => error instanceof SendError && !(error instanceof NotConnectedError)
        )
        assert.equal(called, false, 'the engine was never asked')
    })

    it('surfaces a disconnected engine as its own failure', async () => {
        // The one a consumer can act on without knowing the engine.
        const sender = createSender({
            sendNode: async () => {
                throw new Error('socket is closed')
            },
        })

        await assert.rejects(
            () => sender.sendFrame(new Uint8Array(encodeBinaryNode(receipt()))),
            NotConnectedError
        )
    })

    it('keeps any other engine failure as an engine failure', async () => {
        const sender = createSender({
            sendNode: async () => {
                throw new Error('rate limited')
            },
        })

        await assert.rejects(
            () => sender.sendFrame(new Uint8Array(encodeBinaryNode(receipt()))),
            (error: unknown) =>
                error instanceof SendError &&
                !(error instanceof NotConnectedError) &&
                /engine refused/.test(error.message)
        )
    })

    it('keeps the engine error reachable for a report', async () => {
        const cause = new Error('rate limited')
        const sender = createSender({
            sendNode: async () => {
                throw cause
            },
        })

        await sender.sendFrame(new Uint8Array(encodeBinaryNode(receipt()))).catch((error) => {
            assert.equal((error as SendError).cause, cause)
        })
    })
})

describe('what sending declares', () => {
    it('is declared separately from observing', () => {
        // An adapter built to observe genuinely cannot send. One capability set
        // covering both would be false for whichever the consumer holds.
        assert.ok(!declares(INFO, Capability.L0Outbound), 'the tap does not send')
        assert.ok(declares(SENDING_INFO, Capability.L0Outbound))
    })

    it('adds to what the tap does rather than replacing it', () => {
        for (const capability of INFO.capabilities) {
            assert.ok(
                declares(SENDING_INFO, capability),
                `${capability} was lost when sending was added`
            )
        }
    })

    it('does not claim request/response along with it', () => {
        // Writing to the socket and being handed the answer are different
        // powers. `zapo` can correlate a reply, but that is its own claim.
        assert.ok(!declares(SENDING_INFO, Capability.L0Request))
    })
})
