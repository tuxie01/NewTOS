#!/usr/bin/env python3 -I
"""slprobe-qmpshots.py <qmp-socket> <serial-file> <shots.txt> <out-dir> -- the NewTOS half of
tosdump/probes/run-slprobe.sh (TASK/A130): waits for the injector's
"[inj] launch:SLPROBE armed at hz_200" line on the serial file, then takes one QMP
screendump (PNG) per line `<ticks> <name>` of shots.txt at <ticks>/200 s after that line
(QEMU raspi4b without icount: the guest's hz_200 runs on the host clock).  Writes
<out-dir>/<NN>-<name>.png and <out-dir>/qmpshots.txt (planned and actual offsets)."""
import json
import os
import socket
import sys
import time

sock_path, serial, shots_file, out = sys.argv[1:5]
shots = []
for line in open(shots_file):
    line = line.split("#", 1)[0].strip()
    if line:
        tk, name = line.split(None, 1)
        shots.append((int(tk), name.strip()))

deadline = time.monotonic() + 120
while not os.path.exists(sock_path):
    if time.monotonic() > deadline:
        sys.exit("no QMP socket")
    time.sleep(0.05)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock_path)
f = s.makefile("rw")


def cmd(c, args=None):
    m = {"execute": c}
    if args:
        m["arguments"] = args
    f.write(json.dumps(m) + "\n")
    f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r


json.loads(f.readline())  # greeting
cmd("qmp_capabilities")

marker = "launch:SLPROBE armed at hz_200"
pos = 0
t0 = None
while t0 is None:
    if time.monotonic() > deadline:
        sys.exit("injector never armed")
    with open(serial, "rb") as sf:
        sf.seek(pos)
        chunk = sf.read()
    if marker.encode() in chunk:
        t0 = time.monotonic()
        break
    # keep a tail so a marker split across reads is still found
    pos += max(0, len(chunk) - len(marker))
    time.sleep(0.005)

log = open(os.path.join(out, "qmpshots.txt"), "w")
log.write(f"t0 = detection of '{marker}' on the serial file\n")
for i, (tk, name) in enumerate(shots):
    due = t0 + tk / 200.0
    now = time.monotonic()
    if due > now:
        time.sleep(due - now)
    t_req = time.monotonic() - t0
    path = os.path.join(out, f"{i:02d}-{name}.png")
    r = cmd("screendump", {"filename": path, "format": "png"})
    t_done = time.monotonic() - t0
    log.write(f"{i:02d} {name} planned {tk / 200.0:.3f}s request {t_req:.3f}s done {t_done:.3f}s"
              f" {'ok' if 'return' in r else r}\n")
    log.flush()
log.close()
