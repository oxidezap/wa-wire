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
    /** Raw request/response against a stanza. */
    L0Request: 'l0.request',
    /** Emits the payloads it decrypted alongside the frame. */
    L0Plaintext: 'l0.plaintext',
    /** Suppresses the engine's own dispatch, leaving it as transport and acks. */
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
 * What this adapter claims.
 *
 * Every entry is asserted against real behaviour in this package's tests, so a
 * claim cannot quietly stop being true.
 *
 * Three notable absences:
 *
 * - **`l0.inbound.auth-phase`** — `zapo` protects `success` and `failure` from
 *   stanza filters, so the tap does not see the authentication exchange. That
 *   is deliberate on `zapo`'s side: a filter that dropped `success` would break
 *   the login flow.
 * - **`l0.zero-copy-frame`** — the filter receives a decoded node, not the
 *   buffer it came from, so the frame is re-encoded. See the README for the
 *   one-line change upstream that would close this.
 * - **`l0.plaintext`** — the filter runs before decryption, so a `<message>`
 *   crosses with its ciphertext and no plaintext table.
 */
export const INFO: AdapterInfo = {
    id: 'zapo',
    version: '0.1.0',
    engineVersion: '1.7',
    contractVersion: 1,
    capabilities: [
        Capability.L0InboundTap,
        Capability.L0Plaintext,
        Capability.Takeover,
        Capability.DrainHook,
    ],
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

/** Whether `info` declares `capability`. */
export function declares(info: AdapterInfo, capability: Capability): boolean {
    return info.capabilities.includes(capability)
}

/** Whether this adapter declares `capability`. */
export function has(capability: Capability): boolean {
    return INFO.capabilities.includes(capability)
}

/** Which of `required` this adapter lacks. */
export function missing(required: readonly Capability[]): Capability[] {
    return required.filter((capability) => !has(capability))
}

/** Raised when a consumer required capabilities this adapter does not have. */
export class UnmetCapabilitiesError extends Error {
    constructor(public readonly missing: readonly Capability[]) {
        super(`adapter lacks required capabilities: ${missing.join(', ')}`)
        this.name = 'UnmetCapabilitiesError'
    }
}

/**
 * Throw unless this adapter has every capability in `required`.
 *
 * The setup-time gate. Without it a consumer discovers that its engine never
 * emits plaintext, or re-encodes frames it meant to replay, as *missing
 * traffic* — where the evidence of the problem is the thing that is absent.
 * Naming the requirement turns that into a refused install.
 *
 * Reports everything missing at once, so a caller fixes its setup in one pass
 * rather than one round trip per capability.
 */
export function require(required: readonly Capability[]): void {
    const absent = missing(required)
    if (absent.length > 0) {
        throw new UnmetCapabilitiesError(absent)
    }
}
