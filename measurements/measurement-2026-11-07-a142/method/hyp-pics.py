#!/usr/bin/env python3
"""hyp-pics.py <file.hyp>... -- TASK/A142: how many PICTURE entries (type 3) has a .HYP, and which entry is it? Index format: sdk/overlays/hypview/hyp/hyp.c."""
import struct, sys
for p in sys.argv[1:]:
    d = open(p, "rb").read()
    if d[:4] != b"HDOC": print(p, "not HDOC"); continue
    size, num = struct.unpack(">IH", d[4:10])
    o = 12; pics = []; nodes = []
    for i in range(num):
        ln, ty = d[o], d[o + 1]
        name = d[o + 12:o + ln].split(b"\0")[0].decode("latin1")
        if ty == 3: pics.append(i)
        elif ty == 0: nodes.append((i, name))
        o += ln
    print(f"{p}: entries {num}, pictures {len(pics)}, first node {nodes[:1]}, first picture indices {pics[:5]}")
