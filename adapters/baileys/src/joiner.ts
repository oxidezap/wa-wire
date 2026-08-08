/**
 * Joining a stanza to the plaintexts decrypted out of it.
 *
 * The two arrive separately and cannot be made to arrive together. The frame
 * hook runs where a stanza is decoded, which is necessarily before Signal has
 * run; a plaintext exists only afterwards.
 *
 * # Knowing when to stop waiting
 *
 * The stanza says how many `<enc>` children it has, so the common case — every
 * one decrypts — closes by counting, with no clock: the last payload completes
 * the table and the envelope goes at once.
 *
 * What has no signal is an `<enc>` that will never produce a payload. So
 * something has to give up, and giving up is measured in **stanzas rather than
 * milliseconds**: the receive path is ordered, and a count is the same on every
 * machine, which a duration is not — and this output is compared against other
 * engines'.
 *
 * # Stanzas leave in the order they arrived
 *
 * A stanza waiting on payloads is not a licence to reorder the ones behind it.
 * Emitting an ack the moment it arrives would put it ahead of a message that
 * came first, and a recording compared position by position would report that
 * as a divergence in whichever engine happened to be slower.
 *
 * The `zapo` and `whatsapp-rust` adapters emit unheld stanzas immediately and
 * do reorder. They were written before this was understood; the Go adapter and
 * this one queue. Worth reconciling — three adapters ordering differently is a
 * difference the engines are not responsible for.
 *
 * # What this adapter does not have to do
 *
 * Work out which node a payload belongs to. Baileys reports the child index
 * directly, so the path is that number and nothing is inferred.
 */

import { Direction, FrameOrigin, PlaintextStatus, type Plaintext, type Stanza } from '@oxidezap/wa-wire-ts'

/** A decoded stanza, as much of one as this adapter reads. */
export interface Node {
    readonly tag: string
    readonly attrs: Record<string, string | undefined>
    readonly content?: unknown
}

/**
 * How many later stanzas a pending message tolerates.
 *
 * Sized for the widest real fan-out rather than tuned: a message's payloads all
 * arrive within its own processing, so a handful of intervening stanzas already
 * means they are not coming.
 */
export const DEFAULT_LOOKAHEAD = 64

/** One decrypted payload, as the engine reports it. */
export interface DecryptedEnc {
    /** The stanza id this belongs to. */
    readonly messageId: string
    /** The position of the `<enc>` among the stanza's children. */
    readonly childIndex: number
    /** The plaintext. */
    readonly plaintext: Uint8Array
}

/** Receives envelopes as they complete, in the order their stanzas arrived. */
export type StanzaSink = (stanza: Stanza) => void

interface Slot {
    /** Empty for a stanza that is not held. */
    id: string
    frame: Uint8Array
    /** The child indices of this stanza's `<enc>` nodes, in document order. */
    encIndices: number[]
    plaintexts: Map<number, Plaintext>
    age: number
    done: boolean
}

const ready = (slot: Slot) => slot.done || slot.plaintexts.size >= slot.encIndices.length

export class PlaintextJoiner {
    /** Every stanza seen, in the order the receive path saw them. */
    private queue: Slot[] = []
    private byId = new Map<string, Slot>()

    constructor(private readonly lookahead: number = DEFAULT_LOOKAHEAD) {}

    /**
     * Take a decoded stanza and the bytes it came from.
     *
     * The frame is copied: the engine's buffer is a view into what it read, and
     * queueing means outliving the call.
     */
    public acceptFrame(node: Node, frame: Uint8Array, sink: StanzaSink): void {
        const encIndices = encChildIndices(node)
        const id = node.attrs.id ?? ''
        // A stanza carrying `<enc>` children and no id is not one this can
        // hold: it would never be matched to its payloads. Finished on arrival.
        const holdable = encIndices.length > 0 && id !== ''

        // Every stanza ages the ones waiting, including the ones finished on
        // arrival: the lookahead counts *later stanzas*, and an ack going past
        // is one. A receive path carrying nothing but acks would otherwise hold
        // a message for ever.
        this.age()

        const slot: Slot = {
            id: holdable ? id : '',
            frame: Uint8Array.prototype.slice.call(frame),
            encIndices,
            plaintexts: new Map(),
            age: 0,
            done: !holdable
        }
        if (holdable) {
            // A repeated id — a retry arriving before the first finished —
            // leaves the earlier stanza in the queue rather than replacing it.
            // It was a real stanza and this adapter reports every one; what it
            // gives up is the chance of a payload.
            const previous = this.byId.get(id)
            if (previous) {
                previous.done = true
            }

            this.byId.set(id, slot)
        }

        this.queue.push(slot)
        this.drain(sink)
    }

