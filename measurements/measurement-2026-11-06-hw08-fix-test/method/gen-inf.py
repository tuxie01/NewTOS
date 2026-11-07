#!/usr/bin/env python3 -I
"""gen-inf.py <script-out> <shots-out> <item-dy|0> : desktop, corner reset, hover Options (x=196,y=9); item-dy>0 = move down by item-dy px and click (Save Desktop)."""
import sys
out, so, dy = sys.argv[1], sys.argv[2], int(sys.argv[3])
L = []; S = []; t = 100
def pk(b, dx, dy):
    global t
    L.append((t, [b, dx & 255, dy & 255])); t += 3
for _ in range(6): pk(0xf8, -127, 127)
t += 20
for d in (-127, -127, -127, -89): pk(0xf8, 0, d)
t += 20; pk(0xf8, 98, 0); pk(0xf8, 98, 0); pk(0xf8, 0, 0); t += 40
S.append((t + 100, "options-open"))
if dy:
    t += 300; pk(0xf8, 0, dy // 2); pk(0xf8, 0, dy - dy // 2); pk(0xf8, 0, 0); t += 40
    S.append((t + 100, "item-hover"))
    t += 200
    for _ in range(3): pk(0xfa, 0, 0)
    pk(0xf8, 0, 0)
    S.append((t + 600, "after-click")); S.append((t + 1500, "later")); S.append((t + 3000, "end"))
open(out, "w").write("".join("%d %s\n" % (tt, " ".join("0x%02x" % b for b in bs)) for tt, bs in L))
open(so, "w").write("".join("%d %s\n" % (tt, n) for tt, n in sorted(S)))
