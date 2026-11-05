#!/usr/bin/env python3
"""phasetable.py <compare.txt>... -- per gesture (cfg, axis, gesture) the differing pixels outside the
baseline mask (ALL column of compare.py) for every phase, max over the given runs; '=' for 0."""
import collections
import re
import sys

tab = collections.OrderedDict()
for path in sys.argv[1:]:
    for line in open(path):
        m = re.match(r"\d+-\d+-(1000|300|min)-([VH-])-(\w+)-(\w+)\s+\|\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)", line)
        if not m:
            continue
        key = (m.group(1), m.group(2), m.group(3))
        ph = m.group(4)
        v = int(m.group(8))
        d = tab.setdefault(key, collections.OrderedDict())
        d[ph] = max(d.get(ph, 0), v)
for (c, a, g), d in tab.items():
    print(f"{c:4s} {a} {g:9s} " + " ".join(f"{p}={v if v else '='}" for p, v in d.items()))
