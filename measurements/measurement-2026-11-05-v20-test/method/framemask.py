#!/usr/bin/env python3
"""framemask.py <rest.png> <held.png> x0 y0 x1 y1 [--falcon] -- ASCII map of the pixels that changed between the rest shot and the
held shot (the XOR frame, the selected icon, the pointer) inside the rectangle; '#' changed."""
import sys
from PIL import Image
def load(p, falcon):
    im = Image.open(p).convert("RGB")
    if falcon and im.size[0] == 644:
        w, h = im.size
        px = im.load()
        best = None
        for off in (0, 4, 2):
            cols = [x for x in range(w) if x < off or x >= off + 640]
            if all(px[x, y] == px[cols[0], 0] for x in cols for y in range(0, h, 7)):
                best = off
                break
        im = im.crop((best, 0, best + 640, h))
    return im
rest, held = sys.argv[1], sys.argv[2]
x0, y0, x1, y1 = map(int, sys.argv[3:7])
fal = "--falcon" in sys.argv
a = load(rest, fal).load(); b = load(held, fal).load()
for y in range(y0, y1):
    print("%3d " % y + "".join("#" if a[x, y] != b[x, y] else "." for x in range(x0, x1)))
