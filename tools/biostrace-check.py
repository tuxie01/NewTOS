#!/usr/bin/env python3
"""biostrace-check -- check a NewTOS BIOS/XBIOS argument trace against a call table.

The input check of a DUMPBIOS run on NewTOS (TOOL08, "inputs before outputs"):
did the arguments that arrived at the trap #13/#14 dispatcher match what the
case table says? The trace is the serial log of a kernel built with the cargo
feature `bios-arg-trace` (`kernel/src/dbg_biostrace.rs`): one `[biostrace]`
line per completed BIOS/XBIOS call, its arguments at the width of the EmuTOS
call form. Procedure: docs/TOS-REFERENCE-DUMPER.md section 6, "Input check on
NewTOS".

usage: biostrace-check.py SERIAL.txt TABLE.tsv [--bin DUMP.BIN] [--caller U0]

How the trace is cut to the table: the table's calls in the order the runner
issues them (per case its preludes, then the case; tosdump/core/runner_call.c
td_run_call) are searched as one contiguous run in the trace of one caller --
the first caller (U<slot>/K) whose BIOS/XBIOS sequence contains it, unless
--caller names one. Every call of that caller outside the run is listed; a
foreign call inside the run breaks the match and is reported where it stands.

Per argument (table column `args`, docs/TOS-REFERENCE-DUMPER.md section 6):

  w:<v>  against a w: slot: the low 16 bits equal, and the register was the
         sign extension (no "=" annotation: the backend sign-narrows a WORD,
         tosdump/abi/newtos_svc.c)
  l:<v>  against an l: slot: the low 32 bits equal, register sign-extended;
         against a p: slot: the register equals <v> sign-extended to 64 bits
  b:<n>  (buffer), f:<n> (callback): a p: or l: slot holding a non-null,
         non-poison address, the same address for the same n in the whole run
  t:0    the Super token: not in the trace (Super is GEMDOS, not traced) --
         reported UNCHECKED
  a slot of the call form beyond the table's arguments must hold the poison
  0xDEADDEAD sign-extended (runner_call.c issue()); a table argument beyond
  the call form is BEYOND-FORM (the EmuTOS handler has no such parameter).
  A table width that differs from the call form's (w: against l:/p:, l: or
  b: against w:) is WIDTH.

--bin cross-checks the alignment against the dump of the same run: record k
has the case id (= table line), the trap and the function of the k-th case,
and its 64-bit return (ret_hi:ret) equals the result the trace shows.

Exit 0 when every row matches, 1 on any difference, 2 on a usage or parse
error.
"""
import argparse
import re
import struct
import sys

POISON32 = 0xDEADDEAD
POISON64 = 0xFFFFFFFFDEADDEAD

LINE = re.compile(
    r'\[biostrace\] (\d+) (K|U\S+)\s+(\d+):0x([0-9a-f]+)(?:=0x([0-9a-f]+))? (\S+)\s+(.*?) -> (0x[0-9a-f]+|upcall)\s*$')
ARG = re.compile(r'^([wlp]):0x([0-9a-f]+)(?:=0x([0-9a-f]+))?$')


def value32(s):
    """tosdump/core/cases.c parse_value32: decimal or 0x hex, optional sign,
    -2147483648..4294967295, returned as the 32-bit pattern."""
    t = s.strip()
    neg = t.startswith('-')
    if t[:1] in '+-':
        t = t[1:]
    v = int(t, 16) if t[:2].lower() == '0x' and len(t) > 2 else int(t, 10)
    if neg:
        if v > 0x80000000:
            raise ValueError(s)
        v = (-v) & 0xFFFFFFFF
    if v > 0xFFFFFFFF:
        raise ValueError(s)
    return v


def sext32(v):
    return (v - (1 << 32)) & 0xFFFFFFFFFFFFFFFF if v & 0x80000000 else v


def load_table(path):
    """The calls in issue order: [(line, 'prelude'|'case', trap, fn, [(kind, value)])]."""
    calls, pending = [], []
    with open(path, encoding='latin-1') as f:
        for no, raw in enumerate(f, 1):
            s = raw.rstrip('\r\n')
            st = s.lstrip()
            if not st or st.startswith('#') or st.startswith('!'):
                continue
            cols = s.split('\t')
            op = cols[0].strip()
            prelude = op.startswith('+')
            if prelude:
                op = op[1:]
            trap_s, fn_s = op.split(':')
            trap, fn = value32(trap_s), value32(fn_s) & 0xFFFF
            args = []
            a = cols[1].strip() if len(cols) > 1 else '-'
            if a != '-':
                for tok in a.split(','):
                    k, v = tok.strip().split(':', 1)
                    args.append((k, value32(v) if k in 'wl' else int(v, 0)))
            entry = (no, 'prelude' if prelude else 'case', trap, fn, args)
            if prelude:
                pending.append(entry)
            else:
                calls.extend(pending)
                pending = []
                calls.append(entry)
    if pending:
        raise ValueError('prelude without a case after it')
    return calls


