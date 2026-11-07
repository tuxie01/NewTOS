#!/usr/bin/env python3
"""check-log.py <run dir> -- the paged run's WFPROBE.LOG against the probe's
console text on the serial line.

The paged build prints the run line by line as before (pass 1, up to
"log Fwrite done"); if the screen ran full while a fault could be pending it
shows the run once more, paged, after the marker line (pass 2, up to
"WFPROBE done -- any key"). The log is written from the same buffer, unpaged.
Checked: the log has no prompt text; pass 1 == the log; pass 2 == the tail of
the log from the top of the page that was open at arm() (here the whole log).

Not probe text, dropped: kernel lines ("[..." and the kernel's own
[sddiag] ... [sddiag] end block), the prompts, the blanks that erase a prompt,
the key echoes after the two menu questions."""
import re
import sys
import pathlib

d = pathlib.Path(sys.argv[1])
raw = d.joinpath("BOOT-SERIAL.txt").read_bytes().decode("latin-1")
ser = raw.replace("\r\n", "\n").replace("\r", "\n").split("\n")
log = d.joinpath("WFPROBE.LOG").read_bytes().decode("latin-1")
logl = [x for x in log.split("\n") if x != ""]
MARK = "--- the run once more, paged (no waits while a fault can be pending)"

start = next(i for i, l in enumerate(ser) if l.startswith("[auto] running"))
passes, cur, insd, prompts = [], [], False, 0
for l in ser[start + 1:]:
    l = re.sub(r"^ {19}", "", l)  # the blanks that erased a prompt
    if l.startswith("WFPROBE done -- any key"):
        break
    if l == "[sddiag]":
        insd = True
        continue
    if insd:
        insd = l != "[sddiag] end"
        continue
    if l.startswith("[") or l.strip() == "":
        continue
    if l == "-- more: any key --":
        prompts += 1
        continue
    if l in ("u", "d"):
        continue
    if l == "log Fwrite done":
        passes.append(cur)
        cur = []
        continue
    if l == MARK:
        continue
    cur.append(l)
passes.append(cur)

print("prompts on the serial line:", prompts, "| prompt text in the log:", log.count("more: any key"))
print("log lines:", len(logl), "| pass 1 lines:", len(passes[0]), "| pass 2 lines:", len(passes[1]))
print("marker line present:", MARK in raw)
print("pass 1 == log:", passes[0] == logl)
print("pass 2 == tail of the log:", passes[1] == logl[len(logl) - len(passes[1]):] and len(passes[1]) > 0)
