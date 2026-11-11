#!/bin/bash
# run-qmp.sh <repo> <scratch> <kernel8.img> <out-dir> <nap>... -- like measurement-2026-10-29-bios06/method/run-newtos.sh
# (card: tools/mkcard.sh, #Z autorun of the first NAP, QEMU raspi4b headless, -append serial=on) plus a QMP
# socket (relative path) for screendumps / key events: before the wait, after the wait, after a key.
set -u
REPO=$1; S=$2; K=$3; D=$4; shift 4
NAP1=$(basename "$1"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; RUN="$S/run-$(basename "$D").img"
rm -f "$IMG" "$IMG.manifest" "$RUN"
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$NAP1" > "$S/z-$NAP1.inf"
cp "$S/z-$NAP1.inf" "$D/NEWDESK.INF"
PAIRS=(); for n in "$@"; do PAIRS+=("$(basename "$n")=$n"); done
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAP1.inf" "${PAIRS[@]}" >"$D/mkcard.out" 2>&1) || { echo mkcard failed; cat "$D/mkcard.out"; exit 1; }
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
cp -c "$IMG" "$RUN" 2>/dev/null || cp "$IMG" "$RUN"
: > "$D/BOOT-SERIAL.txt"
cd "$S"; rm -f qmp-$$.sock
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -qmp unix:qmp-$$.sock,server,nowait -append serial=on -kernel "$K" -drive file="$RUN",if=sd,format=raw &
pid=$!
qmp() { python3 -I - "$@" <<'PY'
import socket,sys,json,time
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f=s.makefile('rw')
f.readline(); 
def cmd(c):
    f.write(json.dumps(c)+"\n"); f.flush()
    while True:
        l=json.loads(f.readline())
        if 'return' in l or 'error' in l: return l
cmd({"execute":"qmp_capabilities"})
a=sys.argv[2]
if a=="dump": print(cmd({"execute":"screendump","arguments":{"filename":sys.argv[3],"format":"png"}}))
elif a=="key":
    for down in (True,False):
        print(cmd({"execute":"input-send-event","arguments":{"events":[{"type":"key","data":{"down":down,"key":{"type":"qcode","data":sys.argv[3]}}}]}}))
        time.sleep(0.15)
PY
}
for i in $(seq 1 120); do sleep 2; /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break; kill -0 $pid 2>/dev/null || break; done
sleep 8
qmp qmp-$$.sock dump "$D/before-key.png"; cp "$D/BOOT-SERIAL.txt" "$D/BOOT-SERIAL.before-key.txt"
qmp qmp-$$.sock key spc > "$D/key.out" 2>&1
sleep 6
qmp qmp-$$.sock dump "$D/after-key.png"
sleep 2; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
for f in GNPROBE.LOG GNPROB2.LOG GNPARENT.LOG; do mcopy -o -i "$RUN@@32256" ::/$f "$D/$f" 2>/dev/null; done
shasum -a 256 "$RUN" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest" "$RUN" "$S/qmp-$$.sock"
ls -l "$D"
