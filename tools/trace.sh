#!/usr/bin/env bash
# Record where everything was, every frame, while the dock settled.
#
# The browser runs the page for real — no virtual clock, since under one the
# animation frames a trace is made of barely happen — makes a gesture, and
# posts what it saw to tools/collect.py. What comes out is JSON on stdout:
# one row per frame, each listing every pane, tab and gap and its rectangle.
#
#   tools/trace.sh <gesture> [ms to watch for]
#
# A gesture is steps separated by `|`, one every 100ms from 1000ms:
#   down:980,13|move:900,200|move:450,500|up:450,500
#
# Needs `trunk serve` already running. Nothing is left behind.

set -euo pipefail

gesture=${1:?a gesture}
watch=${2:-2600}
port=${PORT:-7150}
back=${BACK:-7151}
here=$(dirname "$0")

timeout 60 python3 "$here/collect.py" "$back" &
collector=$!

google-chrome-stable --headless --disable-gpu --hide-scrollbars \
    --window-size=1200,700 --remote-debugging-port=0 \
    "http://127.0.0.1:$port/?gesture=$gesture&trace=$watch&to=$back" \
    >/dev/null 2>&1 &
browser=$!

wait "$collector" || true
kill "$browser" 2>/dev/null || true
