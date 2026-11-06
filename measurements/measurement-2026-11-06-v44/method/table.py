#!/usr/bin/env python3
import sys,os,re
for d in sorted(sys.argv[1:]):
    try: t=open(d+'/TXBENCH.LOG',errors='replace').read()
    except OSError: print(os.path.basename(d),'NO LOG'); continue
    v=re.findall(r'ticks200=(\d+)',t)
    print(f"{os.path.basename(d):28s}", ' '.join(f"{x:>5s}" for x in v))
