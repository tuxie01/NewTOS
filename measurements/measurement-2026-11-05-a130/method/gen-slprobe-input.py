#!/usr/bin/env python3 -I
"""gen-slprobe-input.py <mouse-out> <shots-out> <cases-out> -- the input of tosdump/probes/slprobe.c
(TASK/A130, GitHub #2): IKBD relative mouse packets `0xF8|buttons dx dy` (bit 1 = left) at 200 Hz
ticks from the input clock's start, the screenshot schedule `<ticks> <name>`, and a case table
(one line per shot: name, configuration, axis, gesture, phase, pointer x,y, button).

Geometry, measured on the pinned oracle (exploration run, RUNINFO.TXT), window border
80,60,400,320, work 81,83,376,274:
  vertical bar x 458..478, track y ~105..334; horizontal bar y 358..378, track x ~103..434
  elevators (EmuTOS):  size 1000  V y 106..333   H x 104..431
                       size 300   V y 186..253   H x 219..317
                       size -1    V y 211..228   H x 259..276
  work-area columns (slprobe.c config()): x 120 = size 1000, 220 = 300, 310 = -1, 420 = quit
"""
import sys

x = y = 0
t = 0
btn = 0xF8
pk = []      # (line)
shots = []   # (tick, name)
cases = []   # (name, cfg, axis, gesture, phase, x, y, button)
cur = {"cfg": "-", "axis": "-", "gesture": "-"}

VX, HY = 468, 368          # bar centre lines
CFGX = {"1000": 120, "300": 220, "-1": 310}
ELEV_C = {"1000": (220, 268), "300": (220, 268), "-1": (219, 268)}   # (V centre y, H centre x)


def b(v):
    return v & 0xFF


def pkt(tk, bt, dx, dy, note=""):
    pk.append(f"{tk} {bt:#04x} {b(dx):#04x} {b(dy):#04x}" + (f" # {note}" if note else ""))


def move(nx, ny, step=127, every=1, note=""):
    """Move to (nx, ny), at most `step` px per axis per packet, one packet every `every` ticks,
    keeping the current button state."""
    global x, y, t
    dx, dy = nx - x, ny - y
    first = True
    while dx or dy:
        sx = max(-step, min(step, dx))
        sy = max(-step, min(step, dy))
        pkt(t, btn, sx, sy, note if first else "")
        first = False
        dx -= sx
        dy -= sy
        t += every
    x, y = nx, ny


def press(note=""):
    global btn, t
    btn = 0xFA
    pkt(t, btn, 0, 0, "PRESS " + note)
    t += 1


def release(note=""):
    global btn, t
    btn = 0xF8
    pkt(t, btn, 0, 0, "RELEASE " + note)
    t += 1


def wait(n):
    global t
    t += n


def shot(phase, settle=100):
    """A shot `settle` ticks from now; the next step starts after it."""
    global t
    name = f"{len(shots):03d}-{cur['cfg'].replace('-1', 'min')}-{cur['axis']}-{cur['gesture']}-{phase}"
    shots.append((t + settle, name))
    cases.append((name, cur["cfg"], cur["axis"], cur["gesture"], phase, x, y, "down" if btn == 0xFA else "up"))
    t += settle + 20


def burst(phase, n=4, gap=30):
    """n shots `gap` ticks apart, starting 20 ticks from now (the pointer stands still)."""
    global t
    t += 20
    for i in range(n):
        name = f"{len(shots):03d}-{cur['cfg'].replace('-1', 'min')}-{cur['axis']}-{cur['gesture']}-{phase}{i}"
        shots.append((t, name))
        cases.append((name, cur["cfg"], cur["axis"], cur["gesture"], f"{phase}{i}", x, y,
                      "down" if btn == 0xFA else "up"))
        t += gap
    t += 20


def config(cfg):
    """Click the work-area column of `cfg`: re-applies size and position 500 on both bars."""
    move(CFGX[cfg], 200)
    press(f"config {cfg}")
    wait(10)
    release()
    wait(120)


def axis_pt(axis, along):
    return (VX, along) if axis == "V" else (along, HY)


