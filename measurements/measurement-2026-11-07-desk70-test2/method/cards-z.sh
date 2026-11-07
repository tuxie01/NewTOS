#!/bin/bash
# cards.sh <scratch> -- the QEMU base card of this run (tools/mkcard.sh, manifest beside the image).
#   z.img : C:\COREMARK\CORE.NCP + COREVAL.NCP (where tools/make-sd-image.sh installs them), no AUTO folder;
#           NEWDESK.INF of this method dir = measurements/measurement-2026-11-07-desk70-build/method/NEWDESK.INF plus
#           the autorun line "#Z 00 C:\COREMARK\CORE.NCP@" after "#R 02" (00 = not GEM: character mode, like a .NCP).
# The binaries are sdk/apps/coremark/build/*.NCP as built by sdk/apps/build-coremark.sh from the working tree of this
# run (sha256 in ../RUNINFO.TXT).
set -eu
S=$1; M=$(cd "$(dirname "$0")" && pwd); W=/Volumes/Daten/Development/PiRusTOS; B=$W/sdk/apps/coremark/build
mkdir -p "$S/cards"
MKCARD_LABEL=DESK70Z "$W/tools/mkcard.sh" "$S/cards/z.img" "$M/NEWDESK.INF" \
  COREMARK/CORE.NCP="$B/CORE.NCP" COREMARK/COREVAL.NCP="$B/COREVAL.NCP"
