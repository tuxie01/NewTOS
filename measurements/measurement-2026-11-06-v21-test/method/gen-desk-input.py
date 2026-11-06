#!/usr/bin/env python3 -I
"""gen-desk-input.py <out-dir> -- V20 acceptance: mouse script (<ticks> 0xF8|b dx dy) and shot schedule
for the desktop scenes. Absolute coordinates are tracked after pinning the pointer at (0,0)."""
import sys, os
out = sys.argv[1]
lines = []; shots = []
pos = [0, 0]
t = 0

def pkt(tk, b, dx, dy):
    assert -128 <= dx < 128 and -128 <= dy < 128
    lines.append((tk, 0xF8 | (2 if b else 0), dx & 255, dy & 255))

def move(x, y, step=30, dt=3):
    global t
    while (pos[0], pos[1]) != (x, y):
        dx = max(-step, min(step, x - pos[0])); dy = max(-step, min(step, y - pos[1]))
        pkt(t, btn[0], dx, dy); pos[0] += dx; pos[1] += dy; t += dt
btn = [False]
def press():
    global t
    btn[0] = True; pkt(t, True, 0, 0); t += 150
def release():
    global t
    btn[0] = False; pkt(t, False, 0, 0); t += 60
def wait(n):
    global t
    t += n
def shot(name):
    global t
    wait(100); shots.append((t, name)); wait(60)

t = 100
for i in range(5):
    pkt(t, False, -127, 0); t += 1            # left along the initial row, never across the menu bar
for i in range(3):
    pkt(t, False, 0, -127); t += 1            # then up along x = 0 (left of the first menu title)
wait(200)
shot("00-rest")
# menu
move(85, 8); shot("01-menu-file")
move(40, 8); move(80, 27); press(); release(); wait(150); shot("02-dialog")
move(320, 407); press(); release(); wait(100)
# rubberband
move(150, 300); press(); move(320, 430); shot("03-rubberband-held"); release()
# window drag by title (window at 336,97; title bar y 106)
move(480, 106); press(); move(400, 160); shot("04-window-drag-held"); release()
# slider drag: window now at (256,151); thumb centre (532,244)
move(532, 244); press(); move(532, 289); shot("05-slider-drag-held"); release()
# sizer drag: sizer centre (533,427)
move(533, 427); press(); move(573, 447); shot("06-sizer-drag-held"); release()
# icon drag inside the screen
move(37, 35); press(); move(200, 250); shot("07-icon-drag-held"); release()
# icon drag to the top edge (hard disk icon, now at the new place: grab it at 200,250)
move(220, 265); press(); move(220, 0); shot("08-icon-top-edge-held"); move(220, 265); release()
# icon drag to the bottom edge (trash)
move(402, 35); press(); move(402, 479); shot("09-icon-bottom-edge-held"); move(100, 400); release()
shot("10-end")
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, "mouse.txt"), "w") as f:
    for l in lines: f.write("%d 0x%02x 0x%02x 0x%02x\n" % l)
with open(os.path.join(out, "shots.txt"), "w") as f:
    for tk, n in shots: f.write("%d %s\n" % (tk, n))
print(len(lines), "packets,", len(shots), "shots, last tick", t)