def drag_slow(cfg, axis):
    cur.update(gesture="dragslow")
    config(cfg)
    c = ELEV_C[cfg][0 if axis == "V" else 1]
    move(*axis_pt(axis, c), note=f"{cfg} {axis} onto the elevator")
    shot("before", 60)
    press("on the elevator")
    burst("held")                       # pressed, not moved: flicker detector
    for i in range(1, 5):               # slow: 8 px per step, one packet per step
        move(*axis_pt(axis, c + 8 * i), note=f"slow +{8 * i}")
        if i in (2, 4):
            shot(f"slow{8 * i}", 60)
        else:
            wait(40)
    end_hi = 470 if axis == "V" else 630  # beyond the far end (bottom / right)
    move(*axis_pt(axis, end_hi), step=4, every=2, note="beyond the far end")
    shot("beyondfar", 60)
    move(*axis_pt(axis, 30), step=4, every=2, note="beyond the near end")
    shot("beyondnear", 60)
    if axis == "V":                      # off the track sideways, held
        move(300, 30, step=4, every=2, note="sideways off the bar")
    else:
        move(30, 200, step=4, every=2, note="sideways off the bar")
    shot("sideways", 60)
    move(*axis_pt(axis, c + 40), step=4, every=2, note="back to centre+40")
    shot("back40", 60)
    release("after the slow drag")
    shot("released", 100)
    burst("after", 2, 30)


def drag_fast(cfg, axis):
    cur.update(gesture="dragfast")
    config(cfg)
    c = ELEV_C[cfg][0 if axis == "V" else 1]
    move(*axis_pt(axis, c), note=f"{cfg} {axis} onto the elevator")
    # v2: the probe is idle in evnt_multi before the press (v1 pressed 2 ticks after the
    # move, while NewTOS's probe was still writing its log after the config click)
    shot("before", 60)
    wait(100)
    press("on the elevator")
    # v3: hold still 100 ticks (500 ms) before the fast moves: the press is handled only after
    # the double-click wait (b_delay, ~44 ticks with the default evnt_dclick rate; v2 measured that
    # both systems pick the gadget at the pointer position after that wait)
    wait(100)
    lo, hi = (40, 440) if axis == "V" else (40, 600)
    move(*axis_pt(axis, hi), step=40, every=1, note="fast to beyond the far end, 40 px/5 ms")
    move(*axis_pt(axis, lo), step=40, every=1, note="fast to beyond the near end")
    move(*axis_pt(axis, c + 30), step=40, every=1, note="fast back to centre+30")
    burst("heldfast", 3, 30)
    release("after the fast drag")
    shot("released", 100)


def page(cfg, axis, side):
    cur.update(gesture=f"page{side}")
    config(cfg)
    if cfg == "1000":
        along = 334 if axis == "V" else 433   # the track's last row/column beside the full elevator
    else:
        along = (140 if side == "lo" else 300) if axis == "V" else (150 if side == "lo" else 390)
    move(*axis_pt(axis, along), note=f"{cfg} {axis} track {side}")
    shot("before", 60)
    press("in the track")
    shot("held1", 40)
    shot("held2", 120)
    release("after the page press")
    shot("released", 100)


# pin to (0,479), then rest outside the window
for i in range(6):
    pkt(600 + i, 0xF8, -127, 127, "pin to (0,479)" if i == 0 else "")
y = 479
t = 620
move(560, 440, note="rest")
cur.update(cfg="1000", axis="-", gesture="start")
shot("rest", 100)

for cfg in ("1000", "300", "-1"):
    cur["cfg"] = cfg
    for axis in ("V", "H"):
        cur["axis"] = axis
        drag_slow(cfg, axis)
        drag_fast(cfg, axis)
        if cfg == "1000":
            page(cfg, axis, "hi")
        else:
            page(cfg, axis, "lo")
            page(cfg, axis, "hi")

move(420, 200, note="quit column")
press("quit")
wait(10)
release()

hdr = ("# slprobe-mouse.txt -- mouse input of tosdump/probes/slprobe.c (TASK/A130); generated by\n"
       "# measurements/measurement-2026-11-05-a130/method/gen-slprobe-input.py.\n"
       "# <ticks> <0xF8|buttons> <dx> <dy>: one IKBD relative mouse packet per line (bit 1 = left).\n")
open(sys.argv[1], "w").write(hdr + "\n".join(pk) + "\n")
open(sys.argv[2], "w").write(
    "# slprobe-shots.txt -- screenshot schedule of tosdump/probes/slprobe.c (<ticks> <name>); generated\n"
    "# by measurements/measurement-2026-11-05-a130/method/gen-slprobe-input.py.\n"
    + "".join(f"{tk} {n}\n" for tk, n in shots))
open(sys.argv[3], "w").write("name\tcfg\taxis\tgesture\tphase\tx\ty\tbutton\n"
                             + "".join("\t".join(str(v) for v in c) + "\n" for c in cases))
ticks = [s[0] for s in shots]
assert ticks == sorted(ticks) and len(set(ticks)) == len(ticks), "shot ticks must rise"
print(f"{len(pk)} packets, {len(shots)} shots, last tick {t} ({t / 200:.0f} s)", file=sys.stderr)
