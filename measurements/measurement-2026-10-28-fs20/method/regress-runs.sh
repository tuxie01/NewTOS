#!/bin/sh
# regress-runs.sh <scratch> <runs-dir> <prefix> -- the BIOS tables on NewTOS
# (QEMU raspi4b) through the TOOL09 runner, this tree, one run each:
#   bios-r2.tsv and the closing tables bios-r2-{x001,x004,x005,x006,phys}.tsv
#   with --disk against their EMUTOS-1a7484ab-*-disk references, and
#   bios-r1.tsv against the tt and falcontc references.
# The method of measurements/measurement-2026-10-28-bios01-s2/method/newtos-runs.sh
# without --tree. A destructive table leaves <out>/testpart-after.img; it is
# moved to <scratch>/end/<prefix>-<t>.img (scratch, deleted after the run).
set -u
S=$1; R=$2; P=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/end" "$S/xtask"
for T in r2 x001 x004 x005 x006 phys; do
    case $T in r2) TSV=bios-r2.tsv; REF=EMUTOS-1a7484ab-BIOS-r2-disk.BIN ;;
               *)  TSV=bios-r2-$T.tsv; REF=EMUTOS-1a7484ab-BIOS-r2-$T-disk.BIN ;; esac
    cargo xtask tosdump bios "tosdump/cases/$TSV" --disk \
        --reference "tosdump/reference/$REF" \
        --out "$R/$P-$T" --scratch "$S/xtask" > "$S/$P-$T.stdout" 2>&1
    echo "$P-$T: cargo xtask tosdump exit $?"
    [ -f "$R/$P-$T/testpart-after.img" ] && mv "$R/$P-$T/testpart-after.img" "$S/end/$P-$T.img"
done
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS.BIN \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc.BIN \
    --out "$R/$P-r1" --scratch "$S/xtask" > "$S/$P-r1.stdout" 2>&1
echo "$P-r1: cargo xtask tosdump exit $?"
