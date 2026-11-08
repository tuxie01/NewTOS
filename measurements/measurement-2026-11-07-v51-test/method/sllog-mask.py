#!/usr/bin/env python3
"""V51: compare the SLPROBE.LOG pairs with the time-dependent fields masked.

sllog-mask.py <runs-dir>

Masks the tick stamps (t=N), the running event counter that starts a line (X/R/V/T + digits) and the repeat
count of a held arrow ("X... N more equal"); prints raw and masked difference counts per pair and every line
that still differs after masking.
"""
import re, sys

runs = sys.argv[1]

def norm(l):
    l = re.sub(r't=\d+', 't=#', l)
    l = re.sub(r'^([XRVT])\d+', r'\1#', l)
    l = re.sub(r'^X# \d+ more', 'X# # more', l)
    return l

for a, b in [('sl-base', 'sl-base2'), ('sl-chg', 'sl-chg2'), ('sl-base', 'sl-chg'), ('sl-base2', 'sl-chg2')]:
    A = open(f'{runs}/{a}/SLPROBE.LOG', errors='replace').read().splitlines()
    B = open(f'{runs}/{b}/SLPROBE.LOG', errors='replace').read().splitlines()
    raw = sum(1 for x, y in zip(A, B) if x != y)
    left = [(x, y) for x, y in zip(A, B) if norm(x) != norm(y)]
    print(f'{a} vs {b}: {len(A)}/{len(B)} lines, raw {raw} differ, masked {len(left)} differ')
    for x, y in left:
        print('   ', x, '|', y)
