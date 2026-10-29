#!/usr/bin/env python3
# listslots.py BIN case... -- display only: the first six 8-byte layout slots
# (big-endian, docs/TOS-REFERENCE-DUMPER.md section 6) of a call record's capture
# (record bytes 40..295), for an r:iorec case: ibuf ibufsiz ibufhd ibuftl ibuflow
# ibufhi. The verdicts come from tosdiff.
import struct, sys
d = open(sys.argv[1], 'rb').read()
want = {int(x) for x in sys.argv[2:]}
rs, n = struct.unpack('>HH', d[8:12])
for i in range(n):
    r = d[32 + i * rs: 32 + (i + 1) * rs]
    cid, = struct.unpack('>h', r[0:2])
    if cid in want:
        s = struct.unpack('>6Q', r[40:88])
        print('%4d ibuf=%x size=%d hd=%d tl=%d low=%d high=%d' % ((cid,) + tuple(
            v - (1 << 64) if v >> 63 else v for v in s)))
