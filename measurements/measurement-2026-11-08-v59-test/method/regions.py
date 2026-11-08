import re,sys
fn=None; d={}
for l in open(sys.argv[1]):
    m=re.match(r'^[0-9a-f]+ <(.*)>:$',l)
    if m: fn=m.group(1); d[fn]=[]; continue
    if fn: d[fn].append(l.rstrip())
def mn(l):
    p=l.split('\t'); return p[1].strip() if len(p)>2 else ''
def regs(l):
    p=l.split('\t',2); body=p[2] if len(p)>2 else ''
    return set(int(x) for x in re.findall(r'\b[qv](\d+)\b',body))
for f in sys.argv[2:]:
    ls=d[f]; print("==",f)
    i=0; seg=0
    # segment: from first stp q.. to the matching last ldp q.. before the next stp q after at least one ldp
    stps=[k for k,l in enumerate(ls) if mn(l)=='stp' and re.search(r'\bq\d+',l)  or mn(l)=='str' and re.search(r'\tq\d+,',l)]
    lds=[k for k,l in enumerate(ls) if mn(l)=='ldp' and re.search(r'\bq\d+',l) or mn(l)=='ldr' and re.search(r'\tq\d+,',l)]
    # group: consecutive stp/str run = a save block; consecutive ldp/ldr run = restore block
    def runs(idx):
        out=[];cur=[]
        for k in idx:
            if cur and k-cur[-1]>8: out.append(cur);cur=[]
            cur.append(k)
        if cur: out.append(cur)
        return out
    S_=runs(stps); L_=runs(lds)
    print(" save blocks",len(S_),"restore blocks",len(L_))
    for si,(sb,lb) in enumerate(zip(S_,L_)):
        saved=set().union(*[regs(ls[k]) for k in sb]); rest=set().union(*[regs(ls[k]) for k in lb])
        used=set().union(*[regs(ls[k]) for k in range(sb[-1]+1,lb[0]) ])
        calls=[ls[k] for k in range(sb[-1]+1,lb[0]) if mn(ls[k]) in('bl','blr','ret','svc','msr','mrs','wfi')]
        print(f" region {si}: saved {sorted(saved)} restored {sorted(rest)} used-in-between {sorted(used)} | used⊆saved {used<=saved} saved==restored {saved==rest} calls/ret/msr in between: {len(calls)}")
