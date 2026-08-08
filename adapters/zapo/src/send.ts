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
import { decodeBinaryNode, encodeBinaryNode } from 'zapo-js/transport'

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

/** What a requester needs: send, and be handed the reply. */
export interface NodeRequester extends NodeSender {
    readonly query: (node: BinaryNode, timeoutMs?: number) => Promise<BinaryNode>
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

/** Raised when a request produced no usable reply. */
export class RequestError extends Error {
    constructor(
        message: string,
        public override readonly cause?: unknown
    ) {
        super(message)
        this.name = 'RequestError'
    }
}

/**
 * Raised when the stanza left and nothing came back in time.
 *
 * Kept apart from a failed send because the two call for opposite responses: a
 * send that failed can be retried, while a request that timed out may well have
 * been acted on — retrying repeats whatever it did.
 */
export class RequestTimeoutError extends RequestError {
    constructor(cause?: unknown) {
        super('no reply before the deadline', cause)
        this.name = 'RequestTimeoutError'
    }
}

/** Sends a stanza and hands back the reply the server correlated to it. */
export interface StanzaRequester extends StanzaSender {
    /**
     * Send `frame` and wait for its reply, which crosses as a frame like
     * everything else — unparsed, because interpreting it is L1's job and a
     * consumer may want the bytes exactly as they arrived.
     */
    readonly requestFrame: (frame: Uint8Array, timeoutMs?: number) => Promise<Uint8Array>
}

/**
 * A requester over an engine's own `query`.
 *
 * A strictly stronger claim than {@link createSender}, and a separate capability
 * for that reason: correlating a reply means holding the engine's table of
 * outstanding requests, which an engine may not expose even when it will
 * happily write to the socket.
 */
export function createRequester(engine: NodeRequester): StanzaRequester {
    const sender = createSender(engine)
    return {
        sendFrame: sender.sendFrame,
        async requestFrame(frame: Uint8Array, timeoutMs?: number): Promise<Uint8Array> {
            let node: BinaryNode
            try {
                node = decodeBinaryNode(Buffer.from(frame))
            } catch (error) {
                throw new RequestError('frame is not a decodable stanza', error)
            }

            try {
                const reply = await engine.query(node, timeoutMs)
                return new Uint8Array(encodeBinaryNode(reply))
            } catch (error) {
                if (isTimeout(error)) {
                    throw new RequestTimeoutError(error)
                }
                throw isNotConnected(error)
                    ? new NotConnectedError(error)
                    : new RequestError('the request failed', error)
            }
        },
    }
}

/**
 * Whether the engine failed because nothing came back in time.
 *
 * Matched on the message, like {@link isNotConnected} and for the same reason:
 * `zapo` does not type this failure, and a miss degrades to a plain
 * {@link RequestError}, which still tells the caller the request failed.
 */
function isTimeout(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error)
    return /timed? ?out|deadline/i.test(message)
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
