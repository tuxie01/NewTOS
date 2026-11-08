#!/usr/bin/env python3
"""virt-spin2.py <qmp-socket> <serial-file> <out-dir> -- V55 tester: key 'c' of the scratch desktop = spin 3e9, ONE pump pass
(evnt_multi MU_TIMER), spin 3e9, FLATHAND. The mouse is moved 8x(+20,+15) through QMP shortly after the key (during the first spin);
dumps every 1 s from the key on (s000.. ), host-timed; log gives the times."""
import json, os, socket, sys, time
sock_path, serial, out = sys.argv[1:4]
log = open(os.path.join(out, "virt-spin2.txt"), "w")
deadline = time.monotonic() + 240
while not os.path.exists(sock_path):
    if time.monotonic() > deadline: sys.exit("no QMP socket")
    time.sleep(0.05)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.connect(sock_path); f = s.makefile("rw")
def cmd(c, args=None):
    m = {"execute": c}
    if args: m["arguments"] = args
    f.write(json.dumps(m) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline()); cmd("qmp_capabilities")
t0 = time.monotonic()
while True:
    if time.monotonic() > deadline: sys.exit("desktop never came up")
    with open(serial, "rb") as sf:
        if b"[desk] desktop up" in sf.read(): break
    time.sleep(0.2)
time.sleep(3)
def shot(name):
    p = os.path.join(os.path.abspath(out), name + ".png")
    r = cmd("screendump", {"filename": p, "format": "png"})
    print("%.2f screendump %s %s" % (time.monotonic() - t0, name, r), file=log, flush=True)
shot("rest")
tk = time.monotonic()
for down in (True, False):
    cmd("input-send-event", {"events": [{"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": "c"}}}]})
    time.sleep(0.1)
print("%.2f key c sent" % (time.monotonic() - t0), file=log, flush=True)
time.sleep(1.0)
for _ in range(8):
    cmd("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": 20}}, {"type": "rel", "data": {"axis": "y", "value": 15}}]})
    time.sleep(0.05)
print("%.2f 8 moves sent" % (time.monotonic() - t0), file=log, flush=True)
n = 0
while time.monotonic() - tk < 30:
    shot("s%03d" % n); n += 1; time.sleep(0.8)