    /** Take one decrypted payload, completing its stanza if it was the last. */
    public acceptPlaintext(decrypted: DecryptedEnc, sink: StanzaSink): void {
        const slot = this.byId.get(decrypted.messageId)
        if (!slot || slot.done) {
            // No stanza to attach it to: one already given up on, or a frame
            // never seen. Inventing somewhere to put it would be worse.
            return
        }

        if (!slot.encIndices.includes(decrypted.childIndex)) {
            // A payload for a child this stanza has no `<enc>` at. The engine
            // and the frame disagree; counting it would close the stanza early
            // and drop the payload that was actually coming.
            return
        }

        slot.plaintexts.set(decrypted.childIndex, {
            path: [decrypted.childIndex],
            status: PlaintextStatus.Ok,
            payload: Uint8Array.prototype.slice.call(decrypted.plaintext)
        })
        this.drain(sink)
    }

    /**
     * Emit everything still queued, with whatever it has.
     *
     * For shutdown: a frame still waiting for a payload that will never arrive
     * is better emitted unobserved than lost, the stanza having been real.
     */
    public flush(sink: StanzaSink): void {
        for (const slot of this.queue) {
            slot.done = true
        }

        this.drain(sink)
    }

    /** How many stanzas are waiting on payloads. */
    public get pending(): number {
        return this.queue.filter(slot => !ready(slot)).length
    }

    /** How many are queued, waiting or merely behind one that is. */
    public get queued(): number {
        return this.queue.length
    }

    private age(): void {
        for (const slot of this.queue) {
            if (slot.done) {
                continue
            }

            slot.age += 1
            if (slot.age > this.lookahead) {
                slot.done = true
            }
        }
    }

    /**
     * Take the finished stanzas off the front.
     *
     * The front, and only the front: a finished stanza behind an unfinished one
     * waits, because the unfinished one arrived first and that is the order the
     * wire had.
     */
    private drain(sink: StanzaSink): void {
        let at = 0
        for (; at < this.queue.length; at++) {
            const slot = this.queue[at]!
            if (!ready(slot)) {
                break
            }

            if (slot.id !== '' && this.byId.get(slot.id) === slot) {
                this.byId.delete(slot.id)
            }

            sink(envelopeOf(slot))
        }

        this.queue = this.queue.slice(at)
    }
}

/**
 * What this stanza has.
 *
 * An `<enc>` that produced nothing is reported as `Unobserved` rather than
 * omitted: a table missing an entry says the node was not there, and it was.
 * `Unobserved` claims only what this adapter can see — that no payload arrived
 * — and not why, which it cannot.
 */
const envelopeOf = (slot: Slot): Stanza => {
    if (slot.encIndices.length === 0) {
        return { direction: Direction.Inbound, frameOrigin: FrameOrigin.Original, frame: slot.frame }
    }

    const plaintexts = slot.encIndices.map(
        index =>
            slot.plaintexts.get(index) ?? {
                path: [index],
                status: PlaintextStatus.Unobserved,
                payload: new Uint8Array(0)
            }
    )

    return {
        direction: Direction.Inbound,
        frameOrigin: FrameOrigin.Original,
        frame: slot.frame,
        plaintexts
    }
}

/**
 * The positions of a stanza's `<enc>` children among all of its children.
 *
 * The same numbering the engine reports a payload under — an `<enc>` is not
 * necessarily the first child, and counting `<enc>` nodes instead would address
 * a plaintext to the wrong node.
 */
const encChildIndices = (node: Node): number[] => {
    if (!Array.isArray(node.content)) {
        return []
    }

    const indices: number[] = []
    for (const [index, child] of node.content.entries()) {
        if (typeof child === 'object' && child !== null && (child as Node).tag === 'enc') {
            indices.push(index)
        }
    }

    return indices
}
