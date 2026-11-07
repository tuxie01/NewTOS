#!/usr/bin/env python3
"""prof-smoke.py <kernel.img> <out-dir> : (<kernel.img> = llvm-objcopy -O binary of the qemu-virt ELF, as
`xtask run` boots it) qemu-virt headless (the `xtask run --headless` machine,
plus a QMP socket) with a raster-prof kernel. After "[desk] desktop up": screendump, Ctrl+Alt+F8
(show: report + restart), screendump, Ctrl+Alt+F8 (hide), screendump, wait 5 s, Ctrl+Alt+F8 again
(show the interval), screendump. Serial to <out>/serial.txt, PNGs to <out>/."""
import json, os, socket, subprocess, sys, time

elf, out = sys.argv[1], os.path.abspath(sys.argv[2])
os.makedirs(out, exist_ok=True)
os.chdir(out)  # QMP socket by relative path: macOS limits sun_path to 104 bytes
sock = "qmp.sock"
serial = os.path.join(out, "serial.txt")
cmd = ["qemu-system-aarch64", "-M", "virt,gic-version=2", "-cpu", "cortex-a72", "-m", "272M",
       "-device", "ramfb", "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse",
       "-display", "none", "-serial", "file:" + serial, "-no-reboot", "-kernel", elf,
       "-qmp", "unix:%s,server=on,wait=off" % sock]
open(os.path.join(out, "cmd.txt"), "w").write(" ".join(cmd) + "\n")
q = subprocess.Popen(cmd, cwd=out)
try:
    t0 = time.monotonic()
    while not os.path.exists(sock):
        if q.poll() is not None:
            sys.exit("qemu exited: %s" % q.returncode)
        time.sleep(0.05)
    s = socket.socket(socket.AF_UNIX); s.connect(sock); f = s.makefile("rw")
    def qmp(c, a=None):
        m = {"execute": c}
        if a:
            m["arguments"] = a
        f.write(json.dumps(m) + "\n"); f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r:
                return r
    json.loads(f.readline()); qmp("qmp_capabilities")
    while b"desktop up" not in open(serial, "rb").read():
        if time.monotonic() - t0 > 120:
            sys.exit("no desktop")
        time.sleep(0.2)
    time.sleep(3)
    def shot(n):
        print(n, qmp("screendump", {"filename": os.path.join(out, n + ".png"), "format": "png"}))
    def hot():
        keys = [{"type": "qcode", "data": k} for k in ("ctrl", "alt", "f8")]
        print("send-key", qmp("send-key", {"keys": keys, "hold-time": 120}))
        time.sleep(2)
    shot("0-desktop"); hot(); shot("1-shown"); hot(); shot("2-hidden")
    time.sleep(5); hot(); shot("3-shown-again")
finally:
    q.terminate(); time.sleep(1); q.kill()
