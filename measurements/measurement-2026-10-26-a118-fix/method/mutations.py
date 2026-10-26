# mutations.py <scratch> -- A118 rework: each mutation applied to the WORKING TREE file, the A118 host test run, the file written back
# from the copy in memory (try/finally; byte-compared with a scratch copy afterwards: restored). Output checks/mutations.txt.
import sys,subprocess,shutil
S=sys.argv[1]; WS='/Volumes/Daten/Development/PiRusTOS'
muts=[('a close_window without update_beg','tos-api/src/aes/window.rs',"    // ([`wm_new`], [`wm_new_for_slot`]) nest on a drained or their own hold.\n    update_beg();\n","    // ([`wm_new`], [`wm_new_for_slot`]) nest on a drained or their own hold.\n"),
 ('b graf_dragbox without update_beg','tos-api/src/aes/graf.rs',"pub fn graf_dragbox(pb: &Aespb) {\n    super::window::update_beg();\n","pub fn graf_dragbox(pb: &Aespb) {\n"),
 ('c evnt_multi parked rect own_mouse','tos-api/src/aes/event.rs',"let own_m = super::input::owns_mouse(idx);","let own_m = super::input::own_mouse();"),
 ('d evnt_mouse parked rect own_mouse','tos-api/src/aes/event.rs',"|| (super::input::owns_mouse(idx) && rect_event(&m));","|| (super::input::own_mouse() && rect_event(&m));"),
 ('e no end_unused_grant','tos-api/src/aes/mod.rs',"        window::end_unused_grant();\n","")]
for name,f,a,b in muts:
    p=WS+'/'+f; orig=open(p).read(); assert orig.count(a)==1,name
    open(p,'w').write(orig.replace(a,b))
    try:
        r=subprocess.run(['cargo','test','-p','tos-api','--test','aes_a118_lock_takers'],cwd=WS,capture_output=True,text=True)
        out=r.stdout+r.stderr
        fails=[l.split()[1] for l in out.splitlines() if l.startswith('test ') and l.endswith('FAILED')]
        res=[l for l in out.splitlines() if l.startswith('test result')]
        print(name,'->','RED' if fails else 'GREEN(survived)',res[-1] if res else 'BUILD FAIL',fails)
    finally:
        open(p,'w').write(orig)
