#!/usr/bin/env python3
"""gen-mouse.py <mouse-out> <keys...> : IKBD script for the cross-drive copy/move (fs31 test). Corner reset (bottom-left, the menu bar must not be crossed), then relative moves.
ICON (104,103) = COPYME.TXT of the C: window, DROP (400,180) = empty area of the D: window (640x480 screen)."""
import sys
out = sys.argv[1]; extra = sys.argv[2:]   # extra: "t:bytes" lines appended verbatim, e.g. "700:0x1c" (key make) 
L = []
t = 100
def pk(b, dx, dy):
    global t
    L.append(f"{t} 0x{b:02x} 0x{dx & 255:02x} 0x{dy & 255:02x}"); t += 3
for _ in range(6): pk(0xf8, -127, 127)          # to the bottom-left corner (0,479): never across the menu bar
t = 200
pk(0xf8, 104, 0)
for dy in (-127, -127, -119): pk(0xf8, 0, dy)    # up to y=103 (479-373)
t = 230
for _ in range(3): pk(0xfa, 0, 0)               # first click: activates the C: window (an inactive window only comes to the top)
t = 260
for _ in range(3): pk(0xf8, 0, 0)
t = 330
for _ in range(4): pk(0xfa, 0, 0)               # press and hold, settle (the first packet after a press starts the drag only after the next one)
t = 450
for dx, dy in [(40, 10), (40, 10), (40, 10), (40, 10), (40, 10), (40, 10), (40, 10), (16, 5)]:
    pk(0xfa, dx, dy)                            # drag to (104+296, 106+75)
t = 560
for _ in range(3): pk(0xfa, 0, 0)
t = 640; pk(0xf8, 0, 0)                          # release = drop
for e in extra:
    L.append(e.replace(':', ' ', 1))
L.sort(key=lambda l: int(l.split()[0]))   # the injector feeds in file order
open(out, 'w').write("\n".join(L) + "\n")
