/**
 * The wa-wire boundary format, written out.
 *
 * This is the same layout `wa-wire-contract` decodes, and the two must agree
 * byte for byte — a stanza encoded here is read there. The layout is fixed by
 * RFC-008:
 *
 * ```text
 * Envelope
 *   version      u16
 *   flags        u16     bit0 direction, bit1 frame_origin
 *   frame_len    u32
 *   frame        u8[frame_len]
 *   pt_count     u16
 *   pt_entries   PlaintextEntry[pt_count]
 *
 * PlaintextEntry
 *   path_len     u8
 *   path         u16[path_len]      little-endian child indices from the root
 *   status       u8
 *   payload_len  u32
 *   payload      u8[payload_len]
 * ```
 *
 * Little-endian throughout — unlike the stanza inside `frame`, which is
 * WhatsApp's own big-endian encoding and travels untouched.
 */

/** The contract version this module writes. */
export const CONTRACT_VERSION = 1

/** Bytes before the frame: version, flags, frame length. */
export const HEADER_LEN = 8

/** Which way a stanza was travelling. */
export enum Direction {
    Inbound = 0,
    Outbound = 1,
}

/** Whether the frame is the engine's own buffer or a re-encoding. */
export enum FrameOrigin {
    /** The buffer the engine's decoder consumed, verbatim. */
    Original = 0,
    /** Re-encoded from a decoded node, because the bytes were not reachable. */
    ReEncoded = 1,
}

/** Whether a plaintext entry holds usable bytes, and if not, why. */
export enum PlaintextStatus {
    Ok = 0,
    DecryptFailed = 1,
    Unsupported = 2,
}

/** One decrypted payload, addressed by the path of the node it came from. */
export interface Plaintext {
    /** Child indices from the root node. */
    readonly path: readonly number[]
    readonly status: PlaintextStatus
    /** Empty unless `status` is `Ok`. */
    readonly payload: Uint8Array
}

/** One stanza, ready to encode. */
export interface Stanza {
    readonly direction: Direction
    readonly frameOrigin: FrameOrigin
    /** The unpacked binary-node buffer. */
    readonly frame: Uint8Array
    readonly plaintexts?: readonly Plaintext[]
}

/** A value could not be represented in the envelope. */
export class EncodeError extends Error {
    public constructor(message: string) {
        super(message)
        this.name = 'EncodeError'
    }
}

const U8_MAX = 0xff
const U16_MAX = 0xffff
const U32_MAX = 0xffff_ffff

/** How many bytes `stanza` will encode to. */
export function encodedLength(stanza: Stanza): number {
    requireU32(stanza.frame.length, 'frame')
    const plaintexts = stanza.plaintexts ?? []
    requireU16(plaintexts.length, 'plaintext count')

    let total = HEADER_LEN + stanza.frame.length + 2
    for (const plaintext of plaintexts) {
        requireU8(plaintext.path.length, 'path')
        requireU32(plaintext.payload.length, 'payload')
        total += 1 + plaintext.path.length * 2 + 1 + 4 + plaintext.payload.length
    }
    return total
}

/**
 * Encode one stanza.
 *
 * Sized first and written once, so the buffer is never grown mid-write and the
 * length the decoder reads is the length that was produced.
 */
export function encodeEnvelope(stanza: Stanza): Uint8Array {
    const out = new Uint8Array(encodedLength(stanza))
    const view = new DataView(out.buffer)
    let at = 0

    view.setUint16(at, CONTRACT_VERSION, true)
    at += 2
    view.setUint16(at, packFlags(stanza), true)
    at += 2
    view.setUint32(at, stanza.frame.length, true)
    at += 4
    out.set(stanza.frame, at)
    at += stanza.frame.length

    const plaintexts = stanza.plaintexts ?? []
    view.setUint16(at, plaintexts.length, true)
    at += 2

    for (const plaintext of plaintexts) {
        out[at] = plaintext.path.length
        at += 1
        for (const component of plaintext.path) {
            requireU16(component, 'path component')
            view.setUint16(at, component, true)
            at += 2
        }
        out[at] = plaintext.status
        at += 1
        view.setUint32(at, plaintext.payload.length, true)
        at += 4
        out.set(plaintext.payload, at)
        at += plaintext.payload.length
    }

    return out
}

function packFlags(stanza: Stanza): number {
    let bits = 0
    if (stanza.direction === Direction.Outbound) bits |= 1 << 0
    if (stanza.frameOrigin === FrameOrigin.ReEncoded) bits |= 1 << 1
    return bits
}

/**
 * Check a length against the prefix that will carry it.
 *
 * Exported because the frame and payload limits need a 4 GiB buffer to reach
 * through `encodeEnvelope`, so the narrowing is checked here directly rather
 * than left as a branch no test can enter.
 */
export function fitsPrefix(value: number, bits: 8 | 16 | 32): boolean {
    const max = bits === 8 ? U8_MAX : bits === 16 ? U16_MAX : U32_MAX
    return Number.isInteger(value) && value >= 0 && value <= max
}

function requireU8(value: number, what: string): void {
    if (!fitsPrefix(value, 8)) {
        throw new EncodeError(`${what} of ${value} does not fit in a byte`)
    }
}

function requireU16(value: number, what: string): void {
    if (!fitsPrefix(value, 16)) {
        throw new EncodeError(`${what} of ${value} does not fit in 16 bits`)
    }
}

function requireU32(value: number, what: string): void {
    if (!fitsPrefix(value, 32)) {
        throw new EncodeError(`${what} of ${value} does not fit in 32 bits`)
    }
}
