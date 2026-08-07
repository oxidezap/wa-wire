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
        Capability.Takeover,
        Capability.DrainHook,
    ],
}

/** Whether this adapter declares `capability`. */
export function has(capability: Capability): boolean {
    return INFO.capabilities.includes(capability)
}

/** Which of `required` this adapter lacks. */
export function missing(required: readonly Capability[]): Capability[] {
    return required.filter((capability) => !has(capability))
}
