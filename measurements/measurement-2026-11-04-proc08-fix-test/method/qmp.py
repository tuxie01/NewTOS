#!/usr/bin/env python3
# qmp.py <socket> <command-json>... : send QMP commands, print replies.
import json, socket, sys, time
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(100):
    try:
        s.connect(sys.argv[1]); break
    except OSError:
        time.sleep(0.1)
f = s.makefile('rw')
f.readline()
def cmd(o):
    f.write(json.dumps(o) + '\n'); f.flush()
    while True:
        r = json.loads(f.readline())
        if 'return' in r or 'error' in r:
            return r
cmd({'execute': 'qmp_capabilities'})
for a in sys.argv[2:]:
    print(json.dumps(cmd(json.loads(a))))
