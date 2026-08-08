import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
    ArtifactClass,
    CONTAINER_VERSION,
    CRITICAL_BIT,
    MAGIC,
    RECORDING_HEADER_LEN,
    RecordKind,
    RecordingReadError,
    RecordingTag,
    RecordingWriteError,
    crc32,
    decodeRecording,
    encodeRecording,
    readMark,
    type RecordingMeta
} from '../recording.js'

const META: RecordingMeta = {
    adapter: {
        id: 'zapo',
        version: '0.1.0',
        engineVersion: '1.7',
        contractVersion: 1,
        capabilities: ['l0.inbound.tap', 'l0.plaintext']
    },
    artifactClass: ArtifactClass.Synthetic
}

const envelope = (bytes: number[]) => ({
    kind: RecordKind.Envelope as typeof RecordKind.Envelope,
    envelope: Uint8Array.from(bytes)
})

describe('the checksum', () => {
    it('matches the published vectors', () => {
        // Checked against published values rather than against this
        // implementation's own output: a checksum that only agrees with itself
        // agrees with no other language.
        assert.equal(crc32(new Uint8Array()), 0x00000000)
        assert.equal(crc32(new TextEncoder().encode('a')), 0xe8b7be43)
        assert.equal(crc32(new TextEncoder().encode('abc')), 0x352441c2)
        assert.equal(crc32(new TextEncoder().encode('123456789')), 0xcbf43926)
        assert.equal(
            crc32(new TextEncoder().encode('The quick brown fox jumps over the lazy dog')),
            0x414fa339
        )
    })

    it('changes when a single bit does', () => {
        assert.notEqual(crc32(Uint8Array.from([0x00])), crc32(Uint8Array.from([0x80])))
    })
})

describe('a recording', () => {
    it('round trips with its metadata', () => {
        const bytes = encodeRecording(META, [envelope([1, 2]), envelope([3])])
        const read = decodeRecording(bytes)

        assert.equal(read.containerVersion, CONTAINER_VERSION)
        assert.deepEqual(read.integrity, { kind: 'complete' })
        assert.deepEqual(read.envelopes, [Uint8Array.from([1, 2]), Uint8Array.from([3])])
        assert.equal(read.unknownCriticalTags, 0)
        assert.equal(read.skippedRecords, 0)

        const adapter = read.meta.find((entry) => entry.tag === RecordingTag.Adapter)
        assert.ok(adapter, 'the adapter tag was written')
        const artifact = read.meta.find((entry) => entry.tag === RecordingTag.ArtifactClass)
        assert.deepEqual(artifact?.value, Uint8Array.from([ArtifactClass.Synthetic]))
    })

    it('starts with the magic and the version', () => {
        // Pinned against the Rust reader's own constants. If either side moves,
        // one of the two fails, which is the point of writing it twice.
        const bytes = encodeRecording({}, [])
        assert.deepEqual(bytes.subarray(0, 4), MAGIC)
        assert.equal(new DataView(bytes.buffer).getUint16(4, true), CONTAINER_VERSION)
        assert.equal(new DataView(bytes.buffer).getUint32(6, true), 0, 'no metadata')
        assert.equal(RECORDING_HEADER_LEN, 10)
    })

    it('carries every metadata field it was given', () => {
        const bytes = encodeRecording(
            {
                provenance: {
                    whatsappVersion: '2.3000.1',
                    manifestHash: 'sha256:abc',
                    generatorVersion: '0.1.0'
                },
                dictionary: 'whatspec@2.3000.1',
                artifactClass: ArtifactClass.Sanitized,
                inputDigest: Uint8Array.from([1, 2, 3, 4]),
                transform: { identity: 'pseudonymise-jids', configDigest: 'sha256:cfg' },
                createdAt: 1_754_000_000_000n,
                note: 'from the test account'
            },
            []
        )
        const read = decodeRecording(bytes)
        assert.equal(read.meta.length, 7)
        assert.deepEqual(
            read.meta.find((entry) => entry.tag === RecordingTag.InputDigest)?.value,
            Uint8Array.from([1, 2, 3, 4])
        )
    })

    it('keeps marks out of the envelopes', () => {
        const bytes = encodeRecording(META, [
            envelope([1]),
            { kind: RecordKind.Mark, deltaUs: 1_500, label: 'stream:error' },
            envelope([2])
        ])
        const read = decodeRecording(bytes)

        assert.deepEqual(read.envelopes, [Uint8Array.from([1]), Uint8Array.from([2])])
        assert.equal(read.records.length, 3)
        const mark = read.records.map(readMark).find((entry) => entry !== undefined)
        assert.deepEqual(mark, { deltaUs: 1_500, label: 'stream:error' })
    })

    it('reads a mark as nothing when it is not one', () => {
        assert.equal(readMark({ kind: RecordKind.Envelope, payload: new Uint8Array() }), undefined)
        assert.equal(
            readMark({ kind: RecordKind.Mark, payload: Uint8Array.from([1, 2]) }),
            undefined,
            'too short for the delta'
        )
    })
})

