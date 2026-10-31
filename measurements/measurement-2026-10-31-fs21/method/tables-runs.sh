#!/bin/sh
# tables-runs.sh <scratch> <runs-dir> <head-tree> -- TASK/FS21: the four gemdos-mc tables
# (fs20, fs22, paths, fs24) plus the canary bios-r1 on NewTOS before (HEAD, --tree) and after
# (this tree), and the four gemdos-mc tables once on the oracle for their end states.
# Everything is called, not copied:
#   test image  measurements/measurement-2026-10-29-tool20/method/mkimage.sh <scratch>
#   NewTOS      measurements/measurement-2026-10-30-fs28/method/newtos-runs.sh before|after
#               (cargo xtask tosdump bios <table> --testdisk --reference EMUTOS-1a7484ab-BIOS-mc-<t>-disk.BIN)
#   oracle      measurements/measurement-2026-10-29-tool20/method/oracle-runs.sh (fs20, fs22, paths)
#               and measurements/measurement-2026-10-29-fs24/method/oracle-runs.sh (fs24), one run each
# The end states land in <scratch>/end/{original,oracle-<t>-1,before-<t>,newtos-<t>}.img.
set -u
S=$1; R=$2; T=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
M=measurements
sh $M/measurement-2026-10-29-tool20/method/mkimage.sh "$S" > "$S/mkimage.txt" 2>&1; echo "mkimage exit $?"
cp "$S/test.img.manifest" "$R/test.img.manifest"
sh $M/measurement-2026-10-30-fs28/method/newtos-runs.sh after "$S" "$R"
sh $M/measurement-2026-10-30-fs28/method/newtos-runs.sh before "$S" "$R" "$T"
sh $M/measurement-2026-10-29-tool20/method/oracle-runs.sh "$S" "$R" 1
sh $M/measurement-2026-10-29-fs24/method/oracle-runs.sh "$S" "$R" 1
