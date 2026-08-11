import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { Barrier, Gate, Quiet } from '../quiesce.js'

describe('holding commands while a session moves', () => {
    it('passes until it is told not to', () => {
        const gate = new Gate<number>(4)

        assert.ok(!gate.isQuiesced)
        assert.deepEqual(gate.offer(1), { kind: 'pass', command: 1 })

        gate.quiesce()
        assert.deepEqual(gate.offer(2), { kind: 'held' })
        assert.equal(gate.backlog, 1)
    })

    it('gives the backlog back in the order it went in', async () => {
        // Releasing out of order reorders an application's sends — a bug it
        // cannot see and did not cause.
        const gate = new Gate<number>(8)
        gate.quiesce()
        for (const command of [1, 2, 3, 4, 5]) gate.offer(command)

        const released: number[] = []
        const count = await gate.resume((command) => {
            released.push(command)
        })

        assert.equal(count, 5)
        assert.deepEqual(released, [1, 2, 3, 4, 5])
        assert.equal(gate.backlog, 0)
        assert.ok(!gate.isQuiesced)
    })

    it('hands a command back when full rather than dropping it', () => {
        // Dropping would make a full backlog look like a successful hold, and
        // the application would never learn its command went nowhere.
        const gate = new Gate<number>(2)
        gate.quiesce()
        gate.offer(1)
        gate.offer(2)

        assert.ok(gate.isFull)
        assert.deepEqual(gate.offer(3), { kind: 'full', command: 3 })
        assert.equal(gate.backlog, 2, 'nothing already held was displaced')
    })

    it('opens before the backlog drains', async () => {
        // A command produced by releasing another must be sent, not appended to
        // a queue being emptied — it would wait for a resume that already
        // happened.
        const gate = new Gate<number>(4)
        gate.quiesce()
        gate.offer(1)

        const sent: number[] = []
        await gate.resume((command) => {
            sent.push(command)
            assert.deepEqual(gate.offer(9), { kind: 'pass', command: 9 })
        })
        assert.deepEqual(sent, [1])
    })

    it('discards the backlog when a handoff is abandoned', () => {
        const gate = new Gate<number>(4)
        gate.quiesce()
        gate.offer(1)
        gate.offer(2)

        assert.equal(gate.abandon(), 2)
        assert.equal(gate.backlog, 0)
        assert.deepEqual(gate.offer(3), { kind: 'pass', command: 3 })
    })

    it('refuses a capacity that cannot hold anything', () => {
        // A zero-capacity gate would report every command as full, which reads
        // as a stalled handoff rather than as a misconfigured one.
        assert.throws(() => new Gate<number>(0), RangeError)
        assert.throws(() => new Gate<number>(1.5), RangeError)
    })
})

describe('waiting for the engine to go quiet', () => {
    it('says nothing until something reports', async () => {
        const barrier = new Barrier()
        assert.equal(barrier.state, Quiet.Unconfirmed)

        barrier.drained()
        assert.equal(barrier.state, Quiet.Confirmed)
        assert.equal(await barrier.wait(1000), Quiet.Confirmed)
    })

    it('resolves rather than rejects when nothing reports', async () => {
        // A barrier that could not be confirmed is an outcome a host acts on,
        // not an error it recovers from — three of the four engines cannot
        // report a drain at all, and their handoffs still have to work.
        const barrier = new Barrier()

        assert.equal(await barrier.wait(10), Quiet.Unconfirmed)
        assert.equal(barrier.state, Quiet.Unconfirmed)
    })

    it('wakes everyone waiting, once', async () => {
        const barrier = new Barrier()
        const waits = [barrier.wait(1000), barrier.wait(1000), barrier.wait(1000)]

        barrier.drained()
        barrier.drained()

        assert.deepEqual(await Promise.all(waits), [
            Quiet.Confirmed,
            Quiet.Confirmed,
            Quiet.Confirmed,
        ])
    })

    it('reads as words, because a report is read by a person', () => {
        // "drained" against "not known to have drained" is the difference
        // between two handoffs that otherwise look alike in a log.
        assert.equal(String(Quiet.Confirmed), 'drained')
        assert.equal(String(Quiet.Unconfirmed), 'not known to have drained')
    })
})
