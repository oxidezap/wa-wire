/**
 * Joining a stanza to the plaintexts decrypted out of it.
 *
 * The two arrive separately and cannot be made to arrive together. The stanza
 * filter runs when a node is decoded, which is necessarily before Signal has
 * run; a plaintext exists only afterwards. An adapter that wants to emit
 * L0-plain has to hold the stanza until its payloads catch up.
 *
 * # Knowing when to stop waiting
 *
 * The stanza itself says how many `<enc>` nodes it has, so the common case —
 * every one decrypts — closes by counting, with no clock involved: the last
 * payload completes the table and the envelope goes out immediately.
 *
 * What has no signal is an `<enc>` that will never produce a payload. The
 * engine skips one in several places, and nothing reports that per node. So
 * something has to give up, and giving up is measured in **stanzas, not
 * milliseconds**: the receive path is processed in order, so a stanza whose
 * payloads have not arrived after {@link DEFAULT_LOOKAHEAD} later ones is a
 * stanza whose payloads are not coming. A count is also the same on every
 * machine, which a duration is not — and this output is compared against other
 * engines'.
 *
 * The Rust adapter reaches the same conclusions from the same constraints
 * (`wa-wire` DESIGN D-052, D-053, D-055); the two are deliberately alike.
 *
 * # Fan-out stanzas are left as L0-wire
 *
 * A fan-out `<message>` carries a copy per device under
 * `<participants><to jid=…>`, and an engine numbers the ones addressed to *its*
 * device separately from the direct children. Reproducing that numbering needs
 * the device's own JID, which this adapter does not have — so for those the
 * index cannot be resolved to a node with certainty. They are emitted at once
 * with no plaintext table: a stanza without payloads is a smaller claim than a
 * payload on the wrong `<enc>`, which would read as a message from the wrong
 * device.
 */

import type { BinaryNode } from 'zapo-js'

import { Direction, FrameOrigin, PlaintextStatus, type Plaintext, type Stanza } from '@oxidezap/wa-wire-ts'

/**
 * How many later stanzas a pending message tolerates before it is emitted with
 * whatever it has.
 *
 * Sized for the widest real fan-out rather than tuned: a single message's
 * payloads all arrive within its own processing, so anything past a handful of
 * intervening stanzas already means they are not coming.
 */
export const DEFAULT_LOOKAHEAD = 64

/** One decrypted payload, as the engine reports it. */
export interface DecryptedEnc {
    /** The stanza id this belongs to. */
    readonly messageId: string
    /** Which `<enc>` of the stanza produced it, counting from zero. */
    readonly encIndex: number
    /** The plaintext. */
    readonly plaintext: Uint8Array
}

/** Where the joiner delivers finished stanzas. */
export type StanzaSink = (stanza: Stanza) => void

interface Pending {
    readonly messageId: string
    readonly frame: Uint8Array
    /** One slot per `<enc>`, in stanza order; `undefined` until its payload lands. */
    readonly slots: Array<Uint8Array | undefined>
    /** Index of each `<enc>` among the root's children. */
    readonly childIndices: number[]
    /** How many stanzas have gone by since this one arrived. */
    age: number
}

/**
 * Holds stanzas until their plaintexts arrive, then emits one envelope each.
 *
 * Not internally synchronised, and does not need to be: the engine drives the
 * filter and the event from one receive path.
 */
export class PlaintextJoiner {
    private readonly pending: Pending[] = []
    private abandonedCount = 0

    constructor(private readonly lookahead: number = DEFAULT_LOOKAHEAD) {}

    /** How many stanzas were emitted without all of their plaintexts. */
    public get abandoned(): number {
        return this.abandonedCount
    }

    /** How many stanzas are waiting. */
    public get waiting(): number {
        return this.pending.length
    }

