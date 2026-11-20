#!/usr/bin/env python3
"""doctor-monotone.py IN OUT CASEID MODE -- patch the second read (capture[0..3], big-endian)
of the MONOTONE peek record with line id CASEID: MODE stands (= first read), back (first-5),
wrap (second = first-0x7FFFFFF0 mod 2^32: advance modulo 2^32, should still pass), grow (first+3)."""
import struct, sys
b = bytearray(open(sys.argv[1], 'rb').read()); cid = int(sys.argv[3]); mode = sys.argv[4]
n = struct.unpack('>H', b[10:12])[0]
for i in range(n):
    o = 32 + 320 * i
    if struct.unpack('>H', b[o:o+2])[0] != cid: continue
    ret, = struct.unpack('>I', b[o+36:o+40])
    new = {'stands': ret, 'back': (ret - 5) & 0xFFFFFFFF, 'grow': (ret + 3) & 0xFFFFFFFF}[mode]
    b[o+40:o+44] = struct.pack('>I', new)
open(sys.argv[2], 'wb').write(b)
