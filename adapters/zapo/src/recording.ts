/**
 * The wa-wire recording container, written from TypeScript.
 *
 * The second implementation of RFC-010, for the same reason the envelope has
 * one: an adapter has to run inside a JavaScript engine, and two descriptions
 * of one format that are only ever tested separately are two formats waiting to
 * diverge. Fixtures written here are read by the Rust crate and the reverse.
 *
 * Little-endian throughout, matching the envelope inside it.
 *
 * # Truncation is a state, not a failure
 *
 * The record count lives in a trailer rather than the header, so a writer never
 * has to know its own length before the first byte. An interrupted recording
 * therefore has no trailer, and {@link decodeRecording} reports that as
 * `truncated` with every complete record still readable — the artifact a crash
 * recorder exists to produce is by definition the interrupted one.
 */

/** Every recording starts with these four bytes. */
export const MAGIC = Uint8Array.from([0x57, 0x41, 0x57, 0x52]) // "WAWR"

/** The container layout this build writes. */
export const CONTAINER_VERSION = 1

/** Magic, version and metadata length. */
export const RECORDING_HEADER_LEN = 10

/** Marks a tag a reader must understand to compare the recording. */
export const CRITICAL_BIT = 0x8000

/** Metadata tags. The critical ones decide comparability. */
export const RecordingTag = {
    Adapter: CRITICAL_BIT | 0x0001,
    Provenance: CRITICAL_BIT | 0x0002,
    Dictionary: CRITICAL_BIT | 0x0003,
    ArtifactClass: CRITICAL_BIT | 0x0004,
    InputDigest: CRITICAL_BIT | 0x0005,
    Transform: CRITICAL_BIT | 0x0006,
    CreatedAt: 0x0001,
    Note: 0x0002
} as const

export type RecordingTag = (typeof RecordingTag)[keyof typeof RecordingTag]

/** How a recording came to exist. */
export const ArtifactClass = {
    /** From a live session, so nothing else saw the same traffic. */
    Captured: 0,
    /** Produced by replaying another recording through an engine. */
    Replayed: 1,
    /** Derived from another recording by rewriting its frames. */
    Sanitized: 2,
    /** Written by hand or by a generator. */
    Synthetic: 3
} as const

export type ArtifactClass = (typeof ArtifactClass)[keyof typeof ArtifactClass]

/** What a record is. */
export const RecordKind = {
    /** An RFC-008 envelope, verbatim. */
    Envelope: 0x00,
    /** A delta in microseconds, then a UTF-8 label. */
    Mark: 0x01,
    /** The last record: how many came before it, and their checksum. */
    Trailer: 0xff
} as const

export type RecordKind = (typeof RecordKind)[keyof typeof RecordKind]

/** A value could not be represented in the container. */
export class RecordingWriteError extends Error {
    public constructor(message: string) {
        super(message)
        this.name = 'RecordingWriteError'
    }
}

/** A buffer is not a readable recording. */
export class RecordingReadError extends Error {
    public constructor(message: string) {
        super(message)
        this.name = 'RecordingReadError'
    }
}

/**
 * CRC-32, for detecting damage.
 *
 * Not for detecting tampering: anything able to rewrite the records can rewrite
 * the checksum, and nothing here is signed. Hand-written rather than taken from
 * a Node module so the adapter still runs in a browser, and pinned against
 * published vectors rather than against its Rust twin's output.
 */
export function crc32(bytes: Uint8Array): number {
    let crc = 0xffffffff
    for (const byte of bytes) {
        crc ^= byte
        for (let bit = 0; bit < 8; bit += 1) {
            const mask = -(crc & 1)
            crc = (crc >>> 1) ^ (0xedb88320 & mask)
        }
    }
    return (crc ^ 0xffffffff) >>> 0
}

/** One metadata entry, ready to write. */
export interface MetaEntry {
    readonly tag: number
    readonly value: Uint8Array
}

/** What a recording says about itself. */
export interface RecordingMeta {
    readonly adapter?: {
        readonly id: string
        readonly version: string
        readonly engineVersion: string
        readonly contractVersion: number
        readonly capabilities: readonly string[]
    }
    readonly provenance?: {
        readonly whatsappVersion: string
        readonly manifestHash: string
        readonly generatorVersion: string
    }
    readonly dictionary?: string
    readonly artifactClass?: ArtifactClass
    /** The traffic this is a replay of. Omitted by a live capture. */
    readonly inputDigest?: Uint8Array
    readonly transform?: { readonly identity: string; readonly configDigest: string }
    readonly createdAt?: bigint
    readonly note?: string
    /** Anything this build does not model, preserved as written. */
    readonly extra?: readonly MetaEntry[]
}

