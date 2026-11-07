#!/usr/bin/env python3
"""bc-timing.py <qmp-socket> <serial-file> <out-dir> <tag> : poll the QEMU serial file every 10 ms; record the
wall time since start of the first byte and of each marker line; at the last marker + 3 s take a QMP screendump.
Markers: first byte, 'SX15', 'B1 BIOS', 'BC07'. Also dumps at fixed times 12 14 16 18 20 22 (s after start) so the
fine-grained image sequence exists. Output: out-dir/<tag>-times.txt, out-dir/<tag>-tNN.png"""
import json, os, socket, sys, time
sock_path, ser, out, tag = sys.argv[1:5]
t0 = time.monotonic()
while not os.path.exists(sock_path):
    if time.monotonic() - t0 > 60: sys.exit("no socket")
    time.sleep(0.02)
s = socket.socket(socket.AF_UNIX); s.connect(sock_path); f = s.makefile("rw")
def cmd(c, a=None):
    m = {"execute": c}
    if a: m["arguments"] = a
    f.write(json.dumps(m) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline()); cmd("qmp_capabilities")
markers = [("\nT%d " % i).encode() for i in (20,23,25,27,28,29,30,31,32,33)] + [b"\nSX01", b"SX15", b"BC07"]
seen = {}
times = [12, 14, 16, 18, 20, 22]
log = open(os.path.join(out, tag + "-times.txt"), "w")
first = None
end = None
while True:
    now = time.monotonic() - t0
    data = open(ser, "rb").read() if os.path.exists(ser) else b""
    if data and first is None:
        first = now; log.write("first-byte %.2f\n" % now)
    for m in markers:
        if m not in seen and m in data:
            seen[m] = now; log.write("%s %.2f\n" % (m.decode(), now))
    if times and now >= times[0]:
        t = times.pop(0)
        p = os.path.join(os.path.abspath(out), "%s-t%03d.png" % (tag, t))
        cmd("screendump", {"filename": p, "format": "png"}); log.write("dump t%d at %.2f\n" % (t, now))
    if b"BC07" in seen and end is None: end = now + 3
    if end is not None and now >= end and not times:
        p = os.path.join(os.path.abspath(out), "%s-final.png" % tag)
        cmd("screendump", {"filename": p, "format": "png"}); log.write("final dump at %.2f\n" % now); break
    if now > 90: log.write("timeout\n"); break
    time.sleep(0.01)
log.close()
