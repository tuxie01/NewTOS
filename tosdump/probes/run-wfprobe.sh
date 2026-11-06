#!/usr/bin/env bash
#
# run-wfprobe.sh -- one case of the write-failure probe
# tosdump/probes/wfprobe.c (TASK/HW08) on one side, with C: = a fresh clone of
# the oracle's test image.
#
#   tosdump/probes/run-wfprobe.sh emutos <case> <out-dir>
#   tosdump/probes/run-wfprobe.sh newtos <case> <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh profile disk, the fault
#            a hook on hdv_rw inside the probe
#   newtos   tools/mkcard.sh with MKCARD_TESTDISK, QEMU raspi4b with `-append
#            serial=on`, the desktop's #Z autorun starts D:\WFPROBE.NAP; the
#            kernel needs the feature sd-fault (`cargo xtask build --board rpi4
#            --release --features sd-fault`).
#
# <out-dir> (must not exist) receives WFPROBE.LOG, fsck.txt and dir.txt (the
# test partition as it is after the run, with no shutdown: QEMU and Hatari are
# killed, what was not on the disk by then is lost), test.img.manifest, run.txt
# and the side's log (hatari.log + disk.sha256, or BOOT-SERIAL.txt +
# card.img.manifest). The disk images stay in the scratch and are deleted.
#
# WFPROBE_QMP=<socket> (newtos): a QMP socket for a screendump during the run.
# WFPROBE_COPY="NAME.EXT ..." copies files of the test partition after the run.
#
# Scratch: $WFPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
set -euo pipefail

usage() { echo "usage: $0 emutos <case> <out-dir> | newtos <case> <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -ge 3 ] || usage
SIDE=$1; CASE=$2; OUT=$3
case $SIDE in
    emutos) [ $# -eq 3 ] || usage ;;
    newtos) [ $# -eq 4 ] || usage; KERNEL=$4; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${WFPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set WFPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/wfprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

PART_START=63
PART_COUNT=130977
part() { dd if="$1" of="$2" bs=512 skip=$PART_START count=$PART_COUNT 2>/dev/null; }
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

printf '#R 02\r\n#Z 01 D:\\WFPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mktestdisk.sh "$SCR/test.img" NEWDESK.INF="$SCR/newdesk.inf" > "$SCR/mktestdisk.stdout"
cp "$SCR/test.img.manifest" "$OUT/test.img.manifest"

{
    echo "wfprobe run, case $CASE, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "test.img  $(sha "$SCR/test.img") (tools/mktestdisk.sh NEWDESK.INF=#R 02/#Z 01 D:\\WFPROBE.NAP@)"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    make -s -C tosdump wfprobe-m68k WFCASE="$CASE" > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
    PRG=$TD/build/WFPROBE.PRG
    echo "probe     WFPROBE.PRG $(sha "$PRG") WFCASE=$CASE" >> "$OUT/run.txt"
    HATARI_ORACLE_DISK="$SCR/test.img" tools/hatari-oracle.sh disk "$PRG" - "$SCR/oracle" > "$SCR/oracle.stdout" 2>&1 || {
        cat "$SCR/oracle.stdout" >&2; exit 1; }
    cp "$SCR/oracle/WFPROBE.LOG" "$SCR/oracle/hatari.log" "$SCR/oracle/disk.sha256" "$OUT/"
    part "$SCR/oracle/disk.img" "$SCR/after.img"
    echo "oracle    tools/hatari-oracle.sh disk (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
else
    make -s -C tosdump wfprobe-newtos WFCASE="$CASE" > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
    NAP=$TD/build/WFPROBE.NAP
    echo "probe     WFPROBE.NAP $(sha "$NAP") WFCASE=$CASE" >> "$OUT/run.txt"
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    MKCARD_TESTDISK="$SCR/test.img" tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" \
        WFPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh MKCARD_TESTDISK, D:\\WFPROBE.NAP)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    set -- -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw
    if [ -n "${WFPROBE_QMP:-}" ]; then
        set -- "$@" -qmp unix:"$WFPROBE_QMP",server=on,wait=off
    fi
    qemu-system-aarch64 "$@" &
    pid=$!
    state=timeout
    for _ in $(seq 1 240); do
        sleep 1
        if grep -q "WFPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt" &&
           grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        if grep -q "WFPROBE power off" "$OUT/BOOT-SERIAL.txt"; then state=poweroff; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 1
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    # D: starts behind the 64-MiB test disk (MKCARD_TESTDISK).
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((131072 * 512))" ::/WFPROBE.LOG "$OUT/WFPROBE.LOG" ||
        echo "no WFPROBE.LOG on D: (the serial log has the text)" >> "$OUT/run.txt"
    part "$SCR/boot.img" "$SCR/after.img"
fi
echo "testpart  after  $(sha "$SCR/after.img")" >> "$OUT/run.txt"
fsck_msdos -n "$SCR/after.img" > "$OUT/fsck.txt" 2>&1 || echo "fsck_msdos exit $?" >> "$OUT/fsck.txt"
MTOOLS_SKIP_CHECK=1 mdir -i "$SCR/after.img" :: > "$OUT/dir.txt" 2>&1 || true
# WFPROBE_COPY="NAME.EXT ...": files of the test partition as they are now, into <out-dir>.
for f in ${WFPROBE_COPY:-}; do
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/after.img" "::/$f" "$OUT/$f" 2>/dev/null || echo "no $f on the partition" >> "$OUT/run.txt"
done
