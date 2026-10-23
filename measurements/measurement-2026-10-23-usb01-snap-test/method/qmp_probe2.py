#!/usr/bin/env python3
"""qmp_probe2.py SOCK SERIAL_LOG -- driver for usbprobe2.c: at "USBPROBE2: move now" send 300 relative mouse moves (+1,+1), then wait for "USBPROBE2: done", quit."""
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
wait("USBPROBE2: move now")
for i in range(300):
    r = cmd({"execute": "input-send-event", "arguments": {"events": [
        {"type": "rel", "data": {"axis": "x", "value": 1}}, {"type": "rel", "data": {"axis": "y", "value": 1}}]}})
    if 'error' in r: print("input-send-event:", json.dumps(r))
    time.sleep(0.01)
wait("USBPROBE2: done"); time.sleep(1)
cmd({"execute": "quit"})
