#!/usr/bin/env bash
# mkcards.sh <scratch> -- the two base cards of this run, both via tools/mkcard.sh (64 MiB MBR+FAT16), TeraDesk by #Z
# autorun (z-teradesk.inf, as ARCH06), no AUTO program, so _VDO is absent and TeraDesk takes the ST branch:
#   before: <scratch>/bin/td-before (TeraDesk built without the SDK07 overlay, method/build-before.sh)
#   after:  <scratch>/bin/td-after  (sdk/apps/build-teradesk.sh with sdk/overlays/teradesk/video.c + nohard.h)
# The resource files are the same in both (mkrsc of the vendored .rsc).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; S="$1"; B="$S/bin"
mkdir -p "$S/cards"
for side in before after; do
  (cd "$WS" && tools/mkcard.sh "$S/cards/$side.img" "$HERE/z-teradesk.inf" \
     Apps/TERADESK/DESKTOP.NAP="$B/td-$side/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$B/td-$side/desktop.rsc" \
     Apps/TERADESK/icons.rsc="$B/td-$side/icons.rsc" Apps/TERADESK/cicons.rsc="$B/td-$side/cicons.rsc" \
     Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF")
done
