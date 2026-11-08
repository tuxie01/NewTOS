# TESTER-TEMP scratch patch (applied to a git-archive tree in the scratchpad only, never the main checkout):
# desktop key 'c' = trap-free EL0 spin, ONE pump pass (evnt_multi MU_TIMER), trap-free spin, FLATHAND.
import sys
p = sys.argv[1] + "/programs/desktop/src/main.rs"
s = open(p).read()
a = "        // `wind_update(BEG_UPDATE); /* grab the screen */` (`deskmain.c:2094`)."
assert a in s
ins = """        if ev.which & MU_KEYBD != 0 && (ev.kret & 0xff) == i16::from(b'c') {
            tester_spin();
            let mut m = [0i16; 8];
            super::evnt_multi_msg(MU_TIMER, &mut m);
            tester_spin();
            super::graf_mouse(FLATHAND);
        }
"""
s = s.replace(a, ins + a, 1)
s += """
fn tester_spin() {
    let mut i: u64 = 0;
    while i < 3_000_000_000 {
        i = core::hint::black_box(i) + 1;
    }
}
"""
open(p, "w").write(s)
