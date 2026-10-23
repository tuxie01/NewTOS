#!/usr/bin/env python3
"""qmp_probe.py SOCK SERIAL_LOG -- drive the USBPROBE scratch kernel (method/usbprobe.c).

Waits for "USBPROBE: move now" and sends 20 relative mouse moves (+10,+6) through QMP
input-send-event (no device id: QEMU routes them to the usb-mouse); waits for "USBPROBE: muted"
and sends 10 more; waits for "bad pointer next", 4 s, then a screendump to SCREEN (env, optional)
and quit. Adapted from measurements/measurement-2026-10-23-usb01-test/method/qmp_mouse.py.
"""
import json, os, socket, sys, time
sock_path, log_path = sys.argv[1:3]
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
def wait(marker, limit=120):
    t = time.time()
    while marker not in open(log_path, errors='replace').read():
        if time.time() - t > limit: sys.exit(f"marker {marker!r} NOT FOUND")
        time.sleep(0.1)
    print(f"marker {marker!r} at {time.time() - t0:.1f}s")
def moves(n):
    for _ in range(n):
        r = cmd({"execute": "input-send-event", "arguments": {"events": [
            {"type": "rel", "data": {"axis": "x", "value": 10}},
            {"type": "rel", "data": {"axis": "y", "value": 6}}]}})
        if 'error' in r: print("input-send-event:", json.dumps(r))
        time.sleep(0.05)
wait("USBPROBE: move now"); moves(20)
wait("USBPROBE: muted"); moves(10)
wait("bad pointer next"); time.sleep(4)
if os.environ.get("SCREEN"):
    print(json.dumps(cmd({"execute": "screendump", "arguments": {"filename": os.environ["SCREEN"], "format": "png"}})))
cmd({"execute": "quit"})
