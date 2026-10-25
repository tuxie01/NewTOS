#!/usr/bin/env python3
"""coldiff.py <a.png> <b.png> -- differing pixels between two screendumps, grouped by column: x -> (y_min..y_max, count)."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
pa, pb = a.load(), b.load(); cols = {}
for y in range(a.size[1]):
    for x in range(a.size[0]):
        if pa[x, y] != pb[x, y]:
            c = cols.setdefault(x, [y, y, 0]); c[1] = y; c[2] += 1
n = sum(c[2] for c in cols.values())
print("%d px differ" % n, " ".join("x%d:y%d..%d(%d)" % (x, c[0], c[1], c[2]) for x, c in sorted(cols.items())))
