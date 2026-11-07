#!/bin/bash
# V49 acceptance, base 3a9221eb (before V49) against chg 8403f75b (V49 kernel + raster-prof, feature off).
#   accept.sh <phase>...   phases: trees builds identity boots tables probe gui
# Run from anywhere; W = the V49 worktree, S = scratch (required, never /tmp itself).
# Every tree is a `git archive` of a commit into $S/t/<name> with references/ linked; nothing is built
# in the worktree itself. QEMU runs one at a time, sockets by relative path in their own scratch dirs.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-ac0a43815a49627e8
S=${S:?}
M=$W/measurements/measurement-2026-11-07-v49
A=$M/analysis
BASE=3a9221eb743b8c333a0687f8b08d0c808ba2be84
C1=87b2a9fd   # V49 kernel without raster-prof (identity baseline)
CHG=8403f75b  # V49 kernel + raster-prof
OC=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy
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

phase_builds() { # warnings + clippy both boards, rpi4 release images; chg also with raster-prof
  for T in base chg; do
    cd "$S/t/$T"
    for B in rpi4 qemu-virt; do
      rm -rf target/aarch64-unknown-none-softfloat
      cargo xtask build --board $B > "$A/build-$T-$B.log" 2>&1; echo "build $T $B rc $?" >> "$A/builds-rc.txt"
      cargo clippy -p kernel --features board-$B > "$A/clippy-$T-$B.txt" 2>&1; echo "clippy $T $B rc $?" >> "$A/builds-rc.txt"
    done
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board rpi4 --release > "$A/build-$T-rpi4-release.log" 2>&1; echo "build $T rpi4 release rc $?" >> "$A/builds-rc.txt"
    cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/$T-rpi4-release.img"
  done
  cd "$S/t/chg"
  for B in rpi4 qemu-virt; do
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board $B --features raster-prof > "$A/build-chg-prof-$B.log" 2>&1; echo "build chg+raster-prof $B rc $?" >> "$A/builds-rc.txt"
    cargo clippy -p kernel --features board-$B,raster-prof > "$A/clippy-chg-prof-$B.txt" 2>&1; echo "clippy chg+raster-prof $B rc $?" >> "$A/builds-rc.txt"
  done
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board rpi4 --release --features raster-prof > "$A/build-chg-prof-rpi4-release.log" 2>&1; echo "build chg+raster-prof rpi4 release rc $?" >> "$A/builds-rc.txt"
  cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/chg-prof-rpi4-release.img"
  (cd "$S/k" && shasum -a 256 *.img) | tee "$A/kernels.sha256"
  for f in "$A"/build-*.log; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | tr '\n' ' ')"; done > "$A/warnings.txt"
  for f in "$A"/clippy-*.txt; do echo "$(basename "$f"): $(grep -E 'generated [0-9]+ warning' "$f" | sed 's/ (run.*//' | tr '\n' ' ')"; done >> "$A/warnings.txt"
  echo BUILDSDONE >> "$A/builds-rc.txt"
}

buildall() { # buildall <tree> <out> : rpi4/qemu-virt x release/dev raw binaries, feature off
  mkdir -p "$2"; cd "$1"
  for b in rpi4 qemu-virt; do for p in release dev; do
    R=(); [ $p = release ] && R=(--release)
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board $b ${R[@]+"${R[@]}"} > "$2/build-$b-$p.log" 2>&1 || { echo "BUILD FAIL $b $p"; return 1; }
    d=$([ $p = release ] && echo release || echo debug)
    "$OC" -O binary target/aarch64-unknown-none-softfloat/$d/kernel "$2/$b-$p.bin"
  done; done
  (cd "$2" && shasum -a 256 *.bin)
}

phase_identity() { # CLAUDE.md "Testing": feature off = the build without the feature commit, but Location lines
  # both commits built in the same directory, so the tree path in panic Locations is equal
  tree id $C1; buildall "$S/t/id" "$S/id-c1" > "$A/identity-c1.sha256"
  tree id $CHG; buildall "$S/t/id" "$S/id-chg" > "$A/identity-chg-off.sha256"
  : > "$A/identity-locdiff.txt"
  for b in rpi4-release qemu-virt-release rpi4-dev qemu-virt-dev; do
    echo "== $b" >> "$A/identity-locdiff.txt"
    python3 -I "$M/method/locdiff.py" "$S/id-c1/$b.bin" "$S/id-chg/$b.bin" >> "$A/identity-locdiff.txt" 2>&1
    echo "exit $?" >> "$A/identity-locdiff.txt"
  done
  rm -rf "$S/t/id"
}

