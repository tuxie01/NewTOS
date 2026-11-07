#!/usr/bin/env python3
"""glyph-compare.py <runs-dir> -- A142-test, own script. Compares glyphs of QEMU 16-plane screens (base, fix) with the TT-medium 4-plane Hatari oracle.
A 16-plane and a 4-plane screen cannot be compared by RGB value (different palettes, TT gets 4 bits/channel). Each pixel is mapped to the nearest of
five reference classes (black, white, blue, red, grey) using the colours measured in that screen itself for the dialog background and the glyph pixels;
same pixel coordinates (the oracle screen is 640 wide, its top-left is the same as the QEMU screen's; extra rows below are Hatari's status bar)."""
import sys
from PIL import Image
R = sys.argv[1]
def im(p): return Image.open(f"{R}/{p}").convert("RGB")
def cls(c):
    r, g, b = c
    if r < 64 and g < 64 and b < 64: return "K"
    if r > 230 and g > 230 and b > 230: return "W"
    if b > 150 and r < 100 and g < 100: return "B"
    if r > 150 and g < 100 and b < 100: return "R"
    if abs(r - g) < 30 and abs(g - b) < 30: return "g"   # grey dialog background (any shade)
    return "?"
def grid(i, x, y, w=16, h=16):
    p = i.load(); return [[cls(p[x + a, y + b]) for a in range(w)] for b in range(h)]
def diff(a, b): return sum(c != d for ra, rb in zip(a, b) for c, d in zip(ra, rb))
def show(g): return "\n".join("".join(r) for r in g)
oracle = {"global": im("oracle-qed-tt-global/shots/grab0002.png"), "info": im("oracle-qed-tt-info/shots/grab0003.png"), "alert": im("oracle-qed-tt-alert/shots/grab0003.png")}
q = {v: {"global": im(f"qed-{v}-issue/shots/00-global.png"), "info": im(f"qed-{v}-issue/shots/03-info.png"), "alert": im(f"qed-{v}-issue/shots/01-alert.png")} for v in ("base", "fix")}
# unclassified pixels anywhere in glyph areas?
unk = 0
rows = [133 + 16 * i for i in range(9)]
def state(g):  # an 'on' box has an inner mark: count K pixels
    return sum(c == "K" for r in g for c in r)
og = {y: grid(oracle["global"], 334, y) for y in rows}
print("oracle check boxes, K-pixel counts per row:", [state(g) for g in og.values()])
for v in ("base", "fix"):
    print(f"== {v}: check boxes (Globale Einstellungen), each vs the oracle's box with the same state; best/worst over same-state oracle boxes")
    for y in rows:
        g = grid(q[v]["global"], 334, y)
        k = state(g)
        # same state: K count class (on boxes have more K than off boxes): compare with oracle boxes of similar K count
        cand = [diff(g, o) for o in og.values()]
        print(f"  y{y} K={k:3d}  diff vs oracle boxes: min {min(cand):3d}  (all: {cand})")
for v in ("base", "fix"):
    print(f"== {v}: radio buttons (Textinfo) at y281: x184 (on), x292 (off), x384 (off) vs oracle same coordinates")
    for x in (184, 292, 384):
        g = grid(q[v]["info"], x, 281); o = grid(oracle["info"], x, 281)
        print(f"  x{x}: {diff(g,o)} px differ of 256")
print("fix glyph at check y=165 (on) / y=133 (off), oracle same y:")
print(show(grid(q['fix']['global'], 334, 165))); print(); print(show(grid(oracle['global'], 334, 165)))
print("radio on, fix vs oracle:"); print(show(grid(q['fix']['info'], 184, 281))); print(); print(show(grid(oracle['info'], 184, 281)))
# alert icon: 32x32 blue block; find its top-left in each screen (blue pixels bbox) and compare by bbox-aligned 32x32
def bbox_blue(i):
    p = i.load(); xs = []; ys = []
    for y in range(180, 320):
        for x in range(180, 260):
            if cls(p[x, y]) == "B": xs.append(x); ys.append(y)
    return (min(xs), min(ys)) if xs else None
bo = bbox_blue(oracle["alert"]); print("alert blue bbox origin oracle", bo)
for v in ("base", "fix"):
    b = bbox_blue(q[v]["alert"]); print(v, "origin", b)
    if b and bo:
        # anchor on the box origin minus a common offset of 2 px to include icon margin
        print("  32x32 around blue origin: differ", diff(grid(q[v]["alert"], b[0] - 2, b[1] - 2, 32, 32), grid(oracle["alert"], bo[0] - 2, bo[1] - 2, 32, 32)), "of 1024")
