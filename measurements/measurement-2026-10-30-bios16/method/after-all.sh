#!/bin/bash
# scratch driver: all "after" runs of BIOS16 on the final tree
set -u
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/45e8e806-7ba7-4edc-ace3-d4dd625a7f8d/scratchpad/bios16
REPO=/Volumes/Daten/Development/PiRusTOS
M=$REPO/measurements/measurement-2026-10-30-bios16
R=$M/runs
cd $REPO
$M/method/lint.sh $REPO $R/lint-after; echo "lint done"
touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask build --board rpi4 --release > $S/build-after.log 2>&1; echo "build rpi4 $?"
cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-after.img
touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask build --board rpi4 --release --features bios-arg-trace > $S/build-trace.log 2>&1; echo "build trace $?"
cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-after-trace.img
shasum -a 256 $S/kernel-*.img
VWPROBE_SCRATCH=$S/vw tosdump/probes/run-vwprobe.sh newtos $R/vwprobe-NEWTOS-after $S/kernel-after.img > $S/vw-na.out 2>&1; echo "vw after $?"
VWPROBE_SCRATCH=$S/vw tosdump/probes/run-vwprobe.sh newtos $R/vwprobe-NEWTOS-after-trace $S/kernel-after-trace.img > $S/vw-nt.out 2>&1; echo "vw trace $?"
SXPROBE_SCRATCH=$S/sx tosdump/probes/run-sxprobe.sh newtos $R/sxprobe-NEWTOS-after $S/kernel-after.img > $S/sx-after.out 2>&1; echo "sx after $?"
HKPROBE_SCRATCH=$S/hk tosdump/probes/run-hkprobe.sh newtos $R/hkprobe-NEWTOS-after $S/kernel-after.img > $S/hk-after.out 2>&1; echo "hk after $?"
mkdir -p $S/xt-after2
$M/method/run-tables.sh $R/after $REPO $S/xt-after2
# P0PAR injector kernel: scratch copy of this tree
rm -rf $S/inj-tree; mkdir -p $S/inj-tree
rsync -a --exclude target --exclude .git --exclude 'tosdump/build' --exclude references --exclude measurements --exclude Applications ./ $S/inj-tree/
python3 tools/inject-arm.py $S/inj-tree launch:P0PAR=$M/method/p0.script
diff -ru kernel $S/inj-tree/kernel > $M/method/injector-p0par-scratch.diff
diff -r tos-api/src $S/inj-tree/tos-api/src && echo "inj tos-api identical"
(cd $S/inj-tree && cargo xtask build --board rpi4 --release > $S/inj-build.log 2>&1; echo "inj build $?")
cp $S/inj-tree/target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-p0par.img
$M/method/run-p0par.sh $S $S/kernel-p0par.img $R/p0par-NEWTOS-after
# qemu-virt boot
mkdir -p $R/boot-qemu-virt-after
touch kernel/src/lib.rs
cargo xtask run --board qemu-virt --release --headless > $R/boot-qemu-virt-after/BOOT-SERIAL.txt 2>&1 &
pid=$!
for i in $(seq 1 80); do sleep 3; grep -aq "desktop up" $R/boot-qemu-virt-after/BOOT-SERIAL.txt && break; done
sleep 3
for c in $(pgrep -P $pid); do for g in $(pgrep -P $c); do kill $g 2>/dev/null; done; kill $c 2>/dev/null; done
kill $pid 2>/dev/null; wait $pid 2>/dev/null
grep -a "desktop up\|PANIC" $R/boot-qemu-virt-after/BOOT-SERIAL.txt
echo ALL-DONE
