#!/bin/sh
# newtos-run.sh <scratch> <runs-dir> -- TOOL16: tosdump/cases/xbios-ptr-r1.tsv on NewTOS
# (QEMU raspi4b, kernel rpi4 release of this tree) through the TOOL09 runner, compared
# against both oracle references.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/xtask"
cargo xtask tosdump bios tosdump/cases/xbios-ptr-r1.tsv \
    --reference tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr.BIN \
    --reference tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN \
    --out "$R/newtos-xbios-ptr-r1" --scratch "$S/xtask" > "$S/newtos.stdout" 2>&1
echo "newtos-xbios-ptr-r1: cargo xtask tosdump exit $?"
