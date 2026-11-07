#!/usr/bin/env python3
"""sheet.py <out.png> <cols> <file[:label]> ... -- contact sheet (half size, label burned in above each tile)."""
import sys
from PIL import Image, ImageDraw
out, cols = sys.argv[1], int(sys.argv[2]); items = sys.argv[3:]
W, H = 320, 240; L = 14; rows = (len(items) + cols - 1) // cols
sh = Image.new('RGB', (cols * (W + 4), rows * (H + L + 4)), (90, 90, 90)); d = ImageDraw.Draw(sh)
for i, it in enumerate(items):
    f, _, lab = it.partition(':')
    x, y = (i % cols) * (W + 4), (i // cols) * (H + L + 4)
    sh.paste(Image.open(f).convert('RGB').crop((0, 0, 640, 480)).resize((W, H)), (x, y + L)); d.text((x + 2, y + 1), lab or f[-20:], fill=(255, 255, 0))
sh.save(out)
