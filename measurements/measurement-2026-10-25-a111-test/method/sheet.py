import sys
from PIL import Image, ImageDraw
out, box, scale = sys.argv[1], tuple(map(int, sys.argv[2].split(","))), int(sys.argv[3]); files = sys.argv[4:]
crops = [Image.open(f).convert("RGB").crop(box).resize(((box[2]-box[0])*scale, (box[3]-box[1])*scale), Image.NEAREST) for f in files]
w, h = crops[0].size; cols = 2 if len(crops) > 3 else 1
rows = (len(crops)+cols-1)//cols
sh = Image.new("RGB", (cols*w, rows*(h+14)), (255,255,255)); d = ImageDraw.Draw(sh)
for i, (c, f) in enumerate(zip(crops, files)):
    x, y = (i % cols)*w, (i//cols)*(h+14); sh.paste(c, (x, y+14)); d.text((x+2, y+1), f.split("/")[-1], fill=(255,0,0))
sh.save(out)
