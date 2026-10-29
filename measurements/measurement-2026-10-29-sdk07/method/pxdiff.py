#!/usr/bin/env python3
"""pxdiff.py A.png B.png -- count differing pixels between two screendumps and print their bounding box (SDK07)."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
assert a.size == b.size
w, h = a.size; pa = a.load(); pb = b.load(); n = 0; box = [w, h, -1, -1]
for y in range(h):
    for x in range(w):
        if pa[x, y] != pb[x, y]:
            n += 1; box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
print("%s vs %s: %d pixels differ%s" % (sys.argv[1], sys.argv[2], n, "" if n == 0 else ", bbox x %d-%d y %d-%d" % (box[0], box[2], box[1], box[3])))
