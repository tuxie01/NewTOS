#!/bin/sh
# newtos-tables.sh <before|after> <scratch> <runs-dir> [<head-tree>] -- BIOS04: the four
# no-regression tables of TASK/BIOS04 on NewTOS (QEMU raspi4b, kernel rpi4 release) through
# the TOOL09 runner, each against its tt and falcontc oracle references; copied unchanged
# but for this header from measurement-2026-10-29-xbios02/method/newtos-tables.sh.
#   after   this tree (with the BIOS04 change)                  -> <runs>/newtos-<table>
#   before  <head-tree> = `git archive HEAD` (679c505e) extracted
#           into scratch (no .git), passed as --tree             -> <runs>/before-<table>
set -u
W=$1; S=$2; R=$3; T=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$R"; R=$(cd "$R" && pwd)
cd "$REPO"
mkdir -p "$S/xtask-$W"
REF=tosdump/reference
case $W in
  after)  PFX=newtos; TREE= ;;
  before) PFX=before; [ -n "$T" ] || { echo "before needs <head-tree>" >&2; exit 2; }; TREE="--tree $T" ;;
  *) echo "before|after" >&2; exit 2 ;;
esac
run() { # run <name> <table> <tt ref> <falcontc ref>
    # shellcheck disable=SC2086
    cargo xtask tosdump bios "tosdump/cases/$2" $TREE --reference "$REF/$3" --reference "$REF/$4" \
        --out "$R/$PFX-$1" --scratch "$S/xtask-$W" > "$S/$PFX-$1.stdout" 2>&1
    echo "$PFX-$1: cargo xtask tosdump exit $?"
}
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN           EMUTOS-1a7484ab-BIOS-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN  EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN      EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN   EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
