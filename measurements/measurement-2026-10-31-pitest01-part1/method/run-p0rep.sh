#!/usr/bin/env bash
#
# run-p0rep.sh <out-dir> <kernel8.img> [<max-seconds>] -- P0REP.NAP (the P0PAR
# repeat run, sdk/examples/p0rep.c, default 1000 rounds) with P0KID.NAP in QEMU
# raspi4b, started by the desktop's #Z autorun, no key, no injector.
# <kernel8.img> should be the crash-nowait kernel (`cargo xtask build --board
# rpi4 --release --features crash-nowait`); with the release kernel the first
# fault case waits for a key that this script never gives (the run then ends
# by <max-seconds>).
#
# Card: tools/mkcard.sh (defaults), C:\P0REP.NAP, C:\P0KID.NAP, NEWDESK.INF
# "#Z 01 C:\P0REP.NAP@". Booted on an APFS clone with -append serial=on and a
# QMP socket; one screendump when "P0PAR-REP: round 100" appears (the crash
# reports on screen). Stops when the serial log says P0REP exited, or after
# <max-seconds> (default 7200).
#
# <out-dir> (must not exist) receives P0PAR.LOG (from C: of the booted clone,
# CR stripped), SERIAL.txt, run.txt (sha256 of programs, kernel, card before
# and after, wall time), card.img.manifest, round100.png.
#
# Scratch: $P0REP_SCRATCH (never the shared temp dir itself); deleted at the end.
set -euo pipefail
[ $# -ge 2 ] || { echo "usage: $0 <out-dir> <kernel8.img> [<max-seconds>]" >&2; exit 2; }
OUT=$1; KERNEL=$2; MAXS=${3:-7200}
[ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
BASE=${P0REP_SCRATCH:?set P0REP_SCRATCH}
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/p0rep.XXXXXX")
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

bash sdk/build.sh sdk/examples/p0rep.c "$SCR/P0REP.NAP" > "$SCR/build.txt" 2>&1
bash sdk/build.sh sdk/examples/p0kid.c "$SCR/P0KID.NAP" >> "$SCR/build.txt" 2>&1
cc -o "$SCR/qmpshot" tools/qmpshot.c
printf '#R 02\r\n#Z 01 C:\\P0REP.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" P0REP.NAP="$SCR/P0REP.NAP" P0KID.NAP="$SCR/P0KID.NAP" \
    > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{
    echo "p0rep run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) + working tree"
    echo "programs  P0REP.NAP $(sha "$SCR/P0REP.NAP") P0KID.NAP $(sha "$SCR/P0KID.NAP")"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, defaults)"
    echo "kernel    $(sha "$KERNEL") $KERNEL"
} > "$OUT/run.txt"

cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/SERIAL.txt"
# The QMP socket path is relative (a unix socket path must stay < 104 bytes):
# QEMU and qmpshot both run in $SCR.
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/SERIAL.txt" \
    -qmp unix:qmp.sock,server,nowait \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
t0=$(date +%s)
state=timeout; shot=0
while [ $(( $(date +%s) - t0 )) -lt "$MAXS" ]; do
    sleep 5
    if [ $shot = 0 ] && grep -q "P0PAR-REP: round 100 " "$OUT/SERIAL.txt"; then
        ./qmpshot qmp.sock round100.ppm >/dev/null 2>&1 &&
            sips -s format png "$SCR/round100.ppm" --out "$OUT/round100.png" >/dev/null 2>&1 || true
        shot=1
    fi
    if grep -q "P0REP.NAP' exited with rc" "$OUT/SERIAL.txt"; then state=done; break; fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
t1=$(date +%s)
sleep 2
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
pid=
echo "qemu      $state after $((t1 - t0)) s; stopped" >> "$OUT/run.txt"
echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/P0PAR.LOG "$SCR/P0PAR.LOG" 2>/dev/null ||
    { echo "no P0PAR.LOG on C:" >&2; exit 1; }
tr -d '\r' < "$SCR/P0PAR.LOG" > "$OUT/P0PAR.LOG"
cat "$OUT/P0PAR.LOG"
