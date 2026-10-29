#!/bin/bash
# newtos-tables.sh <scratch> <runs-dir> [<injector tree>] -- XBIOS18: the regression
# tables on NewTOS (QEMU raspi4b, kernel rpi4 release, the TOOL09 runner) on the
# changed tree, each against its tt and falcontc oracle reference: bios-r1 and
# xbios-chardev-r1 on the main tree; xbios-flush-r1 on <injector tree> (a copy of the
# main tree without .git/target/ patched by `tools/inject-arm.py <tree>
# launch:DUMPBIOS=method/keys-ikbd.txt`), when given.
set -u
S=$1; R=$2; INJ=${3:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
REF=tosdump/reference
run() { # run <tree|-> <name> <table> <tt ref> <falcontc ref>
    tree=""; [ "$1" = "-" ] || tree="--tree $1"
    mkdir -p "$S/xtask-after"
    # shellcheck disable=SC2086
    cargo xtask tosdump bios "tosdump/cases/$3" $tree --reference "$REF/$4" --reference "$REF/$5" \
        --out "$R/after-$2" --scratch "$S/xtask-after" > "$S/after-$2.stdout" 2>&1
    echo "after-$2: cargo xtask tosdump exit $?"
}
if [ -z "$INJ" ]; then
    run - bios-r1          bios-r1.tsv          EMUTOS-1a7484ab-BIOS.BIN          EMUTOS-1a7484ab-BIOS-falcontc.BIN
    run - xbios-chardev-r1 xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
else
    run "$INJ" xbios-flush-r1 xbios-flush-r1.tsv EMUTOS-1a7484ab-XBIOS-flush.BIN  EMUTOS-1a7484ab-XBIOS-flush-falcontc.BIN
fi
