import struct,sys,os
def load(p):
    b=open(os.path.join(p,"VDPROBE.BIN"),"rb").read(); off=6; r={}
    while off<len(b):
        no,=struct.unpack(">H",b[off+1:off+3]); n=b[off+3:off+11].rstrip(b"\0").decode()
        x,y,w,h=struct.unpack(">hhhh",b[off+11:off+19]); off+=19
        r[n]=(x,y,w,h,b[off:off+2*w*h]); off+=2*w*h
    return r
for s in sys.argv[1:]:
    d=load(s); print(s)
    for k in sorted(d):
        if k.startswith("K15"):
            x,y,w,h,px=d[k]; ws=struct.unpack(">%dH"%(w*h),px)
            red=sum(1 for v in ws if v==0xf800); blk=sum(1 for v in ws if v==0)
            print(" ",k,"red",red,"black",blk,"nonwhite",sum(1 for v in ws if v!=0xffff))