phase_boots() { # qemu-virt headless, each tree's own fresh release build, 100 s
  for T in base chg; do bash "$M/method/boot-virt.sh" "$S/t/$T" "$M/runs/boot-virt-$T"; done
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
    bash "$M/method/desk-inj.sh" kernel $C "$M/method/desk-mouse.txt" "$S/k/k-$T.img" | tail -1
  done
  mkdir -p "$M/runs/gui"
  for T in base chg; do
    "$P/run-shprobe.sh" newtos "$M/runs/gui/shprobe-$T" "$S/k/kernel-sh-$T.img" > "$M/runs/gui/shprobe-$T.stdout" 2>&1
    "$P/run-shprobe.sh" newtos-desk "$M/runs/gui/shprobe-desk-$T" "$S/k/$T-rpi4-release.img" > "$M/runs/gui/shprobe-desk-$T.stdout" 2>&1
  done
  for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
    n=${g%%:*}; x=${g#*:}
    for T in base chg; do
      QEMU_EXTRA="$x" bash "$M/method/desk-inj.sh" newtos "$M/runs/gui/desk$n-$T" "$S/k/k-$T.img" "$M/method/desk-NEWDESK.INF" "$M/method/desk-shots.txt" | tail -3
      bash "$M/method/boot-console.sh" "$M/runs/gui/bc$n-$T" "$S/k/$T-rpi4-release.img" "$x" | tail -2
    done
  done
  G=$M/runs/gui
  for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
    b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
    for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
    echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
  done > "$A/shots-compare.txt"
}

phase_pikernels() { # the two Pi test kernels (raster-prof): before = base + profiler, after = chg; raspi4b boot each
  [ -d "$S/t/bprof" ] || { tree bprof $BASE; bash "$M/method/make-bprof-tree.sh" "$S/t/bprof"; }
  cd "$S/t/bprof"
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board rpi4 --release --features raster-prof > "$A/build-bprof-rpi4-release.log" 2>&1; echo "build base+raster-prof rpi4 release rc $?" >> "$A/builds-rc.txt"
  cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/k/base-prof-rpi4-release.img"
  (cd "$S/k" && shasum -a 256 base-prof-rpi4-release.img chg-prof-rpi4-release.img) | tee "$A/pi-kernels.sha256"
  for K in base-prof chg-prof; do
    O=$M/runs/boot-rpi4-$K; mkdir -p "$O"; SC=$(mktemp -d "$S/pk.XXXXXX")
    printf '#R 02\r\n' > "$SC/NEWDESK.INF"
    (cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF") > "$SC/mk.out" 2>&1 || cat "$SC/mk.out"
    cp "$SC/card.img.manifest" "$O/"; cp -c "$SC/card.img" "$SC/boot.img"
    (cd "$SC" && qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$O/BOOT-SERIAL.txt" -append serial=on \
        -kernel "$S/k/$K-rpi4-release.img" -drive file="$SC/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off & echo $! > "$SC/pid")
    for _ in $(seq 1 60); do sleep 2; grep -q "desktop up" "$O/BOOT-SERIAL.txt" 2>/dev/null && break; done
    sleep 2
    (cd "$SC" && python3 -I "$M/method/timed-shots.py" qmp.sock "$O" 0 > "$O/shots.log" 2>&1)
    kill "$(cat "$SC/pid")" 2>/dev/null; sleep 1; kill -9 "$(cat "$SC/pid")" 2>/dev/null
    { echo "kernel $(shasum -a 256 "$S/k/$K-rpi4-release.img" | cut -d' ' -f1)"; grep -a -c -iE "exception|panic" "$O/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /'; grep -a "desktop up" "$O/BOOT-SERIAL.txt"; } | tee "$O/summary.txt"
    rm -rf "$SC"
  done
}

for ph in "$@"; do echo "=== phase $ph $(date +%H:%M:%S)"; phase_$ph; echo "=== done $ph $(date +%H:%M:%S)"; done
