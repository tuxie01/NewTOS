#!/bin/sh
# run-oracle.sh <out-root> -- the EmuTOS reference of bios-r1.tsv on the
# pinned oracle (tools/hatari-oracle.sh, default ROM = the pin), TT profile
# like every other area.  Dumper: ../dumpers/a8b7/DUMPBIOS.PRG, built from
# HEAD a8b75f88 plus method/dumper-source.diff.
set -u
OUTROOT=$1
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
mkdir -p "$OUTROOT"
"$REPO/tools/hatari-oracle.sh" tt "$RUN/dumpers/a8b7/DUMPBIOS.PRG" \
    "$REPO/tosdump/cases/bios-r1.tsv" "$OUTROOT/EMUTOS-tt" > "$OUTROOT/EMUTOS-tt.stdout" 2>&1
echo "EMUTOS-tt  exit $?"
# The same table on the Falcon030 truecolor profile: many XBIOS answers
# depend on the machine (TT shifter, Videl, DSP, DMA sound, palette), and
# NewTOS's target profile is the Falcon030/TOS4 one.
"$REPO/tools/hatari-oracle.sh" falcontc "$RUN/dumpers/a8b7/DUMPBIOS.PRG" \
    "$REPO/tosdump/cases/bios-r1.tsv" "$OUTROOT/EMUTOS-falcontc" > "$OUTROOT/EMUTOS-falcontc.stdout" 2>&1
echo "EMUTOS-falcontc  exit $?"
