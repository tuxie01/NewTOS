#!/bin/sh
# newtos-tables.sh <scratch> <runs-dir> -- XBIOS05: the frozen tables of
# newtos-tables-before.sh plus the new xbios-bconmap-r1.tsv on NewTOS (QEMU raspi4b, kernel
# rpi4 release of this tree) through the TOOL09 runner, each against its tt and falcontc
# reference.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask-after"
REF=tosdump/reference
run() { # run <name> <table> <tt ref> <falcontc ref>
    cargo xtask tosdump bios "tosdump/cases/$2" --reference "$REF/$3" --reference "$REF/$4" \
        --out "$R/after-$1" --scratch "$S/xtask-after" > "$S/after-$1.stdout" 2>&1
    echo "after-$1: cargo xtask tosdump exit $?"
}
run xbios-bconmap-r1 xbios-bconmap-r1.tsv EMUTOS-1a7484ab-XBIOS-bconmap.BIN  EMUTOS-1a7484ab-XBIOS-bconmap-falcontc.BIN
run bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN           EMUTOS-1a7484ab-BIOS-falcontc.BIN
run xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN  EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
run bios-core-r1     bios-core-r1.tsv     EMUTOS-1a7484ab-BIOS-core.BIN      EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run xbios-system-r1  xbios-system-r1.tsv  EMUTOS-1a7484ab-XBIOS-system.BIN   EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
run xbios-ptr-r1     xbios-ptr-r1.tsv     EMUTOS-1a7484ab-XBIOS-ptr.BIN      EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN
