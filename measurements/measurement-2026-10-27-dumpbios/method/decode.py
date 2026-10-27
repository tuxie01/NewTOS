#!/usr/bin/env python3
"""decode.py TABLE.TSV DUMP.BIN -- one line per case of a DUMPBIOS dump.

Prints case id, trap:function, the 64-bit return register (ret_hi:ret, the
record's two halves, docs/TOS-REFERENCE-DUMPER.md section 3) and the captured
bytes that differ from the 0xDEAD poison.  Reading aid only; the verdicts
are tosdiff's.
"""
import struct, sys

tsv, binf = sys.argv[1], sys.argv[2]
rows = {}
for n, line in enumerate(open(tsv, encoding='latin-1').read().split('\n'), 1):
    s = line.lstrip(' \t')
    if not s or s[0] in '#!':
        continue
    cols = line.split('\t', 4)
    rows[n] = cols
d = open(binf, 'rb').read()
ver, area, rsz, cnt = struct.unpack('>HHHH', d[4:12])
print('format %d area %d records %d build %s ptr %d order %d' %
      (ver, area, cnt, d[12:16].decode(), *struct.unpack('>HH', d[20:24])))
for i in range(cnt):
    r = d[32 + i * rsz: 32 + (i + 1) * rsz]
    cid, fn = struct.unpack('>HH', r[0:4])
    lo, = struct.unpack('>I', r[36:40])
    hi, = struct.unpack('>I', r[296:300])
    trap, = struct.unpack('>H', r[300:302])
    cap = r[40:296]
    used = len(cap.rstrip(b'')) if False else 0
    # captured bytes: up to the last byte that is not the poison pattern
    last = -1
    for j in range(0, 256, 2):
        if cap[j:j + 2] != b'\xde\xad':
            last = j + 1
    c = rows.get(cid, ['?', '?', '?', '?', ''])
    capx = cap[:last + 1].hex() if last >= 0 else '-'
    print('%3d %2d:0x%02X %-8s ret=%08X%08X  cap=%s  # %s' %
          (cid, trap, fn, c[2], hi, lo, capx, c[4] if len(c) > 4 else ''))
