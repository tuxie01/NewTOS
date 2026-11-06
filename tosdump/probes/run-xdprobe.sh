#!/usr/bin/env bash
#
# run-xdprobe.sh -- the two-drive probe tosdump/probes/xdprobe.c (TASK/FS31) on
# one side, on the same card bytes on both sides.
#
#   tosdump/probes/run-xdprobe.sh emutos <out-dir>
#   tosdump/probes/run-xdprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh profile disk (Falcon030,
#            the card cloned as IDE master: C: D: E:), the probe on the GEMDOS
#            drive F: (HATARI_ORACLE_GEMDOS_DRIVE=F), --auto F:\XDPROBE.PRG.
#   newtos   QEMU raspi4b with `-append serial=on` on an APFS clone of the
#            card: E:\XDPROBE.NAP, started by the desktop's #Z autorun.
#
# The card (256 MiB, built here with mtools as tools/mktestdisk.sh does it: no
# mount, no configuration file, a fixed time stamp): an MBR with three FAT16
# partitions of the geometry of the oracle's test image, 130977 sectors each,
# at LBA 63, 131103 and 262143. C: and D: hold the same root: label, NEWDESK.INF.
# E: holds XDPROBE.NAP. NEWDESK.INF is what NewTOS' desktop reads; EmuTOS reads
# EMUDESK.INF only and does not see it.
#
# <out-dir> (must not exist) receives XDPROBE.LOG, card.img.manifest, run.txt
# (what ran, sha256 of probe, card, kernel, fsck of C: and D: after the run) and
# the side's log (hatari.log + disk.sha256, or BOOT-SERIAL.txt).
#
# Scratch: $XDPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos <out-dir> | newtos <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -ge 2 ] || usage
SIDE=$1; OUT=$2
case $SIDE in
    emutos) [ $# -eq 2 ] || usage ;;
    newtos) [ $# -eq 3 ] || usage; KERNEL=$3
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${XDPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set XDPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/xdprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump xdprobe-m68k xdprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/XDPROBE.PRG
NAP=$TD/build/XDPROBE.NAP

# The card (header).
PSIZE=130977
STARTS="63 131103 262143"
STAMP=202610171200
export MTOOLSRC=/dev/null MTOOLS_NO_VFAT=1 MTOOLS_SKIP_CHECK=1 TZ=UTC
SOURCE_DATE_EPOCH=$(date -j -u -f %Y%m%d%H%M%S "${STAMP}00" +%s)
export SOURCE_DATE_EPOCH
CARD=$SCR/card.img
dd if=/dev/zero of="$CARD" bs=1m count=256 status=none
python3 - "$CARD" "$PSIZE" $STARTS <<'PY'
import struct, sys
card, size, starts = sys.argv[1], int(sys.argv[2]), [int(a) for a in sys.argv[3:]]
mbr = bytearray(512)
for i, start in enumerate(starts):
    # LBA only: CHS "beyond the addressable range", as for any large disk.
    mbr[446 + i * 16:462 + i * 16] = bytes([0, 0xFE, 0xFF, 0xFF, 6, 0xFE, 0xFF, 0xFF]) + struct.pack("<II", start, size)
mbr[510:512] = b"\x55\xaa"
with open(card, "r+b") as f:
    f.write(mbr)
PY
printf '#R 02\r\n#Z 01 E:\\XDPROBE.NAP@\r\n' > "$SCR/NEWDESK.INF"
touch -t "$STAMP" "$SCR/NEWDESK.INF"
cp -X "$NAP" "$SCR/XDPROBE.NAP"; touch -t "$STAMP" "$SCR/XDPROBE.NAP"
for start in $STARTS; do
    mformat -i "$CARD@@$((start * 512))" -T "$PSIZE" -h 16 -s 63 -H "$start" -c 4 -r 32 -L 128 \
        -N 4E543331 -v NTFS31 ::
done
for start in 63 131103; do
    mcopy -m -i "$CARD@@$((start * 512))" "$SCR/NEWDESK.INF" ::NEWDESK.INF
done
mcopy -m -i "$CARD@@$((262143 * 512))" "$SCR/XDPROBE.NAP" ::XDPROBE.NAP
mkdir "$SCR/stage-cd" "$SCR/stage-e"
cp -X "$SCR/NEWDESK.INF" "$SCR/stage-cd/"; cp -X "$SCR/XDPROBE.NAP" "$SCR/stage-e/"
tools/sd-manifest.sh files "$SCR/stage-cd" C "$CARD"
tools/sd-manifest.sh files "$SCR/stage-cd" D "$CARD"
tools/sd-manifest.sh files "$SCR/stage-e" E "$CARD"
tools/sd-manifest.sh write "$CARD" 256 tosdump/probes/run-xdprobe.sh --switches n/a > /dev/null
cp "$CARD.manifest" "$OUT/card.img.manifest"

{
    echo "xdprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    XDPROBE.PRG $(sha "$PRG") XDPROBE.NAP $(sha "$NAP")"
    echo "card      $(sha "$CARD") (built by this script)"
    echo "host_clang $(clang --version | head -1)"
} > "$OUT/run.txt"

fsck_after() { # <image>
    for start in 63 131103; do
        dd if="$1" of="$SCR/part.img" bs=512 skip="$start" count="$PSIZE" 2>/dev/null
        if fsck_msdos -n "$SCR/part.img" > "$OUT/fsck-$start.txt" 2>&1; then
            echo "fsck      partition at LBA $start after the run: fsck_msdos -n exit 0" >> "$OUT/run.txt"
        else
            echo "fsck      partition at LBA $start after the run: fsck_msdos -n exit $? (see fsck-$start.txt)" >> "$OUT/run.txt"
        fi
    done
}

if [ "$SIDE" = emutos ]; then
    HATARI_ORACLE_DISK="$CARD" HATARI_ORACLE_GEMDOS_DRIVE=F tools/hatari-oracle.sh disk "$PRG" - "$SCR/oracle" \
        > "$SCR/oracle.stdout" 2>&1 || { cat "$SCR/oracle.stdout" >&2; exit 1; }
    tr -d '\r' < "$SCR/oracle/XDPROBE.LOG" > "$OUT/XDPROBE.LOG"
    cp "$SCR/oracle/hatari.log" "$SCR/oracle/disk.sha256" "$OUT/"
    echo "oracle    tools/hatari-oracle.sh disk, GEMDOS drive F: (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
    fsck_after "$SCR/oracle/disk.img"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    cp -c "$CARD" "$SCR/boot.img" 2>/dev/null || cp "$CARD" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -q "XDPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 2
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state; stopped" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    mcopy -o -i "$SCR/boot.img@@$((262143 * 512))" ::XDPROBE.LOG "$SCR/XDPROBE.LOG" 2>/dev/null ||
        { echo "no XDPROBE.LOG on E:" >&2; cat "$OUT/run.txt" >&2; exit 1; }
    tr -d '\r' < "$SCR/XDPROBE.LOG" > "$OUT/XDPROBE.LOG"
    fsck_after "$SCR/boot.img"
fi
cat "$OUT/XDPROBE.LOG"
