#!/usr/bin/env python3
"""deskcmp.py <emutos-desk-shot> <newtos-desk-shot> <label> [--tt] -- A131: the desktop's two drive
windows (EMUDESK/NEWDESK.INF #W 00 00 02 02 26 0E 00 and #W 00 00 2A 06 24 12 00, 8x16 cells:
W1 border 16,32,304,224, W2 border 336,96,288,288 -- the last #W window is topped).
Compared by VDI pen (the side's own measured palette, as pencmp.py; Falcon PNG 4 extra right
columns dropped) in three regions per window, the window CONTENT (icons, info text, title text)
being different by design (different C: contents, desk language):
  RING   the outer border and the drop shadow: pixels within 3 px inside the border rectangle's
         edge plus the 2 px shadow bands right and below (x+w..x+w+1, y+h..y+h+1)
  VBAR   the vertical scroll bar column (x+w-22 .. x+w-1, y+44 .. y+h-1) incl. arrows, track, elevator
  HBAR   the horizontal scroll bar band (x .. x+w-1, y+h-22 .. y+h-1)
The mouse pointer box x 318..334, y 236..262 (both sides park it at the screen centre) is excluded.
"""
import sys
from collections import Counter

from PIL import Image

pe, pn, label = sys.argv[1:4]
TT = "--tt" in sys.argv
PT = {(255, 255, 255): 0, (0, 0, 0): 1, (255, 0, 0): 2, (0, 255, 0): 3, (170, 170, 170): 8, (102, 102, 102): 9}
PF = {(255, 251, 255): 0, (0, 0, 0): 1, (255, 0, 0): 2, (0, 251, 0): 3, (189, 186, 189): 8, (140, 138, 140): 9}
PN = {(255, 255, 255): 0, (0, 0, 0): 1, (255, 0, 0): 2, (0, 255, 0): 3, (189, 189, 189): 8, (140, 140, 140): 9}
ie = Image.open(pe).convert("RGB")
if ie.size[0] == 644:
    ie = ie.crop((0, 0, 640, 480))
a = ie.load()
b = Image.open(pn).convert("RGB").load()
PE = PT if TT else PF
WINS = {"W1": (16, 32, 304, 224), "W2": (336, 96, 288, 288)}


def pen(m, c):
    return m.get(c, "?%d,%d,%d" % c)


def ptr(x, y):
    return 318 <= x <= 334 and 236 <= y <= 262


for wn, (x, y, w, h) in WINS.items():
    regs = {
        "RING": [(i, j) for j in range(y, y + h + 2) for i in range(x, x + w + 2)
                 if (i >= x + w or j >= y + h or i < x + 3 or j < y + 3 or i >= x + w - 3 or j >= y + h - 3)],
        "VBAR": [(i, j) for j in range(y + 44, y + h) for i in range(x + w - 22, x + w)],
        "HBAR": [(i, j) for j in range(y + h - 22, y + h) for i in range(x, x + w)],
    }
    for rn, pts in regs.items():
        pairs = Counter()
        n = 0
        for (i, j) in pts:
            if ptr(i, j):
                continue
            n += 1
            e, nn = pen(PE, a[i, j]), pen(PN, b[i, j])
            if e != nn:
                pairs[(e, nn)] += 1
        tot = sum(pairs.values())
        print(f"{label} {wn} {rn}: {n} px compared, {tot} differ by pen" +
              ("; E->N " + ", ".join(f"{e}->{q} x{c}" for (e, q), c in pairs.most_common(5)) if tot else ""))
