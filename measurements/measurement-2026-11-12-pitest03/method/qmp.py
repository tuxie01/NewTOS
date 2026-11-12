import socket,json,sys,time
s=socket.socket(socket.AF_UNIX); s.connect('q.sock'); f=s.makefile('rw')
f.readline()
def cmd(c,**a):
    f.write(json.dumps({"execute":c,"arguments":a})+"\n"); f.flush()
    while True:
        r=json.loads(f.readline())
        if 'return' in r or 'error' in r: return r
cmd('qmp_capabilities')
print(cmd('screendump',filename=sys.argv[1],format='ppm') if len(sys.argv)>2 else cmd('screendump',filename=sys.argv[1]))
