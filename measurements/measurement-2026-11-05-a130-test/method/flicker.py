#!/usr/bin/env python3
"""flicker.py <shots-dir> -- the flicker metric of the slprobe flicker run (TASK/A130).

For every sample: the number of black pixels (luma < 128) inside the elevator's INTERIOR (a rect
inside the elevator, clear of its border, its 3D shadow and of EmuTOS's XOR drag outline). The
pointer is off the bar during the holds (gen-flicker-input.py), so a white elevator gives 0; the
track's dither pattern or a half-drawn elevator gives > 0. Elevator rects from the oracle's
exploration (RUNINFO.TXT): size 1000 V y 106..333 / H x 104..431, size 300 V y 186..253,
size -1 V y 211..228; vertical bar x 458..478, horizontal bar y 358..378.
Prints one line per sample and a summary per hold: samples, samples with black > 0, max."""
import os
import sys

from PIL import Image

D = sys.argv[1]
RECT = {"idle1000": (461, 110, 474, 328), "h1-1000-V": (461, 110, 474, 328),
        "h2-1000-H": (107, 361, 428, 375),
        "idle300": (461, 190, 474, 248), "h3-300-V": (461, 190, 474, 248),
        "idlemin": (461, 214, 474, 224), "h4-min-V": (461, 214, 474, 224)}
summ = {}
for f in sorted(os.listdir(D)):
    if not f.endswith(".png"):
        continue
    name = f[:-4].split("-", 1)[1]          # drop the NN- prefix of run-slprobe.sh
    tag = name.rsplit("-", 1)[0]
    x0, y0, x1, y1 = RECT[tag]
    im = Image.open(os.path.join(D, f)).convert("L").load()
    blk = sum(1 for yy in range(y0, y1 + 1) for xx in range(x0, x1 + 1) if im[xx, yy] < 128)
    print(f"{name:22s} black {blk:5d}")
    s = summ.setdefault(tag, [0, 0, 0])
    s[0] += 1
    s[1] += blk > 0
    s[2] = max(s[2], blk)
print("hold        samples  with-black  max-black")
for tag, (n, k, m) in summ.items():
    print(f"{tag:12s} {n:7d} {k:11d} {m:10d}")
