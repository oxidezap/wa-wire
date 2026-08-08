package wawire

import (
	"sync"

	"github.com/oxidezap/wa-wire/adapters/hypermeow/wire"
	waBinary "go.mau.fi/whatsmeow/binary"
)

// DefaultLookahead is how many later stanzas a pending message tolerates
// before it is emitted with whatever it has.
//
// Sized for the widest real fan-out rather than tuned: a message's payloads
// all arrive within its own processing, so a handful of intervening stanzas
// already means they are not coming.
// DefaultLookahead must exceed the engine's handler queue, and does.
//
// The two hooks sit on opposite sides of that queue: the raw-node hook runs on
// the receive goroutine, and a plaintext arrives from `handlerQueueLoop`, which
// buffers 256 nodes. So a message can legitimately lag by the whole queue
// before its payload appears, and a lookahead under that would give up on
// traffic that was about to arrive — recording `Unobserved` for a stanza that
// decrypted perfectly well.
//
// Twice the queue, so the margin does not depend on reading the engine's
// constant exactly right. The Rust and TypeScript adapters use 64 because
// their engines hand both halves over on one path and no queue sits between.
const DefaultLookahead = 512

// Joiner holds a stanza until the plaintexts decrypted out of it catch up.
//
// The two arrive separately and cannot be made to arrive together. The raw-node
// hook runs inside the Noise frame callback, necessarily before Signal has run;
// a plaintext exists only afterwards.
//
// # Knowing when to stop waiting
//
// The stanza says how many `<enc>` children it has, so the common case — every
// one decrypts — closes by counting, with no clock: the last payload completes
// the table and the envelope goes out at once.
//
// What has no signal is an `<enc>` that will never produce a payload. So
// something has to give up, and giving up is measured in **stanzas rather than
// milliseconds**: the receive path is ordered, so a stanza whose payloads have
// not arrived after DefaultLookahead later ones is a stanza whose payloads are
// not coming. A count is also the same on every machine, which a duration is
// not, and this output is compared against other engines'.
//
// The Rust and TypeScript adapters reach the same conclusions from the same
// constraints (wa-wire DESIGN D-052, D-053, D-055). The three are deliberately
// alike: an adapter that decided differently would produce recordings that
// differ for reasons the engines are not responsible for.
//
// # What this adapter does not have to do
//
// Address a payload to its node. The other two engines report which `<enc>` of
// a stanza decrypted, counting `<enc>` nodes, and an adapter has to work out
// which child that is — ambiguous the moment a stanza carries anything else,
// and unresolvable for a fan-out `<message>`, where the copies for this device
// are numbered apart from the direct children.
//
// hypermeow reports the child index directly, so the path is that index and
// nothing is inferred. The hook was written against this need.
type Joiner struct {
	mu sync.Mutex
	// Every stanza seen, in the order the receive goroutine saw them.
	//
	// A queue rather than a set of pending ones, because emitting an unheld
	// stanza the moment it arrives reorders it ahead of a held one that came
	// first — and a recording compared position by position would call that a
	// divergence. The front of the queue drains whenever the front is ready,
	// so what leaves is what arrived, in that order.
	queue   []*slot
	byID    map[string]*slot
	pending int

	lookahead int

	// Held across the sink call and nothing else.
	//
	// The two hooks run on different goroutines — the raw-node one on the
	// receive path, the plaintext one on the engine's handler queue — so
	// without this two deliveries can be in `Accept` at once. A sink is a
	// consumer's code and is entitled not to be reentrant.
	//
	// Separate from `mu` so a slow sink stalls deliveries rather than the
	// engine's receive path.
	emitMu sync.Mutex
	sink   Sink
}

// slot is one stanza's place in the queue.
type slot struct {
	// id is the message id, empty for a stanza that is not held.
	id    string
	frame []byte
	// The child indices of this stanza's `<enc>` nodes, in document order.
	//
	// The indices themselves rather than a count: an `<enc>` is not
	// necessarily the first child, and a stanza whose `<enc>` nodes sit at
	// positions two and three has nothing at zero and one. Keeping the real
	// positions is also what lets an `<enc>` that produced nothing be reported
	// at the path it actually occupies.
	encIndices []int
	plaintexts map[int]wire.Plaintext
	// How many stanzas have arrived since this one.
	age int
	// Whether this slot is finished and only waiting for its turn.
	done bool
}

