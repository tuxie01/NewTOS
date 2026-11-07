# shift.py <e-dir> <n-dir> PREFIX...: per case, the horizontal shift s (-210..210) with N(x) == E(x - s) over the
# whole grab (both read as white outside it); prints the shifts that match, or "none".
import sys, importlib.util
spec = importlib.util.spec_from_file_location("vc", "tosdump/probes/vdprobe-compare.py")
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
E = {c[1]: c for c in vc.load(sys.argv[1])}
N = {c[1]: c for c in vc.load(sys.argv[2])}
for name in sorted(E):
    if not any(name.startswith(p) for p in sys.argv[3:]):
        continue
    _, _, x, y, w, h, we = E[name]
    wn = N[name][6]
    hits = []
    for s in range(-210, 211):
        ok = True
        for r in range(h):
            for c in range(w):
                cs = c + s
                ev = we[r * w + cs] if 0 <= cs < w else 0xFFFF
                if ev != wn[r * w + c]:
                    ok = False; break
            if not ok: break
        if ok: hits.append(s)
    print(name, hits if hits else "none")
