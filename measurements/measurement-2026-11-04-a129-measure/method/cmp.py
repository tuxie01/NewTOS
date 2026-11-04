#!/usr/bin/env python3
# cmp.py <a.png> ax ay <b.png> bx by w h -- classify each pixel (B <64, L 64-239 grey, W >=240; a
# colour with 16-bit rounding stays in its class) and list the differing positions (relative to the origin).
import sys
from PIL import Image
a=Image.open(sys.argv[1]).convert("L"); ax,ay=int(sys.argv[2]),int(sys.argv[3])
b=Image.open(sys.argv[4]).convert("L"); bx,by=int(sys.argv[5]),int(sys.argv[6])
w,h=int(sys.argv[7]),int(sys.argv[8])
def cl(v): return "B" if v<64 else ("W" if v>=240 else ("1" if v>=170 else "2"))
d=[]
for y in range(h):
    for x in range(w):
        ca=cl(a.getpixel((ax+x,ay+y))); cb=cl(b.getpixel((bx+x,by+y)))
        if ca!=cb: d.append((x,y,ca,cb))
print("region %dx%d, differing pixels: %d" % (w,h,len(d)))
cols={}; rows={}
for x,y,ca,cb in d: cols[x]=cols.get(x,0)+1; rows[y]=rows.get(y,0)+1
print("columns with diffs:", sorted(cols.items())[:40]); print("rows with diffs:", sorted(rows.items())[:60])
