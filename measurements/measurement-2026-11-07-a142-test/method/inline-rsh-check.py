import re,sys
up=open(sys.argv[1]).read(); ov=open(sys.argv[2]).read()
pu=re.compile(r'static\s+_UBYTE\s+(\w+)\[\]\s*=\s*\{(.*?)\};',re.S)
po=re.compile(r'static\s+unsigned\s+short\s+(\w+)\[\]\s*=\s*\{(.*?)\};',re.S)
U={m.group(1):[int(x,16) for x in re.findall(r'0x[0-9A-Fa-f]+',m.group(2))] for m in pu.finditer(up)}
O={m.group(1):[int(x,16) for x in re.findall(r'0x[0-9A-Fa-f]+',m.group(2))] for m in po.finditer(ov)}
print("upstream byte tables",len(U),"overlay word tables",len(O),"names equal",set(U)==set(O))
bad=0;tot=0
for k,b in U.items():
    assert len(b)%2==0
    w=[(b[i]<<8)|b[i+1] for i in range(0,len(b),2)]
    tot+=len(w)
    if O.get(k)!=w: bad+=1;print("MISMATCH",k)
print("words compared",tot,"mismatching tables",bad)
print("overlay still _UBYTE tables:",len(re.findall(r'static\s+_UBYTE',ov)))
# rest of file identical modulo tables and header comment
ru=pu.sub('T',up); ro=po.sub('T',ov)
import difflib
d=[l for l in difflib.unified_diff(ru.splitlines(),ro.splitlines(),lineterm='',n=0) if l[0] in '+-' and l[:3] not in('+++','---')]
print("non-table diff lines:",len(d)); print("\n".join(d))
