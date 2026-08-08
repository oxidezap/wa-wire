// Command emit-fixtures writes envelopes for the Rust side to read back.
//
// The boundary format is described three times — in `wa-wire-contract`, in the
// zapo adapter, and here — because an adapter runs inside its engine and the
// engines are in three languages. Three descriptions only ever tested
// separately are three formats waiting to diverge.
//
// The files are committed and CI regenerates them, so a change to this encoder
// shows up as a diff here rather than as traffic nobody can read.
//
//	go run ./cmd/emit-fixtures
package main

import (
	"fmt"
	"os"
	"path/filepath"

	wawire "github.com/oxidezap/wa-wire/adapters/hypermeow"
)

func main() {
	dir := "fixtures"
	if len(os.Args) > 1 {
		dir = os.Args[1]
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	// Each one exercises something the layout has to get right, and each is
	// read by a named test on the Rust side rather than by a loop — a fixture
	// nobody asserts anything about is a file, not a test.
	fixtures := map[string]wawire.Envelope{
		// The common case: a stanza with nothing encrypted.
		"receipt": {
			Frame: []byte{0xf8, 0x03, 0x2c, 0xfc, 0x04, 't', 'e', 's', 't'},
		},
		// A frame of no bytes at all, which is legal and has caught an
		// off-by-one in a length prefix before.
		"empty-frame": {},
		// The direction bit, on a frame that crosses verbatim.
		"outbound-verbatim": {
			Direction: wawire.Outbound,
			Frame:     []byte{0x01, 0x02, 0x03},
		},
		// A payload addressed to a child that is not the first, which is what
		// a real message looks like.
		"message-with-enc": {
			Frame: []byte{0xf8, 0x02, 0x2f, 0xf8, 0x01},
			Plaintexts: []wawire.Plaintext{{
				Path:    []uint16{1},
				Status:  wawire.StatusOk,
				Payload: []byte("decrypted"),
			}},
		},
		// Several payloads, and one node that produced nothing — the shape a
		// fan-out message leaves behind.
		"multi-device-with-plaintexts": {
			Frame: []byte{0xf8, 0x04},
			Plaintexts: []wawire.Plaintext{
				{Path: []uint16{1}, Status: wawire.StatusOk, Payload: []byte("first")},
				{Path: []uint16{2}, Status: wawire.StatusOk, Payload: []byte("second")},
				{Path: []uint16{3}, Status: wawire.StatusUnobserved},
			},
		},
		// A path with no components: the root node itself decrypted. Legal,
		// and the one case where a reader might expect at least one index.
		"root-path-plaintext": {
			Frame: []byte{0xf8, 0x01},
			Plaintexts: []wawire.Plaintext{{
				Path:    nil,
				Status:  wawire.StatusOk,
				Payload: []byte("root"),
			}},
		},
		// A path deep enough to need more than one component, so a reader that
		// stopped after the first is caught.
		"nested-path-plaintext": {
			Frame: []byte{0xf8, 0x01},
			Plaintexts: []wawire.Plaintext{{
				Path:    []uint16{2, 0, 7},
				Status:  wawire.StatusOk,
				Payload: []byte("nested"),
			}},
		},
		// A failure, which carries no payload by contract — the case a reader
		// must not try to read bytes for.
		"decrypt-failed": {
			Frame: []byte{0xf8, 0x01},
			Plaintexts: []wawire.Plaintext{{
				Path:   []uint16{0},
				Status: wawire.StatusDecryptFailed,
			}},
		},
	}

	for name, envelope := range fixtures {
		encoded, err := envelope.Encode()
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", name, err)
			os.Exit(1)
		}
		path := filepath.Join(dir, name+".bin")
		if err := os.WriteFile(path, encoded, 0o644); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		fmt.Printf("  %s (%d bytes)\n", path, len(encoded))
	}
}
