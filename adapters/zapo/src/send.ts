/**
 * Sending a stanza through `zapo`.
 *
 * The outbound half of the boundary. Frames cross in the same shape they do
 * inbound — the bytes a decoder consumes — so a captured envelope can be sent
 * back as it stands, and record and replay stop being separate features.
 *
 * `zapo` sends decoded nodes rather than bytes, so this decodes the frame on the
 * way out. That is the adapter's job precisely so a consumer never has to know
 * which shape its engine wants: the Rust adapter puts the format byte back and
 * hands over bytes, this one decodes, and the consumer writes the same code
 * against either.
 *
 * # What this is not
 *
 * Not request/response. `zapo` can correlate a reply — `queryWithContext` — and
 * that is a separate capability (`l0.request`) an adapter claims on its own.
 * Writing to the socket and being handed the answer are different powers, and
 * an engine may offer one without the other.
 */

import type { BinaryNode } from 'zapo-js'
import { decodeBinaryNode } from 'zapo-js/transport'

/** Why a stanza could not be sent. */
export class SendError extends Error {
    constructor(
        message: string,
        /** The engine's own error, when there was one. */
        public override readonly cause?: unknown
    ) {
        super(message)
        this.name = 'SendError'
    }
}

/**
 * Raised when there is no live connection to send on.
 *
 * Separate from a plain {@link SendError} because it is the one a consumer can
 * act on without knowing anything about the engine: wait, reconnect, try again.
 */
export class NotConnectedError extends SendError {
    constructor(cause?: unknown) {
        super('not connected', cause)
        this.name = 'NotConnectedError'
    }
}

/** What this sender needs of the engine — the one call, nothing more. */
export interface NodeSender {
    readonly sendNode: (node: BinaryNode) => Promise<void>
}

/** Puts a stanza on the wire. */
export interface StanzaSender {
    /**
     * Send one stanza, as the frame bytes a decoder would consume.
     *
     * Resolving means the engine accepted the frame for delivery — not that the
     * server acted on it. Nothing at L0 can promise the latter: the answer to a
     * stanza is another stanza, and it arrives inbound.
     */
    readonly sendFrame: (frame: Uint8Array) => Promise<void>
}

/**
 * A sender over an engine's own `sendNode`.
 *
 * Takes the narrowest thing that works rather than the whole plugin context: a
 * seam this small is one a test can stand in for, and one a future `zapo`
 * reshuffle is unlikely to move.
 */
export function createSender(engine: NodeSender): StanzaSender {
    return {
        async sendFrame(frame: Uint8Array): Promise<void> {
            let node: BinaryNode
            try {
                node = decodeBinaryNode(Buffer.from(frame))
            } catch (error) {
                // Distinguished from an engine refusal on purpose: a frame this
                // adapter cannot read never reached the socket, and the fix is
                // in what the caller passed rather than in the connection.
                throw new SendError('frame is not a decodable stanza', error)
            }

            try {
                await engine.sendNode(node)
            } catch (error) {
                throw isNotConnected(error)
                    ? new NotConnectedError(error)
                    : new SendError('engine refused the send', error)
            }
        },
    }
}

/**
 * Whether the engine failed because nothing is connected.
 *
 * Matched on the message because `zapo` does not type this failure, and the
 * distinction is worth keeping: it is the only send failure a consumer can
 * retry its way out of. A miss degrades to a plain {@link SendError}, which is
 * the safe direction — the caller still learns the send failed.
 */
function isNotConnected(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error)
    return /not connected|no connection|socket (is )?closed/i.test(message)
}
