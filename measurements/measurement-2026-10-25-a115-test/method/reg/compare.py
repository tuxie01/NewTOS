#!/usr/bin/env python3
# (A115: copied from measurements/measurement-2026-10-21-gh1-reg/method/compare.py; outputs one level up from method/reg/)
"""compare.py DIR MODE... -- pixel diff of every <head>-MODE-*.png against <wt>-MODE-*.png in DIR; prints name, differing pixels, bbox."""
import sys, glob, os
from PIL import Image
d = sys.argv[1]
for mode in sys.argv[2:]:
    tot = 0; same = 0; diff = 0; miss = 0
    for h in sorted(glob.glob("%s/head-%s-*.png" % (d, mode))):
        w = h.replace("/head-", "/wt-")
        name = os.path.basename(h)[len("head-"):]
        if not os.path.exists(w): print("%-45s MISSING wt" % name); miss += 1; continue
        a = Image.open(h).convert("RGB"); b = Image.open(w).convert("RGB")
        if a.size != b.size: print("%-45s size differs" % name); diff += 1; continue
        pa, pb = a.load(), b.load(); n = 0; box = [10**9, 10**9, -1, -1]
        for y in range(a.size[1]):
            for x in range(a.size[0]):
                if pa[x, y] != pb[x, y]:
                    n += 1; box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
        tot += 1
        if n == 0: same += 1
        else: diff += 1
        print("%-45s %6d px%s" % (name, n, "" if n == 0 else "  bbox x%d..%d y%d..%d" % (box[0], box[2], box[1], box[3])))
    print("== mode %s: %d compared, %d identical, %d differ, %d missing" % (mode, tot, same, diff, miss))
