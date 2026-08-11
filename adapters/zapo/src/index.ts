/**
 * wa-wire adapter for the zapo engine.
 *
 * `zapo` exposes an incoming stanza filter that runs against every inbound
 * stanza before any handler, and that one hook is the whole adapter: observe,
 * encode, hand on. Returning `true` from it also suppresses the engine's own
 * dispatch, which is how takeover works without a fork.
 *
 * ```ts
 * import { WaClient } from 'zapo-js'
 * import { Mode, waWire } from '@oxidezap/wa-wire-adapter-zapo'
 *
 * const client = new WaClient({
 *     plugins: [waWire({ sink: (stanza) => queue.push(stanza) })],
 * })
 * ```
 *
 * What it can and cannot do is in {@link INFO}, and asserted in this package's
 * tests rather than left as a claim.
 */

export { Capability, UnmetCapabilitiesError, declares, type AdapterInfo } from '@oxidezap/wa-wire-ts'
export { has, missing, require } from './capability.js'
export { DETACHING_INFO, REQUESTING_INFO, SENDING_INFO, TAKEOVER_CAPABILITIES, TAP_CAPABILITIES } from './capability.js'
export {
    DetachError,
    createDetacher,
    type SessionClient,
    type SessionDetacher,
} from './handoff.js'
export {
    NotConnectedError,
    RequestError,
    RequestTimeoutError,
    SendError,
    createRequester,
    createSender,
    type NodeRequester,
    type NodeSender,
    type StanzaRequester as OutboundRequester,
    type StanzaSender as OutboundSender,
} from './send.js'
export { ArtifactClass, CONTAINER_VERSION, CRITICAL_BIT, MAGIC, RECORDING_HEADER_LEN, RecordKind, RecordingReadError, RecordingTag, RecordingWriteError, crc32, decodeRecording, encodeRecording, readMark, type DecodedRecording, type Integrity, type MetaEntry, type RecordInput, type RecordingMeta } from '@oxidezap/wa-wire-ts'
export { CONTRACT_VERSION, Direction, EncodeError, FrameOrigin, HEADER_LEN, PlaintextStatus, encodeEnvelope, encodedLength, fitsPrefix, type Plaintext, type Stanza } from '@oxidezap/wa-wire-ts'
export {
    INFO,
    Mode,
    forward,
    supports,
    toEnvelope,
    toStanza,
    waWire,
    type Options,
    type StanzaSink,
} from './adapter.js'
