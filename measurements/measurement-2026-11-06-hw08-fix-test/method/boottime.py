#!/usr/bin/env python3 -I
"""boottime.py <out-dir> <rounds> <name>=<kernel> ... : QEMU raspi4b to 'desktop up', wall clock, kernels alternating, same card each time
(tools/mkcard.sh card, cloned per run). Serial log of every run kept."""
import os, subprocess, sys, time, shutil
out, rounds = sys.argv[1], int(sys.argv[2]); ks = [a.split("=", 1) for a in sys.argv[3:]]
scr = os.environ["BT_SCRATCH"]; repo = "/Volumes/Daten/Development/PiRusTOS"
os.makedirs(out, exist_ok=True); os.makedirs(scr, exist_ok=True)
open(scr + "/nd.inf", "w").write("#R 02\r\n")
subprocess.run([repo + "/tools/mkcard.sh", scr + "/card.img", scr + "/nd.inf"], check=True, capture_output=True, cwd=repo)
shutil.copy(scr + "/card.img.manifest", out + "/card.img.manifest")
res = {n: [] for n, _ in ks}
for r in range(rounds):
    for n, k in ks:
        shutil.copyfile(scr + "/card.img", scr + "/boot.img")
        log = "%s/%s-%d.serial" % (out, n, r)
        open(log, "w").close()
        t0 = time.monotonic()
        p = subprocess.Popen(["qemu-system-aarch64", "-M", "raspi4b", "-no-reboot", "-display", "none", "-serial", "file:" + log,
                              "-append", "serial=on", "-kernel", k, "-drive", "file=%s/boot.img,if=sd,format=raw" % scr],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        dt = None
        while time.monotonic() - t0 < 120:
            time.sleep(0.05)
            if "desktop up" in open(log, errors="replace").read():
                dt = time.monotonic() - t0; break
        p.kill(); p.wait()
        res[n].append(dt)
        print(n, r, "%.2f" % dt if dt else "TIMEOUT", flush=True)
for n, v in res.items():
    v = sorted(x for x in v if x)
    print("SUMMARY", n, "n=%d min %.2f median %.2f max %.2f" % (len(v), v[0], v[len(v)//2], v[-1]))
os.remove(scr + "/boot.img")
