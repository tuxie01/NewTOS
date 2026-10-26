import sys,subprocess,shutil,os
R='/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a118-test'
N=R+'/new/tos-api/src/aes/'; M=R+'/mut/tos-api/src/aes/'
muts={
'gate101':('mod.rs',None),
}
def sub(f,a,b,cnt=1):
    s=open(N+f).read()
    assert s.count(a)>=1,(f,a,s.count(a))
    s=s.replace(a,b,cnt)
    open(M+f,'w').write(s)
M_LIST=[
('gate wm_open 101','window.rs','101 => win(handle).is_some_and(|w| !w.open),','101 => false,'),
('gate wm_close 102','window.rs','102 | 103 => win(handle).is_some_and(|w| w.open),','102 => false, 103 => win(handle).is_some_and(|w| w.open),'),
('gate wm_delete 103','window.rs','102 | 103 => win(handle).is_some_and(|w| w.open),','102 => win(handle).is_some_and(|w| w.open), 103 => false,'),
('gate wind_set 105','window.rs','105 => pb.intin(1) == WF_DCOLOR || win(handle).is_some(),','105 => false,'),
('gate WF_DCOLOR only','window.rs','105 => pb.intin(1) == WF_DCOLOR || win(handle).is_some(),','105 => win(handle).is_some(),'),
('gate graf 70','mod.rs','19 | 36 | 50 | 52 | 70 | 71 => true,','19 | 36 | 50 | 52 | 71 => true,'),
('gate graf 71','mod.rs','19 | 36 | 50 | 52 | 70 | 71 => true,','19 | 36 | 50 | 52 | 70 => true,'),
('gate ap_exit 19','mod.rs','19 | 36 | 50 | 52 | 70 | 71 => true,','36 | 50 | 52 | 70 | 71 => true,'),
('gate fsel 90/91','mod.rs','90 | 91 => pb.addrin(0) != 0,','90 | 91 => false,'),
('fsel fm_own(true)','fsel.rs','super::window::fm_own(true);','',),
('fsel fm_own(false)','fsel.rs','super::window::fm_own(false);','',),
('quick-check evnt_multi','event.rs','let arm_own = super::input::owns_mouse(idx);','let arm_own = super::input::own_mouse();'),
('rect wait evnt_multi','event.rs','let own_m = super::input::owns_mouse(idx);','let own_m = super::input::own_mouse();'),
('rect wait evnt_mouse','event.rs','(super::input::owns_mouse(idx) && rect_event(&m))','(super::input::own_mouse() && rect_event(&m))'),
('wind_open beg','window.rs','        update_beg();\n        // `draw_change` first','        // `draw_change` first'),
('wind_open end','window.rs','update_end(); // `wm_update(END_UPDATE)` (`:1429`)',''),
('close_window beg','window.rs','    update_beg();\n    let r = w.curr;','    let r = w.curr;'),
('close_window end','window.rs','update_end(); // `wm_update(END_UPDATE)` (`:1456`)',''),
('wind_set beg','window.rs','    if takes {\n        update_beg();\n    }','    if takes {\n    }'),
('wind_set end','window.rs','    if takes {\n        update_end();\n    }','    if takes {\n    }'),
('rubberbox beg','graf.rs','pub fn graf_rubberbox(pb: &Aespb) {\n    super::window::update_beg();','pub fn graf_rubberbox(pb: &Aespb) {'),
('rubberbox end','graf.rs','    super::window::update_end();\n    pb.set_intout(1, w);','    pb.set_intout(1, w);'),
('dragbox beg','graf.rs','pub fn graf_dragbox(pb: &Aespb) {\n    super::window::update_beg();','pub fn graf_dragbox(pb: &Aespb) {'),
('dragbox end','graf.rs','    super::window::update_end();\n    pb.set_intout(1, lx);','    pb.set_intout(1, lx);'),
('appl_exit beg','appl.rs','        super::window::update_beg();\n        super::menu::mn_cleanup();','        super::menu::mn_cleanup();'),
('appl_exit end','appl.rs','    super::window::update_end();\n    pb.set_ret(1);\n    None','    pb.set_ret(1);\n    None'),
]
only=sys.argv[1:] 
for name,f,a,b in M_LIST:
    if only and not any(o in name for o in only): continue
    for ff in set(x[1] for x in M_LIST):
        shutil.copy(N+ff,M+ff)
    sub(f,a,b)
    r=subprocess.run('cd %s/mut && cargo test -p tos-api --test aes_a118_lock_takers 2>&1'%R,shell=True,capture_output=True,text=True)
    out=r.stdout
    res=[l for l in out.splitlines() if l.startswith('test result')]
    fails=[l.split()[1] for l in out.splitlines() if l.startswith('test ') and l.endswith('FAILED')]
    err='error[' in out or 'could not compile' in out
    print(name,'|','COMPILE-ERR' if err else (res[0] if res else '??'),'|',','.join(fails),flush=True)
for ff in set(x[1] for x in M_LIST): shutil.copy(N+ff,M+ff)
