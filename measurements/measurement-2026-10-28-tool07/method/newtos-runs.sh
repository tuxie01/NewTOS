#!/bin/bash
# newtos-runs.sh <scratch> <runs-dir> <test.img> <kernel8.img> [n] -- bios-r2.tsv on NewTOS,
# QEMU raspi4b, n runs (default 1), each on its own APFS clone of one card.
#   dumper  tosdump/build-dumpbios-newtos.sh <scratch>/DUMPBIOS.NAP (build id = HEAD's first 4)
#   card    MKCARD_TESTDISK=<test.img> tools/mkcard.sh <scratch>/card.img method/newdesk-d.inf
#             DUMPBIOS.NAP=<scratch>/DUMPBIOS.NAP BIOS.TSV=tosdump/cases/bios-r2.tsv
#           <test.img> is the oracle runs' test image (oracle-runs.sh), the same file.
#   run     run-newtos.sh <kernel8.img> <clone> <runs-dir>/newtos-<i>
set -u
S=$1; R=$2; T=$3; K=$4; N=${5:-1}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
cd "$REPO"
mkdir -p "$S" "$R"
tosdump/build-dumpbios-newtos.sh "$S/DUMPBIOS.NAP" > "$R/dumper-build-newtos.stdout" 2>&1
MKCARD_TESTDISK="$T" tools/mkcard.sh "$S/card.img" "$HERE/newdesk-d.inf" \
    DUMPBIOS.NAP="$S/DUMPBIOS.NAP" BIOS.TSV=tosdump/cases/bios-r2.tsv > "$R/mkcard.stdout" 2>&1 || exit 1
cp "$S/card.img.manifest" "$R/card.img.manifest"
shasum -a 256 "$K" "$S/DUMPBIOS.NAP" "$T" "$S/card.img" tosdump/cases/bios-r2.tsv > "$R/inputs-newtos.sha256"
for i in $(seq 1 "$N"); do
  rm -f "$S/run-$i.img"; cp -c "$S/card.img" "$S/run-$i.img"
  "$HERE/run-newtos.sh" "$K" "$S/run-$i.img" "$R/newtos-$i" BIOS > /dev/null 2>&1
  rm -f "$S/run-$i.img"
done
