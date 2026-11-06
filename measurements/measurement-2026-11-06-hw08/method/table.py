#!/usr/bin/env python3 -I
"""table.py <results-dir>: one line per save of every case, both sides, from WFPROBE.LOG (or the
serial log where the log could not be written), plus the state of the partition after the run."""
import os, re, sys
root = sys.argv[1]
for d in sorted(os.listdir(root)):
    p = os.path.join(root, d)
    if not os.path.isdir(p):
        continue
    log = os.path.join(p, "WFPROBE.LOG")
    src = "log"
    text = open(log, errors="replace").read() if os.path.exists(log) else ""
    if not text.strip():
        src = "serial"
        text = open(os.path.join(p, "BOOT-SERIAL.txt"), errors="replace").read().replace("\r", "")
    print("=== %s (%s)" % (d, src))
    m = re.search(r"^=== WFPROBE case \w+: (.*)$", text, re.M)
    print("    " + (m.group(1) if m else "?"))
    seen = set()
    for line in text.splitlines():
        if re.match(r"(save|Dfree|Fdelete|disarm)", line) and line not in seen:
            seen.add(line)
            print("    " + re.sub(r"\s+", " ", line))
    if "gave up" in text:
        cache = [l.strip() for l in text.splitlines() if l.strip().startswith("cache ")]
        print("    last " + cache[-1])
    fsck = [l.strip() for l in open(os.path.join(p, "fsck.txt"), errors="replace") if l.startswith("Warning")]
    print("    fsck: " + ("; ".join(fsck) if fsck else "-"))
    ent = [l.split() for l in open(os.path.join(p, "dir.txt"), errors="replace")]
    print("    card: " + " ".join("%s.%s=%s" % (e[0], e[1], e[2]) for e in ent if len(e) >= 4 and e[2].isdigit() and e[0] != "NEWDESK" and e[1] != "files"))