    /**
     * Take a decoded stanza.
     *
     * One with `<enc>` children is held for its plaintexts; anything else goes
     * straight to the sink. Either way this first ages the stanzas already
     * waiting, emitting any that have waited long enough.
     *
     * Returns whether the stanza is being held. A caller that also decides
     * whether the engine sees the stanza needs to know: a held one is waiting
     * on payloads only the engine can produce.
     */
    public acceptNode(node: BinaryNode, frame: Uint8Array, sink: StanzaSink): boolean {
        this.age(sink)

        const pending = begin(node, frame)
        if (pending === null) {
            sink({ direction: Direction.Inbound, frameOrigin: FrameOrigin.ReEncoded, frame })
            return false
        }
        this.pending.push(pending)
        return true
    }

    /**
     * Take a plaintext the engine decrypted.
     *
     * Completing a stanza's last slot emits it immediately. A payload for a
     * stanza that is not waiting — one already given up on, or one whose node
     * was never seen — is dropped: there is no frame to attach it to, and
     * inventing one would be worse than losing it.
     */
    public acceptPlaintext(decrypted: DecryptedEnc, sink: StanzaSink): void {
        const index = this.pending.findIndex((p) => p.messageId === decrypted.messageId)
        if (index < 0) {
            return
        }
        const pending = this.pending[index]!
        if (decrypted.encIndex < 0 || decrypted.encIndex >= pending.slots.length) {
            // The engine reported an `<enc>` the stanza does not have, so the
            // two disagree about it. Keeping the stanza is the conservative
            // half of that: it still emits, just without this payload.
            return
        }
        pending.slots[decrypted.encIndex] = decrypted.plaintext

        if (pending.slots.every((slot) => slot !== undefined)) {
            this.pending.splice(index, 1)
            emit(pending, sink)
        }
    }

    /**
     * Emit every stanza still waiting, complete or not.
     *
     * For a caller shutting the adapter down: whatever is buffered is the last
     * anyone will hear about those stanzas.
     */
    public flush(sink: StanzaSink): void {
        const held = this.pending.splice(0, this.pending.length)
        for (const pending of held) {
            if (pending.slots.some((slot) => slot === undefined)) {
                this.abandonedCount += 1
            }
            emit(pending, sink)
        }
    }

    /** Age every waiting stanza by one, emitting those that ran out. */
    private age(sink: StanzaSink): void {
        for (let i = this.pending.length - 1; i >= 0; i -= 1) {
            const pending = this.pending[i]!
            pending.age += 1
            if (pending.age > this.lookahead) {
                this.pending.splice(i, 1)
                this.abandonedCount += 1
                emit(pending, sink)
            }
        }
    }
}

/**
 * Start holding `node`, or `null` if it has nothing to wait for.
 *
 * Only a stanza with both an `id` and at least one `<enc>` waits: without an id
 * no payload could be matched back to it, and without an `<enc>` there is
 * nothing to wait for. A fan-out stanza does not wait either — see the module
 * documentation.
 */
function begin(node: BinaryNode, frame: Uint8Array): Pending | null {
    const messageId = node.attrs?.id
    if (messageId === undefined || !Array.isArray(node.content)) {
        return null
    }
    if (node.content.some((child) => child.tag === 'participants')) {
        return null
    }

    const childIndices: number[] = []
    node.content.forEach((child, index) => {
        if (child.tag === 'enc') {
            childIndices.push(index)
        }
    })
    if (childIndices.length === 0) {
        return null
    }

    return {
        messageId,
        frame,
        slots: new Array<Uint8Array | undefined>(childIndices.length).fill(undefined),
        childIndices,
        age: 0,
    }
}

/** Hand one pending stanza to the sink, with the table it accumulated. */
function emit(pending: Pending, sink: StanzaSink): void {
    const plaintexts: Plaintext[] = pending.childIndices.map((childIndex, slot) => {
        const payload = pending.slots[slot]
        return payload === undefined
            ? // Not `DecryptFailed`: this adapter watches payloads appear and is
              // never told why one did not, so it reports the absence and no
              // cause.
              { path: [childIndex], status: PlaintextStatus.Unobserved, payload: new Uint8Array() }
            : { path: [childIndex], status: PlaintextStatus.Ok, payload }
    })

    sink({
        direction: Direction.Inbound,
        frameOrigin: FrameOrigin.ReEncoded,
        frame: pending.frame,
        plaintexts,
    })
}
