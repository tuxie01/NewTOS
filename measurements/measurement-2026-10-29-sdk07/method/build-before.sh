#!/usr/bin/env bash
# build-before.sh <scratch> -- the "before" TeraDesk: sdk/apps/build-teradesk.sh run unchanged except for three paths
# (sed below): SDK = the working tree's sdk/ (libraries read, not rebuilt when current), OVL = a scratch copy of
# sdk/overlays/teradesk/ WITHOUT video.c and nohard.h (the SDK07 patch) and without tests/, BUILD = scratch.
# Result: <scratch>/bin/td-before/. Expected = the SDK04-C6 binary DESKTOP.NAP f763e7d9... (ARCH06 RUNINFO.TXT).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; S="$1"; X="$S/before"
rm -rf "$X"; mkdir -p "$X/ovl" "$S/bin/td-before"
cp -R "$WS/sdk/overlays/teradesk/." "$X/ovl/"; rm -rf "$X/ovl/video.c" "$X/ovl/nohard.h" "$X/ovl/tests"
sed -e "s#^SDK=.*#SDK=\"$WS/sdk\"#" -e "s#^OVL=.*#OVL=\"$X/ovl\"#" -e "s#^BUILD=.*#BUILD=\"$X/build\"#" \
  "$WS/sdk/apps/build-teradesk.sh" > "$X/build-teradesk-before.sh"
bash "$X/build-teradesk-before.sh" > "$S/logs/build-before.log" 2>&1
cp "$X/build/dist/"* "$S/bin/td-before/"; shasum -a 256 "$S/bin/td-before/"*
