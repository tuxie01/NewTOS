#!/usr/bin/env python3
"""dumpcrop.py <png> x0 y0 x1 y1 -- ASCII map of a crop: '#' black, '.' white, 'g' grey (192..), other -> first letter of hex."""
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB"); x0, y0, x1, y1 = map(int, sys.argv[2:6]); p = im.load()
cols = {}
for y in range(y0, y1 + 1):
    row = ""
    for x in range(x0, x1 + 1):
        c = p[x, y]
        if c == (0, 0, 0): ch = "#"
        elif min(c) >= 240: ch = "."
        else:
            cols.setdefault(c, "abcdefghijklmnopqrstuvwxyz"[len(cols) % 26]); ch = cols[c]
        row += ch
    print("%3d %s" % (y, row))
print("x0=%d" % x0, {v: k for k, v in cols.items()})
