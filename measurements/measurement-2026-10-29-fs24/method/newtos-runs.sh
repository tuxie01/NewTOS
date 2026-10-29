#!/bin/sh
# newtos-runs.sh <scratch> <runs-dir> -- FS24: tosdump/cases/gemdos-mc-fs24.tsv
# on NewTOS (QEMU raspi4b, kernel rpi4 release, this tree) through the TOOL09
# runner (cargo xtask tosdump), against the pinned oracle reference
# EMUTOS-1a7484ab-BIOS-mc-fs24-disk.  C: is the test image <scratch>/test.img
# of oracle-runs.sh (TOOL20's mkimage.sh) as --testdisk; the card is built
# by tools/mkcard.sh inside the runner and booted as an APFS clone.  The test
# partition after the run goes to <scratch>/end/newtos-fs24.img.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$R"; R=$(cd "$R" && pwd)
cd "$REPO"
mkdir -p "$S/xtask" "$S/end"
cargo xtask tosdump bios tosdump/cases/gemdos-mc-fs24.tsv --testdisk "$S/test.img" \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-mc-fs24-disk.BIN \
    --out "$R/newtos-fs24" --scratch "$S/xtask" > "$S/newtos-fs24.stdout" 2>&1
echo "newtos-fs24: cargo xtask tosdump exit $?"
[ -f "$R/newtos-fs24/testpart-after.img" ] && mv "$R/newtos-fs24/testpart-after.img" "$S/end/newtos-fs24.img"
python3 "$REPO/measurements/measurement-2026-10-29-tool20/method/decode-mc.py" \
    "$R/newtos-fs24/BIOS.BIN" tosdump/cases/gemdos-mc-fs24.tsv > "$R/newtos-fs24/decoded.txt"
