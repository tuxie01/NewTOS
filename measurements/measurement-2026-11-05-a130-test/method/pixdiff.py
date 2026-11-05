#!/usr/bin/env python3
"""pixdiff.py <emutos-shots> <newtos-shots> -- for every shot with class differences outside the baseline
mask (compare.py's nearest-of-4 classes), the raw RGB value histogram of the differing pixels on each side
and their count/bbox. Also prints, per shot, how many differing pixels lie in rows/cols of the elevator
interior (bbox of the diff). Baseline mask as compare.py."""
import collections, os, sys
from PIL import Image
PAL = {"W": (255,255,255), "B": (0,0,0), "g": (102,102,102), "G": (0,255,0)}
def cls(c): return min(PAL, key=lambda n: sum((a-b)**2 for a,b in zip(c, PAL[n])))
E, N = sys.argv[1:3]
names = sorted(f for f in os.listdir(E) if f.endswith(".png"))
def load(p): return Image.open(p).convert("RGB")
base = [f for f in names if "-start-rest" in f][0]
a0, b0 = load(os.path.join(E, base)), load(os.path.join(N, base))
mask = {(x,y) for y in range(480) for x in range(640) if cls(a0.getpixel((x,y))) != cls(b0.getpixel((x,y)))}
for f in names:
    a, b = load(os.path.join(E,f)), load(os.path.join(N,f))
    ha, hb, n, bb = collections.Counter(), collections.Counter(), 0, None
    pa, pb = a.load(), b.load()
    for y in range(480):
        for x in range(640):
            if (x,y) in mask: continue
            if cls(pa[x,y]) != cls(pb[x,y]):
                n += 1; ha[pa[x,y]] += 1; hb[pb[x,y]] += 1
                bb = (min(bb[0],x),min(bb[1],y),max(bb[2],x),max(bb[3],y)) if bb else (x,y,x,y)
    if n:
        print(f"{f[:-4]:44s} n={n:5d} bbox={bb} E={dict(ha)} N={dict(hb)}")
