#!/bin/bash
# aes-run.sh <kernel> <tag> "<append>" "<script>" -- qemu-virt + usb mouse, QMP-driven; PPM -> png in screendumps/
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f; R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-v67-test; M=$R/method
K=$1; TAG=$2; APP=$3; SCR=$4; LOG=$R/serial/$TAG.log; D=$A/xt/aes-$TAG; rm -rf $D; mkdir -p $D; rm -f $LOG; cd $D
ARGS=(-M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse -display none -no-reboot -kernel $K)
[ -n "$APP" ] && ARGS+=(-append "$APP")
qemu-system-aarch64 "${ARGS[@]}" -serial file:$LOG -qmp unix:$D/qmp.sock,server=on,wait=off &
Q=$!
python3 -I $M/aes-drive.py $D/qmp.sock $D $LOG "$SCR"
sleep 1; kill -9 $Q 2>/dev/null; wait $Q 2>/dev/null
for p in $D/*.ppm; do b=$(basename $p .ppm); sips -s format png $p --out $R/screendumps/$TAG-$b.png >/dev/null 2>&1; done
cd $A; mkdir -p $A/runs/$TAG; cp $D/*.ppm $A/runs/$TAG/; rm -rf $D
echo "aes run $TAG: EXC/PANIC $(grep -aEc '\[EXC\]|PANIC|panicked' $LOG); desktop up $(grep -ac 'desktop up' $LOG)"
