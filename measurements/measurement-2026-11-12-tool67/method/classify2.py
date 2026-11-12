# classify every differing byte of two MBR+FAT16 images (partition 1) by owner in each image
import sys,struct,collections
a=open(sys.argv[1],'rb').read(); b=open(sys.argv[2],'rb').read(); assert len(a)==len(b)
plba=struct.unpack('<I',a[454:458])[0]
class Fat:
    def __init__(s,d):
        s.d=d; bs=d[plba*512:plba*512+512]; s.spc=bs[13]; res=struct.unpack('<H',bs[14:16])[0]; nf=bs[16]; s.rootn=struct.unpack('<H',bs[17:19])[0]; s.spf=struct.unpack('<H',bs[22:24])[0]
        s.fat0=plba*512+res*512; s.fatend=s.fat0+nf*s.spf*512; s.root=s.fatend; s.data=s.root+s.rootn*32; s.cs=s.spc*512
        s.owner={}  # cluster -> (kind,path)
        s.walk()
    def fat(s,c): return struct.unpack('<H',s.d[s.fat0+2*c:s.fat0+2*c+2])[0]
    def coff(s,c): return s.data+(c-2)*s.cs
    def chain(s,c):
        r=[]
        while 2<=c<0xFFF8 and len(r)<70000: r.append(c); c=s.fat(c)
        return r
    def entries(s,off,n):
        for i in range(n):
            e=off+i*32; ent=s.d[e:e+32]
            if ent[0]==0: return
            yield e,ent
    def scan(s,off,n,path):
        for e,ent in s.entries(off,n):
            if ent[0]==0xE5 or ent[11]==0x0F or ent[11]&8: continue
            nm=ent[:8].decode('latin1').strip()+('.'+ent[8:11].decode('latin1').strip() if ent[8:11].strip() else '')
            if nm in('.','..'): continue
            c=struct.unpack('<H',ent[26:28])[0]; ch=s.chain(c) if c else []
            isd=bool(ent[11]&0x10)
            for cc in ch: s.owner[cc]=('dir' if isd else 'file',path+nm)
            if isd:
                for cc in ch: s.scan(s.coff(cc),s.cs//32,path+nm+'/')
    def walk(s): s.scan(s.root,s.rootn,'/')
fa,fb=Fat(a),Fat(b)
RAND=set(sys.argv[3].split(',')) if len(sys.argv)>3 else set()
cls=collections.defaultdict(list)
def tf(r): return r in range(13,20) or r in range(22,26)
for o in range(len(a)):
    if a[o]==b[o]: continue
    if o<plba*512: c='OTHER mbr/pre'
    elif o<plba*512+512: c='volume serial' if 39<=o-plba*512<=42 else 'OTHER bootsector'
    elif o<fa.fatend: c='FAT (cluster chain entries; follows placement)'
    elif o<fa.data:
        c='root dir time field' if tf((o-fa.root)%32) else 'root dir entry (name/cluster/other)'
    else:
        ca=(o-fa.data)//fa.cs+2; oa=fa.owner.get(ca); ob=fb.owner.get(ca)
        r=(o-fa.data)%32
        if oa and oa[0]=='file' and oa[1] in RAND and ob and ob[0]=='file' and ob[1]==oa[1]: c='urandom file '+oa[1]
        elif oa and ob and oa==ob and oa[0]=='dir': c='dir time field' if tf(r) else 'OTHER dir cluster entry bytes'
        elif oa and ob and oa==ob: c='OTHER same live file %s'%oa[1]
        elif not oa and not ob: c='free/deleted clusters in both (.fseventsd/.Spotlight residue)'
        else: c='placement: cluster live in A as %s, in B as %s'%(oa,ob)
    cls[c].append(o)
for c,v in sorted(cls.items()): print('%-95s %6d bytes first %d last %d'%(c,len(v),v[0],v[-1]))
