#!/usr/bin/env python3 -I
"""compare.py <emutos-results> <before-results> <after-results> [<earlier-emutos>]: per case, the
program's codes side by side: EmuTOS, NewTOS before the fix, NewTOS after it. One row per
step, "=" where the sides agree with the one to their left."""
import os, re, sys

def steps(d):
    p = os.path.join(d, "WFPROBE.LOG")
    text = open(p, errors="replace").read() if os.path.exists(p) else ""
    if not text.strip():
        s = os.path.join(d, "BOOT-SERIAL.txt")
        text = open(s, errors="replace").read().replace("\r", "") if os.path.exists(s) else ""
    out, seen = {}, set()
    for line in text.splitlines():
        m = re.match(r"save (\S+)\s+(.*)", line)
        if m and m.group(1) not in seen:
            seen.add(m.group(1))
            out[m.group(1)] = m.group(2)
        m = re.match(r"Dfree\(.:\)=(\S+) free=(\d+)", line)
        if m:
            out["Dfree"] = "rc %s free %s" % m.groups()
    return out

def card(d):
    p = os.path.join(d, "dir.txt")
    if not os.path.exists(p):
        return ""
    ent = [l.split() for l in open(p, errors="replace")]
    return " ".join("%s=%s" % (e[0], e[2]) for e in ent if len(e) >= 4 and e[2].isdigit() and e[0] != "NEWDESK" and e[1] != "files" and e[0] != "1")

def fsck(d):
    p = os.path.join(d, "fsck.txt")
    if not os.path.exists(p):
        return ""
    w = [l.strip().replace("Warning: ", "") for l in open(p, errors="replace") if l.startswith("Warning") and "files," not in l]
    return "; ".join(w) if w else "clean"

em, be, af = sys.argv[1:4]
cases = sorted({n.split("-")[1] for r in (be, af) for n in os.listdir(r) if os.path.isdir(os.path.join(r, n))})
for c in cases:
    if c.startswith("n"):
        continue
    E, B, A = (os.path.join(r, s + c) for r, s in ((em, "emutos-"), (be, "newtos-"), (af, "newtos-")))
    print("=== %s" % c)
    se, sb, sa = steps(E), steps(B), steps(A)
    for k in list(dict.fromkeys(list(sa) + list(sb) + list(se))):
        print("  %-12s E %-40s B %-40s A %s" % (k, se.get(k, "-"), sb.get(k, "-"), sa.get(k, "-")))
    print("  card E %s\n       B %s\n       A %s" % (card(E), card(B), card(A)))
    print("  fsck E %s\n       B %s\n       A %s" % (fsck(E), fsck(B), fsck(A)))