def load_trace(path):
    recs = []
    with open(path, 'rb') as f:
        text = f.read().decode('latin-1')
    for ln, line in enumerate(text.splitlines(), 1):
        m = LINE.search(line)
        if not m:
            continue
        seq, who, trap, fn, x8, name, args, res = m.groups()
        parsed = []
        if args.strip() != '-':
            for tok in args.split():
                am = ARG.match(tok)
                if not am:
                    raise ValueError(f'serial line {ln}: bad argument {tok!r}')
                k, v, rawv = am.groups()
                parsed.append((k, int(v, 16), int(rawv, 16) if rawv else None))
        recs.append({'serial': ln, 'seq': int(seq), 'who': who, 'trap': int(trap),
                     'fn': int(fn, 16), 'x8': x8, 'name': name, 'args': parsed,
                     'res': None if res == 'upcall' else int(res, 16)})
    return recs


def find_run(keys, want):
    n = len(want)
    for i in range(len(keys) - n + 1):
        if keys[i:i + n] == want:
            return i, n
    best = (0, 0)
    for i in range(len(keys)):
        k = 0
        while k < n and i + k < len(keys) and keys[i + k] == want[k]:
            k += 1
        if k > best[1]:
            best = (i, k)
    return None, best


def check_args(entry, rec, addr):
    """Compare one table call with its trace record; returns a list of problems."""
    probs, unchecked = [], []
    targs, slots = entry[4], rec['args']
    for j, slot in enumerate(slots):
        k, v, raw = slot
        reg = raw if raw is not None else (v if k == 'p' else (sext32(v) if k == 'l' else
                                           ((v - 0x10000) & 0xFFFFFFFFFFFFFFFF if v & 0x8000 else v)))
        if j >= len(targs):
            want = {'w': POISON32 & 0xFFFF, 'l': POISON32, 'p': POISON64}[k]
            if v != want or (k != 'p' and reg != POISON64):
                probs.append(f'arg{j} {k}: unused slot is 0x{reg:x}, not the poison 0x{POISON64:x}')
            continue
        tk, tv = targs[j]
        if tk == 'w':
            if k != 'w':
                probs.append(f'arg{j} WIDTH: table w:, call form {k}:')
            elif v != tv & 0xFFFF:
                probs.append(f'arg{j} w: table 0x{tv & 0xFFFF:04x}, arrived 0x{v:04x}')
            elif raw is not None:
                probs.append(f'arg{j} w:0x{v:04x}: register 0x{raw:016x} is not its sign extension')
        elif tk == 'l':
            if k == 'l':
                if v != tv:
                    probs.append(f'arg{j} l: table 0x{tv:08x}, arrived 0x{v:08x}')
                elif raw is not None:
                    probs.append(f'arg{j} l:0x{v:08x}: register 0x{raw:016x} is not its sign extension')
            elif k == 'p':
                if v != sext32(tv):
                    probs.append(f'arg{j} l: in a pointer slot: table 0x{sext32(tv):016x}, arrived 0x{v:016x}')
            else:
                probs.append(f'arg{j} WIDTH: table l:, call form w:')
        elif tk in 'bf':
            if k == 'w':
                probs.append(f'arg{j} WIDTH: table {tk}: (an address), call form w:')
                continue
            if reg in (0, POISON64) or reg & 0xFFFFFFFF == POISON32:
                probs.append(f'arg{j} {tk}:{tv}: arrived 0x{reg:x}, not an address')
                continue
            key = (tk, tv)
            if key in addr and addr[key] != reg:
                probs.append(f'arg{j} {tk}:{tv}: 0x{reg:x}, earlier 0x{addr[key]:x}')
            addr.setdefault(key, reg)
        else:
            unchecked.append(f'arg{j} {tk}:{tv} (not in the trace)')
    for j in range(len(slots), len(targs)):
        probs.append(f'arg{j} BEYOND-FORM: table {targs[j][0]}:{targs[j][1]}, '
                     f'the call form of {rec["name"]} has {len(slots)} argument(s)')
    if rec['x8'] is not None:
        probs.append(f'x8 = 0x{rec["x8"]}: not the sign extension of the function number')
    return probs, unchecked


