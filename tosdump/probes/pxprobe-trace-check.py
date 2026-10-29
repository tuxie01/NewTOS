#!/usr/bin/env python3
"""pxprobe-trace-check.py -- the input check of a pxprobe run on NewTOS (TASK/FS22).

usage: pxprobe-trace-check.py <newtos-out-dir>

Reads <out-dir>/BOOT-SERIAL.txt of tosdump/probes/run-pxprobe.sh with a kernel
built with `--features bios-arg-trace` (kernel/src/dbg_biostrace.rs, line
format docs/TOS-REFERENCE-DUMPER.md section 6, "Input check on NewTOS") and
<out-dir>/PXPROBE.LOG. The method of gcprobe-trace-check.py (TASK/FS20):
the BIOS calls the probe makes (tosdump/probes/pxprobe.c) are found, in
order, as one contiguous run in the trace of one EL0 caller, and every
argument is compared with what the source passes, at the width of the EmuTOS
call form; a register that is not the sign extension of its printed value
("=0x..." annotation) is a difference. The traced result of each call is
checked against the value the probe logged for that step. The caller's other
BIOS/XBIOS calls (C start-up) and those of every other EL0 caller (the
probe's Pexec child) are listed, and every Rwabs/Getbpb/Mediach of the kernel
itself (caller K) too -- the BDOS's re-login does not use the trap, so none
is expected.

Exit 0 when every call matches, 1 on a difference, 2 on a usage or parse error.
"""
import os
import re
import sys

LINE = re.compile(
    r'\[biostrace\] (\d+) (K|U\S+)\s+(\d+):0x([0-9a-f]+)(?:=0x([0-9a-f]+))? (\S+)\s+(.*?) -> (0x[0-9a-f]+|upcall)\s*$')

RW = lambda flag: ['w:0x%04x' % flag, 'p:BUF', 'w:0x0001', 'w:0x0000', 'w:0x0002', 'l:0x00000000']
MEDIACH = (13, 0x09, ['w:0x0002'])
GETBPB = (13, 0x07, ['w:0x0002'])

# The BIOS calls of pxprobe.c, in its order; Fsfirst/Pexec/... are GEMDOS.
EXPECTED = [
    ('P02', 13, 0x04, RW(2)), ('P03', 13, 0x04, RW(3)), ('P04', *MEDIACH), ('P06', *MEDIACH),
    ('F01', 13, 0x04, RW(2)), ('F02', 13, 0x04, RW(3)), ('F03', *MEDIACH), ('F05', *MEDIACH),
]


def logged(log):
    """step -> the value the probe logged: ret=<n>, or nz=<0|1> for Getbpb."""
    out = {}
    for line in log.splitlines():
        m = re.match(r'^([PF]\d\d) .*?(ret|nz)=(-?\d+)', line)
        if m:
            out[m.group(1)] = (m.group(2), int(m.group(3)))
    return out


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    d = argv[1]
    serial = open(os.path.join(d, 'BOOT-SERIAL.txt'), 'rb').read().decode('latin-1')
    log = logged(open(os.path.join(d, 'PXPROBE.LOG'), 'rb').read().decode('latin-1'))
    calls = []
    for raw in serial.splitlines():
        m = LINE.search(raw)
        if m:
            calls.append(dict(seq=int(m.group(1)), who=m.group(2), trap=int(m.group(3)),
                              fn=int(m.group(4), 16), name=m.group(6), args=m.group(7).split(),
                              ret=m.group(8)))
    if not calls:
        print('no [biostrace] lines: was the kernel built with --features bios-arg-trace?')
        return 2

    def fits(c, e):
        return c['trap'] == e[1] and c['fn'] == e[2]

    found = None
    for who in sorted({c['who'] for c in calls if c['who'] != 'K'}):
        seq = [c for c in calls if c['who'] == who]
        for i in range(len(seq) - len(EXPECTED) + 1):
            if all(fits(seq[i + k], e) for k, e in enumerate(EXPECTED)):
                found = (who, seq, i)
                break
        if found:
            break
    if not found:
        print('the probe\'s BIOS call sequence is not in the trace of any EL0 caller')
        return 1
    who, seq, i = found
    run = seq[i:i + len(EXPECTED)]
    print(f'caller {who}: the probe\'s {len(EXPECTED)} BIOS calls at trace numbers '
          f'{run[0]["seq"]}..{run[-1]["seq"]}')
    ok = True
    buf = None
    for c, (step, trap, fn, want) in zip(run, EXPECTED):
        bad = []
        if len(c['args']) != len(want):
            bad.append(f'{len(c["args"])} arguments, call form has {len(want)}')
        for k, (got, w) in enumerate(zip(c['args'], want)):
            if '=' in got:
                bad.append(f'arg {k} {got}: register not the sign extension')
            elif w == 'p:BUF':
                if not got.startswith('p:') or int(got[2:], 16) == 0:
                    bad.append(f'arg {k} {got}: not a buffer address')
                elif buf is None:
                    buf = got
                elif got != buf:
                    bad.append(f'arg {k} {got}: not the buffer of the first call ({buf})')
            elif got != w:
                bad.append(f'arg {k} {got} != {w}')
        r = int(c['ret'], 16)
        r = r - (1 << 64) if r >= 1 << 63 else r
        kind, val = log.get(step, (None, None))
        if kind == 'ret' and r != val:
            bad.append(f'result {r}, the probe logged {val}')
        if kind == 'nz' and (r != 0) != (val == 1):
            bad.append(f'result {c["ret"]}, the probe logged nz={val}')
        if kind is None:
            bad.append('step not in PXPROBE.LOG')
        print(f'  {step} #{c["seq"]:>6} {trap}:0x{fn:02x} {c["name"]:<8} {" ".join(c["args"])} -> {c["ret"]}'
              + ('  OK' if not bad else '  DIFF: ' + '; '.join(bad)))
        ok = ok and not bad
    others = seq[:i] + seq[i + len(EXPECTED):]
    print(f'other BIOS/XBIOS calls of {who}: {len(others)}')
    for c in others:
        print(f'  #{c["seq"]:>6} {c["trap"]}:0x{c["fn"]:02x} {c["name"]} {" ".join(c["args"])} -> {c["ret"]}')
    for other in sorted({c['who'] for c in calls if c['who'] not in ('K', who)}):
        oc = [c for c in calls if c['who'] == other]
        print(f'BIOS/XBIOS calls of {other}: {len(oc)}')
        for c in oc:
            print(f'  #{c["seq"]:>6} {c["trap"]}:0x{c["fn"]:02x} {c["name"]} {" ".join(c["args"])} -> {c["ret"]}')
    kdrive = [c for c in calls if c['who'] == 'K' and c['trap'] == 13 and c['fn'] in (4, 7, 9)]
    print(f'kernel (K) Rwabs/Getbpb/Mediach calls: {len(kdrive)}')
    for c in kdrive:
        print(f'  #{c["seq"]:>6} {c["name"]} {" ".join(c["args"])} -> {c["ret"]}')
    print('VERDICT: ' + ('every argument and result as the probe passes and logs' if ok else 'DIFFERENCES'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv))
