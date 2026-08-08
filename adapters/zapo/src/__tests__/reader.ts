/**
 * A decoder for the envelopes this package writes.
 *
 * Test-only, and deliberately a separate implementation from the encoder: a
 * round trip through the same code proves the code is self-consistent, which is
 * not the same as proving it is right. The authority is the Rust decoder; this
 * one exists so a mistake surfaces on this side too, at the line that made it.
 */

import { HEADER_LEN } from '@oxidezap/wa-wire-ts'

export interface ReadPlaintext {
    readonly path: number[]
    readonly status: number
    readonly payload: Uint8Array
}

export interface ReadEnvelope {
    readonly version: number
    readonly flags: number
    readonly frame: Uint8Array
    readonly plaintexts: ReadPlaintext[]
}

export class EnvelopeReader {
    private at = 0

    public constructor(private readonly bytes: Uint8Array) {}

    public read(): ReadEnvelope {
        const view = new DataView(
            this.bytes.buffer,
            this.bytes.byteOffset,
            this.bytes.byteLength,
        )
        const version = view.getUint16(this.at, true)
        this.at += 2
        const flags = view.getUint16(this.at, true)
        this.at += 2
        const frameLen = view.getUint32(this.at, true)
        this.at += 4
        const frame = this.bytes.subarray(this.at, this.at + frameLen)
        this.at += frameLen

        const count = view.getUint16(this.at, true)
        this.at += 2

        const plaintexts: ReadPlaintext[] = []
        for (let i = 0; i < count; i += 1) {
            const components = this.bytes[this.at] ?? 0
            this.at += 1
            const path: number[] = []
            for (let c = 0; c < components; c += 1) {
                path.push(view.getUint16(this.at, true))
                this.at += 2
            }
            const status = this.bytes[this.at] ?? 0
            this.at += 1
            const payloadLen = view.getUint32(this.at, true)
            this.at += 4
            plaintexts.push({
                path,
                status,
                payload: this.bytes.subarray(this.at, this.at + payloadLen),
            })
            this.at += payloadLen
        }

        if (this.at !== this.bytes.length) {
            throw new Error(
                `${this.bytes.length - this.at} trailing byte(s) after the envelope`,
            )
        }
        if (HEADER_LEN !== 8) throw new Error('header length changed')

        return { version, flags, frame, plaintexts }
    }
}
