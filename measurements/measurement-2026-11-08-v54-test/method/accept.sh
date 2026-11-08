#!/bin/bash
# V54 acceptance: base $BASE (main before V54) against the V54 change ($CHG).
#   S=<scratch> BASE=<commit> CHG=<commit> accept.sh <phase>...
#   phases: trees builds virtimg boots tables vdprobe wkprobe gui
# Adapted from measurements/measurement-2026-11-08-v59/method/accept.sh (main checkout instead of a worktree; the
# wkprobe phase adds the oracle side and the e/n comparisons). Every QEMU here is started by this script or by a
# committed probe runner and killed by the PID recorded at its launch, never by name or pattern; after each kill the
# PID is checked to be gone (analysis/qemu-pids.txt; the probe runners kill their own QEMU by its PID). Every tree is a
# `git archive` of its commit into $S/t/<name> with references/ linked from the main checkout; nothing is built in the
# main checkout. The probes (tosdump/probes/*) run from the main checkout at $CHG: base and change kernels meet the
# same probe source (wkprobe version 2, commit 61989b6d).
set -u
W=/Volumes/Daten/Development/PiRusTOS
MAIN=$W
S=${S:?}
BASE=${BASE:?}
CHG=${CHG:?}
M=$W/measurements/measurement-2026-11-08-v54-test
A=$M/analysis
V51=$W/measurements/measurement-2026-11-07-v51/method
mkdir -p "$S/t" "$S/k" "$A" "$M/runs"
export S

reap() {
  kill "$1" 2>/dev/null; sleep 1; kill -9 "$1" 2>/dev/null; wait "$1" 2>/dev/null
  if kill -0 "$1" 2>/dev/null; then echo "$(date +%H:%M:%S) $2 pid $1 STILL ALIVE" >> "$A/qemu-pids.txt"
  else echo "$(date +%H:%M:%S) $2 pid $1 gone" >> "$A/qemu-pids.txt"; fi
}

tree() { # tree <name> <commit>
  rm -rf "$S/t/$1"; mkdir -p "$S/t/$1"
  (cd "$W" && git archive "$2") | tar -x -C "$S/t/$1"
  rm -rf "$S/t/$1/references"; ln -s "$MAIN/references" "$S/t/$1/references"
  echo "tree $1 = $(cd "$W" && git rev-parse "$2")" | tee -a "$A/trees.txt"
}

phase_trees() { : > "$A/trees.txt"; tree base $BASE; tree chg $CHG; }

phase_builds() {
  : > "$A/builds-rc.txt"
  for T in base chg; do
    cd "$S/t/$T" || exit 1
    for B in rpi4 qemu-virt; do
      rm -rf target/aarch64-unknown-none-softfloat
      cargo xtask build --board $B > "$A/build-$T-$B.log" 2>&1; echo "build $T $B rc $?" >> "$A/builds-rc.txt"
      cargo clippy -p kernel --features board-$B > "$A/clippy-$T-kernel-$B.txt" 2>&1; echo "clippy kernel $T $B rc $?" >> "$A/builds-rc.txt"
    done
    cargo clippy -p tos-api > "$A/clippy-$T-tosapi.txt" 2>&1; echo "clippy tos-api $T rc $?" >> "$A/builds-rc.txt"
    cargo clippy -p tos-api --tests > "$A/clippy-$T-tosapi-tests.txt" 2>&1; echo "clippy tos-api --tests $T rc $?" >> "$A/builds-rc.txt"
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board rpi4 --release > "$A/build-$T-rpi4-release.log" 2>&1; echo "build $T rpi4 release rc $?" >> "$A/builds-rc.txt"
    cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/$T-rpi4.img"
  done
  rm -rf "$S/t/base/target/aarch64-unknown-none-softfloat" "$S/t/chg/target/aarch64-unknown-none-softfloat"
  (cd "$S/k" && shasum -a 256 base-rpi4.img chg-rpi4.img) | tee "$A/kernels.sha256"
  for f in "$A"/build-*.log; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | tr '\n' ' ')"; done > "$A/warnings.txt"
  for f in "$A"/clippy-*.txt; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | sed 's/ (run.*//' | awk '{s+=$(NF-1)} END {print NR" lines, sum "s}')"; done >> "$A/warnings.txt"
  for f in "$A"/clippy-*.txt; do echo "$(basename "$f") hits in touched files: $(grep -cE -- '--> (tos-api/src/vdi/mouse\.rs|tos-api/tests/vdi_v54_vsc_form_no_redraw\.rs)' "$f")"; done >> "$A/warnings.txt"
  echo BUILDSDONE >> "$A/builds-rc.txt"
}

phase_virtimg() {
  for T in base chg; do
    cd "$S/t/$T" || exit 1
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board qemu-virt --release > "$A/build-$T-qemu-virt-release-img.log" 2>&1; echo "build $T qemu-virt release (img) rc $?" >> "$A/builds-rc.txt"
    OC=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-objcopy
    "$OC" -O binary target/aarch64-unknown-none-softfloat/release/kernel "$S/k/$T-virt.img"
    rm -rf target/aarch64-unknown-none-softfloat
  done
  (cd "$S/k" && shasum -a 256 base-virt.img chg-virt.img) | tee -a "$A/kernels.sha256"
}

