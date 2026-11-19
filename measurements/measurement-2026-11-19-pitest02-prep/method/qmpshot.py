import socket, sys, json, time
sock_path, out = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX); s.connect(sock_path)
f = s.makefile('rw')
f.readline()
def cmd(c):
    f.write(json.dumps(c) + "\n"); f.flush()
    while True:
        l = f.readline()
        if '"return"' in l or '"error"' in l: return l
cmd({"execute": "qmp_capabilities"})
print(cmd({"execute": "screendump", "arguments": {"filename": out, "format": "png"}}))
