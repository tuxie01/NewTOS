#!/bin/sh
# canary-ptr.sh <empty-scratch-dir> -- no-regression check of the dumper
# changes of TASK/TOOL20 (the layout grammar, the runner's token and poison
# code) on a frozen pointer-round table: rebuild DUMPBIOS with the build id
# of the reference EMUTOS-1a7484ab-XBIOS-ptr (c0f9, its header), run
# tosdump/cases/xbios-ptr-r1.tsv on the oracle's tt profile, and compare BIN
# and LOG byte for byte with the pinned reference. Run from the repository
# root. The disk-round counterpart is the canary of
# measurements/measurement-2026-10-28-tool07b/method/canary-disk.sh (run as
# it is).
set -eu
S=$1
[ ! -e "$S" ] || { echo "scratch dir exists: $S" >&2; exit 2; }
mkdir -p "$S"
make -s -C tosdump BUILD="$S/dumpers/c0f9" BUILD_ID=c0f9 dumpbios-m68k > "$S/make.txt" 2>&1
tools/hatari-oracle.sh tt "$S/dumpers/c0f9/DUMPBIOS.PRG" tosdump/cases/xbios-ptr-r1.tsv "$S/tt" > "$S/tt.stdout" 2>&1
if cmp -s "$S/tt/BIOS.BIN" tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr.BIN &&
   cmp -s "$S/tt/BIOS.LOG" tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr.LOG; then
    echo "canary ptr: OK (BIN+LOG identical to EMUTOS-1a7484ab-XBIOS-ptr)"
else
    echo "canary ptr: FAIL ($S/tt)"; exit 1
fi
