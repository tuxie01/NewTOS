#!/usr/bin/env python3 -I
"""gen-mgprobe-input.py <mouse-out> <shots-out> -- the input of tosdump/probes/mgprobe.c
(TASK/A127 D2/D3, GitHub #19): IKBD relative mouse packets `0xF8|buttons dx dy` (bit 1 =
left) at 200 Hz ticks from the input clock's start, and the screenshot schedule
`<ticks> <name>` (one shot per settled step, SETTLE ticks after the step's last packet).
Positions are tracked from the corner (0,479) the pointer is pinned to first.
Geometry (absolute pixels, mgprobe.c build_tree):
  bar y 0..18; titles Desk x 16..63, File x 64..111, Gaps x 112..159
  File box x 64..159 y 19..98 (border x 63 / 160, y 99): Item A y 19..34, Item B 35..50,
       separator (DISABLED) 51..66, Item C 67..82, Quit 83..98 -- contiguous, no gap
  Gaps box x 112..223 y 19..114 (border x 111 / 224, y 115): G1 19..34, G2 35..50,
       inner gap 51..58, separator (DISABLED) 59..74, G4 75..90, G5 91..106 x 112..207,
       right gap x 208..223 y 91..106, bottom gap y 107..114
"""
import sys

x = y = 0
t = 0
pk = []      # (tick, line)
shots = []   # (tick, name)
DWELL = 240  # ticks from a step's last packet to the next step
SETTLE = 180  # ticks from a step's last packet to its shot


def b(v):
    return v & 0xFF


def pkt(tk, btn, dx, dy, note=""):
    pk.append(f"{tk} {btn:#04x} {b(dx):#04x} {b(dy):#04x}" + (f" # {note}" if note else ""))


def jump(nx, ny, btn=0xF8, note=""):
    """Move to (nx, ny) with as few packets as possible, one per tick."""
    global x, y, t
    dx, dy = nx - x, ny - y
    first = True
    while dx or dy:
        sx = max(-127, min(127, dx))
        sy = max(-127, min(127, dy))
        pkt(t, btn, sx, sy, note if first else "")
        first = False
        dx -= sx
        dy -= sy
        t += 1
    x, y = nx, ny


def glide(nx, ny, step, every, note=""):
    """Fast move: at most `step` px per axis per packet, one packet every `every` ticks."""
    global x, y, t
    dx, dy = nx - x, ny - y
    first = True
    while dx or dy:
        sx = max(-step, min(step, dx))
        sy = max(-step, min(step, dy))
        pkt(t, 0xF8, sx, sy, note if first else "")
        first = False
        dx -= sx
        dy -= sy
        t += every
    t -= every - 1
    x, y = nx, ny


def shot(name):
    """The step is complete: shot SETTLE ticks after its last packet, next step after DWELL."""
    global t
    shots.append((t - 1 + SETTLE, name))
    t += DWELL - 1


def click(note):
    global t
    pkt(t, 0xFA, 0, 0, f"PRESS {note}")
    pkt(t + 10, 0xF8, 0, 0, "RELEASE")
    t += 11


# pin to the bottom-left corner (0,479): left and down only, so the pointer never
# crosses the menu bar on the way (run 1 pinned to (0,0) and crossed the File title,
# see RUNINFO.TXT)
for i in range(6):
    pkt(600 + i, 0xF8, -127, 127, "pin to (0,479)" if i == 0 else "")
y = 479
t = 620
jump(300, 300, note="rest")
shot("s00-rest")

# --- A: File box (contiguous items, DISABLED separator), slow single-packet steps
A = [("a01-file-title", 88, 8), ("a02-item-a", 100, 26), ("a03-separator", 100, 58),
     ("a04-item-c", 100, 74), ("a05-left-border", 63, 74), ("a06-item-c", 100, 74),
     ("a07-right-border", 160, 74), ("a08-item-c", 100, 74), ("a09-bottom-border", 100, 99),
     ("a10-item-b", 100, 42), ("a11-out-below", 100, 200), ("a12-back-item-b", 100, 42),
     ("a13-out-right", 300, 42), ("a14-back-item-a", 100, 26), ("a15-out-left", 30, 60),
     ("a16-back-separator", 100, 58), ("a17-item-c", 100, 74)]
