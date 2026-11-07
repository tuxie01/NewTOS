#!/usr/bin/env python3
"""V50: the AES records base vs change differ byte for byte (V49's did not); show where.

aes-bin-diff.py <worktree> <out-file>

Per table: tosdiff base vs change with the table (address cells are compared as addresses, not values),
and the raw byte differences grouped into 8-byte little-endian words (the format-3 pointer cell) with
both values and their difference.
"""
import os, subprocess, sys

root, out = sys.argv[1], sys.argv[2]
runs = os.path.join(root, "measurements/measurement-2026-11-07-v50/runs")
tosdiff = os.path.join(root, "tosdump/host/build/tosdiff")
lines = []
for t, tab in [("r2", "aes-r2-wind"), ("r3", "aes-r3-wind-state"), ("r4", "aes-r4-subfunctions")]:
    a = os.path.join(runs, f"tab-base-{t}/AES.BIN")
    b = os.path.join(runs, f"tab-chg-{t}/AES.BIN")
    r = subprocess.run([tosdiff, os.path.join(root, f"tosdump/cases/{tab}.tsv"), a, b],
                       capture_output=True, text=True)
    lines.append(f"== {t} ({tab}.tsv): tosdiff base vs chg exit {r.returncode}: "
                 + next((l for l in r.stdout.splitlines() if "cases differ" in l), "?"))
    x, y = open(a, "rb").read(), open(b, "rb").read()
    lines.append(f"   sizes {len(x)} / {len(y)}")
    offs = sorted({i - i % 8 for i in range(min(len(x), len(y))) if x[i] != y[i]})
    deltas = {}
    for o in offs:
        va = int.from_bytes(x[o:o + 8], "little")
        vb = int.from_bytes(y[o:o + 8], "little")
        deltas.setdefault(vb - va, []).append((o, va, vb))
    lines.append(f"   {len(offs)} differing 8-byte words")
    for d, l in sorted(deltas.items()):
        o, va, vb = l[0]
        lines.append(f"   delta {d:+#x}: {len(l)} words, first at offset {o:#x}: {va:#018x} -> {vb:#018x}")
open(out, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
