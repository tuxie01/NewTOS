#!/usr/bin/env python3
"""qmp_usb_clicks.py SOCK OUTDIR TAG : drive the usb-mouse of a qemu-virt run through QMP
(input-send-event: rel axes + btn) -- the REAL path usb::poll -> hid -> ikbd::feed,
unlike the scripted IKBD injector. Steps are plain data below; each step is
(wall_seconds_since_start, kind, args). Screendumps (QMP screendump, ppm) at 'shot'."""
import json, socket, sys, time
sock_path, out, tag = sys.argv[1:4]
s = socket.socket(socket.AF_UNIX); 
for _ in range(100):
    try: s.connect(sock_path); break
    except OSError: time.sleep(0.2)
f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, "arguments": a}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline()); cmd("qmp_capabilities")
def rel(dx, dy):
    ev = []
    if dx: ev.append({"type": "rel", "data": {"axis": "x", "value": dx}})
    if dy: ev.append({"type": "rel", "data": {"axis": "y", "value": dy}})
    cmd("input-send-event", events=ev)
def move(dx, dy, step=20, gap=0.01):
    # real mice report many small deltas
    while dx or dy:
        sx = max(-step, min(step, dx)); sy = max(-step, min(step, dy))
        rel(sx, sy); dx -= sx; dy -= sy; time.sleep(gap)
def btn(down): cmd("input-send-event", events=[{"type": "btn", "data": {"button": "left", "down": down}}])
def click(hold=0.08): btn(True); time.sleep(hold); btn(False)
def shot(name): cmd("screendump", filename=f"{out}/{tag}-{name}.ppm")
t0 = time.time()
def at(t):
    while time.time() - t0 < t: time.sleep(0.05)
exec(open(sys.argv[4]).read())   # the step file: uses at/move/click/btn/shot/rel
