import re,sys,collections
fn=None; d=collections.OrderedDict()
for l in open(sys.argv[1]):
    m=re.match(r'^[0-9a-f]+ <(.*)>:$',l)
    if m: fn=m.group(1); d[fn]=[]; continue
    if fn: d[fn].append(l.rstrip())
res={}
for f,ls in d.items():
    vs=[l for l in ls if re.search(r'\t[^\t]*\b[qvdsbh]\d+\b|\bv\d+\.',l.split('\t',2)[-1] if '\t' in l else '')]
    vs=[l for l in ls if re.search(r'\b[qv]\d+\b',l)]
    if vs: res[f]=ls
for f,ls in res.items():
    regs=sorted(set(re.findall(r'\b[qv]\d+\b',' '.join(ls))),key=lambda x:int(x[1:]))
    print(f, len(ls), 'insns; regs', ' '.join(sorted(set(r[1:] for r in regs),key=int)))
