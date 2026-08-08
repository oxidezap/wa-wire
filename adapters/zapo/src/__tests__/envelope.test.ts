import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
    CONTRACT_VERSION,
    Direction,
    EncodeError,
    FrameOrigin,
    HEADER_LEN,
    PlaintextStatus,
    encodeEnvelope,
    encodedLength,
    fitsPrefix,
    type Stanza,
} from '../envelope.js'

function inbound(frame: Uint8Array, plaintexts?: Stanza['plaintexts']): Stanza {
    return {
        direction: Direction.Inbound,
        frameOrigin: FrameOrigin.Original,
        frame,
        ...(plaintexts ? { plaintexts } : {}),
    }
}

describe('envelope layout', () => {
    it('writes exactly the bytes the contract expects', () => {
        // Pinned against the Rust decoder's own layout test. If either side
        // moves, one of the two fails — which is the point of writing it twice.
        const stanza: Stanza = {
            direction: Direction.Outbound,
            frameOrigin: FrameOrigin.Original,
            frame: Uint8Array.from([0x01, 0x02]),
            plaintexts: [
                {
                    // `Ok` because the layout needs a non-empty payload to
                    // show, and only `Ok` may carry one.
                    path: [258],
                    status: PlaintextStatus.Ok,
                    payload: Uint8Array.from([0x61, 0x62]),
                },
            ],
        }

        assert.deepEqual(
            Array.from(encodeEnvelope(stanza)),
            [
                0x01, 0x00, // version = 1
                0x01, 0x00, // flags = outbound
                0x02, 0x00, 0x00, 0x00, // frame_len = 2
                0x01, 0x02, // frame
                0x01, 0x00, // pt_count = 1
                0x01, // path_len = 1 component
                0x02, 0x01, // path[0] = 258 little-endian
                0x00, // status = Ok
                0x02, 0x00, 0x00, 0x00, // payload_len = 2
                0x61, 0x62, // payload
            ],
        )
        assert.equal(HEADER_LEN, 8)
        assert.equal(CONTRACT_VERSION, 1)
    })

    it('sizes a stanza before writing it', () => {
        const frame = Uint8Array.from([1, 2, 3])
        const stanza = inbound(frame)
        assert.equal(encodedLength(stanza), HEADER_LEN + 3 + 2)
        assert.equal(encodeEnvelope(stanza).length, encodedLength(stanza))
    })

    it('encodes every flag combination', () => {
        const cases: ReadonlyArray<readonly [Direction, FrameOrigin, number]> = [
            [Direction.Inbound, FrameOrigin.Original, 0b00],
            [Direction.Outbound, FrameOrigin.Original, 0b01],
            [Direction.Inbound, FrameOrigin.ReEncoded, 0b10],
            [Direction.Outbound, FrameOrigin.ReEncoded, 0b11],
        ]
        for (const [direction, frameOrigin, expected] of cases) {
            const bytes = encodeEnvelope({
                direction,
                frameOrigin,
                frame: new Uint8Array(),
            })
            assert.equal(bytes[2], expected, `${direction}/${frameOrigin}`)
            assert.equal(bytes[3], 0, 'the high byte stays clear')
        }
    })

    it('encodes an empty frame and no plaintexts', () => {
        const bytes = encodeEnvelope(inbound(new Uint8Array()))
        assert.equal(bytes.length, HEADER_LEN + 2)
        assert.deepEqual(Array.from(bytes.subarray(4, 8)), [0, 0, 0, 0])
    })

    it('encodes several plaintexts in order', () => {
        const stanza = inbound(Uint8Array.from([0xaa]), [
            { path: [0], status: PlaintextStatus.Ok, payload: Uint8Array.from([1]) },
            { path: [1, 2], status: PlaintextStatus.Unsupported, payload: new Uint8Array() },
        ])
        const bytes = encodeEnvelope(stanza)
        assert.equal(bytes.length, encodedLength(stanza))

        // pt_count sits right after the frame.
        const countAt = HEADER_LEN + 1
        assert.equal(bytes[countAt], 2)
        assert.equal(bytes[countAt + 1], 0)
    })

    it('encodes an empty path as the root', () => {
        const stanza = inbound(Uint8Array.from([1]), [
            { path: [], status: PlaintextStatus.Ok, payload: Uint8Array.from([9]) },
        ])
        const bytes = encodeEnvelope(stanza)
        const pathLenAt = HEADER_LEN + 1 + 2
        assert.equal(bytes[pathLenAt], 0, 'no components')
    })

    it('writes path components little-endian', () => {
        const stanza = inbound(new Uint8Array(), [
            { path: [1, 0x0102], status: PlaintextStatus.Ok, payload: new Uint8Array() },
        ])
        const bytes = encodeEnvelope(stanza)
        const pathAt = HEADER_LEN + 2 + 1
        assert.deepEqual(Array.from(bytes.subarray(pathAt, pathAt + 4)), [
            0x01, 0x00, 0x02, 0x01,
        ])
    })

    it('handles a frame larger than any real stanza', () => {
        // Real captures peak around 433 KB.
        const frame = new Uint8Array(500_000).fill(0xab)
        const bytes = encodeEnvelope(inbound(frame))
        assert.equal(bytes.length, HEADER_LEN + frame.length + 2)
        assert.deepEqual(
            Array.from(bytes.subarray(HEADER_LEN, HEADER_LEN + 4)),
            [0xab, 0xab, 0xab, 0xab],
        )
    })
})

