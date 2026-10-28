#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> [n] -- the closing tables of TASK/TOOL07
# (bios-r2-phys, -x004, -x005, -x006) on the oracle's profile disk, n runs each
# (default 2), every run on its own fresh clone of one test image
# (tools/hatari-oracle.sh clones it; a destructive table is refused otherwise).
#   test image  tools/mktestdisk.sh <scratch>/test.img NEWDESK.INF=<scratch>/newdesk-d.inf
#               (the INF bytes of measurements/measurement-2026-10-28-tool07,
#               written here by printf: the same image, sha256 d95a60c3...)
#   dumper      make -C tosdump BUILD=<scratch>/dumpers/<id> BUILD_ID=<id> dumpbios-m68k,
#               <id> = the first 4 hex digits of HEAD
# Kept per run in <runs-dir>/oracle-<table>-<i>: BIOS.BIN, BIOS.LOG, disk.sha256,
# hatari.log, hatari-oracle.stdout, fsck-before.txt (destructive tables).
# The test partition after each run goes to <scratch>/end/oracle-<table>-<i>.img
# for compare-ends.sh (LBA 63, 130977 sectors); the disk clones are deleted.
set -eu
S=$1; R=$2; N=${3:-2}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
ID=$(git rev-parse HEAD | cut -c1-4)
mkdir -p "$S/end" "$R"
printf '#R 02\r\n#Z 01 D:\\DUMPBIOS.NAP@\r\n' > "$S/newdesk-d.inf"
tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$S/newdesk-d.inf" > "$R/mktestdisk.stdout"
cp "$S/test.img.manifest" "$R/test.img.manifest"
make -C tosdump BUILD="$S/dumpers/$ID" BUILD_ID="$ID" dumpbios-m68k > "$R/dumper-build.stdout" 2>&1
shasum -a 256 "$S/dumpers/$ID/DUMPBIOS.PRG" "$S/test.img" "$S/newdesk-d.inf" \
    tosdump/cases/bios-r2-phys.tsv tosdump/cases/bios-r2-x004.tsv \
    tosdump/cases/bios-r2-x005.tsv tosdump/cases/bios-r2-x006.tsv > "$R/inputs-oracle.sha256"
dd if="$S/test.img" of="$S/end/original.img" bs=512 skip=63 count=130977 2>/dev/null
for T in phys x004 x005 x006; do
    i=1
    while [ "$i" -le "$N" ]; do
        O=$S/oracle-$T-$i
        rm -rf "$O"
        HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk "$S/dumpers/$ID/DUMPBIOS.PRG" \
            "tosdump/cases/bios-r2-$T.tsv" "$O" > "$O.stdout" 2>&1 || true
        D=$R/oracle-$T-$i
        mkdir -p "$D"
        for f in BIOS.BIN BIOS.LOG disk.sha256 hatari.log fsck-before.txt; do
            [ -f "$O/$f" ] && cp "$O/$f" "$D/$f"
        done
        cp "$O.stdout" "$D/hatari-oracle.stdout"
        dd if="$O/disk.img" of="$S/end/oracle-$T-$i.img" bs=512 skip=63 count=130977 2>/dev/null
        rm -f "$O/disk.img"
        i=$((i + 1))
    done
done
