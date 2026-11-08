import sys
from PIL import Image
from collections import Counter
S=sys.argv[1]
for g in ("480","1080"):
    for t in ("00-e0-topleft","01-e1-botright","02-e2-topright","03-e3-botleft","04-e4-bottom-slide"):
        im=Image.open(f"{S}/r/gui/edge{g}-chg/shots/{t}.png").convert("RGB"); w,h=im.size; px=im.load()
        def cell(x0,y0):
            return dict(Counter(px[x,y] for x in range(max(x0,0),min(x0+16,w)) for y in range(max(y0,0),min(y0+16,h))))
        reg={"00-e0-topleft":(0,0),"01-e1-botright":(w-16,h-16),"02-e2-topright":(w-16,0),"03-e3-botleft":(0,h-16),"04-e4-bottom-slide":(None,None)}[t]
        if reg[0] is None:
            # bottom row: find non-green pixels in last 2 rows
            row=[x for x in range(w) if px[x,h-1]!=px[w//2,h-3] ]
            print(g,t,im.size,"bottom-row non-bg px:",len(row),row[:8]); continue
        print(g,t,im.size,cell(*reg))
