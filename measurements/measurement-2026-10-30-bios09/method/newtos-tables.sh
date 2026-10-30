#!/bin/sh
# newtos-tables.sh before|after <scratch> <runs-dir> [<tree>] -- BIOS09: the
# no-regression tables on NewTOS (QEMU raspi4b, kernel rpi4 release) through the
# TOOL09 runner, each against its tt and falcontc reference. "before" = <tree>
# (`git archive f065a705` extracted into scratch, no .git, passed as --tree);
# "after" = this tree with BIOS09.
# (copied from measurement-2026-10-30-bios08/method/newtos-tables.sh; changes:
# the header, the four BIOS09 tables -- xbios-bconmap-r1 dropped.)
set -u
SIDE=$1; S=$2; R=$3; T=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask-$SIDE"
REF=tosdump/reference
run() { # run <name> <table> <tt ref> <falcontc ref>
    if [ -n "$T" ]; then set -- "$@" --tree "$T"; fi
    n=$1; t=$2; a=$3; b=$4; shift 4
    cargo xtask tosdump bios "tosdump/cases/$t" "$@" --reference "$REF/$a" --reference "$REF/$b" \
        --out "$R/$SIDE-$n" --scratch "$S/xtask-$SIDE" > "$S/$SIDE-$n.stdout" 2>&1
    echo "$SIDE-$n: cargo xtask tosdump exit $?"
}
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN           EMUTOS-1a7484ab-BIOS-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN      EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN  EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN   EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
