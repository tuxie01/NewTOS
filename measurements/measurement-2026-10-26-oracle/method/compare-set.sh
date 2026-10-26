#!/bin/sh
# compare-set.sh <tag> <against> -- compare every dump of runs/<tag>/ with
# the same reference's dump in <against>, which is either "reference"
# (tosdump/reference/<ref>.BIN) or another tag under runs/. Writes
# runs/<tag>/COMPARE-vs-<against>.txt (one line per reference) and the
# per-reference tosdiff output next to the dump.
set -u
TAG=$1; AGAINST=$2
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
TOSDIFF=$REPO/tosdump/host/build/tosdiff
OUT=$RUN/runs/$TAG/COMPARE-vs-$AGAINST.txt
: > "$OUT"
while read -r REF TABLE; do
    D=$RUN/runs/$TAG/$REF
    B=$(ls "$D"/*.BIN 2>/dev/null | head -1)
    [ -n "$B" ] || { echo "$REF  NO-DUMP" >> "$OUT"; continue; }
    AREA=$(basename "$B" .BIN)
    if [ "$AGAINST" = reference ]; then
        OB=$REPO/tosdump/reference/$REF.BIN; OL=$REPO/tosdump/reference/$REF.LOG
    else
        OB=$RUN/runs/$AGAINST/$REF/$AREA.BIN; OL=$RUN/runs/$AGAINST/$REF/$AREA.LOG
    fi
    cmp -s "$B" "$OB" && BC=BIN-same || BC=BIN-DIFF
    cmp -s "$D/$AREA.LOG" "$OL" && LC=LOG-same || LC=LOG-DIFF
    "$TOSDIFF" "$REPO/tosdump/cases/$TABLE" "$OB" "$B" > "$D/DIFF-vs-$AGAINST.txt" 2>&1
    CASES=$(grep -E '^[0-9]+ of [0-9]+ cases differ' "$D/DIFF-vs-$AGAINST.txt" | head -1)
    echo "$REF  $BC  $LC  ${CASES:-tosdiff refused, see DIFF-vs-$AGAINST.txt}" >> "$OUT"
done <<'EOF'
EMUTOS14-VDI                    vdi-r1.tsv
EMUTOS14-VDI-r2                 vdi-r2.tsv
EMUTOS14-VDI-r3                 vdi-r3.tsv
EMUTOS14-VDI-all                vdi-all.tsv
EMUTOS14-VDI-all-falcontc       vdi-all.tsv
EMUTOS14-AES                    aes-r1.tsv
EMUTOS14-AES-r2wind             aes-r2-wind.tsv
EMUTOS14-AES-r2brlist           aes-r2b-rlist.tsv
EMUTOS14-AES-r3                 aes-r3-wind-state.tsv
EMUTOS14-AES-m05                aes-m05-addrin.tsv
EMUTOS14-AES-m06                aes-m06-sysvar.tsv
EMUTOS14-AES-m07                aes-m07-parameter.tsv
EMUTOS14-RASTER                 vdi-raster.tsv
EMUTOS14-RASTER-screen-falcontc vdi-raster-screen.tsv
EOF
cat "$OUT"
