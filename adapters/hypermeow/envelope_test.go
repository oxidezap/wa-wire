package wawire

import (
	"bytes"
	"errors"
	"testing"
)

// The layout, pinned against the same vector `wa-wire-contract` pins.
//
// Copied deliberately rather than generated: this is the one place where the
// Go writer and the Rust reader can be compared without running either, and a
// vector derived from one of them would only prove that one is consistent with
// itself.
func TestByteLayoutIsExact(t *testing.T) {
	envelope := Envelope{
		Direction: Outbound,
		Frame:     []byte{0x01, 0x02},
		Plaintexts: []Plaintext{{
			Path:    []uint16{258},
			Status:  StatusOk,
			Payload: []byte("ab"),
		}},
	}

	got, err := envelope.Encode()
	if err != nil {
		t.Fatalf("encode: %v", err)
	}

	want := []byte{
		0x01, 0x00, // version = 1
		0x01, 0x00, // flags = outbound
		0x02, 0x00, 0x00, 0x00, // frame_len = 2
		0x01, 0x02, // frame
		0x01, 0x00, // pt_count = 1
		0x01,       // path_len = 1 component
		0x02, 0x01, // path[0] = 258 little-endian
		0x00,                   // status = Ok
		0x02, 0x00, 0x00, 0x00, // payload_len = 2
		'a', 'b', // payload
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("layout differs\n got: %x\nwant: %x", got, want)
	}
	if HeaderLen != 8 {
		t.Fatalf("HeaderLen = %d, want 8", HeaderLen)
	}
}

// An envelope with nothing decrypted still says so, with a count of zero.
//
// Most stanzas never carried anything encrypted, so this is the common case
// rather than an edge one.
func TestAnEnvelopeWithNoPlaintextsIsWellFormed(t *testing.T) {
	got, err := Envelope{Frame: []byte("f")}.Encode()
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	want := []byte{
		0x01, 0x00,
		0x00, 0x00, // flags = inbound, original
		0x01, 0x00, 0x00, 0x00,
		'f',
		0x00, 0x00, // pt_count = 0
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("got %x, want %x", got, want)
	}
}

// Both flags travel, and they are independent bits.
func TestFlagsArePackedIndependently(t *testing.T) {
	for _, testCase := range []struct {
		name      string
		direction Direction
		origin    FrameOrigin
		want      byte
	}{
		{"inbound original", Inbound, Original, 0b00},
		{"outbound original", Outbound, Original, 0b01},
		{"inbound re-encoded", Inbound, ReEncoded, 0b10},
		{"outbound re-encoded", Outbound, ReEncoded, 0b11},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			got, err := Envelope{
				Direction:   testCase.direction,
				FrameOrigin: testCase.origin,
			}.Encode()
			if err != nil {
				t.Fatalf("encode: %v", err)
			}
			if got[2] != testCase.want {
				t.Fatalf("flags = %#b, want %#b", got[2], testCase.want)
			}
		})
	}
}

// A payload on an entry that did not decrypt is refused rather than written.
//
// The contract says a non-Ok entry carries nothing, and a reader is entitled
// to skip its bytes — so writing some would put data where nobody looks. Both
// sides enforce it, because each is the other's only guard: a producer in
// another language does not run the Rust encoder.
func TestOnlyAnOkEntryMayCarryAPayload(t *testing.T) {
	_, err := Envelope{Plaintexts: []Plaintext{{
		Status:  StatusDecryptFailed,
		Payload: []byte("something"),
	}}}.Encode()
	if !errors.Is(err, ErrPayloadWithoutOk) {
		t.Fatalf("err = %v, want ErrPayloadWithoutOk", err)
	}

	// And an empty one is fine: that is how a failure is reported.
	if _, err := (Envelope{Plaintexts: []Plaintext{{
		Status: StatusDecryptFailed,
	}}}).Encode(); err != nil {
		t.Fatalf("a failure with no payload must encode: %v", err)
	}
}

// A path the length prefix cannot count is refused.
func TestAPathTooDeepToCountIsRefused(t *testing.T) {
	_, err := Envelope{Plaintexts: []Plaintext{{
		Path:   make([]uint16, MaxPathDepth+1),
		Status: StatusUnobserved,
	}}}.Encode()
	if !errors.Is(err, ErrPathTooDeep) {
		t.Fatalf("err = %v, want ErrPathTooDeep", err)
	}
}

// The reserved length is the length written, so encoding never grows the slice.
//
// Not a performance assertion so much as a check that the two calculations
// agree: one that drifted would still produce correct bytes, and the next
// reader would trust the wrong one.
func TestTheReservedLengthIsTheWrittenLength(t *testing.T) {
	envelope := Envelope{
		Direction: Outbound,
		Frame:     []byte("a longer frame"),
		Plaintexts: []Plaintext{
			{Path: []uint16{0}, Status: StatusOk, Payload: []byte("one")},
			{Path: []uint16{1, 2, 3}, Status: StatusUnobserved},
		},
	}
	got, err := envelope.Encode()
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	if len(got) != envelope.encodedLen() {
		t.Fatalf("wrote %d bytes, reserved %d", len(got), envelope.encodedLen())
	}
	if cap(got) != envelope.encodedLen() {
		t.Fatalf("capacity %d, reserved %d — the slice grew", cap(got), envelope.encodedLen())
	}
}