/** One record to write. */
export type RecordInput =
    | { readonly kind: typeof RecordKind.Envelope; readonly envelope: Uint8Array }
    | { readonly kind: typeof RecordKind.Mark; readonly deltaUs: number; readonly label: string }
    | { readonly kind: number; readonly payload: Uint8Array }

/** Whether a recording ends where it says it does. */
export type Integrity =
    | { readonly kind: 'complete' }
    | {
          readonly kind: 'damaged'
          readonly claimed: number
          readonly found: number
          readonly checksumOk: boolean
      }
    | { readonly kind: 'truncated'; readonly found: number; readonly dangling: number }

/** A decoded recording. */
export interface DecodedRecording {
    readonly containerVersion: number
    readonly meta: readonly MetaEntry[]
    readonly records: ReadonlyArray<{ readonly kind: number; readonly payload: Uint8Array }>
    readonly envelopes: readonly Uint8Array[]
    readonly integrity: Integrity
    /** Critical tags this build could not interpret. */
    readonly unknownCriticalTags: number
    /** Records this build skipped because it does not know the kind. */
    readonly skippedRecords: number
}

class Writer {
    private readonly parts: Uint8Array[] = []

    public bytes(value: Uint8Array): void {
        this.parts.push(value)
    }

    public u8(value: number): void {
        this.parts.push(Uint8Array.from([value & 0xff]))
    }

    public u16(value: number): void {
        const out = new Uint8Array(2)
        new DataView(out.buffer).setUint16(0, value, true)
        this.parts.push(out)
    }

    public u32(value: number): void {
        const out = new Uint8Array(4)
        new DataView(out.buffer).setUint32(0, value >>> 0, true)
        this.parts.push(out)
    }

    public u64(value: bigint): void {
        const out = new Uint8Array(8)
        new DataView(out.buffer).setBigUint64(0, value, true)
        this.parts.push(out)
    }

    /** A `u16`-prefixed UTF-8 string. */
    public str(value: string): void {
        const encoded = new TextEncoder().encode(value)
        if (encoded.length > 0xffff) {
            throw new RecordingWriteError(
                `string of ${encoded.length} byte(s) does not fit in 16 bits`
            )
        }
        this.u16(encoded.length)
        this.bytes(encoded)
    }

    public collect(): Uint8Array {
        let total = 0
        for (const part of this.parts) {
            total += part.length
        }
        const out = new Uint8Array(total)
        let at = 0
        for (const part of this.parts) {
            out.set(part, at)
            at += part.length
        }
        return out
    }
}

function encodeMeta(meta: RecordingMeta): MetaEntry[] {
    const entries: MetaEntry[] = []

    if (meta.adapter !== undefined) {
        const w = new Writer()
        w.str(meta.adapter.id)
        w.str(meta.adapter.version)
        w.str(meta.adapter.engineVersion)
        w.u16(meta.adapter.contractVersion)
        if (meta.adapter.capabilities.length > 0xffff) {
            throw new RecordingWriteError('too many capability identifiers')
        }
        w.u16(meta.adapter.capabilities.length)
        for (const capability of meta.adapter.capabilities) {
            w.str(capability)
        }
        entries.push({ tag: RecordingTag.Adapter, value: w.collect() })
    }

    if (meta.provenance !== undefined) {
        const w = new Writer()
        w.str(meta.provenance.whatsappVersion)
        w.str(meta.provenance.manifestHash)
        w.str(meta.provenance.generatorVersion)
        entries.push({ tag: RecordingTag.Provenance, value: w.collect() })
    }

    if (meta.dictionary !== undefined) {
        const w = new Writer()
        w.str(meta.dictionary)
        entries.push({ tag: RecordingTag.Dictionary, value: w.collect() })
    }

    if (meta.artifactClass !== undefined) {
        entries.push({
            tag: RecordingTag.ArtifactClass,
            value: Uint8Array.from([meta.artifactClass])
        })
    }

    if (meta.inputDigest !== undefined) {
        entries.push({ tag: RecordingTag.InputDigest, value: meta.inputDigest })
    }

    if (meta.transform !== undefined) {
        const w = new Writer()
        w.str(meta.transform.identity)
        w.str(meta.transform.configDigest)
        entries.push({ tag: RecordingTag.Transform, value: w.collect() })
    }

    if (meta.createdAt !== undefined) {
        const w = new Writer()
        w.u64(meta.createdAt)
        entries.push({ tag: RecordingTag.CreatedAt, value: w.collect() })
    }

    if (meta.note !== undefined) {
        entries.push({ tag: RecordingTag.Note, value: new TextEncoder().encode(meta.note) })
    }

    entries.push(...(meta.extra ?? []))

    const seen = new Set<number>()
    for (const entry of entries) {
        if (seen.has(entry.tag)) {
            // A reader takes the first, so a duplicate is a value the writer
            // believes it set and the reader will never see.
            throw new RecordingWriteError(`metadata tag ${entry.tag} written twice`)
        }
        seen.add(entry.tag)
    }
    return entries
}

