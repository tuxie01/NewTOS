#!/usr/bin/env python3
"""flicker-sheet.py <emutos-shots> <newtos-shots> <out.png> <tag> <x0> <y0> <x1> <y1> <n...> --
one EmuTOS sample (e05) and the NewTOS samples n<k> of hold <tag>, cropped to the bar, side by
side, 3x (TASK/A130 flicker run)."""
import os
import sys

from PIL import Image

E, N, OUT, TAG = sys.argv[1:5]
x0, y0, x1, y1 = map(int, sys.argv[5:9])
ks = sys.argv[9:]


def find(d, suffix):
    return [os.path.join(d, f) for f in sorted(os.listdir(d)) if f.endswith(f"{TAG}-{suffix}.png")][0]


crops = [Image.open(find(E, "e05")).convert("RGB").crop((x0, y0, x1 + 1, y1 + 1))]
crops += [Image.open(find(N, f"n{k}")).convert("RGB").crop((x0, y0, x1 + 1, y1 + 1)) for k in ks]
w, h = crops[0].size
horiz = w > h
sheet = Image.new("RGB", ((w + 6) * len(crops), h) if not horiz else (w, (h + 6) * len(crops)), (0, 0, 255))
for i, c in enumerate(crops):
    sheet.paste(c, ((w + 6) * i, 0) if not horiz else (0, (h + 6) * i))
sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(OUT)
