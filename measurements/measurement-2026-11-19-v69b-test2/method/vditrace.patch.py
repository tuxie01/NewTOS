#!/usr/bin/env python3
"""vditrace.patch.py <scratch-tree> -- SCRATCH-ONLY instrument (never in a branch): extends the kernel's bios-arg-trace
hook (kernel/src/dbg_biostrace.rs `record`) with a decoder for AES/VDI selector 115 calls: one serial line per completed
vq_scrninfo (opcode 102, CONTRL[5]==1, INTIN[0]==2: the answer words), vq_extnd (102 otherwise: INTIN[0], INTOUT[4]) and
vro_cpyfm (109: both MFDB pointers and, for a non-null MFDB, fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes; INTIN[0]).
LP64 EL0 callers only (zView is LP64): VDIPB = five 8-byte pointers; MFDB pointers at CONTRL[7+n*4]."""
import sys
p = sys.argv[1] + "/kernel/src/dbg_biostrace.rs"
s = open(p).read()
old = "pub fn record(class: TrapClass, args: &TrapArgs, result: i64) {\n"
new = '''pub fn record(class: TrapClass, args: &TrapArgs, result: i64) {
    if let TrapClass::Gem = class {
        if args.regs[0] == 115 && args.regs[1] != 0 { vdi_probe(args.regs[1]); }
        return;
    }
'''
assert old in s
s = s.replace(old, new, 1)
s += '''
// ---- SCRATCH vditrace (V69 part B tester run) ----
fn vdi_probe(block: u64) {
    // SAFETY: the same param-block read dbg_qedtrace::record_vdi performs; LP64 caller (8-byte pointers).
    unsafe {
        let rd = |a: u64| (a as *const u64).read_unaligned();
        let contrl = rd(block) as *const i16;
        let intin = rd(block + 8) as *const i16;
        let intout = rd(block + 24) as *const i16;
        let c = |i: usize| contrl.add(i).read();
        let op = c(0);
        let slot = tos_api::bdos::process::sched::running_slot();
        match op {
            102 => {
                if c(5) == 1 && intin.read() == 2 {
                    crate::kprint!("[vditrace] U{} vq_scrninfo: out[0..8] = {:#06x} {:#06x} {:#06x} {:#06x} {:#06x} {:#06x} {:#06x} {:#06x}\\n",
                        slot, intout.read() as u16, intout.add(1).read() as u16, intout.add(2).read() as u16, intout.add(3).read() as u16,
                        intout.add(4).read() as u16, intout.add(5).read() as u16, intout.add(6).read() as u16, intout.add(7).read() as u16);
                } else {
                    crate::kprint!("[vditrace] U{} vq_extnd: intin0={} intout4(planes)={}\\n", slot, intin.read(), intout.add(4).read());
                }
            }
            109 => {
                let mfdb = |n: usize| {
                    let p = rd(contrl as u64 + 2 * (7 + n * 4) as u64);
                    if p == 0 { return (0u64, 0i16, 0i16, 0i16, 0i16, 0i16); }
                    let w = |o: u64| ((p + o) as *const i16).read_unaligned();
                    (p, w(8), w(10), w(12), w(14), w(16))
                };
                let (sp, sw, sh, sd, ss, sn) = mfdb(0);
                let (dp, dw, dh, dd, ds, dn) = mfdb(1);
                crate::kprint!("[vditrace] U{} vro_cpyfm mode={} src={:#x} (w{} h{} wd{} stand{} planes{}) dst={:#x} (w{} h{} wd{} stand{} planes{})\\n",
                    slot, intin.read(), sp, sw, sh, sd, ss, sn, dp, dw, dh, dd, ds, dn);
            }
            _ => {}
        }
    }
}
'''
open(p, "w").write(s)
print("patched", p)
