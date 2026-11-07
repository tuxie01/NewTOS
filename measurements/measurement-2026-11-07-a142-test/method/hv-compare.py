#!/usr/bin/env python3
"""hv-compare.py <runs-dir> -- A142-test, own script. Per .HYP and page: pixels differing (5 bits per channel) between a NewTOS HypView screen (base | fix) and the
oracle's Falcon-truecolor screen, in the window content area x 0..620, y 70..460 (below the toolbar, left of the scroll bar). The oracle screenshot is 644x516
(Hatari border); the offset that aligns it is searched on page 0 of the file (best of dx,dy in -6..6) and then fixed for all pages."""
import sys, glob, os
from PIL import Image
from PIL import ImageChops
R = sys.argv[1]
def load(p): return Image.open(p).convert("RGB").point(lambda v: v & 0xF8)
def crop(a, dx=0, dy=0, x0=0, y0=70, x1=620, y1=460):
    return a.crop((x0+dx, y0+dy, x1+dx, y1+dy))
def ndiff(a, b):
    d = ImageChops.difference(a, b)
    m = ImageChops.lighter(ImageChops.lighter(d.getchannel(0), d.getchannel(1)), d.getchannel(2))
    h = m.histogram(); return sum(h[1:])
tot = {}
for f in ("QED", "TERADESK", "ZVIEW"):
    o = sorted(glob.glob(f"{R}/oracle-hv-{f}/shots/*.png"))
    b = sorted(glob.glob(f"{R}/hv-{f}-base/shots/*.png")); a = sorted(glob.glob(f"{R}/hv-{f}-fix/shots/*.png"))
    best = None
    for dx in range(-6, 7):
        for dy in range(-6, 7):
            d = ndiff(crop(load(o[0]), dx, dy), crop(load(a[0])))
            if best is None or d < best[0]: best = (d, dx, dy)
    print(f"== {f}: pages {len(o)}/{len(b)}/{len(a)}; best oracle offset (dx,dy)=({best[1]},{best[2]}) page-0 diff {best[0]}")
    for i in range(min(len(o), len(a))):
        oc = crop(load(o[i]), best[1], best[2])
        db = ndiff(oc, crop(load(b[i]))); da = ndiff(oc, crop(load(a[i])))
        flag = "  <-- worse after" if da > db else ("  <-- improved" if da < db else "")
        print(f"  page {i:2d}: before {db:6d}  after {da:6d}{flag}")
