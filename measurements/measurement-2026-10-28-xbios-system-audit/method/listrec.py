#!/usr/bin/env python3
# listrec.py BIN [lo hi] -- print case id, trap:fn, args, 64-bit ret of each
# 320-byte call record (docs/TOS-REFERENCE-DUMPER.md section 3). Display only;
# the verdicts come from tosdiff.
import struct, sys
d = open(sys.argv[1], 'rb').read()
lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
hi = int(sys.argv[3]) if len(sys.argv) > 3 else 1 << 30
rs, n = struct.unpack('>HH', d[8:12])
for i in range(n):
    r = d[32 + i * rs: 32 + (i + 1) * rs]
    cid, op = struct.unpack('>hH', r[0:4])
    if not lo <= cid <= hi:
        continue
    args = struct.unpack('>8l', r[4:36])
    ret, = struct.unpack('>L', r[36:40]); rh, = struct.unpack('>L', r[296:300])
    trap, = struct.unpack('>H', r[300:302])
    a = ','.join('%x' % (x & 0xffffffff) for x in args if (x & 0xffffffff) != 0xDEADDEAD)
    print('%4d %d:%-3d (%s) ret=%08x%08x' % (cid, trap, op, a, rh, ret))
