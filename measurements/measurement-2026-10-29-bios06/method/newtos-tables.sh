#!/bin/sh
# newtos-tables.sh <before|after> <scratch> <runs-dir> [<head-tree>] -- BIOS06: the
# BIOS/XBIOS tables on NewTOS (QEMU raspi4b, kernel rpi4 release) through the TOOL09
# runner, each against its tt and falcontc oracle references; after
# measurement-2026-10-29-xbios01/method/newtos-tables.sh and -before.sh.
#   after   this tree (with the BIOS06 change)              -> <runs>/newtos-<table>
#   before  <head-tree> = `git archive 90f409ab` extracted
#           into scratch (no .git), passed as --tree        -> <runs>/before-<table>
# Acceptance table xbios-ptr-r1 (case 34 = Getmpb); no-regression tables bios-r1,
# xbios-chardev-r1 (lead), bios-core-r1, xbios-system-r1 (TASK/BIOS06 "Abnahme").
set -u
W=$1; S=$2; R=$3; T=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
# <runs-dir> is taken relative to the caller, before the cd below (the run itself
# passed "runs" from the run directory, which the cd made the repository root's;
# the ten run directories were moved into runs/ afterwards, see RUNINFO.TXT).
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
run xbios-ptr-r1     xbios-ptr-r1.tsv     EMUTOS-1a7484ab-XBIOS-ptr.BIN      EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN           EMUTOS-1a7484ab-BIOS-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN  EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN      EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN   EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
