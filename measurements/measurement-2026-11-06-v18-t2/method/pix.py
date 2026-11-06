#!/usr/bin/env python3
"""pix.py <out-dir> <case> x,y ... -- the screen words of one vdprobe case at the given points."""
import sys, os, importlib.util
spec = importlib.util.spec_from_file_location("vc", os.path.join(os.path.dirname(__file__), "../../../tosdump/probes/vdprobe-compare.py"))
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
for (no, nm, x, y, w, h, ws) in vc.load(sys.argv[1]):
    if nm == sys.argv[2]:
        print(nm, " ".join("(%s)=%04x" % (p, ws[(int(p.split(',')[1]) - y) * w + int(p.split(',')[0]) - x]) for p in sys.argv[3:]))
