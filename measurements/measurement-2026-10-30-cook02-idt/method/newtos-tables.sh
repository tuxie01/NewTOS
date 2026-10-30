#!/bin/sh
# newtos-tables.sh before|after <scratch> <runs-dir> [<tree>] -- COOK02 (_IDT 0x112E):
# jar-r1 against the three jar references (tt, falcontc, disk), bios-r1 and
# xbios-system-r1 against tt and falcontc, on NewTOS (QEMU raspi4b, kernel rpi4
# release) through the TOOL09 runner. "before" = <tree> (`git archive a8d875a7`
# extracted into scratch, no .git, passed as --tree); "after" = this tree with
# the COOK02 _IDT change.
# (adapted from measurement-2026-10-30-xbios04/method/newtos-tables.sh: the table
# set, the jar's three references, header and commit names.)
set -u
SIDE=$1; S=$2; R=$3; T=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask-$SIDE"
REF=tosdump/reference
run() { # run <name> <table> <ref>...
    n=$1; t=$2; shift 2
    set -- $(for r in "$@"; do printf -- '--reference %s ' "$REF/$r"; done)
    if [ -n "$T" ]; then set -- "$@" --tree "$T"; fi
    cargo xtask tosdump bios "tosdump/cases/$t" "$@" \
        --out "$R/$SIDE-$n" --scratch "$S/xtask-$SIDE" > "$S/$SIDE-$n.stdout" 2>&1
    echo "$SIDE-$n: cargo xtask tosdump exit $?"
}
run jar-r1          jar-r1.tsv          EMUTOS-1a7484ab-BIOS-jar.BIN EMUTOS-1a7484ab-BIOS-jar-falcontc.BIN EMUTOS-1a7484ab-BIOS-jar-disk.BIN
run bios-r1         bios-r1.tsv         EMUTOS-1a7484ab-BIOS.BIN EMUTOS-1a7484ab-BIOS-falcontc.BIN
run xbios-system-r1 xbios-system-r1.tsv EMUTOS-1a7484ab-XBIOS-system.BIN EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
