import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { BinaryNode } from 'zapo-js'
import { decodeBinaryNode } from 'zapo-js/transport'

import { Capability, INFO, has, missing } from '../capability.js'
import { Direction, FrameOrigin, PlaintextStatus } from '../envelope.js'
import { Mode, forward, supports, toEnvelope, toStanza, waWire } from '../adapter.js'

function receipt(): BinaryNode {
    return {
        tag: 'receipt',
        attrs: { id: 'ABCD1234', from: '5511999998888@s.whatsapp.net', type: 'read' },
    }
}

function message(): BinaryNode {
    return {
        tag: 'message',
        attrs: { id: 'MSG1', from: '5511999998888@s.whatsapp.net', t: '1700000000' },
        content: [
            {
                tag: 'enc',
                attrs: { v: '2', type: 'msg' },
                content: new TextEncoder().encode('ciphertext'),
            },
        ],
    }
}

describe('turning a node into a stanza', () => {
    it('marks the frame re-encoded, because it is', () => {
        // The filter hands over a decoded node, not the buffer it came from.
        // Claiming the bytes were verbatim would make a consumer trust a frame
        // it should not.
        const stanza = toStanza(receipt())
        assert.equal(stanza.frameOrigin, FrameOrigin.ReEncoded)
        assert.equal(stanza.direction, Direction.Inbound)
        assert.ok(stanza.frame.length > 0)
    })

    it('produces a frame that decodes back to the same node', () => {
        // The re-encoding has to be faithful even though it is not byte-exact:
        // this is what a host will parse.
        for (const node of [receipt(), message()]) {
            const decoded = decodeBinaryNode(toStanza(node).frame)
            assert.equal(decoded.tag, node.tag)
            assert.deepEqual(decoded.attrs, node.attrs)
        }
    })

    it('carries no plaintext table', () => {
        // The filter runs before decryption, so a <message> crosses with its
        // ciphertext and nothing else. Saying so is the honest option.
        const stanza = toStanza(message())
        assert.equal(stanza.plaintexts, undefined)
        assert.ok(!has(Capability.L0Plaintext))
    })

    it('encodes straight to envelope bytes', () => {
        const bytes = toEnvelope(receipt())
        assert.equal(bytes[0], 1, 'contract version')
        // Inbound and re-encoded.
        assert.equal(bytes[2], 0b10)
    })
})

describe('forwarding', () => {
    it('hands every stanza to the sink', () => {
        const seen: string[] = []
        forward(receipt(), {
            sink: (stanza) => {
                seen.push(decodeBinaryNode(stanza.frame).tag)
            },
        })
        forward(message(), {
            sink: (stanza) => {
                seen.push(decodeBinaryNode(stanza.frame).tag)
            },
        })
        assert.deepEqual(seen, ['receipt', 'message'])
    })

    it('reports a sink failure instead of letting it escape', () => {
        // Throwing from the filter would take delivery down for every stanza
        // after this one.
        const failures: unknown[] = []
        assert.doesNotThrow(() =>
            forward(receipt(), {
                sink: () => {
                    throw new Error('consumer blew up')
                },
                onError: (error) => failures.push(error),
            }),
        )
        assert.equal(failures.length, 1)
        assert.match(String(failures[0]), /consumer blew up/)
    })

    it('swallows a failure when no reporter was given', () => {
        assert.doesNotThrow(() =>
            forward(receipt(), {
                sink: () => {
                    throw new Error('nobody is listening')
                },
            }),
        )
    })

    it('passes the offending node to the reporter', () => {
        let reported: BinaryNode | undefined
        forward(receipt(), {
            sink: () => {
                throw new Error('x')
            },
            onError: (_error, node) => {
                reported = node
            },
        })
        assert.equal(reported?.tag, 'receipt')
    })
})

