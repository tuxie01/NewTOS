#!/usr/bin/env python3
"""probe-mem12.py <scratch-tree> -- MEM12 3b scratch probe (never the shared tree). Adds serial lines, prefix [m12p]:
  * kernel/src/arch/aarch64/exceptions.rs: every Super (slot, argument, result, state after), every Supexec entry
    (slot, callback) and return (callback x0, state restored), the first 32 kbdvec upcalls (byte, handler, state);
  * kernel/src/host_env.rs (as measurement-2026-10-27-mem12-td-lp64/method/probe-conterm.py): once per trap-side input
    pump, the shell child's page-0 conterm and the 32-bit cells at $472/$476/$47E, printed on change.
Each anchor must occur exactly once. Marker MEM12-PROBE-TEMP."""
import pathlib, sys
T = pathlib.Path(sys.argv[1])
def patch(rel, pairs):
    f = T / rel; s = f.read_text()
    for a, b in pairs:
        assert s.count(a) == 1, (rel, a)
        s = s.replace(a, b)
    f.write_text(s)
patch("kernel/src/arch/aarch64/exceptions.rs", [
("    let result = unsafe { superstate::super_call(slot, frame.gpr[0], frame.sp_el0) };\n",
 "    let result = unsafe { superstate::super_call(slot, frame.gpr[0], frame.sp_el0) };\n"
 "    crate::kprintln!(\"[m12p] Super slot {} arg {:#x} -> {:#x}, super={}\", slot, frame.gpr[0], result, superstate::is_super(slot)); // MEM12-PROBE-TEMP\n"),
("    let codeptr = frame.gpr[0];\n",
 "    let codeptr = frame.gpr[0];\n"
 "    crate::kprintln!(\"[m12p] Supexec slot {} callback {:#x}, super was {}\", slot, codeptr, superstate::is_super(slot)); // MEM12-PROBE-TEMP\n"),
("        Disposition::SupexecReturn { prev_super } => {\n",
 "        Disposition::SupexecReturn { prev_super } => {\n"
 "            crate::kprintln!(\"[m12p] Supexec return slot {} x0 {:#x}, super back to {}\", slot_idx, handler_x0, prev_super); // MEM12-PROBE-TEMP\n"),
("        let prev_super = tos_api::bdos::process::superstate::is_super(slot);\n",
 "        let prev_super = tos_api::bdos::process::superstate::is_super(slot);\n"
 "        { static N: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0); if N.fetch_add(1, Ordering::Relaxed) < 32 { crate::kprintln!(\"[m12p] kbdvec upcall slot {} byte {:#04x} handler {:#x}, super was {}\", slot, byte, handler, prev_super); } } // MEM12-PROBE-TEMP\n"),
])
patch("kernel/src/host_env.rs", [
("    crate::ikbd::poll_repeat();\n}\n", "    crate::ikbd::poll_repeat();\n    m12_probe::step(); // MEM12-PROBE-TEMP\n}\n"),
])
f = T / "kernel/src/host_env.rs"
f.write_text(f.read_text() + '''
// MEM12-PROBE-TEMP: scratch-only page-0 watch of the shell child.
mod m12_probe {
    use core::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    pub fn step() {
        let Some(slot) = crate::launch::shell_child_slot() else { return };
        let Some(p0) = tos_api::bdos::process::mmu::page0_kva(slot) else { return };
        // SAFETY: scratch probe; the slot's page 0 is a static kernel page, read only.
        let (c, a, b, m) = unsafe {
            (
                p0.add(0x484).read_volatile(),
                (p0.add(0x472) as *const u32).read_unaligned(),
                (p0.add(0x476) as *const u32).read_unaligned(),
                (p0.add(0x47E) as *const u32).read_unaligned(),
            )
        };
        let sig = (slot as u64) << 56 ^ (c as u64) << 48 ^ (a as u64) << 32 ^ (b as u64) << 16 ^ m as u64;
        if LAST.swap(sig, Ordering::Relaxed) != sig {
            crate::kprintln!(
                "[m12p] slot {} conterm 0x{:02x} hdv_bpb 0x{:08x} hdv_rw 0x{:08x} hdv_mediach 0x{:08x}",
                slot, c, a, b, m
            );
        }
    }
}
''')
print("probe-mem12: patched", T)
