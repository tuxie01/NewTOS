#!/bin/sh
# V35: what ran, in order, from the main tree; S = session scratchpad (…/scratchpad/v35). Stages:
#   head  tree at 35eeb6b5, product code unchanged
#   fix   the V35 change in the working tree (uncommitted then)
S=${S:?set S to the session scratchpad}
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
set -x
# --- head: warnings, release kernel, probe (unchanged probe first: e0/n0, then with t35: e1/n-head)
for B in rpi4 qemu-virt; do rm -rf target/aarch64-unknown-none-softfloat; cargo xtask build --board $B > $S/builds/base/build-$B.log 2>&1; cargo clippy -p kernel --features board-$B > $S/builds/base/clippy-$B.log 2>&1; done
rm -rf target/aarch64-unknown-none-softfloat; cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-head.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e0-origprobe     # probe before t35
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n0-origprobe $S/kernel-head.img
# (tosdump/probes/vdprobe.c: t35 appended)
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e1
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-head $S/kernel-head.img
# red: git archive of HEAD + the new test files only
cd $S/base-tree && cargo test -p tos-api --test vdi_reference v35; cd -
# --- fix
cargo test -p tos-api --test vdi_reference v35; cargo xtask test
for B in rpi4 qemu-virt; do rm -rf target/aarch64-unknown-none-softfloat; cargo xtask build --board $B > $S/builds/fix/build-$B.log 2>&1; cargo clippy -p kernel --features board-$B > $S/builds/fix/clippy-$B.log 2>&1; done
rm -rf target/aarch64-unknown-none-softfloat; cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-fix.img
for r in n-fix1 n-fix2; do VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/$r $S/kernel-fix.img; done
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e2
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n-fix1          # and e1/n-head, n-head/n-fix1
sh measurements/measurement-2026-11-07-v35/method/tables.sh          # base tree = git archive of 35eeb6b5
cargo xtask deps GSX_ENTRY --up
measurements/measurement-2026-11-07-v35/method/trees.sh; measurements/measurement-2026-11-07-v35/method/gui.sh
# boot console 1080 repeated twice more (bc1080b-*, bc1080c-* fix first) with the V34 test run's boot-console.sh
# benchmark: VBENCH.NAP built from method/vbench in the scratchpad; VB_NAP, VB_SCR set
for r in 1 2; do for T in base fix; do measurements/measurement-2026-11-07-v35/method/vbench-run.sh $S/vb/ic$r-$T <kernel> -icount shift=0,sleep=off; done; done
for r in 1 2 3; do for T in base fix; do measurements/measurement-2026-11-07-v35/method/vbench-run.sh $S/vb/wc$r-$T <kernel>; done; done
