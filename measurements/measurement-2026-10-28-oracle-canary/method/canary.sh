#!/bin/sh
# Oracle canary (docs/analysis/ORACLE.md section 6): rebuild DUMPVDI with the
# reference's build id, run vdi-all.tsv on the pinned ROM in both Hatari
# profiles, compare BIN and LOG byte for byte with the pinned references.
#   sh canary.sh <empty-scratch-dir>      (run from the repository root)
set -eu
S=$1
[ ! -e "$S" ] || { echo "scratch dir exists: $S" >&2; exit 2; }
mkdir -p "$S"
make -s -C tosdump BUILD="$S/dumpers/19b7" BUILD_ID=19b7 dumpvdi-m68k
R=tosdump/reference/EMUTOS-1a7484ab-VDI-all
rc=0
for p in tt falcontc; do
    tools/hatari-oracle.sh "$p" "$S/dumpers/19b7/DUMPVDI.PRG" tosdump/cases/vdi-all.tsv "$S/$p" > "$S/$p.stdout" 2>&1
    case $p in tt) ref=$R ;; falcontc) ref=$R-falcontc ;; esac
    if cmp -s "$S/$p/VDI.BIN" "$ref.BIN" && cmp -s "$S/$p/VDI.LOG" "$ref.LOG"; then
        echo "canary $p: OK (BIN+LOG bit-identical to $(basename "$ref"))"
    else
        echo "canary $p: FAIL ($S/$p vs $(basename "$ref"))"; rc=1
    fi
done
exit $rc
