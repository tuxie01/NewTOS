#!/bin/bash
# V51 acceptance, base abe8dae1 (the V50 head) against chg 1197fed7 (V51 kernel 3).
#   accept.sh <phase>...   phases: trees builds boots tables probe slprobe gui pikernels
# Adapted from measurements/measurement-2026-11-07-v50/method/accept.sh (V50 branch). W = the V51 worktree,
# S = scratch (required, never /tmp itself). Every tree is a `git archive` of a commit into $S/t/<name> with
# references/ linked; nothing is built in the worktree itself. QEMU runs one at a time, each killed by its PID
# by the helper that started it; after every phase `leak_check` lists any QEMU whose command line names $S
# (there must be none). The pikernels phase is new (not V49's): one pi-boot.sh per kernel, nothing else.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a76fdd16f4fbe1958
S=${S:?}
M=$W/measurements/measurement-2026-11-07-v51
A=$M/analysis
BASE=abe8dae1
CHG=1197fed7
mkdir -p "$S/t" "$S/k" "$A" "$M/runs" "$M/probe"
export S

leak_check() {
  local l; l=$(pgrep -f "qemu-system-aarch64.*$S" || true)
  echo "$(date +%H:%M:%S) after $1: qemu under scratch: ${l:-none}" >> "$A/qemu-leak-check.txt"
  [ -z "$l" ] || { echo "LEAK: $l" >&2; for q in $l; do kill "$q"; done; }
}

tree() { # tree <name> <commit>
  rm -rf "$S/t/$1"; mkdir -p "$S/t/$1"
  (cd "$W" && git archive "$2") | tar -x -C "$S/t/$1"
  ln -s "$W/references" "$S/t/$1/references"
  echo "tree $1 = $(cd "$W" && git rev-parse "$2")"
}

phase_trees() {
  tree base $BASE; tree chg $CHG
}

phase_builds() { # warnings + clippy both boards, rpi4 release images; both also with raster-prof
  : > "$A/builds-rc.txt"
  for T in base chg; do
    cd "$S/t/$T"
    for F in "" raster-prof; do
      FA=(); N=$T; [ -n "$F" ] && { FA=(--features "$F"); N=$T-prof; }
      for B in rpi4 qemu-virt; do
        rm -rf target/aarch64-unknown-none-softfloat
        cargo xtask build --board $B ${FA[@]+"${FA[@]}"} > "$A/build-$N-$B.log" 2>&1; echo "build $N $B rc $?" >> "$A/builds-rc.txt"
        CF=board-$B; [ -n "$F" ] && CF=$CF,$F
        cargo clippy -p kernel --features $CF > "$A/clippy-$N-$B.txt" 2>&1; echo "clippy $N $B rc $?" >> "$A/builds-rc.txt"
      done
      rm -rf target/aarch64-unknown-none-softfloat
      cargo xtask build --board rpi4 --release ${FA[@]+"${FA[@]}"} > "$A/build-$N-rpi4-release.log" 2>&1; echo "build $N rpi4 release rc $?" >> "$A/builds-rc.txt"
      cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/$N-rpi4-release.img"
    done
  done
  (cd "$S/k" && shasum -a 256 *-rpi4-release.img) | tee "$A/kernels.sha256"
  for f in "$A"/build-*.log; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | tr '\n' ' ')"; done > "$A/warnings.txt"
  for f in "$A"/clippy-*.txt; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | sed 's/ (run.*//' | tr '\n' ' ')"; done >> "$A/warnings.txt"
  for f in "$A"/clippy-chg*.txt; do echo "$(basename "$f") hits in touched files: $(grep -E -- '--> kernel/src/(raster_copy|raster_kernel|raster_expand|raster_prof|vdi_backend|lib)\.rs' "$f" | tr '\n' ' ')"; done >> "$A/warnings.txt"
  echo BUILDSDONE >> "$A/builds-rc.txt"
}

phase_boots() { # qemu-virt headless, each tree's own fresh release build, 100 s
  for T in base chg; do bash "$M/method/boot-virt.sh" "$S/t/$T" "$M/runs/boot-virt-$T"; leak_check boot-virt-$T; done
}

phase_tables() {
  cd "$W"
  R=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --reference $R/$REF.BIN --scratch "$S/xt-$T"
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --scratch "$S/xt-$T"
    fi; leak_check "$O"; }
  for T in chg base; do
    run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
    run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
    run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
    run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
    run $T aes aes-r4-subfunctions.tsv tab-$T-r4
  done
  for t in v18t2 all r2 r3 r4; do
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
    leak_check "vdprobe-${n%%:*}"
  done
  for p in "n-base n-chg" "n-chg n-chg2"; do set -- $p
    python3 -I "$P/vdprobe-compare.py" "$M/probe/$1" "$M/probe/$2" > "$M/probe/compare-$1-$2.txt" 2>&1
    cmp "$M/probe/$1/VDPROBE.BIN" "$M/probe/$2/VDPROBE.BIN" > "$M/probe/cmp-bin-$1-$2.txt" 2>&1; echo "cmp BIN exit $?" >> "$M/probe/cmp-bin-$1-$2.txt"
    diff "$M/probe/$1/VDPROBE.LOG" "$M/probe/$2/VDPROBE.LOG" > "$M/probe/log-$1-$2.diff"; echo "diff LOG exit $?" >> "$M/probe/log-$1-$2.diff"
  done
}

