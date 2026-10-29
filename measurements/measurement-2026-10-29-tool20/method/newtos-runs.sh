#!/bin/sh
# newtos-runs.sh <scratch> <runs-dir> -- the three gemdos-mc tables of
# TASK/TOOL20 on NewTOS, QEMU raspi4b, one run each, through the runner of
# TOOL09:
#   cargo xtask tosdump bios tosdump/cases/gemdos-mc-<t>.tsv \
#       --testdisk <scratch>/test.img \
#       --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-mc-<t>-disk.BIN \
#       --out <runs-dir>/newtos-<t> --scratch <scratch>/xtask
# --testdisk: the image of method/mkimage.sh, the one the oracle runs used
# (its C:\NEWDESK.INF autostarts D:\DUMPBIOS.NAP); the card is built from it
# for each run (tools/mkcard.sh MKCARD_TESTDISK, D:\DISKUNIT.TXT 26,
# D:\PROGEXT.TXT NAP) and booted as an APFS clone.  The destructive table's
# <out>/testpart-after.img is moved to <scratch>/end/newtos-<t>.img for
# compare-ends.sh; decoded.txt = method/decode-mc.py of the BIN.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/end" "$S/xtask"
for T in fs20 fs22 paths; do
    cargo xtask tosdump bios "tosdump/cases/gemdos-mc-$T.tsv" --testdisk "$S/test.img" \
        --reference "tosdump/reference/EMUTOS-1a7484ab-BIOS-mc-$T-disk.BIN" \
        --out "$R/newtos-$T" --scratch "$S/xtask" > "$S/newtos-$T.stdout" 2>&1
    echo "newtos-$T: cargo xtask tosdump exit $?"
    [ -f "$R/newtos-$T/testpart-after.img" ] && mv "$R/newtos-$T/testpart-after.img" "$S/end/newtos-$T.img"
    [ -f "$R/newtos-$T/BIOS.BIN" ] && python3 "$HERE/decode-mc.py" "$R/newtos-$T/BIOS.BIN" \
        "tosdump/cases/gemdos-mc-$T.tsv" > "$R/newtos-$T/decoded.txt"
done
