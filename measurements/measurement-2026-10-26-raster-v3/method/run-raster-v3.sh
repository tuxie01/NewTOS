#!/bin/sh
# run-raster-v3.sh <out-root> -- record both RASTER tables in record format 3
# on the pinned oracle (tools/hatari-oracle.sh, default ROM = the pin), one
# Hatari session per table, one after another.
#
# Copied from measurements/measurement-2026-10-26-ref07-v3/method/run-v3.sh,
# reduced to the two RASTER rows.  Dumpers: ../dumpers/<id>/DUMPRAST.PRG,
# built from HEAD (RUNINFO.TXT) with the 4-byte build id the reference
# already carries:
#   make -C tosdump BUILD=<run>/dumpers/<id> BUILD_ID=<id> dumprast-m68k
set -u
OUTROOT=$1
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
mkdir -p "$OUTROOT"
# ref-suffix  profile  dumper  table  build-id
while read -r SUF PROF DUMP TABLE ID; do
    case $SUF in ''|\#*) continue ;; esac
    O=$OUTROOT/$SUF
    "$REPO/tools/hatari-oracle.sh" "$PROF" "$RUN/dumpers/$ID/$DUMP" \
        "$REPO/tosdump/cases/$TABLE" "$O" > "$O.stdout" 2>&1
    echo "$SUF  exit $?"
done <<'TABLES'
RASTER                 tt        DUMPRAST.PRG  vdi-raster.tsv          fdf8
RASTER-screen-falcontc falcontc  DUMPRAST.PRG  vdi-raster-screen.tsv   c1f4
TABLES
