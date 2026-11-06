# V33: the expected words of tos-api/tests/vdi_reference/cases_v33.rs from the oracle's probe/e1:
# FNV-1a over the screen words of each TX grab (row by row, each word high byte first) + the non-white count.
# Run from the repo root: python3 method/gen-grabs.py <probe e1 dir>
import sys, importlib.util
spec = importlib.util.spec_from_file_location("vc", "tosdump/probes/vdprobe-compare.py")
vc = importlib.util.module_from_spec(spec); spec.loader.exec_module(vc)
for c in vc.load(sys.argv[1]):
    no, name, x, y, w, h, ws = c
    if not (name.startswith("TX") or name in ("T13A", "T13B")):
        continue
    v = 0xcbf29ce484222325
    n = 0
    for word in ws:
        for b in (word >> 8, word & 0xFF):
            v = ((v ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        if word != 0xFFFF:
            n += 1
    print('    ("%s", (%d, %d, %d, %d), 0x%016x, %d),' % (name, x, y, w, h, v, n))
