from cf import *
import pickle
def blank(): return [[0]*W for _ in range(H)]
def dots(s,y0,y1,pct,step):
    seed=12345
    for y in range(y0,y1+1):
        for x in range(640):
            if step: d = (x%step==1)
            else:
                seed=(seed*1103515245+12345)&0xffffffff
                d=(((seed>>16)&0x7fff)%100)<pct
            if d: s[y][x]=1
res={}
for nm,(y0,y1,pct,step,sx,sy) in {'K14A':(100,104,25,0,0,50),'K14B':(100,139,10,0,0,300),'K14C':(100,299,0,2,0,50)}.items():
    s=blank(); dots(s,y0,y1,pct,step)
    out,st=run(s,sx,sy)
    print(nm,'max',st['max'],'ov',st['ov'],'filled',sum(r.count(2) for r in out),'dots',sum(r.count(1) for r in s))
    res[nm]=out
pickle.dump(res,open('pred.pkl','wb'))
