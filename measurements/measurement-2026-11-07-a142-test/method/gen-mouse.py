#!/usr/bin/env python3
r"""gen-mouse.py <out> <cmd> [<cmd> ...] -- IKBD injector script for TASK/A140 (input of tools/inject-arm.py).

Commands, executed in order, each with an absolute screen coordinate (640x480, QEMU raspi4b):
  corner              six packets to the bottom-left corner (0,479); never across the menu bar
  wait:<ticks>        advance the clock by <ticks> (200 Hz)
  at:<ticks>          set the clock to <ticks> after the arm (must not go back)
  move:<x>,<y>        relative packets to (x,y): first along x at the current y, then along y
  click:<x>,<y>       move, settle, press, release
  dclick:<x>,<y>      move, settle, double click
  keys:<hex>,<hex>..  IKBD make/break bytes, one every 10 ticks
The script starts at the corner, so moves never enter the menu bar except at their target column."""
import sys

out, cmds = sys.argv[1], sys.argv[2:]
L = []
t = 100
x, y = 0, 479


def pk(b, dx, dy):
    global t
    L.append(f"{t} 0x{b:02x} 0x{dx & 255:02x} 0x{dy & 255:02x}")
    t += 3


def move(nx, ny):
    global x, y
    while x != nx:
        d = max(-30, min(30, nx - x))
        pk(0xf8, d, 0)
        x += d
    while y != ny:
        d = max(-30, min(30, ny - y))
        pk(0xf8, 0, d)
        y += d


def xy(a):
    a, b = a.split(",")
    return int(a), int(b)


for c in cmds:
    op, _, arg = c.partition(":")
    if op == "corner":
        for _ in range(6):
            pk(0xf8, -127, 127)
        x, y = 0, 479
    elif op == "wait":
        t += int(arg)
    elif op == "at":
        assert int(arg) >= t, f"{c}: clock already at {t}"
        t = int(arg)
    elif op == "move":
        move(*xy(arg))
    elif op == "click":
        move(*xy(arg))
        t += 40
        pk(0xfa, 0, 0)
        t += 15
        pk(0xf8, 0, 0)
    elif op == "dclick":
        move(*xy(arg))
        t += 40
        pk(0xfa, 0, 0)
        t += 6
        pk(0xf8, 0, 0)
        t += 6
        pk(0xfa, 0, 0)
        t += 6
        pk(0xf8, 0, 0)
    elif op == "keys":
        for b in arg.split(","):
            L.append(f"{t} 0x{b}")
            t += 10
    else:
        sys.exit(f"unknown command {c}")
    print(f"# {c} -> clock {t}", file=sys.stderr)
open(out, "w").write("\n".join(L) + "\n")
