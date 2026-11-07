#!/usr/bin/env bash
# A142-test: one QEMU raspi4b run on an APFS clone of a fresh card, QMP screendumps at the ticks of <shots.txt> after the injector arms.
# (structure of the developer's measurement-2026-11-07-a142/method/run.sh, which it reuses: tools/mkcard.sh, tosdump/probes/shprobe-qmpshots.py)
#   run.sh <out-dir> <kernel8.img> <PROG> <prog.NAP> <rsc|-> <shots.txt>      PROG = QED or ALT (name = armed program, folder APPS/<PROG>/)
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
BASE=${A142_SCRATCH:?}
OUT=$1; KERNEL=$2; PROG=$3; NAP=$4; RSC=$5; SHOTS=$6
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
cp "$SHOTS" "$OUT/shots.txt"
printf '#R 02\r\n#Z 01 C:\\APPS\\%s\\%s.NAP@\r\n' "$PROG" "$PROG" > "$SCR/NEWDESK.INF"
: > "$SCR/CONFIG.TXT"
ARGS=("APPS/$PROG/$PROG.NAP=$NAP")
[ "$RSC" = "-" ] || ARGS+=("APPS/$PROG/qed.rsc=$RSC" "APPS/$PROG/CONFIG.TXT=$SCR/CONFIG.TXT")
tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" "${ARGS[@]}" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{ echo "a142-test run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "nap $(sha "$NAP")"; [ "$RSC" = "-" ] || echo "rsc $(sha "$RSC")"
  echo "card $(sha "$SCR/card.img")"; echo "shots $(sha "$SHOTS")"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
    -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] up:$PROG armed at hz_200" &
spid=$!
wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -E "Panic|Bus Error|Bus error|exception" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
