#!/bin/sh
# run-virt.sh <kernel.img> <serial.log> <seconds> <topology> [qmp-script args...]
#
# QEMU `-M virt` with the machine line tools/xtask uses for --board qemu-virt
# (gic-version=2, cortex-a72, 272M, ramfb, qemu-xhci), but a chosen USB
# topology instead of the fixed usb-kbd + usb-mouse pair. Serial to a file,
# QMP on a unix socket in the job scratch; the run is ended after <seconds>
# by QMP quit (or kill as the fallback -- macOS has no `timeout`).
#
# topology:
#   default  usb-kbd + usb-mouse on root ports (= tools/xtask)
#   hubtree  usb-hub on root port 1 with 7 HID devices behind it (4 usb-kbd,
#            3 usb-mouse) -- 8 device entries = MAX_HID, 3 commands each =
#            24 command TRBs, more than one pass over the 16-entry ring
#   overfull hubtree + a 9th device (usb-mouse) on root port 2 -- one more
#            than MAX_HID, to show the F8 line
#   storage  usb-storage (scratch raw image) + usb-kbd + usb-mouse
#   mtp      usb-mtp (Still Image class 6, subclass 1, protocol 1, with an
#            interrupt-IN event endpoint -- a non-HID interface whose protocol
#            byte is 1) + usb-tablet (HID, report protocol only) + usb-kbd +
#            usb-mouse
set -u
IMG=$1; LOG=$2; SECS=$3; TOPO=$4; shift 4
SCR=${SCR:-/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usb01-test/run}
mkdir -p "$SCR"
# Relative socket name, QEMU started inside $SCR: a unix socket path must be
# shorter than 104 bytes and the scratch path alone is longer.
SOCK="qmp-$$.sock"
case $TOPO in
  default)  USB="-device usb-kbd -device usb-mouse" ;;
  hubtree)  USB="-device usb-hub,bus=usb.0,port=1 \
                 -device usb-kbd,bus=usb.0,port=1.1 -device usb-kbd,bus=usb.0,port=1.2 \
                 -device usb-kbd,bus=usb.0,port=1.3 -device usb-kbd,bus=usb.0,port=1.4 \
                 -device usb-mouse,bus=usb.0,port=1.5 -device usb-mouse,bus=usb.0,port=1.6 \
                 -device usb-mouse,bus=usb.0,port=1.7" ;;
  overfull) USB="-device usb-hub,bus=usb.0,port=1 \
                 -device usb-kbd,bus=usb.0,port=1.1 -device usb-kbd,bus=usb.0,port=1.2 \
                 -device usb-kbd,bus=usb.0,port=1.3 -device usb-kbd,bus=usb.0,port=1.4 \
                 -device usb-mouse,bus=usb.0,port=1.5 -device usb-mouse,bus=usb.0,port=1.6 \
                 -device usb-mouse,bus=usb.0,port=1.7 -device usb-mouse,bus=usb.0,port=2" ;;
  storage)  # APFS clone of a zeroed base image, deleted after the run.
            [ -f "$SCR/stor-base.raw" ] || dd if=/dev/zero of="$SCR/stor-base.raw" bs=1m count=16 2>/dev/null
            cp -c "$SCR/stor-base.raw" "$SCR/stor-$$.raw"
            USB="-drive if=none,id=stick,format=raw,file=$SCR/stor-$$.raw \
                 -device usb-storage,bus=usb.0,port=1,drive=stick \
                 -device usb-kbd,bus=usb.0,port=2 -device usb-mouse,bus=usb.0,port=3" ;;
  mtp)      mkdir -p "$SCR/mtp-root"
            USB="-device usb-mtp,bus=usb.0,port=1,rootdir=$SCR/mtp-root,readonly=on \
                 -device usb-tablet,bus=usb.0,port=2 \
                 -device usb-kbd,bus=usb.0,port=3 -device usb-mouse,bus=usb.0,port=4" ;;
  hubkbd)   # TESTER: like hubtree but devices carry ids (kbd1..4, mouse1..3) for per-device QMP input
            USB="-device usb-hub,bus=usb.0,port=1 \
                 -device usb-kbd,id=kbd1,bus=usb.0,port=1.1 -device usb-kbd,id=kbd2,bus=usb.0,port=1.2 \
                 -device usb-kbd,id=kbd3,bus=usb.0,port=1.3 -device usb-kbd,id=kbd4,bus=usb.0,port=1.4 \
                 -device usb-mouse,id=mouse1,bus=usb.0,port=1.5 -device usb-mouse,id=mouse2,bus=usb.0,port=1.6 \
                 -device usb-mouse,id=mouse3,bus=usb.0,port=1.7" ;;
  latekbd)  # TESTER: 3 mice first, then 4 keyboards behind the hub: the keyboards are enumerated with commands >15
            USB="-device usb-hub,bus=usb.0,port=1 \
                 -device usb-mouse,id=mouse1,bus=usb.0,port=1.1 -device usb-mouse,id=mouse2,bus=usb.0,port=1.2 \
                 -device usb-mouse,id=mouse3,bus=usb.0,port=1.3 \
                 -device usb-kbd,id=kbd1,bus=usb.0,port=1.4 -device usb-kbd,id=kbd2,bus=usb.0,port=1.5 \
                 -device usb-kbd,id=kbd3,bus=usb.0,port=1.6 -device usb-kbd,id=kbd4,bus=usb.0,port=1.7" ;;
  *) echo "unknown topology $TOPO"; exit 2 ;;
esac
cd "$SCR"
# shellcheck disable=SC2086
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M \
  -device ramfb -device qemu-xhci,id=usb $USB \
  -display none -serial "file:$LOG" -no-reboot \
  -qmp "unix:$SOCK,server=on,wait=off" -kernel "$IMG" &
QPID=$!
if [ $# -gt 0 ]; then
  python3 -I "$@" "$SOCK" "$LOG" || echo "qmp script rc=$?"
else
  sleep "$SECS"
fi
kill "$QPID" 2>/dev/null
wait "$QPID" 2>/dev/null
rm -f "$SOCK" "$SCR/stor-$$.raw"
echo "run $TOPO done: $LOG"