def load_bin(path):
    with open(path, 'rb') as f:
        data = f.read()
    if data[:4] != b'TDMP':
        raise ValueError(f'{path}: no TDMP header')
    rsize, count = struct.unpack_from('>HH', data, 8)
    if rsize != 320:
        raise ValueError(f'{path}: record size {rsize}, not 320')
    recs = []
    for k in range(count):
        o = 32 + k * rsize
        case, opcode = struct.unpack_from('>HH', data, o)
        ret_lo, = struct.unpack_from('>I', data, o + 36)
        ret_hi, = struct.unpack_from('>I', data, o + 296)
        trap, = struct.unpack_from('>H', data, o + 300)
        recs.append((case, opcode, trap, (ret_hi << 32) | ret_lo))
    return recs


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('serial')
    ap.add_argument('table')
    ap.add_argument('--bin', help='the dump of the same run, for the alignment cross-check')
    ap.add_argument('--caller', help='the caller column to use (U0, K, ...)')
    a = ap.parse_args()
    try:
        calls = load_table(a.table)
        trace = load_trace(a.serial)
        bins = load_bin(a.bin) if a.bin else None
    except (OSError, ValueError) as e:
        print(f'error: {e}', file=sys.stderr)
        return 2

    ncases = sum(1 for c in calls if c[1] == 'case')
    print(f'table   {a.table}: {ncases} cases, {len(calls) - ncases} preludes, '
          f'{len(calls)} calls in issue order')
    print(f'trace   {a.serial}: {len(trace)} [biostrace] records')
    if not trace:
        print('RESULT  FAIL: no [biostrace] line (kernel without bios-arg-trace, or serial=off?)')
        return 1

    want = [(c[2], c[3]) for c in calls]
    callers = [a.caller] if a.caller else list(dict.fromkeys(r['who'] for r in trace))
    chosen = None
    diag = []
    for who in callers:
        mine = [r for r in trace if r['who'] == who]
        start, info = find_run([(r['trap'], r['fn']) for r in mine], want)
        if start is not None:
            chosen = (who, mine, start)
            break
        diag.append((who, info))
    if chosen is None:
        print('RESULT  FAIL: the table\'s call sequence is not in the trace as one run')
        for who, (i, k) in diag:
            print(f'        caller {who}: longest matching run {k} of {len(want)} calls')
            if k < len(want):
                c = calls[k]
                mine = [r for r in trace if r['who'] == who]
                got = mine[i + k] if i + k < len(mine) else None
                print(f'          expected next: line {c[0]} {c[2]}:0x{c[3]:02x}; '
                      + (f'trace has {got["trap"]}:0x{got["fn"]:02x} {got["name"]} '
                         f'(serial line {got["serial"]})' if got else 'trace ends'))
        return 1

    who, mine, start = chosen
    block = mine[start:start + len(calls)]
    outside = mine[:start] + mine[start + len(calls):]
    print(f'caller  {who}: run = its records {start}..{start + len(calls) - 1} '
          f'(seq {block[0]["seq"]}..{block[-1]["seq"]}, serial lines '
          f'{block[0]["serial"]}..{block[-1]["serial"]}); {len(outside)} of its '
          f'BIOS/XBIOS calls lie outside the run')
    for r in outside[:20]:
        print(f'        outside: seq {r["seq"]} {r["trap"]}:0x{r["fn"]:02x} {r["name"]}')
    if len(outside) > 20:
        print(f'        outside: ... {len(outside) - 20} more')

    addr = {}
    bad = 0
    nargs = 0
    unchecked_all = []
    case_k = 0
    for c, r in zip(calls, block):
        probs, unchecked = check_args(c, r, addr)
        nargs += len(c[4])
        if bins is not None and c[1] == 'case':
            if case_k >= len(bins):
                probs.append('BIN: no record for this case')
            else:
                bc, bop, btrap, bret = bins[case_k]
                if (bc, bop, btrap) != (c[0], c[3], c[2]):
                    probs.append(f'BIN record {case_k}: case {bc} {btrap}:0x{bop:02x}, '
                                 f'table line {c[0]} {c[2]}:0x{c[3]:02x}')
                elif r['res'] is not None and bret != r['res']:
                    probs.append(f'BIN record {case_k}: ret 0x{bret:016x}, trace 0x{r["res"]:016x}')
            case_k += 1
        tag = 'FAIL' if probs else 'ok  '
        argtxt = ' '.join(f'{k}:0x{v:x}' + (f'=0x{raw:x}' if raw is not None else '')
                          for k, v, raw in r['args']) or '-'
        print(f'{tag} line {c[0]:4} {c[1]:7} {c[2]}:0x{c[3]:02x} {r["name"]:<12} {argtxt}')
        for p in probs:
            print(f'       {p}')
        for u in unchecked:
            print(f'       UNCHECKED {u}')
        unchecked_all += unchecked
        bad += bool(probs)
    if bins is not None and len(bins) != ncases:
        print(f'FAIL BIN has {len(bins)} records, the table {ncases} cases')
        bad += 1
    print(f'RESULT  {"PASS" if not bad else "FAIL"}: {len(calls)} calls, {nargs} table '
          f'arguments; {bad} row(s) differ; {len(unchecked_all)} argument(s) unchecked'
          + (f'; BIN {len(bins)} records cross-checked' if bins is not None else ''))
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