phase_slprobe() { # window scroll-bar probe: base twice (its own noise set), change twice
  export SLPROBE_SCRATCH=$S/sl; mkdir -p "$SLPROBE_SCRATCH"
  P=$W/tosdump/probes
  for p in base:$BASE chg:$CHG; do
    "$P/run-slprobe.sh" kernel ${p#*:} "$S/k/sl-${p%%:*}.img" > "$M/runs/slprobe-kernel-${p%%:*}.stdout" 2>&1
  done
  for r in base base2 chg chg2; do
    k=${r%2}
    "$P/run-slprobe.sh" newtos "$M/runs/sl-$r" "$S/k/sl-$k.img" > "$M/runs/sl-$r.stdout" 2>&1
    leak_check "slprobe-$r"
  done
  G=$M/runs
  for pr in "base base2" "chg chg2" "base chg" "base2 chg2"; do set -- $pr
    n=0; d=0; l=""
    for p in "$G/sl-$1"/shots/*.png; do n=$((n+1)); q=$G/sl-$2/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    lg=$(cmp -s "$G/sl-$1/SLPROBE.LOG" "$G/sl-$2/SLPROBE.LOG" && echo same || echo DIFFER)
    echo "sl-$1 vs sl-$2: $n shots, $d differ, SLPROBE.LOG $lg:$l"
  done > "$A/slprobe-compare.txt"
}

phase_gui() {
  P=$W/tosdump/probes
  export SHPROBE_SCRATCH=$S/sh DESK_SCRATCH=$S/dk DESK_FILES=$S/files APITEST=$S/APITEST.NCP
  mkdir -p "$S/files"
  for i in $(seq -w 1 40); do printf 'V49 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
  cp /Volumes/Daten/Development/PiRusTOS/sdk/apps/apitest/APITEST.NCP "$S/APITEST.NCP"
  echo "APITEST.NCP $(shasum -a 256 "$S/APITEST.NCP" | cut -d' ' -f1); files $(cat "$S"/files/* | shasum -a 256 | cut -c1-16)" > "$A/gui-inputs.txt"
  for p in base:$BASE chg:$CHG; do T=${p%%:*}; C=${p#*:}
    "$P/run-shprobe.sh" kernel $C "$S/k/kernel-sh-$T.img"
    bash "$M/method/desk-inj.sh" kernel $C "$M/method/desk-mouse.txt" "$S/k/k-$T.img" | tail -1
  done
  mkdir -p "$M/runs/gui"
  for T in base chg; do
    "$P/run-shprobe.sh" newtos "$M/runs/gui/shprobe-$T" "$S/k/kernel-sh-$T.img" > "$M/runs/gui/shprobe-$T.stdout" 2>&1
    leak_check shprobe-$T
    "$P/run-shprobe.sh" newtos-desk "$M/runs/gui/shprobe-desk-$T" "$S/k/$T-rpi4-release.img" > "$M/runs/gui/shprobe-desk-$T.stdout" 2>&1
    leak_check shprobe-desk-$T
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      QEMU_EXTRA="$x" bash "$M/method/desk-inj.sh" newtos "$M/runs/gui/desk$n-$T" "$S/k/k-$T.img" "$M/method/desk-NEWDESK.INF" "$M/method/desk-shots.txt" | tail -3
      leak_check desk$n-$T
      bash "$M/method/boot-console.sh" "$M/runs/gui/bc$n-$T" "$S/k/$T-rpi4-release.img" "$x" | tail -2
      leak_check bc$n-$T
    done
  done
  G=$M/runs/gui
  for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
  done > "$A/shots-compare.txt"
}

phase_pikernels() { # the Pi test kernels boot in QEMU raspi4b: change, and change + raster-prof
  for K in chg chg-prof; do
    bash "$M/method/pi-boot.sh" "$S/k/$K-rpi4-release.img" "$M/runs/boot-rpi4-$K"
    leak_check boot-rpi4-$K
  done
  mkdir -p "$M/pi-kernels"
  for K in base base-prof chg chg-prof; do cp "$S/k/$K-rpi4-release.img" "$M/pi-kernels/$K-kernel8.img"; done
  (cd "$M/pi-kernels" && shasum -a 256 *.img) | tee "$A/pi-kernels.sha256"
}

for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; leak_check "phase $ph"; echo "=== done $ph $(date +%H:%M:%S)"; done
