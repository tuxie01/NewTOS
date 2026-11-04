#!/usr/bin/env bash
# run-shot.sh <scratch> <out-dir> <shot-dir> <kernel8.img> "<append>" <cases> <WxH> <wait-s>
# One QEMU raspi4b boot of an APFS clone (cp -c) of <scratch>/base.img
# (make-card.sh) with C:\CWCASE.TXT in the clone overwritten by <cases>
# (mtools, partition 1 at LBA 63 as tools/mkcard.sh lays it out), display
# WxH (bcm2835-fb xres/yres; the kernel renders at the size the VideoCore
# reports), -append "<append>" (may be empty: serial off), serial to
# <out-dir>/SERIAL.txt, QMP on a socket in <scratch> (the cwd). After
# <wait-s> seconds a QMP screendump to <shot-dir>/<run>.png (<run> = the
# last component of <out-dir>), 5 s later a second one (<run>-5s.png) --
# equal hashes = the screen stands still (the report waits for a key).
# Records kernel, card before/after and the end state in <out-dir>/run.txt;
# the clone is deleted afterwards.
set -euo pipefail
SCR=$1; OUT=$2; SHOTS=$3; KERNEL=$4; APPEND=$5; CASES=$6; GEOM=$7; WAIT=$8
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
mkdir -p "$SHOTS"
SHOTS=$(cd "$SHOTS" && pwd)
RUN=$(basename "$OUT")
M=$(cd "$(dirname "$0")" && pwd)
W=${GEOM%x*}; H=${GEOM#*x}
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
CLONE="$SCR/boot-$$.img"
SOCK="qmp-$$.sock"   # relative to $SCR: a unix socket path must stay < 104 bytes
cp -c "$SCR/base.img" "$CLONE"
printf '%s' "$CASES" > "$SCR/case-$$.txt"
mcopy -o -i "$CLONE@@$((63 * 512))" "$SCR/case-$$.txt" ::CWCASE.TXT
rm -f "$SCR/case-$$.txt"
pid=
cleanup() {
    if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; fi
    rm -f "$CLONE" "$SCR/$SOCK"
}
trap cleanup EXIT
{
    echo "run $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "kernel    $(sha "$KERNEL") $KERNEL"
    echo "append    \"$APPEND\""
    echo "cases     $CASES"
    echo "display   ${W}x${H} (-global bcm2835-fb.xres/yres)"
    echo "card      $(sha "$CLONE") (APFS clone of base.img, C:\\CWCASE.TXT = \"$CASES\")"
} > "$OUT/run.txt"
: > "$OUT/SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/SERIAL.txt" \
    -global bcm2835-fb.xres="$W" -global bcm2835-fb.yres="$H" \
    -qmp unix:"$SOCK",server=on,wait=off \
    -append "$APPEND" -kernel "$KERNEL" -drive file="$CLONE",if=sd,format=raw &
pid=$!
sleep "$WAIT"
kill -0 "$pid" 2>/dev/null || { echo "qemu      exited before the screendump" >> "$OUT/run.txt"; exit 1; }
python3 "$M/qmpshot.py" "$SOCK" "$SHOTS/$RUN.png" > "$OUT/qmp.txt"
sleep 5
python3 "$M/qmpshot.py" "$SOCK" "$SHOTS/$RUN-5s.png" >> "$OUT/qmp.txt"
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
pid=
echo "qemu      screendumps after ${WAIT} s and $((WAIT + 5)) s; stopped" >> "$OUT/run.txt"
echo "screen    $(sha "$SHOTS/$RUN.png") $RUN.png" >> "$OUT/run.txt"
echo "screen    $(sha "$SHOTS/$RUN-5s.png") $RUN-5s.png" >> "$OUT/run.txt"
echo "card      after $(sha "$CLONE") (the booted clone)" >> "$OUT/run.txt"
