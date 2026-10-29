#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> [n] -- FS24: tosdump/cases/gemdos-mc-fs24.tsv
# on the oracle's profile disk, n runs (default 2), each on its own fresh
# clone of one test image (tools/hatari-oracle.sh clones it; a destructive
# table is refused otherwise).
#   test image  measurements/measurement-2026-10-29-tool20/method/mkimage.sh
#               <scratch> (called, not copied; tools/mktestdisk.sh)
#   dumper      make -C tosdump BUILD=<scratch>/dumpers/<id> BUILD_ID=<id>
#               dumpbios-m68k, <id> = the first 4 hex digits of HEAD
# Kept per run in <runs-dir>/oracle-fs24-<i>: BIOS.BIN, BIOS.LOG, disk.sha256,
# hatari.log, hatari-oracle.stdout, decoded.txt (TOOL20's decode-mc.py,
# called).  The test partition after each run goes to
# <scratch>/end/oracle-fs24-<i>.img (LBA 63, 130977 sectors); the disk clones
# are deleted.
set -eu
S=$1; R=$2; N=${3:-2}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
T20=$REPO/measurements/measurement-2026-10-29-tool20/method
cd "$REPO"
ID=$(git rev-parse HEAD | cut -c1-4)
mkdir -p "$S/end" "$R"
[ -f "$S/test.img" ] || sh "$T20/mkimage.sh" "$S" > "$S/mkimage.txt"
make -s -C tosdump BUILD="$S/dumpers/$ID" BUILD_ID="$ID" dumpbios-m68k > "$S/make-$ID.txt" 2>&1
shasum -a 256 "$S/dumpers/$ID/DUMPBIOS.PRG"
TBL=tosdump/cases/gemdos-mc-fs24.tsv
i=1
while [ "$i" -le "$N" ]; do
    O=$S/oracle-fs24-$i
    HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk \
        "$S/dumpers/$ID/DUMPBIOS.PRG" "$TBL" "$O" \
        > "$S/oracle-fs24-$i.stdout" 2>&1 || echo "oracle-fs24-$i: hatari-oracle.sh exit $?"
    mkdir -p "$R/oracle-fs24-$i"
    cp "$O/BIOS.BIN" "$O/BIOS.LOG" "$O/disk.sha256" "$O/hatari.log" "$R/oracle-fs24-$i/"
    cp "$S/oracle-fs24-$i.stdout" "$R/oracle-fs24-$i/hatari-oracle.stdout"
    python3 "$T20/decode-mc.py" "$O/BIOS.BIN" "$TBL" > "$R/oracle-fs24-$i/decoded.txt"
    dd if="$O/disk.img" of="$S/end/oracle-fs24-$i.img" bs=512 skip=63 count=130977 2>/dev/null
    rm -f "$O/disk.img"
    echo "oracle-fs24-$i: BIN $(shasum -a 256 "$O/BIOS.BIN" | cut -c1-16) LOG $(shasum -a 256 "$O/BIOS.LOG" | cut -c1-16)"
    i=$((i + 1))
done
