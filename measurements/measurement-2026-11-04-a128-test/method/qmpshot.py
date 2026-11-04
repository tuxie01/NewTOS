#!/usr/bin/env python3
"""qmpshot.py SOCK FILE.ppm -- one QMP screendump."""
import json, socket, sys, time
s = socket.socket(socket.AF_UNIX)
for _ in range(50):
    try:
        s.connect(sys.argv[1]); break
    except OSError:
        time.sleep(0.1)
f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, "arguments": a}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r
json.loads(f.readline()); cmd("qmp_capabilities")
print(cmd("screendump", filename=sys.argv[2]))
