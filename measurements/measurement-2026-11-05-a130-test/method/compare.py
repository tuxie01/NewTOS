#!/usr/bin/env python3
"""compare.py <emutos-shots> <newtos-shots> <out-dir> -- pixel comparison of the slprobe screenshots
(TASK/A130), shot by shot (same file names on both sides).

EmuTOS TT medium (PNG palette) and NewTOS (RGB565 via QMP) encode colours differently, so every
pixel is mapped to the nearest of the four colours the oracle frame uses -- W white, B black,
g grey (102,102,102: the 3D shadow), G green (the desktop pattern) -- before comparing.
Regions (absolute pixels, slprobe.c window border 80,60,400,320):
  VBAR  the vertical scroll bar column incl. its border   x 456..480, y  80..358
  HBAR  the horizontal scroll bar band incl. its border   x  78..458, y 356..381
  WIN   the window incl. the shadow                       x  78..482, y  58..382
  ALL   the whole screen
BASELINE: the pixels whose class differs already in the first shot (the window at rest, before any
input: the drop shadow and outer border light grey (189) on NewTOS where EmuTOS draws black, x 480-481
and y 380-381, and 85 px in the top row y 4..15, x 343..362, outside the window) form a mask; the
counts exclude it (the scroll bars are not in it), the diff images paint it blue.
Per shot: differing pixels per region, the bounding box of the WIN differences, and for every
differing shot a 3x crop <out>/diff-<shot>.png (EmuTOS | NewTOS | difference in red) of the bar
that differs most (or the window).
"""
import os
import sys

from PIL import Image

E, N, OUT = sys.argv[1:4]
PAL = {"W": (255, 255, 255), "B": (0, 0, 0), "g": (102, 102, 102), "G": (0, 255, 0)}
REG = {"VBAR": (456, 80, 480, 358), "HBAR": (78, 356, 458, 381), "WIN": (78, 58, 482, 382)}


def cls(path):
    im = Image.open(path).convert("RGB")
    assert im.size == (640, 480), (path, im.size)
    cache = {}
    out = []
    for c in im.get_flattened_data():
        k = cache.get(c)
        if k is None:
            k = min(PAL, key=lambda n: sum((a - b) ** 2 for a, b in zip(c, PAL[n])))
            cache[c] = k
        out.append(k)
    return out, im


names = sorted(f for f in os.listdir(E) if f.endswith(".png"))
nn = sorted(f for f in os.listdir(N) if f.endswith(".png"))
assert names == nn, f"shot names differ: {set(names) ^ set(nn)}"
os.makedirs(OUT, exist_ok=True)
base = [f for f in names if "-start-rest" in f][0]
a0, _ = cls(os.path.join(E, base))
b0, _ = cls(os.path.join(N, base))
mask = set(i for i in range(640 * 480) if a0[i] != b0[i])
rep = open(os.path.join(OUT, "compare.txt"), "w")
rep.write(f"baseline mask: {len(mask)} px from {base}\n")
rep.write("shot | diff px outside the mask: VBAR HBAR WIN ALL | WIN diff bbox\n")
for f in names:
    a, ia = cls(os.path.join(E, f))
    b, ib = cls(os.path.join(N, f))
    cnt = {}
    bbox = None
    for k, (x0, y0, x1, y1) in list(REG.items()) + [("ALL", (0, 0, 639, 479))]:
        c = 0
        for yy in range(y0, y1 + 1):
            row = yy * 640
            for xx in range(x0, x1 + 1):
                if a[row + xx] != b[row + xx] and row + xx not in mask:
                    c += 1
                    if k == "WIN":
                        bbox = (min(bbox[0], xx), min(bbox[1], yy), max(bbox[2], xx), max(bbox[3], yy)) \
                            if bbox else (xx, yy, xx, yy)
        cnt[k] = c
    step = f[:-4]
    rep.write(f"{step:40s} | {cnt['VBAR']:5d} {cnt['HBAR']:5d} {cnt['WIN']:6d} {cnt['ALL']:6d} | "
              f"{bbox if bbox else '-'}\n")
    if cnt["ALL"]:
        if cnt["VBAR"] >= cnt["HBAR"] and cnt["VBAR"] == cnt["WIN"]:
            x0, y0, x1, y1 = REG["VBAR"]
        elif cnt["HBAR"] == cnt["WIN"] and cnt["HBAR"] > 0:
            x0, y0, x1, y1 = REG["HBAR"]
        else:
            x0, y0, x1, y1 = 0, 0, 639, 479
        w, h = x1 - x0 + 1, y1 - y0 + 1
        ca = ia.crop((x0, y0, x1 + 1, y1 + 1))
        cb = ib.crop((x0, y0, x1 + 1, y1 + 1))
        cd = Image.new("RGB", (w, h), (255, 255, 255))
        pd = cd.load()
        for yy in range(h):
            for xx in range(w):
                i = (y0 + yy) * 640 + x0 + xx
                if i in mask:
                    pd[xx, yy] = (0, 0, 255)
                elif a[i] != b[i]:
                    pd[xx, yy] = (255, 0, 0)
                elif a[i] == "B":
                    pd[xx, yy] = (160, 160, 160)
        horiz = w > h
        if horiz:
            sheet = Image.new("RGB", (w, 3 * h + 8), (0, 0, 255))
            sheet.paste(ca, (0, 0))
            sheet.paste(cb, (0, h + 4))
            sheet.paste(cd, (0, 2 * h + 8))
        else:
            sheet = Image.new("RGB", (3 * w + 8, h), (0, 0, 255))
            sheet.paste(ca, (0, 0))
            sheet.paste(cb, (w + 4, 0))
            sheet.paste(cd, (2 * w + 8, 0))
        s = 3 if w < 640 else 1
        sheet.resize((sheet.width * s, sheet.height * s), Image.NEAREST).save(
            os.path.join(OUT, f"diff-{step}.png"))
rep.close()
print(open(os.path.join(OUT, "compare.txt")).read())
