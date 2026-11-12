import sys,struct
def load(p): return open(p,'rb').read()
class Fat:
    def __init__(s,d,part_lba):
        s.d=d; b=part_lba*512; s.b=b
        bs=d[b:b+512]
        s.bps,s.spc,s.res,s.nfat,s.rootn,s.tot16,_,s.spf=struct.unpack('<HBHBHHBH',bs[11:11+13+... if False else 24][0:0]) if False else (0,)*8
        s.bps=struct.unpack('<H',bs[11:13])[0]; s.spc=bs[13]; s.res=struct.unpack('<H',bs[14:16])[0]
        s.nfat=bs[16]; s.rootn=struct.unpack('<H',bs[17:19])[0]; s.spf=struct.unpack('<H',bs[22:24])[0]
        s.fat0=b+s.res*512; s.root=s.fat0+s.nfat*s.spf*512; s.data=s.root+s.rootn*32; s.cs=s.spc*512
    def fat(s,c): return struct.unpack('<H',s.d[s.fat0+2*c:s.fat0+2*c+2])[0]
    def coff(s,c): return s.data+(c-2)*s.cs
    def chain(s,c):
        r=[]
        while 2<=c<0xFFF8 and len(r)<100000: r.append(c); c=s.fat(c)
        return r
    def walk(s):
        # returns list of (path, kind, dir_entry_abs_offset_range list, clusters)
        out=[]; 
        def scan(off,n,path):
            for i in range(n):
                e=off+i*32; ent=s.d[e:e+32]
                if ent[0]==0: break
                if ent[0]==0xE5 or ent[11]==0x0F: continue
                name=ent[:8].decode('latin1').strip()+('.'+ent[8:11].decode('latin1').strip() if ent[8:11].strip() else '')
                if ent[11]&8: out.append((path+'<label>','label',e,[])); continue
                if name in ('.','..'): out.append((path+name,'dot',e,[])); continue
                c=struct.unpack('<H',ent[26:28])[0]; ch=s.chain(c) if c else []
                isd=bool(ent[11]&0x10)
                out.append((path+name,'dir' if isd else 'file',e,ch))
                if isd:
                    for cc in ch: scan(s.coff(cc),s.cs//32,path+name+'/')
        scan(s.root,s.rootn,'/')
        return out
def main():
    a,b=load(sys.argv[1]),load(sys.argv[2]); offs=[int(x) for x in open(sys.argv[3])]
    # partition start from MBR entry1
    plba=struct.unpack('<I',a[454:458])[0]; print('partition LBA',plba)
    fa,fb=Fat(a,plba),Fat(b,plba)
    print('geometry',fa.spc,fa.res,fa.nfat,fa.rootn,fa.spf,'root',fa.root,'data',fa.data)
    for nm,f in (('A(fixed)',fa),('B(base)',fb)):
        print(nm); 
        for p,k,e,ch in f.walk(): print('  ',k,p,'entry@',e,'clusters',ch[:6],len(ch))
    # classify
    cls={}
    def add(c,o): cls.setdefault(c,[]).append(o)
    live={}
    for nm,f in (('A',fa),('B',fb)):
        for p,k,e,ch in f.walk():
            live[nm,'entry']=live.get((nm,'entry'),[])+[(e,e+32,p,k)]
    dirent={}
    for o in offs:
        c=None
        if o>=plba*512 and o<plba*512+512:
            rel=o-plba*512; c='volume serial' if 39<=rel<=42 else 'other:bootsector'
        elif fa.fat0<=o<fa.root:
            ci=((o-fa.fat0)%(fa.spf*512))//2; c='FAT entry cluster %d'%ci
        elif fa.root<=o<fa.data:
            rel=(o-fa.root)%32
            c='dirent time field (root, off %d)'%rel if rel in (13,14,15,16,17,18,19,22,23,24,25) else 'other:root dirent off %d'%rel
        else:
            cl=(o-fa.data)//fa.cs+2
            c='data cluster %d'%cl
        add(c.split(' (')[0] if 0 else c,o)
    for c,v in cls.items(): 
        if c.startswith('data cluster') or c.startswith('FAT'): continue
        print(c,len(v),min(v),max(v))
    import collections
    dc=collections.defaultdict(list)
    for c,v in cls.items():
        if c.startswith('data cluster'): dc[int(c.split()[2])]=v
    print('FAT diff:',[ (c,len(v)) for c,v in cls.items() if c.startswith('FAT')])
    print('data clusters differing:',[(k,len(v),min(v),max(v)) for k,v in sorted(dc.items())])
main()
