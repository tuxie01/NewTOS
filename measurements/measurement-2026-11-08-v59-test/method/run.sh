#!/bin/bash
# V59 tester run (own script, not the developer's accept.sh): base 6821179c vs change 5a3052ec.
#   S=<scratch> run.sh <phase>...   phases: boots tables vdprobe wkprobe shprobe edge gui
# Trees $S/t/{base,chg} = git archive of the commits; kernels $S/k/{base,chg}-{rpi4,virt}.img (release).
# Every QEMU started here is killed by the PID recorded at its launch (never by name/pattern); PIDs in $A/qemu-pids.txt.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a194b0b2cc3f6d929
S=${S:?}
M=$W/measurements/measurement-2026-11-08-v59-test
A=$M/analysis
R=$S/r                       # big intermediates (deleted at the end)
V51=$W/measurements/measurement-2026-11-07-v51/method
mkdir -p "$R" "$A"
export S

reap() { kill "$1" 2>/dev/null; sleep 1; kill -9 "$1" 2>/dev/null; wait "$1" 2>/dev/null
  if kill -0 "$1" 2>/dev/null; then echo "$(date +%H:%M:%S) $2 pid $1 STILL ALIVE" >> "$A/qemu-pids.txt"
  else echo "$(date +%H:%M:%S) $2 pid $1 gone" >> "$A/qemu-pids.txt"; fi; }

phase_boots() {
  for T in base chg; do
    O=$M/runs/boot-virt-$T; mkdir -p "$O"
    qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
      -device qemu-xhci -device usb-kbd -device usb-mouse -display none -serial file:"$O/serial.txt" -no-reboot \
      -kernel "$S/k/$T-virt.img" &
    pid=$!; echo "$(date +%H:%M:%S) boot-virt-$T pid $pid started" >> "$A/qemu-pids.txt"
    for _ in $(seq 1 60); do sleep 2; grep -aq "desktop up" "$O/serial.txt" && break; done; sleep 10
    reap $pid boot-virt-$T
    { echo "kernel $(shasum -a 256 "$S/k/$T-virt.img" | cut -d' ' -f1)"; echo "exception-panic-lines $(grep -a -c -iE 'exception|panic' "$O/serial.txt")"; grep -a "desktop up" "$O/serial.txt"; } > "$O/summary.txt"; cat "$O/summary.txt"
  done
  for T in base chg; do
    O=$M/runs/boot-rpi4-$T; mkdir -p "$O"; SC=$(mktemp -d "$S/pb.XXXXXX")
    printf '#R 02\r\n' > "$SC/NEWDESK.INF"
    (cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF") > "$SC/mk.out" 2>&1 || { cat "$SC/mk.out"; return 1; }
    cp "$SC/card.img.manifest" "$O/"; cp -c "$SC/card.img" "$SC/boot.img"; : > "$O/BOOT-SERIAL.txt"
    ( cd "$SC" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$O/BOOT-SERIAL.txt" -append serial=on \
        -kernel "$S/k/$T-rpi4.img" -drive file="$SC/boot.img",if=sd,format=raw ) &
    pid=$!; echo "$(date +%H:%M:%S) boot-rpi4-$T pid $pid started" >> "$A/qemu-pids.txt"
    for _ in $(seq 1 60); do sleep 2; grep -aq "desktop up" "$O/BOOT-SERIAL.txt" && break; done; sleep 5
    reap $pid boot-rpi4-$T
    { echo "kernel $(shasum -a 256 "$S/k/$T-rpi4.img" | cut -d' ' -f1)"; echo "exception-panic-lines $(grep -a -c -iE 'exception|panic' "$O/BOOT-SERIAL.txt")"; grep -a "desktop up" "$O/BOOT-SERIAL.txt"; } > "$O/summary.txt"; cat "$O/summary.txt"
    rm -rf "$SC"
  done
}

phase_tables() {
  cd "$W"; Rf=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$R/$O" --reference $Rf/$REF.BIN --scratch "$S/xt-$T" > "$R/$O.stdout" 2>&1
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$R/$O" --scratch "$S/xt-$T" > "$R/$O.stdout" 2>&1
    fi; echo "$O rc $?" >> "$A/tables-rc.txt"; }
  : > "$A/tables-rc.txt"
  for T in base chg; do
    run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
    run $T vdi vdi-v18-t2.tsv tab-$T-t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
    run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
    run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
    run $T aes aes-r4-subfunctions.tsv tab-$T-r4
  done
  : > "$A/tables-compare.txt"
  for t in all t2 r2 r3 r4; do
    for f in "$R/tab-base-$t"/*.BIN; do
      g="$R/tab-chg-$t/$(basename "$f")"
      if cmp -s "$f" "$g"; then echo "$t $(basename "$f") IDENTICAL $(shasum -a 256 "$f" | cut -c1-16)"
      else echo "$t $(basename "$f") DIFFER: $(cmp -l "$f" "$g" | wc -l | tr -d ' ') bytes"; fi
    done
  done >> "$A/tables-compare.txt"
  for o in tab-base-all tab-chg-all tab-base-t2 tab-chg-t2 tab-base-r2 tab-chg-r2 tab-base-r3 tab-chg-r3 tab-base-r4 tab-chg-r4; do
    echo "== $o"; tail -6 "$R/$o.stdout"; done > "$A/tables-stdout-tail.txt"
}

phase_vdprobe() {
  export VDPROBE_SCRATCH=$S/vp; mkdir -p "$VDPROBE_SCRATCH"
  P=$W/tosdump/probes; O=$R/vdprobe; mkdir -p "$O"
  for n in n-base:base n-chg:chg; do
    "$P/run-vdprobe.sh" newtos "$O/${n%%:*}" "$S/k/${n#*:}-rpi4.img" > "$O/${n%%:*}.stdout" 2>&1
  done
  python3 -I "$P/vdprobe-compare.py" "$O/n-base" "$O/n-chg" > "$A/vdprobe-compare.txt" 2>&1
  { cmp "$O/n-base/VDPROBE.BIN" "$O/n-chg/VDPROBE.BIN" && echo "VDPROBE.BIN identical"; shasum -a 256 "$O"/n-*/VDPROBE.BIN; diff "$O/n-base/VDPROBE.LOG" "$O/n-chg/VDPROBE.LOG" && echo "VDPROBE.LOG identical"; } > "$A/vdprobe-cmp.txt" 2>&1
  cp "$O/n-chg/card.img.manifest" "$M/runs/vdprobe-card.img.manifest"
}

