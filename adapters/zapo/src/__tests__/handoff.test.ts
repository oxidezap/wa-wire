import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { Capability, declares } from '@oxidezap/wa-wire-ts'
import { DETACHING_INFO, INFO } from '../capability.js'
import { DetachError, createDetacher } from '../handoff.js'

/**
 * The engine's two ways to stop, and the account they act on.
 *
 * `disconnect()` closes the transport and keeps the credentials — `connect()`
 * again resumes the same session. `logout()` unpairs the device. A detach must
 * be the first and must not be able to become the second.
 */
function engine() {
    const state = { disconnected: 0, paired: true }
    return {
        state,
        disconnect: async () => {
            state.disconnected += 1
        },
        // Deliberately not something `createDetacher` is given: the detacher
        // takes the one call it uses, so this is not even in scope for it.
        logout: async () => {
            state.paired = false
        },
    }
}

describe('releasing a session', () => {
    it('closes the transport and leaves the device paired', async () => {
        const client = engine()
        const detacher = createDetacher(client)

        await detacher.detach()

        assert.equal(client.state.disconnected, 1)
        assert.ok(client.state.paired, 'a detach that unpaired would be a logout')
    })

    it('is idempotent, because a host may have crashed mid-handoff', async () => {
        const client = engine()
        const detacher = createDetacher(client)

        await detacher.detach()
        await detacher.detach()

        assert.equal(client.state.disconnected, 2)
        assert.ok(client.state.paired)
    })

    it('reports a failure rather than swallowing it', async () => {
        // A host must not read a failure as permission to attach elsewhere: the
        // old connection may still be live, and that is the two-writer case.
        const cause = new Error('the socket never closed')
        const detacher = createDetacher({
            disconnect: async () => {
                throw cause
            },
        })

        await assert.rejects(() => detacher.detach(), (error: unknown) => {
            assert.ok(error instanceof DetachError)
            assert.equal(error.cause, cause)
            return true
        })
    })

    it('is claimed only by the declaration that holds a client', async () => {
        // A plugin instance has the filter's view and no `WaClient`, so the tap
        // genuinely cannot release anything.
        assert.ok(!declares(INFO, Capability.Detach))
        assert.ok(declares(DETACHING_INFO, Capability.Detach))
        assert.deepEqual(
            DETACHING_INFO.capabilities.filter((one) => one !== Capability.Detach),
            INFO.capabilities,
            'detaching is one addition, not a different adapter'
        )
    })
})
