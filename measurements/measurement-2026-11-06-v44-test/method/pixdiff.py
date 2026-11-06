import sys,glob,os
from PIL import Image, ImageChops
ra,rb=sys.argv[1:3]; nums=sys.argv[3:]
for n in nums:
    fa=glob.glob(f"{ra}/shots/*-{n}-*.png"); fb=glob.glob(f"{rb}/shots/*-{n}-*.png")
    if not fa or not fb: print(n,"missing"); continue
    a=Image.open(fa[0]).convert("RGB"); b=Image.open(fb[0]).convert("RGB")
    d=ImageChops.difference(a,b); bb=d.getbbox()
    cnt=sum(1 for p in d.getdata() if p!=(0,0,0)) if bb else 0
    print(os.path.basename(fa[0]), a.size, "diffpx",cnt,"bbox",bb)
