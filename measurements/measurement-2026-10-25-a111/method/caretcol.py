#!/usr/bin/env python3
"""caretcol.py <png> <x0> <x1> <yscan> <y0> <y1> -- the caret column(s): every x in [x0,x1] whose pixel in row
<yscan> (a row of the field with no text in it) differs from the field face (the colour at (x0, yscan)); for each,
the vertical colour runs over rows y0..y1 as "colour x count"."""
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB"); p = im.load(); x0, x1, ys, y0, y1 = map(int, sys.argv[2:7])
face = p[x0, ys]; out = []
for x in range(x0, x1 + 1):
    if p[x, ys] != face:
        runs = []
        for y in range(y0, y1 + 1):
            c = p[x, y]
            if runs and runs[-1][0] == c: runs[-1][1] += 1
            else: runs.append([c, 1])
        out.append("x=%d [%s]" % (x, " ".join("%02x%02x%02x*%d" % (c[0], c[1], c[2], n) for c, n in runs)))
print("face %02x%02x%02x; " % face + ("; ".join(out) if out else "no caret in scan row"))
