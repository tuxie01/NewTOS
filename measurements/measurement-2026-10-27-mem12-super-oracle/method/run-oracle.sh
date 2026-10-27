#!/bin/sh
# run-oracle.sh <out-root> -- MEM12 checkpoint 2: the EmuTOS reference of super-r1.tsv on the pinned
# oracle (tools/hatari-oracle.sh, default ROM = the pin), TT and Falcon030 truecolor profiles, plus a
# compatibility run of the unchanged bios-r1.tsv with the new dumper (TT).  Dumper: ../dumpers/a7ff/
# DUMPBIOS.PRG, built from HEAD a7ffd707 plus method/dumper-source.diff.
set -u
OUTROOT=$1
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
P=$RUN/dumpers/a7ff/DUMPBIOS.PRG
mkdir -p "$OUTROOT"
for prof in tt falcontc; do
  "$REPO/tools/hatari-oracle.sh" $prof "$P" "$REPO/tosdump/cases/super-r1.tsv" "$OUTROOT/SUPER-$prof" > "$OUTROOT/SUPER-$prof.stdout" 2>&1
  echo "SUPER-$prof  exit $?"
done
"$REPO/tools/hatari-oracle.sh" tt "$P" "$REPO/tosdump/cases/bios-r1.tsv" "$OUTROOT/BIOS-r1-tt" > "$OUTROOT/BIOS-r1-tt.stdout" 2>&1
echo "BIOS-r1-tt  exit $?"