# --- B: Gaps box (inner gap, right gap, bottom gap), slow single-packet steps
B = [("b01-gaps-title", 136, 8), ("b02-g1", 150, 27), ("b03-inner-gap", 150, 54),
     ("b04-g2", 150, 43), ("b05-inner-gap", 150, 54), ("b06-separator", 150, 66),
     ("b07-inner-gap", 150, 54), ("b08-g4", 150, 83), ("b09-right-gap", 215, 98),
     ("b10-g5", 150, 98), ("b11-bottom-gap", 150, 110), ("b12-g5", 150, 98),
     ("b13-right-border", 224, 83), ("b14-bottom-gap", 150, 110), ("b15-out-below", 150, 200),
     ("b16-back-inner-gap", 150, 54), ("b17-out-right", 300, 54), ("b18-back-g4", 150, 83),
     ("b19-out-left", 60, 83), ("b20-back-right-gap", 215, 98), ("b21-g1", 150, 27)]
for name, px, py in A + B:
    jump(px, py, note=name)
    shot(name)

# --- C: fast moves in the Gaps box
glide(150, 129, 6, 4, "c01 fast down G1 -> out below, 6px/20ms"); shot("c01-fast-down-out")
glide(150, 43, 6, 4, "c02 fast up -> G2"); shot("c02-fast-back-g2")
glide(150, 54, 6, 4, "c03 fast G2 -> inner gap -> out left")
glide(100, 54, 6, 4); shot("c03-fast-gap-out-left")
glide(150, 83, 6, 4, "c04 fast back -> G4"); shot("c04-fast-back-g4")
glide(150, 129, 6, 1, "c05 fast down G4 -> out, 6px/5ms"); shot("c05-fast5ms-down-out")
glide(150, 27, 6, 1, "c06 fast up -> G1, 6px/5ms"); shot("c06-fast5ms-back-g1")
jump(150, 54, note="c07 jump G1 -> inner gap")
jump(300, 54, note="c07 jump -> out right"); shot("c07-jump-gap-out")
jump(150, 43, note="c08 jump back -> G2"); shot("c08-jump-back-g2")
glide(260, 110, 20, 4, "c09 fast 20px/20ms G2 -> right gap -> out"); shot("c09-fast20-out")
glide(150, 83, 20, 4, "c10 fast 20px back -> G4"); shot("c10-fast20-back-g4")

# --- D: fast moves in the File box (through the separator)
jump(88, 8, note="d01 File title"); shot("d01-file-title")
jump(100, 26, note="d02 Item A"); shot("d02-item-a")
glide(100, 130, 6, 4, "d03 fast down Item A -> out below"); shot("d03-fast-down-out")
glide(100, 58, 6, 4, "d04 fast up -> separator"); shot("d04-fast-back-separator")
glide(100, 74, 6, 4, "d05 fast -> Item C"); shot("d05-fast-item-c")
glide(40, 74, 6, 4, "d06 fast out left"); shot("d06-fast-out-left")
glide(100, 26, 6, 4, "d07 fast back -> Item A"); shot("d07-fast-back-item-a")

# --- E: a press in the inner gap of the Gaps box
jump(136, 8, note="e01 Gaps title"); shot("e01-gaps-title")
jump(150, 54, note="e02 inner gap"); shot("e02-inner-gap")
click("in the inner gap"); shot("e03-after-gap-press")

# --- F: the end: File, Quit
jump(88, 8, note="f01 File title"); shot("f01-file-title")
jump(100, 90, note="f02 Quit"); shot("f02-quit")
click("on Quit")

hdr = ("# mgprobe-mouse.txt -- mouse input of tosdump/probes/mgprobe.c (TASK/A127 D2/D3); generated by\n"
       "# measurements/measurement-2026-11-05-a127-gap/method/gen-mgprobe-input.py.\n"
       "# <ticks> <0xF8|buttons> <dx> <dy>: one IKBD relative mouse packet per line (bit 1 = left).\n")
open(sys.argv[1], "w").write(hdr + "\n".join(pk) + "\n")
open(sys.argv[2], "w").write(
    "# mgprobe-shots.txt -- screenshot schedule of tosdump/probes/mgprobe.c (<ticks> <name>); generated\n"
    "# by measurements/measurement-2026-11-05-a127-gap/method/gen-mgprobe-input.py.\n"
    + "".join(f"{tk} {n}\n" for tk, n in shots))
print(f"{len(pk)} packets, {len(shots)} shots, last tick {t}", file=sys.stderr)