describe('envelope limits', () => {
    it('rejects a path deeper than the prefix can describe', () => {
        const stanza = inbound(new Uint8Array(), [
            {
                path: new Array<number>(256).fill(0),
                status: PlaintextStatus.Ok,
                payload: new Uint8Array(),
            },
        ])
        assert.throws(() => encodedLength(stanza), EncodeError)
        assert.throws(() => encodeEnvelope(stanza), /path of 256/)
    })

    it('accepts a path at the prefix limit', () => {
        const stanza = inbound(new Uint8Array(), [
            {
                path: new Array<number>(255).fill(1),
                status: PlaintextStatus.Ok,
                payload: new Uint8Array(),
            },
        ])
        assert.equal(encodeEnvelope(stanza).length, encodedLength(stanza))
    })

    it('rejects a path component that does not fit', () => {
        const stanza = inbound(new Uint8Array(), [
            { path: [0x1_0000], status: PlaintextStatus.Ok, payload: new Uint8Array() },
        ])
        assert.throws(() => encodeEnvelope(stanza), /path component/)
    })

    it('refuses a payload under a status that defines none', () => {
        // The Rust decoder rejects one of these. Writing one here would mean
        // the two halves of the boundary disagree about what is valid, which
        // is the thing writing the format twice is supposed to catch.
        for (const status of [
            PlaintextStatus.DecryptFailed,
            PlaintextStatus.Unsupported,
            PlaintextStatus.Unobserved,
        ]) {
            const stanza = inbound(new Uint8Array(), [
                { path: [0], status, payload: Uint8Array.from([1, 2]) },
            ])
            assert.throws(() => encodeEnvelope(stanza), /only ok may carry any/)
        }
        assert.doesNotThrow(() =>
            encodeEnvelope(
                inbound(new Uint8Array(), [
                    { path: [0], status: PlaintextStatus.Ok, payload: Uint8Array.from([1, 2]) },
                ]),
            ),
        )
    })

    it('rejects more plaintexts than the count can describe', () => {
        const one = {
            path: [0],
            status: PlaintextStatus.Ok,
            payload: new Uint8Array(),
        }
        const stanza = inbound(new Uint8Array(), new Array(0x1_0000).fill(one))
        assert.throws(() => encodedLength(stanza), /plaintext count/)
    })

    it('names which field it could not represent', () => {
        const error = new EncodeError('frame of 5 does not fit')
        assert.equal(error.name, 'EncodeError')
        assert.ok(error instanceof Error)
        assert.match(error.message, /frame/)
    })
})

describe('length prefixes', () => {
    it('accept what they can carry and reject what they cannot', () => {
        // The frame and payload limits need a 4 GiB buffer to reach through
        // `encodeEnvelope`, so the narrowing itself is checked here.
        const cases: ReadonlyArray<readonly [8 | 16 | 32, number]> = [
            [8, 0xff],
            [16, 0xffff],
            [32, 0xffff_ffff],
        ]
        for (const [bits, max] of cases) {
            assert.ok(fitsPrefix(0, bits), `0 fits ${bits} bits`)
            assert.ok(fitsPrefix(max, bits), `${max} fits ${bits} bits`)
            assert.ok(!fitsPrefix(max + 1, bits), `${max + 1} does not`)
        }
    })

    it('reject values that are not counts at all', () => {
        assert.ok(!fitsPrefix(-1, 8))
        assert.ok(!fitsPrefix(1.5, 16))
        assert.ok(!fitsPrefix(Number.NaN, 32))
        assert.ok(!fitsPrefix(Number.POSITIVE_INFINITY, 32))
    })
})
