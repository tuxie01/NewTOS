import sys
p,src,m=sys.argv[1],sys.argv[2],sys.argv[3]
s=open(src).read()
M={
 'skew':("        w += (f.left_offset.wrapping_add(f.right_offset) as i32).max(rows);\n","        w += 0;\n"),
 'offsets':("(f.left_offset.wrapping_add(f.right_offset) as i32).max(rows)","rows"),
 'rows':("delx >= 0 && w <= side && rows + outline <= side","delx >= 0 && w <= side"),
 'dda':("let dda = if vwk.scaled { 2 } else { 1 };","let dda = 1;"),
 'outline':("let mut w = dda * delx + outline;","let mut w = dda * delx;"),
 'thicken':("        w += f.thicken.max(0) as i32;\n","        w += 0;\n"),
 'neg':("delx >= 0 && w <= side","w <= side"),
}
a,b=M[m]
assert s.count(a)==1,m
open(p,'w').write(s.replace(a,b))
