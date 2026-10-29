#!/usr/bin/env python3
"""compare-ends.py <scratch> <runs-dir> -- the end-state rule of a destructive table
(docs/TOS-REFERENCE-DUMPER.md section 6, "The closing tables") for the three gemdos-mc
tables of TASK/TOOL20, on the TEST PARTITION only (LBA 63, 130977 sectors):
  original vs oracle-<t>-1        which sectors the table changed
  oracle-<t>-1 vs oracle-<t>-2    the oracle reproduces its end state
  oracle-<t>-1 vs newtos-<t>      both sides end in the same state (THE RULE)
each raw and with the root directory's FAT time stamps zeroed (bytes 13-19 and 22-25 of
every root entry, as cargo xtask tosdump's normalise_root_times and the FS20 run: each side
stamps a file it creates with its own clock). A difference names the partition sector and
byte offset. Input <scratch>/end/{original,oracle-<t>-<i>,newtos-<t>}.img; output
<runs-dir>/endstate-<t>.txt."""
import hashlib
import os
import sys


def normalise(d):
    d = bytearray(d)
    u16 = lambda o: d[o] | d[o + 1] << 8
    bps, res, nfats, nroot, spf = u16(11), u16(14), d[16], u16(17), u16(22)
    start = (res + nfats * spf) * bps
    for k in range(nroot):
        o = start + 32 * k
        if o + 32 > len(d) or d[o] == 0:
            break
        for i in list(range(13, 20)) + list(range(22, 26)):
            d[o + i] = 0
    return bytes(d)


def cmp(label, a, b, out):
    sa, sb = hashlib.sha256(a).hexdigest(), hashlib.sha256(b).hexdigest()
    if a == b:
        out.append(f'{label}: IDENTICAL ({sa[:16]})')
        return True
    diffs = [i for i in range(min(len(a), len(b))) if a[i] != b[i]]
    na, nb = normalise(a), normalise(b)
    norm = na == nb
    out.append(f'{label}: DIFFERENT ({sa[:16]} vs {sb[:16]}), {len(diffs)} bytes; '
               + ('IDENTICAL after zeroing the root directory time stamps' if norm
                  else 'still different after zeroing the root directory time stamps'))
    shown = diffs if not norm else diffs[:20]
    if not norm:
        shown = [i for i in range(min(len(na), len(nb))) if na[i] != nb[i]]
        out.append(f'  differing bytes after the normalisation: {len(shown)}')
    for i in shown[:40]:
        out.append(f'  sector {i // 512} offset 0x{i % 512:03X}: {a[i]:02X} {b[i]:02X}')
    return norm


S, R = sys.argv[1], sys.argv[2]
E = os.path.join(S, 'end')
img = lambda n: open(os.path.join(E, n + '.img'), 'rb').read()
for t in ('fs20', 'fs22', 'paths'):
    out = [f'end state of gemdos-mc-{t}.tsv (test partition, LBA 63 + 130977)']
    o1 = img(f'oracle-{t}-1')
    cmp('original vs oracle-1', img('original'), o1, out)
    if os.path.exists(os.path.join(E, f'oracle-{t}-2.img')):
        cmp('oracle-1 vs oracle-2', o1, img(f'oracle-{t}-2'), out)
    if os.path.exists(os.path.join(E, f'newtos-{t}.img')):
        cmp('original vs newtos', img('original'), img(f'newtos-{t}'), out)
        cmp('oracle-1 vs newtos (THE RULE)', o1, img(f'newtos-{t}'), out)
    else:
        out.append('newtos: NO END STATE (no testpart-after.img)')
    open(os.path.join(R, f'endstate-{t}.txt'), 'w').write('\n'.join(out) + '\n')
    print('\n'.join(l for l in out if not l.startswith('  sector')))
