#!/usr/bin/env python3
"""pixcmp.py <emutos-shots> <newtos-shots> <out-dir> [--falcon] -- A131: pixel-exact comparison of the
shprobe / desk screenshots, shot by shot (same file names on both sides).

Colour model: every pixel is reduced to the 5-bit-per-gun VDI word space (top 5 bits of each 8-bit
gun) before comparing -- NewTOS's framebuffer holds expand5to8(word) per gun (kernel/src/vdi_backend.rs
word_to_device), Hatari's Falcon screenshot expands the 16-bit RGB565 word with 6-bit green (pen 0
= 0xFFDF -> 255,251,255), Hatari's TT screenshot is a PNG palette of 4-bit guns x 17. In the 5-bit
space Falcon and NewTOS represent the same VDI word identically; TT values are reported raw too.
--falcon: the Hatari Falcon PNG is 644 px wide; the 4 extra columns are located and dropped.
Output per shot: differing pixels, their bounding box, a histogram of (EmuTOS RGB, NewTOS RGB)
pairs, and the pixel rows/columns across the frame and shadow edges listed in EDGES.
"""
import os
import sys
from collections import Counter

from PIL import Image

E, N, OUT = sys.argv[1:4]
FALCON = "--falcon" in sys.argv
os.makedirs(OUT, exist_ok=True)


def load(p, falcon=False):
    im = Image.open(p).convert("RGB")
    if im.size[0] == 644 and falcon:
        # find the 4 constant extra columns: try offsets 0 and 4
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


def q(c):
    return (c[0] >> 3, c[1] >> 3, c[2] >> 3)


summary = []
for name in sorted(os.listdir(E)):
    if not name.endswith(".png"):
        continue
    pe = os.path.join(E, name)
    pn = os.path.join(N, name)
    if not os.path.exists(pn):
        summary.append(f"{name}: missing on NewTOS")
        continue
    ie = load(pe, FALCON)
    inn = load(pn)
    a = ie.load()
    b = inn.load()
    w, h = inn.size
    diff = []
    pairs = Counter()
    for y in range(h):
        for x in range(w):
            if q(a[x, y]) != q(b[x, y]):
                diff.append((x, y))
                pairs[(a[x, y], b[x, y])] += 1
    line = f"{name}: {len(diff)} px differ"
    if diff:
        xs = [p[0] for p in diff]
        ys = [p[1] for p in diff]
        line += f", bbox x {min(xs)}..{max(xs)} y {min(ys)}..{max(ys)}; pairs E->N " + \
            ", ".join(f"{e}->{n} x{c}" for (e, n), c in pairs.most_common(6))
    summary.append(line)
    with open(os.path.join(OUT, name[:-4] + ".txt"), "w") as f:
        f.write(line + "\n")
        for (x, y) in diff:
            f.write(f"{x},{y} E={a[x, y]} N={b[x, y]}\n")
with open(os.path.join(OUT, "summary.txt"), "w") as f:
    f.write("\n".join(summary) + "\n")
print("\n".join(summary))
