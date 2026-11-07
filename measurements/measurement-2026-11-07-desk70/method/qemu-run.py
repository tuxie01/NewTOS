#!/usr/bin/env python3
"""qemu-run.py <kernel8.img> <card.img> <out-dir> <seconds> [--append <bootargs>] -- one headless QEMU raspi4b boot on a per-run APFS clone of the
card (deleted after the run). Serial goes to a file; a persistent QMP connection takes screendumps as fast as QEMU answers
and keeps only the distinct screens (a PNG plus one line per run of identical dumps, host monotonic seconds); a second thread stamps each
new serial line with the same clock. Output: <out>/serial.log, <out>/serial-stamped.log, <out>/screens.txt, <out>/screen-NNN.png."""
import sys, os, json, socket, subprocess, time, threading, hashlib, shutil, tempfile
from PIL import Image
k, card, out, secs = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[3]), float(sys.argv[4])
app = sys.argv[sys.argv.index('--append')+1] if '--append' in sys.argv else 'serial=on'
os.makedirs(out); rd = tempfile.mkdtemp(prefix='run', dir=os.path.dirname(card)); c = rd + '/card.img'
subprocess.check_call(['cp', '-c', card, c])
ser = out + '/serial.log'; sock = 'q.sock'; os.chdir(rd); ppm = rd + '/s.ppm'
q = subprocess.Popen(['qemu-system-aarch64', '-M', 'raspi4b', '-no-reboot', '-display', 'none', '-append', app, '-kernel', k,
                      '-drive', f'file={c},if=sd,format=raw', '-serial', f'file:{ser}', '-qmp', f'unix:{sock},server=on,wait=off'])
t0 = time.monotonic()
stop = False
def tail():
    pos = 0; buf = b''
    with open(out + '/serial-stamped.log', 'w') as o:
        while not stop:
            try:
                with open(ser, 'rb') as f:
                    f.seek(pos); d = f.read(); pos += len(d)
            except FileNotFoundError:
                d = b''
            buf += d
            while b'\n' in buf:
                l, buf = buf.split(b'\n', 1)
                o.write(f"{time.monotonic()-t0:8.3f} {l.decode('latin-1').rstrip()}\n"); o.flush()
            time.sleep(0.02)
th = threading.Thread(target=tail, daemon=True); th.start()
for _ in range(100):
    if os.path.exists(sock): break
    time.sleep(0.05)
s = socket.socket(socket.AF_UNIX); s.connect(sock); f = s.makefile('rw')
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
f.readline(); cmd('qmp_capabilities')
prev = None; n = 0; first = None; runs = []; dumps = 0
tend = t0 + secs
while time.monotonic() < tend:
    tb = time.monotonic()
    cmd('screendump', filename=ppm)
    d = open(ppm, 'rb').read(); h = hashlib.sha1(d).hexdigest()[:12]; dumps += 1
    t = (tb + time.monotonic()) / 2 - t0
    if h != prev:
        if prev is not None: runs[-1][2] = lastt
        runs.append([n, t, t, h]); shutil.copy(ppm, f'{rd}/f{n:03d}.ppm'); n += 1; prev = h
    lastt = t
runs[-1][2] = lastt
cmd('quit')
try: q.wait(10)
except Exception: q.kill()
stop = True; th.join()
with open(out + '/screens.txt', 'w') as o:
    o.write(f"{dumps} dumps in {secs} s (mean spacing {secs/dumps*1000:.0f} ms)\n# screen first_s last_s sha1(ppm)\n")
    for a, b, c2, h in runs:
        o.write(f"{a:3d} {b:8.3f} {c2:8.3f} {h}\n")
        Image.open(f'{rd}/f{a:03d}.ppm').save(f'{out}/screen-{a:03d}.png')
shutil.rmtree(rd)
print(out, 'dumps', dumps, 'distinct', n)
