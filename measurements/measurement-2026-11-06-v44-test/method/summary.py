import sys,glob,re,statistics as st,os
d=sys.argv[1]; names=sys.argv[2:]
res={}
for k in names:
  for p in sorted(glob.glob(f"{d}/{k}-*/TXBENCH.LOG")+glob.glob(f"{d}/{k}-*/TXFONT.LOG")):
    i=int(p.split(k+'-')[1].split('/')[0])
    for l in open(p):
      m=re.match(r"(\S+) calls=\d+ chars_per_call=\d+ ticks200=(\d+)",l)
      if m: res.setdefault(m.group(1),{}).setdefault(k,{})[i]=int(m.group(2))
for case,v in res.items():
  line=f"{case:22}"
  for k in names:
    xs=[v[k][i] for i in sorted(v.get(k,{}))]
    if xs: line+=f" {k}: n={len(xs)} min={min(xs)} med={st.median(xs)} max={max(xs)} |"
  b=names[0]
  for k in names[1:]:
    rs=[v[k][i]/v[b][i] for i in v.get(k,{}) if i in v.get(b,{})]
    if rs: line+=f" {k}/{b} med={st.median(rs):.2f} ({min(rs):.2f}..{max(rs):.2f}) minmin={min(v[k].values())/min(v[b].values()):.2f}"
  print(line)
