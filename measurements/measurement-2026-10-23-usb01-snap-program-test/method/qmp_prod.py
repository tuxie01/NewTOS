#!/usr/bin/env python3
"""qmp_prod.py SOCK SERIAL_LOG -- the UNMODIFIED product USBSNAP (as NEWDESK.NAP) on qemu-virt, which has USB but no C:."""
import json, os, socket, sys, time
sock_path, log_path = sys.argv[1:3]
t0 = time.time()
while not os.path.exists(sock_path):
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
t = time.time()
while not any(k in log() for k in ("Press any key to exit", "Press RETURN to start")) and time.time() - t < 40: time.sleep(0.2)
time.sleep(1.5)
print(cmd({"execute": "screendump", "arguments": {"filename": os.environ["SHOT"], "format": "png"}}))
print("tail of log:"); print(log()[-900:])
cmd({"execute": "quit"})
