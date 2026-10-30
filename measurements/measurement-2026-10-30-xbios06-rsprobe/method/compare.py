#!/usr/bin/env python3
"""compare.py -- field-by-field comparison of three RSPROBE.LOGs (TASK/XBIOS06).

usage: compare.py TT.LOG FALCONTC.LOG NEWTOS.LOG

For every step line (P.., S.., Z01) the fields ret hi q qhi g ghi ib if iu idm
iw of the three sides; per line the fields in which NewTOS differs from tt,
from falcontc, and in which tt differs from falcontc. Other lines (L00,
P17 args, P18, Z02) are compared as whole text. wr5 (iw) is printed and
compared like the rest; docs/api/structures/iorec.md classes it IGNORE.
"""
import re
import sys

FIELDS = ["ret", "hi", "q", "qhi", "g", "ghi", "ib", "if", "iu", "idm", "iw"]

def load(p):
    d, order = {}, []
    for l in open(p, encoding="ascii"):
        l = l.rstrip("\n")
        if not l:
            continue
        key = l.split(" ", 1)[0]
        if key == "P17" and l.startswith("P17 args"):
            key = "P17args"
        kv = dict(re.findall(r"\b(\w+)=([0-9a-f]+)\b", l)) if " ret=" in l else None
        d[key] = (l, kv)
        order.append(key)
    return d, order

tt, order = load(sys.argv[1])
fc, _ = load(sys.argv[2])
nt, _ = load(sys.argv[3])

def diff(a, b):
    if a[1] is None or b[1] is None:
        return [] if a[0] == b[0] else ["text"]
    return [f for f in FIELDS if a[1].get(f) != b[1].get(f)]

print(f"{'line':5} {'NT!=tt':22} {'NT!=fc':26} {'tt!=fc':22}")
for k in order:
    a, b, c = tt.get(k), fc.get(k), nt.get(k)
    if a is None or b is None or c is None:
        print(f"{k:5} MISSING on a side")
        continue
    x, y, z = diff(c, a), diff(c, b), diff(a, b)
    print(f"{k:5} {','.join(x) or '-':22} {','.join(y) or '-':26} {','.join(z) or '-':22}")
print()
print("values (tt / falcontc / NewTOS), fields that differ on any side:")
for k in order:
    a, b, c = tt[k], fc[k], nt[k]
    if a[1] is None:
        if not (a[0] == b[0] == c[0]):
            print(f"{k:5} tt: {a[0]}\n      fc: {b[0]}\n      nt: {c[0]}")
        continue
    fs = [f for f in FIELDS if not (a[1].get(f) == b[1].get(f) == c[1].get(f))]
    if fs:
        print(f"{k:5} " + "  ".join(f"{f}={a[1][f]}/{b[1][f]}/{c[1][f]}" for f in fs))
