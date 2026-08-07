import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import * as api from '../index.js'

/**
 * The package's public surface.
 *
 * A re-export that stops existing is a break for every consumer, and nothing
 * inside the package would notice — every module imports its neighbours
 * directly.
 */
describe('public surface', () => {
    it('exports what an adapter consumer needs', () => {
        for (const name of [
            'waWire',
            'Mode',
            'INFO',
            'toStanza',
            'toEnvelope',
            'forward',
            'supports',
            'encodeEnvelope',
            'encodedLength',
            'fitsPrefix',
            'Direction',
            'FrameOrigin',
            'PlaintextStatus',
            'EncodeError',
            'Capability',
            'has',
            'missing',
            'CONTRACT_VERSION',
            'HEADER_LEN',
        ]) {
            assert.ok(name in api, `missing export: ${name}`)
        }
    })

    it('exports usable values, not just names', () => {
        assert.equal(api.CONTRACT_VERSION, 1)
        assert.equal(api.HEADER_LEN, 8)
        assert.equal(api.Mode.Tap, 'tap')
        assert.equal(api.Mode.Takeover, 'takeover')
        assert.equal(api.INFO.id, 'zapo')
        assert.equal(typeof api.waWire, 'function')
        assert.ok(api.has(api.Capability.L0InboundTap))
    })
})
