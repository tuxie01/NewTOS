#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> [n] -- the three gemdos-mc tables of
# TASK/TOOL20 on the oracle's profile disk, n runs each (default 2), every run
# on its own fresh clone of one test image (tools/hatari-oracle.sh clones it;
# a destructive table is refused otherwise).
#   test image  method/mkimage.sh <scratch> (tools/mktestdisk.sh, sha256 in
#               the RUNINFO)
#   dumper      make -C tosdump BUILD=<scratch>/dumpers/<id> BUILD_ID=<id>
#               dumpbios-m68k, <id> = the first 4 hex digits of HEAD
# Kept per run in <runs-dir>/oracle-<t>-<i>: BIOS.BIN, BIOS.LOG, disk.sha256,
# hatari.log, hatari-oracle.stdout, decoded.txt (method/decode-mc.py).  The
# test partition after each run goes to <scratch>/end/oracle-<t>-<i>.img for
# compare-ends.sh (LBA 63, 130977 sectors); the disk clones are deleted.
set -eu
S=$1; R=$2; N=${3:-2}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
ID=$(git rev-parse HEAD | cut -c1-4)
mkdir -p "$S/end" "$R"
[ -f "$S/test.img" ] || sh "$HERE/mkimage.sh" "$S" > "$S/mkimage.txt"
dd if="$S/test.img" of="$S/end/original.img" bs=512 skip=63 count=130977 2>/dev/null
make -s -C tosdump BUILD="$S/dumpers/$ID" BUILD_ID="$ID" dumpbios-m68k > "$S/make-$ID.txt" 2>&1
shasum -a 256 "$S/dumpers/$ID/DUMPBIOS.PRG"
for T in fs20 fs22 paths; do
    i=1
    while [ "$i" -le "$N" ]; do
        O=$S/oracle-$T-$i
        HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk \
            "$S/dumpers/$ID/DUMPBIOS.PRG" "tosdump/cases/gemdos-mc-$T.tsv" "$O" \
            > "$S/oracle-$T-$i.stdout" 2>&1 || echo "oracle-$T-$i: hatari-oracle.sh exit $?"
        mkdir -p "$R/oracle-$T-$i"
        cp "$O/BIOS.BIN" "$O/BIOS.LOG" "$O/disk.sha256" "$O/hatari.log" "$R/oracle-$T-$i/"
        cp "$S/oracle-$T-$i.stdout" "$R/oracle-$T-$i/hatari-oracle.stdout"
        python3 "$HERE/decode-mc.py" "$O/BIOS.BIN" "tosdump/cases/gemdos-mc-$T.tsv" \
            > "$R/oracle-$T-$i/decoded.txt"
        dd if="$O/disk.img" of="$S/end/oracle-$T-$i.img" bs=512 skip=63 count=130977 2>/dev/null
        rm -f "$O/disk.img"
        echo "oracle-$T-$i: BIN $(shasum -a 256 "$O/BIOS.BIN" | cut -c1-16) LOG $(shasum -a 256 "$O/BIOS.LOG" | cut -c1-16)"
        i=$((i + 1))
    done
done
