#!/bin/sh
# run-virt.sh <kernel.img> <serial.log> <topology> <qmp-script> [args...]
# QEMU -M virt with the machine line of tools/xtask (gic-version=2, cortex-a72, 272M, ramfb, qemu-xhci) and a chosen
# USB topology; serial to a file, QMP on a unix socket in the scratch dir; the QMP script ends the run.
# Adapted from measurements/measurement-2026-10-23-usb01-snap-test/method/run-virt.sh.
#   default  usb-kbd + usb-mouse (= cargo xtask run)
#   extra    = cargo xtask run --usb-extra: default + usb-tablet (port 3) + usb-hub (port 4) with usb-mouse 4.1 + usb-kbd 4.2
#   tablet   usb-tablet + usb-kbd only (the observed device as the only pointer)
#   none     the xHCI controller without any device ("no devices" message)
set -u
IMG=$1; LOG=$2; TOPO=$3; shift 3
SCR=${SCR:-/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usbsnap-t/run}
mkdir -p "$SCR"; SOCK="qmp-$$.sock"
case $TOPO in
  default) USB="-device qemu-xhci -device usb-kbd -device usb-mouse" ;;
  extra)   USB="-device qemu-xhci,id=usb -device usb-kbd -device usb-mouse -device usb-tablet,bus=usb.0,port=3 \
                -device usb-hub,bus=usb.0,port=4 -device usb-mouse,bus=usb.0,port=4.1 -device usb-kbd,bus=usb.0,port=4.2" ;;
  tablet)  USB="-device qemu-xhci,id=usb -device usb-tablet,bus=usb.0,port=1 -device usb-kbd,bus=usb.0,port=2" ;;
  none)    USB="-device qemu-xhci" ;;
  storage) # TESTER: a role-0 device (usb-storage, not an input device) next to kbd + mouse; scratch raw image
           [ -f "$SCR/stor-base.raw" ] || dd if=/dev/zero of="$SCR/stor-base.raw" bs=1m count=16 2>/dev/null
           cp -c "$SCR/stor-base.raw" "$SCR/stor-$$.raw"
           USB="-device qemu-xhci,id=usb -device usb-kbd -device usb-mouse -drive if=none,id=stick,format=raw,file=$SCR/stor-$$.raw -device usb-storage,bus=usb.0,drive=stick" ;;
  *) echo "unknown topology $TOPO"; exit 2 ;;
esac
cd "$SCR"; rm -f "$LOG"
# shellcheck disable=SC2086
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb $USB \
  -display none -serial "file:$LOG" -no-reboot -qmp "unix:$SOCK,server=on,wait=off" -kernel "$IMG" &
QPID=$!
python3 -I "$@" "$SOCK" "$LOG" || echo "qmp script rc=$?"
kill "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null; rm -f "$SOCK" "$SCR/stor-$$.raw"
echo "run $TOPO done: $LOG ($(grep -ac . "$LOG") lines, EXC/PANIC: $(grep -Eac '\[EXC\]|PANIC|panicked' "$LOG"))"
