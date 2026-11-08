#!/usr/bin/env python3
"""loops.py <objdump-file> : the innermost loops of a disassembled function (V58 code check).

Lists every backward branch whose body holds no other backward branch target (the innermost loops), with
its length and instruction mnemonics, and flags a division or call inside. Used on
`kernel::raster_line::abline` of the release rpi4 kernel: the per-pixel loops must hold no `udiv`/`sdiv`,
no `bl`/`blr` (no dyn, no call).
"""
import re, sys

ins = []
for l in open(sys.argv[1]):
    m = re.match(r'\s*([0-9a-f]+):\s+(\S+)\s*(.*)', l)
    if m:
        ins.append((int(m.group(1), 16), m.group(2), m.group(3)))
addr = {a: i for i, (a, _, _) in enumerate(ins)}
BR = ('b', 'cbz', 'cbnz', 'tbz', 'tbnz')
loops = []
for i, (a, op, args) in enumerate(ins):
    if op in BR or op.startswith('b.'):
        m = re.search(r'0x([0-9a-f]+)', args)
        if m:
            t = int(m.group(1), 16)
            if t <= a and t in addr:
                loops.append((addr[t], i))
for s, e in loops:
    inner = not any(s < s2 <= e2 < e or (s <= s2 and e2 < e and (s2, e2) != (s, e)) for s2, e2 in loops)
    if not inner or e - s > 40:
        continue
    ops = [o for _, o, _ in ins[s:e + 1]]
    flag = [o for o in ops if o in ('udiv', 'sdiv', 'bl', 'blr', 'br')]
    print(f"loop {ins[s][0]:x}-{ins[e][0]:x} len {len(ops)} {'DIV/CALL ' + str(flag) if flag else 'no div, no call'} :: {' '.join(ops)}")
