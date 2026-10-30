#!/bin/sh
# newtos-runs.sh <before|after> <scratch> <runs-dir> [<head-tree>] -- FS28: the four gemdos-mc
# tables (fs20, fs22, paths, fs24) and the canary bios-r1 on NewTOS (QEMU raspi4b, kernel rpi4
# release) through the TOOL09 runner (cargo xtask tosdump); after
# measurements/measurement-2026-10-29-fs23/method/newtos-runs.sh plus the fs24 table.
#   after   this tree (with the FS28 change)                         -> <runs>/newtos-<t>
#   before  <head-tree> = `git archive HEAD` extracted into scratch
#           (no .git), passed as --tree                              -> <runs>/before-<t>
# The gemdos-mc tables boot the test image of
# measurements/measurement-2026-10-29-tool20/method/mkimage.sh (<scratch>/test.img, called, not
# copied) as --testdisk; the card is built by tools/mkcard.sh inside the runner. The test
# partition after each destructive run is moved to <scratch>/end/<pfx>-<t>.img. bios-r1 runs
# against its tt and falcontc references, no disk. fs24 is decoded with TOOL20's decode-mc.py.
set -u
W=$1; S=$2; R=$3; T=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$R"; R=$(cd "$R" && pwd)
cd "$REPO"
mkdir -p "$S/xtask-$W" "$S/end"
REF=tosdump/reference
case $W in
  after)  PFX=newtos; TREE= ;;
  before) PFX=before; [ -n "$T" ] || { echo "before needs <head-tree>" >&2; exit 2; }; TREE="--tree $T" ;;
  *) echo "before|after" >&2; exit 2 ;;
esac
for t in fs20 fs22 paths fs24; do
    # shellcheck disable=SC2086
    cargo xtask tosdump bios "tosdump/cases/gemdos-mc-$t.tsv" $TREE --testdisk "$S/test.img" \
        --reference "$REF/EMUTOS-1a7484ab-BIOS-mc-$t-disk.BIN" \
        --out "$R/$PFX-$t" --scratch "$S/xtask-$W" > "$S/$PFX-$t.stdout" 2>&1
    echo "$PFX-$t: cargo xtask tosdump exit $?"
    [ -f "$R/$PFX-$t/testpart-after.img" ] && mv "$R/$PFX-$t/testpart-after.img" "$S/end/$PFX-$t.img"
done
python3 "$REPO/measurements/measurement-2026-10-29-tool20/method/decode-mc.py" \
    "$R/$PFX-fs24/BIOS.BIN" tosdump/cases/gemdos-mc-fs24.tsv > "$R/$PFX-fs24/decoded.txt"
# shellcheck disable=SC2086
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv $TREE \
    --reference "$REF/EMUTOS-1a7484ab-BIOS.BIN" --reference "$REF/EMUTOS-1a7484ab-BIOS-falcontc.BIN" \
    --out "$R/$PFX-bios-r1" --scratch "$S/xtask-$W" > "$S/$PFX-bios-r1.stdout" 2>&1
echo "$PFX-bios-r1: cargo xtask tosdump exit $?"
