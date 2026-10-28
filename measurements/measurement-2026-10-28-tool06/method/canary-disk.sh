#!/bin/sh
# Disk-profile canary (docs/analysis/ORACLE.md section 6, "Canary of the disk
# profile"): rebuild DUMPBIOS with build id a8b7, build the empty test image with
# tools/mktestdisk.sh, run bios-r1.tsv without its two time-dependent lines on
# the pinned ROM in profile disk, compare BIN and LOG byte for byte with the pinned sha256, and
# check that the run left the test image unchanged.
#   sh canary-disk.sh <empty-scratch-dir>      (run from the repository root)
#
# Why bios-r1.tsv minus lines 112 and 125: their return value depends on when the
# run happens, not on the oracle. Line 125, Gettime (XBIOS 23, class NONZERO),
# returns the RTC time, which Hatari takes from the host clock (bios/clock.c:1266);
# line 112, Random (XBIOS 17, class IGNORE), seeds itself from hz_200 at its first
# call (bios/xbios.c:421-426), and the tick count at that point varied between
# runs. Measured: two runs of the full table differ in exactly these records
# (measurements/measurement-2026-10-28-tool06/RUNINFO.TXT). Both lines are
# commented out in place, so every other case keeps its id (= line number); the
# derived table has its own CRC and sha256, both pinned below. Until TOOL07's
# read-only disk cases exist this is the canary table (TASK/TOOL06).
set -eu
S=$1
[ ! -e "$S" ] || { echo "scratch dir exists: $S" >&2; exit 2; }
mkdir -p "$S"

TABLE_SHA=231f2dfe73e5a80628167b216a66fe6475992b9b3a692b9025a92e8d6f3b2bf8   # bios-r1.tsv
CANARY_SHA=7f1dfe2d575c40b3336aa321637c953eae68e6e8bec0d01ab68249728a0404c7   # bios-r1.tsv, lines 112 and 125 commented out
IMAGE_SHA=d83c543471fd30f29e4704cb9450f70517fe43a4e5e0250e137205e409bfc4a0  # mktestdisk, no files
BIN_SHA=0bce674c24bb094c6647157c9d4a8b5d0c98ab3210a66bbe46efaca128c9d1f9
LOG_SHA=ae160cac6e37c0d54acc7dc1092c093f0a2e7717b782a22772835938dc33fb6f

sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
check() {   # check <what> <file> <expected sha256>
    got=$(sha "$2")
    [ "$got" = "$3" ] || { echo "canary disk: FAIL ($1 $2: $got, expected $3)"; exit 1; }
}

check table tosdump/cases/bios-r1.tsv "$TABLE_SHA"     # pins lines 112 and 125 too
sed '112s/^/# canary: time dependent -- /; 125s/^/# canary: time dependent -- /' \
    tosdump/cases/bios-r1.tsv > "$S/canary-bios.tsv"
check table "$S/canary-bios.tsv" "$CANARY_SHA"

make -s -C tosdump BUILD="$S/dumpers/a8b7" BUILD_ID=a8b7 dumpbios-m68k
tools/mktestdisk.sh "$S/test.img" > "$S/mktestdisk.stdout"
check image "$S/test.img" "$IMAGE_SHA"

HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk \
    "$S/dumpers/a8b7/DUMPBIOS.PRG" "$S/canary-bios.tsv" "$S/disk" > "$S/disk.stdout" 2>&1
rc=0
if [ "$(sha "$S/disk/BIOS.BIN")" = "$BIN_SHA" ] && [ "$(sha "$S/disk/BIOS.LOG")" = "$LOG_SHA" ]; then
    echo "canary disk: OK (BIN+LOG sha256 as pinned)"
else
    echo "canary disk: FAIL ($S/disk: BIN $(sha "$S/disk/BIOS.BIN"), LOG $(sha "$S/disk/BIOS.LOG"))"; rc=1
fi
after=$(sha "$S/disk/disk.img")
if [ "$after" = "$IMAGE_SHA" ]; then
    echo "canary disk: test image unchanged ($IMAGE_SHA)"
else
    echo "canary disk: FAIL (test image changed by the run: $after)"; rc=1
fi
rm -f "$S/disk/disk.img" "$S/test.img" "$S/test.img.manifest"
exit $rc
