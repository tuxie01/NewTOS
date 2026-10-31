#!/usr/bin/env bash
#
# run-hzprobe.sh -- the _hz_200 user-state probe tosdump/probes/hzprobe.c
# (TASK/PITEST01) in QEMU raspi4b. NewTOS only.
#
#   tosdump/probes/run-hzprobe.sh <out-dir> <kernel8.img>
#
# A card from tools/mkcard.sh (C:\HZPROBE.NAP, NEWDESK.INF "#Z 01
# C:\HZPROBE.NAP@": the desktop starts the probe, no key needed), an APFS
# clone booted with `-append serial=on`. The probe measures its default 30 s;
# the run stops when the serial log says the program exited (at most 300 s).
#
# The real-Pi step (A2 of the test scenario) needs no script: start
# HZPROBE.NAP from the desktop and read C:\HZPROBE.LOG afterwards.
#
# <out-dir> (must not exist) receives HZPROBE.LOG (from C: of the booted
# clone), run.txt (what ran, sha256 of probe, kernel, card before and after),
# SERIAL.txt, card.img.manifest.
#
# Scratch: $HZPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -eq 2 ] || usage
OUT=$1; KERNEL=$2
[ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${HZPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set HZPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/hzprobe.XXXXXX")
pid=
cleanup() {
    if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; fi
    rm -rf "$SCR"
}
trap cleanup EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
KERNEL=$(cd "$(dirname "$KERNEL")" && pwd)/$(basename "$KERNEL")
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump hzprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
NAP=$TD/build/HZPROBE.NAP

printf '#R 02\r\n#Z 01 C:\\HZPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "HZPROBE.NAP=$NAP" > "$SCR/mkcard.stdout" 2>&1 ||
    { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{
    echo "hzprobe run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probe     HZPROBE.NAP $(sha "$NAP")"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, defaults)"
    echo "kernel    $(sha "$KERNEL") $KERNEL"
} > "$OUT/run.txt"

cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
state=timeout
for _ in $(seq 1 150); do
    sleep 2
    if grep -q "HZPROBE.NAP' exited with rc" "$OUT/SERIAL.txt"; then state=done; break; fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
pid=
echo "qemu      $state; stopped" >> "$OUT/run.txt"
echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
# C: = MBR entry 1; its first sector is the LONG at byte 454.
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/HZPROBE.LOG "$SCR/HZPROBE.LOG" 2>/dev/null ||
    { echo "no HZPROBE.LOG on C:" >&2; exit 1; }
tr -d '\r' < "$SCR/HZPROBE.LOG" > "$OUT/HZPROBE.LOG"
cat "$OUT/HZPROBE.LOG"