describe('an interrupted recording', () => {
    it('is readable, and says so', () => {
        // The artifact a crash recorder exists to produce. Rejecting it would
        // fail the format's most important use while passing every test
        // written against well-formed files.
        const whole = encodeRecording(META, [envelope([1]), envelope([2])])
        const frozen = whole.subarray(0, whole.length - 13)

        const read = decodeRecording(frozen)
        assert.deepEqual(read.integrity, { kind: 'truncated', found: 2, dangling: 0 })
        assert.deepEqual(read.envelopes, [Uint8Array.from([1]), Uint8Array.from([2])])
    })

    it('drops a record cut in half and keeps the rest', () => {
        const whole = encodeRecording(META, [envelope([1]), envelope([2, 3, 4])])
        const read = decodeRecording(whole.subarray(0, whole.length - 15))

        assert.equal(read.integrity.kind, 'truncated')
        assert.deepEqual(read.envelopes, [Uint8Array.from([1])])
    })

    it('is readable at every cut past the header', () => {
        // Not "does not throw" — a ring buffer can be frozen at any offset, so
        // every one has to produce a usable answer.
        const whole = encodeRecording(META, [envelope([1]), envelope([2]), envelope([3])])
        const metaLen = new DataView(whole.buffer, whole.byteOffset).getUint32(6, true)

        for (let cut = RECORDING_HEADER_LEN + metaLen; cut < whole.length; cut += 1) {
            const read = decodeRecording(whole.subarray(0, cut))
            assert.notEqual(read.integrity.kind, 'complete', `cut ${cut}`)
            for (const found of read.envelopes) {
                assert.ok(found.length === 1, `cut ${cut} produced a partial envelope`)
            }
        }
        assert.equal(decodeRecording(whole).integrity.kind, 'complete')
    })
})

describe('a damaged recording', () => {
    it('reports a flipped byte without becoming unreadable', () => {
        const bytes = encodeRecording(META, [envelope([1, 2, 3])])
        const at = bytes.length - 15
        bytes[at] = (bytes[at] ?? 0) ^ 0xff

        const read = decodeRecording(bytes)
        assert.equal(read.integrity.kind, 'damaged')
        assert.equal(
            read.integrity.kind === 'damaged' ? read.integrity.checksumOk : true,
            false
        )
        assert.equal(read.envelopes.length, 1, 'damaged is not unreadable')
    })

    it('catches a trailer that miscounts even though the checksum holds', () => {
        // The checksum covers everything before the trailer, so it cannot cover
        // the count the trailer carries. The count is its own witness.
        const bytes = encodeRecording(META, [envelope([1]), envelope([2])])
        bytes[bytes.length - 8] = 9

        const read = decodeRecording(bytes)
        assert.deepEqual(read.integrity, {
            kind: 'damaged',
            claimed: 9,
            found: 2,
            checksumOk: true
        })
    })
})

