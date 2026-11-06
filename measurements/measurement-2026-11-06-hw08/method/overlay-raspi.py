#!/usr/bin/env python3 -I
"""overlay-raspi.py <kernel8.img> <card.img> <out-dir> -- the report of sd-diag on the screen on
QEMU raspi4b, 1920x1080: boots the card (the desktop's #Z starts WFPROBE, case b3), takes a QMP
screendump when the first failure has shown the report by itself ("[sddiag] end" on the serial
line, no key involved), and one when the probe has ended."""
import json, os, socket, subprocess, sys, time

kernel, card, out = (os.path.abspath(a) for a in sys.argv[1:4])
os.makedirs(out, exist_ok=True)
os.chdir(out)  # the socket path stays short (AF_UNIX: 104 bytes on macOS)
serial = "SERIAL.txt"
qemu = subprocess.Popen(["qemu-system-aarch64", "-M", "raspi4b", "-no-reboot", "-display", "none",
    "-serial", "file:" + serial, "-append", "serial=on", "-kernel", kernel,
    "-drive", "file=%s,if=sd,format=raw" % card, "-qmp", "unix:q.sock,server=on,wait=off"])
try:
    deadline = time.monotonic() + 180
    while not os.path.exists("q.sock"):
        if time.monotonic() > deadline or qemu.poll() is not None:
            sys.exit("no QMP socket")
        time.sleep(0.05)
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("q.sock")
    f = s.makefile("rw")
    def cmd(c, args=None):
        m = {"execute": c}
        if args:
            m["arguments"] = args
        f.write(json.dumps(m) + "\n")
        f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r:
                return r
    json.loads(f.readline())
    cmd("qmp_capabilities")
    def wait_for(text):
        while text.encode() not in open(serial, "rb").read():
            if time.monotonic() > deadline:
                sys.exit("never saw " + text)
            time.sleep(0.02)
    def shot(name):
        print(name, cmd("screendump", {"filename": name, "format": "png"}))
    wait_for("[sddiag] end")
    shot("1-first-failure.png")
    wait_for("WFPROBE done")
    shot("2-probe-end.png")
finally:
    qemu.terminate()
    qemu.wait()
    try:
        os.unlink("q.sock")
    except OSError:
        pass
