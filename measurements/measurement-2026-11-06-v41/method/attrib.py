#!/usr/bin/env python3
"""attrib.py <out-dir>: for each scratch variant offN (the candidate with part N of the EmuTOS behaviour left
out) the cases that then differ from the full candidate off0. Prints one block per part, the case names, grouped."""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from rawmap import load
out = sys.argv[1]
NAMES = {1: "window slides (screen_blit16)", 2: "mono bold keeps the spill (pre_blit width + weight)",
         3: "outline, last row reads its own result", 4: "buffer stride from LOFF+ROFF, not from the lean",
         5: "multi-word skew phase (skewopw)", 6: "window source pointer +1 byte at a mask wrap",
         7: "skew+outline adjustments of screen_blit16", 8: "window also slides after rotate",
         9: "do_clip rectangle + slide past it (instead of a clip per pixel)"}
base = load(os.path.join(out, "off0"))
for k in NAMES:
    v = load(os.path.join(out, "off%d" % k))
    names = [n for n in base if base[n][5] != v[n][5]]
    px = {n: sum(1 for p, q in zip(base[n][5], v[n][5]) if p != q) for n in names}
    groups = {}
    for n in names:
        g = n[:2] if n[0] == "Z" else ("Y" if n[0] == "Y" else "other")
        groups.setdefault(g, []).append(n)
    print("off%d %s: %d cases" % (k, NAMES[k], len(names)))
    for g in sorted(groups):
        print("   %-5s %3d: %s" % (g, len(groups[g]), " ".join("%s(%d)" % (n, px[n]) for n in groups[g])))
