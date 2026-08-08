/**
 * What the Baileys adapter claims.
 *
 * The vocabulary is the contract's and lives in `@oxidezap/wa-wire-ts`; what an
 * adapter has is its own, and belongs with the adapter.
 */

import {
    Capability,
    CONTRACT_VERSION,
    missing as missingFrom,
    require as requireOf,
    type AdapterInfo
} from '@oxidezap/wa-wire-ts'

/** The adapter version, as it appears in a recording. */
export const ADAPTER_VERSION = '0.1.0'

/** The engine version this was written against. */
export const ENGINE_VERSION = '7.0.0-rc14'

/** How this adapter names itself in a recording. */
export const ID = 'baileys'

/**
 * What a tap provides.
 *
 * Two notable presences and two notable absences.
 *
 * **`l0.inbound.auth-phase`** — the frame hook sits inside the Noise frame
 * loop, before anything decides what a stanza is, so `success` and `failure`
 * reach it. `zapo` protects those from its stanza filters and cannot.
 *
 * **`l0.zero-copy-frame`** — the hook carries the buffer the node was decoded
 * from. Baileys had it in scope and dropped it; the observation point added for
 * this adapter hands it over, so nothing is re-encoded.
 *
 * **No `l0.outbound.observed`** — nothing reports what the client sent. Only
 * `whatsapp-rust` does today.
 *
 * **No `lifecycle.drain-hook`** — nothing says when handlers have drained, so a
 * consumer cannot know its queue is quiet. Absent rather than approximated.
 */
export const TAP_CAPABILITIES: readonly Capability[] = [
    Capability.L0InboundTap,
    Capability.L0InboundAuthPhase,
    Capability.L0Plaintext,
    Capability.ZeroCopyFrame
]

/** What this adapter claims. */
export const INFO: AdapterInfo = {
    id: ID,
    version: ADAPTER_VERSION,
    engineVersion: ENGINE_VERSION,
    contractVersion: CONTRACT_VERSION,
    capabilities: TAP_CAPABILITIES
}

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