function encodeRecord(record: RecordInput): { kind: number; payload: Uint8Array } {
    if (record.kind === RecordKind.Envelope && 'envelope' in record) {
        return { kind: RecordKind.Envelope, payload: record.envelope }
    }
    if (record.kind === RecordKind.Mark && 'deltaUs' in record) {
        const w = new Writer()
        w.u32(record.deltaUs)
        w.bytes(new TextEncoder().encode(record.label))
        return { kind: RecordKind.Mark, payload: w.collect() }
    }
    if ('payload' in record) {
        return { kind: record.kind, payload: record.payload }
    }
    throw new RecordingWriteError(`record of kind ${record.kind} has no payload`)
}

/**
 * Encode a whole recording, trailer included.
 *
 * For a writer that may be interrupted — a ring buffer — write the same bytes
 * incrementally and simply stop: a recording with no trailer is readable, and
 * reports itself as truncated.
 */
export function encodeRecording(
    meta: RecordingMeta,
    records: readonly RecordInput[]
): Uint8Array {
    const entries = encodeMeta(meta)

    const metaWriter = new Writer()
    for (const entry of entries) {
        metaWriter.u16(entry.tag)
        metaWriter.u32(entry.value.length)
        metaWriter.bytes(entry.value)
    }
    const metaBytes = metaWriter.collect()

    const body = new Writer()
    body.bytes(MAGIC)
    body.u16(CONTAINER_VERSION)
    body.u32(metaBytes.length)
    body.bytes(metaBytes)

    let count = 0
    for (const record of records) {
        const { kind, payload } = encodeRecord(record)
        if (payload.length > 0xffff_ffff) {
            throw new RecordingWriteError(`record of ${payload.length} byte(s) exceeds u32`)
        }
        body.u8(kind)
        body.u32(payload.length)
        body.bytes(payload)
        count += 1
    }

    const withoutTrailer = body.collect()
    const trailer = new Writer()
    // Everything before the trailer, which is exactly what a reader
    // recomputes. The count is not covered by it and does not need to be: a
    // damaged count is caught by disagreeing with what was actually found.
    trailer.u32(count)
    trailer.u32(crc32(withoutTrailer))
    const trailerPayload = trailer.collect()

    const out = new Writer()
    out.bytes(withoutTrailer)
    out.u8(RecordKind.Trailer)
    out.u32(trailerPayload.length)
    out.bytes(trailerPayload)
    return out.collect()
}

class Reader {
    private at = 0

    public constructor(private readonly buf: Uint8Array) {}

    public get remaining(): number {
        return this.buf.length - this.at
    }

    public get position(): number {
        return this.at
    }

    public take(n: number): Uint8Array | undefined {
        if (n < 0 || this.remaining < n) {
            return undefined
        }
        const out = this.buf.subarray(this.at, this.at + n)
        this.at += n
        return out
    }

    public u8(): number | undefined {
        const bytes = this.take(1)
        return bytes === undefined ? undefined : bytes[0]
    }

    public u16(): number | undefined {
        const bytes = this.take(2)
        return bytes === undefined
            ? undefined
            : new DataView(bytes.buffer, bytes.byteOffset, 2).getUint16(0, true)
    }

    public u32(): number | undefined {
        const bytes = this.take(4)
        return bytes === undefined
            ? undefined
            : new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0, true)
    }
}

