#!/usr/bin/env bash
# run-qmp.sh <rpi4|virt> <kernel> <card|-> <tag> "<append>" "<end-regex>" <after-s> <moves 0|1>
# One headless QEMU (one at a time). Serial -> serial/<tag>.log, QMP screendump(s) -> screendumps/<tag>*.png.
# moves=1 (virt): shot A, then relative usb-mouse motion over QMP, shot B (pointer-moving check).
# Card: per-run APFS clone of <card>, deleted after.
set -uo pipefail
B=$1; K=$2; CARD=$3; TAG=$4; APP=$5; END=$6; AFTER=$7; MOVES=${8:-0}
R=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7
RD=$(mktemp -d $R/xt/run-$TAG.XXXX); LOG=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-v63-test/serial/$TAG.log; rm -f $LOG; cd $RD
if [ $B = rpi4 ]; then
  ARGS=(-M raspi4b -no-reboot -display none -append "$APP" -kernel $K)
  if [ "$CARD" != - ]; then cp -c $CARD $RD/card.img; ARGS+=(-drive file=$RD/card.img,if=sd,format=raw); fi
else
  ARGS=(-M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse -display none -no-reboot -kernel $K)
  [ -n "$APP" ] && ARGS+=(-append "$APP")
fi
qemu-system-aarch64 "${ARGS[@]}" -serial file:$LOG -qmp unix:qmp.sock,server=on,wait=off &
QPID=$!
for _ in $(seq 1 ${TMO:-240}); do grep -Eq "$END" $LOG 2>/dev/null && break; kill -0 $QPID 2>/dev/null || break; sleep 1; done
sleep $AFTER
python3 -I - qmp.sock $RD $MOVES <<'PY'
import json, socket, sys, time
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
f.readline(); cmd("qmp_capabilities")
print("shotA", cmd("screendump", filename=sys.argv[2]+"/a.ppm"))
if sys.argv[3] == "1":
    for i in range(20):
        cmd("input-send-event", events=[{"type":"rel","data":{"axis":"x","value":8}},{"type":"rel","data":{"axis":"y","value":5}}]); time.sleep(0.1)
    time.sleep(2)
    print("shotB", cmd("screendump", filename=sys.argv[2]+"/b.ppm"))
cmd("quit")
PY
for _ in $(seq 1 20); do kill -0 $QPID 2>/dev/null || break; sleep 0.5; done; kill -9 $QPID 2>/dev/null; wait $QPID 2>/dev/null
for x in a b; do [ -f $RD/$x.ppm ] && sips -s format png $RD/$x.ppm --out /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-v63-test/screendumps/$TAG-$x.png >/dev/null 2>&1; done
cd $R; rm -rf $RD
echo "run $TAG: $(grep -c . $LOG) serial lines; end: $(grep -Ec "$END" $LOG); desktop up: $(grep -c 'desktop up' $LOG); EXC/PANIC: $(grep -Ec '\[EXC\]|PANIC|panicked' $LOG)"