phase_boots() {
  for T in base chg; do
    O=$M/runs/boot-virt-$T; mkdir -p "$O"
    echo "kernel $(shasum -a 256 "$S/k/$T-virt.img" | cut -d' ' -f1)" > "$O/run.txt"
    qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
      -device qemu-xhci -device usb-kbd -device usb-mouse -display none -serial file:"$O/serial.txt" -no-reboot \
      -kernel "$S/k/$T-virt.img" &
    pid=$!; echo "$(date +%H:%M:%S) boot-virt-$T pid $pid started" >> "$A/qemu-pids.txt"
    sleep 100; reap $pid boot-virt-$T
    { grep -a -c -iE "exception|panic" "$O/serial.txt" | sed 's/^/exception-panic-lines /'; grep -a "desktop up" "$O/serial.txt"; } | tee -a "$O/run.txt"
  done
}

phase_tables() {
  cd "$W"
  R=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --reference $R/$REF.BIN --scratch "$S/xt-$T" > "$M/runs/$O.stdout" 2>&1
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --scratch "$S/xt-$T" > "$M/runs/$O.stdout" 2>&1
    fi; echo "$O rc $?" >> "$A/tables-rc.txt"; }
  : > "$A/tables-rc.txt"
  for T in base chg; do
    run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
    run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
    run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
    run $T aes aes-r4-subfunctions.tsv tab-$T-r4
  done
  for t in all r2 r3 r4; do
    for f in "$M/runs/tab-base-$t"/*.BIN; do
      g="$M/runs/tab-chg-$t/$(basename "$f")"
      if cmp -s "$f" "$g"; then echo "$t $(basename "$f") identical $(shasum -a 256 "$f" | cut -c1-16)"
      else echo "$t $(basename "$f") DIFFER: $(cmp -l "$f" "$g" | wc -l | tr -d ' ') bytes"; fi
    done
  done > "$A/tables-bin-compare.txt"
}

phase_vdprobe() {
  export VDPROBE_SCRATCH=$S/vp; mkdir -p "$VDPROBE_SCRATCH"
  P=$W/tosdump/probes; O=$M/runs/vdprobe; mkdir -p "$O"
  for n in n-base:base n-chg:chg; do
    "$P/run-vdprobe.sh" newtos "$O/${n%%:*}" "$S/k/${n#*:}-rpi4.img" > "$O/${n%%:*}.stdout" 2>&1
  done
  python3 -I "$P/vdprobe-compare.py" "$O/n-base" "$O/n-chg" > "$O/compare-n-base-n-chg.txt" 2>&1
  cmp "$O/n-base/VDPROBE.BIN" "$O/n-chg/VDPROBE.BIN" > "$O/cmp-bin-n-base-n-chg.txt" 2>&1; echo "cmp BIN exit $?" >> "$O/cmp-bin-n-base-n-chg.txt"
  diff "$O/n-base/VDPROBE.LOG" "$O/n-chg/VDPROBE.LOG" > "$O/log-n-base-n-chg.diff"; echo "diff LOG exit $?" >> "$O/log-n-base-n-chg.diff"
  (cd "$O" && for d in n-base n-chg; do shasum -a 256 $d/VDPROBE.BIN; done) > "$O/bin.sha256"
  for d in n-base n-chg; do rm -f "$O/$d/VDPROBE.BIN"; done # 27 MB each; sha256 kept
}

phase_wkprobe() { # wkprobe version 2 (F1/F2 + the AES counter-check G1/G2); armed kernels feed its packets
  export WKPROBE_SCRATCH=$S/wk; mkdir -p "$WKPROBE_SCRATCH"
  P=$W/tosdump/probes; O=$M/runs/wkprobe; mkdir -p "$O"
  [ -d "$O/e1" ] || "$P/run-wkprobe.sh" emutos falcontc "$O/e1" > "$O/e1.stdout" 2>&1
  "$P/run-wkprobe.sh" emutos falcontc "$O/e2" > "$O/e2.stdout" 2>&1; echo "wk oracle e2 rc $?" >> "$O/rc.txt"
  for T in base chg; do
    "$P/run-wkprobe.sh" kernel "$S/t/$T" "$S/k/wk-$T.img" > "$O/kernel-$T.stdout" 2>&1; echo "wk kernel $T rc $?" >> "$O/rc.txt"
  done
  for n in n-base:base n-chg:chg n-chg2:chg; do
    "$P/run-wkprobe.sh" newtos "$O/${n%%:*}" "$S/k/wk-${n#*:}.img" > "$O/${n%%:*}.stdout" 2>&1; echo "wk run ${n%%:*} rc $?" >> "$O/rc.txt"
  done
  cmp "$O/e1/WKPROBE.BIN" "$O/e2/WKPROBE.BIN" > "$O/cmp-bin-e1-e2.txt" 2>&1; echo "cmp BIN exit $?" >> "$O/cmp-bin-e1-e2.txt"
  diff "$O/e1/WKPROBE.LOG" "$O/e2/WKPROBE.LOG" > "$O/log-e1-e2.diff"; echo "diff LOG exit $?" >> "$O/log-e1-e2.diff"
  for p in "e1 n-base" "e1 n-chg"; do set -- $p
    python3 -I "$P/wkprobe-compare.py" "$O/$1" "$O/$2" --maps "$O/maps-$1-$2" > "$O/compare-$1-$2.txt" 2>&1; echo "compare exit $?" >> "$O/compare-$1-$2.txt"
  done
  for p in "n-base n-chg" "n-chg n-chg2"; do set -- $p
    cmp "$O/$1/WKPROBE.BIN" "$O/$2/WKPROBE.BIN" > "$O/cmp-bin-$1-$2.txt" 2>&1; echo "cmp BIN exit $?" >> "$O/cmp-bin-$1-$2.txt"
    diff "$O/$1/WKPROBE.LOG" "$O/$2/WKPROBE.LOG" > "$O/log-$1-$2.diff"; echo "diff LOG exit $?" >> "$O/log-$1-$2.diff"
    python3 -I "$P/wkprobe-compare.py" "$O/$1" "$O/$2" > "$O/compare-$1-$2.txt" 2>&1; echo "compare exit $?" >> "$O/compare-$1-$2.txt"
  done
  (cd "$S/k" && shasum -a 256 wk-*.img) > "$O/kernels.sha256"
  (cd "$O" && for d in e1 e2 n-base n-chg n-chg2; do shasum -a 256 $d/WKPROBE.BIN; done) > "$O/bin.sha256"
}

desk() {
  local OUT=$1 K=$2 X=$3 SC
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
  (cd "$SC" && python3 -I "$W/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$V51/desk-shots.txt" "$OUT/shots" "[inj] desktop armed at hz_200") > "$OUT/shots.log" 2>&1 || echo "qmpshots failed" >> "$OUT/run.txt"
  sleep 1; reap $pid "$(basename "$OUT")"
  grep -a -c -iE "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt"
  rm -rf "$SC"
}

bootcon() {
  local OUT=$1 K=$2 X=$3 SC
  mkdir -p "$OUT/shots"; SC=$(mktemp -d "$S/bc.XXXXXX")
  printf '#R 02\r\n' > "$SC/NEWDESK.INF"
  (cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF" AUTO/APITEST.NCP="$S/APITEST.NCP") > "$SC/mk.out" 2>&1 || { cat "$SC/mk.out"; return 1; }
  cp "$SC/card.img.manifest" "$OUT/"; cp -c "$SC/card.img" "$SC/boot.img"
  echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1) APITEST.NCP $(shasum -a 256 "$S/APITEST.NCP" | cut -d' ' -f1) extra [$X]" > "$OUT/run.txt"
  ( cd "$SC" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" $X \
      -append serial=on -kernel "$K" -drive file="$SC/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off ) &
  local pid=$!; echo "$(date +%H:%M:%S) $(basename "$OUT") pid $pid started" >> "$A/qemu-pids.txt"
  (cd "$SC" && python3 -I "$V51/timed-shots.py" qmp.sock "$OUT/shots" 6 10 18 24 32 45) > "$OUT/shots.log" 2>&1 || true
  reap $pid "$(basename "$OUT")"
  grep -a -c -iE "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt"
  rm -rf "$SC"
}

phase_gui() {
  G=$M/runs/gui; mkdir -p "$G" "$S/files"
  for i in $(seq -w 1 40); do printf 'V49 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
  cp "$MAIN/sdk/apps/apitest/APITEST.NCP" "$S/APITEST.NCP"
  echo "APITEST.NCP $(shasum -a 256 "$S/APITEST.NCP" | cut -d' ' -f1); files $(cat "$S"/files/* | shasum -a 256 | cut -c1-16); mouse $(shasum -a 256 "$V51/desk-mouse.txt" | cut -d' ' -f1)" > "$A/gui-inputs.txt"
  for T in base chg; do
    rm -rf "$S/t/arm-$T"; rsync -a --exclude /target "$S/t/$T/" "$S/t/arm-$T/"
    (cd "$W" && python3 -I tools/inject-arm.py "$S/t/arm-$T" "desktop=$V51/desk-mouse.txt") > "$G/arm-$T.txt" 2>&1
    (cd "$S/t/arm-$T" && cargo xtask build --board rpi4 --release) > "$G/arm-$T-build.log" 2>&1; echo "armed build $T rc $?" >> "$G/arm-$T.txt"
    cp "$S/t/arm-$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/k/arm-$T.img"
    rm -rf "$S/t/arm-$T"
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      desk "$G/desk$n-$T" "$S/k/arm-$T.img" "$x"
      bootcon "$G/bc$n-$T" "$S/k/$T-rpi4.img" "$x"
    done
  done
  for r in desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
  done > "$A/shots-compare.txt"
  (cd "$S/k" && shasum -a 256 arm-*.img) > "$G/armed-kernels.sha256"
}

for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
