#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> [n] -- bios-r2.tsv on the oracle's disk profile, n times
# (default 2), each on its own clone of one test image.
#   test image  tools/mktestdisk.sh <scratch>/test.img NEWDESK.INF=method/newdesk-d.inf
#               (the NEWDESK.INF NewTOS needs on C:, inert for EmuTOS: the same bytes on both)
#   dumper      make -C tosdump BUILD=<scratch>/dumpers/b81d BUILD_ID=b81d dumpbios-m68k
#   run         HATARI_ORACLE_DISK=<scratch>/test.img tools/hatari-oracle.sh disk ... <scratch>/oracle-<i>
# Kept per run in <runs-dir>/oracle-<i>: BIOS.BIN, BIOS.LOG, disk.sha256, hatari.log,
# testpart-compare.txt (compare-testpart.sh on the clone's partition, before vs after).
# The clones are deleted.
set -eu
S=$1; R=$2; N=${3:-2}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S" "$R"
tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$HERE/newdesk-d.inf" > "$R/mktestdisk.stdout"
cp "$S/test.img.manifest" "$R/test.img.manifest"
make -C tosdump BUILD="$S/dumpers/b81d" BUILD_ID=b81d dumpbios-m68k > "$R/dumper-build.stdout" 2>&1
shasum -a 256 "$S/dumpers/b81d/DUMPBIOS.PRG" tosdump/cases/bios-r2.tsv "$S/test.img" > "$R/inputs.sha256"
i=1
while [ "$i" -le "$N" ]; do
    O=$S/oracle-$i
    rm -rf "$O"
    HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk "$S/dumpers/b81d/DUMPBIOS.PRG" \
        tosdump/cases/bios-r2.tsv "$O" > "$O.stdout" 2>&1 || true
    mkdir -p "$R/oracle-$i"
    for f in BIOS.BIN BIOS.LOG disk.sha256 hatari.log; do [ -f "$O/$f" ] && cp "$O/$f" "$R/oracle-$i/$f"; done
    cp "$O.stdout" "$R/oracle-$i/hatari-oracle.stdout"
    dd if="$S/test.img" of="$S/tp-before.img" bs=512 skip=63 count=130977 2>/dev/null
    dd if="$O/disk.img" of="$S/tp-after.img" bs=512 skip=63 count=130977 2>/dev/null
    sh "$HERE/compare-testpart.sh" "$S/tp-before.img" "$S/tp-after.img" > "$R/oracle-$i/testpart-compare.txt" 2>&1 || true
    rm -f "$O/disk.img" "$S/tp-before.img" "$S/tp-after.img"
    i=$((i + 1))
done
