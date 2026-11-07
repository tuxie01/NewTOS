#!/usr/bin/env python3
"""TASK/A141 question 3, does A142 have the same root? cflib's dialog images, as they sit in its C source.

usage: a142-inline-images.py <inline.rsh> <QED.NAP> <out.png>

sdk/cflib/overlay/inline.rsh declares `static _UBYTE inline_IMAGEn[] = { 0x.., ... }` (ORCS output, m68k byte order) and
cflib/userdef.c passes them on as `short *` to vrt_cpyfm. Prints each array's length and whether its bytes stand unchanged in
the linked program, and draws each mono image twice x8: left the bytes taken big-endian (as on the m68k), right as a
little-endian machine reads the same bytes as WORDs. Which WORD width an image has comes from the rs_bitblk table.
"""
import re
import sys

from PIL import Image

src = open(sys.argv[1], "r", encoding="latin-1").read()
nap = open(sys.argv[2], "rb").read()
arrays = {}
for m in re.finditer(r"static _UBYTE (inline_IMAGE\d+)\[\]\s*=\s*\{(.*?)\};", src, re.S):
    arrays[m.group(1)] = bytes(int(x, 16) for x in re.findall(r"0x[0-9A-Fa-f]{2}", m.group(2)))
bitblk = re.findall(r"\{\s*CP?\s*\(?_?\w*\)?\s*(inline_IMAGE\d+)\s*,\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\s*\}", src)
if not bitblk:
    # the plain ORCS form: { inline_IMAGE0, wb, hl, x, y, col }
    bitblk = re.findall(r"(inline_IMAGE\d+)[^,]*,\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)", src)
print("arrays:", {k: len(v) for k, v in arrays.items()})
print("rs_bitblk:", bitblk)
for k, v in arrays.items():
    print("%s bytes %d, found unchanged in the program at file offset %s" % (k, len(v), nap.find(v)))

SC = 8
rows = [(n, int(wb), int(hl)) for n, wb, hl, *_ in bitblk]
W = 32 * SC * 2 + 60
H = sum(hl * SC + 12 for _, _, hl in rows) + 12
sheet = Image.new("RGB", (W, H), (186, 186, 186))
y0 = 8
for name, wb, hl in rows:
    data = arrays[name]
    for side, swap in ((0, False), (1, True)):
        im = Image.new("RGB", (wb * 8, hl), (255, 255, 255))
        px = im.load()
        for y in range(hl):
            for bx in range(wb):
                b = data[y * wb + (bx ^ 1 if swap else bx)]
                for i in range(8):
                    if b & (0x80 >> i):
                        px[bx * 8 + i, y] = (0, 0, 0)
        sheet.paste(im.resize((wb * 8 * SC, hl * SC), Image.NEAREST), (8 + side * (32 * SC + 20), y0))
    y0 += hl * SC + 12
sheet.save(sys.argv[3])
