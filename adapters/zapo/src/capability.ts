/**
 * What the `zapo` adapter claims.
 *
 * The vocabulary is the contract's and lives in `@oxidezap/wa-wire-ts`; what an
 * adapter has is its own, and belongs with the adapter.
 */
import { Capability, missing as missingFrom, require as requireOf, type AdapterInfo } from '@oxidezap/wa-wire-ts'

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

/** Whether this adapter declares `capability`. */
export function has(capability: Capability): boolean {
    return INFO.capabilities.includes(capability)
}

/**
 * Which of `required` this adapter does not provide.
 *
 * Defaults to everything it can do. An installed instance passes the set that
 * instance actually provides, which is not always the same thing.
 */
export function missing(
    required: readonly Capability[],
    provided: readonly Capability[] = INFO.capabilities
): Capability[] {
    return missingFrom(required, provided)
}

/** Throw unless `provided` covers every capability in `required`. */
export function require(
    required: readonly Capability[],
    provided: readonly Capability[] = INFO.capabilities
): void {
    requireOf(required, provided)
}
