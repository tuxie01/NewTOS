#!/usr/bin/env python3
# elfsecdiff.py A B -- compare two ELF64 LE relocatable objects section by
# section (header fields except sh_offset, plus contents) and the ELF header
# (except e_shoff). Prints one line: "same", or "diff <what>,<what>...".
import struct, sys

def parse(p):
    d = open(p, 'rb').read()
    assert d[:4] == b'\x7fELF' and d[4] == 2 and d[5] == 1, p
    eh = list(struct.unpack_from('<16sHHIQQQIHHHHHH', d, 0))
    shoff, shentsize, shnum, shstrndx = eh[6], eh[11], eh[12], eh[13]
    secs = []
    for i in range(shnum):
        s = struct.unpack_from('<IIQQQQIIQQ', d, shoff + i * shentsize)
        secs.append(s)
    so = secs[shstrndx]
    strtab = d[so[4]:so[4] + so[5]]
    out = []
    for s in secs:
        name = strtab[s[0]:strtab.index(b'\0', s[0])].decode()
        body = b'' if s[1] == 8 else d[s[4]:s[4] + s[5]]   # SHT_NOBITS
        hdr = (s[0], s[1], s[2], s[3], s[5], s[6], s[7], s[8], s[9])  # all but sh_offset
        out.append((name, hdr, body))
    eh[6] = None  # e_shoff
    return tuple(eh), out

ea, sa = parse(sys.argv[1]); eb, sb = parse(sys.argv[2])
diffs = []
if ea != eb: diffs.append('ELFHDR')
if [n for n, _, _ in sa] != [n for n, _, _ in sb]:
    diffs.append('SECTIONLIST')
else:
    for (n, ha, ba), (_, hb, bb) in zip(sa, sb):
        if ha != hb: diffs.append(n + '[hdr]')
        if ba != bb: diffs.append(n)
print('same' if not diffs else 'diff ' + ','.join(diffs))