const KNOWN_TAGS = new Set<number>(Object.values(RecordingTag))
const KNOWN_KINDS = new Set<number>(Object.values(RecordKind))

/**
 * Read a recording.
 *
 * Throws only when the buffer never was a recording: bad magic, a header too
 * short to hold, a metadata block that runs past the end, or a container
 * version this build does not implement. A recording whose *records* stop early
 * is reported through `integrity`, not thrown.
 */
export function decodeRecording(buf: Uint8Array): DecodedRecording {
    if (buf.length < RECORDING_HEADER_LEN) {
        throw new RecordingReadError(
            `header needs ${RECORDING_HEADER_LEN} byte(s), ${buf.length} available`
        )
    }
    for (let i = 0; i < MAGIC.length; i += 1) {
        if (buf[i] !== MAGIC[i]) {
            throw new RecordingReadError('not a wa-wire recording: bad magic')
        }
    }

    const header = new Reader(buf.subarray(4, RECORDING_HEADER_LEN))
    const containerVersion = header.u16() ?? 0
    if (containerVersion !== CONTAINER_VERSION) {
        throw new RecordingReadError(`container version ${containerVersion} is not supported`)
    }
    const metaLen = header.u32() ?? 0
    if (buf.length - RECORDING_HEADER_LEN < metaLen) {
        throw new RecordingReadError(
            `metadata claims ${metaLen} byte(s), ${buf.length - RECORDING_HEADER_LEN} available`
        )
    }

    const metaBytes = buf.subarray(RECORDING_HEADER_LEN, RECORDING_HEADER_LEN + metaLen)
    const meta: MetaEntry[] = []
    let unknownCriticalTags = 0
    const metaReader = new Reader(metaBytes)
    while (metaReader.remaining > 0) {
        const tag = metaReader.u16()
        const len = metaReader.u32()
        const value = len === undefined ? undefined : metaReader.take(len)
        if (tag === undefined || value === undefined) {
            throw new RecordingReadError(`metadata entry ${tag ?? 0} runs past the block`)
        }
        if ((tag & CRITICAL_BIT) !== 0 && !KNOWN_TAGS.has(tag)) {
            unknownCriticalTags += 1
        }
        meta.push({ tag, value })
    }

    const bodyStart = RECORDING_HEADER_LEN + metaLen
    const body = new Reader(buf.subarray(bodyStart))
    const records: Array<{ kind: number; payload: Uint8Array }> = []
    const envelopes: Uint8Array[] = []
    let skippedRecords = 0
    let integrity: Integrity = { kind: 'truncated', found: 0, dangling: body.remaining }

    for (;;) {
        const before = body.position
        const kind = body.u8()
        const len = kind === undefined ? undefined : body.u32()
        const payload = len === undefined ? undefined : body.take(len)
        if (kind === undefined || payload === undefined) {
            integrity = {
                kind: 'truncated',
                found: records.length,
                dangling: buf.length - bodyStart - before
            }
            break
        }

        if (kind === RecordKind.Trailer) {
            const trailer = new Reader(payload)
            const claimed = trailer.u32() ?? 0
            const stated = trailer.u32() ?? 0
            const actual = crc32(buf.subarray(0, bodyStart + before))
            const checksumOk = actual === stated
            integrity =
                checksumOk && claimed === records.length
                    ? { kind: 'complete' }
                    : { kind: 'damaged', claimed, found: records.length, checksumOk }
            break
        }

        if (!KNOWN_KINDS.has(kind)) {
            skippedRecords += 1
        }
        if (kind === RecordKind.Envelope) {
            envelopes.push(payload)
        }
        records.push({ kind, payload })
    }

    return {
        containerVersion,
        meta,
        records,
        envelopes,
        integrity,
        unknownCriticalTags,
        skippedRecords
    }
}

/** Read a mark's delta and label, if the record is one. */
export function readMark(record: {
    readonly kind: number
    readonly payload: Uint8Array
}): { readonly deltaUs: number; readonly label: string } | undefined {
    if (record.kind !== RecordKind.Mark || record.payload.length < 4) {
        return undefined
    }
    const view = new DataView(
        record.payload.buffer,
        record.payload.byteOffset,
        record.payload.length
    )
    return {
        deltaUs: view.getUint32(0, true),
        label: new TextDecoder('utf-8', { fatal: false }).decode(record.payload.subarray(4))
    }
}
