#!/usr/bin/env python3
"""timed-shots.py <qmp-socket> <out-dir> <seconds> [...] : QMP screendump (PNG) at <seconds> after start."""
import json, os, socket, sys, time
sock_path, out = sys.argv[1], sys.argv[2]
times = [float(x) for x in sys.argv[3:]]
t0 = time.monotonic()
d = t0 + 60
while not os.path.exists(sock_path):
    if time.monotonic() > d: sys.exit("no socket")
    time.sleep(0.05)
s = socket.socket(socket.AF_UNIX); s.connect(sock_path); f = s.makefile("rw")
def cmd(c, a=None):
    m = {"execute": c}
    if a: m["arguments"] = a
    f.write(json.dumps(m) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline()); cmd("qmp_capabilities")
for i, t in enumerate(times):
    w = t0 + t - time.monotonic()
    if w > 0: time.sleep(w)
    p = os.path.join(os.path.abspath(out), "%02d-t%03d.png" % (i, int(t)))
    r = cmd("screendump", {"filename": p, "format": "png"})
    print(round(time.monotonic() - t0, 2), p, r)
