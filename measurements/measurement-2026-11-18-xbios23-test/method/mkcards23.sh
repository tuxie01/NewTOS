#!/usr/bin/env bash
# mkcards23.sh <scratch> -- the two cards of this run, both via tools/mkcard.sh (64 MiB MBR+FAT16):
#   td:     TeraDesk only (z-teradesk.inf = #Z autorun, TERADESK.INF)
#   tdapps: td + QED + zView (+ codecs, icons, RSC) with desktop icons (TERADESK-apps.INF)
# Run from the main tree root (tools/mkcard.sh, sdk/ inputs); default TMPDIR (a data-volume TMPDIR breaks the mount).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="$1"; mkdir -p "$S/cards"
TD=sdk/apps/teradesk/build/dist; Z=sdk/zview/build/dist/ZVIEW; ZO=sdk/zview/build/out
TDP=(Apps/TERADESK/DESKTOP.NAP="$TD/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$TD/desktop.rsc" Apps/TERADESK/icons.rsc="$TD/icons.rsc" Apps/TERADESK/cicons.rsc="$TD/cicons.rsc")
tools/mkcard.sh "$S/cards/td.img" "$HERE/z-teradesk.inf" "${TDP[@]}" Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF"
APPS=(Apps/QED/QED.NAP=sdk/apps/qed/build/QED.NAP Apps/QED/qed.rsc=sdk/apps/qed/build/qed.rsc ZVIEW/ZVIEW.NAP=$ZO/ZVIEW.NAP ZVIEW/ZVIEW.RSC=$Z/ZVIEW.RSC)
for f in $ZO/CODECS/*.NLD; do APPS+=(ZVIEW/CODECS/$(basename $f)=$f); done
for f in $Z/ICONS/*.GOD; do APPS+=(ZVIEW/ICONS/$(basename $f)=$f); done
tools/mkcard.sh "$S/cards/tdapps.img" "$HERE/z-teradesk.inf" "${TDP[@]}" "${APPS[@]}" Apps/TERADESK/TERADESK.INF="$HERE/TERADESK-apps.INF"
