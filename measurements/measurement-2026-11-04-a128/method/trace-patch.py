#!/usr/bin/env python3
"""trace-patch.py <scratch-tree> -- A128 measurement only: serial trace lines in a
SCRATCH copy of the tree (never the shared tree), tag `[a128]`:
  tos-api/src/aes/input.rs  wheel_change: each detent the file selector's claim
                            consumes, with the pointer and counted/dropped;
  tos-api/src/aes/fsel.rs   modal: each wheel take, slider page, thumb drag value
                            and the top row before/after;
  tos-api/src/aes/mod.rs    A128_SINK, the serial writer the kernel registers;
  kernel/src/host_env.rs    install() registers a128_sink (kprint!, serial only).
Each anchor must occur exactly once."""
import pathlib, sys
T = pathlib.Path(sys.argv[1])
SAY = '''
// A128-TRACE-TEMP: the line goes to the kernel's serial writer only (set by the
// scratch kernel in host_env.rs install), never to the screen console.
pub(crate) fn a128_say(args: core::fmt::Arguments) {
    struct W { b: [u8; 160], n: usize }
    impl core::fmt::Write for W {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for &c in s.as_bytes() { if self.n < self.b.len() { self.b[self.n] = c; self.n += 1; } }
            Ok(())
        }
    }
    let mut w = W { b: [0; 160], n: 0 };
    let _ = core::fmt::Write::write_fmt(&mut w, args);
    let f = super::A128_SINK.load(core::sync::atomic::Ordering::Relaxed);
    if f != 0 {
        // SAFETY: A128_SINK holds a `fn(&[u8])` stored by the scratch kernel.
        let f: fn(&[u8]) = unsafe { core::mem::transmute(f) };
        f(&w.b[..w.n]);
    }
}
'''
def patch(rel, old, new):
    p = T / rel; s = p.read_text()
    assert s.count(old) == 1, (rel, old[:60], s.count(old))
    p.write_text(s.replace(old, new))
patch("tos-api/src/aes/input.rs",
      "    if FSEL_WHEEL.feed(XRAT.load(Ordering::Relaxed), YRAT.load(Ordering::Relaxed), wheel_amount) {\n        return;\n    }\n",
      "    if FSEL_WHEEL.feed(XRAT.load(Ordering::Relaxed), YRAT.load(Ordering::Relaxed), wheel_amount) {\n"
      "        let (x, y) = (XRAT.load(Ordering::Relaxed), YRAT.load(Ordering::Relaxed));\n"
      "        a128_say(format_args!(\"[a128] detent {} at ({}, {}) {}\\r\\n\", wheel_amount, x, y, if FSEL_WHEEL.hit(x, y) { \"counted\" } else { \"dropped\" }));\n"
      "        return;\n    }\n")
p = T / "tos-api/src/aes/input.rs"; p.write_text(p.read_text() + SAY)
patch("tos-api/src/aes/fsel.rs",
      "        let wheel = super::input::fsel_wheel_take();\n        if wheel != 0 && scroll(tree, wheel) {\n            cur_sel = -1;\n        }\n",
      "        let wheel = super::input::fsel_wheel_take();\n"
      "        let top0 = unsafe { *SYS.top.get() };\n"
      "        if wheel != 0 && scroll(tree, wheel) {\n            cur_sel = -1;\n        }\n"
      "        if wheel != 0 { super::input::a128_say(format_args!(\"[a128] wheel take {} top {} -> {}\\r\\n\", wheel, top0, unsafe { *SYS.top.get() })); }\n")
patch("tos-api/src/aes/fsel.rs",
      "                if scroll(tree, delta) {\n                    cur_sel = -1;\n                }\n            } else if hit == FCLSBOX {",
      "                let top0 = unsafe { *SYS.top.get() };\n"
      "                if scroll(tree, delta) {\n                    cur_sel = -1;\n                }\n"
      "                super::input::a128_say(format_args!(\"[a128] slider hit {} at ({}, {}) {} delta {} top {} -> {}\\r\\n\", hit, mx, my, if page.is_some() { \"page\" } else { \"drag\" }, delta, top0, unsafe { *SYS.top.get() }));\n"
      "            } else if hit == FCLSBOX {")
patch("tos-api/src/aes/fsel.rs",
      "                        let value = super::graf::gr_slidebox(t, FSVSLID, FSVELEV, true);\n",
      "                        let value = super::graf::gr_slidebox(t, FSVSLID, FSVELEV, true);\n"
      "                        super::input::a128_say(format_args!(\"[a128] gr_slidebox value {} thumb ob_y {}\\r\\n\", value, t.y(FSVELEV)));\n")
p = T / "tos-api/src/aes/mod.rs"; p.write_text(p.read_text() + "\n// A128-TRACE-TEMP\npub static A128_SINK: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);\n")
patch("kernel/src/host_env.rs", "pub fn install() {\n", "pub fn install() {\n    tos_api::aes::A128_SINK.store(a128_sink as fn(&[u8]) as usize, core::sync::atomic::Ordering::Relaxed); // A128-TRACE-TEMP\n")
p = T / "kernel/src/host_env.rs"; p.write_text(p.read_text() + "\n// A128-TRACE-TEMP\nfn a128_sink(b: &[u8]) {\n    crate::kprint!(\"{}\", core::str::from_utf8(b).unwrap_or(\"?\"));\n}\n")
print("trace-patch: 6 sites patched in", T)
