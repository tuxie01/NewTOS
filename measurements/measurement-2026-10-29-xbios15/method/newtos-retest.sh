#!/bin/sh
# newtos-retest.sh <before-tree> <inj-before> <inj-after> <scratch> <runs-dir> -- XBIOS15:
# the ticket's retest tables on NewTOS (QEMU raspi4b, kernel rpi4 release) through the
# TOOL09 runner, each against its tt and falcontc oracle reference, on the tree without
# the change (<before-tree> = `git archive 18e8c115`, --tree) and on this tree (after):
# bios-r1, xbios-chardev-r1, xbios-ptr-r1; then xbios-flush-r1 with three keys from the
# IKBD injector (keys-ikbd.txt, copied from measurement-2026-10-29-xbios01/method):
# <inj-before> / <inj-after> are copies of the before / after tree (no .git, no target/)
# patched by `tools/inject-arm.py <tree> launch:DUMPBIOS=method/keys-ikbd.txt`.
# The baseline for "no previously equal case newly differs".
set -u
B=$1; IB=$2; IA=$3; S=$4; R=$5
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
REF=tosdump/reference
run() { # run <side> <tree|-> <name> <table> <tt ref> <falcontc ref>
    tree=""; [ "$2" = "-" ] || tree="--tree $2"
    mkdir -p "$S/xtask-$1"
    cargo xtask tosdump bios "tosdump/cases/$4" $tree --reference "$REF/$5" --reference "$REF/$6" \
        --out "$R/$1-$3" --scratch "$S/xtask-$1" > "$S/$1-$3.stdout" 2>&1
    echo "$1-$3: cargo xtask tosdump exit $?"
}
for side in before after; do
    if [ $side = before ]; then t=$B; ti=$IB; else t=-; ti=$IA; fi
    run $side $t  bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN          EMUTOS-1a7484ab-BIOS-falcontc.BIN
    run $side $t  xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
    run $side $t  xbios-ptr-r1     xbios-ptr-r1.tsv     EMUTOS-1a7484ab-XBIOS-ptr.BIN     EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN
    run $side $ti xbios-flush-r1   xbios-flush-r1.tsv   EMUTOS-1a7484ab-XBIOS-flush.BIN   EMUTOS-1a7484ab-XBIOS-flush-falcontc.BIN
done
