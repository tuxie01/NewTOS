#!/usr/bin/env python3 -I
"""analyse.py <script> <run-dir> [...] -- per click of a mnprobe run: _hz_200 of the left-button
PRESS and RELEASE packets and of the next evnt_multi return with MN_SELECTED (R-lines, with the
wait mode the probe logged there); ticks = 5 ms.
Packet times: EmuTOS with the probe's feeder: J-lines (script order); EmuTOS --cmd-fifo
cross-check (no J-lines): the V-lines of the mousevec observer (left-button edges, bit 1);
NewTOS: BOOT-SERIAL.txt "[inj] ... feed" lines (the tick of a packet's 3rd byte, script order).
NewTOS dbg kernels: also "[dbg] track end hz_200" (menu::track's end, scratch patch)."""
import re, sys, pathlib
script = pathlib.Path(sys.argv[1]).read_text().splitlines()
pk = []
for ln in script:
    body, _, note = ln.partition("#")
    if not body.strip(): continue
    pk.append((note.strip(), [int(x, 0) for x in body.split()[1:]]))
def side(run):
    run = pathlib.Path(run)
    log = (run / "MNPROBE.LOG").read_text().splitlines()
    rs = []
    for l in log:
        m = re.match(r"R(\d+) (\S+)\s+t=(\d+) which=(\w+) msg=(-?\d+),(-?\d+),(-?\d+)", l)
        if m: rs.append(dict(t=int(m[3]), mode=m[2], msg=int(m[5]), item=int(m[7]), k=int(m[1])))
    dc = re.search(r"evnt_dclick (\d+)", "\n".join(log))
    js = {int(m[1]): int(m[2]) for l in log for m in [re.match(r"J(\d+) t=(\d+)", l)] if m}
    vs = [(int(m[1]), int(m[2], 16)) for l in log for m in [re.match(r"V\d+ t=(\d+) pkt=(\w\w)", l)] if m]
    ends = []
    clicks = []  # (press, release, kind)
    ser = run / "BOOT-SERIAL.txt"
    if js or ser.exists():
        if ser.exists():
            lines = ser.read_text(errors="replace").splitlines()
            fed = [int(m[1]) for l in lines for m in [re.search(r"\[inj\] \S+ feed 0x[0-9a-f]+ at hz_200 (\d+)", l)] if m]
            at = {i: fed[3 * i + 2] for i in range(len(fed) // 3)}
            ends = [int(m[1]) for l in lines for m in [re.search(r"\[dbg\] track end hz_200 (\d+)", l)] if m]
        else:
            at = js
        kind = None
        p = None
        for i, (note, b) in enumerate(pk):
            m = re.match(r"seg \d rep \d (\w+)", note)
            if m: kind = m[1]
            if note.startswith("PRESS"): p = at.get(i)
            if note == "RELEASE" and p is not None and at.get(i) is not None:
                clicks.append((p, at[i], kind)); p = None
        src = "J" if js else "inj"
    else:
        prev = 0; p = None
        for t, b0 in vs:
            down = b0 & 2
            if down and not prev: p = t
            if prev and not down and p is not None: clicks.append((p, t, "click")); p = None
            prev = down
        src = "V"
    return rs, clicks, ends, dc[1] if dc else "?", src
for run in sys.argv[2:]:
    rs, clicks, ends, dc, src = side(run)
    print(f"== {pathlib.Path(run).name}  evnt_dclick {dc}  press/release from {src}")
    print("  #  kind  mode  press  release  return  ret-press  ret-release  trackend-press  item")
    for n, (p, r, kind) in enumerate(clicks):
        nxt = [x for x in rs if x["t"] >= p and x["msg"] == 10]
        ret = nxt[0] if nxt else None
        te = [e for e in ends if e >= p and (ret is None or e <= ret["t"])]
        print(f"{n:3}  {kind:5} {ret['mode'] if ret else '-':4} {p:6} {r:7} {ret['t'] if ret else '-':7} "
              f"{(ret['t'] - p) if ret else '-':9} {(ret['t'] - r) if ret else '-':11} "
              f"{(te[0] - p) if te else '':>14}  {ret['item'] if ret else '-'}")
