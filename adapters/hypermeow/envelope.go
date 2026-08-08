// Package wawire is the wa-wire adapter for the hypermeow engine.
//
// The boundary format, written out a third time. `wa-wire-contract` decodes
// it, the zapo adapter writes it in TypeScript, and this writes it in Go —
// because an adapter runs inside its engine, and this engine is Go. Rust in Go
// means cgo, and cgo in the per-stanza hot path is the cost the boundary exists
// to avoid.
//
// Three descriptions of one format that are only ever tested separately are
// three formats waiting to diverge, so the fixtures this package writes are
// read back by the Rust side and the ones it writes are read here.
//
// The layout is fixed by RFC-008:
//
//	Envelope
//	  version      u16
//	  flags        u16     bit0 direction, bit1 frame_origin
//	  frame_len    u32
//	  frame        u8[frame_len]
//	  pt_count     u16
//	  pt_entries   PlaintextEntry[pt_count]
//
//	PlaintextEntry
//	  path_len     u8
//	  path         u16[path_len]      little-endian child indices from the root
//	  status       u8
//	  payload_len  u32
//	  payload      u8[payload_len]
//
// Little-endian throughout — unlike the stanza inside frame, which is
// WhatsApp's own big-endian encoding and travels untouched.
package wawire

import (
	"encoding/binary"
	"errors"
	"fmt"
)

// ContractVersion is the boundary version this package writes.
//
// Not the WhatsApp protocol's version and never bumped for it: a protocol
// change crosses at L0 without the boundary noticing, which is what keeps a
// deployed adapter working when Meta ships something.
const ContractVersion uint16 = 1

// HeaderLen counts the bytes before the frame: version, flags, frame length.
const HeaderLen = 8

// MaxPathDepth is what the format can express: the length prefix is a u8.
//
// Not the same as what is reachable. `wa-wire-adapter` bounds a path at 64
// because the codec will not nest deeper than that, so a path past it cannot
// address anything — but that is the Rust SDK's rule and this is the format's,
// and a Go adapter has no business enforcing a limit the contract does not
// state.
const MaxPathDepth = 255

// Direction says which way a stanza was travelling.
type Direction uint8

const (
	// Inbound is a stanza received from the server.
	Inbound Direction = 0
	// Outbound is a stanza the client sent.
	Outbound Direction = 1
)

// FrameOrigin says whether the frame is the engine's own buffer.
type FrameOrigin uint8

const (
	// Original is the buffer the engine's decoder consumed, verbatim.
	Original FrameOrigin = 0
	// ReEncoded came from a decoded node, the bytes being unreachable.
	ReEncoded FrameOrigin = 1
)

// PlaintextStatus says whether an entry holds usable bytes, and if not, why.
type PlaintextStatus uint8

const (
	// StatusOk carries the plaintext.
	StatusOk PlaintextStatus = 0
	// StatusDecryptFailed means Signal refused it.
	StatusDecryptFailed PlaintextStatus = 1
	// StatusUnsupported means the adapter cannot decrypt this kind.
	StatusUnsupported PlaintextStatus = 2
	// StatusUnobserved means the node produced nothing the adapter saw.
	//
	// Claims less than the two failures above: an adapter watching plaintexts
	// appear can say a node produced none, but not why.
	StatusUnobserved PlaintextStatus = 3
)

// Plaintext is one decrypted payload, addressed by the path of the node it
// came from.
type Plaintext struct {
	// Path holds child indices from the root node.
	Path []uint16
	// Status says whether Payload is usable.
	Status PlaintextStatus
	// Payload is empty unless Status is StatusOk.
	Payload []byte
}

// Envelope is one stanza crossing the boundary.
type Envelope struct {
	Direction   Direction
	FrameOrigin FrameOrigin
	// Frame is the stanza exactly as the engine decoded it.
	Frame []byte
	// Plaintexts is the side table, one entry per node that decrypted.
	Plaintexts []Plaintext
}

// Errors an envelope can refuse to encode with. Each is a claim the format
// cannot carry, caught here rather than written out for a reader to trip over.
var (
	// ErrPathTooDeep is a path the length prefix cannot count.
	ErrPathTooDeep = errors.New("wawire: plaintext path is deeper than the contract can count")
	// ErrPayloadWithoutOk is a payload on an entry that did not decrypt.
	//
	// The contract says a non-Ok entry carries nothing, and a reader is
	// entitled to skip its bytes. Writing some would put data where nobody
	// looks.
	ErrPayloadWithoutOk = errors.New("wawire: only an Ok plaintext may carry a payload")
	// ErrTooManyPlaintexts is more entries than the count field can hold.
	ErrTooManyPlaintexts = errors.New("wawire: more plaintexts than the contract can count")
	// ErrFrameTooLong is a frame longer than the length field can hold.
	ErrFrameTooLong = errors.New("wawire: frame is longer than the contract can count")
)

// Encode writes the envelope in the boundary format.
//
// Allocates once and copies the frame, which an in-process consumer would not
// need — but this adapter's consumer is on the other side of a process or a
// language, so the copy is the crossing rather than an overhead on top of it.
func (e Envelope) Encode() ([]byte, error) {
	if len(e.Frame) > int(^uint32(0)) {
		return nil, ErrFrameTooLong
	}
	if len(e.Plaintexts) > int(^uint16(0)) {
		return nil, ErrTooManyPlaintexts
	}
	for index, plaintext := range e.Plaintexts {
		if len(plaintext.Path) > MaxPathDepth {
			return nil, fmt.Errorf("%w: entry %d has %d components", ErrPathTooDeep, index, len(plaintext.Path))
		}
		if plaintext.Status != StatusOk && len(plaintext.Payload) > 0 {
			return nil, fmt.Errorf("%w: entry %d", ErrPayloadWithoutOk, index)
		}
	}

	out := make([]byte, 0, e.encodedLen())
	out = binary.LittleEndian.AppendUint16(out, ContractVersion)
	out = binary.LittleEndian.AppendUint16(out, e.flags())
	out = binary.LittleEndian.AppendUint32(out, uint32(len(e.Frame)))
	out = append(out, e.Frame...)
	out = binary.LittleEndian.AppendUint16(out, uint16(len(e.Plaintexts)))
	for _, plaintext := range e.Plaintexts {
		out = append(out, uint8(len(plaintext.Path)))
		for _, component := range plaintext.Path {
			out = binary.LittleEndian.AppendUint16(out, component)
		}
		out = append(out, uint8(plaintext.Status))
		out = binary.LittleEndian.AppendUint32(out, uint32(len(plaintext.Payload)))
		out = append(out, plaintext.Payload...)
	}
	return out, nil
}

// flags packs direction and frame origin into the word the contract reads.
func (e Envelope) flags() uint16 {
	var flags uint16
	if e.Direction == Outbound {
		flags |= 1 << 0
	}
	if e.FrameOrigin == ReEncoded {
		flags |= 1 << 1
	}
	return flags
}

func (e Envelope) encodedLen() int {
	total := HeaderLen + len(e.Frame) + 2
	for _, plaintext := range e.Plaintexts {
		total += 1 + 2*len(plaintext.Path) + 1 + 4 + len(plaintext.Payload)
	}
	return total
}
