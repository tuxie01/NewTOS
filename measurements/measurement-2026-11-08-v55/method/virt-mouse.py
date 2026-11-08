#!/usr/bin/env python3
"""virt-mouse.py <qmp-socket> <serial-file> <out-dir> -- TASK/V55: the USB mouse on QEMU virt moves
the pointer. Waits for "[desk] desktop up" on the serial file, settles 3 s, then: screendump
00-rest; QMP input-send-event relative motion (the usb-mouse device, which the kernel's HID
driver turns into IKBD packets) in 8 steps of +20,+15 with 50 ms between them, waits 1 s,
screendump 01-moved; 8 steps of -10,+5, 1 s, screendump 02-moved2. Writes <out-dir>/*.png
and <out-dir>/virt-mouse.txt (what was sent, when, and the QMP answers)."""
import json
import os
import socket
import sys
import time

sock_path, serial, out = sys.argv[1:4]
log = open(os.path.join(out, "virt-mouse.txt"), "w")
deadline = time.monotonic() + 240
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


json.loads(f.readline())
cmd("qmp_capabilities")
t0 = time.monotonic()
while True:
    if time.monotonic() > deadline:
        sys.exit("desktop never came up")
    with open(serial, "rb") as sf:
        if b"[desk] desktop up" in sf.read():
            break
    time.sleep(0.2)
time.sleep(3)


def shot(name):
    p = os.path.join(os.path.abspath(out), name + ".png")
    r = cmd("screendump", {"filename": p, "format": "png"})
    print("%.2f screendump %s %s" % (time.monotonic() - t0, name, r), file=log, flush=True)


def move(dx, dy, n):
    for _ in range(n):
        r = cmd("input-send-event", {"events": [
            {"type": "rel", "data": {"axis": "x", "value": dx}},
            {"type": "rel", "data": {"axis": "y", "value": dy}}]})
        print("%.2f rel %d %d %s" % (time.monotonic() - t0, dx, dy, r), file=log, flush=True)
        time.sleep(0.05)


shot("00-rest")
move(20, 15, 8)
time.sleep(1)
shot("01-moved")
move(-10, 5, 8)
time.sleep(1)
shot("02-moved2")
