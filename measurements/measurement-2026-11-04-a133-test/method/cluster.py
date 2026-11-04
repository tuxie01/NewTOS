#!/usr/bin/env python3
# cluster.py <base.png> <head.png> [<out-crop-prefix>] -- exact RGB diff of two same-size screendumps:
# count, then clusters (8-connected after a 12 px dilation), each with bbox, pixel count and colour pairs
# (base -> head); with a prefix, writes <prefix>-<n>.png = base (top) / head (bottom) crop x4 of each cluster.
import sys
import numpy as np
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print("size differs", a.size, b.size); sys.exit(0)
A = np.asarray(a).astype(int); B = np.asarray(b).astype(int)
m = (A != B).any(axis=2)
print("%dx%d differing pixels: %d" % (a.size[0], a.size[1], m.sum()))
if not m.sum(): sys.exit(0)
h, w = m.shape; D = 12
lab = np.zeros((h, w), int); n = 0
ys, xs = np.nonzero(m)
pts = set(zip(ys.tolist(), xs.tolist()))
seen = {}
for p in sorted(pts):
    if p in seen: continue
    n += 1; st = [p]; seen[p] = n
    while st:
        y, x = st.pop()
        for yy in range(y - D, y + D + 1):
            for xx in range(x - D, x + D + 1):
                q = (yy, xx)
                if q in pts and q not in seen:
                    seen[q] = n; st.append(q)
cl = {}
for (y, x), k in seen.items(): cl.setdefault(k, []).append((x, y))
for k, pl in sorted(cl.items(), key=lambda t: min(p[1] for p in t[1])):
    x0 = min(p[0] for p in pl); x1 = max(p[0] for p in pl); y0 = min(p[1] for p in pl); y1 = max(p[1] for p in pl)
    cols = {}
    for x, y in pl:
        key = (tuple(A[y, x]), tuple(B[y, x])); cols[key] = cols.get(key, 0) + 1
    top = sorted(cols.items(), key=lambda t: -t[1])[:3]
    print("cluster %d: x=%d..%d y=%d..%d px=%d  %s" % (k, x0, x1, y0, y1, len(pl), top))
    if len(sys.argv) > 3:
        pad = 6; box = (max(0, x0 - pad), max(0, y0 - pad), min(w, x1 + pad + 1), min(h, y1 + pad + 1))
        ca = a.crop(box); cb = b.crop(box); s = 4 if (box[2]-box[0]) < 400 else 2
        ca = ca.resize((ca.width * s, ca.height * s), Image.NEAREST); cb = cb.resize((cb.width * s, cb.height * s), Image.NEAREST)
        im = Image.new("RGB", (ca.width, ca.height * 2 + 6), (255, 0, 0)); im.paste(ca, (0, 0)); im.paste(cb, (0, ca.height + 6))
        im.save("%s-%d.png" % (sys.argv[3], k))
