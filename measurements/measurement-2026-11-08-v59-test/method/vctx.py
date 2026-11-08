import re,sys
want=sys.argv[2]
fn=None; ls=[]
for l in open(sys.argv[1]):
    m=re.match(r'^[0-9a-f]+ <(.*)>:$',l)
    if m: fn=m.group(1); continue
    if fn==want: ls.append(l.rstrip())
vi=[i for i,l in enumerate(ls) if re.search(r'\b[qv]\d+\b',l)]
print(want, len(ls), "insns; V-touching:", len(vi), "first idx",vi[0],"last idx",vi[-1])
first,last=vi[0],vi[-1]
print("-- first 6 / last 6 V insns")
for i in vi[:6]+vi[-6:]: print(ls[i])
print("-- instruction mnemonics naming V regs:")
from collections import Counter
c=Counter(l.split('\t')[2].split()[0] if l.count('\t')>=2 else '?' for l in (ls[i] for i in vi)); print(dict(c))
print("-- bl/blr/ret/svc/msr/mrs between first and last V insn:")
for l in ls[first:last+1]:
    mn=l.split('\t')[2].split()[0] if l.count('\t')>=2 else ''
    if mn in('bl','blr','ret','svc','msr','mrs','br','wfi','hvc'): print(l)
print("-- fp arithmetic mnemonics in V insns:", [m for m in c if m.startswith('f') and m not in('ldr',)])
