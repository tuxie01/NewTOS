#!/usr/bin/env bash
# run.sh KERNEL8 IMG OUTDIR TAG TIMEOUT_S SPEC...   (SPEC = MARKER:PNG[:SETTLE]); raspi4b headless, QMP screendump at serial markers
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL8="$1"; IMG="$2"; OUTDIR="$3"; TAG="$4"; TMO="$5"; shift 5
mkdir -p "$OUTDIR"; LOG="$OUTDIR/$TAG-serial.log"
SOCK="/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a4ef322ddf8dbc7da/a83-$TAG.sock"; RUNIMG="/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a4ef322ddf8dbc7da/$TAG-run.img"
cp "$IMG" "$RUNIMG"; rm -f "$LOG" "$SOCK"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$LOG" -append serial=on -kernel "$KERNEL8" \
  -drive file="$RUNIMG",if=sd,format=raw -qmp unix:"$SOCK",server,nowait &
QPID=$!
python3 "$HERE/qmp_multi.py" "$SOCK" "$LOG" "$OUTDIR" "$TMO" "$@"; RC=$?
for _ in $(seq 1 20); do kill -0 "$QPID" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null; rm -f "$SOCK" "$RUNIMG"
echo "run $TAG: rc=$RC log=$LOG"; exit $RC
