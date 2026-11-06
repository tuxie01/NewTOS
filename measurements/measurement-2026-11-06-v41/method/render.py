#!/usr/bin/env python3
"""render.py <out.ppm> <scale> <case> <run>...: one case of several runs, stacked; word 0xf800 etc. mapped from 5-6-5."""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from rawmap import load
out, sc, case = sys.argv[1], int(sys.argv[2]), sys.argv[3]
runs = [load(r)[case] for r in sys.argv[4:]]
_, x, y, w, h, _ = runs[0]
gap = 6
H = (h*sc+gap)*len(runs)
with open(out, "wb") as f:
    f.write(b"P6 %d %d 255\n" % (w*sc, H))
    for r in runs:
        ws = r[5]
        for j in range(h):
            row = bytearray()
            for i in range(w):
                v = ws[j*w+i]
                row += bytes(((v>>11)<<3, ((v>>5)&63)<<2, (v&31)<<3))*sc
            for _ in range(sc): f.write(row)
        for _ in range(gap): f.write(b"\x40\x40\xc0"*(w*sc))
