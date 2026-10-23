#!/usr/bin/env python3
"""qmp_mutechk.py SOCK SERIAL_LOG -- drive method/mutechk.c (as NEWDESK.NAP on qemu-virt). No device ids (TASK/USB02 item 7)."""
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
def wait(marker, count=1, limit=120):
    t = time.time()
    while log().count(marker) < count:
        if time.time() - t > limit: print(f"marker {marker!r} x{count} NOT FOUND"); return None
        time.sleep(0.05)
    w = time.time() - t0; print(f"marker {marker!r} x{count} at {w:.1f}s"); return w
def ev(*e):
    r = cmd({"execute": "input-send-event", "arguments": {"events": list(e)}})
    if 'error' in r: print("input-send-event:", json.dumps(r))
def moves(k):
    for _ in range(k): ev({"type": "rel", "data": {"axis": "x", "value": 10}}, {"type": "rel", "data": {"axis": "y", "value": 6}}); time.sleep(0.05)
def btn(name, down): ev({"type": "btn", "data": {"down": down, "button": name}})
def key(k, down): ev({"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": k}}})
wait("MUTECHK: move now"); moves(10)
wait("MUTECHK: hold now"); btn("left", True); key("x", True)
wait("MUTECHK: muted"); time.sleep(0.5); btn("left", False); key("x", False); moves(10)
btn("left", True); time.sleep(0.1); btn("left", False)
wait("MUTECHK: unmuted"); moves(10)
tc = t0 + wait("MUTECHK: crash next")
time.sleep(5.0); key("ret", True); time.sleep(0.1); key("ret", False); print(f"RETURN at {time.time() - tc:.1f}s after the crash (muted)")
time.sleep(10.0)
print("restarted within 15 s after the crash (must be False):", log().count("MUTECHK: start") >= 2)
# from 25 s on: RETURN every 2 s until the shell restarts the probe (the mute deadline, 7000 ticks, ended)
w = None
while time.time() - tc < 90 and w is None:
    while time.time() - tc < 25: time.sleep(0.2)
    key("ret", True); time.sleep(0.1); key("ret", False); tk = time.time() - tc
    time.sleep(1.9)
    if log().count("MUTECHK: start") >= 2: w = tk
print("input back: the first RETURN that restarted the probe came", "never" if w is None else f"{w:.1f} s after the crash")
time.sleep(1.0); cmd({"execute": "quit"})
