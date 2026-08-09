/**
 * What this adapter can and cannot do, stated rather than implied.
 *
 * No engine offers everything, and the gaps are real rather than cosmetic. A
 * consumer that needs a capability the adapter lacks should find that out at
 * setup, from this list, instead of discovering it as missing traffic.
 */

import { Direction, FrameOrigin, PlaintextStatus, type Stanza } from './envelope.js'

/** One thing an adapter may or may not be able to do. */
export const Capability = {
    /** Observes every inbound stanza, without exception. */
    L0InboundTap: 'l0.inbound.tap',
    /** The inbound tap also covers the auth and stream-control phase. */
    L0InboundAuthPhase: 'l0.inbound.auth-phase',
    /** Sends a raw stanza. */
    L0Outbound: 'l0.outbound',
    /**
     * Reports each stanza the engine sent, as it went to the wire.
     *
     * Distinct from {@link L0Outbound}, which is the ability to send. Sending
     * is what an adapter does; knowing what left is what a recording needs.
     *
     * Named here even though this adapter does not have it: the vocabulary is
     * the contract's, not one adapter's, and a consumer that cannot name a
     * capability cannot require one either — it would discover the absence as
     * missing traffic, which is the outcome this list exists to prevent.
     */
    L0OutboundObserved: 'l0.outbound.observed',
    /** Raw request/response against a stanza. */
    L0Request: 'l0.request',
    /** Emits the payloads it decrypted alongside the frame. */
    L0Plaintext: 'l0.plaintext',
    /**
     * Says *why* an `<enc>` produced no plaintext, not merely that none
     * arrived.
     *
     * `PlaintextStatus` has carried `DecryptFailed` and `Unsupported` since the
     * format was written, and no adapter has ever emitted either: all four
     * watch payloads appear and are never told why one did not, so they report
     * `Unobserved` and no cause.
     *
     * The distinction is what a gate needs. Under `Unobserved`, a candidate
     * build whose messages stopped decrypting looks exactly like one whose
     * adapter stopped observing — the failure and the blind spot are the same
     * absence.
     */
    L0PlaintextCause: 'l0.plaintext.cause',
    /**
     * Suppresses the engine's own dispatch, leaving it as transport and acks.
     *
     * Never suppresses decryption: L0-plain is produced by the engine, so a
     * takeover that stopped it would silently downgrade the contract to
     * L0-wire. A stanza carrying ciphertext still reaches the engine.
     */
    Takeover: 'l0.takeover',
    /** Supplies the engine's original frame bytes, so nothing is re-encoded. */
    ZeroCopyFrame: 'l0.zero-copy-frame',
    /** Reports when incoming handlers have drained. */
    DrainHook: 'lifecycle.drain-hook',
} as const

export type Capability = (typeof Capability)[keyof typeof Capability]

/** An adapter's identity and capabilities. */
export interface AdapterInfo {
    readonly id: string
    readonly version: string
    readonly engineVersion: string
    readonly contractVersion: number
    readonly capabilities: readonly Capability[]
}



export function declares(info: AdapterInfo, capability: Capability): boolean {
    return info.capabilities.includes(capability)
}

/**
 * Which of `required` is not in `provided`.
 *
 * Both sets are the caller's: an installed instance provides what that instance
 * provides, which is not always everything its adapter can do. A default here
 * would have to name one adapter, and this package knows none.
 */
export function missing(required: readonly Capability[], provided: readonly Capability[]): Capability[] {
    return required.filter((capability) => !provided.includes(capability))
}

/** Raised when a consumer required capabilities this adapter does not have. */
export class UnmetCapabilitiesError extends Error {
    constructor(public readonly missing: readonly Capability[]) {
        super(`adapter lacks required capabilities: ${missing.join(', ')}`)
        this.name = 'UnmetCapabilitiesError'
    }
}

/**
 * Throw unless `provided` covers every capability in `required`.
 *
 * The setup-time gate. Without it a consumer discovers that its engine never
 * emits plaintext, or re-encodes frames it meant to replay, as *missing
 * traffic* — where the evidence of the problem is the thing that is absent.
 * Naming the requirement turns that into a refused install.
 *
 * Checked against what the instance provides rather than against everything
 * its adapter could do: one that can take over but was installed as a tap
 * suppresses nothing, and a consumer told otherwise would be waiting for the
 * engine to stop interpreting stanzas it is still interpreting.
 *
 * Reports everything missing at once, so a caller fixes its setup in one pass
 * rather than one round trip per capability.
 */
export function require(required: readonly Capability[], provided: readonly Capability[]): void {
    const absent = missing(required, provided)
    if (absent.length > 0) {
        throw new UnmetCapabilitiesError(absent)
    }
}

/**
 * Whether an envelope contradicts the declaration that produced it, and how.
 *
 * The declaration is what a consumer selects an engine on, so a capability that
 * has stopped being true should fail rather than quietly mislead. Both sides of
 * the boundary check it, because each is the other's only guard: a producer in
 * TypeScript does not run the Rust encoder, and a consumer in Rust does not run
 * this one.
 *
 * Returns the reason rather than throwing, so a caller decides whether a
 * contradiction is worth stopping for. `undefined` when nothing is wrong.
 */
export function verify(info: AdapterInfo, stanza: Stanza): string | undefined {
    if (declares(info, Capability.ZeroCopyFrame) && stanza.frameOrigin === FrameOrigin.ReEncoded) {
        return 'declared l0.zero-copy-frame and delivered a re-encoded frame'
    }

    if (!declares(info, Capability.L0Plaintext) && (stanza.plaintexts?.length ?? 0) > 0) {
        return 'delivered plaintexts without declaring l0.plaintext'
    }

    // `l0.outbound` is the ability to send and says nothing about seeing what
    // left, which is why this names the other one.
    if (!declares(info, Capability.L0OutboundObserved) && stanza.direction === Direction.Outbound) {
        return 'delivered an outbound stanza without declaring l0.outbound.observed'
    }

    // A cause is a claim about a failure, and only an adapter that reports
    // failures can make one. Without the capability an entry says `Unobserved`
    // — no payload arrived, cause unknown — which is what every adapter has
    // said so far.
    if (
        !declares(info, Capability.L0PlaintextCause) &&
        stanza.plaintexts?.some(
            (entry) =>
                entry.status === PlaintextStatus.DecryptFailed ||
                entry.status === PlaintextStatus.Unsupported
        )
    ) {
        return 'named a cause for a missing plaintext without declaring l0.plaintext.cause'
    }

    return undefined
}
