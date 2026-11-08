#!/usr/bin/env python3
"""timed-shots.py <qmp-socket> <out-dir> <seconds> [...] : QMP screendump (PNG) at <seconds> after start.

V58: written for this run after the pattern of measurements/measurement-2026-11-07-v51/method/timed-shots.py;
the connect is retried (V51 lost two runs to ConnectionRefusedError right after the socket file appeared).
"""
import json, os, socket, sys, time

sock_path, out = sys.argv[1], sys.argv[2]
times = [float(x) for x in sys.argv[3:]]
t0 = time.monotonic()
deadline = t0 + 60
s = None
while s is None:
    if time.monotonic() > deadline:
        sys.exit("no QMP connection")
    if os.path.exists(sock_path):
        try:
            c = socket.socket(socket.AF_UNIX)
            c.connect(sock_path)
            s = c
            break
        except OSError:
            c.close()
    time.sleep(0.1)
f = s.makefile("rw")


def cmd(c, a=None):
    m = {"execute": c}
    if a:
        m["arguments"] = a
    f.write(json.dumps(m) + "\n")
    f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r


json.loads(f.readline())
cmd("qmp_capabilities")
for i, t in enumerate(times):
    w = t0 + t - time.monotonic()
    if w > 0:
        time.sleep(w)
    p = os.path.join(os.path.abspath(out), "%02d-t%03d.png" % (i, int(t)))
    r = cmd("screendump", {"filename": p, "format": "png"})
    print(round(time.monotonic() - t0, 2), p, r)
