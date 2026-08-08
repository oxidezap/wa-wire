import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { BinaryNode } from 'zapo-js'
import { decodeBinaryNode } from 'zapo-js/transport'

import { Capability, UnmetCapabilitiesError } from '@oxidezap/wa-wire-ts'
import { has, missing } from '../capability.js'
import { INFO } from '../capability.js'
import { Direction, FrameOrigin, PlaintextStatus, type Stanza } from '@oxidezap/wa-wire-ts'
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

    it('carries no plaintext table on its own', () => {
        // `toStanza` is the frame-only shape, which is what a stanza with no
        // `<enc>` crosses as. Payloads arrive later and are joined by
        // `PlaintextJoiner`, which is where the table comes from.
        const stanza = toStanza(message())
        assert.equal(stanza.plaintexts, undefined)
        assert.ok(has(Capability.L0Plaintext), 'but the adapter does emit them')
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

    it('passes the offending stanza to the reporter', () => {
        const node = receipt()
        let reported: Stanza | undefined
        forward(node, {
            sink: () => {
                throw new Error('x')
            },
            onError: (_error, stanza) => {
                reported = stanza
            },
        })
        assert.deepEqual(reported, toStanza(node))
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
            on() {},
            off() {},
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

        for (const node of [receipt(), receipt(), receipt()]) {
            filter(node)
        }
        assert.deepEqual(seen, ['receipt', 'receipt', 'receipt'])
    })

    it('lets an encrypted stanza through under takeover, so it still gets decrypted', () => {
        // `zapo` decrypts inside the dispatch takeover suppresses. Dropping a
        // held message here would mean its payloads never arrive, and every
        // encrypted stanza would cross as `Unobserved` — L0-wire wearing an
        // L0-plain label.
        const filter = install({ sink: () => {}, mode: Mode.Takeover })

        assert.equal(filter(message()), false, 'the engine must still decrypt it')
        assert.equal(filter(receipt()), true, 'everything else is still suppressed')
    })

    it('still produces plaintext under takeover', () => {
        const stanzas: Array<{ readonly plaintexts?: readonly { status: number }[] }> = []
        const { filter, onPayload } = installFull({
            sink: (s) => stanzas.push(s),
            mode: Mode.Takeover,
        })

        filter(message())
        onPayload({
            stanzaId: message().attrs.id,
            encIndex: 0,
            plaintext: new Uint8Array([7, 7]),
        })

        assert.deepEqual(
            stanzas[0]?.plaintexts?.map((p) => p.status),
            [PlaintextStatus.Ok],
            'takeover suppresses dispatch, never crypto',
        )
    })

    it('refuses to install as a tap for a consumer that needs takeover', () => {
        // The instance is what a consumer gets, not the adapter's full range.
        // Installed as a tap it suppresses nothing, whatever it is capable of.
        assert.throws(
            () => install({ sink: () => {}, requires: [Capability.Takeover] }),
            UnmetCapabilitiesError,
        )
        assert.doesNotThrow(() =>
            install({ sink: () => {}, mode: Mode.Takeover, requires: [Capability.Takeover] }),
        )
    })

    it('joins a payload onto the message it belongs to', () => {
        const stanzas: Array<{ readonly plaintexts?: unknown }> = []
        const { filter, onPayload } = installFull({ sink: (s) => stanzas.push(s) })

        filter(message())
        onPayload({
            stanzaId: message().attrs.id,
            encIndex: 0,
            plaintext: new Uint8Array([7, 7]),
        })

        assert.equal(stanzas.length, 1, 'the join released it')
        assert.deepEqual(stanzas[0]?.plaintexts, [
            { path: [0], status: PlaintextStatus.Ok, payload: new Uint8Array([7, 7]) },
        ])
    })

    it('ignores a payload with no stanza to attach it to', () => {
        // Nothing to match on, so there is no stanza it could belong to.
        const stanzas: unknown[] = []
        const { filter, onPayload } = installFull({ sink: (s) => stanzas.push(s) })

        filter(message())
        onPayload({ encIndex: 0, plaintext: new Uint8Array([1]) })

        assert.equal(stanzas.length, 0, 'the message is still waiting')
    })

    it('emits what is still held when the plugin is disposed', () => {
        const stanzas: Array<{ readonly plaintexts?: readonly { status: number }[] }> = []
        const { filter, dispose } = installFull({ sink: (s) => stanzas.push(s) })

        filter(message())
        assert.equal(stanzas.length, 0)

        dispose()

        assert.equal(stanzas.length, 1, 'held stanzas are not lost on shutdown')
        assert.deepEqual(
            stanzas[0]?.plaintexts?.map((p) => p.status),
            [PlaintextStatus.Unobserved],
        )
    })

    it('reports a sink failure on the joined path too', () => {
        const errors: unknown[] = []
        const { filter, onPayload } = installFull({
            sink: () => {
                throw new Error('consumer blew up')
            },
            onError: (error) => errors.push(error),
        })

        filter(message())
        onPayload({ stanzaId: message().attrs.id, encIndex: 0, plaintext: new Uint8Array([1]) })

        assert.equal(errors.length, 1, 'the failure is reported, not thrown at the engine')
    })

    it('holds a message until its plaintexts arrive', () => {
        // The one shape that does not cross immediately: a `<message>` waits so
        // its payloads can be joined onto it, which is what makes the adapter
        // emit L0-plain rather than L0-wire. See `PlaintextJoiner`.
        const seen: string[] = []
        const filter = install({
            sink: (s) => seen.push(decodeBinaryNode(s.frame).tag),
        })

        filter(message())
        assert.deepEqual(seen, [], 'held, not dropped')

        filter(receipt())
        assert.deepEqual(seen, ['receipt'], 'and does not hold up what follows it')
    })
})

