#!/usr/bin/env python3
"""findpix.py <emutos-out> <newtos-out> <case>... -- for each named vdprobe case, print up to 4 screen
pixels whose pen differs (EmuTOS pen, NewTOS pen) and one where both are pen 1 (a control), to pick the
v_get_pixel points of tosdump/cases/vdi-v18-t2.tsv.  Uses tosdump/probes/vdprobe-compare.py's loader."""
import sys, os, importlib.util
spec = importlib.util.spec_from_file_location("vc", os.path.join(os.path.dirname(__file__), "../../../tosdump/probes/vdprobe-compare.py"))
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
ea, na = vc.load(sys.argv[1]), vc.load(sys.argv[2])
me, mn = vc.penmap(ea), vc.penmap(na)
for want in sys.argv[3:]:
    for (no, nm, x, y, w, h, we), (_, _, _, _, _, _, wn) in zip(ea, na):
        if nm != want:
            continue
        d = [(i, vc.sym(me, we[i]), vc.sym(mn, wn[i])) for i in range(w * h) if vc.sym(me, we[i]) != vc.sym(mn, wn[i])]
        same = [i for i in range(w * h) if vc.sym(me, we[i]) == '#' and vc.sym(mn, wn[i]) == '#']
        out = ["(%d,%d) E=%s/%04x N=%s/%04x" % (x + i % w, y + i // w, a, we[i], b, wn[i]) for i, a, b in d[:4]]
        ctl = ["(%d,%d) both %04x" % (x + i % w, y + i // w, we[i]) for i in same[:1]]
        print(nm, "diff", len(d), "|", "; ".join(out), "| control", "; ".join(ctl))
