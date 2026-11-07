#!/usr/bin/env bash
# desk-run.sh <out-dir> <kernel8.img> <NEWDESK.INF> <shots.txt> <scratch>
# Card by mkcard3.sh (C: D:, COPYME.TXT slot 2 on both), QEMU raspi4b on an APFS clone, QMP shots as in
# measurements/measurement-2026-11-06-v44-test/method/desk-inj.sh, then the files and fsck of both partitions off the booted clone.
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
OUT=$1; KERNEL=$2; INF=$3; SHOTS=$4; SCR=$5
mkdir -p "$OUT/shots" "$SCR"; OUT=$(cd "$OUT" && pwd); cd "$REPO"
M=measurements/measurement-2026-11-06-hw08-fix-test/method
$M/mkcard3.sh "$SCR/card.img" "$INF" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"; cp "$INF" "$OUT/NEWDESK.INF"; cp "$SHOTS" "$OUT/shots.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
{ echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)"; echo "card before $(shasum -a 256 "$SCR/card.img" | cut -d' ' -f1)"; } > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" ${QEMU_EXTRA:-} \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] desktop armed at hz_200" || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "card after  $(shasum -a 256 "$SCR/boot.img" | cut -d' ' -f1)" >> "$OUT/run.txt"
export MTOOLSRC=/dev/null MTOOLS_SKIP_CHECK=1 MTOOLS_NO_VFAT=1
for p in "C 63" "D 131103"; do set -- $p
  for f in COPYME.TXT NEWDESK.INF; do
    mcopy -n -i "$SCR/boot.img@@$(($2 * 512))" "::$f" "$SCR/$1-$f" 2>/dev/null || true
    echo "$1:\\$f sha256 $(shasum -a 256 < "$SCR/$1-$f" | cut -c1-16) $(wc -c < "$SCR/$1-$f" 2>/dev/null || echo missing) bytes: $(cat "$SCR/$1-$f" 2>/dev/null | head -c 60)" >> "$OUT/run.txt"
    cp "$SCR/$1-$f" "$OUT/after-$1-$f" 2>/dev/null || true
  done
  mdir -i "$SCR/boot.img@@$(($2 * 512))" :: > "$OUT/mdir-$1.txt" 2>&1 || true
  dd if="$SCR/boot.img" of="$SCR/part.img" bs=512 skip=$2 count=130977 2>/dev/null
  if fsck_msdos -n "$SCR/part.img" > "$OUT/fsck-$1.txt" 2>&1; then echo "fsck $1: exit 0" >> "$OUT/run.txt"; else echo "fsck $1: exit $?" >> "$OUT/run.txt"; fi
done
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -i -E "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
