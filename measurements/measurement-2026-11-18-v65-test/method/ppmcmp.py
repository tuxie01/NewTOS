# ppmcmp.py a.ppm b.ppm -- per-gun compare of two P6 screendumps (QEMU writes P6 maxval 255)
import sys
def rd(p):
    d = open(p, 'rb').read()
    parts = d.split(b'\n', 3)
    w, h = map(int, parts[1].split())
    return w, h, parts[3]
wa, ha, a = rd(sys.argv[1]); wb, hb, b = rd(sys.argv[2])
if (wa, ha) != (wb, hb): print("size differs", wa, ha, wb, hb); sys.exit(1)
n = [0, 0, 0]; first = None
for i in range(0, len(a) - 2, 3):
    for g in range(3):
        if a[i+g] != b[i+g]:
            n[g] += 1
            if first is None: first = (i // 3 % wa, i // 3 // wa)
print(f"{wa}x{ha}: differing pixels per gun R {n[0]} G {n[1]} B {n[2]}; first at {first}")
