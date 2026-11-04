#!/usr/bin/env python3
"""contact.py OUT.png IN.png... : contact sheet (3 columns, half size, label = file stem)."""
import sys
from PIL import Image, ImageDraw
out, ins = sys.argv[1], sys.argv[2:]
w, h, cols = 320, 240, 3
rows = (len(ins) + cols - 1) // cols
sheet = Image.new("RGB", (w * cols, h * rows), "white")
for i, p in enumerate(ins):
    im = Image.open(p).convert("RGB").resize((w, h), Image.NEAREST)
    ImageDraw.Draw(im).text((4, h - 12), p.rsplit("/", 1)[-1][:-4], fill=(255, 0, 0))
    sheet.paste(im, ((i % cols) * w, (i // cols) * h))
sheet.save(out)
