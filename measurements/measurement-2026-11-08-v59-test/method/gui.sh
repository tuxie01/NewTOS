#!/bin/bash
# V59 tester: desktop scenes with the IKBD injector, base vs change, 640x480 and 1920x1080.
#   S=<scratch> gui.sh arm | shots
# arm:   armed rpi4 release kernels ($S/k/arm-{desk,edge}-{base,chg}.img) from archive copies (tools/inject-arm.py desktop=<script>)
# shots: QEMU raspi4b per scene, PNG per tick (shprobe-qmpshots.py) + boot screen/console (timed screendumps).
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a194b0b2cc3f6d929
MAIN=/Volumes/Daten/Development/PiRusTOS
S=${S:?}
M=$W/measurements/measurement-2026-11-08-v59-test; A=$M/analysis; R=$S/r; G=$R/gui
V51=$W/measurements/measurement-2026-11-07-v51/method
mkdir -p "$G" "$A" "$S/files"
reap() { kill "$1" 2>/dev/null; sleep 1; kill -9 "$1" 2>/dev/null; wait "$1" 2>/dev/null
  if kill -0 "$1" 2>/dev/null; then echo "$(date +%H:%M:%S) $2 pid $1 STILL ALIVE" >> "$A/qemu-pids.txt"
  else echo "$(date +%H:%M:%S) $2 pid $1 gone" >> "$A/qemu-pids.txt"; fi; }
do_arm() {
  for sc in desk:$V51/desk-mouse.txt edge:$M/method/edge-mouse.txt; do
    for T in base chg; do
      rm -rf "$S/t/arm-$T"; rsync -a --exclude /target "$S/t/$T/" "$S/t/arm-$T/"
      (cd "$W" && python3 -I tools/inject-arm.py "$S/t/arm-$T" "desktop=${sc#*:}") > "$G/arm-${sc%%:*}-$T.txt" 2>&1
      (cd "$S/t/arm-$T" && cargo xtask build --board rpi4 --release) > "$G/arm-${sc%%:*}-$T-build.log" 2>&1; echo "armed build ${sc%%:*} $T rc $?" >> "$G/arm-${sc%%:*}-$T.txt"
      cp "$S/t/arm-$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/k/arm-${sc%%:*}-$T.img"
      rm -rf "$S/t/arm-$T"
    done
  done
  (cd "$S/k" && shasum -a 256 arm-*.img) > "$A/armed-kernels.sha256"
}
desk() { # desk <out> <kernel> <qemu-extra> <shots.txt>
  local OUT=$1 K=$2 X=$3 SH=$4 SC
  mkdir -p "$OUT/shots"; SC=$(mktemp -d "$S/dk.XXXXXX")
  local PAIRS=(); for f in "$S/files"/*; do PAIRS+=("$(basename "$f")=$f"); done
  (cd "$W" && tools/mkcard.sh "$SC/card.img" "$V51/desk-NEWDESK.INF" "${PAIRS[@]}") > "$SC/mk.out" 2>&1 || { cat "$SC/mk.out"; return 1; }
  cp "$SC/card.img.manifest" "$OUT/"; cp -c "$SC/card.img" "$SC/boot.img"
  echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1) card $(shasum -a 256 "$SC/card.img" | cut -d' ' -f1) extra [$X]" > "$OUT/run.txt"
  : > "$OUT/BOOT-SERIAL.txt"
  ( cd "$SC" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" $X \
      -append serial=on -kernel "$K" -drive file="$SC/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off ) &
  local pid=$!; echo "$(date +%H:%M:%S) $(basename "$OUT") pid $pid started" >> "$A/qemu-pids.txt"
  for _ in $(seq 1 100); do [ -S "$SC/qmp.sock" ] && break; sleep 0.1; done; sleep 2
  (cd "$SC" && python3 -I "$W/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$SH" "$OUT/shots" "[inj] desktop armed at hz_200") > "$OUT/shots.log" 2>&1 || echo "qmpshots failed" >> "$OUT/run.txt"
  sleep 1; reap $pid "$(basename "$OUT")"
  echo "exception-panic-lines $(grep -a -c -iE 'exception|panic' "$OUT/BOOT-SERIAL.txt")" >> "$OUT/run.txt"
  rm -rf "$SC"
}
bootcon() {
  local OUT=$1 K=$2 X=$3 SC
  mkdir -p "$OUT/shots"; SC=$(mktemp -d "$S/bc.XXXXXX")
  printf '#R 02\r\n' > "$SC/NEWDESK.INF"
  (cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF" AUTO/APITEST.NCP="$MAIN/sdk/apps/apitest/APITEST.NCP") > "$SC/mk.out" 2>&1 || { cat "$SC/mk.out"; return 1; }
  cp "$SC/card.img.manifest" "$OUT/"; cp -c "$SC/card.img" "$SC/boot.img"
  echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1) extra [$X]" > "$OUT/run.txt"
  : > "$OUT/BOOT-SERIAL.txt"
  ( cd "$SC" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" $X \
      -append serial=on -kernel "$K" -drive file="$SC/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off ) &
  local pid=$!; echo "$(date +%H:%M:%S) $(basename "$OUT") pid $pid started" >> "$A/qemu-pids.txt"
  (cd "$SC" && python3 -I "$V51/timed-shots.py" qmp.sock "$OUT/shots" 6 10 18 24 32 45) > "$OUT/shots.log" 2>&1 || true
  reap $pid "$(basename "$OUT")"
  echo "exception-panic-lines $(grep -a -c -iE 'exception|panic' "$OUT/BOOT-SERIAL.txt")" >> "$OUT/run.txt"
  rm -rf "$SC"
}
do_shots() {
  for i in $(seq -w 1 40); do printf 'V49 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      desk "$G/desk$n-$T" "$S/k/arm-desk-$T.img" "$x" "$V51/desk-shots.txt"
      desk "$G/edge$n-$T" "$S/k/arm-edge-$T.img" "$x" "$M/method/edge-shots.txt"
      bootcon "$G/bc$n-$T" "$S/k/$T-rpi4.img" "$x"
    done
  done
  : > "$A/shots-compare.txt"
  for r in desk480 desk1080 edge480 edge1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")" >> "$A/shots-compare.txt"
  done
}
for ph in "$@"; do echo "=== $ph $(date +%H:%M:%S)"; do_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
