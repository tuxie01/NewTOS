#!/usr/bin/env python3
"""wkprobe-compare.py <emutos-out> <newtos-out> [--maps DIR] -- compare two runs of
tosdump/probes/wkprobe.c (TASK/V18 part 3) record by record.  A copy of vdprobe-compare.py
that reads WKPROBE.BIN and also compares WKPROBE.LOG line by line (lines starting with "N "
-- NewTOS' wait details -- and "T0" -- hz_200 timestamps -- left out).

Reads WKPROBE.BIN of both out-dirs (format: wkprobe.c header).  Per case it compares
the raw screen WORDs (both sides' words are the 16-bit Falcon truecolor screen word,
big-endian in the file) and, as a second view, the pen each word belongs to, by the
side's own calibration case CAL (pens 0..15; the v_clrwk background 0xFFFF counted as
pen 0 'W').  Prints one line per case: EQUAL, or DIFF with the number of differing
pixels and the first differing coordinates (screen x,y) with both words.  With --maps,
writes <DIR>/<case>.txt: the case's rectangle as characters, E = EmuTOS, N = NewTOS,
'.' white/background, '#' pen 1, digits/letters other pens, '?' an unknown word,
and a third map with '*' where the two differ.
"""
import struct
import sys
import os


def load(path):
    b = open(os.path.join(path, "WKPROBE.BIN"), "rb").read()
    assert b[:4] == b"VDPR", path
    off = 6
    cases = []
    while off < len(b):
        assert b[off:off + 1] == b"C", (path, off)
        no, = struct.unpack(">H", b[off + 1:off + 3])
        name = b[off + 3:off + 11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off + 11:off + 19])
        off += 19
        words = struct.unpack(">%dH" % (w * h), b[off:off + 2 * w * h])
        off += 2 * w * h
        cases.append((no, name, x, y, w, h, words))
    return cases


def penmap(cases):
    cal = [c for c in cases if c[1] == "CAL"][0]
    _, _, x, y, w, h, words = cal
    m = {}
    for p in range(16):
        m.setdefault(words[3 * w + 8 * p + 3], p)
    m.setdefault(words[3 * w + 130], 0)   # v_clrwk background = pen 0
    m.setdefault(0xFFFF, 0)
    return m


SYM = ".#23456789ABCDEF"


def sym(m, v):
    p = m.get(v)
    return "?" if p is None else SYM[p]


def main():
    a, b = sys.argv[1], sys.argv[2]
    maps = None
    if "--maps" in sys.argv:
        maps = sys.argv[sys.argv.index("--maps") + 1]
        os.makedirs(maps, exist_ok=True)
    ca, cb = load(a), load(b)
    ma, mb = penmap(ca), penmap(cb)
    print("pens EmuTOS:", " ".join("%d=%04x" % (p, v) for v, p in sorted(ma.items(), key=lambda t: t[1])))
    print("pens NewTOS:", " ".join("%d=%04x" % (p, v) for v, p in sorted(mb.items(), key=lambda t: t[1])))
    assert len(ca) == len(cb), "case count differs"
    ndiff = 0
    for (na, nm, x, y, w, h, wa), (nb, nm2, x2, y2, w2, h2, wb) in zip(ca, cb):
        assert (na, nm, x, y, w, h) == (nb, nm2, x2, y2, w2, h2), (nm, nm2)
        raw = [i for i in range(w * h) if wa[i] != wb[i]]
        pen = [i for i in raw if sym(ma, wa[i]) != sym(mb, wb[i]) or sym(ma, wa[i]) == "?"]
        if not raw:
            print("%02d %-5s EQUAL  (%dx%d at %d,%d)" % (na, nm, w, h, x, y))
        else:
            ndiff += 1
            first = ", ".join("(%d,%d) E=%04x N=%04x" % (x + i % w, y + i // w, wa[i], wb[i]) for i in raw[:6])
            print("%02d %-5s DIFF   %d px raw, %d px by pen (%dx%d at %d,%d): %s" %
                  (na, nm, len(raw), len(pen), w, h, x, y, first))
        if maps and raw:
            with open(os.path.join(maps, "%02d-%s.txt" % (na, nm)), "w") as f:
                f.write("%s  rect %d,%d %dx%d  (E = EmuTOS, N = NewTOS, D = differences)\n" % (nm, x, y, w, h))
                rows = sorted({i // w for i in raw})
                r0, r1 = max(0, rows[0] - 2), min(h - 1, rows[-1] + 2)
                cols = sorted({i % w for i in raw})
                c0, c1 = max(0, cols[0] - 4), min(w - 1, cols[-1] + 4)
                for lab, m, ws in (("E", ma, wa), ("N", mb, wb)):
                    f.write("%s x %d..%d\n" % (lab, x + c0, x + c1))
                    for r in range(r0, r1 + 1):
                        f.write("%4d %s\n" % (y + r, "".join(sym(m, ws[r * w + c]) for c in range(c0, c1 + 1))))
                f.write("D\n")
                for r in range(r0, r1 + 1):
                    f.write("%4d %s\n" % (y + r, "".join("*" if wa[r * w + c] != wb[r * w + c] else " "
                                                         for c in range(c0, c1 + 1))))
    print("cases %d, differing %d" % (len(ca), ndiff))
    la, lb = (logl(a), logl(b))
    nlog = 0
    for i in range(max(len(la), len(lb))):
        x = la[i] if i < len(la) else "<none>"
        y = lb[i] if i < len(lb) else "<none>"
        if x != y:
            nlog += 1
            print("LOG E: %s" % x)
            print("LOG N: %s" % y)
    print("log lines %d/%d, differing %d" % (len(la), len(lb), nlog))
    return 1 if ndiff or nlog else 0


def logl(path):
    lines = open(os.path.join(path, "WKPROBE.LOG"), "rb").read().decode("latin-1").replace("\r", "").split("\n")
    return [l for l in lines if l and not l.startswith("N ") and not l.startswith("T0")]


if __name__ == "__main__":
    sys.exit(main())
