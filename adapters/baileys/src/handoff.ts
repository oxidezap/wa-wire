/**
 * Releasing a Baileys session so another engine can take it.
 *
 * `sock.end(undefined)` is the call. It closes the socket, runs the registered
 * end handlers and emits `connection.update` with `connection: 'close'` — and
 * touches no credentials, because Baileys does not own them: the caller holds
 * the auth state and decides what to persist. Baileys never reconnects on its
 * own either; a relaunch is the application calling `makeWASocket` again with
 * the same state. Both halves of a detach, and neither is an approximation.
 *
 * `sock.logout()` is the one that must stay out of reach. It sends
 * `remove-companion-device` to the server and then ends with
 * `DisconnectReason.loggedOut`, which is the status code every Baileys consumer
 * branches on to decide whether to wipe the auth state and print a new QR. A
 * host driving a handoff holds a {@link SessionDetacher} and has no method to
 * call for it.
 *
 * # Why `end(undefined)` and not `end(someError)`
 *
 * The argument becomes `lastDisconnect.error`, and the consumer's relaunch
 * decision reads its `statusCode`. Passing an error would make a deliberate
 * handoff look like a failure to every piece of code downstream that has been
 * written to tell those apart.
 */

/** What a detacher needs of the socket — the one call, nothing more. */
export interface SocketEnd {
    readonly end: (error: Error | undefined) => void | Promise<void>
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
     * Resolving means the socket is closed and Baileys will not open another —
     * it has no auto-reconnect, so the next connection is one the application
     * asks for. A second engine can take the session without the server killing
     * one of them.
     *
     * Rejecting means the session is where it was. A host must not read a
     * failure as permission to attach elsewhere, because the old connection may
     * still be live.
     *
     * Idempotent: Baileys' own `end` returns early once closed, and a host that
     * crashed mid-handoff has to be able to start over.
     */
    readonly detach: () => Promise<void>
}

/**
 * A detacher over a socket's own `end`.
 *
 * Takes the one method it uses rather than a whole `WASocket`, so what this
 * needs of the engine is visible in the type and a test can supply it.
 */
export function createDetacher(socket: SocketEnd): SessionDetacher {
    return {
        async detach(): Promise<void> {
            try {
                // Deliberately no error: it would arrive at the consumer as
                // `lastDisconnect.error` and read as a failure rather than as
                // the handoff it is.
                await socket.end(undefined)
            } catch (error) {
                throw new DetachError('the session was not released', error)
            }
        },
    }
}
