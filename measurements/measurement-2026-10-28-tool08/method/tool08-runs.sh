#!/bin/bash
# tool08-runs.sh <scratch> -- the TOOL08 runs: bios-r1, bios-core-r1 and bios-r2 on NewTOS
# (QEMU raspi4b, -append serial=on) with a kernel built with the cargo feature
# bios-arg-trace, each serial log checked by tools/biostrace-check.py against its table and
# the run's own BIOS.BIN.
#
#   kernel   cargo xtask build --board rpi4 --release --features bios-arg-trace -> <scratch>/kernel8.img
#   dumper   tosdump/build-dumpbios-newtos.sh <scratch>/DUMPBIOS.NAP
#   r1, core run-dumpbios-newtos.sh (copy of measurements/measurement-2026-10-28-bios-core-audit/
#            method/run-dumpbios-newtos.sh, unchanged) -- tools/mkcard.sh card, C: = program drive
#   r2       newtos-runs.sh + run-newtos.sh + compare-testpart.sh (copies of
#            measurements/measurement-2026-10-28-tool07/method/, unchanged): MKCARD_TESTDISK card,
#            C: = the test image from tools/mktestdisk.sh, D: = the program partition
# Results go to runs/ beside this method/ directory.
#
# tool08-runs.sh <scratch> control -- the same three runs with the kernel built WITHOUT the
# feature (the default build), into runs-control/: the BIN records of the two must be equal
# (the instrument changes nothing it measures); the checker then finds no [biostrace] line.
set -u
S=$1; MODE=${2:-trace}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
if [ "$MODE" = control ]; then R="$HERE/../runs-control"; FEAT=(); else R="$HERE/../runs"; FEAT=(--features bios-arg-trace); fi
mkdir -p "$S" "$R"; R="$(cd "$R" && pwd)"
cd "$REPO"
git rev-parse HEAD > "$R/head.txt"; git status --short -- kernel tools/biostrace-check.py >> "$R/head.txt"

cargo xtask build --board rpi4 --release ${FEAT[@]+"${FEAT[@]}"} > "$R/kernel-build.stdout" 2>&1 || exit 1
cp kernel8.img "$S/kernel8.img"
tosdump/build-dumpbios-newtos.sh "$S/DUMPBIOS.NAP" > "$R/dumper-build.stdout" 2>&1 || exit 1
shasum -a 256 "$S/kernel8.img" "$S/DUMPBIOS.NAP" tosdump/cases/bios-r1.tsv tosdump/cases/bios-core-r1.tsv \
    tosdump/cases/bios-r2.tsv tools/biostrace-check.py > "$R/inputs.sha256"

# bios-r1 and bios-core-r1: the round-1 card (DUMPBIOS.NAP + BIOS.TSV on C:)
bash "$HERE/run-dumpbios-newtos.sh" "$REPO" "$S" "$R/r1" tosdump/cases/bios-r1.tsv \
    tosdump/reference/EMUTOS-1a7484ab-BIOS tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc \
    > "$R/r1.stdout" 2>&1
bash "$HERE/run-dumpbios-newtos.sh" "$REPO" "$S" "$R/core" tosdump/cases/bios-core-r1.tsv \
    tosdump/reference/EMUTOS-1a7484ab-BIOS-core tosdump/reference/EMUTOS-1a7484ab-BIOS-core-falcontc \
    > "$R/core.stdout" 2>&1

# bios-r2: the test-disk card, test image as TOOL07 made it
tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$HERE/newdesk-d.inf" > "$R/mktestdisk.stdout" 2>&1 || exit 1
cp "$S/test.img.manifest" "$R/test.img.manifest" 2>/dev/null
shasum -a 256 "$S/test.img" > "$R/test.img.sha256"
"$HERE/newtos-runs.sh" "$S/r2" "$R/r2" "$S/test.img" "$S/kernel8.img" 1 > "$R/r2.stdout" 2>&1
mkdir -p "$R/r2/cmp"
cp tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN "$R/r2/cmp/EMUTOS.BIN"
cp "$R/r2/newtos-1/BIOS.BIN" "$R/r2/cmp/NEWTOS.BIN"
(cd "$R/r2/cmp" && "$REPO/tosdump/host/build/tosdiff" "$REPO/tosdump/cases/bios-r2.tsv" EMUTOS.BIN NEWTOS.BIN \
    > ../tosdiff-newtos-vs-emutos-r2-disk.txt; echo "r2 exit $?" >> "$R/r2.stdout")

# the input check
python3 -I tools/biostrace-check.py "$R/r1/BOOT-SERIAL.txt" tosdump/cases/bios-r1.tsv \
    --bin "$R/r1/BIOS.BIN" > "$R/check-bios-r1.txt"; echo "exit $?" >> "$R/check-bios-r1.txt"
python3 -I tools/biostrace-check.py "$R/core/BOOT-SERIAL.txt" tosdump/cases/bios-core-r1.tsv \
    --bin "$R/core/BIOS.BIN" > "$R/check-bios-core-r1.txt"; echo "exit $?" >> "$R/check-bios-core-r1.txt"
python3 -I tools/biostrace-check.py "$R/r2/newtos-1/BOOT-SERIAL.txt" tosdump/cases/bios-r2.tsv \
    --bin "$R/r2/newtos-1/BIOS.BIN" > "$R/check-bios-r2.txt"; echo "exit $?" >> "$R/check-bios-r2.txt"
tail -n 2 "$R"/check-*.txt