func (s *slot) ready() bool {
	return s.done || len(s.plaintexts) >= len(s.encIndices)
}

// Sink receives envelopes as they complete.
//
// Called with the joiner's lock released, so a slow sink does not stall the
// engine's receive path — but the calls are ordered, and a sink that blocks
// still delays the envelopes behind it.
type Sink interface {
	Accept(envelope wire.Envelope)
}

// SinkFunc adapts a function to a Sink.
type SinkFunc func(envelope wire.Envelope)

// Accept calls the function.
func (f SinkFunc) Accept(envelope wire.Envelope) { f(envelope) }

// NewJoiner returns a joiner emitting to sink.
func NewJoiner(sink Sink) *Joiner {
	return &Joiner{
		byID:      make(map[string]*slot),
		lookahead: DefaultLookahead,
		sink:      sink,
	}
}

// WithLookahead sets how many later stanzas a pending one tolerates. For tests
// and for a host that knows its own traffic; the default suits this engine's
// handler queue.
func (j *Joiner) WithLookahead(stanzas int) *Joiner {
	j.lookahead = stanzas
	return j
}

// AcceptFrame takes a decoded stanza and its bytes.
//
// A stanza with no `<enc>` children is finished on arrival; one with them waits
// for its payloads. Either way it takes its place in the queue, and the queue
// drains in order — so an ack behind a held message stays behind it, which is
// where the wire put it.
//
// The frame is copied. The engine documents its buffer as valid only for the
// duration of the call — an uncompressed frame is a window into the transport's
// own read buffer — and queueing means outliving the call by construction.
func (j *Joiner) AcceptFrame(node *waBinary.Node, frame []byte) {
	encIndices := encChildIndices(node)
	id, _ := node.Attrs["id"].(string)
	// A stanza carrying `<enc>` children and no id is not one this can hold: it
	// would never be matched to its payloads. Finished on arrival, as L0-wire.
	holdable := len(encIndices) > 0 && id != ""

	j.mu.Lock()
	// Every stanza ages the ones waiting, including the ones finished on
	// arrival: the lookahead counts *later stanzas*, and an ack going past is
	// one. A receive path carrying nothing but acks would otherwise hold a
	// message for ever.
	j.ageLocked()

	entry := &slot{frame: cloneBytes(frame)}
	if holdable {
		entry.id = id
		entry.encIndices = encIndices
		entry.plaintexts = make(map[int]wire.Plaintext, len(encIndices))
		// A repeated id — a retry arriving before the first one finished —
		// leaves the earlier stanza in the queue and unreachable by id rather
		// than replacing it. It was a real stanza and this adapter promises to
		// report every one; what it loses is the chance of a payload, which
		// the earlier one gives up as `Unobserved`.
		if previous, held := j.byID[id]; held {
			previous.done = true
		}
		j.byID[id] = entry
		j.pending++
	} else {
		entry.done = true
	}
	j.queue = append(j.queue, entry)
	ready := j.drainLocked()
	j.mu.Unlock()

	j.emitAll(ready)
}

// AcceptPlaintext takes one decrypted payload and completes its stanza if this
// was the last one outstanding.
func (j *Joiner) AcceptPlaintext(messageID string, childIndex int, plaintext []byte) {
	j.mu.Lock()
	entry, held := j.byID[messageID]
	if !held || entry.done {
		// No stanza to attach it to: one already given up on, or a frame never
		// seen. Inventing somewhere to put it would be worse than losing it.
		j.mu.Unlock()
		return
	}
	if !entry.expects(childIndex) {
		// A payload for a child this stanza has no `<enc>` at. The engine and
		// the frame disagree about the stanza, and counting it would close the
		// stanza early and drop the payload that was actually coming.
		j.mu.Unlock()
		return
	}
	entry.plaintexts[childIndex] = wire.Plaintext{
		Path:    []uint16{uint16(childIndex)},
		Status:  wire.StatusOk,
		Payload: cloneBytes(plaintext),
	}
	ready := j.drainLocked()
	j.mu.Unlock()

	j.emitAll(ready)
}

