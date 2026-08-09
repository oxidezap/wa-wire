package wawire

import (
	"errors"
	"testing"
)

// The declaration says what this adapter has, and what it does not.
//
// A consumer selects an engine on this list, so an entry that stops being true
// should fail here rather than mislead. The absences are asserted too: they are
// the half a consumer needs in order to look elsewhere.
func TestTheDeclarationNamesWhatThisAdapterHas(t *testing.T) {
	for _, capability := range []Capability{
		L0InboundTap,
		L0InboundAuthPhase,
		L0Plaintext,
		ZeroCopyFrame,
	} {
		if !Info.Has(capability) {
			t.Errorf("Info lacks %s", capability)
		}
	}
	for _, capability := range []Capability{
		// The engine has no outbound observation point, so a recording from
		// here holds the inbound half of a session and nothing the client
		// replied.
		L0OutboundObserved,
		// Nothing says when incoming handlers have drained, so a consumer
		// cannot know its queue is quiet. Absent rather than approximated.
		DrainHook,
		// Takeover is a separate declaration, because it is a separate promise.
		Takeover,
	} {
		if Info.Has(capability) {
			t.Errorf("Info claims %s, which this adapter does not have", capability)
		}
	}
}

// Takeover is declared apart, and is not a superset of the tap.
//
// Two modes, two promises: taking a stanza over means the engine's own handler
// does not run, which is a different thing to guarantee than watching one go
// past.
func TestTakeoverIsItsOwnDeclaration(t *testing.T) {
	if !TakeoverInfo.Has(Takeover) {
		t.Fatal("TakeoverInfo must claim takeover")
	}
	if Info.Has(Takeover) {
		t.Fatal("the tap does not claim takeover")
	}
}

// A consumer asking for what this adapter lacks finds out at setup.
//
// Without the gate it discovers the absence as *missing traffic*, where the
// evidence of the problem is the thing that is not there.
func TestRequireRefusesWhatTheAdapterLacks(t *testing.T) {
	if err := Info.Require(L0InboundTap, L0Plaintext); err != nil {
		t.Fatalf("what it has must be grantable: %v", err)
	}

	err := Info.Require(L0InboundTap, DrainHook, L0OutboundObserved)
	var unmet *UnmetCapabilities
	if !errors.As(err, &unmet) {
		t.Fatalf("err = %v, want UnmetCapabilities", err)
	}
	if len(unmet.Missing) != 2 {
		t.Fatalf("missing = %v, want both of them named", unmet.Missing)
	}
	// The message names them, since a consumer reading it is deciding what to
	// do next.
	for _, name := range unmet.Missing {
		if name == "" {
			t.Fatal("an unnamed capability tells nobody anything")
		}
	}
}

// Verify catches an envelope the declaration forbids.
//
// Both sides of the boundary enforce this, because each is the other's only
// guard: a producer in Go does not run the Rust encoder, and a consumer in Rust
// does not run this one.
func TestVerifyCatchesWhatTheDeclarationForbids(t *testing.T) {
	// Claiming zero-copy and delivering a re-encoding.
	if err := Info.Verify(Envelope{FrameOrigin: ReEncoded}); err == nil {
		t.Error("a re-encoded frame contradicts l0.zero-copy-frame")
	}

	// Outbound without claiming to observe the outbound half. `l0.outbound` is
	// the ability to send and says nothing about seeing what left, which is
	// why the check names the other one.
	if err := Info.Verify(Envelope{Direction: Outbound}); err == nil {
		t.Error("an outbound stanza needs l0.outbound.observed")
	}

	// Plaintexts from a declaration that does not promise them.
	bare := AdapterInfo{Capabilities: []Capability{L0InboundTap}}
	if err := bare.Verify(Envelope{Plaintexts: []Plaintext{{Status: StatusUnobserved}}}); err == nil {
		t.Error("plaintexts need l0.plaintext")
	}

	// And what the declaration does allow passes.
	if err := Info.Verify(Envelope{
		Frame:      []byte("f"),
		Plaintexts: []Plaintext{{Path: []uint16{0}, Status: StatusOk, Payload: []byte("p")}},
	}); err != nil {
		t.Errorf("a well-formed inbound envelope must pass: %v", err)
	}
}

// The vocabulary is the contract's, not this adapter's.
//
// Every identifier contract version 1 defines is named here, including the ones
// this adapter does not have — a consumer that cannot name a capability cannot
// require one either.
func TestTheVocabularyIsCompleteEvenWhereTheAdapterIsNot(t *testing.T) {
	all := []Capability{
		L0InboundTap,
		L0InboundAuthPhase,
		L0Outbound,
		L0OutboundObserved,
		L0Request,
		L0Plaintext,
		L0PlaintextCause,
		Takeover,
		ZeroCopyFrame,
		DrainHook,
	}
	if len(all) != 10 {
		t.Fatalf("contract version 1 defines ten capabilities, this names %d", len(all))
	}
	seen := make(map[Capability]bool, len(all))
	for _, capability := range all {
		if capability == "" {
			t.Fatal("an empty identifier names nothing")
		}
		if seen[capability] {
			t.Fatalf("%s is named twice", capability)
		}
		seen[capability] = true
	}
}

// An envelope the declaration forbids does not reach the sink.
//
// The check runs on the way out, which is the only place that can see it. A
// recording shipping a stanza its own manifest forbids is worse than one
// missing it: nothing downstream can tell the record is not to be trusted.
func TestAForbiddenEnvelopeNeverReachesTheSink(t *testing.T) {
	var reached int
	guard := verifying{
		info: AdapterInfo{Capabilities: []Capability{L0InboundTap}},
		sink: SinkFunc(func(Envelope) { reached++ }),
	}

	defer func() {
		if recover() == nil {
			t.Fatal("a forbidden envelope must fail loudly")
		}
		if reached != 0 {
			t.Fatal("and must not reach the sink first")
		}
	}()
	guard.Accept(Envelope{Plaintexts: []Plaintext{{Status: StatusUnobserved}}})
}

// Takeover cannot promise plaintext, because the engine cannot deliver both.
//
// Claiming a stanza returns `drop` from the raw-node hook, and the engine's
// `handleFrame` returns there — before the stanza is queued for decryption. So
// a claimed stanza never reaches Signal. A declaration offering both would let
// a consumer require a combination that cannot happen and discover it as
// missing payloads.
func TestTakeoverDoesNotPromisePlaintext(t *testing.T) {
	if TakeoverInfo.Has(L0Plaintext) {
		t.Fatal("a claimed stanza is never decrypted")
	}
	if err := TakeoverInfo.Require(Takeover, L0Plaintext); err == nil {
		t.Fatal("the impossible combination must be refused at setup")
	}
	// And the tap, which claims nothing, still has it.
	if !Info.Has(L0Plaintext) {
		t.Fatal("the tap does get plaintexts")
	}
}
