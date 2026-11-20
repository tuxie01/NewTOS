import json, socket, sys, time, os, subprocess
kernel, card, tag, shots = sys.argv[1], sys.argv[2], sys.argv[3], [float(x) for x in sys.argv[4].split(",")]
D = os.path.dirname(os.path.abspath(__file__)); R = D + "/vbruns/" + tag
os.makedirs(R, exist_ok=True)
clone = D + "/run-" + tag + ".img"
subprocess.run(["cp", "-c", card, clone], check=True)
log = R + "/serial.log"; open(log, "w").close()
os.chdir(R)
p = subprocess.Popen(["qemu-system-aarch64", "-M", "raspi4b", "-no-reboot", "-display", "none", "-append", "serial=on",
    "-kernel", kernel, "-drive", f"file={clone},if=sd,format=raw", "-serial", "file:" + log, "-qmp", "unix:q.sock,server=on,wait=off"],
    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
def has(x):
    return x in open(log, errors="replace").read()
t0 = time.time()
while not has("armed at hz_200") and time.time() - t0 < 120: time.sleep(0.2)
armed = has("armed at hz_200"); ta = time.time()
print("armed", armed, round(ta - t0, 1))
for i, off in enumerate(shots):
    while time.time() - ta < off: time.sleep(0.05)
    r = cmd("screendump", filename=f"{R}/s{i}.ppm"); print("shot", i, off, round(time.time() - ta, 2), r)
t0 = time.time()
while not has("exited with rc") and time.time() - t0 < 60: time.sleep(0.5)
time.sleep(2)
cmd("quit"); time.sleep(1); p.kill(); p.wait()
for i in range(len(shots)):
    subprocess.run(["sips", "-s", "format", "png", f"{R}/s{i}.ppm", "--out", f"{R}/s{i}.png"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.remove(f"{R}/s{i}.ppm")
lba = int.from_bytes(open(clone, "rb").read(512)[0x1C6:0x1CA], "little")
env = dict(os.environ, MTOOLS_SKIP_CHECK="1")
subprocess.run(["mcopy", "-o", "-i", f"{clone}@@{lba*512}", "::/VBHOLD.LOG", R + "/VBHOLD.LOG"], env=env)
os.remove(clone)
