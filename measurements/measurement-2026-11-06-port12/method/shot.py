#!/usr/bin/env python3
"""shot.py <qmp-socket> <out.ppm> <out.png>: one QMP screendump in both formats."""
import json, os, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw")
def cmd(c, a=None):
    m = {"execute": c}
    if a: m["arguments"] = a
    f.write(json.dumps(m) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
json.loads(f.readline()); cmd("qmp_capabilities")
print(cmd("screendump", {"filename": os.path.abspath(sys.argv[2])}))
print(cmd("screendump", {"filename": os.path.abspath(sys.argv[3]), "format": "png"}))
