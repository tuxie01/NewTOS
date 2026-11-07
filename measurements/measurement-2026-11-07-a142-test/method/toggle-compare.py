import sys
sys.argv=[sys.argv[0],"runs"]
exec(open("method/glyph-compare.py").read().split("oracle =")[0])
o={"global":im("oracle-qed-tt-global/shots/grab0002.png"),"info":im("oracle-qed-tt-info/shots/grab0003.png")}
for v in ("base","fix"):
    g=im(f"qed-{v}-toggle/shots/00-global-toggled.png"); i=im(f"qed-{v}-toggle/shots/01-info-unix.png")
    # Fenster sichern box y149 (clicked on) vs oracle ON box (y149 in oracle is on) ; pointer tip at (340,157): ignore a 14x22 area
    gg=grid(g,334,149); og=grid(o["global"],334,149)
    ign=lambda x,y: 340<=334+x<=354 and 157<=149+y<=180
    d=sum(1 for y in range(16) for x in range(16) if gg[y][x]!=og[y][x] and not ign(x,y))
    print(v,"clicked check box (on), px differ vs oracle on-box, pointer area ignored:",d)
    # radio: Atari now off (x184 y281) vs oracle Unix off glyph (x292); Unix now on: pointer ignored vs oracle Atari on glyph
    a=grid(i,184,281); ofg=grid(o["info"],292,281)
    u=grid(i,292,281); on=grid(o["info"],184,281)
    ign2=lambda x,y: 300<=292+x<=316 and 289<=281+y<=312
    du=sum(1 for y in range(16) for x in range(16) if u[y][x]!=on[y][x] and not ign2(x,y))
    print(v,"deselected Atari radio vs oracle off glyph:",diff(a,ofg),"| selected Unix radio vs oracle on glyph (pointer ignored):",du)
