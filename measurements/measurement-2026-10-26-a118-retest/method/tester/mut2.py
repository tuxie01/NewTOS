# mut2.py <tree> [filter] -- retest mutations on a scratch copy (tree), test file = developer's aes_a118_lock_takers.rs
# + the previous Tester's 3 extra tests (renamed t2_) appended. One mutation at a time, source restored from memory after each.
import sys,subprocess
T=sys.argv[1]; flt=sys.argv[2:]
A=T+'/tos-api/src/aes/'
muts=[
# --- previous survivors (a-d) and removal (e)
('a close_window without update_beg','window.rs',"    // ([`wm_new`], [`wm_new_for_slot`]) nest on a drained or their own hold.\n    update_beg();\n","    // ([`wm_new`], [`wm_new_for_slot`]) nest on a drained or their own hold.\n"),
('b graf_dragbox without update_beg','graf.rs',"pub fn graf_dragbox(pb: &Aespb) {\n    super::window::update_beg();\n","pub fn graf_dragbox(pb: &Aespb) {\n"),
('c evnt_multi parked rect own_mouse','event.rs',"let own_m = super::input::owns_mouse(idx);","let own_m = super::input::own_mouse();"),
('d evnt_mouse parked rect own_mouse','event.rs',"|| (super::input::owns_mouse(idx) && rect_event(&m));","|| (super::input::own_mouse() && rect_event(&m));"),
('e end_unused_grant call removed','mod.rs',"        window::end_unused_grant();\n",""),
# --- own mutations on end_unused_grant
('f unsync -> drain_update_lock (all waiters, wrong hand-over)','window.rs',"        unsync(me);\n    }\n}\n\n/// `wind_update` (107)","        drain_update_lock();\n    }\n}\n\n/// `wind_update` (107)"),
('g grant not cleared (swap->load)','window.rs',"    if !UPDATE_GRANTED.get(me).is_some_and(|g| g.swap(false, Ordering::Relaxed)) {\n        return;\n    }\n    if UPDATE_LOCK","    if !UPDATE_GRANTED.get(me).is_some_and(|g| g.load(Ordering::Relaxed)) {\n        return;\n    }\n    if UPDATE_LOCK"),
('h applied before the gate (also when gate takes lock)','mod.rs',"    let takes_screen = match opcode {\n        19 |","    if matches!(opcode, 101 | 102 | 103 | 105) { window::end_unused_grant(); }\n    let takes_screen = match opcode {\n        19 |"),
('i applied after gate take too (no else)','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    }\n    if matches!(opcode, 101 | 102 | 103 | 105) {"),
('j same_holder check dropped','window.rs',"UPDATE_LOCK.load(Ordering::Relaxed) > 0 && same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me)","UPDATE_LOCK.load(Ordering::Relaxed) > 0"),
('k lock>0 check dropped','window.rs',"UPDATE_LOCK.load(Ordering::Relaxed) > 0 && same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me)","same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me)"),
('l any opcode (matches! dropped)','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    } else {"),
('m opcode 105 dropped','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    } else if matches!(opcode, 101 | 102 | 103) {"),
('n opcode 101 dropped','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    } else if matches!(opcode, 102 | 103 | 105) {"),
('o opcode 102 dropped','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    } else if matches!(opcode, 101 | 103 | 105) {"),
('p opcode 103 dropped','mod.rs',"    } else if matches!(opcode, 101 | 102 | 103 | 105) {","    } else if matches!(opcode, 101 | 102 | 105) {"),
('q unsync twice (double release)','window.rs',"        unsync(me);\n    }\n}\n\n/// `wind_update` (107)","        unsync(me);\n        unsync(me);\n    }\n}\n\n/// `wind_update` (107)"),
('r wrong identity unsync(CTL_PD)','window.rs',"        unsync(me);\n    }\n}\n\n/// `wind_update` (107)","        unsync(CTL_PD);\n    }\n}\n\n/// `wind_update` (107)"),
('s grant cleared only when released (swap inside)','window.rs',"    if !UPDATE_GRANTED.get(me).is_some_and(|g| g.swap(false, Ordering::Relaxed)) {\n        return;\n    }\n    if UPDATE_LOCK.load(Ordering::Relaxed) > 0 && same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me) {\n        unsync(me);\n    }","    if !UPDATE_GRANTED.get(me).is_some_and(|g| g.load(Ordering::Relaxed)) {\n        return;\n    }\n    if UPDATE_LOCK.load(Ordering::Relaxed) > 0 && same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me) {\n        UPDATE_GRANTED[me].store(false, Ordering::Relaxed);\n        unsync(me);\n    }"),
]
def run():
    r=subprocess.run(['cargo','test','-p','tos-api','--test','aes_a118_lock_takers'],cwd=T,capture_output=True,text=True)
    out=r.stdout+r.stderr
    fails=[l.split()[1] for l in out.splitlines() if l.startswith('test ') and l.endswith('FAILED')]
    res=[l for l in out.splitlines() if l.startswith('test result')]
    err='error[' in out or 'could not compile' in out
    return err,res,fails
if not flt or flt==['base']:
    print('UNMUTATED',run(),flush=True)
for name,f,a,b in muts:
    if flt and flt!=['base'] and not any(name.startswith(x+' ') for x in flt): continue
    if flt==['base']: break
    p=A+f; orig=open(p).read(); assert orig.count(a)==1,(name,orig.count(a))
    open(p,'w').write(orig.replace(a,b))
    try:
        err,res,fails=run()
        print(name,'->','COMPILE-ERR' if err else ('RED' if fails or (res and ' 0 passed' in res[-1]) else 'GREEN(survived)'),res[-1] if res else '??',fails,flush=True)
    finally:
        open(p,'w').write(orig)
