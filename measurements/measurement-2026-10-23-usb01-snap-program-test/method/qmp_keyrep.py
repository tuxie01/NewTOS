#!/usr/bin/env python3
"""qmp_keyrep.py SOCK SERIAL_LOG -- drive method/keyrep.c (NEWDESK.NAP on qemu-virt). No device ids (TASK/USB02 item 7)."""
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
f = s.makefile('rw', encoding='utf-8', newline='\n')
def cmd(o):
    f.write(json.dumps(o) + "\n"); f.flush()
    while True:
        m = json.loads(f.readline())
        if 'return' in m or 'error' in m: return m
f.readline(); cmd({"execute": "qmp_capabilities"})
def log(): return open(log_path, errors='replace').read()
def wait(marker, limit=150):
    t = time.time()
    while log().count(marker) < 1:
        if time.time() - t > limit: print(f"marker {marker!r} NOT FOUND"); return False
        time.sleep(0.02)
    print(f"marker {marker!r} at {time.time() - t0:.1f}s"); return True
def ev(*e):
    r = cmd({"execute": "input-send-event", "arguments": {"events": list(e)}})
    if 'error' in r: print("input-send-event:", json.dumps(r))
def btn(name, down): ev({"type": "btn", "data": {"down": down, "button": name}})
def key(k, down): ev({"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}})
def tap(k, hold=0.1): key(k, True); time.sleep(hold); key(k, False)
wait("KR: P1 idle now"); time.sleep(5.0)           # nothing at all for P1
wait("KR: P2 hold-a now"); time.sleep(0.2); tap("a", 1.0)   # 1 s hold (P2 is 1000 ticks = 5 s)
wait("KR: P3 tap-shift-buttons now"); time.sleep(0.3)
tap("b"); time.sleep(0.4); key("shift", True); time.sleep(0.1); tap("c"); key("shift", False); time.sleep(0.4)
btn("left", True); time.sleep(1.0); btn("left", False); time.sleep(0.4); btn("right", True); time.sleep(0.1); btn("right", False)
wait("KR: P4 mute-idle now"); time.sleep(3.0)
wait("KR: P5a hold-z-and-left now"); time.sleep(0.3); key("z", True); btn("left", True)
wait("KR: P5 mute-held now"); time.sleep(0.8); key("z", False); btn("left", False); time.sleep(1.4); tap("y"); btn("right", True); btn("right", False)
wait("KR: P6 typing now"); time.sleep(0.3); tap("d"); time.sleep(0.5); tap("e", 1.0)
wait("KR: END", 60); time.sleep(0.5); cmd({"execute": "quit"})
