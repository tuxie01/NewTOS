#!/usr/bin/env python3
"""render-cases.py <out.png> <scale> <case>... -- E | head | base panels of vdprobe screen grabs (RGB565 words) for one or more cases.
Needs the three runs probe/e1, probe/n-head, probe/n-base next to this method dir's parent."""
import sys, os, importlib.util
from PIL import Image, ImageDraw
spec = importlib.util.spec_from_file_location("c", "tosdump/probes/vdprobe-compare.py"); c = importlib.util.module_from_spec(spec); spec.loader.exec_module(c)
P = "measurements/measurement-2026-11-06-v33-test/probe/"
runs = [("EmuTOS", c.load(P + "e1")), ("head 877a14b9", c.load(P + "n-head")), ("base 587ddbd5", c.load(P + "n-base"))]
out, sc, names = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
def img(case):
    no, nm, x, y, w, h, words = case
    im = Image.new("RGB", (w, h))
    px = im.load()
    for i, v in enumerate(words):
        px[i % w, i // w] = (((v >> 11) & 31) * 255 // 31, ((v >> 5) & 63) * 255 // 63, (v & 31) * 255 // 31)
    return im.resize((w * sc, h * sc), Image.NEAREST), (x, y)
rows = []
for n in names:
    ims = []
    for lab, cs in runs:
        cc = [k for k in cs if k[1] == n][0]
        im, xy = img(cc); ims.append((lab, im, xy))
    rows.append((n, ims))
W = sum(i[1].width for i in rows[0][1]) + 20 * 4
H = sum(max(i[1].height for i in r[1]) + 24 for r in rows)
W = max(W, max(sum(i[1].width for i in r[1]) + 80 for r in rows))
canvas = Image.new("RGB", (W, H), (200, 200, 255)); d = ImageDraw.Draw(canvas); y = 0
for n, ims in rows:
    x = 10
    for lab, im, xy in ims:
        d.text((x, y + 4), f"{n} {lab} at {xy}", fill=(0, 0, 0)); canvas.paste(im, (x, y + 20)); x += im.width + 20
    y += max(i[1].height for i in ims) + 24
canvas.save(out)
