#!/usr/bin/env python3
"""compare-ends.py <scratch> <runs-dir> -- TASK/FS21: the end state of the test partition
(LBA 63, 130977 sectors) after each destructive gemdos-mc table, oracle vs NewTOS before (HEAD)
and after (FS21), after measurements/measurement-2026-10-29-fs24/method/compare-ends.py:
raw and with the FAT time stamps (bytes 13-19 and 22-25 of a directory entry) zeroed in the
root directory and in C:\\SUB's first cluster -- each side stamps a file it creates with its
own clock. The geometry is read from each image's own boot sector (gemdos-mc-fs20 changes it).
A difference names the partition sector, its byte offsets and the bytes (left vs right).
Input <scratch>/end/{original,oracle-<t>-1,before-<t>,newtos-<t>}.img; output
<runs-dir>/endstate.txt."""
import hashlib
import os
import sys

TABLES = ('fs20', 'fs22', 'paths', 'fs24')


def u16(d, o):
    return d[o] | d[o + 1] << 8


def geometry(d):
    bps, clsiz, res, nfats, nroot, fsiz = u16(d, 11), d[13], u16(d, 14), d[16], u16(d, 17), u16(d, 22)
    root0 = res + nfats * fsiz
    data0 = root0 + (nroot * 32 + bps - 1) // bps
    return clsiz, root0, data0


def dir_sectors(d):
    clsiz, root0, data0 = geometry(d)
    secs = list(range(root0, data0))
    for s in range(root0, data0):
        for e in range(16):
            o = s * 512 + 32 * e
            if d[o] == 0:
                break
            if d[o:o + 11] == b'SUB        ' and d[o + 11] & 0x10:
                first = data0 + (u16(d, o + 26) - 2) * clsiz
                secs += list(range(first, first + clsiz))
    return secs


def normalise(d):
    d = bytearray(d)
    for s in dir_sectors(d):
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
            shown = ', '.join(f'0x{i:03X} {x[i]:02X}/{y[i]:02X}' for i in offs[:8])
            out.append(f'    sector {s}: {len(offs)} bytes: {shown}' + (' ...' if len(offs) > 8 else ''))
    return out


S, R = sys.argv[1], sys.argv[2]
lines = []
for t in TABLES:
    img = {}
    for n in ('original', f'oracle-{t}-1', f'before-{t}', f'newtos-{t}'):
        with open(os.path.join(S, 'end', n + '.img'), 'rb') as f:
            img[n] = f.read()
    lines.append(f'== gemdos-mc-{t}')
    for n, d in img.items():
        lines.append(f'{n:15s} sha256 {hashlib.sha256(d).hexdigest()}')
    for a, b in (('original', f'oracle-{t}-1'), (f'oracle-{t}-1', f'before-{t}'),
                 (f'oracle-{t}-1', f'newtos-{t}'), (f'before-{t}', f'newtos-{t}')):
        for mode, f in (('raw', lambda x: x), ('time stamps zeroed', normalise)):
            d = diff(f(img[a]), f(img[b]))
            lines.append(f'{a} vs {b}, {mode}: ' + ('IDENTICAL' if not d else f'{len(d)} sectors differ'))
            lines.extend(d)
with open(os.path.join(R, 'endstate.txt'), 'w') as f:
    f.write('\n'.join(lines) + '\n')
print('\n'.join(lines))
