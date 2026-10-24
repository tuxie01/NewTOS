#!/usr/bin/env python3
"""qmp_btn.py OUTDIR TAG X Y X2 Y2 SOCK SERIAL_LOG -- (TESTER) button check: wait for the desktop, clamp the pointer to the
top-left corner with large negative relative moves, move to (X,Y) relative to that corner in steps, screendump -0-moved,
(then on to X2,Y2 = a menu item, screendump -0b-on-item) press+release the LEFT button (click), screendump -1-clicked, then press the RIGHT button, screendump -2-right, quit."""
import json, os, socket, sys, time
outdir, tag, X, Y, X2, Y2, sock_path, log_path = sys.argv[1:9]; X=int(X); Y=int(Y); X2=int(X2); Y2=int(Y2)
t0 = time.time()
while not os.path.exists(sock_path):
    if time.time() - t0 > 30: sys.exit("qmp: socket never appeared")
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(100):
    try: s.connect(sock_path); break
    except OSError: time.sleep(0.2)
else: sys.exit("qmp: cannot connect")
f = s.makefile('rw', encoding='utf-8', newline='\n')
def cmd(o):
    f.write(json.dumps(o) + "\n"); f.flush()
    while True:
        m = json.loads(f.readline())
        if 'return' in m or 'error' in m: return m
f.readline(); cmd({"execute": "qmp_capabilities"})
t0 = time.time()
while "[desk] desktop up" not in open(log_path, errors='replace').read():
    if time.time() - t0 > 120: sys.exit("marker NOT FOUND")
    time.sleep(0.2)
time.sleep(3.0)
def ev(*e):
    r = cmd({"execute": "input-send-event", "arguments": {"events": list(e)}})
    if 'error' in r: print("input-send-event", json.dumps(r))
def rel(dx, dy): ev({"type": "rel", "data": {"axis": "x", "value": dx}}, {"type": "rel", "data": {"axis": "y", "value": dy}}); time.sleep(0.06)
def shot(n):
    p = os.path.join(outdir, f"{tag}-{n}.png"); print("screendump", p, json.dumps(cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}}))); time.sleep(0.5)
def btn(b, down): ev({"type": "btn", "data": {"down": down, "button": b}}); time.sleep(0.15)
for _ in range(30): rel(-100, -100)
for _ in range(X // 10): rel(10, 0)
for _ in range(Y // 10): rel(0, 10)
time.sleep(1.0); shot("0-moved")
for _ in range(abs(X2 - X) // 10): rel(10 if X2 > X else -10, 0)
for _ in range(abs(Y2 - Y) // 10): rel(0, 10 if Y2 > Y else -10)
time.sleep(1.0); shot("0b-on-item")
btn("left", True); btn("left", False); time.sleep(1.0); shot("1-clicked")
btn("right", True); time.sleep(0.5); shot("2-right-down"); btn("right", False); time.sleep(0.5)
cmd({"execute": "quit"})
