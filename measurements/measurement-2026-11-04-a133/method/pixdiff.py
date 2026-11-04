#!/usr/bin/env python3
# pixdiff.py <a.png> <b.png> -- exact RGB comparison of two screendumps of the
# same size: number of differing pixels, their bounding box and the first few.
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print("size differs: %s vs %s" % (a.size, b.size)); sys.exit(0)
w, h = a.size; pa = a.load(); pb = b.load()
d = [(x, y) for y in range(h) for x in range(w) if pa[x, y] != pb[x, y]]
print("%dx%d, differing pixels: %d" % (w, h, len(d)))
if d:
    xs = [p[0] for p in d]; ys = [p[1] for p in d]
    print("bbox x=%d..%d y=%d..%d" % (min(xs), max(xs), min(ys), max(ys)))
    print("first:", [(x, y, pa[x, y], pb[x, y]) for x, y in d[:12]])
