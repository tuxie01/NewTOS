#!/usr/bin/env python3 -I
"""trackend.py <script> <oracle-trackend-run> -- per click of the oracle-trackend.sh run: press,
release (J-lines), _hz_200 at the first _ap_sendmsg after the press (= mn_do has returned,
aes/gemctrl.c:331-366) and at the evnt_multi return with MN_SELECTED; hatari.log carries the
breakpoint dumps as "Invalid value 'BP <name>'!" (the echo line) + "000004BA: b3 b2 b1 b0"."""
import re, sys, pathlib, subprocess
run = pathlib.Path(sys.argv[2])
bps, name = [], None
for l in (run / "hatari.log").read_text(errors="replace").splitlines():
    m = re.search(r"'BP (\w+)'", l)
    if m: name = m[1]; continue
    m = re.match(r"000004BA: (\w\w) (\w\w) (\w\w) (\w\w)", l)
    if m and name: bps.append((name, int("".join(m.groups()), 16))); name = None
here = pathlib.Path(__file__).parent
out = subprocess.run([sys.executable, "-I", str(here / "analyse.py"), sys.argv[1], str(run)],
                     capture_output=True, text=True).stdout.splitlines()
print(out[0]); print("  #  kind  mode  press release return  ret-press  mn_do-entry  sendmsg  sendmsg-press")
for l in out[2:]:
    f = l.split()
    if len(f) < 7 or not f[0].isdigit(): continue
    p, r, ret = int(f[3]), int(f[4]), f[5]
    ent = [t for n, t in bps if n == "mn_do" and t <= p]
    snd = [t for n, t in bps if n == "ap_sendmsg" and t >= p]
    print(f"{f[0]:>3}  {f[1]:5} {f[2]:4} {p:6} {r:6} {ret:6} {f[6]:>9}  {ent[-1] if ent else '-':>11}  {snd[0] if snd else '-':>7}  {(snd[0]-p) if snd else '-':>12}")
print(f"breakpoint hits: {len(bps)} (mn_do {sum(1 for n,_ in bps if n=='mn_do')}, ap_sendmsg {sum(1 for n,_ in bps if n=='ap_sendmsg')})")