// Flush emits every stanza still queued, with whatever it has.
//
// For shutdown: a frame still waiting for a payload that will now never arrive
// is better emitted unobserved than lost, since the stanza was real either way.
func (j *Joiner) Flush() {
	j.mu.Lock()
	for _, entry := range j.queue {
		entry.done = true
	}
	ready := j.drainLocked()
	j.mu.Unlock()

	j.emitAll(ready)
}

// Pending reports how many stanzas are waiting on payloads.
func (j *Joiner) Pending() int {
	j.mu.Lock()
	defer j.mu.Unlock()
	return j.pending
}

// Queued reports how many stanzas are in the queue, waiting or merely behind
// one that is.
func (j *Joiner) Queued() int {
	j.mu.Lock()
	defer j.mu.Unlock()
	return len(j.queue)
}

func (j *Joiner) ageLocked() {
	for _, entry := range j.queue {
		if entry.done {
			continue
		}
		entry.age++
		if entry.age > j.lookahead {
			entry.done = true
		}
	}
}

// drainLocked takes the finished stanzas off the front of the queue.
//
// The front, and only the front: a finished stanza behind an unfinished one
// waits, because the unfinished one arrived first and that is the order the
// wire had.
func (j *Joiner) drainLocked() []wire.Envelope {
	var ready []wire.Envelope
	at := 0
	for ; at < len(j.queue); at++ {
		entry := j.queue[at]
		if !entry.ready() {
			break
		}
		ready = append(ready, entry.envelope())
		if entry.id != "" {
			if current, held := j.byID[entry.id]; held && current == entry {
				delete(j.byID, entry.id)
			}
			j.pending--
		}
	}
	j.queue = j.queue[at:]
	return ready
}

// emitAll delivers in order, and one at a time.
//
// The lock is this adapter's own rather than the engine's: two hooks on two
// goroutines both reach here, and a sink is a consumer's code that is entitled
// not to be reentrant.
func (j *Joiner) emitAll(envelopes []wire.Envelope) {
	if len(envelopes) == 0 || j.sink == nil {
		return
	}
	j.emitMu.Lock()
	defer j.emitMu.Unlock()
	for _, envelope := range envelopes {
		j.sink.Accept(envelope)
	}
}

// expects reports whether this stanza has an `<enc>` at that child index.
func (s *slot) expects(childIndex int) bool {
	for _, index := range s.encIndices {
		if index == childIndex {
			return true
		}
	}
	return false
}

// envelope builds what this stanza has.
//
// An `<enc>` that produced nothing is reported as `Unobserved` rather than
// omitted: a table missing an entry says the node was not there, and it was.
// `Unobserved` claims only what this adapter can see — that no payload arrived
// — and not why, which it cannot.
func (s *slot) envelope() wire.Envelope {
	if len(s.encIndices) == 0 {
		return wire.Envelope{Frame: s.frame}
	}
	entries := make([]wire.Plaintext, 0, len(s.encIndices))
	for _, index := range s.encIndices {
		if entry, seen := s.plaintexts[index]; seen {
			entries = append(entries, entry)
			continue
		}
		entries = append(entries, wire.Plaintext{
			Path:   []uint16{uint16(index)},
			Status: wire.StatusUnobserved,
		})
	}
	return wire.Envelope{Frame: s.frame, Plaintexts: entries}
}

// encChildIndices are the positions of a stanza's `<enc>` children among all
// of its children — the same numbering the engine reports a payload under.
func encChildIndices(node *waBinary.Node) []int {
	var indices []int
	for index, child := range node.GetChildren() {
		if child.Tag == "enc" {
			indices = append(indices, index)
		}
	}
	return indices
}

func cloneBytes(source []byte) []byte {
	if source == nil {
		return nil
	}
	out := make([]byte, len(source))
	copy(out, source)
	return out
}
