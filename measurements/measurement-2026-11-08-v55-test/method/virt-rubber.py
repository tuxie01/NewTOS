#!/usr/bin/env python3
"""virt-rubber.py <qmp-socket> <serial-file> <out-dir> -- V55 tester rubber-band scenario on qemu-virt (usb-kbd + usb-mouse).
Scratch desktop patched (TESTER-TEMP in programs/desktop/src/main.rs, scratch tree only): key 'a' = trap-free EL0 spin then
graf_mouse(FLATHAND); key 'b' = graf_mouse(M_OFF), spin, graf_mouse(M_ON), graf_mouse(FLATHAND).
Waits for "[desk] desktop up", settles, dump 00-rest, sends the key through QMP, then 8x(+20,+15) -> dump 01, 8x(+30,-10) -> dump 02,
every 5 s a dump till 90 s after the key (sNN-<t>.png) to see the spin end (the pointer turns into a hand)."""
import json, os, socket, sys, time
sock_path, serial, out = sys.argv[1:4]
log = open(os.path.join(out, "virt-rubber.txt"), "w")
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
def move(dx, dy, n):
    for _ in range(n):
        r = cmd("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": dx}}, {"type": "rel", "data": {"axis": "y", "value": dy}}]})
        print("%.2f rel %d %d %s" % (time.monotonic() - t0, dx, dy, r), file=log, flush=True)
        time.sleep(0.05)

def btn(down):
    r = cmd("input-send-event", {"events": [{"type": "btn", "data": {"down": down, "button": "left"}}]})
    print("%.2f btn %s %s" % (time.monotonic() - t0, down, r), file=log, flush=True)
def move(dx, dy, n):
    for _ in range(n):
        cmd("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": dx}}, {"type": "rel", "data": {"axis": "y", "value": dy}}]})
        time.sleep(0.05)
shot("00-rest")
move(-30, -20, 8); time.sleep(0.5)   # to (719,379)
shot("01-start")
btn(True); time.sleep(0.3)
move(25, 20, 8); time.sleep(0.8)
shot("02-band-held")
move(25, 20, 8); time.sleep(0.8)
shot("03-band-held2")
btn(False); time.sleep(1.0)
shot("04-released")
move(10, 10, 6); time.sleep(0.8)
shot("05-moved-after")
