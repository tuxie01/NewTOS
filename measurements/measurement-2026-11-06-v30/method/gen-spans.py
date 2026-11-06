import sys, importlib.util
spec = importlib.util.spec_from_file_location("vc", "tosdump/probes/vdprobe-compare.py")
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
e={ (c[1]):c for c in vc.load(sys.argv[1])}
def word(w):
    return {0:'B',0xFFDF:'P'}.get(w,'0x%04X'%w)
def spans(name, rect=None):
    no,nm,x,y,w,h,ws=e[name]
    gx,gy,gw,gh = rect or (x,y,w,h)
    out=[]
    for yy in range(gy,gy+gh):
        xx=gx
        while xx<gx+gw:
            v=ws[(yy-y)*w+(xx-x)]
            xe=xx
            while xe+1<gx+gw and ws[(yy-y)*w+(xe+1-x)]==v: xe+=1
            if v!=0xFFFF: out.append("(%d, %d, %d, %s)"%(yy,xx,xe,word(v)))
            xx=xe+1
    return no,(gx,gy,gw,gh),out
def const(cname,name,rect=None):
    no,r,sp=spans(name,rect)
    lines=[];cur="   "
    for s in sp:
        if len(cur)+len(s)+2>110: lines.append(cur);cur="   "
        cur+=" "+s+","
    if cur.strip(): lines.append(cur)
    if not sp:
        print("/// %s: grab (%d,%d) %dx%d, record %d of the V30 run's probe/e1 (all white)."%(name,r[0],r[1],r[2],r[3],no))
        print("const %s: &[Span] = &[];"%cname); print(); return r
    print("/// %s: grab (%d,%d) %dx%d, record %d of the V30 run's probe/e1."%(name,r[0],r[1],r[2],r[3],no))
    print("#[rustfmt::skip]")
    print("const %s: &[Span] = &["%cname)
    for l in lines: print(l)
    print("];"); print()
    return r
L={'TVF':(201,100,4,220),'TVG':(296,80,4,220),'TVH':(300,96,220,4),'TVI':(80,301,160,4)}
for c,n in [("SP_TVA","TVA"),("SP_TVB","TVB"),("SP_TVBW","TVBw"),("SP_TVC","TVC"),("SP_TVCW","TVCw"),("SP_TVD","TVD"),("SP_TVDW","TVDw"),("SP_TVE","TVE"),
 ("SP_TVF","TVF"),("SP_TVG","TVG"),("SP_TVH","TVH"),("SP_TVI","TVI"),("SP_TVJ","TVJ"),("SP_TVL","TVL"),("SP_TVM","TVM"),("SP_TVN","TVN"),("SP_TVQ","TVQ"),
 ("SP_TVR","TVR"),("SP_TVRW","TVRw"),("SP_TVS","TVS"),("SP_TVSW","TVSw"),("SP_TVT","TVT"),("SP_TVTW","TVTw"),("SP_TVV","TVV"),("SP_TVVW","TVVw"),("SP_TVW","TVW"),("SP_TVWW","TVWw"),("SP_TVX","TVX"),("SP_TVY","TVY"),("SP_TVYW","TVYw")]:
    const(c,n,L.get(n))
const('SP_TWA','TWA',(201,90,4,230))
