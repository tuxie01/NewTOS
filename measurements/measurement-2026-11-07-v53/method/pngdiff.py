import zlib,struct,sys
def load(p):
    d=open(p,'rb').read(); i=8; idat=b''
    while i<len(d):
        l=struct.unpack('>I',d[i:i+4])[0]; t=d[i+4:i+8]; c=d[i+8:i+8+l]
        if t==b'IHDR': w,h,bd,ct=struct.unpack('>IIBB',c[:10])
        if t==b'IDAT': idat+=c
        i+=12+l
    raw=zlib.decompress(idat); bpp={2:3,6:4}[ct]; st=w*bpp; out=[]; prev=bytearray(st); o=0
    for y in range(h):
        f=raw[o]; line=bytearray(raw[o+1:o+1+st]); o+=1+st
        for x in range(st):
            a=line[x-bpp] if x>=bpp else 0; b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
            if f==1: line[x]=(line[x]+a)&255
            elif f==2: line[x]=(line[x]+b)&255
            elif f==3: line[x]=(line[x]+((a+b)>>1))&255
            elif f==4:
                p=a+b-c; pa=abs(p-a); pb=abs(p-b); pc=abs(p-c)
                pr=a if pa<=pb and pa<=pc else (b if pb<=pc else c); line[x]=(line[x]+pr)&255
        out.append(bytes(line)); prev=line
    return w,h,bpp,out
A=load(sys.argv[1]); B=load(sys.argv[2]); w,h,bpp=A[:3]
pts=[(x,y) for y in range(h) for x in range(w) if A[3][y][x*bpp:x*bpp+3]!=B[3][y][x*bpp:x*bpp+3]]
if pts:
    xs=[p[0] for p in pts]; ys=[p[1] for p in pts]
    print(len(pts),'px, box',min(xs),min(ys),max(xs),max(ys)); x,y=pts[0]; print('first',x,y,A[3][y][x*bpp:x*bpp+3].hex(),B[3][y][x*bpp:x*bpp+3].hex())
else: print('0 px')
