#!/bin/sh
# V34: probe T18A (tosdump/probes/vdprobe.c) on both sides, the VDI tables base vs fix, the input
# trace. Run from the main tree; S = session scratchpad. Stages in the order they ran:
#   head  tree at 6595238b, product code unchanged: n-head and e1 ran before any edit, then the
#         host test file was added and run red
#   011   tos-api/src/xbios/system.rs scrdmp edited (dumpflg = -1), escape.rs not yet
#   fix   both edits (scrdmp + v_escape arm 17)
S=${S:?set S to the session scratchpad}
R=tosdump/reference
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
set -x
# --- head
cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-head.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-head $S/kernel-head.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e1
cargo test -p tos-api --test xbios_scrdmp_dumpflg > $S/test-red-head.txt 2>&1   # test file added, product unchanged
# --- 011 (after the scrdmp edit)
cargo test -p tos-api --test xbios_scrdmp_dumpflg > $S/test-after-011.txt 2>&1
cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-011.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-011 $S/kernel-011.img
# --- fix (after the v_escape edit)
cargo test -p tos-api --test xbios_scrdmp_dumpflg > $S/test-after-012.txt 2>&1
cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-fix.img
for r in n-fix1 n-fix2; do VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/$r $S/kernel-fix.img; done
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e2
for p in "e1 n-head" "e1 n-fix1" "n-head n-011" "n-head n-fix1"; do
  set -- $p
  python3 tosdump/probes/vdprobe-compare.py $S/$1 $S/$2 > $S/cmp/compare-$1-$2.txt
  diff $S/$1/VDPROBE.LOG $S/$2/VDPROBE.LOG > $S/cmp/log-$1-$2.diff
done
# input trace: the fix kernel with the tracer
cargo xtask build --board rpi4 --release --features bios-arg-trace; cp -c $K $S/kernel-fix-trace.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-fix-trace $S/kernel-fix-trace.img
# tables: base = git archive of HEAD (6595238b) into $S/base-tree, references/ linked; fix = this tree
git archive HEAD | tar -x -C $S/base-tree; ln -s "$PWD/references" $S/base-tree/references
for t in "vdi-v18-t2 vdi-v18t2 VDI-v18t2" "vdi-all vdi-all VDI-all"; do
  set -- $t
  cargo xtask tosdump vdi tosdump/cases/$1.tsv --tree $S/base-tree --out $S/runs/base-$2 --reference $R/EMUTOS-1a7484ab-$3-falcontc.BIN --scratch $S/xt
  cargo xtask tosdump vdi tosdump/cases/$1.tsv --out $S/runs/newtos-$2 --reference $R/EMUTOS-1a7484ab-$3-falcontc.BIN --scratch $S/xt
  tosdump/host/build/tosdiff tosdump/cases/$1.tsv $S/runs/base-$2/VDI.BIN $S/runs/newtos-$2/VDI.BIN > $S/runs/tosdiff-base-fix-$2.txt
done
cargo xtask deps scrdmp --up > $S/deps/scrdmp-up.txt; cargo xtask deps escfn17 --up > $S/deps/escfn17-up.txt
