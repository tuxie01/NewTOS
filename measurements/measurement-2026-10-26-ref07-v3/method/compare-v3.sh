#!/bin/sh
# compare-v3.sh <runs-root> <v2-dir> -- for every re-recorded table: the
# sha256 of the PAYLOAD (everything after the 32-byte header) of the format 2
# reference and of the format 3 recording, equal yes/no; both headers in hex;
# the LOG diff; and two tosdiff runs on the real data: v3 against itself (the
# address check on every cell) and v3 against v2 (must be refused, exit 2).
#
# <v2-dir> holds the format 2 references as committed before this run
# (EMUTOS-1a7484ab-<suffix>.{BIN,LOG}); <runs-root>/<suffix>/ the new dumps.
set -u
RUNS=$1; V2=$2
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
TOSDIFF=$REPO/tosdump/host/build/tosdiff
payload() { tail -c +33 "$1" | shasum -a 256 | cut -d' ' -f1; }
printf '%-24s %-64s %-64s %s\n' table v2-payload-sha256 v3-payload-sha256 equal
while read -r SUF TABLE; do
    case $SUF in ''|\#*) continue ;; esac
    D=$RUNS/$SUF
    B=$(ls "$D"/*.BIN 2>/dev/null | head -1)
    OLD=$V2/EMUTOS-1a7484ab-$SUF.BIN
    if [ -z "$B" ]; then echo "$SUF  NO-DUMP"; continue; fi
    P2=$(payload "$OLD"); P3=$(payload "$B")
    [ "$P2" = "$P3" ] && EQ=yes || EQ=NO
    printf '%-24s %s %s %s\n' "$SUF" "$P2" "$P3" "$EQ"
    {
        echo "== $SUF"
        echo "v2 header $(xxd -l 32 -p "$OLD" | tr -d '\n')"
        echo "v3 header $(xxd -l 32 -p "$B" | tr -d '\n')"
        echo "-- LOG diff (v2 reference vs v3)"
        diff "$V2/EMUTOS-1a7484ab-$SUF.LOG" "${B%.BIN}.LOG"
        echo "-- tosdiff v3 vs v3 (address check)"
        "$TOSDIFF" "$REPO/tosdump/cases/$TABLE" "$B" "$B"; echo "exit $?"
        echo "-- tosdiff v2 vs v3 (must refuse)"
        "$TOSDIFF" "$REPO/tosdump/cases/$TABLE" "$OLD" "$B" 2>&1; echo "exit $?"
        echo "-- tosdiff v2 vs v2 (address check not checkable)"
        "$TOSDIFF" "$REPO/tosdump/cases/$TABLE" "$OLD" "$OLD" > "$D.v2v2.tmp" 2>&1
        RC=$?; tail -3 "$D.v2v2.tmp"; rm -f "$D.v2v2.tmp"; echo "exit $RC"
    } > "$D.detail.txt" 2>&1
done <<'TABLES'
VDI                    vdi-r1.tsv
VDI-r2                 vdi-r2.tsv
VDI-r3                 vdi-r3.tsv
VDI-all                vdi-all.tsv
VDI-all-falcontc       vdi-all.tsv
AES                    aes-r1.tsv
AES-r2wind             aes-r2-wind.tsv
AES-r2brlist           aes-r2b-rlist.tsv
AES-r3                 aes-r3-wind-state.tsv
AES-m05                aes-m05-addrin.tsv
AES-m06                aes-m06-sysvar.tsv
AES-m07                aes-m07-parameter.tsv
RASTER                 vdi-raster.tsv
RASTER-screen-falcontc vdi-raster-screen.tsv
TABLES
