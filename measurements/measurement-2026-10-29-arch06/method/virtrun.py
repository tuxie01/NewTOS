#!/usr/bin/env python3
"""virtrun.py <ELF> <flat.img> <log> <shot.png> <max-s> <stop-regex> -- one headless qemu-virt boot (xtask run's argument set, -serial file), stop when the regex shows in the log (+3 s) or after max-s, QMP screendump, quit."""
import json, os, re, socket, subprocess, sys, time
ELF, IMG, LOG, PNG, MAX, RX = sys.argv[1:7]; MAX = float(MAX)
OBJ = os.path.expanduser("~/.rustup/toolchains/nightly-2026-04-15-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/rust-objcopy")
subprocess.check_call([OBJ, "-O", "binary", ELF, IMG])
d = os.path.dirname(os.path.abspath(LOG)); sock = "qv-%d.sock" % os.getpid(); os.chdir(os.environ.get("SOCKDIR", d))
if os.path.exists(LOG): os.remove(LOG)
q = subprocess.Popen(["qemu-system-aarch64", "-M", "virt,gic-version=2", "-cpu", "cortex-a72", "-m", "272M", "-device", "ramfb", "-device", "qemu-xhci",
                      "-device", "usb-kbd", "-device", "usb-mouse", "-display", "none", "-serial", "file:" + LOG, "-no-reboot", "-kernel", IMG,
                      "-qmp", "unix:%s,server=on,wait=off" % sock])
t0 = time.time()
while time.time() - t0 < MAX and q.poll() is None:
    try: txt = open(LOG, errors="replace").read()
    except FileNotFoundError: txt = ""
    if re.search(RX, txt): time.sleep(3); break
    time.sleep(0.5)
try:
    s = socket.socket(socket.AF_UNIX); s.connect(sock); f = s.makefile("rw"); f.readline()
    def cmd(c, a=None):
        f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r: return r
    cmd("qmp_capabilities"); ppm = PNG + ".ppm"; cmd("screendump", {"filename": os.path.abspath(ppm)})
    subprocess.call(["sips", "-s", "format", "png", ppm, "--out", PNG], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); os.remove(ppm)
    cmd("quit")
except Exception as e: print("qmp:", e)
time.sleep(1)
if q.poll() is None: q.kill()
print("elapsed %.1fs" % (time.time() - t0))
