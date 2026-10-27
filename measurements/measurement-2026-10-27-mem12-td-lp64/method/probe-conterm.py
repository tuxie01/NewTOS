#!/usr/bin/env python3
"""probe-conterm.py <scratch-tree> -- MEM12 scratch probe (never the shared tree): append a module to
kernel/src/host_env.rs that, once per trap-side input pump (pump_devices), reads the shell child's page 0
through the kernel alias and prints a line whenever conterm ($484) or one of the three 32-bit words at
$472/$476/$47E changes:
  [m12] slot S conterm 0xCC hdv_bpb 0x........ hdv_rw 0x........ hdv_mediach 0x........
"""
import pathlib, sys
f = pathlib.Path(sys.argv[1]) / "kernel/src/host_env.rs"
s = f.read_text()
anchor = "    crate::ikbd::poll_repeat();\n}\n"
assert s.count(anchor) == 1, "anchor"
s = s.replace(anchor, "    crate::ikbd::poll_repeat();\n    m12_probe::step(); // MEM12-PROBE-TEMP\n}\n")
s += '''
// MEM12-PROBE-TEMP: scratch-only page-0 watch of the shell child (measurement-2026-10-27-mem12-td-lp64).
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
                "[m12] slot {} conterm 0x{:02x} hdv_bpb 0x{:08x} hdv_rw 0x{:08x} hdv_mediach 0x{:08x}",
                slot, c, a, b, m
            );
        }
    }
}
'''
f.write_text(s)
print("probe-conterm: patched", f)
