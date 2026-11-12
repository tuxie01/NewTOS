#!/usr/bin/env python3
# shot.py BOARD KERNEL8 SERIAL_LOG PNG  : boot headless, wait for "[desk] desktop up", QMP screendump
import socket, subprocess, sys, time, json, os
board, kern, log, png = sys.argv[1:5]
sock = "a68-%s.sock" % board
if os.path.exists(sock): os.remove(sock)
m = ["-M","raspi4b"]+(["-drive","file="+os.environ["SDIMG"]+",if=sd,format=raw"] if os.environ.get("SDIMG") else []) if board=="rpi4" else ["-M","virt,gic-version=2","-cpu","cortex-a72","-m","272M","-device","ramfb"]
q = subprocess.Popen(["qemu-system-aarch64"]+m+["-no-reboot","-display","none","-serial","file:"+log,"-kernel",kern,"-qmp","unix:%s,server,nowait"%sock])
try:
    for _ in range(100):
        if os.path.exists(sock): break
        time.sleep(0.1)
    s = socket.socket(socket.AF_UNIX); s.connect(sock); f = s.makefile("rw")
    f.readline(); f.write(json.dumps({"execute":"qmp_capabilities"})+"\n"); f.flush(); f.readline()
    for _ in range(300):
        if "desktop up" in open(log,errors="replace").read(): break
        time.sleep(0.3)
    time.sleep(int(os.environ.get("SETTLE","5")))
    f.write(json.dumps({"execute":"screendump","arguments":{"filename":os.path.abspath(png),"format":"png"}})+"\n"); f.flush()
    print(f.readline().strip())
    time.sleep(1)
finally:
    q.kill(); q.wait()
    if os.path.exists(sock): os.remove(sock)
