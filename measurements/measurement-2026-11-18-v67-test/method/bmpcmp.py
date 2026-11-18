# bmpcmp.py a.bmp b.bmp [top_bits] -- per-gun compare of two BMP screendumps (sips output); with top_bits also counts pixels whose
# value differs after >> (8-top_bits) and the largest per-gun step
import sys, struct
def rd(p):
    d = open(p, 'rb').read(); off = struct.unpack_from('<I', d, 10)[0]; w, h = struct.unpack_from('<ii', d, 18); bpp = struct.unpack_from('<H', d, 28)[0]
    bp = bpp // 8; rowsz = (w * bp + 3) & ~3; px = []
    for y in range(abs(h)):
        r = d[off + y*rowsz: off + y*rowsz + w*bp]
        px.append([(r[i*bp+2], r[i*bp+1], r[i*bp]) for i in range(w)])
    return px
a = rd(sys.argv[1]); b = rd(sys.argv[2]); tb = int(sys.argv[3]) if len(sys.argv) > 3 else 8
n = [0,0,0]; top = [0,0,0]; mx = [0,0,0]; first = None; cnt = 0
for y,(ra,rb) in enumerate(zip(a,b)):
    for x,(pa,pb) in enumerate(zip(ra,rb)):
        d = False
        for g in range(3):
            if pa[g] != pb[g]:
                n[g] += 1; d = True; mx[g] = max(mx[g], abs(pa[g]-pb[g]))
                if (pa[g] >> (8-tb)) != (pb[g] >> (8-tb)): top[g] += 1
        if d:
            cnt += 1
            if first is None: first = (x,y)
print(f"{len(a[0])}x{len(a)}: pixels differing {cnt}; per gun exact R/G/B {n}; top-{tb}-bits R/G/B {top}; max step R/G/B {mx}; first {first}")
