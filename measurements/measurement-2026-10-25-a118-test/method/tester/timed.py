#!/usr/bin/env python3
"""timed.py <kernel8.img> <base.img> <tag> <serial-dir> <shot-dir> <max-s> <t:name> ...
Headless QEMU raspi4b (-append serial=on) on an APFS clone; screendump <name> at t seconds after QEMU start."""
import json, os, socket, subprocess, sys, time
K, BASE, TAG, SER, SHOT, MAX = sys.argv[1:7]; MAX = float(MAX)
shots = sorted((float(a.split(":")[0]), a.split(":")[1]) for a in sys.argv[7:])
os.makedirs(SER, exist_ok=True); os.makedirs(SHOT, exist_ok=True)
rd = os.path.join(os.path.dirname(BASE), "run-" + TAG); os.makedirs(rd, exist_ok=True)
card = os.path.join(rd, "card.img"); log = os.path.join(SER, TAG + ".log"); sock = "qmp.sock"
if os.path.exists(card): os.remove(card)
subprocess.check_call(["cp", "-c", BASE, card])
if os.path.exists(log): os.remove(log)
SER = os.path.abspath(SER); SHOT = os.path.abspath(SHOT); log = os.path.abspath(log)
os.chdir(rd)
q = subprocess.Popen(["qemu-system-aarch64", "-M", "raspi4b", "-no-reboot", "-display", "none", "-append", "serial=on",
                      "-kernel", K, "-drive", "file=%s,if=sd,format=raw" % card, "-serial", "file:" + log,
                      "-qmp", "unix:%s,server=on,wait=off" % sock])
t0 = time.time()
def qmp(cmds):
    s = socket.socket(socket.AF_UNIX); s.connect(sock); f = s.makefile("rw"); f.readline()
    out = []
    for c, a in [("qmp_capabilities", None)] + cmds:
        f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r: out.append(r); break
    s.close(); return out
def shot(name):
    ppm = os.path.join(rd, "shot.ppm"); qmp([("screendump", {"filename": ppm})])
    png = os.path.join(SHOT, "%s-%s.png" % (TAG, name))
    subprocess.call(["sips", "-s", "format", "png", ppm, "--out", png], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    # serial marker with the time of the shot, so serial lines can be placed relative to it
    with open(log + ".shots", "a") as f: f.write("%.1f %s\n" % (time.time() - t0, name))
while time.time() - t0 < MAX and q.poll() is None:
    while shots and time.time() - t0 >= shots[0][0]:
        try: shot(shots[0][1])
        except Exception as e: print("shot fail", e)
        shots.pop(0)
    time.sleep(0.2)
try: qmp([("quit", None)])
except Exception: pass
for _ in range(40):
    if q.poll() is not None: break
    time.sleep(0.25)
if q.poll() is None: q.kill()
print("run", TAG, "left shots:", shots)
