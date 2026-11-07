#!/usr/bin/env python3
"""qemu-shots.py <kernel8.img> <card.img> <out-dir> <seconds> <interval> [<stop-regex> <count>]

One headless QEMU raspi4b boot (-append serial=on) on a per-run APFS clone of the card (`cp -c`, deleted after the run).
Serial goes to <out>/serial.log, stamped with host-monotonic seconds into <out>/serial-stamped.log. A QMP screendump every
<interval> s; only distinct screens are kept (<out>/screen-NNN.png plus one line per run of identical dumps in
<out>/screens.txt: first/last host second, sha1 of the PPM). Optional: stop 3 s after <stop-regex> has matched <count>
serial lines. Derived from measurements/measurement-2026-11-07-desk70/method/qemu-run.py (same clone, QMP and stamping;
fixed interval instead of back-to-back dumps, optional serial stop condition).
After QEMU ends, the clone's sha256 is printed (the image a run wrote to)."""
import sys, os, re, json, socket, subprocess, time, threading, hashlib, shutil, tempfile
from PIL import Image
k, card, out = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2]), os.path.abspath(sys.argv[3])
secs, iv = float(sys.argv[4]), float(sys.argv[5])
stop_re = re.compile(sys.argv[6]) if len(sys.argv) > 7 else None
stop_n = int(sys.argv[7]) if len(sys.argv) > 7 else 0
os.makedirs(out); rd = tempfile.mkdtemp(prefix='run', dir=os.path.dirname(card)); c = rd + '/card.img'
subprocess.check_call(['cp', '-c', card, c])
ser = out + '/serial.log'; sock = 'q.sock'; os.chdir(rd); ppm = rd + '/s.ppm'
q = subprocess.Popen(['qemu-system-aarch64', '-M', 'raspi4b', '-no-reboot', '-display', 'none', '-append', 'serial=on',
                      '-kernel', k, '-drive', f'file={c},if=sd,format=raw', '-serial', f'file:{ser}',
                      '-qmp', f'unix:{sock},server=on,wait=off'])
t0 = time.monotonic()
stop = False; hits = 0; hit_t = None
def tail():
    global hits, hit_t
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
                s = l.decode('latin-1').rstrip()
                o.write(f"{time.monotonic()-t0:8.3f} {s}\n"); o.flush()
                if stop_re and stop_re.search(s):
                    hits += 1
                    if hits >= stop_n and hit_t is None: hit_t = time.monotonic()
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
prev = None; n = 0; runs = []; dumps = 0; lastt = 0.0
tend = t0 + secs
while time.monotonic() < tend and not (hit_t is not None and time.monotonic() > hit_t + 3):
    tb = time.monotonic()
    cmd('screendump', filename=ppm)
    d = open(ppm, 'rb').read(); h = hashlib.sha1(d).hexdigest()[:12]; dumps += 1
    t = (tb + time.monotonic()) / 2 - t0
    if h != prev:
        if prev is not None: runs[-1][2] = lastt
        runs.append([n, t, t, h]); shutil.copy(ppm, f'{rd}/f{n:03d}.ppm'); n += 1; prev = h
    lastt = t
    time.sleep(max(0.0, iv - (time.monotonic() - tb)))
runs[-1][2] = lastt
cmd('quit')
try: q.wait(10)
except Exception: q.kill()
stop = True; th.join()
el = time.monotonic() - t0
with open(out + '/screens.txt', 'w') as o:
    o.write(f"{dumps} dumps in {el:.1f} s (interval {iv} s); stop: {sys.argv[6] if stop_re else 'none'} x{stop_n} "
            f"{'matched' if hit_t else 'NOT matched'}\n# screen first_s last_s sha1(ppm)\n")
    for a, b, c2, h in runs:
        o.write(f"{a:3d} {b:8.3f} {c2:8.3f} {h}\n")
        Image.open(f'{rd}/f{a:03d}.ppm').save(f'{out}/screen-{a:03d}.png')
print('clone-after-run', subprocess.check_output(['shasum', '-a', '256', c]).decode().split()[0])
shutil.rmtree(rd)
print(out, 'dumps', dumps, 'distinct', n, 'stop', 'matched' if hit_t else 'not matched')
