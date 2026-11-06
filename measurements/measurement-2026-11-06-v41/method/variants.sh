#!/bin/bash
# build the candidate with one part of the EmuTOS behaviour left out, run the probe
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad
cd $S/v41/cand
T=tos-api/src/vdi/text.rs
for v in 0 1 2 3 4 5 6 7 8 9; do
  sed -i '' "s/^const V41_OFF: u32 = [0-9]*;/const V41_OFF: u32 = $v;/" $T
  cargo xtask build --board rpi4 --release > $S/v41/out/build-$v.log 2>&1 || echo build-fail $v
  cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/v41/k/off$v.img
done
cd /Volumes/Daten/Development/PiRusTOS
export VDPROBE_SCRATCH=$S/v41/vp
i=0
for v in 0 1 2 3 4 5 6 7 8 9; do
  rm -rf $S/v41/out/off$v
  tosdump/probes/run-vdprobe.sh newtos $S/v41/out/off$v $S/v41/k/off$v.img > $S/v41/out/off$v.stdout 2>&1 &
  i=$((i+1)); [ $((i%3)) = 0 ] && wait
done
wait
echo finished
