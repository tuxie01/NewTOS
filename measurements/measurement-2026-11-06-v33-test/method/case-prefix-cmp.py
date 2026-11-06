#!/usr/bin/env python3
"""case-prefix-cmp.py A-dir B-dir : compare the cases of two VDPROBE.BIN by name (common ones), raw words.
Reports cases only in one, and common cases whose words differ."""
import struct, sys, os
def load(path):
    b = open(os.path.join(path, "VDPROBE.BIN"), "rb").read()
    off = 6; d = {}; order = []
    while off < len(b):
        no, = struct.unpack(">H", b[off+1:off+3]); name = b[off+3:off+11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off+11:off+19]); off += 19
        d[(no, name)] = (x, y, w, h, b[off:off+2*w*h]); off += 2*w*h; order.append((no, name))
    return d, order
a, oa = load(sys.argv[1]); b, ob = load(sys.argv[2])
print("cases A", len(oa), "B", len(ob))
print("only in A:", [k for k in oa if k not in b]); print("only in B:", [k[1] for k in ob if k not in a][:60])
diff = [k for k in oa if k in b and a[k] != b[k]]
print("common", sum(1 for k in oa if k in b), "differ", len(diff), [k[1] for k in diff])
for k in diff:
    x, y, w, h, wa = a[k]; wb = b[k][4]
    n = [i for i in range(w*h) if wa[2*i:2*i+2] != wb[2*i:2*i+2]]
    print(k, "px", len(n), "first", [(x+i % w, y+i//w, wa[2*i:2*i+2].hex(), wb[2*i:2*i+2].hex()) for i in n[:4]])
