# V38: the 5x3 never-ending fill, run through the V21 tester's independent Python model of
# EmuTOS vdi_fill.c contourfill (measurements/measurement-2026-11-06-v21-test/method/cf.py, read
# from there, not copied). fill=0 = the fill paints the background colour, so the screen never
# changes (white-on-white). The model's state after every taken span is hashed; a repeated state
# of a deterministic loop means the fill never ends.
import sys, os
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, '../../measurement-2026-11-06-v21-test/method'))
import cf
scr = [[0, 0, 0, 0, 0],
       [0, 0, 0, 1, 0],
       [0, 1, 0, 0, 0]]
for fill in (0, 2):
    out, st = cf.run(scr, 0, 2, boundary=1, fill=fill, clip=(0, 0, 4, 2), maxsteps=10**6)
    print(f"fill={fill}: hang={st.get('hang', False)} overflow={st['ov']} max_live={st['max']} screen={out}")
