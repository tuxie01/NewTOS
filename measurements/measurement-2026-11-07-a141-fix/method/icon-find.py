#!/usr/bin/env python3
"""TASK/A141 fix run: find each 8-plane colour icon of a classic .rsc in a NewTOS screendump, pixel by pixel.

usage: icon-find.py <classic.rsc> <tos-default.pal> <screen.png> <y0> <y1>

Renders every CICONBLK's 8-plane col_data (plane p = bit p, MSB leftmost; tools/mkrsc.py read_cicon_table) with
register m -> pen REV_MAP_COL[m] (references/emutos/vdi/vdi_col.c:736-747) -> the palette's RGB, quantised as
NewTOS stores it (vdi2fivebits, (c*31+500)/1000, kernel/src/vdi_backend.rs). Compares only pixels under the
icon's 8-plane mask against the screen's top 5 bits per gun, at every position of the band y0..y1, and prints per icon the
best position (fewest differences on the cleaner parity, then overall) and its count of differing mask pixels (0 = the icon stands on the screen exactly as rendered), split
by screen checkerboard parity ((x + y) & 1, even/odd): a DISABLED icon is overdrawn on one parity only.
"""
import importlib.util
import struct
import sys

from PIL import Image

spec = importlib.util.spec_from_file_location("mkrsc", "/Volumes/Daten/Development/PiRusTOS/tools/mkrsc.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

RSC, PAL, PNG = sys.argv[1:4]
y0, y1 = int(sys.argv[4]), int(sys.argv[5])
d = open(RSC, "rb").read()
hdr = struct.unpack(">18H", d[:36])
blocks = m.read_cicon_table(d, hdr[0], hdr[17])
p = open(PAL, "rb").read()
pal = [struct.unpack(">3H", p[4 + 6 * i:10 + 6 * i]) for i in range(256)]
MAP = [0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13] + list(range(16, 256))
MAP[1], MAP[255] = 255, 15
rev = [0] * 256
for pen, reg in enumerate(MAP):
    rev[reg] = pen
five = lambda c: (c * 31 + 500) // 1000


def idx(off, planes, w, h):
    wd = w // 16
    n = wd * h
    words = struct.unpack(">%dH" % (n * planes), d[off:off + 2 * n * planes])
    out = []
    for y in range(h):
        for x in range(w):
            wi = y * wd + x // 16
            bit = 0x8000 >> (x & 15)
            out.append(sum(1 << q for q in range(planes) if words[q * n + wi] & bit))
    return out


scr = Image.open(PNG).convert("RGB")
sp = scr.load()
W = scr.size[0]
for k, (words, _, _, _, variants) in enumerate(blocks):
    v8 = [v for v in variants if v[0] == 8]
    if not v8:
        print("block %2d: no 8-plane variant" % k)
        continue
    w, h = words[5], words[6]
    data = idx(v8[0][1], 8, w, h)
    mask = idx(v8[0][2], 1, w, h)
    px = [(i % w, i // w, tuple(five(c) for c in pal[rev[data[i]]])) for i in range(w * h) if mask[i]]
    # Best position: fewest differing pixels on the cleaner parity, then fewest overall (a DISABLED icon is
    # overdrawn on one parity, so its total alone can prefer a wrong neighbour).
    best = None
    for oy in range(y0, y1 - h + 1):
        for ox in range(0, W - w + 1):
            par = [0, 0]
            for x, y, c in px:
                s = sp[ox + x, oy + y]
                if (s[0] >> 3, s[1] >> 3, s[2] >> 3) != c:
                    par[(ox + x + oy + y) & 1] += 1
            key = (min(par), sum(par))
            if best is None or key < best[0]:
                best = (key, ox, oy, par)
    par = best[3]
    best = (sum(par), best[1], best[2])
    print("block %2d: %dx%d, %4d mask pixels, best at x %3d y %2d: %4d differ (even %d, odd %d)"
          % (k, w, h, len(px), best[1], best[2], best[0], par[0], par[1]))
