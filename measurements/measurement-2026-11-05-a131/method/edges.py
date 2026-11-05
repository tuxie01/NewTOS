#!/usr/bin/env python3
"""edges.py <shot-name> <tt-dir> <falcontc-dir> <newtos-dir> -- A131: the raw RGB of each side on
the pixel runs across a window's right and bottom edge (frame, shadow, desktop), for the report.
W1 = border 20,40,300,250; W2o = W2 in its overlapping position 200,150,300,240 (states 3-5).
Falcon PNG: 644 px wide, the 4 extra columns are at the right (pixcmp.py locates them), so offset 0."""
import sys
from PIL import Image

name, T, F, N = sys.argv[1:5]
ims = {}
for k, d, off in (("E-tt", T, 0), ("E-falcontc", F, 0), ("N", N, 0)):
    im = Image.open(f"{d}/{name}").convert("RGB")
    if im.size[0] == 644:
        im = im.crop((off, 0, off + 640, 480))
    ims[k] = im.load()
runs = [
    ("W1 right edge, row y=150, x 314..324", [(x, 150) for x in range(314, 325)]),
    ("W1 bottom edge, column x=150, y 284..294", [(150, y) for y in range(284, 295)]),
    ("W1 shadow corner, row y=291, x 18..24", [(x, 291) for x in range(18, 25)]),
    ("W1 shadow top end, column x=321, y 38..44", [(321, y) for y in range(38, 45)]),
    ("W2o right edge, row y=300, x 494..504", [(x, 300) for x in range(494, 505)]),
    ("W2o bottom edge, column x=300, y 384..394", [(300, y) for y in range(384, 395)]),
]
for title, pts in runs:
    print(title)
    for k, px in ims.items():
        print(f"  {k:11s} " + " ".join("%d,%d,%d" % px[p] for p in pts))
