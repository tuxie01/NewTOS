#!/bin/bash
# V58 acceptance, base 6821179c (main with V49-V51) against chg 1c1c71a3 (V58 lines).
#   accept.sh <phase>...   phases: builds boots tables probe gui pikernels
# Written for this run (pattern of the V49-V51 runs' phases, not their scripts). W = the V58 worktree,
# S = scratch (required, never /tmp itself). The trees $S/t/base and $S/t/chg are `git archive`s of the
# two commits with references/ linked (made before this script ran, RUNINFO). Nothing is built in the
# worktree. QEMU runs one at a time and every QEMU is stopped by the PID its starter recorded (the helpers
# here, tosdump/probes/run-*.sh, cargo xtask tosdump); after every step `qemu_list` only LISTS the QEMUs
# whose command line names $S into analysis/qemu-list.txt -- it kills nothing.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
S=${S:?}
M=$W/measurements/measurement-2026-11-08-v58
A=$M/analysis
mkdir -p "$S/k" "$A" "$M/runs" "$M/probe"
export S

qemu_list() {
  local l; l=$(pgrep -fl "qemu-system-aarch64" | grep -F "$S" || true)
  echo "$(date +%H:%M:%S) after $1: qemu naming the scratch: ${l:-none}" >> "$A/qemu-list.txt"
}

phase_builds() { # warnings + clippy both boards (dev), host-test clippy, rpi4 release kernels +/- raster-prof
  : > "$A/builds-rc.txt"
  for T in base chg; do
    cd "$S/t/$T" || exit 1
    for B in rpi4 qemu-virt; do
      rm -rf target/aarch64-unknown-none-softfloat
      cargo xtask build --board $B > "$A/build-$T-$B.log" 2>&1; echo "build $T $B rc $?" >> "$A/builds-rc.txt"
      cargo clippy -p kernel --features board-$B > "$A/clippy-$T-$B.txt" 2>&1; echo "clippy $T $B rc $?" >> "$A/builds-rc.txt"
    done
    cargo clippy -p tos-api --tests > "$A/clippy-$T-tos-api-tests.txt" 2>&1; echo "clippy $T tos-api --tests rc $?" >> "$A/builds-rc.txt"
    for F in "" raster-prof; do
      FA=(); N=$T; [ -n "$F" ] && { FA=(--features "$F"); N=$T-prof; }
      rm -rf target/aarch64-unknown-none-softfloat
      cargo xtask build --board rpi4 --release ${FA[@]+"${FA[@]}"} > "$A/build-$N-rpi4-release.log" 2>&1; echo "build $N rpi4 release rc $?" >> "$A/builds-rc.txt"
      cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/$N-rpi4-release.img"
    done
    rm -rf target
  done
  (cd "$S/k" && shasum -a 256 *-rpi4-release.img && ls -l *-rpi4-release.img) | tee "$A/kernels.sha256"
  for f in "$A"/build-*.log; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | tr '\n' ' ')"; done > "$A/warnings.txt"
  for f in "$A"/clippy-*.txt; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | sed 's/ (run.*//' | tr '\n' ' ')"; done >> "$A/warnings.txt"
  for f in "$A"/clippy-chg*.txt; do echo "$(basename "$f") hits in touched files: $(grep -E -- '--> (kernel/src/(raster_line|raster_kernel|vdi_backend|lib)\.rs|tos-api/src/vdi/line\.rs|tos-api/tests/raster_kernel_(line|fill)\.rs)' "$f" | tr '\n' ' ')"; done >> "$A/warnings.txt"
  echo BUILDSDONE >> "$A/builds-rc.txt"
}

phase_boots() { # qemu-virt headless, each tree's own fresh release build
  for T in base chg; do
    bash "$M/method/boot-virt.sh" "$S/t/$T" "$M/runs/boot-virt-$T"; qemu_list boot-virt-$T
    rm -rf "$S/t/$T/target"
  done
}

