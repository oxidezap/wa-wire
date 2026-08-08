package wawire

import (
	"sync"

	waBinary "go.mau.fi/whatsmeow/binary"
)

// DefaultLookahead is how many later stanzas a pending message tolerates
// before it is emitted with whatever it has.
//
// Sized for the widest real fan-out rather than tuned: a message's payloads
// all arrive within its own processing, so a handful of intervening stanzas
// already means they are not coming.
const DefaultLookahead = 64

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
	mu        sync.Mutex
	pending   map[string]*pendingStanza
	order     []string
	lookahead int
	sink      Sink
}

// Sink receives envelopes as they complete.
//
// Called with the joiner's lock released, so a slow sink does not stall the
// engine's receive path — but the calls are ordered, and a sink that blocks
// still delays the envelopes behind it.
type Sink interface {
	Accept(envelope Envelope)
}

// SinkFunc adapts a function to a Sink.
type SinkFunc func(envelope Envelope)

// Accept calls the function.
func (f SinkFunc) Accept(envelope Envelope) { f(envelope) }

type pendingStanza struct {
	frame []byte
	// The child indices of this stanza's `<enc>` nodes, in document order.
	//
	// The indices themselves rather than a count: an `<enc>` is not
	// necessarily the first child, and a stanza whose `<enc>` nodes sit at
	// positions two and three has nothing at zero and one. Keeping the real
	// positions is also what lets an `<enc>` that produced nothing be reported
	// at the path it actually occupies.
	encIndices []int
	plaintexts map[int]Plaintext
	// How many stanzas have arrived since this one. Once it passes the
	// lookahead the payloads are not coming.
	age int
}

// NewJoiner returns a joiner emitting to sink.
func NewJoiner(sink Sink) *Joiner {
	return &Joiner{
		pending:   make(map[string]*pendingStanza),
		lookahead: DefaultLookahead,
		sink:      sink,
	}
}

// WithLookahead sets how many later stanzas a pending one tolerates. For tests
// and for a host that knows its own traffic; the default suits real fan-out.
func (j *Joiner) WithLookahead(stanzas int) *Joiner {
	j.lookahead = stanzas
	return j
}

// AcceptFrame takes a decoded stanza and its bytes.
//
// A stanza with no `<enc>` children crosses at once. One with them is held
// until they arrive or until the lookahead runs out.
//
// The frame is copied. The engine documents its buffer as valid only for the
// duration of the call — an uncompressed frame is a window into the transport's
// own read buffer — and holding a stanza means outliving the call by
// construction.
func (j *Joiner) AcceptFrame(node *waBinary.Node, frame []byte) {
	encIndices := encChildIndices(node)
	id, _ := node.Attrs["id"].(string)
	holdable := len(encIndices) > 0 && id != ""

	// Every stanza ages the ones waiting, including the ones that cross
	// straight through. The lookahead counts *later stanzas*, and an ack going
	// past is one — a receive path carrying nothing but acks would otherwise
	// hold a message for ever.
	j.mu.Lock()
	j.age()
	ready := j.evictLocked()
	if holdable {
		j.pending[id] = &pendingStanza{
			frame:      cloneBytes(frame),
			encIndices: encIndices,
			plaintexts: make(map[int]Plaintext, len(encIndices)),
		}
		j.order = append(j.order, id)
	}
	j.mu.Unlock()

	j.emitAll(ready)
	if !holdable {
		// Nothing to wait for, or nothing to key a wait on. A stanza carrying
		// `<enc>` children and no id is not one this can hold: it would never
		// be matched to its payloads and would sit until the lookahead ran
		// out. Emitting it now as L0-wire is the smaller claim.
		//
		// After the evictions, so stanzas leave in the order they arrived.
		j.emit(Envelope{Frame: cloneBytes(frame)})
	}
}

// AcceptPlaintext takes one decrypted payload and completes its stanza if this
// was the last one outstanding.
func (j *Joiner) AcceptPlaintext(messageID string, childIndex int, plaintext []byte) {
	var complete *Envelope

	j.mu.Lock()
	if stanza, held := j.pending[messageID]; held {
		stanza.plaintexts[childIndex] = Plaintext{
			Path:    []uint16{uint16(childIndex)},
			Status:  StatusOk,
			Payload: cloneBytes(plaintext),
		}
		if len(stanza.plaintexts) >= len(stanza.encIndices) {
			envelope := stanza.envelope()
			j.removeLocked(messageID)
			complete = &envelope
		}
	}
	j.mu.Unlock()

	if complete != nil {
		j.emit(*complete)
	}
}

// Flush emits every stanza still waiting, with whatever it has.
//
// For shutdown: a frame still waiting for a payload that will now never arrive
// is better emitted unobserved than lost, since the stanza was real either way.
func (j *Joiner) Flush() {
	j.mu.Lock()
	ready := make([]Envelope, 0, len(j.order))
	for _, id := range j.order {
		if stanza, held := j.pending[id]; held {
			ready = append(ready, stanza.envelope())
		}
	}
	j.pending = make(map[string]*pendingStanza)
	j.order = nil
	j.mu.Unlock()

	j.emitAll(ready)
}

// Pending reports how many stanzas are waiting. For tests and for a host that
// wants to see the queue rather than infer it.
func (j *Joiner) Pending() int {
	j.mu.Lock()
	defer j.mu.Unlock()
	return len(j.pending)
}

func (j *Joiner) age() {
	for _, stanza := range j.pending {
		stanza.age++
	}
}

func (j *Joiner) evictLocked() []Envelope {
	var ready []Envelope
	kept := j.order[:0]
	for _, id := range j.order {
		stanza, held := j.pending[id]
		if !held {
			continue
		}
		if stanza.age > j.lookahead {
			ready = append(ready, stanza.envelope())
			delete(j.pending, id)
			continue
		}
		kept = append(kept, id)
	}
	j.order = kept
	return ready
}

func (j *Joiner) removeLocked(id string) {
	delete(j.pending, id)
	for at, held := range j.order {
		if held == id {
			j.order = append(j.order[:at], j.order[at+1:]...)
			break
		}
	}
}

func (j *Joiner) emit(envelope Envelope) {
	if j.sink != nil {
		j.sink.Accept(envelope)
	}
}

func (j *Joiner) emitAll(envelopes []Envelope) {
	for _, envelope := range envelopes {
		j.emit(envelope)
	}
}

// envelope builds what this stanza has so far.
//
// An `<enc>` that produced nothing is reported as `Unobserved` rather than
// omitted: a table missing an entry says the node was not there, and it was.
// `Unobserved` claims only what this adapter can see — that no payload arrived
// — and not why, which it cannot.
func (p *pendingStanza) envelope() Envelope {
	entries := make([]Plaintext, 0, len(p.encIndices))
	for _, index := range p.encIndices {
		if entry, seen := p.plaintexts[index]; seen {
			entries = append(entries, entry)
			continue
		}
		entries = append(entries, Plaintext{
			Path:   []uint16{uint16(index)},
			Status: StatusUnobserved,
		})
	}
	return Envelope{Frame: p.frame, Plaintexts: entries}
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
