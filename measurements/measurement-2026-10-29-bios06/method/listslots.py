#!/usr/bin/env python3
# listslots.py BIN case... -- display only: the first seven 8-byte layout slots
# (big-endian, docs/TOS-REFERENCE-DUMPER.md section 6) of a call record's capture
# (record bytes 40..295), printed raw in hex; after
# measurement-2026-10-29-xbios01/method/listslots.py. For case 34 (Getmpb, layout
# mpb) the slots are the layout's fields in order. The verdicts come from tosdiff.
import struct, sys
d = open(sys.argv[1], 'rb').read()
want = {int(x) for x in sys.argv[2:]}
rs, n = struct.unpack('>HH', d[8:12])
for i in range(n):
    r = d[32 + i * rs: 32 + (i + 1) * rs]
    cid, = struct.unpack('>h', r[0:2])
    if cid in want:
        s = struct.unpack('>7Q', r[40:96])
        print('%4d ' % cid + ' '.join('%016x' % v for v in s))
