#!/usr/bin/env python3
"""trace-check.py -- the input check of an rsprobe run on NewTOS (TASK/XBIOS06).

usage: trace-check.py RSPROBE.LOG BOOT-SERIAL.txt

The probe (tosdump/probes/rsprobe.c) issues per step line three Rsconf calls:
the step's own call (its arguments are in the line's text; P17's symbolic
arguments come from the line "P17 args"), Rsconf(-2,-1,-1,-1,-1,-1) and
Rsconf(-1,-1,-1,-1,-1,-1); P00's own call is Rsconf(-1 x6). Before them one
Iorec(0). Supexec calls (the record writes of P16/P18) are counted, not
sequenced. The check: the [biostrace] lines of caller U0 carry exactly that
sequence of trap 14 calls, each argument's low 16 bits equal to the source's
WORD and the register a sign-extended WORD (the tracer prints w:<16 bits>);
and each traced 64-bit result equals the ret/hi, q/qhi, g/ghi of the log.
Exit 0 = every call matched.
"""
import re
import sys

log = [l.rstrip("\n") for l in open(sys.argv[1], encoding="ascii")]
ser = [l.rstrip("\n") for l in open(sys.argv[2], encoding="latin-1")]

sym = {}
for l in log:
    if l.startswith("P17 args "):
        for k, v in re.findall(r"(\w+)=([0-9a-f-]+)", l[len("P17 args "):]):
            sym[k] = int(v, 10) if k in ("baud0", "flow0") else int(v, 16)

def arg(s):
    s = s.strip()
    if s in sym:
        return sym[s]
    return int(s, 0)

expect = [("Iorec", [0], None)]
step_re = re.compile(r"^(\S+) .*?Rsconf\(([^)]*)\).* ret=(\w+) hi=(\w+) q=(\w+) qhi=(\w+) g=(\w+) ghi=(\w+)")
for l in log:
    m = step_re.match(l)
    if not m:
        continue
    sid = m.group(1)
    args = [arg(a) for a in m.group(2).split(",")]
    r = int(m.group(4) + m.group(3), 16)
    q = int(m.group(6) + m.group(5), 16)
    g = int(m.group(8) + m.group(7), 16)
    expect.append(("Rsconf", args, r, sid + " call"))
    expect.append(("Rsconf", [-2, -1, -1, -1, -1, -1], q, sid + " q"))
    expect.append(("Rsconf", [-1] * 6, g, sid + " g"))

tr_re = re.compile(r"^\[biostrace\] (\d+) (\S+)\s+14:0x(\w+) (\w+)\s+(.*?) -> (\S+)")
trace, supexec, other = [], 0, []
for l in ser:
    m = tr_re.match(l)
    if not m:
        if l.startswith("[biostrace]"):
            other.append(l)
        continue
    if m.group(2) != "U0":
        other.append(l)
        continue
    name = m.group(4)
    if name == "Supexec":
        supexec += 1
        continue
    words = [int(w[2:], 16) for w in m.group(5).split()]
    trace.append((name, words, m.group(6), l))

bad = 0
if len(trace) != len(expect):
    print(f"COUNT trace {len(trace)} calls, expected {len(expect)}")
    bad += 1
for i, (e, t) in enumerate(zip(expect, trace)):
    name, args, ret = e[0], e[1], e[2]
    tag = e[3] if len(e) > 3 else "Iorec(0)"
    ok = t[0] == name and len(t[1]) == len(args) and all(
        (a & 0xFFFF) == w for a, w in zip(args, t[1]))
    if ret is not None:
        ok = ok and int(t[2], 16) == ret
    if not ok:
        bad += 1
        print(f"MISMATCH #{i} {tag}: expected {name} {args} -> "
              f"{'' if ret is None else hex(ret)}; trace: {t[3]}")
print(f"calls checked {min(len(trace), len(expect))} (Iorec 1 + Rsconf "
      f"{len(expect) - 1}); Supexec lines {supexec}; other [biostrace] lines {len(other)}")
for l in other:
    print("  other:", l)
print("RESULT", "OK" if bad == 0 else f"FAIL ({bad})")
sys.exit(0 if bad == 0 else 1)
