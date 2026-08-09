package wawire

import (
	"context"
	"strings"

	"github.com/oxidezap/wa-wire/adapters/hypermeow/wire"
	"go.mau.fi/whatsmeow"
	waBinary "go.mau.fi/whatsmeow/binary"
)

// The boundary format, re-exported so a consumer of this adapter sees one
// package. The format lives apart because it does not need the engine; that it
// does not is the point, not an inconvenience to paper over.
type (
	// Envelope is one stanza crossing the boundary.
	Envelope = wire.Envelope
	// Plaintext is one decrypted payload, addressed by its node's path.
	Plaintext = wire.Plaintext
	// Direction says which way a stanza was travelling.
	Direction = wire.Direction
	// FrameOrigin says whether the frame is the engine's own buffer.
	FrameOrigin = wire.FrameOrigin
	// PlaintextStatus says whether an entry holds usable bytes.
	PlaintextStatus = wire.PlaintextStatus
)

// The format's constants, re-exported alongside its types.
const (
	// ContractVersion is the boundary version this adapter writes.
	ContractVersion = wire.ContractVersion
	// Inbound is a stanza received from the server.
	Inbound = wire.Inbound
	// Outbound is a stanza the client sent.
	Outbound = wire.Outbound
	// Original is the buffer the engine's decoder consumed, verbatim.
	Original = wire.Original
	// ReEncoded came from a decoded node, the bytes being unreachable.
	ReEncoded = wire.ReEncoded
	// StatusOk carries the plaintext.
	StatusOk = wire.StatusOk
	// StatusDecryptFailed means Signal refused it.
	StatusDecryptFailed = wire.StatusDecryptFailed
	// StatusUnsupported means the adapter cannot decrypt this kind.
	StatusUnsupported = wire.StatusUnsupported
	// StatusUnobserved means the node produced nothing the adapter saw.
	StatusUnobserved = wire.StatusUnobserved
)

// Capability is one thing an adapter may or may not be able to do.
//
// The vocabulary is the contract's, not this adapter's: a consumer that cannot
// name a capability cannot require one either, and would discover the absence
// as missing traffic — which is the outcome declaring them exists to prevent.
// So every identifier the contract defines appears here, including the ones
// this adapter does not have.
type Capability string

// The capabilities of contract version 1.
const (
	// L0InboundTap observes every inbound stanza, without exception.
	L0InboundTap Capability = "l0.inbound.tap"
	// L0InboundAuthPhase means the tap also covers authentication and stream
	// control, not only post-login traffic.
	L0InboundAuthPhase Capability = "l0.inbound.auth-phase"
	// L0Outbound sends a raw stanza.
	L0Outbound Capability = "l0.outbound"
	// L0OutboundObserved reports each stanza the engine sent, as it went to
	// the wire. Distinct from L0Outbound, which is the ability to send.
	L0OutboundObserved Capability = "l0.outbound.observed"
	// L0Request is raw request/response against a stanza.
	L0Request Capability = "l0.request"
	// L0Plaintext emits the payloads the engine decrypted alongside the frame.
	L0Plaintext Capability = "l0.plaintext"
	// L0PlaintextCause says *why* an `<enc>` produced no plaintext, not
	// merely that none arrived. Without it an entry says `Unobserved`, under
	// which a build whose messages stopped decrypting looks exactly like one
	// whose adapter stopped observing.
	L0PlaintextCause Capability = "l0.plaintext.cause"
	// Takeover suppresses the engine's own dispatch.
	Takeover Capability = "l0.takeover"
	// ZeroCopyFrame supplies the engine's original frame bytes.
	ZeroCopyFrame Capability = "l0.zero-copy-frame"
	// DrainHook reports when incoming handlers have drained.
	DrainHook Capability = "lifecycle.drain-hook"
)

// AdapterInfo is what this adapter declares at setup.
type AdapterInfo struct {
	ID              string
	AdapterVersion  string
	EngineVersion   string
	ContractVersion uint16
	Capabilities    []Capability
}

// The identity this adapter reports.
const (
	// ID is how this adapter names itself in a recording.
	ID = "hypermeow"
	// AdapterVersion is this package's version.
	AdapterVersion = "0.1.0"
	// EngineVersion is the engine this was written against.
	//
	// A branch rather than a release: the hooks exist in
	// polymorfa/hypermeow#5 and nowhere published yet.
	EngineVersion = "0.0.0+frame-bytes-and-plaintext-hooks"
)

// Info is the tap declaration.
//
// No `lifecycle.drain-hook`: nothing in the engine says when incoming handlers
// have finished, so a consumer cannot know its queue is quiet. Absent rather
// than approximated.
//
// No `l0.outbound.observed`: the engine has no outbound observation point.
// A recording from this adapter holds the inbound half of a session and
// nothing the client replied.
var Info = AdapterInfo{
	ID:              ID,
	AdapterVersion:  AdapterVersion,
	EngineVersion:   EngineVersion,
	ContractVersion: ContractVersion,
	Capabilities: []Capability{
		L0InboundTap,
		L0InboundAuthPhase,
		L0Plaintext,
		ZeroCopyFrame,
	},
}

