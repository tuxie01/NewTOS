#!/bin/bash
# V50 TESTER run: the developer accept.sh, outputs under this run (M), helpers reused from the developer run (H); pikernels phase removed.
#   accept.sh <phase>...   phases: trees builds boots tables probe gui pikernels
# Adapted from measurements/measurement-2026-11-07-v49/method/accept.sh. W = the V50 worktree, S = scratch
# (required, never /tmp itself). Every tree is a `git archive` of a commit into $S/t/<name> with references/
# linked; nothing is built in the worktree itself. QEMU runs one at a time, killed by PID, sockets by relative
# path in their own scratch dirs.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a86a976363716f766
S=${S:?}
M=$W/measurements/measurement-2026-11-07-v50-test
H=$W/measurements/measurement-2026-11-07-v50/method
A=$M/analysis
BASE=ce5e79c2
CHG=3255df27
mkdir -p "$S/t" "$S/k" "$A" "$M/runs" "$M/probe"
export S

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
  for f in "$A"/clippy-chg*.txt; do echo "$(basename "$f") hits in touched files: $(grep -E -- '--> kernel/src/(raster_kernel|raster_expand|raster_prof|vdi_backend|lib)\.rs' "$f" | tr '\n' ' ')"; done >> "$A/warnings.txt"
  echo BUILDSDONE >> "$A/builds-rc.txt"
}

phase_boots() { # qemu-virt headless, each tree's own fresh release build, 100 s
  for T in base chg; do bash "$H/boot-virt.sh" "$S/t/$T" "$M/runs/boot-virt-$T"; done
}

phase_tables() {
  cd "$W"
  R=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --reference $R/$REF.BIN --scratch "$S/xt-$T"
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree "$S/t/$T" --out "$M/runs/$O" --scratch "$S/xt-$T"
    fi; }
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
  for p in base:$BASE chg:$CHG; do T=${p%%:*}; C=${p#*:}
    "$P/run-shprobe.sh" kernel $C "$S/k/kernel-sh-$T.img"
    bash "$H/desk-inj.sh" kernel $C "$H/desk-mouse.txt" "$S/k/k-$T.img" | tail -1
  done
  mkdir -p "$M/runs/gui"
  for T in base chg; do
    "$P/run-shprobe.sh" newtos "$M/runs/gui/shprobe-$T" "$S/k/kernel-sh-$T.img" > "$M/runs/gui/shprobe-$T.stdout" 2>&1
    "$P/run-shprobe.sh" newtos-desk "$M/runs/gui/shprobe-desk-$T" "$S/k/$T-rpi4-release.img" > "$M/runs/gui/shprobe-desk-$T.stdout" 2>&1
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      QEMU_EXTRA="$x" bash "$H/desk-inj.sh" newtos "$M/runs/gui/desk$n-$T" "$S/k/k-$T.img" "$H/desk-NEWDESK.INF" "$H/desk-shots.txt" | tail -3
      bash "$H/boot-console.sh" "$M/runs/gui/bc$n-$T" "$S/k/$T-rpi4-release.img" "$x" | tail -2
    done
  done
  G=$M/runs/gui
  for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
  done > "$A/shots-compare.txt"
}


for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
