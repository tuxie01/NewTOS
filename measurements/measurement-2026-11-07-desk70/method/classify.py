#!/usr/bin/env python3
"""classify.py <top-480-RGB PIL image> helpers: decide what a screen shows, the same rule for both sides.
 text_dark : dark pixels in the text zone x<232, y<84 of a screen whose lower 300 lines are all white (the text screen)
 state     : TEXT n (text screen with n dark pixels), WHITE (all white), DESKTOP (menu bar line + green), OTHER.
Run directly on an AVI (avi mode): classify.py avi <file.avi> <first> <last> prints one line per state change;
or on a dir of screen-NNN.png (dir mode): classify.py dir <dir>."""
import sys, io, struct, glob, os
from PIL import Image
def state(img):
    im = img.convert('RGB').crop((0, 0, 640, 480)); px = im.load()
    low = im.crop((0, 180, 640, 480)).getcolors(1 << 16) or []
    nwhite = sum(c for c, v in low if min(v) >= 200); total = 640 * 300
    if nwhite >= total - 80:                # text screen or white screen (a hourglass may sit anywhere)
        zone = im.crop((0, 0, 232, 84)).getcolors(1 << 16) or []
        dark = sum(c for c, v in zone if max(v) < 80)
        hgl = im.crop((232, 0, 640, 180)).getcolors(1 << 16) or []
        d2 = sum(c for c, v in hgl if max(v) < 80)
        if dark: return f'TEXT dark={dark}' + (' +other' if d2 else '')
        return 'WHITE' + (f' dark_elsewhere={d2}' if d2 else '')
    green = sum(c for c, v in (im.getcolors(1 << 16) or []) if v[1] > 150 and v[0] < 200 and v[2] < 200)
    return f'GREEN>{int(100*green/307200/5)*5}%'
if sys.argv[1] == 'avi':
    data = open(sys.argv[2], 'rb').read(); a, b = int(sys.argv[3]), int(sys.argv[4])
    m = data.find(b'movi'); p = m + 4; n = 0; t = 0; prev = None; end = len(data)
    while p + 8 <= end and n <= b:
        tag = data[p:p+4]; size = struct.unpack('<I', data[p+4:p+8])[0]
        if tag in (b'LIST', b'RIFF'): p += 12; continue
        if p + 8 + size > end: break
        if tag == b'00dc':
            if n >= a:
                s = state(Image.open(io.BytesIO(data[p+8:p+8+size])))
                if s != prev: print(f'frame {n:6d} t={t/176400:8.3f} s  {s}'); prev = s
            n += 1
        elif tag == b'01wb': t += size
        p += 8 + size
    print(f'frame {n:6d} t={t/176400:8.3f} s  (end of range)')
else:
    prev = None
    lines = {int(l.split()[0]): l.split() for l in open(sys.argv[2] + '/screens.txt') if l[:3].strip().isdigit()}
    for f in sorted(glob.glob(sys.argv[2] + '/screen-*.png')):
        i = int(os.path.basename(f)[7:10]); s = state(Image.open(f))
        print(f'screen {i:3d} first={lines[i][1]} last={lines[i][2]} {s}')
