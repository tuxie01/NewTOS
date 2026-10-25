#!/usr/bin/env python3
"""run.py <kernel8.img> <base.img> <tag> <serial-dir> <shot-dir> <max-s> [MARK:delay_s:name ...]
One headless QEMU raspi4b boot (-append serial=on) on a per-run APFS clone (cp -c) of the base card.
Screendump <name> is taken <delay_s> seconds after the injector's feed line with "(+MARK)" first appears in
the serial log (MARK = script tick of the step). Quits after max-s; the card clone is kept as <tag>.img in
<shot-dir>/../cards-after for extraction and deleted by the caller."""
import json, os, socket, subprocess, sys, time, re
K, BASE, TAG, SER, SHOT, MAX = sys.argv[1:7]; MAX = float(MAX)
marks = []
for a in sys.argv[7:]:
    m, d, n = a.split(":"); marks.append([int(m), float(d), n, None])
os.makedirs(SER, exist_ok=True); os.makedirs(SHOT, exist_ok=True)
rd = os.path.join(os.path.dirname(BASE), "run-" + TAG); os.makedirs(rd, exist_ok=True)
card = os.path.join(rd, "card.img"); log = os.path.join(SER, TAG + ".log"); sock = "qmp.sock"  # relative: the absolute scratch path exceeds the 104-byte socket limit
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
    print("shot", name, "at %.1f s" % (time.time() - t0), flush=True)
while time.time() - t0 < MAX and q.poll() is None:
    try: txt = open(log, errors="replace").read()
    except FileNotFoundError: txt = ""
    for mk in marks:
        if mk[3] is None and re.search(r"\(\+%d\)" % mk[0], txt): mk[3] = time.time() + mk[1]
        if mk[3] is not None and mk[3] > 0 and time.time() >= mk[3]:
            shot(mk[2]); mk[3] = -1
    time.sleep(0.2)
try: qmp([("quit", None)])
except Exception: pass
for _ in range(40):
    if q.poll() is not None: break
    time.sleep(0.25)
if q.poll() is None: q.kill()
missing = [m[2] for m in marks if m[3] != -1]
print("run", TAG, "missing shots:", missing, "card:", card)
