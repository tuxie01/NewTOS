#!/usr/bin/env python3
# dump.py <png> x0 y0 x1 y1 -- print the pixel rectangle as a character map
# ('#' = black, '.' = white, other grey/colour = letter by luminance class, listing legend)
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
x0, y0, x1, y1 = map(int, sys.argv[2:6])
cols = {}
sym = "#.o+-=*%@&ABCDEFGH"
print("     " + "".join(str((x // 10) % 10) if x % 10 == 0 else " " for x in range(x0, x1 + 1)))
print("     " + "".join(str(x % 10) for x in range(x0, x1 + 1)))
for y in range(y0, y1 + 1):
    row = ""
    for x in range(x0, x1 + 1):
        p = im.getpixel((x, y))
        if p == (0, 0, 0): c = "#"
        elif p == (255, 255, 255): c = "."
        else:
            if p not in cols: cols[p] = sym[2 + len(cols)] if 2 + len(cols) < len(sym) else "?"
            c = cols[p]
        row += c
    print("%4d %s" % (y, row))
print("legend: # black, . white;", {v: k for k, v in cols.items()})
