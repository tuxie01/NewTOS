import sys,struct,collections
a=open(sys.argv[1],'rb').read(); b=open(sys.argv[2],'rb').read()
dirclusters=set(int(x) for x in sys.argv[3].split(',')) # clusters that are directory (or deleted dir) clusters
plba=63; base=plba*512; res=1; nfat=2; spf=128; root=base+(res+nfat*spf)*512; data=root+512*32; cs=2048
cls=collections.defaultdict(list)
for o in range(len(a)):
    if a[o]==b[o]: continue
    if o<base: c='MBR/other'
    elif o<base+512: c='volume serial' if 39<=o-base<=42 else 'OTHER bootsector'
    elif o<root: c='FAT'
    elif o<data:
        r=(o-root)%32; c='dirent time field (root)' if r in range(13,20) or r in range(22,26) else 'OTHER root dirent off %d'%r
    else:
        cl=(o-data)//cs+2; r=(o-data)%32
        if cl in dirclusters: c='dirent time field (cluster %d)'%cl if (r in range(13,20) or r in range(22,26)) else 'OTHER dircluster %d off %d'%(cl,r)
        elif cl==3: c='.fseventsd UUID file (cluster 3)'
        else: c='OTHER data cluster %d'%cl
    cls[c].append(o)
for c,v in sorted(cls.items()): print('%-45s %5d bytes  first %d last %d'%(c,len(v),v[0],v[-1]))