describe('the plugin', () => {
    it('registers a filter and unregisters it on dispose', () => {
        const registered: Array<(node: BinaryNode) => unknown> = []
        let unregistered = false
        let disposer: (() => void) | undefined

        const context = {
            registerIncomingStanzaFilter(filter: (node: BinaryNode) => unknown) {
                registered.push(filter)
                return () => {
                    unregistered = true
                }
            },
            registerDispose(fn: () => void) {
                disposer = fn
            },
        }

        const plugin = waWire({ sink: () => {} })
        assert.equal(plugin.id, 'wa-wire')
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        plugin.setup(context as any)

        assert.equal(registered.length, 1)
        assert.ok(disposer, 'a disposer was registered')
        disposer?.()
        assert.ok(unregistered, 'the filter is removed when the client closes')
    })

    it('leaves the stanza to the engine in tap mode', () => {
        const seen: BinaryNode[] = []
        const filter = install({ sink: (s) => seen.push(decodeBinaryNode(s.frame)) })

        const verdict = filter(receipt())

        assert.equal(verdict, false, 'the engine keeps processing')
        assert.equal(seen.length, 1)
    })

    it('suppresses the engine in takeover mode', () => {
        // `zapo` still acks a dropped stanza, so the server does not redeliver
        // — the engine simply stops interpreting it.
        const seen: BinaryNode[] = []
        const filter = install({
            sink: (s) => seen.push(decodeBinaryNode(s.frame)),
            mode: Mode.Takeover,
        })

        const verdict = filter(receipt())

        assert.equal(verdict, true, 'the engine drops it')
        assert.equal(seen.length, 1, 'but it still reached the sink')
    })

    it('forwards before deciding, so takeover never loses a stanza', () => {
        const seen: string[] = []
        const filter = install({
            sink: (s) => seen.push(decodeBinaryNode(s.frame).tag),
            mode: Mode.Takeover,
        })

        for (const node of [receipt(), message(), receipt()]) {
            filter(node)
        }
        assert.deepEqual(seen, ['receipt', 'message', 'receipt'])
    })
})

function install(options: Parameters<typeof waWire>[0]) {
    let filter: ((node: BinaryNode) => unknown) | undefined
    const context = {
        registerIncomingStanzaFilter(fn: (node: BinaryNode) => unknown) {
            filter = fn
            return () => {}
        },
        registerDispose() {},
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    waWire(options).setup(context as any)
    assert.ok(filter, 'the plugin registered a filter')
    return filter
}

describe('what this adapter claims', () => {
    it('declares only what it does', () => {
        assert.equal(INFO.id, 'zapo')
        assert.equal(INFO.contractVersion, 1)

        assert.ok(has(Capability.L0InboundTap))
        assert.ok(has(Capability.Takeover), 'the filter can drop a stanza')
        assert.ok(has(Capability.DrainHook), 'registerDispose runs after drain')
    })

    it('does not claim what zapo does not offer here', () => {
        assert.ok(
            !has(Capability.L0InboundAuthPhase),
            'success and failure bypass stanza filters',
        )
        assert.ok(
            !has(Capability.ZeroCopyFrame),
            'the filter sees a decoded node, not the buffer',
        )
        assert.ok(
            !has(Capability.L0Plaintext),
            'the filter runs before decryption',
        )
        assert.ok(!has(Capability.L0Outbound))
        assert.ok(!has(Capability.L0Request))
    })

    it('answers what a consumer would ask for', () => {
        assert.ok(supports([Capability.L0InboundTap, Capability.Takeover]))
        assert.ok(!supports([Capability.ZeroCopyFrame]))
        assert.ok(supports([]))

        assert.deepEqual(missing([Capability.L0InboundTap]), [])
        assert.deepEqual(missing([Capability.ZeroCopyFrame, Capability.L0Plaintext]), [
            Capability.ZeroCopyFrame,
            Capability.L0Plaintext,
        ])
    })

    it('never claims a status it cannot produce', () => {
        // Tap mode emits no plaintexts at all, so no status ever crosses.
        assert.equal(toStanza(message()).plaintexts, undefined)
        assert.equal(PlaintextStatus.Ok, 0)
    })
})
