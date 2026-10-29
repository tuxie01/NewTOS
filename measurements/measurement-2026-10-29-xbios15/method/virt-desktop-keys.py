#!/usr/bin/env python3
"""virt-desktop-keys.py -- XBIOS15: qemu-virt boots to the desktop and the console
keyboard is the USB keyboard (what `cargo xtask run` now shows in its window).

usage: virt-desktop-keys.py <flat kernel image> <out-dir> <qmp-socket-path>

QEMU virt as `cargo xtask run` starts it (gic-version=2, cortex-a72, 272M, ramfb,
qemu-xhci + usb-kbd + usb-mouse), but `-display none` (the --headless form), serial to
<out>/SERIAL.txt, QMP on <qmp-socket-path> (< 104 bytes). After "[desk] desktop up":
screendump desktop.ppm; then QMP `send-key` Ctrl+Alt+Delete -- QEMU delivers it to the
only keyboard, the usb-kbd -- and wait for the kernel's IKBD decoder line
"[reset] keyboard Ctrl+Alt+Del" (kernel/src/ikbd.rs decode_key; this board has no restart
mechanism, so the desktop runs on); then screendump after.ppm. Bytes sent on the serial
line meanwhile go to AUX, not to the keyboard (checked by the serrx probe runs).
"""
import json, os, socket, subprocess, sys, time

kimg, out, sock = sys.argv[1:4]
os.makedirs(out, exist_ok=True)
ser = os.path.join(out, 'SERIAL.txt')
if os.path.exists(sock):
    os.unlink(sock)
argv = ['qemu-system-aarch64', '-M', 'virt,gic-version=2', '-cpu', 'cortex-a72', '-m', '272M',
        '-device', 'ramfb', '-device', 'qemu-xhci', '-device', 'usb-kbd', '-device', 'usb-mouse',
        '-display', 'none', '-serial', 'file:' + ser, '-no-reboot',
        '-qmp', f'unix:{sock},server,nowait', '-kernel', kimg]
log = open(os.path.join(out, 'run.txt'), 'w')
log.write('qemu: ' + ' '.join(argv) + '\n')
p = subprocess.Popen(argv, stdout=open(os.path.join(out, 'qemu-stdout.txt'), 'wb'), stderr=subprocess.STDOUT)

def serial():
    try:
        return open(ser, 'rb').read().decode('latin-1')
    except FileNotFoundError:
        return ''

def wait_for(text, limit):
    t0 = time.time()
    while time.time() - t0 < limit:
        if text in serial():
            return True
        if p.poll() is not None:
            return False
        time.sleep(0.2)
    return False

class Qmp:
    def __init__(self, path):
        for _ in range(100):
            try:
                self.s = socket.socket(socket.AF_UNIX); self.s.connect(path); break
            except OSError:
                time.sleep(0.1)
        self.f = self.s.makefile('rw')
        self.f.readline()
        self.cmd('qmp_capabilities')
    def cmd(self, name, **args):
        self.f.write(json.dumps({'execute': name, 'arguments': args}) + '\n'); self.f.flush()
        while True:
            r = json.loads(self.f.readline())
            if 'return' in r or 'error' in r:
                return r

try:
    up = wait_for('[desk] desktop up', 180)
    log.write(f'desktop up: {up}\n')
    q = Qmp(sock)
    time.sleep(2)
    log.write('screendump desktop: %s\n' % q.cmd('screendump', filename=os.path.join(out, 'desktop.ppm')))
    keys = [{'type': 'qcode', 'data': k} for k in ('ctrl', 'alt', 'delete')]
    log.write('send-key ctrl-alt-delete: %s\n' % q.cmd('send-key', keys=keys, **{'hold-time': 200}))
    got = wait_for('[reset] keyboard Ctrl+Alt+Del', 10)
    log.write(f'IKBD decoder saw Ctrl+Alt+Del from the USB keyboard: {got}\n')
    time.sleep(2)
    log.write('screendump after: %s\n' % q.cmd('screendump', filename=os.path.join(out, 'after.ppm')))
    log.write(f'qemu still running: {p.poll() is None}\n')
finally:
    p.kill(); p.wait()
    log.close()
    if os.path.exists(sock):
        os.unlink(sock)
