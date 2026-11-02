#!/usr/bin/env python3 -I
# sample.py <qmp-sock> <outprefix> <t1,t2,...>   : at each time stop VM, pmemsave kernel+arena, cont
import socket, json, sys, time, os
sock, out, times = sys.argv[1], sys.argv[2], [float(x) for x in sys.argv[3].split(',')]
ARENA_PA=0x01000000; ARENA_N=int(sys.argv[4]) if len(sys.argv)>4 else 2415
s=socket.socket(socket.AF_UNIX); 
for _ in range(100):
    try: s.connect(sock); break
    except Exception: time.sleep(0.2)
f=s.makefile('rw')
def rd():
    while True:
        l=f.readline()
        if not l: return None
        j=json.loads(l)
        if 'return' in j or 'error' in j: return j
def cmd(c,**a):
    f.write(json.dumps({'execute':c,'arguments':a})+'\n'); f.flush(); return rd()
f.readline(); cmd('qmp_capabilities')
t0=time.time()
for i,t in enumerate(times):
    time.sleep(max(0,t0+t-time.time()))
    cmd('stop')
    print(cmd('pmemsave',val=0,size=0x400000,filename=f'{out}-k{i}.bin'))
    print(cmd('pmemsave',val=ARENA_PA,size=ARENA_N*4096,filename=f'{out}-a{i}.bin'))
    cmd('cont')
cmd('quit')
