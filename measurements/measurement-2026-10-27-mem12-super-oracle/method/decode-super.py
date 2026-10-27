#!/usr/bin/env python3
"""decode-super.py BIOS.BIN -- one line per call record: case id, trap:function, arg0 token/value, ret_hi:ret (hex)."""
import struct, sys
b = open(sys.argv[1], 'rb').read()
assert b[:4] == b'TDMP'
ver, area, rs, n = struct.unpack('>HHHH', b[4:12])
print("format %d area %d record %d count %d build %s crc %08X ptr %d order %d" % (
    ver, area, rs, n, b[12:16].decode(), struct.unpack('>I', b[16:20])[0], *struct.unpack('>HH', b[20:24])))
for i in range(n):
    r = b[32 + i * rs: 32 + (i + 1) * rs]
    cid, op = struct.unpack('>HH', r[0:4]); a0 = struct.unpack('>I', r[4:8])[0]
    ret, = struct.unpack('>I', r[36:40]); hi, = struct.unpack('>I', r[296:300]); trap, = struct.unpack('>H', r[300:302])
    print("case %2d  %2d:0x%02X  arg0 %08X  ret %08X:%08X" % (cid, trap, op, a0, hi, ret))
