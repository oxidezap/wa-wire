#!/usr/bin/env bash
#
# The whole move, end to end: whatsapp-rust -> zapo -> whatsapp-rust.
#
# One session, three legs, one pairing. The account pairs in the first leg and
# must never pair again; everything else this runs is there to make that claim
# checkable rather than plausible.
#
#   ./run-cycle.sh [port] [seconds-per-leg]
#
# Needs `barback` (from whatsapp-bench's adapter cache) on PATH or in BARBACK,
# and `npm install` already done here.

set -euo pipefail

PORT="${1:-46010}"
SECONDS_PER_LEG="${2:-6}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="${WORK:-$(mktemp -d)}"
RUST_ADAPTER="$HERE/../../adapters/whatsapp-rust"
URL="ws://127.0.0.1:$PORT/ws/chat"

BARBACK="${BARBACK:-$(command -v barback || ls -t "$HOME"/.cache/wabench/entries/*/artifact/barback 2>/dev/null | head -1)}"
if [ ! -x "$BARBACK" ]; then
    echo "no barback: set BARBACK to the mock server binary" >&2
    exit 1
fi

echo "work: $WORK"
"$BARBACK" --host 127.0.0.1 --port "$PORT" --no-tls --pairing-delay 1 > "$WORK/server.log" 2>&1 &
SERVER=$!
trap 'kill "$SERVER" 2>/dev/null || true' EXIT

# The server is ready when it says so. Polling the port would race the moment
# between bind and the websocket route being live.
for _ in $(seq 1 30); do
    grep -q "Server listening" "$WORK/server.log" && break
    sleep 1
done

capture() {
    # $1 store, $2 recording, $3 pair-post url (empty for a leg that must not pair)
    ( cd "$RUST_ADAPTER" && \
      WA_WIRE_CAPTURE_URL="$URL" \
      WA_WIRE_CAPTURE_OUT="$WORK/frames" \
      WA_WIRE_CAPTURE_SECONDS="$SECONDS_PER_LEG" \
      WA_WIRE_CAPTURE_STORE="$1" \
      WA_WIRE_CAPTURE_RECORDING="$2" \
      WA_WIRE_CAPTURE_PAIR_POST="$3" \
      WA_WIRE_CAPTURE_VERSION=2.3000.1027934701 \
      cargo run --quiet --example capture-corpus --features insecure-capture )
}

echo "== leg A — whatsapp-rust pairs and holds the session"
capture "$WORK/A.db" "$WORK/A.wawr" "http://127.0.0.1:$PORT/admin/mock-phone/scan-qr"
python3 "$HERE/dump-rust-store.py" "$WORK/A.db" > "$WORK/A.json"

echo
echo "== leg B — zapo takes it over"
node "$HERE/attach-zapo.mjs" "$WORK/A.json" "$URL" "$WORK/B.wawr" "$WORK/B.json" "$SECONDS_PER_LEG"

echo
echo "== back to whatsapp-rust"
node "$HERE/back-to-rust.mjs" "$WORK/B.json" "$WORK/C.json"
python3 "$HERE/write-rust-store.py" "$WORK/A.db" "$WORK/C.db" "$WORK/C.json"

echo
echo "== leg C — whatsapp-rust picks it back up"
# No pair-post url. A leg that needed to pair would have nowhere to send the
# code, so it would hang and record nothing — which is the assertion, made by
# leaving the means out rather than by checking afterwards.
capture "$WORK/C.db" "$WORK/C.wawr" ""

echo
echo "== continuity"
node "$HERE/continuity.mjs" "$WORK/server.log" "$WORK/A.wawr" "$WORK/B.wawr" "$WORK/C.wawr"
