#!/usr/bin/env python3
"""virt-spin.py <qmp-socket> <serial-file> <out-dir> <key a|b> -- V55 tester scenario on qemu-virt (usb-kbd + usb-mouse).
Scratch desktop patched (TESTER-TEMP in programs/desktop/src/main.rs, scratch tree only): key 'a' = trap-free EL0 spin then
graf_mouse(FLATHAND); key 'b' = graf_mouse(M_OFF), spin, graf_mouse(M_ON), graf_mouse(FLATHAND).
Waits for "[desk] desktop up", settles, dump 00-rest, sends the key through QMP, then 8x(+20,+15) -> dump 01, 8x(+30,-10) -> dump 02,
every 5 s a dump till 90 s after the key (sNN-<t>.png) to see the spin end (the pointer turns into a hand)."""
import json, os, socket, sys, time
sock_path, serial, out, key = sys.argv[1:5]
log = open(os.path.join(out, "virt-spin.txt"), "w")
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
shot("00-rest")
tk = time.monotonic()
for down in (True, False):
    r = cmd("input-send-event", {"events": [{"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": key}}}]})
    print("%.2f key %s down=%s %s" % (time.monotonic() - t0, key, down, r), file=log, flush=True)
    time.sleep(0.1)
time.sleep(1.5)
shot("01-spin-rest")
move(20, 15, 8); time.sleep(1); shot("02-spin-moved")
move(30, -10, 8); time.sleep(1); shot("03-spin-moved2")
n = 0
while time.monotonic() - tk < 90:
    time.sleep(5); n += 1; shot("t%02d" % n)
