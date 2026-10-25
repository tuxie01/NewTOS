#!/usr/bin/env python3
"""pixdiff.py <a.png> <b.png> -- number of differing pixels and their bounding box (PNG bytes are never equal: header)."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size: print("size differs", a.size, b.size); sys.exit(1)
pa, pb = a.load(), b.load(); n = 0; box = [10**9, 10**9, -1, -1]
for y in range(a.size[1]):
    for x in range(a.size[0]):
        if pa[x, y] != pb[x, y]:
            n += 1; box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
print(n, "px differ" + ("" if n == 0 else " bbox x%d..%d y%d..%d" % (box[0], box[2], box[1], box[3])))
