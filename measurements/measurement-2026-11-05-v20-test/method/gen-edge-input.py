#!/usr/bin/env python3 -I
"""gen-edge-input.py <out-dir> <lead-ticks> -- V20 acceptance, edge scenes: icon drags whose XOR outline runs past the
top / bottom screen edge, grab offset varied by one pixel (a pattern phase shift of an odd number of
pixels flips the dotted vertical segments).  Same script for NewTOS (lead 0, ticks after "desktop up")
and EmuTOS (lead = boot time before the desktop is up; ticks after the feeder's installation)."""
import sys, os
out, lead = sys.argv[1], int(sys.argv[2])
lines = []; shots = []
pos = [0, 0]; btn = [False]; t = [lead + 100]

def pkt(b, dx, dy):
    lines.append((t[0], 0xF8 | (2 if b else 0), dx & 255, dy & 255))
def move(x, y, step=30, dt=3):
    while (pos[0], pos[1]) != (x, y):
        dx = max(-step, min(step, x - pos[0])); dy = max(-step, min(step, y - pos[1]))
        pkt(btn[0], dx, dy); pos[0] += dx; pos[1] += dy; t[0] += dt
def press():
    btn[0] = True; pkt(True, 0, 0); t[0] += 150
def release():
    btn[0] = False; pkt(False, 0, 0); t[0] += 60
def shot(name):
    t[0] += 100; shots.append((t[0], name)); t[0] += 60

for i in range(5):
    pkt(False, -127, 0); t[0] += 1            # left along the initial row, never across the menu bar
for i in range(3):
    pkt(False, 0, -127); t[0] += 1            # then up along x = 0 (left of the first menu title)
t[0] += 200
shot("00-rest")
for tag, y in (("a", 40), ("b", 41)):
    move(37, y); press(); move(37, 8); shot("1%s-hd-top-edge-held" % tag); move(37, y); release()
for tag, y in (("a", 170), ("b", 171)):
    move(35, y); press(); move(35, 479); shot("2%s-trash-bottom-edge-held" % tag); move(35, y); release()
shot("30-end")
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, "mouse.txt"), "w") as f:
    for l in lines: f.write("%d 0x%02x 0x%02x 0x%02x\n" % l)
with open(os.path.join(out, "shots.txt"), "w") as f:
    for tk, n in shots: f.write("%d %s\n" % (tk, n))
print(len(lines), "packets,", len(shots), "shots, last tick", t[0])
