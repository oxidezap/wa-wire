/**
 * Releasing a `zapo` session so another engine can take it.
 *
 * `WaClient.disconnect()` is the call, and which call it is carries the whole
 * meaning. Its own documentation says it closes the transport gracefully and
 * *does not* clear stored credentials — `connect()` again resumes the same
 * session. It emits `isLogout: false`, and it stops the transport rather than
 * merely closing the socket: it reaches `WaComms.stopComms()`, which sets
 * `preventRetry`, so once it returns nothing brings the socket back except a
 * caller asking. That is a detach.
 *
 * Worth stating precisely, because `zapo` *does* reconnect on its own — a socket
 * that fails unexpectedly is retried with backoff, which a run against a server
 * it cannot handshake with shows five times over. What it does not do is come
 * back from a disconnect the caller asked for. The difference between the two
 * paths is `preventRetry`, and it is the property a handoff depends on.
 *
 * `WaClient.logout()` is the one that must stay out of reach here. It runs the
 * server-side logout and unpairs the device; nothing brings that back without
 * the account holder scanning a code again. A host driving a handoff holds a
 * {@link SessionDetacher} and has no method to call for it.
 *
 * This adapter carried `lifecycle.detach` as **no** on the grounds that `zapo`
 * could not drop its transport. That was read off a `whatsapp-bench` failure —
 * *"zapo does not support dropping its transport"* — which reports on the
 * benchmark client, not on the engine: the client never registered a drop hook.
 * Given one, five reconnect cycles complete against the mock server with no
 * re-pairing.
 */

/** What a detacher needs of the engine — the one call, nothing more. */
export interface SessionClient {
    readonly disconnect: () => Promise<void>
}

/** Why a session could not be released. */
export class DetachError extends Error {
    constructor(
        message: string,
        /** The engine's own error, when there was one. */
        public override readonly cause?: unknown
    ) {
        super(message)
        this.name = 'DetachError'
    }
}

/** Gives up the session without ending the account's pairing. */
export interface SessionDetacher {
    /**
     * Release the session, leaving the device paired.
     *
     * Resolving means the engine holds no socket and will not open one of its
     * own accord, so a second engine can take the session without the server
     * killing one of them — WhatsApp allows one connection per device.
     *
     * Rejecting means the session is where it was. A host must not read a
     * failure as permission to attach elsewhere, because the old connection may
     * still be live.
     *
     * Idempotent, because a host that crashed mid-handoff has to be able to
     * start over and the postcondition it needs already holds.
     */
    readonly detach: () => Promise<void>
}

/**
 * A detacher over an engine's own `disconnect`.
 *
 * Takes the one method it uses rather than a whole `WaClient`, so what this
 * needs of the engine is visible in the type and a test can supply it — the same
 * reason {@link NodeSender} is one call wide.
 */
export function createDetacher(engine: SessionClient): SessionDetacher {
    return {
        async detach(): Promise<void> {
            try {
                await engine.disconnect()
            } catch (error) {
                throw new DetachError('the session was not released', error)
            }
        },
    }
}
