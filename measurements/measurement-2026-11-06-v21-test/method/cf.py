# faithful model of EmuTOS vdi_fill.c contourfill (seed queue), solid fill, search colour = boundary value 1; 0 bg, 2 filled
EMPTY=0xffff; DOWN=0x8000; QSIZE=1024
W,H=640,480
def run(scr, x0,y0, boundary=1, fill=2, clip=(0,0,639,479), maxsteps=10**7):
    scr=[row[:] for row in scr]
    xmn,ymn,xmx,ymy=clip
    def end_pts(x,y):
        if y<ymn or y>ymy: return None
        c=scr[y][x]
        r=x
        while r<xmx and scr[y][r+1]==c: r+=1   # search_to_right: x++ < xmx
        l=x
        while l>xmn and scr[y][l-1]==c: l-=1
        return (l,r, c!=boundary)
    q=[[EMPTY,0,0] for _ in range(QSIZE+2)]
    st={'qptr':0,'qtop':0,'max':0,'ov':False}
    def crunch():
        while q[st['qtop']-1][0]==EMPTY and st['qtop']>0: st['qtop']-=1
        if st['qptr']>=st['qtop']: st['qptr']=0
    def get_seed(xin,yin):
        e=end_pts(xin,yin&0x7fff)
        if e is None: return 0,None,None
        l,r,isseed=e
        if not isseed: return 0,l,r
        qhole=None
        i=0
        while i<st['qtop']:
            if q[i][0]==EMPTY:
                if qhole is None: qhole=i
                i+=1; continue
            if (q[i][0]^DOWN)==yin and q[i][1]==l:
                yy=yin&0x7fff
                for x in range(l,r+1): scr[yy][x]=fill
                q[i][0]=EMPTY
                if i+1==st['qtop']: crunch()
                return 0,l,r
            i+=1
        if qhole is None:
            st['qtop']+=1
            if st['qtop']>QSIZE:
                st['ov']=True; return -1,l,r
            i=st['qtop']-1
        else: i=qhole
        q[i]=[yin,l,r]
        live=sum(1 for k in range(st['qtop']) if q[k][0]!=EMPTY)
        st['max']=max(st['max'],live)
        return 1,l,r
    e=end_pts(x0,y0)
    if not e or not e[2]: return scr,st
    oldxl,oldxr,_=e; oldy=y0
    st['qptr']=0; q[0]=[oldy|DOWN,oldxl,oldxr]; st['qtop']=1
    steps=0
    while True:
        steps+=1
        if steps>maxsteps: st['hang']=True; break
        d=1 if oldy&DOWN else -1
        g,a,b=get_seed(oldxl,oldy+d)
        if a is not None: nl,nr=a,b
        elif 'nl' not in dir(): nl,nr=oldxl,oldxr
        if g<0: return scr,st
        if g and nl is not None and nl<oldxl-1:
            xl=oldxl
            while xl>nl:
                xl-=1
                g2,a,b=get_seed(xl,oldy^DOWN)
                if g2<0: return scr,st
                if a is not None: xl=a  # *xleftout aliasing: xleft is out param
        while nr<oldxr:
            nr+=1
            g,a,b=get_seed(nr,oldy+d)
            if g<0: return scr,st
            if b is not None: nr=b   # newxright is out param
        if nr>oldxr+1 and g:
            xr=oldxr
            while xr<nr:
                xr+=1
                g2,a,b=get_seed(xr,oldy^DOWN)
                if g2<0: return scr,st
                if b is not None: xr=b
        if st['qtop']==0: break
        while q[st['qptr']][0]==EMPTY:
            st['qptr']+=1
            if st['qptr']==st['qtop']: st['qptr']=0
        oldy,oldxl,oldxr=q[st['qptr']]
        q[st['qptr']][0]=EMPTY
        st['qptr']+=1
        if st['qptr']==st['qtop']: crunch()
        yy=oldy&0x7fff
        for x in range(oldxl,oldxr+1): scr[yy][x]=fill
    return scr,st
