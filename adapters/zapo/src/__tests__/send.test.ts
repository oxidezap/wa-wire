import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { BinaryNode } from 'zapo-js'
import { encodeBinaryNode } from 'zapo-js/transport'

import { Capability, INFO, REQUESTING_INFO, SENDING_INFO, declares } from '../capability.js'
import {
    NotConnectedError,
    RequestError,
    RequestTimeoutError,
    SendError,
    createRequester,
    createSender,
} from '../send.js'

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

    it('sends without any Node-only global', async () => {
        // The adapter is meant to run wherever `zapo` runs, which includes a
        // browser, a worker and Deno. Reaching for `Buffer` on the way out
        // would make every send fail there, and only there.
        const frame = new Uint8Array(encodeBinaryNode(receipt()))
        const sent: BinaryNode[] = []
        const sender = createSender({
            sendNode: async (node) => {
                sent.push(node)
            },
        })

        const held = globalThis.Buffer
        // @ts-expect-error removing a global is the whole point of the test
        delete globalThis.Buffer
        try {
            await sender.sendFrame(frame)
        } finally {
            globalThis.Buffer = held
        }

        assert.equal(sent.length, 1)
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

describe('requesting a reply', () => {
    it('hands back the reply the engine correlated', async () => {
        const reply: BinaryNode = { tag: 'iq', attrs: { id: 'Q1', type: 'result' } }
        const requester = createRequester({
            sendNode: async () => {},
            query: async () => reply,
        })

        const frame = await requester.requestFrame(new Uint8Array(encodeBinaryNode(receipt())))

        assert.deepEqual(frame, new Uint8Array(encodeBinaryNode(reply)))
    })

    it('separates no reply from a failed send', async () => {
        // A send that failed can be retried; a request that timed out may well
        // have been acted on, so retrying repeats whatever it did.
        const requester = createRequester({
            sendNode: async () => {},
            query: async () => {
                throw new Error('request timed out')
            },
        })

        await assert.rejects(
            () => requester.requestFrame(new Uint8Array(encodeBinaryNode(receipt()))),
            RequestTimeoutError
        )
    })

    it('still reports a disconnected engine as its own failure', async () => {
        const requester = createRequester({
            sendNode: async () => {},
            query: async () => {
                throw new Error('socket is closed')
            },
        })

        await assert.rejects(
            () => requester.requestFrame(new Uint8Array(encodeBinaryNode(receipt()))),
            NotConnectedError
        )
    })

    it('reports a frame it cannot read without touching the socket', async () => {
        let asked = false
        const requester = createRequester({
            sendNode: async () => {},
            query: async () => {
                asked = true
                return receipt()
            },
        })

        await assert.rejects(
            () => requester.requestFrame(new Uint8Array([0xff, 0xff])),
            RequestError
        )
        assert.equal(asked, false, 'the engine was never asked')
    })

    it('requests without any Node-only global either', async () => {
        const requester = createRequester({
            sendNode: async () => {},
            query: async (node) => node,
        })

        const held = globalThis.Buffer
        // @ts-expect-error removing a global is the whole point of the test
        delete globalThis.Buffer
        try {
            const reply = await requester.requestFrame(
                new Uint8Array(encodeBinaryNode(receipt()))
            )
            assert.ok(reply.length > 0)
        } finally {
            globalThis.Buffer = held
        }
    })

    it('can still send without requesting', async () => {
        // A requester is a sender too, so a consumer holding one does not need
        // a second object to fire and forget.
        const sent: BinaryNode[] = []
        const requester = createRequester({
            sendNode: async (node) => {
                sent.push(node)
            },
            query: async () => receipt(),
        })

        await requester.sendFrame(new Uint8Array(encodeBinaryNode(receipt())))

        assert.equal(sent.length, 1)
    })

    it('is a stronger claim than sending', () => {
        assert.ok(!declares(SENDING_INFO, Capability.L0Request))
        assert.ok(declares(REQUESTING_INFO, Capability.L0Request))
        assert.ok(
            declares(REQUESTING_INFO, Capability.L0Outbound),
            'and requesting implies sending'
        )
    })
})
