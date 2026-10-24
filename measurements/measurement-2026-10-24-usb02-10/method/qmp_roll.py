#!/usr/bin/env python3
"""qmp_roll.py OUTDIR TAG SOCK SERIAL_LOG -- USB02 item 10: more than six keys held on QEMU's usb-kbd.
Derived from qmp_hold.py (same QMP plumbing). Wait for the desktop, Ctrl+Z (EmuCON), then press a s d f g h (six keys,
0.05 s apart), press the 7th key j, release j, release h..a, all within about one second; screendump -0-roll.
If the device sends ErrorRollOver for the 7-key state, the old decoder breaks the six keys and makes them again when j
is released (the six letters typed a second time); the new decoder keeps the key set (the six letters once; j is never made,
it was only ever reported inside the phantom state)."""
import json, os, socket, sys, time
outdir, tag, sock_path, log_path = sys.argv[1:5]
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
def key(k, down):
    r = cmd({"execute": "input-send-event", "arguments": {"events": [{"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}}]}})
    if 'error' in r: print("input-send-event", json.dumps(r))
def shot(n):
    p = os.path.join(outdir, f"{tag}-{n}.png"); print("screendump", p, json.dumps(cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}}))); time.sleep(0.5)
for k, d in (("ctrl", True), ("z", True), ("z", False), ("ctrl", False)): key(k, d); time.sleep(0.15)
time.sleep(3.0)
six = ["a", "s", "d", "f", "g", "h"]
for k in six: key(k, True); time.sleep(0.05)
key("j", True); time.sleep(0.1); key("j", False); time.sleep(0.1)
for k in reversed(six): key(k, False); time.sleep(0.03)
time.sleep(1.5); shot("0-roll")
cmd({"execute": "quit"})
