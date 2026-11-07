# gen-spans.py <run label> <probe out-dir> NAME[=CONST] ... -- the non-white words of each named vdprobe
# grab as Rust `Span` constants for tos-api/tests/vdi_reference/cases_v31.rs (format of the V30 run's
# method/gen-spans.py). Run from the repository root.
import sys, importlib.util
spec = importlib.util.spec_from_file_location("vc", "tosdump/probes/vdprobe-compare.py")
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
label, out = sys.argv[1], sys.argv[2]
e = {c[1]: c for c in vc.load(out)}
def word(w):
    return {0: 'B', 0xFFDF: 'P'}.get(w, '0x%04X' % w)
def const(cname, name):
    no, nm, x, y, w, h, ws = e[name]
    sp = []
    for yy in range(y, y + h):
        xx = x
        while xx < x + w:
            v = ws[(yy - y) * w + (xx - x)]
            xe = xx
            while xe + 1 < x + w and ws[(yy - y) * w + (xe + 1 - x)] == v:
                xe += 1
            if v != 0xFFFF:
                sp.append("(%d, %d, %d, %s)" % (yy, xx, xe, word(v)))
            xx = xe + 1
    print("/// %s: grab (%d,%d) %dx%d, record %d of %s." % (name, x, y, w, h, no, label))
    print("#[rustfmt::skip]")
    print("const %s: &[Span] = &[" % cname)
    cur = "   "
    for s in sp:
        if len(cur) + len(s) + 2 > 110:
            print(cur); cur = "   "
        cur += " " + s + ","
    if cur.strip():
        print(cur)
    print("];")
    print()
for a in sys.argv[3:]:
    name, _, cname = a.partition("=")
    const(cname or "SP_" + name.upper(), name)
