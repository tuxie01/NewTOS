#!/bin/sh
# compare.sh <runs-dir> -- validate the three dumps and write the tosdiff
# reports.  tosdiff names a dump by its file name, so the dumps are copied
# into <runs-dir>/cmp/ under names that say which machine they are.
set -u
R=$1
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
T=$REPO/tosdump/cases/bios-r1.tsv
D=$REPO/tosdump/host/build/tosdiff     # make -C tosdump host-test builds it
mkdir -p "$R/cmp"
cp "$R/EMUTOS-tt/BIOS.BIN"            "$R/cmp/EMUTOS.BIN"
cp "$R/EMUTOS-falcontc/BIOS.BIN"      "$R/cmp/EMUTOS-FALCONTC.BIN"
cp "$R/NEWTOS-qemu-raspi4b/BIOS.BIN"  "$R/cmp/NEWTOS.BIN"
for f in EMUTOS EMUTOS-FALCONTC NEWTOS; do "$D" --validate "$R/cmp/$f.BIN" "$T"; done
cd "$R/cmp"
"$D" "$T" EMUTOS.BIN NEWTOS.BIN          > ../tosdiff-newtos-vs-emutos.txt;          echo "NewTOS vs EmuTOS tt:       exit $?"
"$D" "$T" EMUTOS-FALCONTC.BIN NEWTOS.BIN > ../tosdiff-newtos-vs-emutos-falcontc.txt; echo "NewTOS vs EmuTOS falcontc: exit $?"
"$D" "$T" EMUTOS.BIN EMUTOS-FALCONTC.BIN > ../tosdiff-emutos-tt-vs-falcontc.txt;    echo "EmuTOS tt vs falcontc:     exit $?"
