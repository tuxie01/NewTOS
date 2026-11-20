#!/usr/bin/env python3
"""decode-peek.py BIN TSV -- one line per peek record: case id, cell, width,
value (zero-extended), the second read of a MONOTONE case, the comment."""
import struct, sys
b = open(sys.argv[1], 'rb').read()
lines = open(sys.argv[2], encoding='latin-1').read().split('\n')
n = struct.unpack('>H', b[10:12])[0]
for i in range(n):
    r = b[32 + 320 * i: 32 + 320 * (i + 1)]
    cid, op = struct.unpack('>HH', r[0:4])
    addr = struct.unpack('>I', r[4:8])[0]
    ret, = struct.unpack('>I', r[36:40])
    hi, = struct.unpack('>I', r[296:300])
    trap, = struct.unpack('>H', r[300:302])
    cap, = struct.unpack('>I', r[40:44])
    cols = lines[cid - 1].split('\t')
    extra = ' then 0x%08X (+%d)' % (cap, (cap - ret) & 0xFFFFFFFF) if cols[2] == 'MONOTONE' else ''
    print('%3d trap %d $%03X:%d %-8s 0x%08X%s%s  %s' % (cid, trap, addr, op, cols[2], ret,
          '' if hi == 0 else ' HI=0x%08X' % hi, extra, cols[4] if len(cols) > 4 else ''))
