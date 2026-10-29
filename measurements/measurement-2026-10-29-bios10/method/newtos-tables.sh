#!/bin/bash
# newtos-tables.sh <scratch> <runs-dir> <before|after> [<tree>] -- BIOS10: the four tables
# of the ticket on NewTOS (QEMU raspi4b, kernel rpi4 release, the TOOL09 runner
# `cargo xtask tosdump`), each against its tt and falcontc oracle reference.
# "before" runs on <tree> = `git archive 8cc34fa0` (the ticket's start commit, unpacked
# in scratch, references/ symlinked), "after" on the main tree with the BIOS10 change.
set -u
S=$1; R=$2; TAG=$3; TREE=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
REF=tosdump/reference
run() { # run <name> <table> <tt ref> <falcontc ref>
    tree=""; [ -z "$TREE" ] || tree="--tree $TREE"
    mkdir -p "$S/xtask-$TAG"
    # shellcheck disable=SC2086
    cargo xtask tosdump bios "tosdump/cases/$2" $tree --reference "$REF/$3" --reference "$REF/$4" \
        --out "$R/$TAG-$1" --scratch "$S/xtask-$TAG" > "$S/$TAG-$1.stdout" 2>&1
    echo "$TAG-$1: cargo xtask tosdump exit $?"
}
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN          EMUTOS-1a7484ab-BIOS-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN     EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN  EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
