#!/usr/bin/env bash
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL="$1"; OUTDIR="$2"; TAG="$3"; TMO="$4"; shift 4
mkdir -p "$OUTDIR"; LOG="$OUTDIR/$TAG-serial.log"; SOCK="/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a4ef322ddf8dbc7da/a83-$TAG.sock"
rm -f "$LOG" "$SOCK"
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse -no-reboot -display none -serial file:"$LOG" -kernel "$KERNEL" -qmp unix:"$SOCK",server,nowait &
QPID=$!
python3 "$HERE/qmp_multi.py" "$SOCK" "$LOG" "$OUTDIR" "$TMO" "$@"; RC=$?
sleep 1; kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null; rm -f "$SOCK"
echo "run $TAG: rc=$RC"; exit $RC
