#!/usr/bin/env python3
"""compare-xgscr.py <xgscr.pgm from the Mac> <screendump.ppm from QEMU> <diff.png> [ymax]: pixel compare, black = set, in y < ymax."""
import sys
def read(p):
    b = open(p, 'rb').read()
    toks = []; i = 0
    while len(toks) < 4:
        while b[i] in b' \t\r\n': i += 1
        j = i
        while b[j] not in b' \t\r\n': j += 1
        toks.append(b[i:j]); i = j
    i += 1
    return toks[0].decode(), int(toks[1]), int(toks[2]), b[i:]
tf, w, h, a = read(sys.argv[1]); qf, w2, h2, q = read(sys.argv[2])
ymax = int(sys.argv[4]) if len(sys.argv) > 4 else h
assert (w, h) == (w2, h2), (w, h, w2, h2)
step = 3 if qf == 'P6' else 1
diff = 0; black_a = black_q = 0; other = {}; first = None
out = bytearray(b'\xff' * (w * h))
for y in range(h):
    for x in range(w):
        pa = a[y * w + x] == 0
        rgb = q[(y * w + x) * step:(y * w + x) * step + step]
        pq = rgb == b'\0' * step
        other[bytes(rgb)] = other.get(bytes(rgb), 0) + 1
        if y < ymax:
            black_a += pa; black_q += pq
            if pa != pq:
                diff += 1; out[y * w + x] = 0
                if first is None: first = (x, y)
print(f"size {w}x{h}, compared y<{ymax}: set pixels Mac {black_a}, NewTOS {black_q}, differing {diff}, first {first}")
print("NewTOS colours:", {k.hex(): v for k, v in sorted(other.items(), key=lambda kv: -kv[1])[:6]})
open(sys.argv[3], 'wb').write(b'P5 %d %d 255\n' % (w, h) + bytes(out))
