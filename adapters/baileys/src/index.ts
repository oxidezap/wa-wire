/**
 * The wa-wire adapter for Baileys.
 *
 * ```ts
 * import makeWASocket from 'baileys'
 * import { toEnvelope, waWire } from '@oxidezap/wa-wire-adapter-baileys'
 *
 * const wire = waWire(stanza => record(toEnvelope(stanza)))
 * const sock = makeWASocket({ ...config, ...wire.config })
 * ```
 */

export { toEnvelope, waWire, type WaWire, type WaWireConfig, type WaWireOptions } from './adapter.js'
export {
    DetachError,
    createDetacher,
    type SessionDetacher,
    type SocketEnd
} from './handoff.js'
export {
    ADAPTER_VERSION,
    DETACHING_INFO,
    ENGINE_VERSION,
    ID,
    INFO,
    TAP_CAPABILITIES,
    has,
    missing,
    require
} from './capability.js'
export {
    DEFAULT_LOOKAHEAD,
    PlaintextJoiner,
    type DecryptedEnc,
    type Node,
    type StanzaSink
} from './joiner.js'

// The boundary itself, re-exported so a consumer sees one package.
export {
    Capability,
    Direction,
    FrameOrigin,
    PlaintextStatus,
    UnmetCapabilitiesError,
    declares,
    encodeEnvelope,
    verify,
    type AdapterInfo,
    type Plaintext,
    type Stanza
} from '@oxidezap/wa-wire-ts'
