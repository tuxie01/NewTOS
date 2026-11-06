#!/usr/bin/env python3 -I
"""hotkey-virt.py <kernel.img> <out-dir> -- the hot key of the debug feature sd-diag end to end on
QEMU qemu-virt (USB keyboard, ramfb): boots to the desktop, screendump, QMP send-key
ctrl-alt-f9 through the USB HID path, screendump, send-key again, screendump. raspi4b has no USB
in QEMU, so the key path can only be driven here."""
import json, os, socket, subprocess, sys, time

kernel, out = (os.path.abspath(a) for a in sys.argv[1:3])
os.makedirs(out, exist_ok=True)
os.chdir(out)  # the socket path stays short (AF_UNIX: 104 bytes on macOS)
sock = "q.sock"
serial = "SERIAL.txt"
qemu = subprocess.Popen(["qemu-system-aarch64", "-M", "virt,gic-version=2", "-cpu", "cortex-a72",
    "-m", "272M", "-device", "ramfb", "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse",
    "-display", "none", "-serial", "file:" + serial, "-no-reboot", "-kernel", kernel,
    "-qmp", "unix:%s,server=on,wait=off" % sock])
try:
    deadline = time.monotonic() + 60
    while not os.path.exists(sock):
        if time.monotonic() > deadline or qemu.poll() is not None:
            sys.exit("no QMP socket")
        time.sleep(0.05)
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(sock)
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
    while b"desktop up" not in open(serial, "rb").read():
        if time.monotonic() > deadline:
            sys.exit("no desktop")
        time.sleep(0.2)
    time.sleep(3)
    def shot(name):
        p = os.path.join(out, name)
        print(name, cmd("screendump", {"filename": p, "format": "png"}))
    def key():
        print("send-key", cmd("send-key", {"keys": [{"type": "qcode", "data": k} for k in ("ctrl", "alt", "f9")], "hold-time": 100}))
        time.sleep(2)
    shot("1-before.png")
    key()
    shot("2-shown.png")
    key()
    shot("3-hidden.png")
    key()
    shot("4-shown-again.png")
finally:
    qemu.terminate()
    qemu.wait()
    try:
        os.unlink(sock)
    except OSError:
        pass
