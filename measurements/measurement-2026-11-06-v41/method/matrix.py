#!/usr/bin/env python3
"""matrix.py <e-run> <n-run>: the Z* series of vdprobe.c as tables of differing pixels (raw words)."""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from rawmap import load
a, b = load(sys.argv[1]), load(sys.argv[2])
def d(n):
    wa, wb = a[n][5], b[n][5]
    return sum(1 for p, q in zip(wa, wb) if p != q)
eff = [1, 4, 16, 5, 17, 20, 21, 12, 2, 6]
names = {1:"bold",4:"skew",16:"outl",5:"b+s",17:"b+o",20:"s+o",21:"b+s+o",12:"s+und",2:"light",6:"l+s"}
print("ZA: differing pixels; rows effect, columns write mode 1 replace 2 transparent 3 xor 4 reverse transparent")
n = 0
for f, fn in enumerate(["8x16", "8x8", "6x6", "proportional loaded"]):
    print(fn)
    for e in eff:
        r = []
        for m in range(4):
            r.append("%6d" % d("ZA%03d" % n)); n += 1
        print("  %-6s%s" % (names[e], "".join(r)))
print("ZB: 8x16 scaled; rows effect; columns mode 1, 2")
n = 0
for s in (26, 20):
    print(" vst_height", s)
    for e in (1, 4, 16, 21):
        print("  %-6s%6d%6d" % (names[e], d("ZB%03d" % n), d("ZB%03d" % (n+1)))); n += 2
print("ZC: 8x16 rotated; rows effect; columns mode 1, 2")
n = 0
for s in (900, 1800, 2700):
    print(" rotation", s)
    for e in (1, 4, 16, 21):
        print("  %-6s%6d%6d" % (names[e], d("ZC%03d" % n), d("ZC%03d" % (n+1)))); n += 2
print("ZD: 8x16 clipped; rows effect; columns mode 1, 2")
n = 0
for s in ("left 40","right 60","top 92","bottom 96"):
    print(" clip", s)
    for e in (1, 4, 16, 21):
        print("  %-6s%6d%6d" % (names[e], d("ZD%03d" % n), d("ZD%03d" % (n+1)))); n += 2
print("ZE: loaded mono, skew masks; columns effects skew, b+s, s+o")
n = 0
cells = [(8,16),(40,16),(8,15),(8,17),(40,15),(40,17)]
masks = [0x5555,0xAAAA,0x8888,0x1111,0xFFFF,0,0x1234]
for c,(W,H) in enumerate(cells):
    for k,m in enumerate(masks):
        if k != 0 and c >= 2: continue
        print("  %dx%d %04x %6d%6d%6d" % (W,H,m,d("ZE%03d"%n),d("ZE%03d"%(n+1)),d("ZE%03d"%(n+2)))); n += 3
