#!/usr/bin/env python3
"""qmp_usb.py SOCK OUTDIR TAG STEPS -- drive the usb-mouse of a qemu-virt run over
QMP input-send-event (rel axes, left button, wheel-up/-down): the REAL path
xHCI -> usb::poll -> hid -> ikbd::feed -> mousexvec, unlike the scripted IKBD
injector. Copied from measurements/measurement-2026-11-04-a127/method/
qmp_usb_clicks.py, plus wheel(). The step file is Python using
at/move/click/btn/wheel/shot."""
import json, socket, sys, time
sock_path, out, tag = sys.argv[1:4]
s = socket.socket(socket.AF_UNIX)
for _ in range(100):
    try:
        s.connect(sock_path); break
    except OSError:
        time.sleep(0.2)
f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, "arguments": a}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r
json.loads(f.readline()); cmd("qmp_capabilities")
def rel(dx, dy):
    ev = []
    if dx: ev.append({"type": "rel", "data": {"axis": "x", "value": dx}})
    if dy: ev.append({"type": "rel", "data": {"axis": "y", "value": dy}})
    cmd("input-send-event", events=ev)
def move(dx, dy, step=20, gap=0.01):
    while dx or dy:
        sx = max(-step, min(step, dx)); sy = max(-step, min(step, dy))
        rel(sx, sy); dx -= sx; dy -= sy; time.sleep(gap)
def btn(down, button="left"):
    cmd("input-send-event", events=[{"type": "btn", "data": {"button": button, "down": down}}])
def click(hold=0.08): btn(True); time.sleep(hold); btn(False)
def wheel(direction, n=1, gap=0.15):
    for _ in range(n):
        b = "wheel-up" if direction < 0 else "wheel-down"
        btn(True, b); time.sleep(0.02); btn(False, b); time.sleep(gap)
def shot(name):
    print(time.time() - t0, "shot", name, cmd("screendump", filename=f"{out}/{tag}-{name}.ppm"), flush=True)
t0 = time.time()
def at(t):
    while time.time() - t0 < t: time.sleep(0.05)
exec(open(sys.argv[4]).read())
