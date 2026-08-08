module github.com/oxidezap/wa-wire/adapters/hypermeow

go 1.25.0

require go.mau.fi/whatsmeow v0.0.0

require (
	filippo.io/edwards25519 v1.2.0 // indirect
	github.com/coder/websocket v1.8.15 // indirect
	github.com/google/uuid v1.6.0 // indirect
	github.com/mattn/go-colorable v0.1.14 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	github.com/polymorfa/libsignal-protocol-go v0.2.3-0.20260806162910-a2adef2e8a11 // indirect
	github.com/rs/zerolog v1.35.1 // indirect
	go.mau.fi/util v0.9.12-0.20260717235539-f9ffa7eca58d // indirect
	golang.org/x/crypto v0.54.0 // indirect
	golang.org/x/net v0.57.0 // indirect
	golang.org/x/sync v0.22.0 // indirect
	golang.org/x/sys v0.47.0 // indirect
	golang.org/x/text v0.40.0 // indirect
	google.golang.org/protobuf v1.36.11 // indirect
)

// The engine, at the branch that exposes the frame bytes and the plaintexts.
//
// A local path rather than a version: polymorfa/hypermeow#5 is open, and the
// hooks this adapter is written against exist only there. Points at a release
// once it lands — the replace is what says plainly that this is not built
// against anything published.
replace go.mau.fi/whatsmeow => ../../../hypermeow
