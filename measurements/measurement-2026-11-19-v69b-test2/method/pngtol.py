#!/usr/bin/env python3
"""pngtol.py A.png B.png -- like pngcmp.py, but a pixel counts as different only when the top 5 bits of a gun differ
(the 16-plane device word keeps 5 bits per gun, the 32-plane word 8: V67's rule for AES pen colours). Also prints the
largest per-gun step among the pixels that differ at all."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size: print("SIZE DIFFERS", a.size, b.size); sys.exit(2)
pa, pb = a.load(), b.load(); w, h = a.size
d = []; mx = 0
for y in range(h):
    for x in range(w):
        p, q = pa[x, y], pb[x, y]
        if p != q:
            mx = max(mx, max(abs(p[g] - q[g]) for g in range(3)))
            if any((p[g] >> 3) != (q[g] >> 3) for g in range(3)): d.append((x, y))
if not d: print("EQUAL AT 5 BITS %dx%d (largest step %d)" % (w, h, mx)); sys.exit(0)
xs = [p[0] for p in d]; ys = [p[1] for p in d]
print("DIFFER AT 5 BITS %d pixels, bbox x %d..%d y %d..%d (largest step %d)" % (len(d), min(xs), max(xs), min(ys), max(ys), mx)); sys.exit(1)
