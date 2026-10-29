#!/usr/bin/env bash
# mkcards.sh <scratch> -- the two base cards of this run, both via tools/mkcard.sh (64 MiB MBR+FAT16):
#   novdo: C:\AUTO\MCHPROBE.NCP (reads _MCH, sets nothing) + TeraDesk (#Z autorun, z-teradesk.inf)
#   vdo:   C:\AUTO\MCHVDO.NCP   (the same probe built with -DSET_VDO=0x00030000L: Ssystem(S_SETCOOKIE,'_VDO')) + TeraDesk
# Probes: <scratch>/bin/{MCHPROBE,MCHVDO}.NCP (method/build-mchprobe.sh); TeraDesk dist: <scratch>/bin/td (sdk/apps/build-teradesk.sh).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; S="$1"; B="$S/bin"
mkdir -p "$S/cards"
for side in novdo vdo; do
  case $side in novdo) P=MCHPROBE.NCP;; vdo) P=MCHVDO.NCP;; esac
  (cd "$WS" && tools/mkcard.sh "$S/cards/$side.img" "$HERE/z-teradesk.inf" AUTO/$P="$B/$P" \
     Apps/TERADESK/DESKTOP.NAP="$B/td/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$B/td/desktop.rsc" \
     Apps/TERADESK/icons.rsc="$B/td/icons.rsc" Apps/TERADESK/cicons.rsc="$B/td/cicons.rsc" \
     Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF")
done
