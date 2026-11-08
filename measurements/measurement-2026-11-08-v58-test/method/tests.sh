#!/bin/bash
# V58 tester run (own script). tests.sh <phase>...   phases: boots tables probe ln gui
# W = V58 worktree, S = scratch (required). Trees $S/t/base ($S/t/chg) = git archive of 6821179c (1c1c71a3),
# references/ linked. QEMU: one at a time, only by the PID its starter recorded (helpers / xtask tosdump).
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
S=${S:?}; M=$W/measurements/measurement-2026-11-08-v58-test; A=$M/analysis
mkdir -p $A $M/runs $M/probe

phase_boots() {
  for T in base chg; do
    bash $M/method/boot-virt.sh $S/t/$T $M/runs/boot-virt-$T
    rm -rf $S/t/$T/target
  done
}

phase_tables() {
  cd $W || exit 1
  R=tosdump/reference
  run() { T=$1; D=$2; TAB=$3; O=$4; REF=${5:-}
    if [ -n "$REF" ]; then
      cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T > $M/runs/$O.stdout 2>&1
    else
      cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $M/runs/$O --scratch $S/xt-$T > $M/runs/$O.stdout 2>&1
    fi; echo "tosdump $O rc $?" >> $A/tables-rc.txt; }
  : > $A/tables-rc.txt
  for T in chg base; do
    run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
    run $T vdi vdi-v18.tsv tab-$T-v18 EMUTOS-1a7484ab-VDI-v18-falcontc
    run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
    run $T vdi vdi-v18-t3.tsv tab-$T-v18t3 EMUTOS-1a7484ab-VDI-v18t3-falcontc
    run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
    run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
    run $T aes aes-r4-subfunctions.tsv tab-$T-r4
    rm -rf $S/t/$T/target $S/xt-$T
  done
  for t in all v18 v18t2 v18t3 r2 r3 r4; do
    for f in $M/runs/tab-base-$t/*.BIN; do
      g=$M/runs/tab-chg-$t/$(basename $f)
      if cmp -s $f $g; then echo "$t $(basename $f) identical $(shasum -a 256 $f | cut -c1-16)"; else echo "$t $(basename $f) DIFFER"; fi
    done
  done > $A/tables-bin-compare.txt
}

phase_probe() { # vdprobe base, chg, chg again
  export VDPROBE_SCRATCH=$S/vp; mkdir -p $VDPROBE_SCRATCH
  P=$W/tosdump/probes
  for n in n-base:base n-chg:chg n-chg2:chg; do
    $P/run-vdprobe.sh newtos $M/probe/${n%%:*} $S/k/${n#*:}-rpi4-release.img > $M/probe/${n%%:*}.stdout 2>&1
  done
  for p in "n-base n-chg" "n-chg n-chg2"; do set -- $p
    python3 -I $P/vdprobe-compare.py $M/probe/$1 $M/probe/$2 > $M/probe/compare-$1-$2.txt 2>&1
    cmp $M/probe/$1/VDPROBE.BIN $M/probe/$2/VDPROBE.BIN > $M/probe/cmp-bin-$1-$2.txt 2>&1; echo "cmp BIN exit $?" >> $M/probe/cmp-bin-$1-$2.txt
    diff $M/probe/$1/VDPROBE.LOG $M/probe/$2/VDPROBE.LOG > $M/probe/log-$1-$2.diff; echo "diff LOG exit $?" >> $M/probe/log-$1-$2.diff
  done
}

phase_ln() { # random line probe
  for T in base chg base2; do
    K=$S/k/${T%2}-rpi4-release.img
    bash $M/method/lnprobe/run-lnprobe.sh $S/LNPROBE.NAP $K $M/probe/ln-$T > $M/probe/ln-$T.stdout 2>&1
  done
}

phase_gui() {
  P=$W/tosdump/probes
  export SHPROBE_SCRATCH=$S/sh DESK_SCRATCH=$S/dk DESK_FILES=$S/files APITEST=$S/APITEST.NCP
  mkdir -p $S/files
  for i in $(seq -w 1 40); do printf 'V49 desktop file %s\r\n' "$i" > $S/files/LONGNM$i.TXT; done
  cp /Volumes/Daten/Development/PiRusTOS/sdk/apps/apitest/APITEST.NCP $S/APITEST.NCP
  echo "APITEST.NCP $(shasum -a 256 $S/APITEST.NCP | cut -d' ' -f1)" > $A/gui-inputs.txt
  for p in base:6821179c chg:1c1c71a3; do T=${p%%:*}; C=${p#*:}
    $P/run-shprobe.sh kernel $C $S/k/kernel-sh-$T.img > $A/kernel-sh-$T.txt 2>&1
    bash $M/method/desk-inj.sh kernel $S/t/$T $M/method/desk-mouse.txt $S/k/k-$T.img > $A/kernel-inj-$T.txt 2>&1
  done
  mkdir -p $M/runs/gui
  for T in base chg; do
    $P/run-shprobe.sh newtos $M/runs/gui/shprobe-$T $S/k/kernel-sh-$T.img > $M/runs/gui/shprobe-$T.stdout 2>&1
    $P/run-shprobe.sh newtos-desk $M/runs/gui/shprobe-desk-$T $S/k/$T-rpi4-release.img > $M/runs/gui/shprobe-desk-$T.stdout 2>&1
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      QEMU_EXTRA="$x" bash $M/method/desk-inj.sh newtos $M/runs/gui/desk$n-$T $S/k/k-$T.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt > $M/runs/gui/desk$n-$T.stdout 2>&1
      bash $M/method/boot-console.sh $M/runs/gui/bc$n-$T $S/k/$T-rpi4-release.img "$x" > $M/runs/gui/bc$n-$T.stdout 2>&1
    done
  done
  G=$M/runs/gui
  for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in $b/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename $p); cmp -s $p $q || { d=$((d+1)); l="$l $(basename $p .png)"; }; done
    echo "$r: $n shots base, $(ls $f/shots/*.png 2>/dev/null | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' $b/BOOT-SERIAL.txt) chg $(grep -a -ciE 'exception|panic' $f/BOOT-SERIAL.txt)"
  done > $A/shots-compare.txt
}

for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
