#!/usr/bin/env python3
"""hyp-compare.py <newtos.png> <oracle.png> <y0> <y1> -- TASK/A142: 5 bits per channel (the oracle screen is 565 with white 248,252,248), band y0..y1 (window content), whole width, with a bounding box."""
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
y0, y1 = int(sys.argv[3]), int(sys.argv[4])
w = min(a.width, b.width); pa, pb = a.load(), b.load()
d = [(x, y) for y in range(y0, y1) for x in range(w) if [c >> 3 for c in pa[x, y]] != [c >> 3 for c in pb[x, y]]]
print(sys.argv[1].split("/")[-3], "vs oracle: y", y0, "..", y1, "pixels", w * (y1 - y0), "differ", len(d), ("box x%d..%d y%d..%d" % (min(x for x, _ in d), max(x for x, _ in d), min(y for _, y in d), max(y for _, y in d))) if d else "")
