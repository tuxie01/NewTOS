#!/bin/bash
# txbench-run.sh <out-dir> <kernel8.img> : boot TXBENCH.NAP via #Z autorun, read C:\TXBENCH.LOG
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
OUT=$1; KERNEL=$2; NAP=$S/txb/TXBENCH.NAP
[ ! -e "$OUT" ] || { echo exists >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/txb-run.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
printf '#R 02\r\n#Z 01 C:\\TXBENCH.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" TXBENCH.NAP="$NAP" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/"; cp -c "$SCR/card.img" "$SCR/boot.img"
echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1) nap $(shasum -a 256 "$NAP" | cut -d' ' -f1)" > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
state=timeout
for _ in $(seq 1 120); do sleep 2
  if grep -a -q "TXBENCH.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2; kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "qemu $state" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/TXBENCH.LOG "$OUT/TXBENCH.LOG" || echo "no LOG" >> "$OUT/run.txt"
cat "$OUT/run.txt" "$OUT/TXBENCH.LOG" 2>&1
