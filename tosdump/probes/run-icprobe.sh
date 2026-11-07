#!/usr/bin/env bash
#
# run-icprobe.sh -- the G_CICON probe tosdump/probes/icprobe.c (TASK/A141) on
# one side, for one resource file. No disk, no input. Copied from run-vdprobe.sh.
#
#   tosdump/probes/run-icprobe.sh emutos falcontc <out-dir> <classic.rsc> <ntree>
#   tosdump/probes/run-icprobe.sh newtos <out-dir> <kernel8.img> <native.rsc> <ntree>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh falcontc, the classic
#            file as C:\ICPROBE.RSC
#   newtos   tools/mkcard.sh (C:\ICPROBE.NAP, the tools/mkrsc.py conversion of
#            the same file as C:\ICPROBE.RSC, NEWDESK.INF "#R 02",
#            "#Z 01 C:\ICPROBE.NAP@"), QEMU raspi4b on an APFS clone of the card
#            with `-append serial=on`.
#
# <out-dir> (must not exist) receives ICPROBE.LOG, ICPROBE.BIN, run.txt and the
# side's log. Compare two runs with tosdump/probes/icprobe-compare.py.
#
# Scratch: $ICPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos falcontc <out-dir> <classic.rsc> <ntree> | newtos <out-dir> <kernel8.img> <native.rsc> <ntree>" >&2; exit 2; }
[ $# -eq 5 ] || usage
SIDE=$1
case $SIDE in
    emutos) PROFILE=$2; OUT=$3; RSC=$4; NTREE=$5 ;;
    newtos) OUT=$2; KERNEL=$3; RSC=$4; NTREE=$5; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ -f "$RSC" ] || { echo "resource not found: $RSC" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${ICPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set ICPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/icprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
RSC=$(cd "$(dirname "$RSC")" && pwd)/$(basename "$RSC")
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump icprobe-m68k icprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/ICPROBE.PRG
NAP=$TD/build/ICPROBE.NAP
printf '%s\r\n' "$NTREE" > "$SCR/ICPROBE.TXT"
{
    echo "icprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    ICPROBE.PRG $(sha "$PRG") ICPROBE.NAP $(sha "$NAP")"
    echo "resource  $(sha "$RSC") $RSC ($NTREE trees)"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    HATARI_ORACLE_STAGE="ICPROBE.RSC=$RSC ICPROBE.TXT=$SCR/ICPROBE.TXT" \
        tools/hatari-oracle.sh "$PROFILE" "$PRG" - "$SCR/oracle" > "$SCR/oracle.stdout" 2>&1 || {
        cat "$SCR/oracle.stdout" >&2; exit 1; }
    cp "$SCR/oracle/ICPROBE.LOG" "$SCR/oracle/hatari.log" "$OUT/"
    cp "$SCR/oracle/hd/ICPROBE.BIN" "$OUT/"
    echo "oracle    tools/hatari-oracle.sh $PROFILE (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\ICPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" ICPROBE.NAP="$NAP" ICPROBE.RSC="$RSC" ICPROBE.TXT="$SCR/ICPROBE.TXT" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\ICPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 120); do
        sleep 2
        if grep -q "ICPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt" &&
           grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/ICPROBE.LOG "$OUT/ICPROBE.LOG" \
        || { echo "no ICPROBE.LOG on C:" >&2; exit 1; }
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/ICPROBE.BIN "$OUT/ICPROBE.BIN" \
        || { echo "no ICPROBE.BIN on C:" >&2; exit 1; }
fi
cat "$OUT/ICPROBE.LOG"
