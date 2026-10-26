#!/bin/sh
# compare-raster-v3.sh <v2-dir> -- the RASTER replacement checks of this run.
#
#   1. exactcheck (method/exactcheck.c), per table: the format 2 reference
#      <v2-dir>/EMUTOS-1a7484ab-<table>.BIN against the format 3 recording
#      runs/<table>/RASTER.BIN -- byte-identical except the header and the
#      address cells the descriptors define        -> exactcheck/<table>.txt
#   2. tosdiff, each EmuTOS v3 recording against itself: the address check
#      and the kept check on that side              -> tosdiff/<table>-self.txt
#   3. tosdiff, the NewTOS v3 vdi-raster recording
#      (measurements/measurement-2026-10-26-newtos-v3/runs/RASTER/RASTER.BIN)
#      against itself and against the EmuTOS v3 RASTER
#                                                   -> tosdiff/newtos-*.txt
# Files are copied to distinct names first, so tosdiff's labels tell the two
# sides apart.  <v2-dir> = copies of the format 2 references as committed at
# HEAD before the replacement.  Needs tosdump/host/build/tosdiff built from
# the tree of this run (make -C tosdump/host build/tosdiff).
set -u
V2=$1
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
TOSDIFF=$REPO/tosdump/host/build/tosdiff
WORK=$(mktemp -d "${TMPDIR:-/tmp}/raster-v3.XXXXXX")
cc -std=c99 -Wall -Wextra -Werror -Wno-unused-parameter -I"$REPO/tosdump/core" \
    "$M/exactcheck.c" "$REPO/tosdump/core/record.c" -o "$WORK/exactcheck" || exit 2
mkdir -p "$RUN/exactcheck" "$RUN/tosdiff"
for T in RASTER:vdi-raster RASTER-screen-falcontc:vdi-raster-screen; do
    S=${T%%:*}; C=$REPO/tosdump/cases/${T##*:}.tsv
    cp "$V2/EMUTOS-1a7484ab-$S.BIN" "$WORK/EMUTOS-v2-$S.BIN"
    cp "$RUN/runs/$S/RASTER.BIN"    "$WORK/EMUTOS-v3-$S.BIN"
    ( cd "$WORK" && ./exactcheck "EMUTOS-v2-$S.BIN" "EMUTOS-v3-$S.BIN" ) \
        > "$RUN/exactcheck/$S.txt" 2>&1
    echo "exit $?" >> "$RUN/exactcheck/$S.txt"
    ( cd "$WORK" && "$TOSDIFF" "$C" "EMUTOS-v3-$S.BIN" "EMUTOS-v3-$S.BIN" ) \
        > "$RUN/tosdiff/$S-self.txt" 2>&1
    echo "exit $?" >> "$RUN/tosdiff/$S-self.txt"
done
C=$REPO/tosdump/cases/vdi-raster.tsv
cp "$REPO/measurements/measurement-2026-10-26-newtos-v3/runs/RASTER/RASTER.BIN" \
    "$WORK/NEWTOS-v3-RASTER.BIN"
( cd "$WORK" && "$TOSDIFF" "$C" NEWTOS-v3-RASTER.BIN NEWTOS-v3-RASTER.BIN ) \
    > "$RUN/tosdiff/newtos-self.txt" 2>&1
echo "exit $?" >> "$RUN/tosdiff/newtos-self.txt"
( cd "$WORK" && "$TOSDIFF" "$C" EMUTOS-v3-RASTER.BIN NEWTOS-v3-RASTER.BIN ) \
    > "$RUN/tosdiff/newtos-vs-emutos.txt" 2>&1
echo "exit $?" >> "$RUN/tosdiff/newtos-vs-emutos.txt"
rm -rf "$WORK"
