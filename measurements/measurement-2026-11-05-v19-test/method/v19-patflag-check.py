#!/usr/bin/env python3
"""v19-patflag-check.py <emutos-out> <newtos-out> -- V19 acceptance item 3.
Reads VDPROBE.BIN of both (format: tosdump/probes/vdprobe.c) and for the appended
mem->mem records (names P<planes><geo><op>p / f) checks
  (a) within each side: the op|0x10 record (f) equals the plain-op record (p)
  (b) across the sides: every record equal (the whole form incl. 32 guard words each side).
Also prints how many of cases 1..54 differ (V18 baseline: 26)."""
import struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
def load(path):
    b = open(os.path.join(path, "VDPROBE.BIN"), "rb").read()
    off = 6; out = []
    while off < len(b):
        no, = struct.unpack(">H", b[off+1:off+3]); name = b[off+3:off+11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off+11:off+19]); off += 19
        out.append((no, name, b[off:off+2*w*h])); off += 2*w*h
    return out
e, n = load(sys.argv[1]), load(sys.argv[2])
assert [(a[0], a[1]) for a in e] == [(a[0], a[1]) for a in n]
old = [(a, b) for a, b in zip(e, n) if a[0] <= 54]
print("cases 1..54: %d differ of %d" % (sum(a[2] != b[2] for a, b in old), len(old)))
new = [(a, b) for a, b in zip(e, n) if a[0] > 54]
print("appended records: %d (%d p/f pairs)" % (len(new), len(new)//2))
cross = [a[1] for a, b in new if a[2] != b[2]]
print("(b) across sides differ: %d of %d" % (len(cross), len(new)), cross[:12])
for side, recs in (("EmuTOS", e), ("NewTOS", n)):
    d = {r[1]: r[2] for r in recs if r[0] > 54}
    bad = [k[:-1] for k in d if k.endswith("p") and d[k] != d[k[:-1] + "f"]]
    print("(a) %s: op|0x10 != plain in %d of %d pairs" % (side, len(bad), len(d)//2), bad[:12])
    # sanity: the copy changed the destination at all (plain op != guard-only) -- count distinct
    print("    %s: distinct dest images %d" % (side, len(set(d.values()))))
# guard check: first/last 64 bytes of every record
for side, recs in (("EmuTOS", e), ("NewTOS", n)):
    g = [r[1] for r in recs if r[0] > 54 and (r[2][:64] != b"\xa5"*64 or r[2][-64:] != b"\xa5"*64)]
    print("guards touched %s: %d" % (side, len(g)), g[:12])
sys.exit(1 if cross else 0)
