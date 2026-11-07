#!/usr/bin/env bash
# A142 one run: card with QED autorun (#Z), QEMU raspi4b on an APFS clone, QMP screendumps at the ticks of <shots.txt>
# after "[inj] up:QED armed".
#   run.sh <out-dir> <kernel8.img> <QED.NAP> <qed.rsc> <shots.txt>
# env A142_SCRATCH (session scratchpad, required).
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
BASE=${A142_SCRATCH:?}
OUT=$1; KERNEL=$2; NAP=$3; RSC=$4; SHOTS=$5
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
cp "$SHOTS" "$OUT/shots.txt"
printf '#R 02\r\n#Z 01 C:\\APPS\\QED\\QED.NAP@\r\n' > "$SCR/NEWDESK.INF"
: > "$SCR/CONFIG.TXT"
tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" "APPS/QED/QED.NAP=$NAP" "APPS/QED/qed.rsc=$RSC" "APPS/QED/CONFIG.TXT=$SCR/CONFIG.TXT" > "$SCR/mkcard.stdout" 2>&1 \
    || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{ echo "a142 run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "nap $(sha "$NAP")"; echo "rsc $(sha "$RSC")"
  echo "card $(sha "$SCR/card.img")"; echo "shots $(sha "$SHOTS")"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
    -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] up:QED armed at hz_200" &
spid=$!
wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "card-after $(sha "$SCR/boot.img")" >> "$OUT/run.txt"
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -E "Panic|Bus Error|Bus error" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
