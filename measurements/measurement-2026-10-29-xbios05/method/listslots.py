#!/usr/bin/env python3
# listslots.py BIN... -- display only: the nine 8-byte layout slots (big-endian,
# docs/TOS-REFERENCE-DUMPER.md section 6) of every call record's capture (record
# bytes 40..), for the r:bconmap case of tosdump/cases/xbios-bconmap-r1.tsv:
# maptab, then MAPTAB[0] (bconstat bconin bcostat bconout rsconf iorec), then
# maptabsize, mapped_device -- slot order = layout order, a followed pointer's
# structure right after the pointer. Also ret (record bytes 36..43). The
# verdicts come from tosdiff. (After measurement-2026-10-29-xbios01/method/listslots.py.)
import struct, sys
names = ['maptab', 'bconstat', 'bconin', 'bcostat', 'bconout', 'rsconf', 'iorec',
         'maptabsize', 'mapped_device']
for path in sys.argv[1:]:
    d = open(path, 'rb').read()
    rs, n = struct.unpack('>HH', d[8:12])
    for i in range(n):
        r = d[32 + i * rs: 32 + (i + 1) * rs]
        cid, = struct.unpack('>h', r[0:2])
        s = struct.unpack('>9Q', r[40:112])
        print('%s case %d: %s' % (path.split('/')[-2] + '/' + path.split('/')[-1], cid, ' '.join(
            '%s=%x' % (k, v) for k, v in zip(names, s))))
