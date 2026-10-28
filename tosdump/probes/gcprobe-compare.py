#!/usr/bin/env python3
"""gcprobe-compare.py -- the end states of gcprobe runs (TASK/FS20).

usage: gcprobe-compare.py <emutos-out-dir> <newtos-out-dir> [<label>=<out-dir> ...]

Each out-dir is one of tosdump/probes/run-gcprobe.sh. The first is the
reference (the oracle). Per run: the probe's answers to the steps the ticket
names (A07, B04, B05) from GCPROBE.LOG, and of the test partition after the
run (testpart-after.img): its sha256, the root-entry count of its boot sector
(bytes 0x11-0x12), the partition sectors holding the probe's marker
"GCPROBE-STALE-GEOMETRY-MARKER-01" with the cluster they are by the boot
sector on disk, and whether it is byte for byte the reference's -- raw, and
if not, after zeroing the time stamps of every root directory entry (each side
stamps GCPROBE.DAT with its own clock; the normalisation of cargo xtask
tosdump's disk round, tools/xtask/src/tosdump.rs normalise_root_times). Every
run is also compared line by line with the reference's GCPROBE.LOG.

Exit 0 when every run's GCPROBE.LOG equals the reference's and its test
partition does (raw or after the normalisation), 1 otherwise, 2 on a usage
error.
"""
import hashlib
import os
import sys

MARKER = b'GCPROBE-STALE-GEOMETRY-MARKER-01'
SECTOR = 512


def bpb(img):
    """recsiz, clsiz, reserved, nfats, rootents, fsiz of the boot sector."""
    le = lambda o, n: int.from_bytes(img[o:o + n], 'little')
    return le(0x0B, 2), img[0x0D], le(0x0E, 2), img[0x10], le(0x11, 2), le(0x16, 2)


def datrec(img):
    recsiz, _clsiz, reserved, nfats, rootents, fsiz = bpb(img)
    return reserved + nfats * fsiz + (rootents * 32 + recsiz - 1) // recsiz


def normalise_root_times(img):
    """Zero bytes 13..19 and 22..25 of every root directory entry up to the
    first free one (tools/xtask/src/tosdump.rs normalise_root_times)."""
    d = bytearray(img)
    recsiz, _clsiz, reserved, nfats, rootents, fsiz = bpb(img)
    start = (reserved + nfats * fsiz) * recsiz
    for k in range(rootents):
        off = start + 32 * k
        if off + 32 > len(d) or d[off] == 0:
            break
        for i in list(range(13, 20)) + list(range(22, 26)):
            d[off + i] = 0
    return bytes(d)


def differing(a, b):
    return [s for s in range(max(len(a), len(b)) // SECTOR)
            if a[s * SECTOR:(s + 1) * SECTOR] != b[s * SECTOR:(s + 1) * SECTOR]]


def steps(log):
    """The probe's step lines by id (first word)."""
    out = {}
    for line in log.splitlines():
        if line and line[0] in 'AB' and line[1:3].isdigit():
            out[line.split(' ', 1)[0]] = line
    return out


def main(argv):
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    runs = []
    for i, a in enumerate(argv[1:]):
        label, path = a.split('=', 1) if '=' in a else (('emutos', 'newtos')[min(i, 1)], a)
        log = open(os.path.join(path, 'GCPROBE.LOG'), 'rb').read().decode('latin-1')
        img = open(os.path.join(path, 'testpart-after.img'), 'rb').read()
        runs.append((label, log, img))
    ref_label, ref_log, ref_img = runs[0]
    ok = True
    for label, log, img in runs:
        st = steps(log)
        d = datrec(img)
        _, clsiz, *_ = bpb(img)
        secs = [s for s in range(len(img) // SECTOR) if img[s * SECTOR:s * SECTOR + 32] == MARKER]
        print(f'{label}:')
        for k in ('A05', 'A06', 'A07', 'B03', 'B04', 'B05', 'B06', 'B07', 'B08'):
            print(f'  {st.get(k, k + " (missing)")}')
        print(f'  testpart-after sha256 {hashlib.sha256(img).hexdigest()}')
        print(f'  root entries {bpb(img)[4]}, data area (datrec) {d}')
        for s in secs:
            print(f'  marker in partition sector {s} = cluster {2 + (s - d) // clsiz} by the boot sector on disk')
        if not secs:
            print('  marker: not in the partition')
        if label != ref_label:
            same_log = log == ref_log
            diff = differing(img, ref_img)
            print(f'  GCPROBE.LOG vs {ref_label}: {"identical" if same_log else "DIFFERENT"}')
            if not same_log:
                rs = steps(ref_log)
                for k in sorted(set(st) | set(rs)):
                    if st.get(k) != rs.get(k):
                        print(f'    {k}: {ref_label} {rs.get(k)!r} / {label} {st.get(k)!r}')
            if not diff:
                print(f'  testpart vs {ref_label}: identical')
                same_part = True
            else:
                ndiff = differing(normalise_root_times(img), normalise_root_times(ref_img))
                print(f'  testpart vs {ref_label}: raw DIFFERENT in partition sectors '
                      f'{diff[:20]}{" ..." if len(diff) > 20 else ""}; after zeroing the root '
                      'directory time stamps: '
                      + ('identical' if not ndiff else f'DIFFERENT in {ndiff[:20]}'))
                same_part = not ndiff
                na, nb = normalise_root_times(img), normalise_root_times(ref_img)
                for sec in ndiff[:8]:
                    offs = [i for i in range(SECTOR) if na[sec * SECTOR + i] != nb[sec * SECTOR + i]]
                    print(f'    sector {sec}: bytes ' + ', '.join(
                        f'0x{o:03x} (entry {o // 32} +0x{o % 32:02x}) {ref_label} '
                        f'{nb[sec * SECTOR + o]:02x} / {label} {na[sec * SECTOR + o]:02x}' for o in offs[:16]))
            ok = ok and same_log and same_part
    print('VERDICT: ' + ('equal to the reference' if ok else 'differs from the reference'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv))
