#!/bin/bash
# newtos-tables.sh <scratch> <runs-dir> <before|after> -- BIOS07: the four tables of the
# ticket on NewTOS (QEMU raspi4b, kernel rpi4 release, the TOOL09 runner `cargo xtask
# tosdump`) in the main tree, each against its tt and falcontc oracle reference.
# "before" ran on the clean tree at the ticket's start commit, "after" on the tree
# with the BIOS07 change.
set -u
S=$1; R=$2; TAG=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
REF=tosdump/reference
run() { # run <name> <table> <tt ref> <falcontc ref>
    mkdir -p "$S/xtask-$TAG"
    cargo xtask tosdump bios "tosdump/cases/$2" --reference "$REF/$3" --reference "$REF/$4" \
        --out "$R/$TAG-$1" --scratch "$S/xtask-$TAG" > "$S/$TAG-$1.stdout" 2>&1
    echo "$TAG-$1: cargo xtask tosdump exit $?"
}
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN          EMUTOS-1a7484ab-BIOS-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN     EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN  EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
