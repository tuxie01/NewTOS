#!/usr/bin/env python3 -I
"""gen-explore.py <mouse-out> <shots-out> -- geometry exploration input for slprobe.c (TASK/A130):
pin to (0,479), rest outside the window, one shot per configuration, quit."""
import sys
pk, shots = [], []
t = 0
x = y = 0
def b(v): return v & 0xFF
def pkt(tk, btn, dx, dy, note=""):
    pk.append(f"{tk} {btn:#04x} {b(dx):#04x} {b(dy):#04x}" + (f" # {note}" if note else ""))
def jump(nx, ny, btn=0xF8, note=""):
    global x, y, t
    dx, dy = nx - x, ny - y
    while dx or dy:
        sx = max(-127, min(127, dx)); sy = max(-127, min(127, dy))
        pkt(t, btn, sx, sy, note); note = ""
        dx -= sx; dy -= sy; t += 1
    x, y = nx, ny
def shot(name):
    global t
    shots.append((t - 1 + 180, name)); t += 239
def click(note):
    global t
    pkt(t, 0xFA, 0, 0, "PRESS " + note); pkt(t + 10, 0xF8, 0, 0, "RELEASE"); t += 11
for i in range(6):
    pkt(600 + i, 0xF8, -127, 127, "pin")
y = 479; t = 620
jump(560, 440, note="rest"); shot("x00-cfg0")
for c, cx in ((1, 200), (2, 290)):
    jump(cx, 200); click(f"cfg{c}"); jump(560, 440); shot(f"x0{c}-cfg{c}")
jump(400, 200); click("quit")
open(sys.argv[1], "w").write("# explore\n" + "\n".join(pk) + "\n")
open(sys.argv[2], "w").write("# explore\n" + "".join(f"{a} {n}\n" for a, n in shots))
