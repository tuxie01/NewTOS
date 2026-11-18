# decode.py <BIN> -> one line per case: id opcode intout[0..n)  (big-endian words), n from contrl[4]; for 26 also ptsout? no
import sys, struct
d = open(sys.argv[1], "rb").read(); H = 32; R = 1592
n = (len(d) - H) // R
for i in range(n):
    r = d[H + i*R: H + (i+1)*R]
    cid, op = struct.unpack(">HH", r[0:4])
    contrl = struct.unpack(">12h", r[8:32])
    nout = min(contrl[4], 128)
    io = struct.unpack(">128h", r[800:1056])[:nout]
    npo = min(contrl[2], 128)
    po = struct.unpack(">256h", r[1056:1568])[:2*npo]
    print(cid, op, "i=" + ",".join(f"{x & 0xFFFF:04X}" if op == 105 and k else str(x) for k, x in enumerate(io)), "p=" + ",".join(map(str, po)))