phase_wkprobe() {
  export WKPROBE_SCRATCH=$S/wk; mkdir -p "$WKPROBE_SCRATCH"
  P=$W/tosdump/probes; O=$R/wkprobe; mkdir -p "$O"
  for T in base chg; do "$P/run-wkprobe.sh" kernel "$S/t/$T" "$S/k/wk-$T.img" > "$O/kernel-$T.stdout" 2>&1; echo "wk kernel $T rc $?" >> "$O/rc.txt"; done
  for n in n-base:base n-chg:chg n-chg2:chg; do
    "$P/run-wkprobe.sh" newtos "$O/${n%%:*}" "$S/k/wk-${n#*:}.img" > "$O/${n%%:*}.stdout" 2>&1; echo "wk run ${n%%:*} rc $?" >> "$O/rc.txt"
  done
  { for p in "n-base n-chg" "n-chg n-chg2"; do set -- $p
      echo "== $1 vs $2"; cmp "$O/$1/WKPROBE.BIN" "$O/$2/WKPROBE.BIN" && echo "WKPROBE.BIN identical"
      python3 -I "$P/wkprobe-compare.py" "$O/$1" "$O/$2" | tail -5
      diff "$O/$1/WKPROBE.LOG" "$O/$2/WKPROBE.LOG" | grep -c '^[<>]' | sed 's/^/LOG differing lines: /'
      diff "$O/$1/WKPROBE.LOG" "$O/$2/WKPROBE.LOG" | grep '^[<>]' | grep -vE ' T0[0-9a-f]|waited' | head; done
    shasum -a 256 "$O"/n-*/WKPROBE.BIN; cat "$O/rc.txt"; } > "$A/wkprobe-cmp.txt" 2>&1
  cp "$O/n-chg/card.img.manifest" "$M/runs/wkprobe-card.img.manifest"
  mkdir -p "$M/runs/wkprobe-chg"; cp "$O/n-chg/WKPROBE.LOG" "$O/n-chg/BOOT-SERIAL.txt" "$M/runs/wkprobe-chg/"
}

phase_shprobe() {
  export SHPROBE_SCRATCH=$S/sh; mkdir -p "$SHPROBE_SCRATCH"; export VDPROBE_SCRATCH=$S/sh
  P=$W/tosdump/probes; O=$R/shprobe; mkdir -p "$O"
  for T in base chg; do "$P/run-shprobe.sh" kernel "$(cd $W && git rev-parse $( [ $T = base ] && echo 6821179c || echo 5a3052ec))" "$S/k/sh-$T.img" > "$O/kernel-$T.stdout" 2>&1; echo "sh kernel $T rc $?" >> "$O/rc.txt"; done
  for T in base chg; do
    "$P/run-shprobe.sh" newtos "$O/n-$T" "$S/k/sh-$T.img" > "$O/n-$T.stdout" 2>&1; echo "sh run $T rc $?" >> "$O/rc.txt"
    "$P/run-shprobe.sh" newtos-desk "$O/d-$T" "$S/k/$T-rpi4.img" > "$O/d-$T.stdout" 2>&1; echo "sh desk $T rc $?" >> "$O/rc.txt"
  done
  { for k in n d; do n=0; d=0; for p in "$O/$k-base"/*.png "$O/$k-base"/shots/*.png; do [ -f "$p" ] || continue; n=$((n+1)); q=${p/$k-base/$k-chg}; cmp -s "$p" "$q" || { d=$((d+1)); echo "DIFF $p"; }; done; echo "$k: $n png base, $d differ"; done; cat "$O/rc.txt"; } > "$A/shprobe-cmp.txt" 2>&1
}
for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
