#!/bin/sh
# newtos-run.sh <scratch> <runs-dir> -- TOOL18: tosdump/cases/jar-r1.tsv on NewTOS
# (QEMU raspi4b, kernel rpi4 release of this tree, no features) through the TOOL09
# runner, compared against the three oracle references (tt, falcontc, disk).
# The NewTOS jar does not depend on the oracle's profile: one run, three comparisons.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
T=tosdump/cases/jar-r1.tsv
mkdir -p "$S/xtask"
cargo xtask tosdump bios "$T" \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-jar.BIN \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-jar-falcontc.BIN \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-jar-disk.BIN \
    --out "$R/newtos-jar-r1" --scratch "$S/xtask" > "$S/newtos.stdout" 2>&1
echo "newtos-jar-r1: cargo xtask tosdump exit $?"
[ -f "$R/newtos-jar-r1/BIOS.BIN" ] && python3 -I "$HERE/decode-jar.py" \
    "$R/newtos-jar-r1/BIOS.BIN" "$T" > "$R/newtos-jar-r1/decoded.txt"
