#!/usr/bin/env python3
# crop.py <a.png> ax ay <b.png> bx by w h scale <out.png> -- the w x h rectangle at
# (ax,ay) of a above the one at (bx,by) of b, each scaled by `scale` (nearest
# neighbour), a red bar between them; for looking at the two side by side.
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); ax, ay = int(sys.argv[2]), int(sys.argv[3])
b = Image.open(sys.argv[4]).convert("RGB"); bx, by = int(sys.argv[5]), int(sys.argv[6])
w, h, s = int(sys.argv[7]), int(sys.argv[8]), int(sys.argv[9])
ca = a.crop((ax, ay, ax + w, ay + h)).resize((w * s, h * s), Image.NEAREST)
cb = b.crop((bx, by, bx + w, by + h)).resize((w * s, h * s), Image.NEAREST)
out = Image.new("RGB", (w * s, 2 * h * s + 2 * s), (255, 0, 0))
out.paste(ca, (0, 0)); out.paste(cb, (0, h * s + 2 * s))
out.save(sys.argv[10])
