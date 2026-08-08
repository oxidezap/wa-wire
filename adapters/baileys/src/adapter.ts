/**
 * The wa-wire adapter for Baileys.
 *
 * Installs as two config callbacks rather than as a plugin, because Baileys has
 * no plugin host: `onFrameDecoded` for every stanza the socket decodes, and
 * `onDecryptedPayload` for every `<enc>` Signal decrypts. The adapter joins
 * them and hands over one envelope per stanza.
 *
 * ```ts
 * const wire = waWire(stanza => record(encodeEnvelope(stanza)))
 * const sock = makeWASocket({ ...config, ...wire.config })
 * ```
 *
 * There is nothing to unregister: a caller that wants to stop stops passing the
 * callbacks, and `flush()` empties what is still held.
 */

import { encodeEnvelope, verify, type Stanza } from '@oxidezap/wa-wire-ts'

import { INFO } from './capability.js'
import { DEFAULT_LOOKAHEAD, PlaintextJoiner, type Node, type StanzaSink } from './joiner.js'

/** The two callbacks a socket is configured with. */
export interface WaWireConfig {
    onFrameDecoded: (frame: unknown, decoded?: Uint8Array) => void
    onDecryptedPayload: (payload: {
        stanza: Node
        childIndex: number
        encType: string
        plaintext: Uint8Array
        unpadded: boolean
    }) => void
}

/** An installed adapter. */
export interface WaWire {
    /** Spread into `makeWASocket`'s config. */
    readonly config: WaWireConfig
    /**
     * Emit everything still waiting on payloads.
     *
     * For shutdown: a frame waiting for a payload that will never arrive is
     * better emitted unobserved than lost, the stanza having been real.
     */
    flush(): void
    /** How many stanzas are waiting on payloads. */
    readonly pending: number
}

/** Options an installation may override. */
export interface WaWireOptions {
    /** How many later stanzas a pending one tolerates. */
    readonly lookahead?: number
}

/**
 * Build the callbacks that forward every stanza to `sink`.
 *
 * Each envelope is checked against {@link INFO} on the way out. The declaration
 * is what a consumer selects an engine on, so a capability that stops being
 * true should fail here rather than quietly mislead — and this is the only
 * place that can see it.
 */
export function waWire(sink: StanzaSink, options: WaWireOptions = {}): WaWire {
    const joiner = new PlaintextJoiner(options.lookahead ?? DEFAULT_LOOKAHEAD)
    const checked: StanzaSink = stanza => {
        const violation = verify(INFO, stanza)
        if (violation) {
            throw new Error(`wa-wire: ${violation}`)
        }

        sink(stanza)
    }

    return {
        config: {
            onFrameDecoded: (frame, decoded) => {
                // Before the transport is up a frame is not a node and there is
                // nothing to pair it with. The handshake exchange crosses as
                // nothing rather than as a stanza it is not.
                if (decoded === undefined || !isNode(frame)) {
                    return
                }

                joiner.acceptFrame(frame, decoded, checked)
            },
            onDecryptedPayload: payload => {
                const id = payload.stanza.attrs.id
                if (id === undefined) {
                    return
                }

                joiner.acceptPlaintext(
                    { messageId: id, childIndex: payload.childIndex, plaintext: payload.plaintext },
                    checked
                )
            }
        },
        flush: () => joiner.flush(checked),
        get pending() {
            return joiner.pending
        }
    }
}

/** Encode a stanza for a consumer on the other side of a boundary. */
export function toEnvelope(stanza: Stanza): Uint8Array {
    return encodeEnvelope(stanza)
}

const isNode = (frame: unknown): frame is Node =>
    typeof frame === 'object' &&
    frame !== null &&
    typeof (frame as Node).tag === 'string' &&
    typeof (frame as Node).attrs === 'object'
