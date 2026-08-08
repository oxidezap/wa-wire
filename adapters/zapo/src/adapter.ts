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

import { require as requireCapabilities, type Capability } from '@oxidezap/wa-wire-ts'
import { INFO, TAKEOVER_CAPABILITIES, TAP_CAPABILITIES } from './capability.js'
import { Direction, FrameOrigin, encodeEnvelope, type Stanza } from '@oxidezap/wa-wire-ts'
import { PlaintextJoiner } from './joiner.js'

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
     * contract. In `zapo` decryption happens *inside* the dispatch this mode
     * suppresses, so a stanza carrying ciphertext is passed on to the engine
     * and only everything else is dropped. That exception is the whole reason
     * this mode is still L0-plain rather than L0-wire.
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
     * Capabilities the consumer relies on. Installing throws
     * {@link UnmetCapabilitiesError} when this adapter lacks any of them.
     *
     * Cheap to state and worth stating even when it currently holds: the point
     * is that it keeps holding when the engine moves underneath.
     */
    readonly requires?: readonly Capability[]
    /**
     * Called when a stanza cannot be forwarded.
     *
     * Throwing from the filter would let one bad stanza take down delivery for
     * the rest, so failures are reported here and the stanza continues to the
     * engine. Silence would be worse: a consumer would see a gap and have no
     * way to know there was one.
     *
     * Carries the stanza rather than the node it came from. A stanza held for
     * its plaintexts reaches the sink long after the filter that saw the node
     * returned, and handing back a placeholder node would put wrong ids and a
     * wrong tag into the one report a consumer has about a gap.
     */
    readonly onError?: (error: unknown, stanza: Stanza) => void
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
            // Before anything is registered: a consumer that asked for what
            // this instance does not do should not get a half-working install.
            // Checked against the mode, not against the adapter: installed as a
            // tap it suppresses nothing, whatever it is capable of.
            requireCapabilities(
                options.requires ?? [],
                mode === Mode.Takeover ? TAKEOVER_CAPABILITIES : TAP_CAPABILITIES
            )
            // The joiner holds a `<message>` until its plaintexts arrive, so a
            // consumer sees one envelope per stanza rather than a frame and
            // then a stream of payloads to correlate itself.
            const joiner = new PlaintextJoiner()
            const sink = (stanza: Stanza): void => {
                try {
                    options.sink(stanza)
                } catch (error) {
                    options.onError?.(error, stanza)
                }
            }

            const unregister = context.registerIncomingStanzaFilter((node) => {
                const held = joiner.acceptNode(node, encodeBinaryNode(node), sink)
                // Under takeover the engine stops interpreting the stanza but
                // still acks it, so the server does not redeliver.
                //
                // Except when the joiner is holding it: the payloads it waits
                // for are produced by the very handler suppression would skip,
                // so dropping it here would leave every encrypted stanza to
                // time out and cross as `Unobserved`. Takeover suppresses
                // dispatch, never crypto.
                return mode === Mode.Takeover && !held
            })
            const onPayload = (event: {
                readonly stanzaId?: string
                readonly encIndex: number
                readonly plaintext: Uint8Array
            }): void => {
                if (event.stanzaId === undefined) {
                    return
                }
                joiner.acceptPlaintext(
                    {
                        messageId: event.stanzaId,
                        encIndex: event.encIndex,
                        plaintext: event.plaintext,
                    },
                    sink
                )
            }
            context.on('debug_decrypted_payload', onPayload)

            context.registerDispose(() => {
                unregister()
                context.off('debug_decrypted_payload', onPayload)
                // Whatever is still held is the last anyone will hear about
                // those stanzas.
                joiner.flush(sink)
            })
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
    const stanza = toStanza(node)
    try {
        options.sink(stanza)
    } catch (error) {
        options.onError?.(error, stanza)
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
