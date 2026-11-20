import json, socket, sys, time, os, subprocess
kernel, tag = sys.argv[1], sys.argv[2]
D = os.path.dirname(os.path.abspath(__file__)); R = D + "/vbruns/" + tag
os.makedirs(R, exist_ok=True); log = R + "/serial.log"; open(log, "w").close(); os.chdir(R)
p = subprocess.Popen(["qemu-system-aarch64", "-M", "virt,gic-version=2", "-cpu", "cortex-a72", "-m", "272M", "-device", "ramfb",
    "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse", "-display", "none", "-no-reboot", "-kernel", kernel,
    "-serial", "file:" + log, "-qmp", "unix:q.sock,server=on,wait=off"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(50):
    if os.path.exists("q.sock"): break
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX); s.connect("q.sock"); f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
f.readline(); cmd("qmp_capabilities")
t0 = time.time()
while "desktop up" not in open(log, errors="replace").read() and time.time() - t0 < 150: time.sleep(0.5)
print("desktop up", "desktop up" in open(log, errors="replace").read(), round(time.time() - t0, 1))
time.sleep(float(os.environ.get("PRE","3")))
print(cmd("screendump", filename=R + "/a.ppm"))
for i in range(20):
    cmd("input-send-event", events=[{"type": "rel", "data": {"axis": "x", "value": 8}}, {"type": "rel", "data": {"axis": "y", "value": 5}}]); time.sleep(0.1)
time.sleep(2)
print(cmd("screendump", filename=R + "/b.ppm"))
cmd("quit"); time.sleep(1); p.kill(); p.wait()
for x in "ab":
    subprocess.run(["sips", "-s", "format", "png", f"{R}/{x}.ppm", "--out", f"{R}/{x}.png"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); os.remove(f"{R}/{x}.ppm")
