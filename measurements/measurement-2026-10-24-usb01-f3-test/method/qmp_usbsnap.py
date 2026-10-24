#!/usr/bin/env python3
"""qmp_usbsnap.py MODE SOCK SERIAL_LOG -- drive USBSNAP (measurement build usbsnap-virt.c as NEWDESK.NAP) on qemu-virt.

MODE run   : wait for "Press RETURN to start", send RETURN; wait for " Recording..."; during the recording send
             20 x rel(+10,+6) mouse moves, a left click, a right click, a middle click, wheel up/down, a key 'a' and
             Shift+'b' (no device id: QEMU routes them to the default devices; TASK/USB02 item 7: device= aborts on
             QEMU 11.0.2); then absolute moves (the usb-tablet, if any); wait for "Press any key to exit";
             screendump SHOT_DIR/<tag>-done.png (env SHOT_DIR, TAG), then wait 3 s: the program must still wait
             (muted keys did not queue: no second "USBSNAP - USB device recorder" start yet), then RETURN -> the
             shell restarts the program (2nd title line); quit.
MODE wait  : like run but sends nothing during the recording (the 15 s start timeout is used: no RETURN).
MODE nodev : controller without devices: wait for the "No USB device was detected" message, screendump, quit.
"""
import json, os, socket, sys, time
mode, sock_path, log_path = sys.argv[1:4]
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
def log(): return open(log_path, errors='replace').read()
def wait(marker, limit=180, count=1):
    t = time.time()
    while log().count(marker) < count:
        if time.time() - t > limit: print(f"marker {marker!r} x{count} NOT FOUND"); return False
        time.sleep(0.1)
    print(f"marker {marker!r} x{count} at {time.time() - t0:.1f}s"); return True
def ev(*events):
    r = cmd({"execute": "input-send-event", "arguments": {"events": list(events)}})
    if 'error' in r: print("input-send-event:", json.dumps(r))
def key(k, down): ev({"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}})
def tap(k): key(k, True); time.sleep(0.08); key(k, False); time.sleep(0.15)
def btn(b): ev({"type": "btn", "data": {"down": True, "button": b}}); time.sleep(0.1); ev({"type": "btn", "data": {"down": False, "button": b}}); time.sleep(0.2)
def shot(name):
    d = os.environ.get("SHOT_DIR")
    if d:
        p = os.path.join(d, f"{os.environ.get('TAG', 'run')}-{name}.png")
        print("screendump", p, json.dumps(cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}})))
TITLE = "USBSNAP - USB device recorder"
if mode == "nodev":
    ok = wait("No USB device was detected at boot", limit=60) and wait("Press any key to exit", limit=10)
    time.sleep(1.0); shot("nodev"); cmd({"execute": "quit"}); sys.exit(0 if ok else 1)
if not wait("Press RETURN to start"): cmd({"execute": "quit"}); sys.exit(1)
time.sleep(1.0); shot("list")
if mode == "run":
    tap("ret")
if not wait(" Recording...", limit=40): cmd({"execute": "quit"}); sys.exit(1)
if mode == "run":
    time.sleep(1.0)
    for _ in range(20):
        ev({"type": "rel", "data": {"axis": "x", "value": 10}}, {"type": "rel", "data": {"axis": "y", "value": 6}}); time.sleep(0.05)
    for b in ("left", "right", "middle", "wheel-up", "wheel-down"): btn(b)
    tap("a"); key("shift", True); time.sleep(0.05); tap("b"); key("shift", False)
    for x, y in ((1000, 1000), (20000, 15000), (32000, 30000)):
        ev({"type": "abs", "data": {"axis": "x", "value": x}}, {"type": "abs", "data": {"axis": "y", "value": y}}); time.sleep(0.1)
    time.sleep(8.0); shot("recording")
if not wait("Press any key to exit", limit=60): cmd({"execute": "quit"}); sys.exit(1)
time.sleep(1.0); shot("done")
time.sleep(3.0)
print("title lines before the exit key:", log().count(TITLE))
tap("ret")
ok = wait(TITLE, limit=20, count=2)
print("restarted after the key:", ok)
time.sleep(1.0)
cmd({"execute": "quit"})
