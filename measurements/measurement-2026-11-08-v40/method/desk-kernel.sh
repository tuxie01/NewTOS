#!/bin/bash
# V40: an armed desktop kernel (tools/inject-arm.py desktop=<mouse>) from a prepared source tree -- V35's
# desk-kernel.sh (measurements/measurement-2026-11-07-v35/method/desk-kernel.sh) with one change: the copy
# leaves the tree's target/ out.
#   desk-kernel.sh <tree> <mouse.txt> <kernel-out>
set -euo pipefail
T=$1; MOUSE=$2; KOUT=$3
W=$(mktemp -d "${S:?}/dkk.XXXXXX"); trap 'rm -rf "$W"' EXIT
rsync -a --exclude=/target "$T/" "$W/"
python3 -I tools/inject-arm.py "$W" "desktop=$MOUSE" | tail -1
(cd "$W" && cargo xtask build --board rpi4 --release) > "$W.build.log" 2>&1 || { tail -30 "$W.build.log" >&2; exit 1; }
cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
rm -f "$W.build.log"
echo "kernel $(shasum -a 256 "$KOUT" | cut -d' ' -f1) $KOUT (tree $T, armed desktop, mouse $(shasum -a 256 "$MOUSE" | cut -d' ' -f1))"
