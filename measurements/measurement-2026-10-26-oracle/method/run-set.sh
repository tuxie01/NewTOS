#!/bin/sh
# run-set.sh <rom> <tag> -- run every EMUTOS14-* reference table on Hatari
# with <rom>, store the dumps under ../runs/<tag>/<ref>/, and compare each
# against tosdump/reference/<ref>.{BIN,LOG}: cmp (bytes) and tosdiff (cases).
#
# Each reference is rerun with the HEAD dumper source, built with the
# reference's own 4-byte build id (dumpers/<id>/), so the header is the same
# and the BIN can be compared byte for byte.
set -u
ROM=$1; TAG=$2
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
TOSDIFF=$REPO/tosdump/host/build/tosdiff
OUTS=$RUN/runs/$TAG
mkdir -p "$OUTS"
SUMMARY=$OUTS/SUMMARY.txt
: > "$SUMMARY"

# ref  profile  dumper  table  build-id
while read -r REF PROF DUMP TABLE ID; do
    case $REF in ''|\#*) continue ;; esac
    O=$OUTS/$REF
    "$REPO/tools/hatari-oracle.sh" "$PROF" "$RUN/dumpers/$ID/$DUMP" \
        "$REPO/tosdump/cases/$TABLE" "$O" "$ROM" > "$O.stdout" 2>&1
    AREA=$(sed -n 's/^\([A-Z]*\)\.BIN: .*/\1/p' "$O"/*.LOG 2>/dev/null | head -1)
    B=$O/$AREA.BIN; L=$O/$AREA.LOG
    if [ ! -f "$B" ]; then
        echo "$REF  NO-DUMP" | tee -a "$SUMMARY"; continue
    fi
    cmp -s "$B" "$REPO/tosdump/reference/$REF.BIN" && BC=BIN-same || BC=BIN-DIFF
    cmp -s "$L" "$REPO/tosdump/reference/$REF.LOG" && LC=LOG-same || LC=LOG-DIFF
    "$TOSDIFF" "$REPO/tosdump/cases/$TABLE" "$REPO/tosdump/reference/$REF.BIN" "$B" \
        > "$O/DIFF-vs-reference.txt" 2>&1
    CASES=$(grep -E '^[0-9]+ of [0-9]+ cases differ' "$O/DIFF-vs-reference.txt" | head -1)
    echo "$REF  $BC  $LC  ${CASES:-tosdiff: see DIFF-vs-reference.txt}" | tee -a "$SUMMARY"
done <<'EOF'
EMUTOS14-VDI                    tt        DUMPVDI.PRG   vdi-r1.tsv              1cd6
EMUTOS14-VDI-r2                 tt        DUMPVDI.PRG   vdi-r2.tsv              1cd6
EMUTOS14-VDI-r3                 tt        DUMPVDI.PRG   vdi-r3.tsv              8be9
EMUTOS14-VDI-all                tt        DUMPVDI.PRG   vdi-all.tsv             19b7
EMUTOS14-VDI-all-falcontc       falcontc  DUMPVDI.PRG   vdi-all.tsv             19b7
EMUTOS14-AES                    tt        DUMPAES.PRG   aes-r1.tsv              b59e
EMUTOS14-AES-r2wind             tt        DUMPAES.PRG   aes-r2-wind.tsv         c911
EMUTOS14-AES-r2brlist           tt        DUMPAES.PRG   aes-r2b-rlist.tsv       c911
EMUTOS14-AES-r3                 tt        DUMPAES.PRG   aes-r3-wind-state.tsv   c911
EMUTOS14-AES-m05                tt        DUMPAES.PRG   aes-m05-addrin.tsv      70ca
EMUTOS14-AES-m06                tt        DUMPAES.PRG   aes-m06-sysvar.tsv      70ca
EMUTOS14-AES-m07                tt        DUMPAES.PRG   aes-m07-parameter.tsv   70ca
EMUTOS14-RASTER                 tt        DUMPRAST.PRG  vdi-raster.tsv          fdf8
EMUTOS14-RASTER-screen-falcontc falcontc  DUMPRAST.PRG  vdi-raster-screen.tsv   c1f4
EOF