phase_tables() {
  cd "$W" || exit 1
  R=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --reference $R/$REF.BIN --scratch "$S/xt-$T" > "$M/runs/$O.stdout" 2>&1
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --scratch "$S/xt-$T" > "$M/runs/$O.stdout" 2>&1
    fi; echo "tosdump $O rc $?" >> "$A/tables-rc.txt"; qemu_list "$O"; }
  : > "$A/tables-rc.txt"
  for T in chg base; do
    run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
    run $T vdi vdi-v18.tsv tab-$T-v18 EMUTOS-1a7484ab-VDI-v18-falcontc
    run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
    run $T vdi vdi-v18-t3.tsv tab-$T-v18t3 EMUTOS-1a7484ab-VDI-v18t3-falcontc
    run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
    run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
    run $T aes aes-r4-subfunctions.tsv tab-$T-r4
    rm -rf "$S/t/$T/target" "$S/xt-$T"
  done
  for t in all v18 v18t2 v18t3 r2 r3 r4; do
    for f in "$M/runs/tab-base-$t"/*.BIN; do
      g="$M/runs/tab-chg-$t/$(basename "$f")"
      if cmp -s "$f" "$g"; then echo "$t $(basename "$f") identical $(shasum -a 256 "$f" | cut -c1-16)"; else echo "$t $(basename "$f") DIFFER"; fi
    done
  done > "$A/tables-bin-compare.txt"
}

phase_probe() {
  export VDPROBE_SCRATCH=$S/vp; mkdir -p "$VDPROBE_SCRATCH"
  P=$W/tosdump/probes
  for n in n-base:base n-chg:chg n-chg2:chg; do
    "$P/run-vdprobe.sh" newtos "$M/probe/${n%%:*}" "$S/k/${n#*:}-rpi4-release.img" > "$M/probe/${n%%:*}.stdout" 2>&1
    qemu_list "vdprobe-${n%%:*}"
  done
  for p in "n-base n-chg" "n-chg n-chg2"; do set -- $p
    python3 -I "$P/vdprobe-compare.py" "$M/probe/$1" "$M/probe/$2" > "$M/probe/compare-$1-$2.txt" 2>&1
    cmp "$M/probe/$1/VDPROBE.BIN" "$M/probe/$2/VDPROBE.BIN" > "$M/probe/cmp-bin-$1-$2.txt" 2>&1; echo "cmp BIN exit $?" >> "$M/probe/cmp-bin-$1-$2.txt"
    diff "$M/probe/$1/VDPROBE.LOG" "$M/probe/$2/VDPROBE.LOG" > "$M/probe/log-$1-$2.diff"; echo "diff LOG exit $?" >> "$M/probe/log-$1-$2.diff"
  done
}

phase_gui() {
  P=$W/tosdump/probes
  export SHPROBE_SCRATCH=$S/sh DESK_SCRATCH=$S/dk DESK_FILES=$S/files APITEST=$S/APITEST.NCP
  mkdir -p "$S/files"
  for i in $(seq -w 1 40); do printf 'V49 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
  cp /Volumes/Daten/Development/PiRusTOS/sdk/apps/apitest/APITEST.NCP "$S/APITEST.NCP"
  echo "APITEST.NCP $(shasum -a 256 "$S/APITEST.NCP" | cut -d' ' -f1); files $(cat "$S"/files/* | shasum -a 256 | cut -c1-16)" > "$A/gui-inputs.txt"
  for p in base:6821179c chg:1c1c71a3; do T=${p%%:*}; C=${p#*:}
    "$P/run-shprobe.sh" kernel $C "$S/k/kernel-sh-$T.img" > "$A/kernel-sh-$T.txt" 2>&1
    bash "$M/method/desk-inj.sh" kernel "$S/t/$T" "$M/method/desk-mouse.txt" "$S/k/k-$T.img" > "$A/kernel-inj-$T.txt" 2>&1
  done
  mkdir -p "$M/runs/gui"
  for T in base chg; do
    "$P/run-shprobe.sh" newtos "$M/runs/gui/shprobe-$T" "$S/k/kernel-sh-$T.img" > "$M/runs/gui/shprobe-$T.stdout" 2>&1
    qemu_list shprobe-$T
    "$P/run-shprobe.sh" newtos-desk "$M/runs/gui/shprobe-desk-$T" "$S/k/$T-rpi4-release.img" > "$M/runs/gui/shprobe-desk-$T.stdout" 2>&1
    qemu_list shprobe-desk-$T
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      QEMU_EXTRA="$x" bash "$M/method/desk-inj.sh" newtos "$M/runs/gui/desk$n-$T" "$S/k/k-$T.img" "$M/method/desk-NEWDESK.INF" "$M/method/desk-shots.txt" > "$M/runs/gui/desk$n-$T.stdout" 2>&1
      qemu_list desk$n-$T
      bash "$M/method/boot-console.sh" "$M/runs/gui/bc$n-$T" "$S/k/$T-rpi4-release.img" "$x" > "$M/runs/gui/bc$n-$T.stdout" 2>&1
      qemu_list bc$n-$T
    done
  done
  compare_gui
}

compare_gui() {
  G=$M/runs/gui
  for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png 2>/dev/null | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
  done > "$A/shots-compare.txt"
}

phase_pikernels() { # the Pi test kernels boot in QEMU raspi4b: change, and change + raster-prof
  for K in chg chg-prof; do
    bash "$M/method/pi-boot.sh" "$S/k/$K-rpi4-release.img" "$M/runs/boot-rpi4-$K"
    qemu_list boot-rpi4-$K
  done
  mkdir -p "$M/pi-kernels"
  for K in base base-prof chg chg-prof; do cp "$S/k/$K-rpi4-release.img" "$M/pi-kernels/$K-kernel8.img"; done
  (cd "$M/pi-kernels" && shasum -a 256 *.img) | tee "$A/pi-kernels.sha256"
}

for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; qemu_list "phase $ph"; echo "=== done $ph $(date +%H:%M:%S)"; done
