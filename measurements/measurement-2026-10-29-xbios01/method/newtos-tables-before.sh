#!/bin/sh
# newtos-tables-before.sh <head-tree> <scratch> <runs-dir> -- the same five tables as
# newtos-tables.sh, measured on the tree WITHOUT the XBIOS01 change: <head-tree> is
# `git archive 00b7c9cd` extracted into scratch (no .git), passed to the runner as --tree.
# The baseline for "no previously equal case newly differs".
set -u
T=$1; S=$2; R=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask"
REF=tosdump/reference
run() { # run <name> <table> <tt ref> <falcontc ref>
    cargo xtask tosdump bios "tosdump/cases/$2" --tree "$T" --reference "$REF/$3" --reference "$REF/$4" \
        --out "$R/before-$1" --scratch "$S/xtask" > "$S/before-$1.stdout" 2>&1
    echo "before-$1: cargo xtask tosdump exit $?"
}
run xbios-ptr-r1     xbios-ptr-r1.tsv     EMUTOS-1a7484ab-XBIOS-ptr.BIN      EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN           EMUTOS-1a7484ab-BIOS-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN  EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN      EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN   EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
