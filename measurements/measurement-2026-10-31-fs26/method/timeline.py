#!/usr/bin/env python3
"""timeline.py BOOT-SERIAL.txt -- TASK/FS26: split each desktop "New folder"
(Ctrl-N, name, Return; method/newfolder.script) into its phases from the
[fsio] lines of the fs-io-trace kernel (kernel/src/dbg_fsio.rs).

Phases, by call boundaries (t = call entry, e = t + dur, microseconds):
  open    Ctrl-N seen (e of the evnt_multi that returned it) -> form_do entry
          (dialog built and drawn)
  close   form_do end (Return seen) -> Dcreate entry
  dcreate the Dcreate call
  rescan  Dcreate end -> end of the Fsnext that returns ENMFIL (-49)
  redraw  -> entry of the next evnt_multi that waits (dur > 0.5 s): idle
'ovh' is the tracer's own printing inside the span (sum of the ovh= fields of
the lines after the span's first line); 'net' = wall - ovh.
"""
import re
import sys

LINE = re.compile(r"^\[fsio\] (\d+) (\S+)\s+(\w+) (\S+) t=(\d+) dur=(\d+)(?: how=\S+)? ret=(-?\d+)(.*)$")
recs = []
for ln in open(sys.argv[1], errors="replace"):
    m = LINE.match(ln.rstrip("\n"))
    if not m:
        continue
    seq, who, kind, op, t, dur, ret, rest = m.groups()
    ovh = int(re.search(r"ovh=(\d+)", rest).group(1))
    dev = re.search(r"dev r=(\d+)/(\d+)/(\d+) w=(\d+)/(\d+)/(\d+)", rest)
    recs.append(dict(kind=kind, op=op, t=int(t), dur=int(dur), e=int(t) + int(dur), ret=int(ret),
                     ovh=ovh, rest=rest, dev=tuple(int(x) for x in dev.groups()) if dev else None))

def span(a, b, i0, i1):
    """wall a..b, the tracer's ovh printed inside it (lines i0+1..i1)."""
    o = sum(r["ovh"] for r in recs[i0 + 1:i1 + 1])
    return b - a, o

n = 0
for i, r in enumerate(recs):
    if not (r["kind"] == "gemdos" and r["op"] == "0x39"):
        continue
    # the form_do (aes 50) before it, the evnt_multi that delivered Ctrl-N before that
    fd = max(j for j in range(i) if recs[j]["kind"] == "aes" and recs[j]["op"] == "50")
    km = max(j for j in range(fd) if recs[j]["kind"] == "aes" and recs[j]["op"] == "25" and recs[j]["dur"] > 500000)
    ls = min(j for j in range(i, len(recs)) if recs[j]["kind"] == "gemdos" and recs[j]["op"] == "0x4f" and recs[j]["ret"] == -49)
    waits = [j for j in range(ls, len(recs)) if recs[j]["kind"] == "aes" and recs[j]["op"] == "25" and recs[j]["dur"] > 500000]
    if waits:
        idle, idle_t, note = waits[0], recs[waits[0]]["t"], ""
    else:
        # the run was stopped while the desktop waited: its last call before the wait
        idle, idle_t, note = len(recs), recs[-1]["e"], " (idle = end of the last call; the waiting evnt_multi never returned)"
    n += 1
    phases = [
        ("open", recs[km]["e"], recs[fd]["t"], km, fd - 1),
        ("close", recs[fd]["e"], r["t"], fd, i - 1),
        ("dcreate", r["t"], r["e"], i - 1, i - 1),
        ("rescan", r["e"], recs[ls]["e"], i, ls),
        ("redraw", recs[ls]["e"], idle_t, ls, idle - 1),
        ("Return->idle", recs[fd]["e"], idle_t, fd, idle - 1),
    ]
    print(f"New folder #{n}: Dcreate path={re.search(r'path=(\"[^\"]*\")', r['rest']).group(1)}{note}")
    for name, a, b, i0, i1 in phases:
        wall, o = span(a, b, i0, i1)
        calls = [x for x in recs[i0 + 1:i1 + 1]] if name != "dcreate" else [r]
        kinds = {}
        for x in calls:
            kinds[x["kind"]] = kinds.get(x["kind"], 0) + 1
        dw = sum(x["dev"][3] for x in calls if x["dev"])
        dr = sum(x["dev"][0] for x in calls if x["dev"])
        print(f"  {name:13s} wall {wall/1000:8.2f} ms  ovh {o/1000:7.2f} ms  net {(wall-o)/1000:8.2f} ms"
              f"  calls {dict(sorted(kinds.items()))}  dev reads {dr} writes {dw}")
