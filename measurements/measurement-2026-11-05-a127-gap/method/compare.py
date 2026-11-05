#!/usr/bin/env python3
"""compare.py <emutos-shots> <newtos-shots> <out-dir> -- pixel comparison of the mgprobe
screenshots, step by step (same file names on both sides).

Both screens are 640x480; EmuTOS TT medium (16 colours, PNG colormap) and NewTOS (RGB565
via QMP) differ in colour encoding, so each pixel is reduced to black/white
(luma < 128 = black) before comparing. The menu is black-and-white on both sides.
Regions (absolute pixels, mgprobe.c build_tree):
  BAR    the menu bar        x 0..639, y 0..19 (incl. the bar's bottom line)
  FILE   File box + border   x 63..160, y 19..99
  GAPS   Gaps box + border   x 111..224, y 19..115
  MENU   the union rectangle x 0..300, y 0..140 (bar, both boxes, margins)
  ALL    the whole screen
Per step: black-pixel count of each side in MENU, differing pixels per region, the bounding
box of the MENU differences, and the state of each item (fraction of black pixels in the
item's rectangle on each side: a selected item is ~ >= 0.7 black, a normal one ~ <= 0.3).
Writes <out>/compare.txt and, for every step with MENU differences, a 4x crop
<out>/diff-<step>.png (EmuTOS | NewTOS | difference in red).
"""
import os
import sys

from PIL import Image

E, N, OUT = sys.argv[1:4]
REG = {"BAR": (0, 0, 639, 19), "FILE": (63, 19, 160, 99), "GAPS": (111, 19, 224, 115),
       "MENU": (0, 0, 300, 140)}
ITEMS = {  # name: (x0, y0, x1, y1) inclusive
    "t.File": (64, 0, 111, 18), "t.Gaps": (112, 0, 159, 18),
    "F.A": (64, 19, 159, 34), "F.B": (64, 35, 159, 50), "F.sep": (64, 51, 159, 66),
    "F.C": (64, 67, 159, 82), "F.Quit": (64, 83, 159, 98),
    "G.1": (112, 19, 223, 34), "G.2": (112, 35, 223, 50), "G.gap": (112, 51, 223, 58),
    "G.sep": (112, 59, 223, 74), "G.4": (112, 75, 223, 90), "G.5": (112, 91, 207, 106),
    "G.rgap": (208, 91, 223, 106), "G.bgap": (112, 107, 223, 114),
}


def bw(path):
    im = Image.open(path).convert("L")
    assert im.size == (640, 480), (path, im.size)
    return im.point(lambda v: 0 if v < 128 else 255)


def black_frac(px, r):
    x0, y0, x1, y1 = r
    n = b = 0
    for yy in range(y0, y1 + 1):
        for xx in range(x0, x1 + 1):
            n += 1
            b += px[xx, yy] == 0
    return b / n


names = sorted(f for f in os.listdir(E) if f.endswith(".png"))
assert names == sorted(f for f in os.listdir(N) if f.endswith(".png")), "shot names differ"
os.makedirs(OUT, exist_ok=True)
rep = open(os.path.join(OUT, "compare.txt"), "w")
rep.write("step | diff px BAR FILE GAPS MENU ALL | MENU diff bbox | item black fraction E/N (only items "
          "where one side >= 0.5)\n")
for f in names:
    a, b = bw(os.path.join(E, f)), bw(os.path.join(N, f))
    pa, pb = a.load(), b.load()
    cnt = {}
    bbox = None
    for k, (x0, y0, x1, y1) in list(REG.items()) + [("ALL", (0, 0, 639, 479))]:
        c = 0
        for yy in range(y0, y1 + 1):
            for xx in range(x0, x1 + 1):
                if pa[xx, yy] != pb[xx, yy]:
                    c += 1
                    if k == "MENU":
                        bbox = (min(bbox[0], xx), min(bbox[1], yy), max(bbox[2], xx), max(bbox[3], yy)) \
                            if bbox else (xx, yy, xx, yy)
        cnt[k] = c
    sel = []
    for it, r in ITEMS.items():
        fe, fn = black_frac(pa, r), black_frac(pb, r)
        if fe >= 0.5 or fn >= 0.5:
            sel.append(f"{it} {fe:.2f}/{fn:.2f}")
    step = f[:-4]
    rep.write(f"{step:28s} | {cnt['BAR']:5d} {cnt['FILE']:5d} {cnt['GAPS']:5d} {cnt['MENU']:5d} {cnt['ALL']:6d}"
              f" | {bbox if bbox else '-'} | {', '.join(sel) if sel else 'no item >= 0.5'}\n")
    if cnt["MENU"]:
        x0, y0, x1, y1 = 0, 0, 300, 140
        w, h = x1 - x0 + 1, y1 - y0 + 1
        ca = a.crop((x0, y0, x1 + 1, y1 + 1)).convert("RGB")
        cb = b.crop((x0, y0, x1 + 1, y1 + 1)).convert("RGB")
        cd = Image.new("RGB", (w, h), (255, 255, 255))
        pd = cd.load()
        for yy in range(h):
            for xx in range(w):
                if pa[x0 + xx, y0 + yy] != pb[x0 + xx, y0 + yy]:
                    pd[xx, yy] = (255, 0, 0)
                elif pa[x0 + xx, y0 + yy] == 0:
                    pd[xx, yy] = (160, 160, 160)
        sheet = Image.new("RGB", (3 * w + 8, h), (0, 0, 255))
        sheet.paste(ca, (0, 0))
        sheet.paste(cb, (w + 4, 0))
        sheet.paste(cd, (2 * w + 8, 0))
        sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(
            os.path.join(OUT, f"diff-{step}.png"))
rep.close()
print(open(os.path.join(OUT, "compare.txt")).read())
