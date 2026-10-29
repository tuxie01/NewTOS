#!/usr/bin/env bash
# mkcards.sh <scratch> -- the two interactive QEMU cards of this run (never a real card), tools/mkcard.sh, the
# shapes of measurements/measurement-2026-10-27-mem12-acceptance/method/mkcards.sh (td, doom); manifests to <run>/cards/.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"
C="$1/cards"; mkdir -p "$C" "$R/cards"; cd "$WS"
TD=sdk/apps/teradesk/build/dist
tools/mkcard.sh "$C/td.img" "$HERE/z-teradesk.inf" Apps/TERADESK/DESKTOP.NAP=$TD/DESKTOP.NAP Apps/TERADESK/desktop.rsc=$TD/desktop.rsc \
  Apps/TERADESK/icons.rsc=$TD/icons.rsc Apps/TERADESK/cicons.rsc=$TD/cicons.rsc Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF" >/dev/null
tools/mkcard.sh "$C/doom.img" "$HERE/z-doom.inf" DOOM/DOOM.NAP=sdk/apps/doom/build/DOOM.NAP DOOM/DOOM1.WAD=Applications/doom-wad/doom1.wad >/dev/null
for i in td doom; do cp "$C/$i.img.manifest" "$R/cards/$i.img.manifest"; done
(cd "$C" && shasum -a 256 td.img doom.img)
