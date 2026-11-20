#!/usr/bin/env python3
"""per-case.py TSV TT.BIN FALCONTC.BIN NEWTOS.BIN TOSDIFF-TT.txt TOSDIFF-FTC.txt
One line per case: id, cell, class, the three values (MONOTONE: first then
second read), and EQUAL/DIFF against tt and falcontc as the two tosdiff
reports name the case (a case tosdiff names under "case <id>" is DIFF)."""
import re, struct, sys
lines = open(sys.argv[1], encoding='latin-1').read().split('\n')
bins = [open(p, 'rb').read() for p in sys.argv[2:5]]
diffs = []
for p in sys.argv[5:7]:
    diffs.append(set(int(m) for m in re.findall(r'^case (\d+) ', open(p).read(), re.M)))
n = struct.unpack('>H', bins[0][10:12])[0]
print('%-4s %-9s %-8s %-23s %-23s %-23s %-5s %-5s %s' % ('case', 'cell', 'class', 'tt', 'falcontc', 'NewTOS', 'tt', 'ftc', 'name'))
for i in range(n):
    vals = []
    for b in bins:
        r = b[32 + 320 * i: 32 + 320 * (i + 1)]
        cid, op = struct.unpack('>HH', r[0:4])
        addr, = struct.unpack('>I', r[4:8])
        ret, = struct.unpack('>I', r[36:40])
        cap, = struct.unpack('>I', r[40:44])
        cols = lines[cid - 1].split('\t')
        vals.append('0x%08X>0x%08X' % (ret, cap) if cols[2] == 'MONOTONE' else '0x%08X' % ret)
    print('%-4d $%03X:%-4d %-8s %-23s %-23s %-23s %-5s %-5s %s' % (cid, addr, op, cols[2], vals[0], vals[1], vals[2],
          'DIFF' if cid in diffs[0] else 'EQUAL', 'DIFF' if cid in diffs[1] else 'EQUAL', cols[4].split(' ')[0]))
