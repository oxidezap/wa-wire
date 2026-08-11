import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { Capability, declares } from '@oxidezap/wa-wire-ts'
import { DETACHING_INFO, INFO } from '../capability.js'
import { DetachError, createDetacher } from '../handoff.js'

/**
 * The socket's two ways to stop, and what each leaves behind.
 *
 * `end(error)` closes and hands the error to the consumer as
 * `lastDisconnect.error`; `logout()` sends `remove-companion-device` and then
 * ends with `DisconnectReason.loggedOut`, which is the status code a consumer
 * reads to decide whether to wipe its auth state.
 */
const LOGGED_OUT = 401

function socket() {
    const state = { ended: 0, endedWith: undefined as Error | undefined, paired: true }
    return {
        state,
        end: async (error: Error | undefined) => {
            state.ended += 1
            state.endedWith = error
        },
        // Not something `createDetacher` is given — it takes `end` alone.
        logout: async () => {
            state.paired = false
            const error = new Error('Intentional Logout') as Error & { statusCode: number }
            error.statusCode = LOGGED_OUT
            state.ended += 1
            state.endedWith = error
        },
    }
}

describe('releasing a session', () => {
    it('ends the socket and leaves the device paired', async () => {
        const sock = socket()

        await createDetacher(sock).detach()

        assert.equal(sock.state.ended, 1)
        assert.ok(sock.state.paired, 'a detach that unpaired would be a logout')
    })

    it('ends with no error, so a consumer does not read a handoff as a failure', async () => {
        // Every Baileys consumer branches on `lastDisconnect.error.statusCode`
        // to decide whether to relaunch. An error here would make a deliberate
        // move look like a connection that broke.
        const sock = socket()

        await createDetacher(sock).detach()

        assert.equal(sock.state.endedWith, undefined)
    })

    it('is idempotent, because a host may have crashed mid-handoff', async () => {
        const sock = socket()
        const detacher = createDetacher(sock)

        await detacher.detach()
        await detacher.detach()

        assert.equal(sock.state.ended, 2)
        assert.ok(sock.state.paired)
    })

    it('reports a failure rather than swallowing it', async () => {
        // A host must not read a failure as permission to attach elsewhere: the
        // old connection may still be live, and that is the two-writer case.
        const cause = new Error('the socket never closed')
        const detacher = createDetacher({
            end: () => {
                throw cause
            },
        })

        await assert.rejects(() => detacher.detach(), (error: unknown) => {
            assert.ok(error instanceof DetachError)
            assert.equal(error.cause, cause)
            return true
        })
    })

    it('is claimed only by the declaration that holds a socket', async () => {
        assert.ok(!declares(INFO, Capability.Detach))
        assert.ok(declares(DETACHING_INFO, Capability.Detach))
        assert.deepEqual(
            DETACHING_INFO.capabilities.filter((one) => one !== Capability.Detach),
            INFO.capabilities,
            'detaching is one addition, not a different adapter'
        )
    })
})
