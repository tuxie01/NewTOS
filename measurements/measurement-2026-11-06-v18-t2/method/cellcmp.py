#!/usr/bin/env python3
"""cellcmp.py <emutos-out> <newtos-out> <case> x0 x1 y0 y1 -- count differing pixels (raw words) of one
vdprobe case inside the screen window x0..x1, y0..y1 and print both windows as pen maps."""
import sys, os, importlib.util
spec = importlib.util.spec_from_file_location("vc", os.path.join(os.path.dirname(__file__), "../../../tosdump/probes/vdprobe-compare.py"))
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
ea, na = vc.load(sys.argv[1]), vc.load(sys.argv[2])
me, mn = vc.penmap(ea), vc.penmap(na)
x0, x1, y0, y1 = map(int, sys.argv[4:8])
for (no, nm, x, y, w, h, we), (_, _, _, _, _, _, wn) in zip(ea, na):
    if nm != sys.argv[3]:
        continue
    d = 0
    rows = []
    for yy in range(y0, y1 + 1):
        re_, rn = "", ""
        for xx in range(x0, x1 + 1):
            i = (yy - y) * w + (xx - x)
            d += we[i] != wn[i]
            re_ += vc.sym(me, we[i]); rn += vc.sym(mn, wn[i])
        rows.append("%4d E %s  N %s" % (yy, re_, rn))
    print("%s window x %d..%d y %d..%d: %d px differ" % (nm, x0, x1, y0, y1, d))
    print("\n".join(rows))
