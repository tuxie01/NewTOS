#!/usr/bin/env python3
"""qemu-serial-drive.py -- run QEMU with its serial0 on a TCP socket this script
listens on, log every received byte, and write bytes into the serial line once a
trigger text has been seen (XBIOS15: the serrx probe's "SERRX:WAIT").

usage: qemu-serial-drive.py <log> <trigger> <bytes|none> <end> <timeout-s> -- <qemu argv...>

The QEMU argv must contain the token @SERIAL@; it is replaced by
  -chardev socket,id=s0,host=127.0.0.1,port=<p> -serial chardev:s0
(QEMU connects as client to the port this script listens on, so no byte is lost).
After <trigger>: wait 0.3 s, send <bytes> (a Python escape string, e.g. 'aZ1'),
note the time and the byte offset in <log>.inject.txt. Stop (SIGKILL) 2 s after
<end> was seen, when QEMU exits, or at <timeout-s>.
"""
import os, signal, socket, subprocess, sys, time

log, trigger, data, end, timeout = sys.argv[1:6]
assert sys.argv[6] == '--'
qargv = sys.argv[7:]
srv = socket.socket(); srv.bind(('127.0.0.1', 0)); srv.listen(1)
port = srv.getsockname()[1]
argv = []
for a in qargv:
    if a == '@SERIAL@':
        argv += ['-chardev', f'socket,id=s0,host=127.0.0.1,port={port}', '-serial', 'chardev:s0']
    else:
        argv.append(a)
inj = open(log + '.inject.txt', 'w')
inj.write('qemu: ' + ' '.join(argv) + '\n'); inj.flush()
p = subprocess.Popen(argv, stdout=open(log + '.qemu-stdout.txt', 'wb'), stderr=subprocess.STDOUT)
srv.settimeout(30); c, _ = srv.accept(); c.settimeout(0.2)
buf = b''; t0 = time.time(); sent = False; end_at = None
with open(log, 'wb') as f:
    while True:
        try:
            d = c.recv(65536)
            if not d:
                break
            buf += d; f.write(d); f.flush()
        except socket.timeout:
            pass
        now = time.time()
        if not sent and trigger.encode() in buf:
            time.sleep(0.3)
            if data != 'none':
                b = data.encode().decode('unicode_escape').encode('latin-1')
                c.sendall(b)
                inj.write(f'sent {b!r} at t+{time.time()-t0:.2f}s, serial offset {len(buf)}\n')
            else:
                inj.write(f'nothing sent (control) at t+{time.time()-t0:.2f}s, serial offset {len(buf)}\n')
            inj.flush(); sent = True
        if end_at is None and end.encode() in buf:
            end_at = now
        if end_at is not None and now - end_at > 2:
            inj.write('end marker seen\n'); break
        if p.poll() is not None:
            inj.write(f'qemu exited {p.returncode}\n'); break
        if now - t0 > float(timeout):
            inj.write('timeout\n'); break
try:
    p.send_signal(signal.SIGKILL)
except Exception:
    pass
p.wait()
inj.write(f'trigger seen: {sent}\n'); inj.close()
