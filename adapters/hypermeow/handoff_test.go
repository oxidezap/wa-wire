package wawire

import "testing"

// engine has the two ways to stop that must not be confused, and an account
// only one of them touches.
type engine struct {
	disconnected int
	paired       bool
}

func newEngine() *engine {
	return &engine{paired: true}
}

func (e *engine) Disconnect() {
	e.disconnected++
}

// Logout is deliberately not part of SessionCloser: reaching it is a choice a
// caller has to make by name, holding the engine's own type.
func (e *engine) Logout() {
	e.paired = false
}

func TestDetachClosesTheSocketAndLeavesTheDevicePaired(t *testing.T) {
	client := newEngine()

	if err := NewDetacher(client).Detach(); err != nil {
		t.Fatalf("detach: %v", err)
	}

	if client.disconnected != 1 {
		t.Fatalf("disconnected %d times, want 1", client.disconnected)
	}
	if !client.paired {
		t.Fatal("the device was unpaired — a detach that logs out is a different act")
	}
}

func TestDetachIsIdempotent(t *testing.T) {
	// A host that crashed mid-handoff has to be able to start over, and the
	// postcondition it needs already holds.
	client := newEngine()
	detacher := NewDetacher(client)

	if err := detacher.Detach(); err != nil {
		t.Fatalf("first detach: %v", err)
	}
	if err := detacher.Detach(); err != nil {
		t.Fatalf("second detach: %v", err)
	}

	if client.disconnected != 2 || !client.paired {
		t.Fatalf("disconnected=%d paired=%v", client.disconnected, client.paired)
	}
}

func TestDetachWithoutAClientReportsRatherThanPretends(t *testing.T) {
	// Reporting success here would tell a host the session is free when nothing
	// released it, which is the two-writer case with a clean log line.
	if err := NewDetacher(nil).Detach(); err == nil {
		t.Fatal("a detacher with no client claimed to have released the session")
	}
	var absent *Detacher
	if err := absent.Detach(); err == nil {
		t.Fatal("a nil detacher claimed to have released the session")
	}
}

func TestOnlyTheDetachingDeclarationClaimsIt(t *testing.T) {
	if Info.Has(Detach) {
		t.Fatal("the tap declares lifecycle.detach and has no client to do it with")
	}
	if !DetachingInfo.Has(Detach) {
		t.Fatal("the detaching declaration does not claim lifecycle.detach")
	}

	// One addition, not a different adapter.
	if len(DetachingInfo.Capabilities) != len(Info.Capabilities)+1 {
		t.Fatalf("detaching declares %d capabilities, the tap %d",
			len(DetachingInfo.Capabilities), len(Info.Capabilities))
	}
	for _, capability := range Info.Capabilities {
		if !DetachingInfo.Has(capability) {
			t.Fatalf("detaching dropped %s", capability)
		}
	}

	// And appending to it did not write into the tap's backing array.
	if Info.Has(Detach) {
		t.Fatal("building the detaching set mutated the tap's")
	}
}
