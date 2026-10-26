#!/usr/bin/env bash
# run-a118.sh <build|card|run <side> <n>> -- A118 probe: a file selector opened by a desk accessory (a118acc.c) while NEWDESK waits.
#   build: base (f42f3719) and new (working tree) release rpi4 kernels, injector armed on "desktop" with a118.script
#          (../a117/build-kernel.sh) -> $S/bin/kernel8-<side>-a118.img
#   card:  $S/cards/a118.img = ../a117/mkcard.sh with a118.inf (= a117.inf) + C:\ACC\A118ACC.NAC
#   run:   ../a117/run.py, shots: sel (selector up, after the double-click on the C: icon), after (2 s after Return)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/../a117/env.sh"
case "$1" in
  build)
    bash "$HERE/../a117/build-kernel.sh" f42f3719 base-a118 "$HERE/a118.script"; rm -rf "$S/trees/base-a118"
    bash "$HERE/../a117/build-kernel.sh" WORKTREE new-a118 "$HERE/a118.script"; rm -rf "$S/trees/new-a118";;
  card)
    mkdir -p "$S/userbin" "$S/cards"
    bash "$WS/sdk/build.sh" "$HERE/a118acc.c" "$S/userbin/A118ACC.NAC" >/dev/null
    bash "$HERE/../a117/mkcard.sh" "$S/cards/a118.img" "$HERE/a118.inf" ACC/A118ACC.NAC="$S/userbin/A118ACC.NAC" >/dev/null
    shasum -a 256 "$S/cards/a118.img" "$S/userbin/A118ACC.NAC";;
  run)
    SIDE=$2; N=$3
    python3 "$HERE/../a117/run.py" "$S/bin/kernel8-$SIDE-a118.img" "$S/cards/a118.img" "$SIDE-a118-$N" \
      "$HERE/../../serial" "$HERE/../../screendumps" 60 1640:1.5:sel 2510:2:after
    rm -rf "$S/cards/run-$SIDE-a118-$N";;
esac
