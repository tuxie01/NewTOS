#!/usr/bin/env python3
"""pencmp.py <emutos-tt-shots> <newtos-shots> <out-dir> -- A131: compare the TT-profile screenshots
with NewTOS's by VDI PEN, not by RGB. The TT and the Falcon VDI seed different default palettes
(references/emutos/vdi/vdi_col.c:58-63 tt_palette1 vs :130-135 videl_palette1; measured by
tosdump/cases/vdi-a131-palette.tsv), so an RGB compare TT vs NewTOS mixes "which pen" with "what a
pen looks like". Each pixel is mapped back to its pen through the side's own measured palette:
  TT (4-bit guns x 17)       pen 0 (255,255,255) 1 (0,0,0) 3 (0,255,0) 8 (170,170,170) 9 (102,102,102)
                             2 (255,0,0)
  NewTOS (5-bit guns, x 255/31) pen 0 (255,255,255) 1 (0,0,0) 3 (0,255,0) 8 (189,189,189) 9 (140,140,140)
                             2 (255,0,0)
An RGB not in the table is reported as '?rgb'.
"""
import os
import sys
from collections import Counter

from PIL import Image

E, N, OUT = sys.argv[1:4]
os.makedirs(OUT, exist_ok=True)
TT = {(255, 255, 255): 0, (0, 0, 0): 1, (255, 0, 0): 2, (0, 255, 0): 3, (170, 170, 170): 8, (102, 102, 102): 9}
NT = {(255, 255, 255): 0, (0, 0, 0): 1, (255, 0, 0): 2, (0, 255, 0): 3, (189, 189, 189): 8, (140, 140, 140): 9}
summary = []
for name in sorted(os.listdir(E)):
    if not name.endswith(".png") or not os.path.exists(os.path.join(N, name)):
        continue
    a = Image.open(os.path.join(E, name)).convert("RGB").load()
    b = Image.open(os.path.join(N, name)).convert("RGB").load()
    pairs = Counter()
    pts = []
    for y in range(480):
        for x in range(640):
            pe = TT.get(a[x, y], "?%s" % (a[x, y],))
            pn = NT.get(b[x, y], "?%s" % (b[x, y],))
            if pe != pn:
                pairs[(pe, pn)] += 1
                pts.append((x, y, pe, pn))
    line = f"{name}: {len(pts)} px differ by pen"
    if pts:
        body = [p for p in pts if p[1] >= 19]
        line += f" ({len(pts) - len(body)} in the menu bar y<19)"
        if body:
            line += f", outside the menu bar bbox x {min(p[0] for p in body)}..{max(p[0] for p in body)} " \
                    f"y {min(p[1] for p in body)}..{max(p[1] for p in body)}"
        line += "; pen pairs E->N " + ", ".join(f"{e}->{n} x{c}" for (e, n), c in pairs.most_common(6))
    summary.append(line)
    with open(os.path.join(OUT, name[:-4] + ".txt"), "w") as f:
        f.write(line + "\n")
        for p in pts:
            f.write("%d,%d pen E=%s N=%s\n" % p)
with open(os.path.join(OUT, "summary.txt"), "w") as f:
    f.write("\n".join(summary) + "\n")
print("\n".join(summary))
