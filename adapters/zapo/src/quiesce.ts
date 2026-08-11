/**
 * RFC-003's phases 1, 2 and 6: stop accepting commands, wait for the engine to
 * go quiet, and let the backlog go.
 *
 * The Rust side of the boundary has the same three in
 * `wa_wire_adapter::handoff`, and for the same reasons; this is the TypeScript
 * half, kept deliberately in step. What differs is only what the engine can
 * tell you, and here that difference is the point: `zapo` is the one engine of
 * the four that declares `lifecycle.drain-hook`, so it is the one where the
 * barrier can be *confirmed* rather than waited out.
 *
 * A handoff is stop-the-world, because one device gets one connection. So
 * something has to hold what the application asks for in between, and that
 * something is the host's — no engine knows those commands exist.
 */

/** Whether anything actually said the engine had gone quiet. */
export const Quiet = {
    /** The engine reported that its handlers had drained. */
    Confirmed: 'drained',
    /**
     * Nothing reported anything. The host stopped waiting, and what was in
     * flight is unknown rather than absent.
     */
    Unconfirmed: 'not known to have drained',
} as const

export type Quiet = (typeof Quiet)[keyof typeof Quiet]

/**
 * The drain, waited on by the host and completed by the adapter.
 *
 * Two outcomes and not one. Collapsing them would let a host read "I stopped
 * waiting" as "there was nothing left" — and what is lost in that gap is an ack
 * the server will resend to whoever holds the session next.
 */
export class Barrier {
    #drained = false
    #waiting: Array<() => void> = []

    /** Report that the engine's handlers have finished. Idempotent. */
    drained(): void {
        if (this.#drained) return
        this.#drained = true
        const waiting = this.#waiting
        this.#waiting = []
        for (const wake of waiting) wake()
    }

    /** What is known right now, without waiting. */
    get state(): Quiet {
        return this.#drained ? Quiet.Confirmed : Quiet.Unconfirmed
    }

    /**
     * Wait up to `timeoutMs` for the drain, and report which happened.
     *
     * Resolves rather than rejects on a timeout: a barrier that could not be
     * confirmed is an outcome a host acts on, not an error it recovers from,
     * and the third answer is the whole reason this returns a {@link Quiet}.
     */
    async wait(timeoutMs: number): Promise<Quiet> {
        if (this.#drained) return Quiet.Confirmed
        return new Promise<Quiet>((resolve) => {
            const timer = setTimeout(() => resolve(Quiet.Unconfirmed), timeoutMs)
            this.#waiting.push(() => {
                clearTimeout(timer)
                resolve(Quiet.Confirmed)
            })
        })
    }
}

/** What happened to a command offered while the session was moving. */
export type Offered<T> =
    | { readonly kind: 'pass'; readonly command: T }
    | { readonly kind: 'held' }
    | { readonly kind: 'full'; readonly command: T }

/**
 * Phases 1 and 6: stop accepting commands, then let them go.
 *
 * Bounded, because an unbounded backlog behind a handoff that has stalled is a
 * leak that presents as a hang. A full gate hands the command back rather than
 * dropping it — dropping would make a full backlog look like a successful hold,
 * and the application would never learn its command went nowhere.
 *
 * Order is preserved. Releasing in a different order than commands were offered
 * reorders an application's sends, which is a bug it cannot see and did not
 * cause.
 */
export class Gate<T> {
    #backlog: T[] = []
    #quiesced = false

    constructor(private readonly capacity: number = 32) {
        if (!Number.isInteger(capacity) || capacity < 1) {
            throw new RangeError(`gate capacity must be a positive integer, got ${capacity}`)
        }
    }

    /** Stop accepting commands. Idempotent. */
    quiesce(): void {
        this.#quiesced = true
    }

    get isQuiesced(): boolean {
        return this.#quiesced
    }

    get backlog(): number {
        return this.#backlog.length
    }

    get isFull(): boolean {
        return this.#backlog.length >= this.capacity
    }

    /** Offer a command: send it now, hold it, or hand it back. */
    offer(command: T): Offered<T> {
        if (!this.#quiesced) return { kind: 'pass', command }
        if (this.isFull) return { kind: 'full', command }
        this.#backlog.push(command)
        return { kind: 'held' }
    }

    /**
     * Phase 6: open the gate and release the backlog, oldest first.
     *
     * The gate opens *before* the backlog drains, so a command the release
     * itself produces is passed rather than appended to a queue being emptied.
     */
    async resume(release: (command: T) => void | Promise<void>): Promise<number> {
        this.#quiesced = false
        const held = this.#backlog
        this.#backlog = []
        for (const command of held) {
            await release(command)
        }
        return held.length
    }

    /**
     * Drop the backlog without releasing it, and open the gate.
     *
     * For a handoff that failed and is being abandoned. Returns how many were
     * discarded, because a host that gives up should be able to say what it
     * cost.
     */
    abandon(): number {
        this.#quiesced = false
        const discarded = this.#backlog.length
        this.#backlog = []
        return discarded
    }
}
