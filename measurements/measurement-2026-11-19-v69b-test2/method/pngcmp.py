#!/usr/bin/env python3
"""pngcmp.py A.png B.png -- pixel compare of two screendumps: size, number of differing pixels, bounding box."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size: print("SIZE DIFFERS", a.size, b.size); sys.exit(2)
pa, pb = a.load(), b.load(); w, h = a.size
d = [(x, y) for y in range(h) for x in range(w) if pa[x, y] != pb[x, y]]
if not d: print("IDENTICAL %dx%d" % (w, h)); sys.exit(0)
xs = [p[0] for p in d]; ys = [p[1] for p in d]
print("DIFFER %d of %d pixels, bbox x %d..%d y %d..%d" % (len(d), w * h, min(xs), max(xs), min(ys), max(ys))); sys.exit(1)
