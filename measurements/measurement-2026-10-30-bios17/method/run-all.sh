#!/bin/bash
# run-all.sh -- the scratch driver of every BIOS17 run, as run (Developer, 2026-10-30).
#   1. before: `git archive HEAD` (f2652440) unpacked in the scratch, rpi4 release kernel built
#      there; then the same before-tree with ONLY the rename applied (the BIOS17 versions of the
#      files whose diff is rename/comment only; loader.rs by the same sed; hooks.rs untouched), rebuilt:
#      kernel8.img must be byte-identical (the rename changes no behaviour).
#   2. after: this tree, rpi4 release kernel; vwprobe, sxprobe, hkprobe on NewTOS.
#   3. MEM12 P0PAR with an injector-armed scratch copy of this tree (as in the BIOS16 run).
#   4. qemu-virt smoke boot to "desktop up".
set -u
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/45e8e806-7ba7-4edc-ace3-d4dd625a7f8d/scratchpad/bios17
REPO=/Volumes/Daten/Development/PiRusTOS
M=$REPO/measurements/measurement-2026-10-30-bios17
R=$M/runs
mkdir -p $S $R
cd $REPO

# 1. before + rename-only
rm -rf $S/before-tree; mkdir -p $S/before-tree
git archive HEAD | tar -x -C $S/before-tree
(cd $S/before-tree && cargo xtask build --board rpi4 --release > $S/build-before.log 2>&1; echo "build before $?")
cp $S/before-tree/target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-before.img
for f in tos-api/src/bios/reset.rs tos-api/src/bios/vectors.rs tos-api/src/bdos/process/mmu.rs \
         tos-api/src/gemdos/ssystem.rs tos-api/src/host.rs kernel/src/reset.rs; do
  cp $f $S/before-tree/$f
done
sed -i '' -e 's/RESET_STUB_VA/RESET_ENTRY_VA/g' -e 's/reset stub/reset entry/g' \
  $S/before-tree/tos-api/src/bdos/process/loader.rs
(cd $S/before-tree && cargo xtask build --board rpi4 --release > $S/build-rename.log 2>&1; echo "build rename-only $?")
cp $S/before-tree/target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-rename.img

# 2. after
touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask build --board rpi4 --release > $S/build-after.log 2>&1; echo "build after $?"
cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-after.img
shasum -a 256 $S/kernel-*.img | tee $R/kernels.sha256
VWPROBE_SCRATCH=$S/vw tosdump/probes/run-vwprobe.sh newtos $R/vwprobe-NEWTOS-after $S/kernel-after.img > $S/vw-after.out 2>&1; echo "vw after $?"
SXPROBE_SCRATCH=$S/sx tosdump/probes/run-sxprobe.sh newtos $R/sxprobe-NEWTOS-after $S/kernel-after.img > $S/sx-after.out 2>&1; echo "sx after $?"
HKPROBE_SCRATCH=$S/hk tosdump/probes/run-hkprobe.sh newtos $R/hkprobe-NEWTOS-after $S/kernel-after.img > $S/hk-after.out 2>&1; echo "hk after $?"

# 3. P0PAR injector kernel: scratch copy of this tree
rm -rf $S/inj-tree; mkdir -p $S/inj-tree
rsync -a --exclude target --exclude .git --exclude 'tosdump/build' --exclude references --exclude measurements --exclude Applications ./ $S/inj-tree/
python3 tools/inject-arm.py $S/inj-tree launch:P0PAR=$M/method/p0.script
diff -ru kernel $S/inj-tree/kernel > $M/method/injector-p0par-scratch.diff
diff -r tos-api/src $S/inj-tree/tos-api/src && echo "inj tos-api identical"
(cd $S/inj-tree && cargo xtask build --board rpi4 --release > $S/inj-build.log 2>&1; echo "inj build $?")
cp $S/inj-tree/target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-p0par.img
shasum -a 256 $S/kernel-p0par.img | tee -a $R/kernels.sha256
$M/method/run-p0par.sh $S $S/kernel-p0par.img $R/p0par-NEWTOS-after

# 4. qemu-virt boot (fresh build for the board)
mkdir -p $R/boot-qemu-virt-after
touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask run --board qemu-virt --release --headless > $R/boot-qemu-virt-after/BOOT-SERIAL.txt 2>&1 &
pid=$!
for i in $(seq 1 80); do sleep 3; grep -aq "desktop up" $R/boot-qemu-virt-after/BOOT-SERIAL.txt && break; done
sleep 3
for c in $(pgrep -P $pid); do for g in $(pgrep -P $c); do kill $g 2>/dev/null; done; kill $c 2>/dev/null; done
kill $pid 2>/dev/null; wait $pid 2>/dev/null
grep -a "desktop up\|PANIC" $R/boot-qemu-virt-after/BOOT-SERIAL.txt
echo ALL-DONE
