#!/usr/bin/env python3
"""rowsdiff.py <e-run> <n-run> <case>...: per case the rows and columns that differ."""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from rawmap import load
a, b = load(sys.argv[1]), load(sys.argv[2])
for n in sys.argv[3:]:
    _, x, y, w, h, wa = a[n]; wb = b[n][5]
    d = [(x+i%w, y+i//w) for i in range(w*h) if wa[i] != wb[i]]
    if not d: print(n, "equal"); continue
    print(n, "rect", x, y, w, h, "px", len(d), "x", min(p[0] for p in d), "..", max(p[0] for p in d), "y", min(p[1] for p in d), "..", max(p[1] for p in d))
