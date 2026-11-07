#!/usr/bin/env python3
"""TASK/A141: which palette were the 8-plane CICONs of a classic .rsc drawn for?

usage: palette-test.py <classic.rsc> <ICPROBE.BIN of the oracle> <out-dir> [<name>=<file.pal> ...]

Decodes each CICON's 4-plane and 8-plane variant by hand from the file (mask bit, plane p = bit p, the classic
plane-sequential standard form, no NewTOS code except tools/mkrsc.py's read_cicon_table), and scores each palette:
mean RGB distance (0..1000 per channel) between the 8-plane pixel in that palette and the 4-plane pixel of the same
place under the default 16-colour registers. The 4-plane variant is the file's own statement of what a pixel is meant
to be; the score is relative (a 4-plane picture is a coarse reduction), so only the ranking is a result.
A palette is 1540 bytes: "PA01" + 256 RGB triples of big-endian WORDs 0..1000 (references/freemint/xaaes/src.km/trnfm.c
rw_syspalette). Writes one sheet per palette, icons x6.
"""
import importlib.util
import os
import struct
import sys

from PIL import Image

spec = importlib.util.spec_from_file_location("mkrsc", "/Volumes/Daten/Development/PiRusTOS/tools/mkrsc.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

RSC, BIN, OUT = sys.argv[1:4]
d = open(RSC, "rb").read()
hdr = struct.unpack(">18H", d[:36])
blocks = m.read_cicon_table(d, hdr[0], hdr[17])
os.makedirs(OUT, exist_ok=True)


def rd(off, n):
    return struct.unpack(">%dH" % n, d[off:off + 2 * n])


def idx(off, planes, w, h):
    wd = w // 16
    n = wd * h
    words = rd(off, n * planes)
    out = []
    for y in range(h):
        for x in range(w):
            wi = y * wd + x // 16
            bit = 0x8000 >> (x & 15)
            v = 0
            for p in range(planes):
                if words[p * n + wi] & bit:
                    v |= 1 << p
            out.append(v)
    return out


# The oracle's pens (vq_color requested), and its pen -> register map (references/emutos/vdi/vdi_col.c:736-747, 256 colours).
b = open(BIN, "rb").read()
off = 6 + 19
pens = [struct.unpack(">6H", b[off + i * 12:off + i * 12 + 12])[:3] for i in range(256)]
MAP = [0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13]
mapc = list(range(256))
mapc[:16] = MAP
mapc[1] = 255
mapc[255] = 15
rev = [0] * 256
for pen, reg in enumerate(mapc):
    rev[reg] = pen
oracle_regs = [pens[rev[n]] for n in range(256)]
# 16-colour registers for the 4-plane variant: MAP_COL[1] = 15 (references/emutos/vdi/vdi_col.c:737)
map16 = list(MAP)
rev16 = [0] * 16
for pen, reg in enumerate(map16):
    rev16[reg] = pen
regs4 = [pens[rev16[n]] for n in range(16)]

palettes = {"emutos-default": oracle_regs}
for a in sys.argv[4:]:
    name, f = a.split("=", 1)
    p = open(f, "rb").read()
    assert p[:4] == b"PA01" and len(p) == 4 + 256 * 6, f
    raw = [struct.unpack(">3H", p[4 + i * 6:10 + i * 6]) for i in range(256)]
    palettes[name + "-index-is-entry"] = raw          # index n reads entry n
    palettes[name + "-reg-to-pen"] = [raw[rev[n]] for n in range(256)]  # index n is a register, the file is by VDI pen
    # What the file's picture becomes when each index is replaced by the nearest colour of the default registers
    # (the idea of XaAES build_pal_xref, references/freemint/xaaes/src.km/trnfm.c:592): a recoloured 8-plane variant.
    def near(c):
        return min(oracle_regs, key=lambda r: sum((r[j] - c[j]) ** 2 for j in range(3)))
    palettes[name + "-recoloured-to-default"] = [near(raw[rev[n]]) for n in range(256)]

icons = [i for i in range(len(blocks)) if {x[0] for x in blocks[i][4]} >= {4, 8}]
SC = 6
print("icons with both variants:", len(icons))
print("%-20s %10s %10s" % ("palette", "mean dist", "pixels"))
for name, regs in palettes.items():
    S = Image.new("RGB", ((32 * SC + 8) * 7, (21 * SC + 8) * 2), (186, 186, 186))
    total = count = 0
    for k, i in enumerate(icons):
        w_, _, _, _, v = blocks[i]
        W, H = w_[5], w_[6]
        v4 = [x for x in v if x[0] == 4][0]
        v8 = [x for x in v if x[0] == 8][0]
        i4, i8 = idx(v4[1], 4, W, H), idx(v8[1], 8, W, H)
        m4, m8 = idx(v4[2], 1, W, H), idx(v8[2], 1, W, H)
        im = Image.new("RGB", (W, H), (186, 186, 186))
        px = im.load()
        for y in range(H):
            for x in range(W):
                t = y * W + x
                if m8[t]:
                    c = regs[i8[t]]
                    px[x, y] = tuple(int(q * 255 / 1000) for q in c)
                    if m4[t]:
                        r = regs4[i4[t]]
                        total += sum(abs(c[j] - r[j]) for j in range(3)) / 3
                        count += 1
        S.paste(im.resize((W * SC, H * SC), Image.NEAREST), ((k % 7) * (32 * SC + 8) + 4, (k // 7) * (21 * SC + 8) + 4))
    print("%-20s %10.1f %10d" % (name, total / count, count))
    S.save(os.path.join(OUT, "sheet-%s.png" % name))
