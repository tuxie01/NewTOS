#!/usr/bin/env python3 -I
import struct, sys
from collections import defaultdict
kf, af, N = sys.argv[1], sys.argv[2], int(sys.argv[3])
K=open(kf,'rb').read(); A=open(af,'rb').read()
ARENA=0x01000000
def q(buf,off): return struct.unpack_from('<Q',buf,off)[0]
USER=0x2dc000; SH_L2=0x293000; BITS=0x34d928
bits=[q(K,BITS+8*i) for i in range(128)]
used={i for i in range(N) if bits[i//64]>>(i%64)&1}
priv=defaultdict(list)   # arena idx -> [(slot,gib,idx)]
shared=set()
for g in range(4):
    for idx in range(512):
        t=q(K,SH_L2+g*4096+idx*8)
        if t&3==3:
            pa=t&0xFFFFFFFFF000
            shared.add(pa)
for slot in range(16):
    for g in range(4):
        for idx in range(512):
            cur=q(K,USER+slot*28672+(2+g)*4096+idx*8)
            tmpl=q(K,SH_L2+g*4096+idx*8)
            if g==0 and idx==0: continue
            if cur!=tmpl and cur&3==3:
                pa=cur&0xFFFFFFFFF000
                if ARENA<=pa<ARENA+N*4096:
                    priv[(pa-ARENA)//4096].append((slot,g,idx))
                else:
                    print("PRIVATE entry outside arena",slot,g,idx,hex(pa))
shared_idx={(p-ARENA)//4096 for p in shared if ARENA<=p<ARENA+N*4096}
print("arena used bits:",len(used)," shared L3:",len(shared_idx)," private tables referenced:",len(priv))
dups={i:v for i,v in priv.items() if len(v)>1}
print("tables referenced by >1 slot entries:",len(dups), list(dups.items())[:5])
notused=[i for i in priv if i not in used]
print("referenced but marked FREE:",len(notused), [(i,priv[i]) for i in notused[:10]])
orph=[i for i in used if i not in priv and i not in shared_idx]
print("marked used but unreferenced (leak):",len(orph), orph[:10])
sh_and_priv=[i for i in priv if i in shared_idx]
print("private ref'd == shared L3:",len(sh_and_priv))
per=defaultdict(int)
for i,v in priv.items():
    for (s,g,x) in v: per[s]+=1
print("private tables per slot:",dict(sorted(per.items())))
