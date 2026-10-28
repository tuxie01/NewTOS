#!/bin/sh
# newtos-runs.sh <scratch> <runs-dir> -- the closing tables of TASK/TOOL07 on
# NewTOS, QEMU raspi4b, one run each, through the runner of TOOL09:
#   cargo xtask tosdump bios tosdump/cases/bios-r2-<t>.tsv --disk \
#       --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-<t>-disk.BIN \
#       --out <runs-dir>/newtos-<t> --scratch <scratch>/xtask
# --disk builds the test image with tools/mktestdisk.sh and the autorun INF
# for D:\DUMPBIOS.NAP (the same bytes as oracle-runs.sh: sha256 d95a60c3...)
# and the card from it with tools/mkcard.sh MKCARD_TESTDISK, for this run.
# A destructive table leaves <out>/testpart-after.img; it is moved to
# <scratch>/end/newtos-<t>.img for compare-ends.sh.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/end" "$S/xtask"
for T in phys x004 x005 x006; do
    cargo xtask tosdump bios "tosdump/cases/bios-r2-$T.tsv" --disk \
        --reference "tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-$T-disk.BIN" \
        --out "$R/newtos-$T" --scratch "$S/xtask" > "$S/newtos-$T.stdout" 2>&1
    echo "newtos-$T: cargo xtask tosdump exit $?"
    [ -f "$R/newtos-$T/testpart-after.img" ] && mv "$R/newtos-$T/testpart-after.img" "$S/end/newtos-$T.img"
done
