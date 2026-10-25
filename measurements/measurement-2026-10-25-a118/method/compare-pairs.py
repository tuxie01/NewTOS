#!/usr/bin/env python3
"""compare-pairs.py DIR -- A118: pixel diff of every base/new (head/wt) screendump pair in DIR that series.sh's compare.py does
not cover: <base>-X.png vs <new>-X.png for base/new and head-onR1/wt-onR1, and api-base-M / api-new-M, fmerr-base / fmerr-new."""
import sys, glob, os
from PIL import Image
d = sys.argv[1]
def diff(a, b):
    A = Image.open(a).convert("RGB"); B = Image.open(b).convert("RGB")
    if A.size != B.size: return "size differs"
    pa, pb = A.load(), B.load(); n = 0; box = [10**9, 10**9, -1, -1]
    for y in range(A.size[1]):
        for x in range(A.size[0]):
            if pa[x, y] != pb[x, y]:
                n += 1; box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
    return "%6d px%s" % (n, "" if n == 0 else "  bbox x%d..%d y%d..%d" % (box[0], box[2], box[1], box[3]))
pairs = []
for b in sorted(glob.glob(d + "/base-*.png")): pairs.append((b, b.replace("/base-", "/new-")))
for b in sorted(glob.glob(d + "/head-onR1-*.png")): pairs.append((b, b.replace("/head-onR1-", "/wt-onR1-")))
for b in sorted(glob.glob(d + "/api-base-*.png")): pairs.append((b, b.replace("/api-base-", "/api-new-")))
pairs.append((d + "/fmerr-base.png", d + "/fmerr-new.png"))
for a, b in pairs:
    name = os.path.basename(a)
    print("%-40s %s" % (name, diff(a, b) if os.path.exists(b) else "MISSING " + os.path.basename(b)))
