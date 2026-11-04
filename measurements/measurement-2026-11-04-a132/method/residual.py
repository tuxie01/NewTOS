#!/usr/bin/env python3
# residual.py <newtos.png> nx ny <hatari.png> hx hy w h -- every pixel of the w x h
# region that differs between the two (classes as A129 cmp.py: B <64, W >=240, grey
# between), grouped by (NewTOS class, EmuTOS class), with its bounding box, plus
# the check that EmuTOS's black pixels there (the glyphs) are black in NewTOS too.
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("L"); ax, ay = int(sys.argv[2]), int(sys.argv[3])
b = Image.open(sys.argv[4]).convert("L"); bx, by = int(sys.argv[5]), int(sys.argv[6])
w, h = int(sys.argv[7]), int(sys.argv[8])
def cl(v): return "B" if v < 64 else ("W" if v >= 240 else ("1" if v >= 170 else "2"))
groups = {}
for y in range(h):
    for x in range(w):
        ca = cl(a.getpixel((ax + x, ay + y))); cb = cl(b.getpixel((bx + x, by + y)))
        if ca != cb:
            groups.setdefault((ca, cb), []).append((x, y))
total = sum(len(v) for v in groups.values())
print("region %dx%d, differing pixels: %d" % (w, h, total))
for (ca, cb), pts in sorted(groups.items()):
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    par = sorted({(x + y) % 2 for x, y in pts})
    print("  NewTOS %s / EmuTOS %s: %d px, bbox x=%d..%d y=%d..%d, (x+y)%%2 in %s"
          % (ca, cb, len(pts), min(xs), max(xs), min(ys), max(ys), par))
glyph = [(x, y) for y in range(h) for x in range(w)
         if cl(b.getpixel((bx + x, by + y))) == "B" and 88 <= x <= 111 and 3 <= y <= 18]
miss = [p for p in glyph if cl(a.getpixel((ax + p[0], ay + p[1]))) != "B"]
print("EmuTOS black pixels in the text cells x=88..111 y=3..18: %d, not black in NewTOS: %d"
      % (len(glyph), len(miss)))
