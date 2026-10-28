#!/bin/sh
# Disk-profile canary (docs/analysis/ORACLE.md section 6, "Canary of the disk
# profile"), since TOOL07 (2026-10-28) on the disk round's own table: rebuild
# DUMPBIOS with build id b81d (the id of the reference), build the test image
# of bios-r2.tsv with tools/mktestdisk.sh, run bios-r2.tsv on the pinned ROM in
# profile disk, compare BIN and LOG byte for byte with the pinned sha256 of
# tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk, and check that the run left
# the test image unchanged.
#   sh canary-disk.sh <empty-scratch-dir>      (run from the repository root)
#
# Why bios-r2.tsv as it is: it is the disk round's non-destructive table -- its
# writes write back what was just read (docs/TOS-REFERENCE-DUMPER.md section 6),
# the image keeps every byte -- and it has no time-dependent case (no Gettime,
# no Random), so the whole table reproduces: two runs of the reference
# (measurements/measurement-2026-10-28-tool07/RUNINFO.TXT) and two runs of this
# script (measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT) were
# byte-identical. The destructive closing tables (bios-r2-x00*.tsv) are not
# canary material: they change the image by design.
#
# The test image carries C:\NEWDESK.INF "#R 02 / #Z 01 D:\DUMPBIOS.NAP@" (CRLF),
# the file NewTOS needs on C: and EmuTOS does not read -- written here by printf,
# the same bytes as measurements/measurement-2026-10-28-tool07/method/newdesk-d.inf.
set -eu
S=$1
[ ! -e "$S" ] || { echo "scratch dir exists: $S" >&2; exit 2; }
mkdir -p "$S"

TABLE_SHA=071300e43736d1c3371e65d4a0e2b3f4b5e327dcd4d7147763fe0c04e553bc44   # bios-r2.tsv, CRC 09C71B80
INF_SHA=c779a308dbfae3b61747e3ebcf5d45f7d616bc53e63b83ec4afa3557f02ecf5b     # the NEWDESK.INF
IMAGE_SHA=d95a60c3f5d76fee767b9dc5cd16dab3b2bc1215e116fe738cbdbaa318a92d37   # mktestdisk with it
BIN_SHA=072d8cd1bdbc03127d6175d888f51fb2ec100209d96b61f1a4c5f0c684177b1e     # EMUTOS-1a7484ab-BIOS-r2-disk.BIN
LOG_SHA=055e586a71f6c3bc32065d08099b14b5e8ee50d1871191083724a52e768a5a76     # EMUTOS-1a7484ab-BIOS-r2-disk.LOG

sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
check() {   # check <what> <file> <expected sha256>
    got=$(sha "$2")
    [ "$got" = "$3" ] || { echo "canary disk: FAIL ($1 $2: $got, expected $3)"; exit 1; }
}

check table tosdump/cases/bios-r2.tsv "$TABLE_SHA"
printf '#R 02\r\n#Z 01 D:\\DUMPBIOS.NAP@\r\n' > "$S/newdesk-d.inf"
check inf "$S/newdesk-d.inf" "$INF_SHA"

make -s -C tosdump BUILD="$S/dumpers/b81d" BUILD_ID=b81d dumpbios-m68k
tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$S/newdesk-d.inf" > "$S/mktestdisk.stdout"
check image "$S/test.img" "$IMAGE_SHA"

HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh disk \
    "$S/dumpers/b81d/DUMPBIOS.PRG" tosdump/cases/bios-r2.tsv "$S/disk" > "$S/disk.stdout" 2>&1
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
