#!/usr/bin/env python3
"""pixdiff.py A.png B.png -- bounding box and count of differing pixels."""
import sys
from PIL import Image, ImageChops

a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
d = ImageChops.difference(a, b)
bbox = d.getbbox()
pix = list(d.get_flattened_data() if hasattr(d, "get_flattened_data") else pix)
n = sum(1 for p in pix if p != (0, 0, 0))
print(f"size {a.size}  differing pixels {n}  bbox {bbox}")
if bbox:
    # The changed pixels in two clusters = cursor gone from the old place,
    # drawn at the new one.
    xs = [i % a.size[0] for i, p in enumerate(pix) if p != (0, 0, 0)]
    ys = [i // a.size[0] for i, p in enumerate(pix) if p != (0, 0, 0)]
    left = [(x, y) for x, y in zip(xs, ys) if x < (bbox[0] + bbox[2]) // 2]
    right = [(x, y) for x, y in zip(xs, ys) if x >= (bbox[0] + bbox[2]) // 2]
    for name, c in (("left cluster", left), ("right cluster", right)):
        if c:
            print(f"  {name}: {len(c)} px, x {min(p[0] for p in c)}..{max(p[0] for p in c)}, "
                  f"y {min(p[1] for p in c)}..{max(p[1] for p in c)}")
