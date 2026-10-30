#!/usr/bin/env python3
"""awprobe-drive.py -- the serial side of a QEMU run of tosdump/probes/awprobe.c
(TASK/BIOS09): run QEMU with its serial0 on a TCP socket this script listens
on, log every serial byte, and once the probe printed "AWPROBE:WAIT" send one
byte on the serial line after <delay-s> seconds -- the byte that ends the
probe's first Bconin(1) wait (AUX reads the serial input ring on both boards
since BIOS09).

usage: awprobe-drive.py <out-dir> <timeout-s> <delay-s> -- <qemu argv...>

The QEMU argv must contain the token @SERIAL@; it is replaced by
  -chardev socket,id=s0,host=127.0.0.1,port=<p> -serial chardev:s0
(QEMU connects as client to the port this script listens on, so no byte is
lost).

Keys are not sent from here: QEMU raspi4b has no USB keyboard. A run that
wants the kbdvec hook to fire boots a kernel with the scratch IKBD injector
armed on the probe (tools/inject-arm.py ... launch:AWPROBE=<script>), see
run-awprobe.sh.

Stops (SIGKILL) 8 s after the probe printed its second "W" line (the wait
after the injected byte), when QEMU exits, or at <timeout-s>. Writes
<out>/SERIAL.txt (every byte QEMU sent on serial0), <out>/drive.txt (the QEMU
command, the injection time), <out>/qemu-stdout.txt.
"""
import os
import signal
import socket
import subprocess
import sys
import time

SERIAL_BYTE = b'Q'

out, timeout, delay = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
assert sys.argv[4] == '--'
qargv = sys.argv[5:]

srv = socket.socket()
srv.bind(('127.0.0.1', 0))
srv.listen(1)
srv.settimeout(30)
port = srv.getsockname()[1]
argv = []
for a in qargv:
    if a == '@SERIAL@':
        argv += ['-chardev', f'socket,id=s0,host=127.0.0.1,port={port}', '-serial', 'chardev:s0']
    else:
        argv.append(a)

log = open(os.path.join(out, 'drive.txt'), 'w')
log.write('qemu: ' + ' '.join(argv) + '\n')
log.flush()
p = subprocess.Popen(argv, stdout=open(os.path.join(out, 'qemu-stdout.txt'), 'wb'),
                     stderr=subprocess.STDOUT)
ser, _ = srv.accept()
ser.settimeout(0.05)
t0 = time.time()

buf = b''
wait_at = None
sent = False
end_at = None
with open(os.path.join(out, 'SERIAL.txt'), 'wb') as f:
    while True:
        try:
            d = ser.recv(65536)
            if not d:
                log.write('serial closed\n')
                break
            buf += d
            f.write(d)
            f.flush()
        except socket.timeout:
            pass
        now = time.time()
        if wait_at is None and b'AWPROBE:WAIT' in buf:
            wait_at = now
            log.write(f'AWPROBE:WAIT seen at t+{now - t0:.2f}s, serial offset {len(buf)}\n')
        if wait_at is not None and not sent and now - wait_at >= delay:
            ser.sendall(SERIAL_BYTE)
            sent = True
            log.write(f'serial {SERIAL_BYTE!r} at t+{time.time() - t0:.2f}s '
                      f'(WAIT+{time.time() - wait_at:.2f}s), serial offset {len(buf)}\n')
        if end_at is None and sent and b'AWPROBE|W2 ' in buf:
            end_at = now
        if end_at is not None and now - end_at > 8:
            log.write('second wait seen\n')
            break
        if p.poll() is not None:
            log.write(f'qemu exited {p.returncode}\n')
            break
        if now - t0 > timeout:
            log.write('timeout\n')
            break
        log.flush()
try:
    p.send_signal(signal.SIGKILL)
except Exception:
    pass
p.wait()
log.write(f'wait seen: {wait_at is not None}, serial sent: {sent}, second wait seen: {end_at is not None}\n')
log.close()
