#!/usr/bin/env python3
"""icprobe-compare.py <emutos-out> <newtos-out> [--png DIR] -- compare two runs of
tosdump/probes/icprobe.c (TASK/A141) case by case.

Reads ICPROBE.BIN (or ICPROBE.BIN.gz) of both out-dirs (format: icprobe.c header) and prints one line per
case: EQUAL, or DIFF with the number of differing words and the first differing
coordinate with both words.  Exit 0 only if every case of both sides is EQUAL.

With --png (needs Pillow), writes <DIR>/<case>-E.png, -N.png for the screen cases T* and
the device-form icons I*/S*, reading each word as RGB565 (a version-2 pixel as its 24 bits).

THE 32-PLANE SETTING (TASK/V69; docs/TOS-REFERENCE-DUMPER.md, "The wide variant
of icprobe.c"). A run at NewTOS' 32-plane setting writes file version 2 (header
bytes 4-5): the I, S and T cases hold two WORDs a pixel, high then low. Two runs
of the same version are compared word for word, as above. A version-1 run (16
planes: the oracle, or NewTOS at 16) against a version-2 run (the first argument
is then the 16-plane side, whichever it is) has no oracle on the 32 side, so the
check is the index rule of V67 I3 through the palette, not through bytes:

  * I and S: each 32-bit pixel must be q(c) = (c * 255 + 500) / 1000 per gun of
    the requested triple (the PENS case, flag 0, of the 32-plane run) of a pen
    whose 16-bit word -- (f(r) << 11) | (f(g) << 6) | f(b), f(c) = (c * 31 +
    500) / 1000, EmuTOS set_color16 (references/emutos/vdi/vdi_col.c:588-613) --
    is the word recorded at the same position in the 16-plane run (set
    membership, no tolerance: the default pens are exactly representable, V64
    P7). The overlay bit 0x0020 of the 16-plane word is not part of the colour.
  * T (the drawn tree read back): the same membership rule; a pixel that is no
    pen's (an icon pixel ANDed into a coloured background) is checked by the
    weaker rule V67 measured, the equality of the 5-bit projection (the top 5
    bits of each 8-bit gun against the 16-plane word's 5-bit guns), and counted.

Such a comparison prints MEMBER (every pixel a pen's), PROJECTION (n pixels only
by the projection) or DIFF per case and "NewTOS contract (ADR-038), no oracle".
"""
import gzip
import os
import struct
import sys


def load(path):
    f = os.path.join(path, "ICPROBE.BIN")
    b = gzip.open(f + ".gz").read() if os.path.exists(f + ".gz") else open(f, "rb").read()
    assert b[:4] == b"ICPR", path
    version = struct.unpack(">H", b[4:6])[0]
    assert version in (1, 2), (path, version)
    off = 6
    cases = {}
    while off < len(b):
        assert b[off:off + 1] == b"C", (path, off)
        name = b[off + 3:off + 11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off + 11:off + 19])
        off += 19
        # version 2: the pixel cases carry two WORDs a pixel, high then low
        n = w * h * (2 if version == 2 and name[0] in "IST" else 1)
        words = struct.unpack(">%dH" % n, b[off:off + 2 * n])
        off += 2 * n
        cases[name] = (x, y, w, h, words)
    return version, cases


