#!/bin/bash
# scenarios.sh <head|base> -- qemu-virt dev kernel: per topology mouse move check (qmp_mouse.py + pixdiff.py),
# then keyboard check (qmp_key.py, Ctrl+Z -> EmuCON, "help") on default, hubkbd, latekbd.
. "$(dirname "$0")/env.sh"
M=$WS/measurements/measurement-2026-10-23-usb01-snap-test
V=$1
mkdir -p $S/shots
for T in default hubtree overfull mtp storage; do
  $M/method/run-virt.sh $S/bin/virt-dev-$V.img $M/serial/$V-$T.log 60 $T $M/method/qmp_mouse.py $S/shots $V-$T 2>&1 | grep -v "^screendump"
  python3 -I $M/method/pixdiff.py $S/shots/$V-$T-0-before.png $S/shots/$V-$T-1-after.png | head -1 | sed "s/^/$V $T mouse: /"
done
for T in default hubkbd latekbd; do
  $M/method/run-virt.sh $S/bin/virt-dev-$V.img $M/serial/$V-key-$T.log 60 $T $M/method/qmp_key.py $S/shots $V-key-$T - 2>&1 | grep -v "^screendump"
  python3 -I $M/method/pixdiff.py $S/shots/$V-key-$T-0-before.png $S/shots/$V-key-$T-1-after.png | head -1 | sed "s/^/$V $T Ctrl+Z: /"
  python3 -I $M/method/pixdiff.py $S/shots/$V-key-$T-1-after.png $S/shots/$V-key-$T-2-typed.png | head -1 | sed "s/^/$V $T typed help: /"
done
