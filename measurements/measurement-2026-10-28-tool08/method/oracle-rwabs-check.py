#!/usr/bin/env python3
"""oracle-rwabs-check.py <repo> <hatari.log> <table.tsv> -- TOOL08 oracle-side probe.

Reads the argument blocks the _lrwabs breakpoint of oracle-traced.sh dumped into
hatari.log (`m "a7"-"a7+20"` at the entry of lrwabs(WORD r_w, UBYTE *adr, WORD numb,
WORD first, WORD drive, LONG lfirst), bios/bios.c:1045: 0(sp) return address, 4 r_w,
6 adr, 10 numb, 12 first, 14 drive, 16 lfirst; big-endian) and compares the ones after
the dumper's "running N cases" line, in order, with the table's Rwabs calls (preludes and
cases in issue order, as tools/biostrace-check.py orders them). A b:<n> buffer must be a
non-zero address, the same for the same n. Exit 0 when every block matches.
"""
import importlib.util
import re
import sys

repo, log, table = sys.argv[1:4]
spec = importlib.util.spec_from_file_location('bc', f'{repo}/tools/biostrace-check.py')
bc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bc)

calls = [c for c in bc.load_table(table) if (c[2], c[3]) == (13, 4)]
text = open(log, encoding='latin-1').read()
start = text.find('running ')
dumps = re.findall(r'^([0-9A-F]{8}): ((?:[0-9a-f]{2} ){16})\s.*\n([0-9A-F]{8}): ((?:[0-9a-f]{2} ){4})',
                   text[start:], re.M)
blocks = []
for _, a, _, b in dumps:
    raw = bytes.fromhex((a + b).replace(' ', ''))
    blocks.append((int.from_bytes(raw[0:4], 'big'),
                   [('w', int.from_bytes(raw[4:6], 'big')), ('l', int.from_bytes(raw[6:10], 'big')),
                    ('w', int.from_bytes(raw[10:12], 'big')), ('w', int.from_bytes(raw[12:14], 'big')),
                    ('w', int.from_bytes(raw[14:16], 'big')), ('l', int.from_bytes(raw[16:20], 'big'))]))
print(f'table {table}: {len(calls)} Rwabs calls (preludes + cases); log: {len(blocks)} lrwabs entries '
      f'after "running"')
bad = 0
addr = {}
for k, c in enumerate(calls):
    if k >= len(blocks):
        print(f'FAIL line {c[0]}: no lrwabs entry'); bad += 1; continue
    ret, got = blocks[k]
    probs = []
    for j, (tk, tv) in enumerate(c[4]):
        gk, gv = got[j]
        if tk == 'w':
            ok = gv == tv & 0xFFFF
        elif tk == 'l':
            ok = gv == tv & 0xFFFFFFFF
        else:  # b:<n> -- the buffer address, consistent per n
            ok = gv != 0 and addr.setdefault((tk, tv), gv) == gv
        if not ok:
            probs.append(f'arg{j} table {tk}:{tv:#x} arrived {gv:#x}')
    print(f'{"FAIL" if probs else "ok  "} line {c[0]:4} {c[1]:7} ret 0x{ret:08x} '
          + ' '.join(f'{gk}:0x{gv:x}' for gk, gv in got))
    for p in probs:
        print(f'       {p}')
    bad += bool(probs)
extra = len(blocks) - len(calls)
if extra > 0:
    print(f'FAIL {extra} lrwabs entr(y/ies) after the table\'s calls'); bad += 1
print(f'RESULT {"PASS" if not bad else "FAIL"}: {len(calls)} Rwabs calls, 6 arguments each, '
      f'{bad} differ')
sys.exit(1 if bad else 0)
