#!/bin/sh
# run-v3.sh <out-root> -- re-record every EMUTOS-1a7484ab-* reference in
# record format 3 on the pinned oracle (tools/hatari-oracle.sh, default ROM =
# the pin), one Hatari session per table, one after another.
#
# Dumpers: ../dumpers/<id>/, the v3 source (method/dumper-source-v3.diff on
# top of the HEAD named in RUNINFO.TXT), each built with the 4-byte build id
# the reference already carries -- exactly as the oracle run did
# (measurements/measurement-2026-10-26-oracle/method/run-set.sh), so the
# header differs from the v2 reference only in what format 3 changes.
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
VDI                    tt        DUMPVDI.PRG   vdi-r1.tsv              1cd6
VDI-r2                 tt        DUMPVDI.PRG   vdi-r2.tsv              1cd6
VDI-r3                 tt        DUMPVDI.PRG   vdi-r3.tsv              8be9
VDI-all                tt        DUMPVDI.PRG   vdi-all.tsv             19b7
VDI-all-falcontc       falcontc  DUMPVDI.PRG   vdi-all.tsv             19b7
AES                    tt        DUMPAES.PRG   aes-r1.tsv              b59e
AES-r2wind             tt        DUMPAES.PRG   aes-r2-wind.tsv         c911
AES-r2brlist           tt        DUMPAES.PRG   aes-r2b-rlist.tsv       c911
AES-r3                 tt        DUMPAES.PRG   aes-r3-wind-state.tsv   c911
AES-m05                tt        DUMPAES.PRG   aes-m05-addrin.tsv      70ca
AES-m06                tt        DUMPAES.PRG   aes-m06-sysvar.tsv      70ca
AES-m07                tt        DUMPAES.PRG   aes-m07-parameter.tsv   70ca
RASTER                 tt        DUMPRAST.PRG  vdi-raster.tsv          fdf8
RASTER-screen-falcontc falcontc  DUMPRAST.PRG  vdi-raster-screen.tsv   c1f4
TABLES
