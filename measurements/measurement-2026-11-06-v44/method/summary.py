#!/usr/bin/env python3
# summary.py <runs/txbench dir> [base-name head-name]: minimum and span per case and kernel, factors.
import sys,os,re,glob
d=sys.argv[1]; res={}
for r in sorted(glob.glob(d+'/*')):
    k=os.path.basename(r).rsplit('-',1)[0]
    try: t=open(r+'/TXBENCH.LOG',errors='replace').read()
    except OSError: continue
    for name,v in re.findall(r'(\w+) calls=\d+ chars_per_call=\d+ ticks200=(\d+)',t):
        res.setdefault(k,{}).setdefault(name,[]).append(int(v))
kernels=sorted(res)
cases=sorted(next(iter(res.values())))
for c in cases:
    print(c)
    for k in kernels:
        v=sorted(res[k][c]); med=v[len(v)//2]
        print(f"  {k:8s} n={len(v)} min {v[0]:4d} median {med:4d} max {v[-1]:4d}  all {res[k][c]}")
    if 'base' in res:
        b=res['base'][c]
        for k in kernels:
            if k=='base': continue
            h=res[k][c]
            pairs=[x/y for x,y in zip(h,b)]
            print(f"  {k}/base min/min x{min(h)/min(b):.2f}  median/median x{sorted(h)[len(h)//2]/sorted(b)[len(b)//2]:.2f}  same-round pairs x{min(pairs):.2f}..x{max(pairs):.2f}")
