#!/usr/bin/env python3
# layers.py <tree> <variant>: rewrite <tree>/tos-api/src/vdi/text.rs from text.rs.orig for one experiment.
import sys, shutil, os
tree, var = sys.argv[1], sys.argv[2]
p = tree + '/tos-api/src/vdi/text.rs'
o = p + '.orig'
if not os.path.exists(o): shutil.copy(p, o)
s = open(o).read()
def rep(a, b, n=1):
    global s
    assert s.count(a) >= 1, a
    s = s.replace(a, b) if n == 0 else s.replace(a, b, n)
is_head = 'struct GlyphStore' in s
RET = "    let count = pb.contrl(3);\n    if count <= 0 {\n        return;\n    }\n"
if is_head:
    LOOP = "for i in 0..count as usize {\n        let (cell, advance) = make_glyph(st, pb, base, f, pb.intin(i), vwk, &mut xdda);\n        draw_cell(backend, st, cell, cx, top_y, vwk, clip);"
    DRAW = "        draw_cell(backend, st, cell, cx, top_y, vwk, clip);\n"
    CALL = "    backend.draw_glyph_sized(x, y, &bytes[..h * bpr], cell_w, cell_h, vwk.text_color, mode, clip);\n"
else:
    LOOP = "for i in 0..count as usize {\n        let (cell, advance) = make_glyph(base, f, pb.intin(i), vwk, &mut xdda);\n        draw_cell(backend, &cell, cx, top_y, vwk, clip);"
    DRAW = "        draw_cell(backend, &cell, cx, top_y, vwk, clip);\n"
    CALL = "    backend.draw_glyph_sized(x, y, &bytes[..h * bpr], cell_w, cell.h, vwk.text_color, mode, clip);\n"
if var == 'L0':   # trap, dispatch, binding: v_gtext returns at once
    rep(RET, RET + "    if count != 12345 { return; }\n")
elif var == 'L1': # + per-call header, width, delh, underline call; no glyph
    rep(LOOP, LOOP.replace('0..count as usize', '0..core::hint::black_box(0usize)'))
elif var == 'L2': # + glyph assembly, no blit
    rep(DRAW, "        let _ = core::hint::black_box(&cell);\n")
elif var == 'L3': # + packing, no backend call
    rep(CALL, "    let _ = core::hint::black_box((&bytes, mode));\n")
elif var == 'L4':
    pass
elif var == 'ROWS34':
    rep("const MAX_ROWS: usize = 512;", "const MAX_ROWS: usize = 34;")
elif var == 'W1':
    rep("const ROW_WORDS: usize = 8;", "const ROW_WORDS: usize = 1;")
elif var == 'ROWS34W1':
    rep("const MAX_ROWS: usize = 512;", "const MAX_ROWS: usize = 34;")
    rep("const ROW_WORDS: usize = 8;", "const ROW_WORDS: usize = 1;")
else:
    sys.exit('unknown ' + var)
open(p, 'w').write(s)
