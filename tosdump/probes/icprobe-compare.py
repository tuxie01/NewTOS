#!/usr/bin/env python3
"""icprobe-compare.py <emutos-out> <newtos-out> [--png DIR] -- compare two runs of
tosdump/probes/icprobe.c (TASK/A141) case by case.

Reads ICPROBE.BIN (or ICPROBE.BIN.gz) of both out-dirs (format: icprobe.c header) and prints one line per
case: EQUAL, or DIFF with the number of differing words and the first differing
coordinate with both words.  Exit 0 only if every case of both sides is EQUAL.

With --png (needs Pillow), writes <DIR>/<case>-E.png, -N.png for the screen cases T* and
the device-form icons I*/S*, reading each word as RGB565.
"""
import gzip
import os
import struct
import sys


def load(path):
    f = os.path.join(path, "ICPROBE.BIN")
    b = gzip.open(f + ".gz").read() if os.path.exists(f + ".gz") else open(f, "rb").read()
    assert b[:4] == b"ICPR", path
    off = 6
    cases = {}
    while off < len(b):
        assert b[off:off + 1] == b"C", (path, off)
        name = b[off + 3:off + 11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off + 11:off + 19])
        off += 19
        words = struct.unpack(">%dH" % (w * h), b[off:off + 2 * w * h])
        off += 2 * w * h
        cases[name] = (x, y, w, h, words)
    return cases


def write_png(path, w, h, words):
    from PIL import Image

    im = Image.new("RGB", (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            v = words[y * w + x]
            px[x, y] = (((v >> 11) & 31) * 255 // 31, ((v >> 5) & 63) * 255 // 63, (v & 31) * 255 // 31)
    im.save(path)


def main():
    args = sys.argv[1:]
    png = None
    if "--png" in args:
        i = args.index("--png")
        png = args[i + 1]
        del args[i:i + 2]
        os.makedirs(png, exist_ok=True)
    if len(args) != 2:
        print(__doc__)
        return 2
    e, n = load(args[0]), load(args[1])
    bad = 0
    for name in list(e) + [k for k in n if k not in e]:
        if name not in e or name not in n:
            print("%-8s only on %s" % (name, "EmuTOS" if name in e else "NewTOS"))
            bad += 1
            continue
        ce, cn = e[name], n[name]
        if ce[:4] != cn[:4]:
            print("%-8s geometry E %s N %s" % (name, ce[:4], cn[:4]))
            bad += 1
            continue
        _, _, w, h, we = ce
        wn = cn[4]
        diffs = [i for i in range(len(we)) if we[i] != wn[i]]
        if diffs:
            bad += 1
            i = diffs[0]
            print("%-8s DIFF %d/%d first at col %d row %d: E %04x N %04x" % (name, len(diffs), len(we), i % w, i // w, we[i], wn[i]))
        else:
            print("%-8s EQUAL (%d words)" % (name, len(we)))
        if png and name[0] in "TIS":
            write_png(os.path.join(png, name + "-E.png"), w, h, we)
            write_png(os.path.join(png, name + "-N.png"), w, h, wn)
    print("cases %d, differing %d" % (len(set(e) | set(n)), bad))
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
