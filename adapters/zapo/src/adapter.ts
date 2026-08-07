/**
 * The `zapo` adapter.
 *
 * Installs as a plugin and registers an incoming stanza filter, which `zapo`
 * runs against every inbound stanza before any handler. The filter is also what
 * makes takeover possible: returning `true` drops the stanza from the engine's
 * own pipeline while `zapo` still emits the ack, so the server stops
 * redelivering.
 *
 * That is the whole adapter. Everything a stanza could be interpreted as
 * happens host-side, so there is nothing here to diverge from another engine.
 */

import { defineWaClientPlugin } from 'zapo-js'
import type { BinaryNode, WaClientPluginContext } from 'zapo-js'
import { encodeBinaryNode } from 'zapo-js/transport'

import { INFO } from './capability.js'
import {
    Direction,
    FrameOrigin,
    encodeEnvelope,
    type Stanza,
} from './envelope.js'

/** Where the adapter delivers stanzas. */
export type StanzaSink = (stanza: Stanza) => void

/** How the adapter treats the stanzas it sees. */
export enum Mode {
    /**
     * Observe and let the engine carry on.
     *
     * The engine still parses, dispatches and updates its own state.
     */
    Tap = 'tap',
    /**
     * Observe and suppress the engine's own dispatch.
     *
     * `zapo` still runs Noise and still acks, so the server does not
     * redeliver — it simply stops interpreting. Under takeover an engine's own
     * semantics stop mattering, which is what makes engines interchangeable.
     *
     * Never suppresses decryption: L0-plain depends on the engine having
     * decrypted, so a takeover that disabled crypto would silently degrade the
     * contract.
     */
    Takeover = 'takeover',
}

/** How to install the adapter. */
export interface Options {
    /** Where stanzas go. */
    readonly sink: StanzaSink
    /** Defaults to {@link Mode.Tap}. */
    readonly mode?: Mode
    /**
     * Called when a stanza cannot be forwarded.
     *
     * Throwing from the filter would let one bad stanza take down delivery for
     * the rest, so failures are reported here and the stanza continues to the
     * engine. Silence would be worse: a consumer would see a gap and have no
     * way to know there was one.
     */
    readonly onError?: (error: unknown, node: BinaryNode) => void
}

/** This adapter's declaration. */
export { INFO } from './capability.js'

/**
 * Build the plugin.
 *
 * ```ts
 * const client = new WaClient({ plugins: [waWire({ sink })] })
 * ```
 */
export function waWire(options: Options) {
    const mode = options.mode ?? Mode.Tap

    return defineWaClientPlugin({
        id: 'wa-wire',
        setup(context: WaClientPluginContext) {
            const unregister = context.registerIncomingStanzaFilter((node) => {
                forward(node, options)
                // Under takeover the engine stops interpreting the stanza but
                // still acks it, so the server does not redeliver.
                return mode === Mode.Takeover
            })
            context.registerDispose(unregister)
        },
    })
}

/**
 * Encode one stanza and hand it to the sink.
 *
 * Exported for tests: the filter itself is a closure inside `setup`, and
 * asserting on what crosses matters more than asserting that a closure was
 * registered.
 */
export function forward(node: BinaryNode, options: Options): void {
    try {
        options.sink(toStanza(node))
    } catch (error) {
        options.onError?.(error, node)
    }
}

/**
 * Turn a decoded node back into a stanza for the boundary.
 *
 * `zapo`'s filter receives the node, not the buffer it was decoded from, so the
 * frame is re-encoded and marked as such. A consumer that needs the original
 * bytes reads `frameOrigin` and knows not to rely on them being byte-identical
 * to what arrived.
 */
export function toStanza(node: BinaryNode): Stanza {
    return {
        direction: Direction.Inbound,
        frameOrigin: FrameOrigin.ReEncoded,
        frame: encodeBinaryNode(node),
    }
}

/** Encode a stanza to the bytes a host reads. */
export function toEnvelope(node: BinaryNode): Uint8Array {
    return encodeEnvelope(toStanza(node))
}

/** Whether this adapter can serve everything in `required`. */
export function supports(required: readonly string[]): boolean {
    return required.every((capability) =>
        (INFO.capabilities as readonly string[]).includes(capability),
    )
}
