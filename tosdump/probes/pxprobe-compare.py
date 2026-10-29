#!/usr/bin/env python3
"""pxprobe-compare.py -- the answers and end states of pxprobe runs (TASK/FS22).

usage: pxprobe-compare.py <emutos-out-dir> <newtos-out-dir> [<label>=<out-dir> ...]

Each out-dir is one of tosdump/probes/run-pxprobe.sh. The first is the
reference (the oracle). Per run: the probe's step lines (P01-P07, F01-F07)
from PXPROBE.LOG, compared line by line with the reference's -- P05 is the
Pexec that meets the media change (EmuTOS: the child's 42), P07 the caller's
DTA after it, F04 the DTA of the Fsfirst that meets the change; and the test
partition after the run (testpart-after.img): its sha256, whether it is the
run's own partition before the run (testpart-before.img; the probe writes
the boot sector back unchanged and writes nothing else to C:), and whether it
is byte for byte the reference's.

Exit 0 when every run's PXPROBE.LOG equals the reference's and every test
partition is unchanged and equal to the reference's, 1 otherwise, 2 on a
usage error.
"""
import hashlib
import os
import sys

SECTOR = 512


def differing(a, b):
    return [s for s in range(max(len(a), len(b)) // SECTOR)
            if a[s * SECTOR:(s + 1) * SECTOR] != b[s * SECTOR:(s + 1) * SECTOR]]


def steps(log):
    """The probe's step lines by id (first word)."""
    out = {}
    for line in log.splitlines():
        if line and line[0] in 'PF' and line[1:3].isdigit():
            out[line.split(' ', 1)[0]] = line
    return out


def main(argv):
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    runs = []
    for i, a in enumerate(argv[1:]):
        label, path = a.split('=', 1) if '=' in a else (('emutos', 'newtos')[min(i, 1)], a)
        log = open(os.path.join(path, 'PXPROBE.LOG'), 'rb').read().decode('latin-1')
        before = open(os.path.join(path, 'testpart-before.img'), 'rb').read()
        after = open(os.path.join(path, 'testpart-after.img'), 'rb').read()
        runs.append((label, log, before, after))
    ref_label, ref_log, _ref_before, ref_after = runs[0]
    ok = True
    for label, log, before, after in runs:
        st = steps(log)
        print(f'{label}:')
        for k in sorted(st):
            print(f'  {st[k]}')
        if 'END' not in log.split():
            print('  PXPROBE.LOG has no END line')
            ok = False
        own = differing(after, before)
        print(f'  testpart-after sha256 {hashlib.sha256(after).hexdigest()}')
        print('  testpart after vs before: '
              + ('identical' if not own else f'DIFFERENT in partition sectors {own[:20]}'))
        ok = ok and not own
        if label != ref_label:
            same_log = st == steps(ref_log)
            print(f'  PXPROBE.LOG vs {ref_label}: {"identical" if same_log else "DIFFERENT"}')
            if not same_log:
                rs = steps(ref_log)
                for k in sorted(set(st) | set(rs)):
                    if st.get(k) != rs.get(k):
                        print(f'    {k}: {ref_label} {rs.get(k)!r} / {label} {st.get(k)!r}')
            diff = differing(after, ref_after)
            print(f'  testpart vs {ref_label}: '
                  + ('identical' if not diff else f'DIFFERENT in partition sectors {diff[:20]}'))
            ok = ok and same_log and not diff
    print('VERDICT: ' + ('equal to the reference' if ok else 'differs from the reference'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv))
