#!/usr/bin/env bash
# run-noserial.sh <kernel> <card> <tag> -- as run.sh but WITHOUT -append serial=on (the real card's default):
# timed QMP screendumps every 4 s for 100 s; gdbstub on GDBPORT.
set -uo pipefail
K="$1"; CARD="$2"; TAG="$3"
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/45e8e806-7ba7-4edc-ace3-d4dd625a7f8d/scratchpad/doomfreeze
OUT="$S/out/$TAG"; mkdir -p "$OUT"
IMG="$S/run-$TAG.img"; cp -c "$CARD" "$IMG"
cd "$S"; Q="q-$TAG"; mkdir -p "$Q"; rm -f "$Q/qmp.sock"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$OUT/serial.log" -qmp unix:"$Q/qmp.sock",server=on,wait=off \
  -gdb tcp::${GDBPORT:-1234} &
QP=$!
python3 - "$Q/qmp.sock" "$OUT" "$TAG" <<'PY'
import json, socket, sys, time, os
sock, out, tag = sys.argv[1:4]
t0=time.time()
while not os.path.exists(sock): time.sleep(0.2)
s=socket.socket(socket.AF_UNIX); 
for _ in range(100):
    try: s.connect(sock); break
    except OSError: time.sleep(0.2)
f=s.makefile('rw')
def cmd(o):
    f.write(json.dumps(o)+'\n'); f.flush()
    while True:
        m=json.loads(f.readline())
        if 'return' in m or 'error' in m: return m
f.readline(); cmd({"execute":"qmp_capabilities"})
for i in range(26):
    time.sleep(4)
    cmd({"execute":"screendump","arguments":{"filename":os.path.abspath("%s/%s-%03d.png"%(out,tag,(i+1)*4)),"format":"png"}})
cmd({"execute":"quit"})
PY
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null
rm -rf "$S/$Q" "$IMG"
md5 -q "$OUT"/*.png | uniq -c | awk '{print $1}' | tr '\n' ' '; echo