describe('extensibility', () => {
    it('preserves an unknown ancillary tag and charges nothing for it', () => {
        const bytes = encodeRecording(
            { ...META, extra: [{ tag: 0x0042, value: new TextEncoder().encode('later') }] },
            []
        )
        const read = decodeRecording(bytes)
        assert.equal(read.unknownCriticalTags, 0)
        assert.deepEqual(
            read.meta.find((entry) => entry.tag === 0x0042)?.value,
            new TextEncoder().encode('later')
        )
    })

    it('counts an unknown critical tag so comparison can refuse', () => {
        // The point of the bit: skipping a field that decides comparability
        // would let a reader produce a confident wrong verdict.
        const bytes = encodeRecording(
            {
                ...META,
                extra: [{ tag: CRITICAL_BIT | 0x0042, value: Uint8Array.from([1]) }]
            },
            []
        )
        const read = decodeRecording(bytes)
        assert.equal(read.unknownCriticalTags, 1)
        assert.equal(read.integrity.kind, 'complete', 'still readable')
    })

    it('walks past an unknown record kind without losing what follows', () => {
        const bytes = encodeRecording(META, [
            envelope([1]),
            { kind: 0x7e, payload: Uint8Array.from([9, 9]) },
            envelope([2])
        ])
        const read = decodeRecording(bytes)
        assert.equal(read.skippedRecords, 1)
        assert.deepEqual(read.envelopes, [Uint8Array.from([1]), Uint8Array.from([2])])
    })
})

describe('refusals', () => {
    it('refuses a buffer that is not a recording', () => {
        const bytes = encodeRecording(META, [])
        bytes[0] = 0x58
        assert.throws(() => decodeRecording(bytes), RecordingReadError)
    })

    it('refuses a header too short to hold', () => {
        assert.throws(() => decodeRecording(new Uint8Array(3)), RecordingReadError)
    })

    it('refuses a newer container version rather than guessing at it', () => {
        const bytes = encodeRecording(META, [])
        new DataView(bytes.buffer, bytes.byteOffset).setUint16(4, 99, true)
        assert.throws(() => decodeRecording(bytes), /version 99/)
    })

    it('refuses a metadata block that runs past the buffer', () => {
        const bytes = encodeRecording(META, [])
        new DataView(bytes.buffer, bytes.byteOffset).setUint32(6, 0xffff, true)
        assert.throws(() => decodeRecording(bytes), /metadata claims/)
    })

    it('refuses a metadata entry that runs past its block', () => {
        const w = new Uint8Array(RECORDING_HEADER_LEN + 6)
        w.set(MAGIC, 0)
        const view = new DataView(w.buffer)
        view.setUint16(4, CONTAINER_VERSION, true)
        view.setUint32(6, 6, true)
        view.setUint16(RECORDING_HEADER_LEN, RecordingTag.Note, true)
        view.setUint32(RECORDING_HEADER_LEN + 2, 99, true)
        assert.throws(() => decodeRecording(w), /runs past the block/)
    })

    it('refuses a repeated tag at the writer', () => {
        assert.throws(
            () =>
                encodeRecording(
                    { note: 'first', extra: [{ tag: RecordingTag.Note, value: new Uint8Array() }] },
                    []
                ),
            RecordingWriteError
        )
    })

    it('refuses a string that does not fit its prefix', () => {
        assert.throws(
            () => encodeRecording({ dictionary: 'x'.repeat(0x10000) }, []),
            /does not fit in 16 bits/
        )
    })

    it('refuses a record with no payload at all', () => {
        assert.throws(
            () => encodeRecording(META, [{ kind: RecordKind.Mark } as never]),
            RecordingWriteError
        )
    })

    it('refuses more capability identifiers than the count can describe', () => {
        assert.throws(
            () =>
                encodeRecording(
                    {
                        adapter: {
                            id: 'x',
                            version: '1',
                            engineVersion: '1',
                            contractVersion: 1,
                            capabilities: new Array<string>(0x10000).fill('c')
                        }
                    },
                    []
                ),
            /too many capability identifiers/
        )
    })
})
