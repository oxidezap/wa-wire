package wawire

// Releasing a hypermeow session so another engine can take it.
//
// Client.Disconnect is the call, and which of the engine's three it is carries
// the whole meaning. It calls expectDisconnect before closing, so the automatic
// reconnect does not fire and the socket stays down until the caller asks for
// another. Its own doc says it emits no events — "the Disconnected event is
// only used when the connection is closed by the server or a network error" —
// and it touches no credentials, so the account stays registered and other
// devices see nothing.
//
// Client.Logout is the one that must stay out of reach. It sends
// remove-companion-device to the server and unpairs the device; nothing brings
// that back without the account holder scanning a code again. A host driving a
// handoff holds a Detacher and has no method to call for it.
//
// Client.ResetConnection is neither: it disconnects in order to reconnect at
// once, which is the opposite of giving the session up.

import "fmt"

// SessionCloser is what a detacher needs of the engine — the one call, nothing
// more, so what this depends on is visible in the type and a test can supply it.
type SessionCloser interface {
	Disconnect()
}

// Detacher gives up the session without ending the account's pairing.
type Detacher struct {
	client SessionCloser
}

// NewDetacher releases client's session when asked.
func NewDetacher(client SessionCloser) *Detacher {
	return &Detacher{client: client}
}

// Detach releases the session, leaving the device paired.
//
// Returning nil means the socket is closed and the engine will not open another
// of its own accord, so a second engine can take the session without the server
// killing one of them — WhatsApp allows one connection per device.
//
// Returning an error means the session is where it was. A host must not read a
// failure as permission to attach elsewhere, because the old connection may
// still be live.
//
// Idempotent: the engine's Disconnect returns immediately when there is no
// socket, and a host that crashed mid-handoff has to be able to start over.
func (detacher *Detacher) Detach() error {
	if detacher == nil || detacher.client == nil {
		return fmt.Errorf("the session was not released: no client to release")
	}
	detacher.client.Disconnect()
	return nil
}

// DetachingInfo is the declaration when the adapter can also release the
// session.
//
// Its own set because detaching needs the Client and an instance installed as a
// tap has only the hooks. A consumer requiring lifecycle.detach is saying it
// holds a Detacher, and one set covering both would be false for whichever it
// actually has.
var DetachingInfo = AdapterInfo{
	ID:              ID,
	AdapterVersion:  AdapterVersion,
	EngineVersion:   EngineVersion,
	ContractVersion: ContractVersion,
	Capabilities:    append(append([]Capability{}, Info.Capabilities...), Detach),
}