// TakeoverInfo is the declaration when the adapter also claims stanzas.
//
// A separate set because the two modes are not the same adapter, and not a
// superset of the tap: claiming a stanza returns `drop` from the raw-node hook,
// and the engine's `handleFrame` returns there — before the stanza is queued
// for decryption. So a claimed stanza never reaches Signal and no plaintext
// ever arrives for it.
//
// Declaring `l0.plaintext` here would let `Require(Takeover, L0Plaintext)`
// succeed for a combination the engine cannot produce, and the consumer would
// discover it as missing payloads. The absence is the honest half of the
// promise.
var TakeoverInfo = AdapterInfo{
	ID:              ID,
	AdapterVersion:  AdapterVersion,
	EngineVersion:   EngineVersion,
	ContractVersion: ContractVersion,
	Capabilities: []Capability{
		L0InboundTap,
		L0InboundAuthPhase,
		ZeroCopyFrame,
		Takeover,
	},
}

// Has reports whether the declaration includes a capability.
func (info AdapterInfo) Has(capability Capability) bool {
	for _, held := range info.Capabilities {
		if held == capability {
			return true
		}
	}
	return false
}

// Require refuses unless the declaration holds every capability in needed.
//
// The setup-time gate. Without it a consumer discovers that its engine never
// emits plaintext, or re-encodes frames it meant to replay, as *missing
// traffic* — where the evidence of the problem is the thing that is absent.
func (info AdapterInfo) Require(needed ...Capability) error {
	var missing []string
	for _, capability := range needed {
		if !info.Has(capability) {
			missing = append(missing, string(capability))
		}
	}
	if len(missing) == 0 {
		return nil
	}
	return &UnmetCapabilities{Missing: missing}
}

// UnmetCapabilities is a consumer asking for what this adapter does not have.
type UnmetCapabilities struct {
	Missing []string
}

func (e *UnmetCapabilities) Error() string {
	return "wawire: adapter lacks " + strings.Join(e.Missing, ", ")
}

// Verify checks an envelope against a declaration.
//
// The declaration is what a consumer selects an engine on, so a capability
// that stops being true should fail a test rather than quietly mislead. Both
// sides of the boundary enforce it, because each is the other's only guard.
func (info AdapterInfo) Verify(envelope Envelope) error {
	if info.Has(ZeroCopyFrame) && envelope.FrameOrigin == ReEncoded {
		return &Violation{Reason: "declared l0.zero-copy-frame and delivered a re-encoded frame"}
	}
	if !info.Has(L0Plaintext) && len(envelope.Plaintexts) > 0 {
		return &Violation{Reason: "delivered plaintexts without declaring l0.plaintext"}
	}
	if !info.Has(L0OutboundObserved) && envelope.Direction == Outbound {
		return &Violation{Reason: "delivered an outbound stanza without declaring l0.outbound.observed"}
	}
	// A cause is a claim about a failure, and only an adapter that reports
	// failures can make one.
	if !info.Has(L0PlaintextCause) {
		for _, plaintext := range envelope.Plaintexts {
			if plaintext.Status == StatusDecryptFailed || plaintext.Status == StatusUnsupported {
				return &Violation{
					Reason: "named a cause for a missing plaintext without declaring l0.plaintext.cause",
				}
			}
		}
	}
	return nil
}

// Violation is an envelope an adapter's own declaration forbids.
type Violation struct {
	Reason string
}

func (e *Violation) Error() string { return "wawire: " + e.Reason }

// Tap forwards every stanza the engine decodes, with its plaintexts, to sink.
//
// Installs by setting the engine's hooks, which is the whole of it: the engine
// has no plugin host, so there is nothing to register with and nothing to
// unregister from. A caller that wants to stop clears the fields itself.
//
// Returns the joiner so a caller can flush it at shutdown — a frame still
// waiting for a payload that will never arrive is better emitted unobserved
// than lost.
func Tap(client *whatsmeow.Client, sink Sink) *Joiner {
	joiner := NewJoiner(verifying{info: Info, sink: sink})

	client.RawNodeHandler = func(_ context.Context, raw whatsmeow.RawNode) (*waBinary.Node, bool) {
		joiner.AcceptFrame(raw.Node, raw.Frame)
		// Observed, never claimed: the engine's own dispatch runs afterwards.
		// Dropping here is takeover, which is a different declaration.
		return nil, false
	}

	client.DecryptedPayloadHandler = func(_ context.Context, payload whatsmeow.DecryptedPayload) {
		joiner.AcceptPlaintext(payload.Info.ID, payload.ChildIndex, payload.Plaintext)
	}

	return joiner
}

// verifying checks each envelope against the declaration on its way out.
type verifying struct {
	info AdapterInfo
	sink Sink
}

func (v verifying) Accept(envelope Envelope) {
	if err := v.info.Verify(envelope); err != nil {
		// A declaration that has stopped being true is worth failing over, and
		// this is the only place that can see it. Dropping the envelope keeps
		// the recording honest rather than shipping a stanza the manifest
		// forbids; the panic is what a test catches.
		panic(err)
	}
	v.sink.Accept(envelope)
}
