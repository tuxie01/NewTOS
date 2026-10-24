#!/usr/bin/env python3
"""qmp_key.py OUTDIR TAG DEVICES SOCK SERIAL_LOG -- keyboard check through QMP input-send-event.

Waits for "[desk] desktop up", screendump TAG-0-before.png, then types Ctrl+Z on each device id of DEVICES
(comma list; "-" = no device id = QEMU's default keyboard route), screendump TAG-1-after.png, quit.
Ctrl+Z in the NEWDESK desktop = File > "Execute EmuCON" (CLIITEM, default shortcut ^Z): the desktop leaves for the
EmuCON console, a full-screen change that proves the key travelled USB HID -> IKBD -> AES -> desktop.
"""
import json, os, socket, sys, time
outdir, tag, devs, sock_path, log_path = sys.argv[1:6]
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
def shot(n):
    p = os.path.join(outdir, f"{tag}-{n}.png")
    print("screendump", p, json.dumps(cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}}))); time.sleep(0.5)
shot("0-before")
def key(k, down, dev):
    a = {"events": [{"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}}]}
    if dev != "-": a["device"] = dev
    r = cmd({"execute": "input-send-event", "arguments": a})
    if 'error' in r: print("input-send-event", dev, k, json.dumps(r))
for dev in devs.split(","):
    print("typing Ctrl+Z via device", dev)
    for k, d in (("ctrl", True), ("z", True), ("z", False), ("ctrl", False)):
        key(k, d, dev); time.sleep(0.15)
    time.sleep(1.0)
time.sleep(2.0)
shot("1-after")
# EmuCON is up now (if the key arrived): type "help" so the echo shows on screen
for k in ("h", "e", "l", "p"):
    key(k, True, "-"); time.sleep(0.1); key(k, False, "-"); time.sleep(0.2)
time.sleep(1.5)
shot("2-typed")
cmd({"execute": "quit"})