def pixels32(words):
    """The 32-bit pixels of a version-2 pixel case."""
    return [(words[2 * i] << 16) | words[2 * i + 1] for i in range(len(words) // 2)]


def write_png(path, w, h, words, version=1):
    from PIL import Image

    im = Image.new("RGB", (w, h))
    px = im.load()
    pix = pixels32(words) if version == 2 else None
    for y in range(h):
        for x in range(w):
            if pix is not None:
                v = pix[y * w + x]
                px[x, y] = ((v >> 16) & 255, (v >> 8) & 255, v & 255)
            else:
                v = words[y * w + x]
                px[x, y] = (((v >> 11) & 31) * 255 // 31, ((v >> 5) & 63) * 255 // 63, (v & 31) * 255 // 31)
    im.save(path)


def q8(c):
    """V64 P2: a gun in permille to 8 bits."""
    return (c * 255 + 500) // 1000


def f5(c):
    """vdi2fivebits (references/emutos/vdi/vdi_col.c:588-591)."""
    return (c * 31 + 500) // 1000


def pen_sets(pens_words):
    """From a PENS case (256 rows: requested R, G, B, realised R, G, B): the
    16-bit word (overlay bit clear) -> the set of 32-bit pixels of the pens
    that have it."""
    out = {}
    for pen in range(256):
        r, g, b = pens_words[pen * 6:pen * 6 + 3]
        w16 = (f5(r) << 11) | (f5(g) << 6) | f5(b)
        out.setdefault(w16, set()).add((q8(r) << 16) | (q8(g) << 8) | q8(b))
    return out


def projection_equal(p32, w16):
    r, g, b = (p32 >> 16) & 255, (p32 >> 8) & 255, p32 & 255
    return (r >> 3, g >> 3, b >> 3) == ((w16 >> 11) & 31, (w16 >> 6) & 31, w16 & 31)


def compare_16_32(e, n, png):
    """A version-1 run e against a version-2 run n (see the module doc)."""
    print("NewTOS contract (ADR-038, 32-plane setting), no oracle: the 32-plane run checked against the 16-plane "
          "run through the palette (V67 I3)")
    if "PENS" not in n:
        print("PENS   missing on the 32-plane side")
        return 1
    members = pen_sets(n["PENS"][4])
    bad = 0
    for name in list(e) + [k for k in n if k not in e]:
        if name not in e or name not in n:
            print("%-8s only on the %s run" % (name, "16-plane" if name in e else "32-plane"))
            bad += 1
            continue
        ce, cn = e[name], n[name]
        if ce[:4] != cn[:4]:
            print("%-8s geometry 16 %s 32 %s" % (name, ce[:4], cn[:4]))
            bad += 1
            continue
        _, _, w, h, we = ce
        if name == "PENS":
            # the requested triples are the same at both depths (V64 P4);
            # the realised one is the 8-bit gun back in permille at 32 (P5)
            wn = cn[4]
            fail = [p for p in range(256)
                    if we[p * 6:p * 6 + 3] != wn[p * 6:p * 6 + 3]
                    or tuple(wn[p * 6 + 3:p * 6 + 6]) != tuple((q8(c) * 1000 + 127) // 255 for c in wn[p * 6:p * 6 + 3])]
            if fail:
                bad += 1
                print("PENS     DIFF %d/256 pens: requested not equal, or realised at 32 not (q * 1000 + 127) / 255 "
                      "(first pen %d)" % (len(fail), fail[0]))
            else:
                print("PENS     CONTRACT (requested equal; realised at 32 = (q * 1000 + 127) / 255, V64 P5)")
            continue
        if name[0] not in "IST":
            diffs = [i for i in range(len(we)) if we[i] != cn[4][i]]
            if diffs:
                bad += 1
                i = diffs[0]
                print("%-8s DIFF %d/%d first at col %d row %d: 16 %04x 32 %04x"
                      % (name, len(diffs), len(we), i % w, i // w, we[i], cn[4][i]))
            else:
                print("%-8s EQUAL (%d words)" % (name, len(we)))
            continue
        p32 = pixels32(cn[4])
        proj, fail = 0, []
        for i, (w16, p) in enumerate(zip(we, p32)):
            if (p & 0x00FFFFFF) in members.get(w16 & ~0x0020, ()):
                continue
            if name[0] == "T" and projection_equal(p & 0x00FFFFFF, w16):
                proj += 1
                continue
            fail.append(i)
        if fail:
            bad += 1
            i = fail[0]
            print("%-8s DIFF %d/%d pixels no pen's%s, first at col %d row %d: 16 %04x 32 %08x"
                  % (name, len(fail), len(we), " and not equal in the 5-bit projection" if name[0] == "T" else "",
                     i % w, i // w, we[i], p32[i]))
        elif proj:
            print("%-8s PROJECTION %d/%d pixels equal only in the 5-bit projection, the rest a pen's"
                  % (name, proj, len(we)))
        else:
            print("%-8s MEMBER (%d pixels, every one a pen's)" % (name, len(we)))
        if png:
            write_png(os.path.join(png, name + "-16.png"), w, h, we, 1)
            write_png(os.path.join(png, name + "-32.png"), w, h, cn[4], 2)
    print("cases %d, differing %d" % (len(set(e) | set(n)), bad))
    return 0 if bad == 0 else 1


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
    (ve, e), (vn, n) = load(args[0]), load(args[1])
    if (ve, vn) == (1, 2):
        return compare_16_32(e, n, png)
    if ve != vn:
        print("refusing: version %d against version %d -- give the 16-plane (version 1) run first" % (ve, vn))
        return 2
    if ve == 2:
        print("NewTOS contract (ADR-038, 32-plane setting), no oracle: two 32-plane runs, word for word")
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
            if ve == 2 and name[0] in "IST":
                print("%-8s DIFF %d/%d words first at pixel col %d row %d: E %04x N %04x"
                      % (name, len(diffs), len(we), (i // 2) % w, (i // 2) // w, we[i], wn[i]))
            else:
                print("%-8s DIFF %d/%d first at col %d row %d: E %04x N %04x" % (name, len(diffs), len(we), i % w, i // w, we[i], wn[i]))
        else:
            print("%-8s EQUAL (%d words)" % (name, len(we)))
        if png and name[0] in "TIS":
            write_png(os.path.join(png, name + "-E.png"), w, h, we, ve)
            write_png(os.path.join(png, name + "-N.png"), w, h, wn, vn)
    print("cases %d, differing %d" % (len(set(e) | set(n)), bad))
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
