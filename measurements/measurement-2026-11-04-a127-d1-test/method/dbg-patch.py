#!/usr/bin/env python3 -I
"""dbg-patch.py <scratch-tree> -- TEMPORARY scratch-only instrumentation (never the shared tree):
menu::track records hz_200 + result at its end; the kernel's pump_input prints it once."""
import sys, pathlib
t = pathlib.Path(sys.argv[1])
m = t / "tos-api/src/aes/menu.rs"; s = m.read_text()
if "DBG_END" not in s:
    a = "    let r = track_owned(tree, backend);\n"
    assert s.count(a) == 1
    s = s.replace(a, a + "    DBG_END.store(((crate::bios::tick::hz_200() as u64) << 8) | if r.is_some() { 1 } else { 2 }, core::sync::atomic::Ordering::Relaxed);\n")
    s += "\npub static DBG_END: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);\n"
    m.write_text(s)
h = t / "kernel/src/host_env.rs"; s = h.read_text()
if "DBG_END" not in s:
    a = "    fn pump_input(&self) {\n"
    assert s.count(a) == 1
    s = s.replace(a, a + "        { let v = tos_api::aes::menu::DBG_END.swap(0, core::sync::atomic::Ordering::Relaxed); if v != 0 { crate::kprintln!(\"[dbg] track end hz_200 {} res {}\", v >> 8, v & 0xff); } }\n")
    h.write_text(s)
