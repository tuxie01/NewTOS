#!/usr/bin/env python3
"""mdrive.py SOCK OUTDIR TAG LOGFILE STEP... [--timeout S] [--quit-delay S]
Marker-keyed QMP screendumps (acceptance run 2026-10-15; replaces wall-clock-from-start offsets, which a sleeping host broke).
STEP = 'REGEX@N=DELAY:name.png' -- wait until the guest serial log (QEMU -serial file) holds the N-th match of REGEX,
sleep DELAY s, take screendump <TAG>-<name>. STEP 'REGEX@N=DELAY:-' = only wait (no dump). If a marker does not appear within
--timeout (default 150 s, per step) the dump is taken anyway as <TAG>-<name>.MISSING.png and the run ends (rc 3).
After the last step: quit after --quit-delay (default 0)."""
import json, os, re, socket, sys, time
a = sys.argv[1:]
sock, outdir, tag, logf = a[:4]
steps = []; timeout = 150.0; qd = 0.0
i = 4
while i < len(a):
    if a[i] == '--timeout': timeout = float(a[i+1]); i += 2; continue
    if a[i] == '--quit-delay': qd = float(a[i+1]); i += 2; continue
    m = re.match(r'^(.*)@(\d+)=([0-9.]+):(.*)$', a[i]); assert m, a[i]
    steps.append((m.group(1).encode(), int(m.group(2)), float(m.group(3)), m.group(4))); i += 1
t0 = time.time()
while not os.path.exists(sock):
    if time.time() - t0 > 30: sys.exit("qmp: socket never appeared")
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(100):
    try: s.connect(sock); break
    except OSError: time.sleep(0.2)
else: sys.exit("qmp: cannot connect")
f = s.makefile('rw', encoding='utf-8', newline='\n')
def cmd(o):
    f.write(json.dumps(o) + '\n'); f.flush()
    while True:
        l = f.readline()
        if not l: sys.exit("qmp: closed")
        m = json.loads(l)
        if 'return' in m or 'error' in m: return m
f.readline(); cmd({"execute": "qmp_capabilities"})
def dump(name):
    p = os.path.abspath(os.path.join(outdir, name))
    r = cmd({"execute": "screendump", "arguments": {"filename": p, "format": "png"}})
    if 'error' in r: r = cmd({"execute": "screendump", "arguments": {"filename": p}})
    print("t=%.1fs -> %s %s" % (time.time() - t0, name, json.dumps(r)), flush=True)
rc = 0
for rx, n, delay, name in steps:
    st = time.time(); ok = False
    while time.time() - st < timeout:
        try: data = open(logf, 'rb').read()
        except OSError: data = b''
        if len(re.findall(rx, data)) >= n: ok = True; break
        time.sleep(0.25)
    if not ok:
        print("MARKER-MISSING %r@%d" % (rx, n), flush=True)
        dump("%s-%s.MISSING.png" % (tag, name if name != '-' else 'x')); rc = 3; break
    time.sleep(delay)
    if name != '-': dump("%s-%s" % (tag, name))
time.sleep(qd)
try: cmd({"execute": "quit"})
except SystemExit: pass
sys.exit(rc)
