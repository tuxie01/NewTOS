#!/usr/bin/env python3
"""Move the emulated USB mouse through QMP and screendump before/after.

usage: qmp_mouse.py OUTDIR TAG SOCK SERIAL_LOG
       (run-virt.sh appends SOCK and SERIAL_LOG)

Waits for "[desk] desktop up" in the serial log, takes screendump
OUTDIR/TAG-0-before.png, sends 20 relative moves of (+10, +6) through
`input-send-event` (QEMU's input layer, which the `usb-mouse` device turns
into real HID boot reports on its interrupt endpoint), takes
OUTDIR/TAG-1-after.png, then quits the guest. The pixel comparison is
method/pixdiff.py.
"""
import json
import os
import socket
import sys
import time

outdir, tag, sock_path, log_path = sys.argv[1:5]

t0 = time.time()
while not os.path.exists(sock_path):
    if time.time() - t0 > 30:
        sys.exit("qmp: socket never appeared")
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(100):
    try:
        s.connect(sock_path)
        break
    except OSError:
        time.sleep(0.2)
else:
    sys.exit("qmp: cannot connect")
f = s.makefile('rw', encoding='utf-8', newline='\n')


def cmd(obj):
    f.write(json.dumps(obj) + "\n")
    f.flush()
    while True:
        line = f.readline()
        if not line:
            sys.exit("qmp: connection closed")
        msg = json.loads(line)
        if 'return' in msg or 'error' in msg:
            return msg


f.readline()
cmd({"execute": "qmp_capabilities"})

marker = "[desk] desktop up"
t0 = time.time()
while True:
    try:
        if marker in open(log_path, errors='replace').read():
            break
    except FileNotFoundError:
        pass
    if time.time() - t0 > 120:
        sys.exit(f"marker {marker!r} NOT FOUND")
    time.sleep(0.2)
print(f"marker found at {time.time() - t0:.1f}s")
time.sleep(3.0)


def shot(name):
    p = os.path.join(outdir, f"{tag}-{name}.png")
    r = cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}})
    print(f"screendump {p}: {json.dumps(r)}")
    time.sleep(0.5)


shot("0-before")
for i in range(20):
    r = cmd({"execute": "input-send-event", "arguments": {"events": [
        {"type": "rel", "data": {"axis": "x", "value": 10}},
        {"type": "rel", "data": {"axis": "y", "value": 6}},
    ]}})
    if 'error' in r:
        print("input-send-event:", json.dumps(r))
    time.sleep(0.1)
time.sleep(1.0)
shot("1-after")
cmd({"execute": "quit"})
