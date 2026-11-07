#!/usr/bin/env python3 -I
"""gen-fbench.py <script-out> <shots-out> : IKBD script for FBENCH.NAP (hw08 test): ^A (run all, ~100 s in QEMU), then ^S + Return (profile),
^E + Return (export), File > Results report (PDF) + Return. 640x480. Ticks = 200 Hz after 'up:FBENCH armed'."""
import sys
out, shots_out = sys.argv[1:3]
L = []; S = []
def key(t, *codes): 
    for i, c in enumerate(codes): L.append((t + i * 3, [c]))
def ctrl(t, scan): key(t, 0x1d, scan, scan | 0x80, 0x9d)
def ret(t): key(t, 0x1c, 0x9c)
def pk(t, b, dx, dy): L.append((t, [b, dx & 255, dy & 255]))
def pkt(t, b, dx, dy): L.append((t, [0xf0 | b if False else b, dx & 255, dy & 255]))
ctrl(800, 0x1e)                      # ^A
T0 = 30000
ctrl(T0, 0x1f); S.append((T0 + 600, "fsel-profile")); ret(T0 + 1000); S.append((T0 + 2000, "after-profile"))
T1 = 33000
ctrl(T1, 0x12); S.append((T1 + 600, "fsel-export")); ret(T1 + 1000); S.append((T1 + 2000, "after-export"))
T2 = 36000
t = T2
for _ in range(6): pk(t, 0xf8, -127, 127); t += 3        # bottom-left corner
t += 20
for dy in (-127, -127, -127, -89): pk(t, 0xf8, 0, dy); t += 3   # up to y=9 at x=0 (blank menu bar)
t += 20; pk(t, 0xf8, 88, 0); t += 3; pk(t, 0xf8, 0, 0); t += 30  # hover File
S.append((t + 100, "file-menu-open"))
t += 300; pk(t, 0xf8, 22, 50); t += 3; pk(t, 0xf8, 0, 0); t += 30  # item 3 (Results report PDF)
S.append((t + 100, "item3-hover"))
t += 200
for _ in range(3): pk(t, 0xfa, 0, 0); t += 3
pk(t, 0xf8, 0, 0); t += 3
S.append((t + 600, "fsel-pdf")); ret(t + 1000); S.append((t + 2500, "after-pdf")); S.append((t + 5000, "end"))
L.sort(key=lambda x: x[0])
open(out, "w").write("".join("%d %s\n" % (tt, " ".join("0x%02x" % b for b in bs)) for tt, bs in L))
open(shots_out, "w").write("".join("%d %s\n" % (tt, n) for tt, n in sorted(S)))
