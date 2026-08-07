import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, it } from 'node:test'

import { PlaintextStatus } from '../envelope.js'
import { EnvelopeReader } from './reader.js'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')
const FIXTURES = join(ROOT, 'fixtures')

/**
 * The fixtures are the contract's cross-language check: the Rust side decodes
 * these exact bytes. If regenerating them changes anything, one of the two
 * encoders moved and the other does not know it yet.
 */
describe('cross-language fixtures', () => {
    it('regenerate byte-identically', () => {
        const before = snapshot()
        execFileSync('npx', ['tsx', 'scripts/emit-fixtures.ts'], {
            cwd: ROOT,
            stdio: 'pipe',
        })
        assert.deepEqual(snapshot(), before, 'fixtures are stale — commit the regenerated ones')
    })

    it('are all readable by this side too', () => {
        // Encoding and never reading back would let a mistake sit until the
        // Rust side happened to run.
        for (const [name, bytes] of Object.entries(snapshot())) {
            const envelope = new EnvelopeReader(bytes).read()
            assert.equal(envelope.version, 1, name)
            assert.ok(envelope.frame.length >= 0, name)
        }
    })

    it('cover the shapes that matter', () => {
        const names = Object.keys(snapshot())
        for (const expected of [
            'receipt',
            'message-with-enc',
            'outbound-verbatim',
            'multi-device-with-plaintexts',
            'root-path-plaintext',
            'empty-frame',
        ]) {
            assert.ok(names.includes(expected), `missing fixture: ${expected}`)
        }
    })

    it('address one plaintext per enc in the multi-device case', () => {
        const bytes = readFileSync(join(FIXTURES, 'multi-device-with-plaintexts.bin'))
        const envelope = new EnvelopeReader(new Uint8Array(bytes)).read()
        assert.equal(envelope.plaintexts.length, 4)
        assert.deepEqual(
            envelope.plaintexts.map((p) => p.path),
            [[0], [1], [2], [3]],
        )
        // Every status, so the fixture pins all four across both encoders.
        assert.deepEqual(
            envelope.plaintexts.map((p) => p.status),
            [
                PlaintextStatus.Ok,
                PlaintextStatus.DecryptFailed,
                PlaintextStatus.Unsupported,
                PlaintextStatus.Unobserved,
            ],
        )
    })

    it('put the root path on the stanza itself', () => {
        const bytes = readFileSync(join(FIXTURES, 'root-path-plaintext.bin'))
        const envelope = new EnvelopeReader(new Uint8Array(bytes)).read()
        assert.deepEqual(envelope.plaintexts[0]?.path, [])
    })
})

function snapshot(): Record<string, Uint8Array> {
    const out: Record<string, Uint8Array> = {}
    for (const name of readdirSync(FIXTURES).sort()) {
        if (!name.endsWith('.bin')) continue
        out[name.replace(/\.bin$/, '')] = new Uint8Array(readFileSync(join(FIXTURES, name)))
    }
    return out
}
