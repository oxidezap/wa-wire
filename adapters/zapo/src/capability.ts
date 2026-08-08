/**
 * What this adapter can and cannot do, stated rather than implied.
 *
 * No engine offers everything, and the gaps are real rather than cosmetic. A
 * consumer that needs a capability the adapter lacks should find that out at
 * setup, from this list, instead of discovering it as missing traffic.
 */

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

/**
 * What an instance installed as a tap provides.
 *
 * A tap observes and lets the engine carry on, so it provides everything
 * except suppression. Stated separately from {@link INFO} because a consumer
 * holds an instance, not an adapter.
 */
export const TAP_CAPABILITIES: readonly Capability[] = [
    Capability.L0InboundTap,
    Capability.L0Plaintext,
    Capability.DrainHook,
]

/** What an instance installed for takeover provides: a tap, plus suppression. */
export const TAKEOVER_CAPABILITIES: readonly Capability[] = [
    ...TAP_CAPABILITIES,
    Capability.Takeover,
]

/**
 * What this adapter claims.
 *
 * Every entry is asserted against real behaviour in this package's tests, so a
 * claim cannot quietly stop being true.
 *
 * Two notable absences:
 *
 * - **`l0.inbound.auth-phase`** — `zapo` protects `success` and `failure` from
 *   stanza filters, so the tap does not see the authentication exchange. That
 *   is deliberate on `zapo`'s side: a filter that dropped `success` would break
 *   the login flow.
 * - **`l0.zero-copy-frame`** — the filter receives a decoded node, not the
 *   buffer it came from, so the frame is re-encoded. See the README for the
 *   one-line change upstream that would close this.
 *
 * This is everything the adapter *can* do. An installed instance may provide
 * less: see {@link TAP_CAPABILITIES}.
 */
export const INFO: AdapterInfo = {
    id: 'zapo',
    version: '0.1.0',
    engineVersion: '1.7',
    contractVersion: 1,
    capabilities: TAKEOVER_CAPABILITIES,
}

/**
 * What this adapter can do when it is also sending.
 *
 * A separate declaration rather than a flag on {@link INFO}: an adapter built
 * for observation alone genuinely cannot send, and one set covering both would
 * be false for whichever the consumer actually holds.
 */
export const SENDING_INFO: AdapterInfo = {
    ...INFO,
    capabilities: [...INFO.capabilities, Capability.L0Outbound],
}

/**
 * What this adapter can do when it also correlates replies.
 *
 * A ladder, not three unrelated sets: requesting includes sending, which
 * includes observing, so a consumer that raises its requirement never loses
 * something it already relied on.
 */
export const REQUESTING_INFO: AdapterInfo = {
    ...SENDING_INFO,
    capabilities: [...SENDING_INFO.capabilities, Capability.L0Request],
}

/** Whether `info` declares `capability`. */
export function declares(info: AdapterInfo, capability: Capability): boolean {
    return info.capabilities.includes(capability)
}

/** Whether this adapter declares `capability`. */
export function has(capability: Capability): boolean {
    return INFO.capabilities.includes(capability)
}

/**
 * Which of `required` is not in `provided`.
 *
 * Defaults to everything the adapter can do. An installed instance passes the
 * set that instance actually provides, which is not always the same thing.
 */
export function missing(
    required: readonly Capability[],
    provided: readonly Capability[] = INFO.capabilities
): Capability[] {
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
 * Checked against what the instance provides rather than against
 * {@link INFO}: an adapter that can take over but was installed as a tap
 * suppresses nothing, and a consumer told otherwise would be waiting for the
 * engine to stop interpreting stanzas it is still interpreting.
 *
 * Reports everything missing at once, so a caller fixes its setup in one pass
 * rather than one round trip per capability.
 */
export function require(
    required: readonly Capability[],
    provided: readonly Capability[] = INFO.capabilities
): void {
    const absent = missing(required, provided)
    if (absent.length > 0) {
        throw new UnmetCapabilitiesError(absent)
    }
}
