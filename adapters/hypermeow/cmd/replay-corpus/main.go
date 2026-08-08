// Command replay-corpus runs the conformance corpus through this engine.
//
// The corpus is frames as an engine receives them: decompressed node bytes,
// without the format byte. Each is decoded by `hypermeow`'s own decoder and
// forwarded as this adapter would forward it, and the envelopes are written for
// the Rust side to compare against the other three engines'.
//
// Envelopes as files rather than a recording container. The container carries
// the claims a *gate* needs — which traffic, which adapter, whether the file is
// whole — and none of them is in question here: the comparison is driven from
// the Rust side, which builds a recording of its own around these. A container
// writer in Go is worth having and is not what this needs.
//
//	go run ./cmd/replay-corpus
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	wawire "github.com/oxidezap/wa-wire/adapters/hypermeow"
	"github.com/oxidezap/wa-wire/adapters/hypermeow/wire"
	waBinary "go.mau.fi/whatsmeow/binary"
)

func main() {
	root := "../../crates/wa-wire-conformance/corpus"
	out := "replay"
	if len(os.Args) > 2 {
		root, out = os.Args[1], os.Args[2]
	}

	frames, err := corpus(root)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if len(frames) == 0 {
		fmt.Fprintf(os.Stderr, "%s: no corpus frames\n", root)
		os.Exit(1)
	}

	if err := os.RemoveAll(out); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if err := os.MkdirAll(out, 0o755); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	written := 0
	for _, frame := range frames {
		// The engine's own decoder, which is the point: a frame this adapter
		// forwards is one `hypermeow` agreed was a stanza.
		node, err := waBinary.Unmarshal(frame.bytes)
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: the engine must decode it: %v\n", frame.name, err)
			os.Exit(1)
		}

		// And the engine's own *encoder*, written alongside.
		//
		// The adapter forwards verbatim, so its envelopes are the corpus bytes
		// and every zero-copy engine's are identical — which makes comparing
		// them a comparison of nothing. Re-encoding is where four engines
		// genuinely differ: each is entitled to write a value its own way, and
		// the property under test is that all four still derive the same event.
		reEncoded, err := waBinary.Marshal(*node)
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: the engine must re-encode it: %v\n", frame.name, err)
			os.Exit(1)
		}
		// Marshal writes the format byte the decoder strips.
		reEncoded = reEncoded[1:]

		// Forwarded as it stands, like the Rust replay: the frame path is what
		// this compares, and a plaintext table needs Signal to have run.
		envelope, err := wire.Envelope{Frame: frame.bytes}.Encode()
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", frame.name, err)
			os.Exit(1)
		}
		if violation := wawire.Info.Verify(wire.Envelope{Frame: frame.bytes}); violation != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", frame.name, violation)
			os.Exit(1)
		}

		name := fmt.Sprintf("%04d.envelope", written)
		if err := os.WriteFile(filepath.Join(out, name), envelope, 0o644); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		encoded, err := wire.Envelope{Frame: reEncoded, FrameOrigin: wire.ReEncoded}.Encode()
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", frame.name, err)
			os.Exit(1)
		}
		reName := fmt.Sprintf("%04d.reencoded", written)
		if err := os.WriteFile(filepath.Join(out, reName), encoded, 0o644); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		written++
	}

	fmt.Printf("%s: %d envelope(s) from %d corpus frame(s)\n", out, written, len(frames))
}

type frame struct {
	name  string
	bytes []byte
}

// corpus reads every `.bin` under root and its `captured/` subdirectory, in
// name order — the same order every replay uses, since the comparison aligns by
// position.
func corpus(root string) ([]frame, error) {
	var frames []frame
	for _, dir := range []string{root, filepath.Join(root, "captured")} {
		entries, err := os.ReadDir(dir)
		if err != nil {
			if dir == root {
				return nil, fmt.Errorf("%s: %w", dir, err)
			}
			continue
		}
		for _, entry := range entries {
			if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".bin") {
				continue
			}
			path := filepath.Join(dir, entry.Name())
			bytes, err := os.ReadFile(path)
			if err != nil {
				return nil, err
			}
			// Prefixed with the directory, so a captured frame and a written
			// one cannot collide on a name.
			frames = append(frames, frame{name: strings.TrimPrefix(path, root+"/"), bytes: bytes})
		}
	}
	sort.Slice(frames, func(i, j int) bool { return frames[i].name < frames[j].name })
	return frames, nil
}
