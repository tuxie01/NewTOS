#!/usr/bin/env python3
"""compare-ends.py <scratch> <runs-dir> -- FS24: the end-state rule of the destructive table
gemdos-mc-fs24.tsv on the TEST PARTITION (LBA 63, 130977 sectors), after
measurements/measurement-2026-10-29-tool20/method/compare-ends.py:
  original vs oracle-fs24-1        which sectors the table changed
  oracle-fs24-1 vs oracle-fs24-2   the oracle reproduces its end state
  oracle-fs24-1 vs newtos-fs24     both sides end in the same state
each raw and with the FAT time stamps (bytes 13-19 and 22-25 of a directory entry) zeroed in
the root directory AND in C:\\SUB's directory sectors: part V3 creates C:\\SUB\\FILL.DAT, which
each side stamps with its own clock. A difference names the partition sector and byte offset.
Input <scratch>/test.img (the original, partition cut out here) and
<scratch>/end/{oracle-fs24-<i>,newtos-fs24}.img; output <runs-dir>/endstate-fs24.txt."""
import hashlib
import os
import sys

PSTART, PSIZE = 63, 130977
RES, NFATS, FSIZ, ROOTSEC, CLSIZ = 1, 2, 128, 32, 4      # tools/mktestdisk.sh geometry
ROOT0 = RES + NFATS * FSIZ                                 # 257
DATA0 = ROOT0 + ROOTSEC                                    # 289


def u16(d, o):
    return d[o] | d[o + 1] << 8


def sub_sectors(d):
    """The partition sectors of C:\\SUB's first cluster (found in the root)."""
    for i in range(ROOTSEC * 16):
        o = ROOT0 * 512 + 32 * i
        if d[o] == 0:
            break
        if d[o:o + 11] == b'SUB        ' and d[o + 11] & 0x10:
            first = DATA0 + (u16(d, o + 26) - 2) * CLSIZ
            return range(first, first + CLSIZ)
    return range(0)


def normalise(d):
    d = bytearray(d)
    for s in list(range(ROOT0, DATA0)) + list(sub_sectors(d)):
        for e in range(16):
            o = s * 512 + 32 * e
            if d[o] == 0:
                continue
            d[o + 13:o + 20] = bytes(7)
            d[o + 22:o + 26] = bytes(4)
    return bytes(d)


def diff(a, b):
    out = []
    for s in range(len(a) // 512):
        x, y = a[s * 512:(s + 1) * 512], b[s * 512:(s + 1) * 512]
        if x != y:
            offs = [i for i in range(512) if x[i] != y[i]]
            out.append(f'    sector {s}: {len(offs)} bytes, first at 0x{offs[0]:03X}'
                       f' ({x[offs[0]]:02X} vs {y[offs[0]]:02X})')
    return out


S, R = sys.argv[1], sys.argv[2]
with open(os.path.join(S, 'test.img'), 'rb') as f:
    f.seek(PSTART * 512)
    orig = f.read(PSIZE * 512)
img = {'original': orig}
for n in ('oracle-fs24-1', 'oracle-fs24-2', 'newtos-fs24'):
    with open(os.path.join(S, 'end', n + '.img'), 'rb') as f:
        img[n] = f.read()
lines = []
for n, d in img.items():
    lines.append(f'{n:15s} sha256 {hashlib.sha256(d).hexdigest()}')
for a, b in (('original', 'oracle-fs24-1'), ('oracle-fs24-1', 'oracle-fs24-2'),
             ('oracle-fs24-1', 'newtos-fs24')):
    for mode, f in (('raw', lambda x: x), ('time stamps zeroed', normalise)):
        d = diff(f(img[a]), f(img[b]))
        lines.append(f'{a} vs {b}, {mode}: ' + ('IDENTICAL' if not d else f'{len(d)} sectors differ'))
        lines.extend(d)
with open(os.path.join(R, 'endstate-fs24.txt'), 'w') as f:
    f.write('\n'.join(lines) + '\n')
print('\n'.join(lines))
