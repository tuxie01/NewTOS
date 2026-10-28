#!/usr/bin/env bash
#
# run-gcprobe.sh -- the media-change probe tosdump/probes/gcprobe.c
# (TASK/FS20) on one side, with C: = a fresh clone of the oracle's test image.
#
#   tosdump/probes/run-gcprobe.sh emutos <out-dir>
#   tosdump/probes/run-gcprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh profile disk (Falcon030,
#            VGA, floppies off, the test image cloned as IDE master C:, the
#            probe on the GEMDOS drive D:, --auto D:\GCPROBE.PRG)
#   newtos   tools/mkcard.sh with MKCARD_TESTDISK (C: = the test image byte for
#            byte, the probe on the program partition D:), QEMU raspi4b booting
#            an APFS clone of the card with `-append serial=on`, the desktop's
#            #Z autorun starts D:\GCPROBE.NAP. For the input check build the
#            kernel with `cargo xtask build --board rpi4 --release --features
#            bios-arg-trace`; tosdump/probes/gcprobe-trace-check.py reads the
#            [biostrace] lines of <out-dir>/BOOT-SERIAL.txt.
#
# The probe is built here (make -C tosdump gcprobe-m68k / gcprobe-newtos). The
# test image is tools/mktestdisk.sh's with C:\NEWDESK.INF = "#R 02", "#Z 01
# D:\GCPROBE.NAP@" (the NewTOS autorun; EmuTOS reads EMUDESK.INF, so it is
# inert there) -- byte-reproducible, so both sides start from the same bytes.
#
# <out-dir> (must not exist) receives GCPROBE.LOG, testpart-after.img (the
# test partition after the run, LBA 63 + 130977 sectors; gitignored, *.img;
# the caller deletes it), test.img.manifest, run.txt (what ran, sha256 of
# probe, image, kernel, partition before and after) and the side's log
# (hatari.log + disk.sha256, or BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $GCPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos <out-dir> | newtos <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -ge 2 ] || usage
SIDE=$1; OUT=$2
case $SIDE in
    emutos) [ $# -eq 2 ] || usage ;;
    newtos) [ $# -eq 3 ] || usage; KERNEL=$3; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${GCPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set GCPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/gcprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

PART_START=63
PART_COUNT=130977
part() { dd if="$1" of="$2" bs=512 skip=$PART_START count=$PART_COUNT 2>/dev/null; }
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

printf '#R 02\r\n#Z 01 D:\\GCPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mktestdisk.sh "$SCR/test.img" NEWDESK.INF="$SCR/newdesk.inf" > "$SCR/mktestdisk.stdout"
cp "$SCR/test.img.manifest" "$OUT/test.img.manifest"
part "$SCR/test.img" "$SCR/testpart-before.img"

{
    echo "gcprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "test.img  $(sha "$SCR/test.img") (tools/mktestdisk.sh NEWDESK.INF=#R 02/#Z 01 D:\\GCPROBE.NAP@)"
    echo "testpart  before $(sha "$SCR/testpart-before.img")"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    make -s -C tosdump gcprobe-m68k > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
    PRG=$TD/build/GCPROBE.PRG
    echo "probe     GCPROBE.PRG $(sha "$PRG")" >> "$OUT/run.txt"
    HATARI_ORACLE_DISK="$SCR/test.img" tools/hatari-oracle.sh disk "$PRG" - "$SCR/oracle" > "$SCR/oracle.stdout" 2>&1 || {
        cat "$SCR/oracle.stdout" >&2; exit 1; }
    cp "$SCR/oracle/GCPROBE.LOG" "$SCR/oracle/hatari.log" "$SCR/oracle/disk.sha256" "$OUT/"
    part "$SCR/oracle/disk.img" "$OUT/testpart-after.img"
    echo "oracle    tools/hatari-oracle.sh disk (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
else
    make -s -C tosdump gcprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
    NAP=$TD/build/GCPROBE.NAP
    echo "probe     GCPROBE.NAP $(sha "$NAP")" >> "$OUT/run.txt"
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    MKCARD_TESTDISK="$SCR/test.img" tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" \
        GCPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh MKCARD_TESTDISK, D:\\GCPROBE.NAP)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -q "GCPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt" &&
           grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    # D: starts behind the 64-MiB test disk (MKCARD_TESTDISK).
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((131072 * 512))" ::/GCPROBE.LOG "$OUT/GCPROBE.LOG" \
        || { echo "no GCPROBE.LOG on D:" >&2; exit 1; }
    part "$SCR/boot.img" "$OUT/testpart-after.img"
fi
echo "testpart  after  $(sha "$OUT/testpart-after.img")" >> "$OUT/run.txt"
cat "$OUT/GCPROBE.LOG"