function install(options: Parameters<typeof waWire>[0]) {
    return installFull(options).filter
}

/** The whole seam: the filter, the payload listener, and the disposer. */
function installFull(options: Parameters<typeof waWire>[0]) {
    let filter: ((node: BinaryNode) => unknown) | undefined
    let onPayload: ((event: unknown) => void) | undefined
    let dispose: (() => void) | undefined
    const context = {
        registerIncomingStanzaFilter(fn: (node: BinaryNode) => unknown) {
            filter = fn
            return () => {}
        },
        registerDispose(fn: () => void) {
            dispose = fn
        },
        on(event: string, handler: (event: unknown) => void) {
            if (event === 'debug_decrypted_payload') {
                onPayload = handler
            }
        },
        off() {},
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    waWire(options).setup(context as any)
    assert.ok(filter, 'the plugin registered a filter')
    assert.ok(onPayload, 'and subscribed to decrypted payloads')
    assert.ok(dispose, 'and registered a disposer')
    return { filter, onPayload, dispose }
}

describe('what this adapter claims', () => {
    it('declares only what it does', () => {
        assert.equal(INFO.id, 'zapo')
        assert.equal(INFO.contractVersion, 1)

        assert.ok(has(Capability.L0InboundTap))
        assert.ok(has(Capability.Takeover), 'the filter can drop a stanza')
        assert.ok(has(Capability.DrainHook), 'registerDispose runs after drain')
        assert.ok(
            has(Capability.L0Plaintext),
            'debug_decrypted_payload reports each one after Signal',
        )
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
        assert.ok(!has(Capability.L0Outbound))
        assert.ok(!has(Capability.L0Request))
    })

    it('answers what a consumer would ask for', () => {
        assert.ok(supports([Capability.L0InboundTap, Capability.Takeover]))
        assert.ok(!supports([Capability.ZeroCopyFrame]))
        assert.ok(supports([]))

        assert.deepEqual(missing([Capability.L0InboundTap]), [])
        assert.deepEqual(missing([Capability.ZeroCopyFrame, Capability.L0Outbound]), [
            Capability.ZeroCopyFrame,
            Capability.L0Outbound,
        ])
    })

    it('never claims a status it cannot produce', () => {
        // Tap mode emits no plaintexts at all, so no status ever crosses.
        assert.equal(toStanza(message()).plaintexts, undefined)
        assert.equal(PlaintextStatus.Ok, 0)
    })
})

describe('the setup-time gate', () => {
    it('installs when the requirement is met', () => {
        const stanzas: unknown[] = []
        const { filter } = installFull({
            sink: (s) => stanzas.push(s),
            requires: [Capability.L0InboundTap, Capability.L0Plaintext],
        })

        filter(receipt())
        assert.equal(stanzas.length, 1, 'the adapter installed and forwards')
    })

    it('refuses to install when it cannot do what was asked', () => {
        // The whole point: a consumer that needs outbound traffic finds out
        // here, not by noticing that none ever arrived.
        assert.throws(
            () =>
                installFull({
                    sink: () => {},
                    requires: [Capability.L0Outbound],
                }),
            UnmetCapabilitiesError,
        )
    })

    it('names everything missing at once', () => {
        // A caller fixes its setup in one pass rather than one round trip per
        // capability.
        try {
            installFull({
                sink: () => {},
                requires: [
                    Capability.L0InboundTap,
                    Capability.L0Outbound,
                    Capability.L0Request,
                ],
            })
            assert.fail('should have refused')
        } catch (error) {
            assert.ok(error instanceof UnmetCapabilitiesError)
            assert.deepEqual(error.missing, [Capability.L0Outbound, Capability.L0Request])
            assert.match(error.message, /l0\.outbound/)
        }
    })

    it('requires nothing by default', () => {
        const stanzas: unknown[] = []
        const { filter } = installFull({ sink: (s) => stanzas.push(s) })

        filter(receipt())
        assert.equal(stanzas.length, 1)
    })
})
