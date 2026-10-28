#!/usr/bin/env python3
# locdiff.py A.bin B.bin -- explain every differing byte between two kernel
# images as the `line` field of a core::panic::Location {file:&str, line:u32, col:u32}
# whose file is one of the edited sources; print anything else as UNEXPLAINED.
import sys, struct
a = open(sys.argv[1], 'rb').read(); b = open(sys.argv[2], 'rb').read()
assert len(a) == len(b), "size differs"
diff = [i for i in range(len(a)) if a[i] != b[i]]
words = sorted({i & ~3 for i in diff})
# the image's load VA: find any Location in A, its file pointer minus the string's file offset
def strat(off, ln):
    return a[off:off+ln]
out = []; bad = 0
base = None
for w in words:
    line_a, = struct.unpack_from('<I', a, w); line_b, = struct.unpack_from('<I', b, w)
    ptr, ln = struct.unpack_from('<QQ', a, w - 16)
    ptr_b, ln_b = struct.unpack_from('<QQ', b, w - 16)
    col_a, = struct.unpack_from('<I', a, w + 4); col_b, = struct.unpack_from('<I', b, w + 4)
    ok = ptr == ptr_b and ln == ln_b and col_a == col_b and 0 < ln < 200
    name = None
    if ok:
        # resolve the string: try every candidate base so that a[ptr-base:+ln] is printable
        for cand in ([base] if base is not None else []) + [0xffff000000080000, 0xffff000040080000]:
            o = ptr - cand
            if 0 <= o < len(a) - ln:
                s = a[o:o+ln]
                if all(32 <= c < 127 for c in s) and s.endswith(b'.rs'):
                    name = s.decode(); base = cand; break
    if name is None:
        bad += 1; out.append(f"UNEXPLAINED @0x{w:x}: {line_a} -> {line_b}")
    else:
        out.append(f"@0x{w:x} {name}: line {line_a} -> {line_b} (+{line_b-line_a}) col {col_a}")
print("\n".join(out))
print(f"{len(diff)} differing bytes in {len(words)} words; unexplained: {bad}; base 0x{base or 0:x}")
sys.exit(1 if bad else 0)
