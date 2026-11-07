#!/bin/bash
# V38 acceptance: green / red runs of tos-api vdi::contour::tests in the scratch trees (S = scratchpad v38t).
#  green  = fix tree as committed (e6ffdedc)
#  base   = base tree (4cc7005c) + the fix's tests module appended, with flood() building the base Flood literal
#           (the base code, no Flood::new/fill_from, no budget)
#  M0 no brake (budget block removed), M1 bound 0, M2 check after the pop (before draw_span),
#  M3 check before the empty-queue exit, M5 budget from the clip instead of the screen, M6 bound 1.
# Every edit is made on the scratch file and put back from the saved copy (cmp checked); nothing in the main tree.
S=${S:?set S}
F=tos-api/src/vdi/contour.rs
BLK='            if self.budget == 0 {
                return Err(Stop::Brake);
            }
            self.budget -= 1;
'
run(){ (cd "$1" && cargo test -p tos-api --lib vdi::contour::tests 2>&1 | /usr/bin/grep -E "^test |test result|harness guard" | sort | uniq); }
cp $S/fix/$F $S/contour.fix.rs; cp $S/base/$F $S/contour.base.rs
echo "== green (fix as committed)"; run $S/fix
echo "== base code + tests module"
python3 - <<PY
fix=open("$S/contour.fix.rs").read(); i=fix.index("#[cfg(test)]\nmod tests {"); t=fix[i:]
t=t.replace("        let _ = Flood::new(c, clip, clip, search_color, seed_type, fill, &mut queue).fill_from(x, y);",
"        let mut fl = Flood { backend: c, bounds: clip, clip, truecolor: true, search_color, seed_type, fill, queue: &mut queue, qtop: 0, qptr: 0 };\n        let Some((l, r, true)) = fl.end_pts(x, y) else { return };\n        let _ = fl.run(y as u16, l, r);")
open("$S/base/$F","a").write("\n"+t)
PY
run $S/base; cp $S/contour.base.rs $S/base/$F; cmp $S/contour.base.rs $S/base/$F && echo "base restored"
mut(){ python3 - "$1" "$2" <<PY
import sys
p="$S/fix/$F"; s=open(p).read(); blk='''$BLK'''
m=sys.argv[1]
if m=="M0": s=s.replace(blk,"")
if m=="M1": s=s.replace("BRAKE_SPANS_PER_PIXEL: u64 = 16;","BRAKE_SPANS_PER_PIXEL: u64 = 0;")
if m=="M6": s=s.replace("BRAKE_SPANS_PER_PIXEL: u64 = 16;","BRAKE_SPANS_PER_PIXEL: u64 = 1;")
if m=="M2":
    s=s.replace(blk,""); s=s.replace("            self.draw_span(oldxleft, oldxright, row(oldy));\n            // \`SEEDABORT\`", blk+"            self.draw_span(oldxleft, oldxright, row(oldy));\n            // \`SEEDABORT\`")
if m=="M3":
    s=s.replace(blk,""); s=s.replace("            if self.qtop == 0 {\n                return Ok(());\n            }\n", blk+"            if self.qtop == 0 {\n                return Ok(());\n            }\n",1)
if m=="M5": s=s.replace("budget: BRAKE_SPANS_PER_PIXEL * backend.width() as u64 * backend.height() as u64,","budget: BRAKE_SPANS_PER_PIXEL * (clip.xmx - clip.xmn + 1) as u64 * (clip.ymx - clip.ymn + 1) as u64,")
open(p,"w").write(s)
PY
}
for m in M0 M1 M2 M3 M5 M6; do
  mut $m; echo "== $m ($(diff $S/contour.fix.rs $S/fix/$F | grep -c '^[<>]') changed lines)"; run $S/fix
  cp $S/contour.fix.rs $S/fix/$F; cmp $S/contour.fix.rs $S/fix/$F || echo "NOT RESTORED"
done
