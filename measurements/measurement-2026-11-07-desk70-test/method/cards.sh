#!/bin/bash
# cards.sh <scratch> -- the two QEMU base cards of this run (tools/mkcard.sh, manifest beside each image).
#   auto.img : C:\AUTO\CORE.NCP + C:\AUTO\COREVAL.NCP (both run at boot, as SD_COREMARK_AUTO=1 of
#              tools/make-sd-image.sh places them) and the same two in C:\COREMARK\ (where make-sd-image installs them)
#   desk.img : C:\COREMARK\CORE.NCP + COREVAL.NCP only, no AUTO folder; started by a double-click
# Both: NEWDESK.INF of this method dir (= measurements/measurement-2026-11-07-desk70/method/NEWDESK.INF with the first
# window on C:\COREMARK\*.* instead of C:\*.*). The binaries are sdk/apps/coremark/build/*.NCP as built by
# sdk/apps/build-coremark.sh from the working tree of this run (sha256 in ../RUNINFO.TXT).
set -eu
S=$1; M=$(cd "$(dirname "$0")" && pwd); W=/Volumes/Daten/Development/PiRusTOS; B=$W/sdk/apps/coremark/build
MKCARD_LABEL=DESK70B "$W/tools/mkcard.sh" "$S/cards/auto.img" "$M/NEWDESK.INF" \
  AUTO/CORE.NCP="$B/CORE.NCP" AUTO/COREVAL.NCP="$B/COREVAL.NCP" COREMARK/CORE.NCP="$B/CORE.NCP" COREMARK/COREVAL.NCP="$B/COREVAL.NCP"
MKCARD_LABEL=DESK70B "$W/tools/mkcard.sh" "$S/cards/desk.img" "$M/NEWDESK.INF" \
  COREMARK/CORE.NCP="$B/CORE.NCP" COREMARK/COREVAL.NCP="$B/COREVAL.NCP"
