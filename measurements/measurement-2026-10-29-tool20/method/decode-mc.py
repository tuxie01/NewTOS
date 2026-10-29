#!/usr/bin/env python3
"""decode-mc.py BIN TSV -- one line per record of a DUMPBIOS dump of a gemdos-mc table
(TASK/TOOL20; format 3, 320-byte records, docs/TOS-REFERENCE-DUMPER.md section 3): case id,
trap:function, ret as the 64-bit ret_hi:ret, and the capture read by the table's capture
column -- a d:dta / r:dta capture as the DTA's public fields (the layout's slots: d_attrib,
d_time, d_date, d_length, d_fname[0..13]), a Dgetpath capture as its string, a Dfree capture
as its four LONGs, a Getbpb capture as its nine WORDs -- then the comment's first words."""
import struct
import sys

POISON8 = 0xFFFFFFFFFFFFFFAD   # never a slot value: a UBYTE slot is zero-extended


def slots(cap, n):
    return [struct.unpack_from('>Q', cap, 8 * i)[0] for i in range(n)]


def dta(cap):
    s = slots(cap, 18)
    name = bytes(v & 0xFF for v in s[4:18])
    name = name.split(b'\0')[0].decode('latin-1') if s[4] != 0xAD else '(poison)'
    def w(v):
        return f'{v & 0xFFFF:04x}' if v != 0xFFFFFFFFFFFFDEAD else 'DEAD'
    att = f'{s[0]:02x}' if s[0] != 0xAD else 'AD'
    ln = s[3] - (1 << 64) if s[3] >= 1 << 63 else s[3]
    return f'attr={att} time={w(s[1])} date={w(s[2])} len={ln} name={name}'


data = open(sys.argv[1], 'rb').read()
table = {}
for n, line in enumerate(open(sys.argv[2], encoding='latin-1'), 1):
    cols = line.rstrip('\r\n').split('\t')
    if len(cols) >= 5 and not line.lstrip().startswith(('#', '!', '+')):
        table[n] = cols
size, count = struct.unpack_from('>HH', data, 8)
for i in range(count):
    r = data[32 + i * size: 32 + (i + 1) * size]
    cid, op = struct.unpack_from('>HH', r, 0)
    ret, = struct.unpack_from('>I', r, 36)
    hi, = struct.unpack_from('>I', r, 296)
    trap, = struct.unpack_from('>H', r, 300)
    v = (hi << 32) | ret
    if v >= 1 << 63:
        v -= 1 << 64
    cap = r[40:296]
    cols = table.get(cid, ['', '', '', '-', ''])
    c = cols[3]
    if c in ('d:dta', 'r:dta'):
        what = dta(cap)
    elif c == '0:16':
        what = 'path=' + repr(cap[:16].split(b'\0')[0].decode('latin-1'))
    elif c == '0:16.L':
        what = 'dfree=' + ','.join(str(x) for x in struct.unpack_from('>4I', cap, 0))
    elif c == 'r:18.W':
        what = 'bpb=' + ','.join(str(x) for x in struct.unpack_from('>9H', cap, 0))
    else:
        what = ''
    comment = ' '.join(cols[4].split()[:4])
    print(f'{cid:3d} {trap:2d}:0x{op:02X} ret={v:<11d} {what:58s} {comment}')
