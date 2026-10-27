#!/usr/bin/env bash
# mkcards.sh -- the run's QEMU cards in $S/cards (never a real card), each with its <image>.manifest, copied to
# <run>/cards/. std = tools/make-sd-image.sh (all programs as rebuilt by build-programs.sh); the others tools/mkcard.sh:
#   td    TeraDesk #Z (z-teradesk.inf + TERADESK.INF)              super  DUMPBIOS.NAP #Z, super-r1.tsv as C:\BIOS.TSV
#   hv    HypView #Z, README.TXT as default document              api    AUTO\APITEST.NCP, NEWDESK.INF = gh1.inf
#   doom  DOOM #Z + DOOM1.WAD (Applications/doom-wad)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12/acc
C="$S/cards"; mkdir -p "$C" "$R/cards"; cd "$WS"
TD=sdk/apps/teradesk/build/dist; HV=sdk/apps/hypview/build
if [ -z "${ONLY_PROGRAM_CARDS:-}" ]; then
bash tools/make-sd-image.sh "$C/std.img" > "$R/logs/make-sd-image.log" 2>&1
tools/mkcard.sh "$C/td.img" "$HERE/z-teradesk.inf" Apps/TERADESK/DESKTOP.NAP=$TD/DESKTOP.NAP Apps/TERADESK/desktop.rsc=$TD/desktop.rsc \
  Apps/TERADESK/icons.rsc=$TD/icons.rsc Apps/TERADESK/cicons.rsc=$TD/cicons.rsc Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF" >/dev/null
tools/mkcard.sh "$C/hv.img" "$HERE/z-hypview.inf" APPS/HYPVIEW/HYP_VIEW.NAP=$HV/HYP_VIEW.NAP APPS/HYPVIEW/hyp_view.rsc=$HV/hyp_view.rsc \
  APPS/HYPVIEW/HYP_VIEW.CFG="$HERE/HYP_VIEW.CFG" APPS/HYPVIEW/README.TXT="$HERE/README.TXT" >/dev/null
tools/mkcard.sh "$C/doom.img" "$HERE/z-doom.inf" DOOM/DOOM.NAP=sdk/apps/doom/build/DOOM.NAP DOOM/DOOM1.WAD=Applications/doom-wad/doom1.wad >/dev/null
tools/mkcard.sh "$C/super.img" "$HERE/z-dumpbios.inf" DUMPBIOS.NAP=tosdump/build/DUMPBIOS.NAP BIOS.TSV=tosdump/cases/super-r1.tsv >/dev/null
tools/mkcard.sh "$C/api.img" "$HERE/gh1.inf" AUTO/APITEST.NCP=sdk/apps/apitest/APITEST.NCP >/dev/null
for i in std td hv doom super api; do cp "$C/$i.img.manifest" "$R/cards/$i.img.manifest"; done
(cd "$C" && shasum -a 256 *.img)
fi
# Program cards for the plain start test (added after the first six; same tool).
cd "$WS"
if [ -z "${P0_CARD:-}" ]; then
Z=sdk/zview/build/dist/ZVIEW
tools/mkcard.sh "$C/qed.img" "$HERE/z-qed.inf" Apps/QED/QED.NAP=sdk/apps/qed/build/QED.NAP Apps/QED/qed.rsc=sdk/apps/qed/build/qed.rsc >/dev/null
tools/mkcard.sh "$C/present.img" "$HERE/z-present.inf" Apps/PRESENT/PRESENT.NAP=sdk/apps/present/build/PRESENT.NAP \
  Apps/PRESENT/SLIDES.TXT=sdk/apps/present/sample/SLIDES.TXT Apps/PRESENT/TEST.PNG=sdk/apps/present/build/TEST.PNG >/dev/null
ZF=(ZVIEW/ZVIEW.NAP=$Z/ZVIEW.NAP ZVIEW/ZVIEW.RSC=$Z/ZVIEW.RSC)
for f in $Z/CODECS/*.NLD; do ZF+=(ZVIEW/CODECS/$(basename "$f")="$f"); done
for f in $Z/ICONS/*.GOD; do ZF+=(ZVIEW/ICONS/$(basename "$f")="$f"); done
tools/mkcard.sh "$C/zview.img" "$HERE/z-zview.inf" "${ZF[@]}" >/dev/null
tools/mkcard.sh "$C/vdidraw.img" "$HERE/z-vdidraw.inf" VDIDRAW.NAP=tosdump/build/VDIDRAW.NAP >/dev/null
for i in qed present zview vdidraw; do cp "$C/$i.img.manifest" "$R/cards/$i.img.manifest"; done
(cd "$C" && shasum -a 256 qed.img present.img zview.img vdidraw.img)
fi
# The MEM12 page-0 probe card (sdk/examples/p0par.c + p0kid.c, built here with sdk/build.sh; copies in <run>/dumpers/).
if [ -n "${P0_CARD:-}" ]; then
  B="$S/userbin"; mkdir -p "$B" "$R/dumpers"
  bash sdk/build.sh sdk/examples/p0par.c "$B/P0PAR.NAP" >/dev/null; bash sdk/build.sh sdk/examples/p0kid.c "$B/P0KID.NAP" >/dev/null
  cp "$B/P0PAR.NAP" "$B/P0KID.NAP" "$R/dumpers/"
  tools/mkcard.sh "$C/p0.img" "$HERE/z-p0par.inf" P0PAR.NAP="$B/P0PAR.NAP" P0KID.NAP="$B/P0KID.NAP" >/dev/null
  cp "$C/p0.img.manifest" "$R/cards/p0.img.manifest"; (cd "$C" && shasum -a 256 p0.img)
fi
