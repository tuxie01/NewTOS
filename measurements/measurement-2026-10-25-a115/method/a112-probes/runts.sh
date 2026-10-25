#!/bin/bash
# usage: runts.sh <kernel8.img> <base.img> <log> <secs>
# QEMU raspi4b, -append serial=on; per-run APFS clone of the base image in the
# base image's directory (scratch), deleted afterwards; every serial line is
# host-timestamped (seconds since QEMU start). (Adapted from
# measurements/measurement-2026-10-17-a112-audit/method/runts.sh: kernel and
# scratch location are parameters, nothing is written next to this script.)
K=$1; B=$2; LOG=$3; SECS=$4
D=$(dirname "$B")
cp -c "$B" "$D/runtmp.img"
rm -f "$D/fifo"; mkfifo "$D/fifo"
perl -MTime::HiRes=time -ne 'BEGIN{$|=1;$t0=time} printf "%7.2f %s", time-$t0, $_' < "$D/fifo" > "$LOG" &
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/fifo" -kernel "$K" -append serial=on -drive file="$D/runtmp.img",if=sd,format=raw &
pid=$!; sleep "$SECS"; kill $pid 2>/dev/null; wait $pid 2>/dev/null
sleep 1; rm -f "$D/runtmp.img" "$D/fifo"
