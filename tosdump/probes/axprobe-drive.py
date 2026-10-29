#!/usr/bin/env python3
"""axprobe-drive.py -- the NewTOS half of the input for tosdump/probes/axprobe.c
(TASK/XBIOS18): run QEMU with its serial0 and its QMP monitor on TCP sockets this
script listens on, log every serial byte, and once the probe printed
"AXPROBE:WAIT" press a key (QMP send-key, the usb-kbd on qemu-virt) and, later,
send one byte on the serial line.

usage: axprobe-drive.py <out-dir> <timeout-s> -- <qemu argv...>

The QEMU argv must contain the token @SOCKETS@; it is replaced by
  -chardev socket,id=s0,host=127.0.0.1,port=<p1> -serial chardev:s0
  -chardev socket,id=q0,host=127.0.0.1,port=<p2> -mon chardev=q0,mode=control
(QEMU connects as client to the ports this script listens on, so no byte is lost
and no Unix socket path is needed).

Timeline after "AXPROBE:WAIT" (the probe calls Bconin(1) right after printing it):
  +0.3 s  QMP send-key 'a', hold-time 100 ms (ST scancode 0x1E: make 1E, break 9E)
  +2.3 s  the byte 'Q' (0x51) on the serial line
Stops (SIGKILL) 2 s after "AXPROBE:END", when QEMU exits, or at <timeout-s>.
Writes <out>/SERIAL.txt (every byte QEMU sent on serial0), <out>/drive.txt (the
QEMU command, the QMP answers, the injection times), <out>/qemu-stdout.txt.
"""
import json
import os
import signal
import socket
import subprocess
import sys
import time

KEY_DELAY = 0.3
SERIAL_DELAY = 2.0
SERIAL_BYTE = b'Q'

out, timeout = sys.argv[1], float(sys.argv[2])
assert sys.argv[3] == '--'
qargv = sys.argv[4:]


def listener():
    s = socket.socket()
    s.bind(('127.0.0.1', 0))
    s.listen(1)
    s.settimeout(30)
    return s, s.getsockname()[1]


ser_srv, ser_port = listener()
qmp_srv, qmp_port = listener()
argv = []
for a in qargv:
    if a == '@SOCKETS@':
        argv += ['-chardev', f'socket,id=s0,host=127.0.0.1,port={ser_port}', '-serial', 'chardev:s0',
                 '-chardev', f'socket,id=q0,host=127.0.0.1,port={qmp_port}',
                 '-mon', 'chardev=q0,mode=control']
    else:
        argv.append(a)

log = open(os.path.join(out, 'drive.txt'), 'w')
log.write('qemu: ' + ' '.join(argv) + '\n')
log.flush()
p = subprocess.Popen(argv, stdout=open(os.path.join(out, 'qemu-stdout.txt'), 'wb'),
                     stderr=subprocess.STDOUT)
ser, _ = ser_srv.accept()
ser.settimeout(0.05)
qmp, _ = qmp_srv.accept()
qf = qmp.makefile('rw')
t0 = time.time()


def qmp_cmd(name, **args):
    msg = {'execute': name}
    if args:
        msg['arguments'] = args
    qf.write(json.dumps(msg) + '\n')
    qf.flush()
    while True:
        r = json.loads(qf.readline())
        if 'return' in r or 'error' in r:
            return r


log.write('qmp greeting: ' + qf.readline().strip() + '\n')
log.write('qmp_capabilities: %s\n' % qmp_cmd('qmp_capabilities'))
log.flush()

buf = b''
wait_at = key_at = None
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
        if wait_at is None and b'AXPROBE:WAIT' in buf:
            wait_at = now
            log.write(f'AXPROBE:WAIT seen at t+{now - t0:.2f}s, serial offset {len(buf)}\n')
        if wait_at is not None and key_at is None and now - wait_at >= KEY_DELAY:
            key_at = time.time()
            r = qmp_cmd('send-key', keys=[{'type': 'qcode', 'data': 'a'}], **{'hold-time': 100})
            log.write(f'send-key a at t+{key_at - t0:.2f}s (WAIT+{key_at - wait_at:.2f}s): {r}\n')
        if key_at is not None and not sent and now - key_at >= SERIAL_DELAY:
            ser.sendall(SERIAL_BYTE)
            sent = True
            log.write(f'serial {SERIAL_BYTE!r} at t+{time.time() - t0:.2f}s '
                      f'(key+{time.time() - key_at:.2f}s), serial offset {len(buf)}\n')
        if end_at is None and b'AXPROBE:END' in buf:
            end_at = now
        if end_at is not None and now - end_at > 2:
            log.write('end marker seen\n')
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
log.write(f'wait seen: {wait_at is not None}, key sent: {key_at is not None}, serial sent: {sent}\n')
log.close()
