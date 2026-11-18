#!/bin/bash
# st-run.sh <virt|rpi4> <kernel> <tag> "<append>" -- boot the selftest kernel, serial -> serial/<tag>.log, stop at "done:" line + 3 s
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f; R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-v67-test
B=$1; K=$2; TAG=$3; APP=$4; LOG=$R/serial/$TAG.log; rm -f $LOG
if [ $B = rpi4 ]; then ARGS=(-M raspi4b -no-reboot -display none -append "serial=on $APP" -kernel $K)
else ARGS=(-M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -display none -no-reboot -kernel $K); [ -n "$APP" ] && ARGS+=(-append "$APP"); fi
qemu-system-aarch64 "${ARGS[@]}" -serial file:$LOG &
Q=$!; for _ in $(seq 1 120); do grep -aq "=== done" $LOG 2>/dev/null && break; kill -0 $Q 2>/dev/null || break; sleep 1; done; sleep 3; kill -9 $Q 2>/dev/null; wait $Q 2>/dev/null
echo "$TAG: $(grep -ac 'CHK' $LOG) CHK, $(grep -ac 'CHK.*FAIL' $LOG) FAIL, $(grep -ac PROBE $LOG) PROBE; $(grep -a 'SELFTEST\|=== done' $LOG | tr '\n' ' '); EXC/PANIC $(grep -aEc '\[EXC\]|PANIC|panicked' $LOG)"
