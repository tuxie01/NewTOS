#!/usr/bin/env python3
"""zvcmp.py <screendump.png> <sample image> -- locate the decoded sample picture in a screendump and compare per gun.
Locating: tolerant (top 5 bits per gun equal) match of the whole picture at every (ox, oy) found by anchoring a
distinctive row (row h*5/8) in each screen row; best candidate = most top-5-bit matches. Then, per gun, at that position:
 exact8   screen value == the decoded 8-bit value
 five     screen value == the 5-bit expansion (c>>3)<<3 | c>>5 of the decoded value (the 16-plane model; for the 16-plane
          device word, green keeps 5 bits here as in gen-samples.py: expand5to8)
 max abs diff screen vs decoded. The mouse sprite rectangle (--mask x0,y0,x1,y1) is excluded."""
import sys
from PIL import Image
png, ref = sys.argv[1], sys.argv[2]
mask = tuple(int(v) for v in sys.argv[sys.argv.index("--mask") + 1].split(",")) if "--mask" in sys.argv else None
T = Image.open(ref).convert("RGB"); w, h = T.size; tp = T.load()
S = Image.open(png).convert("RGB"); SW, SH = S.size; sp = S.load()
five = lambda c: ((c >> 3) << 3) | (c >> 5)
def tol(a, b): return all((a[g] >> 3) == (b[g] >> 3) for g in range(3))
ay = h * 5 // 8
anchor = [tp[x, ay] for x in range(w)]
best = None
for sy in range(SH):
    for sx in range(SW - w + 1):
        if not (tol(sp[sx, sy], anchor[0]) and tol(sp[sx + w - 1, sy], anchor[w - 1])): continue
        if all(tol(sp[sx + x, sy], anchor[x]) for x in range(w)):
            ox, oy = sx, sy - ay
            if oy < 0 or oy + h > SH: continue
            n = sum(1 for y in range(h) for x in range(w) if tol(sp[ox + x, oy + y], tp[x, y]))
            if best is None or n > best[0]: best = (n, ox, oy)
if not best: print("NOT FOUND"); sys.exit(2)
n, ox, oy = best
tot = ex = fv = mx = 0; bad = []
gun = [[0, 0, 0] for _ in range(3)]  # per gun: exact, five, count
for y in range(h):
    for x in range(w):
        X, Y = ox + x, oy + y
        if mask and mask[0] <= X < mask[2] and mask[1] <= Y < mask[3]: continue
        for g in range(3):
            s, t = sp[X, Y][g], tp[x, y][g]
            tot += 1; gun[g][2] += 1
            if s == t: ex += 1; gun[g][0] += 1
            if s == five(t): fv += 1; gun[g][1] += 1
            mx = max(mx, abs(s - t))
            if s != t and len(bad) < 5: bad.append(((x, y), 'RGB'[g], s, t))
print("%s: found at (%d,%d) %dx%d, tolerant-match pixels %d/%d" % (png.split('/')[-1], ox, oy, w, h, n, w * h))
print("  gun values compared %d: exact8 %d (%.1f%%), equal to 5-bit expansion %d (%.1f%%), max |screen-decoded| %d"
      % (tot, ex, 100.0 * ex / tot, fv, 100.0 * fv / tot, mx))
print("  per gun exact8/five/n: R %s G %s B %s" % tuple(gun))
if bad: print("  first mismatches vs decoded 8-bit (x,y gun screen decoded):", bad)
sys.exit(0 if ex == tot else 1)
