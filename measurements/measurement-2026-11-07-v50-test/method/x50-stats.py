#!/usr/bin/env python3
"""Per x50 case (names Y*, Z*, W*, V*) of a VDPROBE.BIN: distinct colours and pixels differing from the case's own backdrop corner colour."""
import struct, sys, collections
d = open(sys.argv[1], 'rb').read()
p = 6
rows = []
while p < len(d):
    assert d[p:p+1] == b'C'
    no = struct.unpack('>H', d[p+1:p+3])[0]; name = d[p+3:p+11].rstrip(b'\0').decode()
    x, y, w, h = struct.unpack('>4H', d[p+11:p+19]); n = w*h
    px = struct.unpack('>%dH' % n, d[p+19:p+19+2*n]); p += 19 + 2*n
    if no > 933 and name[:1] in 'YZWV':
        c = collections.Counter(px)
        rows.append((no, name, len(c), sum(v for k, v in c.items() if k != px[0])))
print("cases", len(rows))
for g in 'YZWV':
    r = [t for t in rows if t[1][0] == g]
    print(g, "cases", len(r), "min distinct colours", min(t[2] for t in r), "max", max(t[2] for t in r),
          "cases with no pixel != corner colour:", [t[1] for t in r if t[3] == 0][:8])
# second view: number of cases that changed at least 20 pixels against the case's own backdrop (the two backdrop
# colours are the pens 3 and 6/2 of x50_backdrop; "changed" = pixels of any other colour or of the colour 0 (black, pen 1))
d = open(sys.argv[1], 'rb').read(); p = 6; out = collections.defaultdict(list)
while p < len(d):
    no = struct.unpack('>H', d[p+1:p+3])[0]; name = d[p+3:p+11].rstrip(b'\0').decode()
    x, y, w, h = struct.unpack('>4H', d[p+11:p+19]); n = w*h
    px = struct.unpack('>%dH' % n, d[p+19:p+19+2*n]); p += 19 + 2*n
    if no > 933 and name[:1] in 'YZWV':
        c = collections.Counter(px); top2 = [k for k, _ in c.most_common(2)]
        out[name[0]].append((name, n - sum(c[k] for k in top2), c[0]))
for g, l in sorted(out.items()):
    print(g, "cases with <20 pixels outside the two most common colours:", [t[0] for t in l if t[1] < 20][:12], "of", len(l))
