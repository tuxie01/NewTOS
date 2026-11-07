#!/usr/bin/env python3
"""compare.py <before-shots> <after-shots> <oracle-global.png> <oracle-alert.png> <oracle-info.png> <out-sheet.png> -- TASK/A142.
Part 1: where do the before and after screens differ (bounding boxes of the differing pixels, 8-connected through a 3-pixel gap)?
Part 2: the glyphs, classed black/white/blue/red/other, after vs oracle and before vs oracle, at the same coordinates (oracle: TT 4 planes, English QED.RSC, so only
the classes are compared and the alert glyph is searched, the alert box has another width). Writes an enlarged sheet: before, after, oracle."""
import sys
from PIL import Image
B, A, OG, OA, OI, OUT = sys.argv[1:7]
names = ["00-global.png", "01-alert.png", "02-neu.png", "03-info.png"]
def rgb(p): return Image.open(p).convert("RGB")
def boxes(a, b):
    w, h = a.size; pa, pb = a.load(), b.load()
    pts = {(x, y) for y in range(h) for x in range(w) if pa[x, y] != pb[x, y]}
    out = []
    while pts:
        st = [pts.pop()]; xs = []; ys = []
        while st:
            x, y = st.pop(); xs.append(x); ys.append(y)
            for dx in range(-3, 4):
                for dy in range(-3, 4):
                    q = (x + dx, y + dy)
                    if q in pts: pts.remove(q); st.append(q)
        out.append((min(xs), min(ys), max(xs), max(ys), len(xs)))
    return sorted(out, key=lambda r: (r[1], r[0]))
for n in names:
    bs = boxes(rgb(f"{B}/{n}"), rgb(f"{A}/{n}"))
    print(n, "differing regions before/after:", len(bs))
    for b in bs: print("   x%3d..%3d y%3d..%3d  %4d px" % (b[0], b[2], b[1], b[3], b[4]))
def cls(c):
    r, g, b = c
    if r < 60 and g < 60 and b < 60: return "K"
    if r > 200 and g > 200 and b > 200: return "W"
    if b > 180 and r < 80 and g < 80: return "B"
    if r > 180 and g < 80 and b < 80: return "R"
    return "."
def grid(im, x, y, w, h):
    p = im.load(); return [[cls(p[x + i, y + j]) for i in range(w)] for j in range(h)]
def diff(a, b): return sum(1 for ra, rb in zip(a, b) for ca, cb in zip(ra, rb) if ca != cb)
ob = {"global": rgb(OG), "alert": rgb(OA), "info": rgb(OI)}
sheet = []
def tile(name, shot, key, x, y, w=16, h=16, ox=None, oy=None):
    ox = x if ox is None else ox; oy = y if oy is None else oy
    be, af, oc = rgb(f"{B}/{shot}"), rgb(f"{A}/{shot}"), ob[key]
    g = grid(oc, ox, oy, w, h)
    print(f"{name:34s} at {x},{y} oracle {ox},{oy} size {w}x{h}: after differs {diff(grid(af, x, y, w, h), g):4d} px, before differs {diff(grid(be, x, y, w, h), g):4d} px")
    sheet.append((name, be.crop((x, y, x + w, y + h)), af.crop((x, y, x + w, y + h)), oc.crop((ox, oy, ox + w, oy + h))))
# check boxes, at their rows in Global Settings (the states of the rows differ between the two QED.CFG, so like is compared with like)
tile("check off: Parameter / Auto-save", names[0], "global", 334, 133)
tile("check on: GEM-Klemmbrett", names[0], "global", 334, 165)
tile("check on: Syntax", names[0], "global", 334, 261)
tile("check off: Ext. Druckdialog", names[0], "global", 334, 213)
# radio buttons, Textinfo
tile("radio on: Atari", names[3], "info", 184, 281)
tile("radio off: Unix", names[3], "info", 292, 281)
tile("radio off: Apple", names[3], "info", 384, 281)
# alert icon: search it in the oracle's alert box
af = rgb(f"{A}/{names[1]}"); g = grid(af, 198, 229, 32, 32); best = None
oc = ob["alert"]
for oy in range(215, 245):
    for ox in range(185, 225):
        d = diff(grid(oc, ox, oy, 32, 32), g)
        if best is None or d < best[0]: best = (d, ox, oy)
print("alert icon found on the oracle at", best[1:], "with", best[0], "px different")
tile('alert "!"', names[1], "alert", 198, 229, 32, 32, best[1], best[2])
S = 6; W = sum(0 for _ in sheet)
rows = len(sheet); cell = 32 * S + 12
img = Image.new("RGB", (3 * cell + 12, rows * cell + 12), (120, 120, 120))
for r, (n, a, b, c) in enumerate(sheet):
    for k, t in enumerate((a, b, c)):
        img.paste(t.resize((t.width * S, t.height * S), Image.NEAREST), (12 + k * cell, 12 + r * cell))
img.save(OUT)
