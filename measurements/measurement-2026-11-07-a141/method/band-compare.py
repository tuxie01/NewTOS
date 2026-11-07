#!/usr/bin/env python3
"""TASK/A141: are two 640x480 screenshots equal in the toolbar band?  band-compare.py <a.png> <b.png> <y0> <y1> [crop.png]

Screens are 16-bit on the EmuTOS side and 16-bit content in an XRGB store on NewTOS; both PNGs may expand 5/6 bits to 8
differently (Hatari, QEMU), and EmuTOS' truecolor words carry a zero as green's low bit (set_color16,
references/emutos/vdi/vdi_col.c:613, r<<11 | g<<6 | b) where a 256-colour screen has the full value, so pixels are
compared after truncation to 5 bits per channel, by value, not by file bytes.
With crop.png writes both bands x2, one above the other. Prints the differing pixel count and the first coordinates.
"""
import sys

from PIL import Image

a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
y0, y1 = int(sys.argv[3]), int(sys.argv[4])


def q(p):
    return (p[0] >> 3, p[1] >> 3, p[2] >> 3)


pa, pb = a.load(), b.load()
diff = []
for y in range(y0, y1):
    for x in range(640):
        if q(pa[x, y]) != q(pb[x, y]):
            diff.append((x, y, pa[x, y], pb[x, y]))
print("band y %d..%d: %d pixels, %d differ" % (y0, y1, 640 * (y1 - y0), len(diff)))
for d in diff[:12]:
    print("  x %d y %d: %s vs %s" % d)
if len(sys.argv) > 5:
    h = y1 - y0
    s = Image.new("RGB", (1280, 2 * (2 * h) + 8), (255, 255, 255))
    s.paste(a.crop((0, y0, 640, y1)).resize((1280, 2 * h), Image.NEAREST), (0, 0))
    s.paste(b.crop((0, y0, 640, y1)).resize((1280, 2 * h), Image.NEAREST), (0, 2 * h + 8))
    s.save(sys.argv[5])
