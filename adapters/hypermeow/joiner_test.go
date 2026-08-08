package wawire

import (
	"bytes"
	"fmt"
	"testing"

	waBinary "go.mau.fi/whatsmeow/binary"
)

// A stanza with an `<enc>` at a position other than the first.
//
// The realistic shape, and the one that catches an adapter numbering `<enc>`
// nodes rather than children: here the payload's child index is 1, and an
// adapter counting `<enc>` nodes would address it as 0 — a plaintext attached
// to the wrong node, which reads as a message from the wrong sender.
func messageNode(id string, encCount int) *waBinary.Node {
	children := []waBinary.Node{{Tag: "participants"}}
	for index := 0; index < encCount; index++ {
		children = append(children, waBinary.Node{
			Tag:     "enc",
			Attrs:   waBinary.Attrs{"type": "msg"},
			Content: []byte("ciphertext"),
		})
	}
	return &waBinary.Node{
		Tag:     "message",
		Attrs:   waBinary.Attrs{"id": id},
		Content: children,
	}
}

type collector struct {
	envelopes []Envelope
}

func (c *collector) Accept(envelope Envelope) {
	c.envelopes = append(c.envelopes, envelope)
}

// A stanza with nothing encrypted crosses at once, as L0-wire.
//
// Most stanzas are this: an ack, a receipt, a notification. Holding them would
// delay every one of them for a payload that was never coming.
func TestAStanzaWithNoEncCrossesImmediately(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)

	joiner.AcceptFrame(&waBinary.Node{Tag: "ack", Attrs: waBinary.Attrs{"id": "A1"}}, []byte("frame"))

	if len(sink.envelopes) != 1 {
		t.Fatalf("got %d envelopes, want 1", len(sink.envelopes))
	}
	if joiner.Pending() != 0 {
		t.Fatalf("nothing should be held")
	}
	if len(sink.envelopes[0].Plaintexts) != 0 {
		t.Fatalf("an ack has no plaintext table")
	}
}

// A payload is addressed by the child index the engine reported.
//
// The engine reports the position among *all* children rather than among the
// `<enc>` nodes, so the path is that number and nothing is inferred. The other
// two engines report an `<enc>`-relative index and their adapters have to
// resolve it, which is ambiguous the moment a stanza carries anything else —
// as this one does.
func TestAPayloadIsAddressedByItsChildIndex(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)
	node := messageNode("M1", 1)

	joiner.AcceptFrame(node, []byte("frame"))
	if len(sink.envelopes) != 0 {
		t.Fatalf("a message with an <enc> waits for it")
	}

	// Child index 1: `<participants>` is child 0.
	joiner.AcceptPlaintext("M1", 1, []byte("plain"))

	if len(sink.envelopes) != 1 {
		t.Fatalf("the last payload completes the stanza")
	}
	entries := sink.envelopes[0].Plaintexts
	if len(entries) != 1 {
		t.Fatalf("got %d entries, want 1", len(entries))
	}
	if len(entries[0].Path) != 1 || entries[0].Path[0] != 1 {
		t.Fatalf("path = %v, want [1]", entries[0].Path)
	}
	if !bytes.Equal(entries[0].Payload, []byte("plain")) {
		t.Fatalf("payload = %q", entries[0].Payload)
	}
}

// A stanza closes on its last payload rather than on a clock.
func TestAStanzaClosesWhenItsLastPayloadArrives(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)
	joiner.AcceptFrame(messageNode("M1", 2), []byte("frame"))

	joiner.AcceptPlaintext("M1", 1, []byte("one"))
	if len(sink.envelopes) != 0 {
		t.Fatalf("one of two is not the last")
	}
	joiner.AcceptPlaintext("M1", 2, []byte("two"))
	if len(sink.envelopes) != 1 {
		t.Fatalf("the second completes it")
	}
	if got := len(sink.envelopes[0].Plaintexts); got != 2 {
		t.Fatalf("got %d entries, want 2", got)
	}
}

