#!/bin/bash
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usb01-snt
export SOURCE_DATE_EPOCH=1792152000
cd $S/wt-base
rm -f target/aarch64-unknown-none-softfloat/release/kernel
cargo xtask build --board rpi4 --release --features usb-diag > $S/logs/base-rpi4-diag.log 2>&1; echo base diag rc=$?
for W in head base; do cd $S/wt-$W
 for B in qemu-virt rpi4; do
  cargo clippy -p kernel --no-default-features --features board-$B > $S/logs/clippy-$W-$B.log 2>&1; echo "$W $B rc=$? $(grep -E 'generated [0-9]+ warning' $S/logs/clippy-$W-$B.log | tr '\n' ' ')"
  grep -E "^\s+--> kernel/src/drivers/usb" $S/logs/clippy-$W-$B.log | sort -u | wc -l
 done
done
