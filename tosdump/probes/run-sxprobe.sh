#!/usr/bin/env bash
#
# run-sxprobe.sh -- the Setexc probe tosdump/probes/sxprobe.c (TASK/BIOS07)
# on one side. No disk: the probe touches no drive but its own.
#
#   tosdump/probes/run-sxprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-sxprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh <profile> (C: = the
#            GEMDOS drive with SXPROBE.PRG, --auto C:\SXPROBE.PRG)
#   newtos   tools/mkcard.sh (C:\SXPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\SXPROBE.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun starts
#            the probe. BOOT-SERIAL.txt carries the kernel's `[hook] warning:`
#            line for vector 0x20 (ADR-031).
#
# <out-dir> (must not exist) receives SXPROBE.LOG, run.txt (what ran, sha256
# of the probe, card, kernel) and the side's log (hatari.log, or
# BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $SXPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos tt|falcontc <out-dir> | newtos <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -eq 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) PROFILE=$2; OUT=$3
            case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    newtos) OUT=$2; KERNEL=$3; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${SXPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set SXPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/sxprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump sxprobe-m68k sxprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/SXPROBE.PRG
NAP=$TD/build/SXPROBE.NAP
{
    echo "sxprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    SXPROBE.PRG $(sha "$PRG") SXPROBE.NAP $(sha "$NAP")"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    tools/hatari-oracle.sh "$PROFILE" "$PRG" - "$SCR/oracle" > "$SCR/oracle.stdout" 2>&1 || {
        cat "$SCR/oracle.stdout" >&2; exit 1; }
    cp "$SCR/oracle/SXPROBE.LOG" "$SCR/oracle/hatari.log" "$OUT/"
    echo "oracle    tools/hatari-oracle.sh $PROFILE (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\SXPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" SXPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\SXPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -q "SXPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt" &&
           grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/SXPROBE.LOG "$OUT/SXPROBE.LOG" \
        || { echo "no SXPROBE.LOG on C:" >&2; exit 1; }
fi
cat "$OUT/SXPROBE.LOG"