// An `<enc>` that never produces a payload is reported, not omitted.
//
// A table missing an entry says the node was not there, and it was.
// `Unobserved` claims only what this adapter can see — that no payload arrived
// — and not why, which it cannot.
func TestAnEncThatProducesNothingIsReportedAsUnobserved(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink).WithLookahead(1)
	joiner.AcceptFrame(messageNode("M1", 2), []byte("frame"))
	joiner.AcceptPlaintext("M1", 1, []byte("one"))

	// Two later stanzas: the payloads are not coming.
	joiner.AcceptFrame(&waBinary.Node{Tag: "ack", Attrs: waBinary.Attrs{"id": "A1"}}, []byte("a"))
	joiner.AcceptFrame(&waBinary.Node{Tag: "ack", Attrs: waBinary.Attrs{"id": "A2"}}, []byte("b"))

	var held *Envelope
	for index, envelope := range sink.envelopes {
		if len(envelope.Plaintexts) == 2 {
			held = &sink.envelopes[index]
		}
	}
	if held == nil {
		t.Fatalf("the held stanza was never emitted: %d envelopes", len(sink.envelopes))
	}
	if held.Plaintexts[0].Status != StatusOk {
		t.Fatalf("the payload that arrived is Ok")
	}
	if held.Plaintexts[1].Status != StatusUnobserved {
		t.Fatalf("the one that did not is Unobserved, got %v", held.Plaintexts[1].Status)
	}
	// And it is reported at the position it occupies, not at the next free one.
	if held.Plaintexts[1].Path[0] != 2 {
		t.Fatalf("path = %v, want [2]", held.Plaintexts[1].Path)
	}
}

// Shutdown emits what is still waiting rather than dropping it.
func TestFlushEmitsWhatIsStillWaiting(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)
	joiner.AcceptFrame(messageNode("M1", 1), []byte("frame"))

	joiner.Flush()

	if len(sink.envelopes) != 1 {
		t.Fatalf("the stanza was real either way")
	}
	if joiner.Pending() != 0 {
		t.Fatalf("nothing is held after a flush")
	}
}

// The frame is copied, because the engine's buffer does not outlive the call.
//
// An uncompressed frame is a window into the transport's own read buffer. A
// joiner that kept the slice would emit whatever the transport read next.
func TestTheFrameIsCopiedOutOfTheEnginesBuffer(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)

	frame := []byte("original")
	joiner.AcceptFrame(messageNode("M1", 1), frame)
	// The engine reuses its buffer for the next read.
	copy(frame, []byte("OVERWRIT"))
	joiner.AcceptPlaintext("M1", 1, []byte("plain"))

	if got := string(sink.envelopes[0].Frame); got != "original" {
		t.Fatalf("frame = %q, want the bytes as they were", got)
	}
}

// A payload is copied too, for the same reason the frame is.
func TestThePayloadIsCopied(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)
	joiner.AcceptFrame(messageNode("M1", 1), []byte("frame"))

	plaintext := []byte("secret")
	joiner.AcceptPlaintext("M1", 1, plaintext)
	copy(plaintext, []byte("CHANGED"))

	if got := string(sink.envelopes[0].Plaintexts[0].Payload); got != "secret" {
		t.Fatalf("payload = %q, want the bytes as they were", got)
	}
}

// A payload for a stanza nobody is holding is dropped rather than invented.
//
// It happens on shutdown, where the frame has already been flushed. Attaching
// it to whatever is pending would put a plaintext on the wrong stanza.
func TestAPayloadWithNoStanzaIsDropped(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)

	joiner.AcceptPlaintext("nobody-is-holding-this", 0, []byte("plain"))

	if len(sink.envelopes) != 0 {
		t.Fatalf("got %d envelopes, want none", len(sink.envelopes))
	}
}

// A stanza carrying `<enc>` children and no id cannot be held, so it crosses
// as L0-wire rather than sitting until the lookahead expires.
func TestAStanzaWithoutAnIdCrossesUnjoined(t *testing.T) {
	sink := &collector{}
	joiner := NewJoiner(sink)

	node := messageNode("", 1)
	delete(node.Attrs, "id")
	joiner.AcceptFrame(node, []byte("frame"))

	if len(sink.envelopes) != 1 {
		t.Fatalf("got %d envelopes, want 1", len(sink.envelopes))
	}
	if len(sink.envelopes[0].Plaintexts) != 0 {
		t.Fatalf("nothing could be joined to it")
	}
}

// Every envelope the joiner emits satisfies the declaration.
//
// The claim a consumer selects an engine on, checked against the traffic rather
// than left as a comment.
func TestEveryEmittedEnvelopeSatisfiesTheDeclaration(t *testing.T) {
	var checked int
	joiner := NewJoiner(SinkFunc(func(envelope Envelope) {
		checked++
		if err := Info.Verify(envelope); err != nil {
			t.Errorf("envelope %d: %v", checked, err)
		}
		if _, err := envelope.Encode(); err != nil {
			t.Errorf("envelope %d does not encode: %v", checked, err)
		}
	}))

	for index := 0; index < 4; index++ {
		id := fmt.Sprintf("M%d", index)
		joiner.AcceptFrame(messageNode(id, 1), []byte("frame"))
		joiner.AcceptPlaintext(id, 1, []byte("plain"))
	}
	joiner.AcceptFrame(&waBinary.Node{Tag: "ack", Attrs: waBinary.Attrs{"id": "A"}}, []byte("a"))
	joiner.Flush()

	if checked != 5 {
		t.Fatalf("checked %d envelopes, want 5", checked)
	}
}
