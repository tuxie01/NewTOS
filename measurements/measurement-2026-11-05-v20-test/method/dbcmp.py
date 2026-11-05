#!/usr/bin/env python3
"""dbcmp.py <runs-dir> -- V20 acceptance, dbprobe: per shot, pixels differing base vs head, and in the strip x 290..370 (clean desktop
background on both sides; the menu bar text ends left of x 270) the pixels differing EmuTOS vs head and EmuTOS vs base."""
import sys, os
from PIL import Image
R = sys.argv[1]
def load(p):
    im = Image.open(p).convert("RGB")
    if im.size[0] == 644:
        w, h = im.size; px = im.load(); best = None
        for off in (0, 4, 2):
            cols = [x for x in range(w) if x < off or x >= off + 640]
            if all(px[x, y] == px[cols[0], 0] for x in cols for y in range(0, h, 7)):
                best = off; break
        im = im.crop((best, 0, best + 640, h))
    return im.load()
def q(c): return (c[0] >> 3, c[1] >> 3, c[2] >> 3)
names = sorted(n for n in os.listdir(os.path.join(R, "db-head", "shots")) if n.endswith(".png"))
for n in names:
    h = load(f"{R}/db-head/shots/{n}"); b = load(f"{R}/db-base/shots/{n}"); e = load(f"{R}/db-emutos/shots/{n}")
    def diff(a, c, x0=0, x1=640):
        return [(x, y) for y in range(480) for x in range(x0, x1) if q(a[x, y]) != q(c[x, y])]
    bh = diff(b, h); eh = diff(e, h, 290, 370); eb = diff(e, b, 290, 370)
    print(f"{n}: base vs head {len(bh)} px; strip x290..369 EmuTOS vs head {len(eh)} px, EmuTOS vs base {len(eb)} px")
    if bh:
        ys = sorted(set(y for x, y in bh)); xs = sorted(set(x for x, y in bh))
        print("   base-vs-head differing: x", xs, "y", ys[0], "..", ys[-1], " (%d rows)" % len(ys))
        print("   of those pixels, equal to EmuTOS: head %d, base %d" % (sum(q(h[x, y]) == q(e[x, y]) for x, y in bh), sum(q(b[x, y]) == q(e[x, y]) for x, y in bh)))

print("--- frame columns x=300 and x=359 (the vertical segments), all rows: EmuTOS vs head / base")
for n in names:
    h = load(f"{R}/db-head/shots/{n}"); b = load(f"{R}/db-base/shots/{n}"); e = load(f"{R}/db-emutos/shots/{n}")
    for x in (300, 359):
        dh = [y for y in range(480) if q(e[x, y]) != q(h[x, y])]
        db_ = [y for y in range(480) if q(e[x, y]) != q(b[x, y])]
        print(f"{n} x={x}: EmuTOS!=head at rows {dh if len(dh) < 12 else str(len(dh)) + ' rows ' + str(dh[:3]) + '..'}; EmuTOS!=base at rows {db_ if len(db_) < 12 else str(len(db_)) + ' rows ' + str(db_[:3]) + '..'}")
