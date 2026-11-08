# V40: decode two PNG screendumps (all filter types) and print how many pixels differ and their bounding box.

import sys,zlib,struct
def png(p):
    d=open(p,'rb').read(); i=8; idat=b''
    while i<len(d):
        l=struct.unpack('>I',d[i:i+4])[0]; t=d[i+4:i+8]; c=d[i+8:i+8+l]
        if t==b'IHDR': w,h,bd,ct=struct.unpack('>IIBB',c[:10])
        if t==b'IDAT': idat+=c
        i+=12+l
    raw=zlib.decompress(idat); bpp=(3 if ct==2 else 4); st=w*bpp+1
    out=bytearray(); prev=bytearray(w*bpp)
    for y in range(h):
        f=raw[y*st]; line=bytearray(raw[y*st+1:(y+1)*st])
        if f:
            for x in range(len(line)):
                a=line[x-bpp] if x>=bpp else 0; b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
                if f==1: line[x]=(line[x]+a)&255
                elif f==2: line[x]=(line[x]+b)&255
                elif f==3: line[x]=(line[x]+(a+b)//2)&255
                elif f==4:
                    p=a+b-c; pa,pb,pc=abs(p-a),abs(p-b),abs(p-c)
                    pr=a if pa<=pb and pa<=pc else (b if pb<=pc else c); line[x]=(line[x]+pr)&255
        out+=line; prev=line
    return w,h,out,bpp
w,h,a,bpp=png(sys.argv[1]); _,_,b,_=png(sys.argv[2])
xs=[];ys=[]
for y in range(h):
    for x in range(w):
        o=(y*w+x)*bpp
        if a[o:o+bpp]!=b[o:o+bpp]: xs.append(x); ys.append(y)
print(sys.argv[1].split('/')[-3], sys.argv[1].split('/')[-1], "pixels",len(xs), "x",min(xs),"-",max(xs),"y",min(ys),"-",max(ys))
