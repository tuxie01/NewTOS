# aes-drive.py <qmp.sock> <outdir> <serial.log> <script> -- drive the usb-mouse of qemu-virt through QMP; screendump named steps.
# script = semicolon list: park | mv:dx,dy | click | dclick | wait:s | shot:name
import json, socket, sys, time, os
sock, out, log, script = sys.argv[1:5]
t0 = time.time()
while not os.path.exists(sock):
    if time.time() - t0 > 30: sys.exit("no qmp socket")
    time.sleep(0.2)
for _ in range(100):
    s = socket.socket(socket.AF_UNIX)
    try: s.connect(sock); break
    except OSError: s.close(); time.sleep(0.3)
f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
f.readline(); cmd("qmp_capabilities")
t0 = time.time()
while "desktop up" not in open(log, errors="replace").read():
    if time.time() - t0 > 120: sys.exit("no desktop up")
    time.sleep(0.5)
time.sleep(4)
def rel(dx, dy):
    # chunks of at most 60 so the HID report does not clip
    while dx or dy:
        sx = max(-60, min(60, dx)); sy = max(-60, min(60, dy)); dx -= sx; dy -= sy
        cmd("input-send-event", events=[{"type":"rel","data":{"axis":"x","value":sx}},{"type":"rel","data":{"axis":"y","value":sy}}])
        time.sleep(0.04)
def btn(down):
    cmd("input-send-event", events=[{"type":"btn","data":{"down":down,"button":"left"}}])
for st in script.split(";"):
    st = st.strip()
    if st == "park": rel(-1500, -1500); time.sleep(0.5); rel(300, 300); time.sleep(0.8)
    elif st.startswith("mv:"): dx, dy = map(int, st[3:].split(",")); rel(dx, dy); time.sleep(0.8)
    elif st == "click": btn(True); time.sleep(0.15); btn(False); time.sleep(1.0)
    elif st == "dclick": btn(True); time.sleep(0.08); btn(False); time.sleep(0.12); btn(True); time.sleep(0.08); btn(False); time.sleep(1.2)
    elif st.startswith("wait:"): time.sleep(float(st[5:]))
    elif st.startswith("shot:"):
        print(st, cmd("screendump", filename=f"{out}/{st[5:]}.ppm"))
cmd("quit")
