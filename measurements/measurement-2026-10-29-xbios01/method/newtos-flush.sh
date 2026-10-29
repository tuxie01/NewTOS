#!/bin/sh
# newtos-flush.sh <injector-tree> <scratch> <runs-dir> -- XBIOS01: xbios-flush-r1.tsv on
# NewTOS with keys. <injector-tree> is an APFS clone of this tree (its .git removed),
# patched by `tools/inject-arm.py <clone> launch:DUMPBIOS=method/keys-ikbd.txt`
# (patch recorded as method/injector-scratch.diff); the runner builds and boots it
# (--tree), compares against both oracle flush references of this run.
set -u
T=$1; S=$2; R=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask"
cargo xtask tosdump bios tosdump/cases/xbios-flush-r1.tsv --tree "$T" \
    --reference tosdump/reference/EMUTOS-1a7484ab-XBIOS-flush.BIN \
    --reference tosdump/reference/EMUTOS-1a7484ab-XBIOS-flush-falcontc.BIN \
    --out "$R/newtos-xbios-flush-r1" --scratch "$S/xtask" > "$S/newtos-flush.stdout" 2>&1
echo "newtos-xbios-flush-r1: cargo xtask tosdump exit $?"
