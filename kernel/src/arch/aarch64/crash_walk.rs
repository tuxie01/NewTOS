//! Crash diagnosis of the debug feature `crash-walk` (TASK/PROC08): extra
//! serial lines, prefixed `[crashwalk]`, right after the `[crash]` line of a
//! program crash ([`super::crash::report_and_wait`]). They answer one
//! question for the fault address (and, for an EL0 crash, the page of the
//! PC): was the page mapped in the tables NewTOS keeps for the faulting slot
//! while the hardware reported the fault, or not?
//!
//! For each address four views, taken in this order before anything else
//! runs:
//!
//! 1. **hardware now** — `AT S1E0R` with the faulting root still in
//!    `TTBR0_EL1`, result from `PAR_EL1` (below);
//! 2. **roots** — `TTBR0_EL1` (table address and ASID) against the root
//!    NewTOS expects for the slot ([`mmu::l0_pa_for`]) and the one the slot
//!    records ([`Process::l0_pa`](tos_api::bdos::process::Process::l0_pa));
//! 3. **tables** — a checked software walk through the slot's tables
//!    ([`mmu::diag::walk`]: descriptor and table kind at every level, where
//!    it stops), and through the root in `TTBR0_EL1` too if that is another
//!    slot's;
//! 4. **model** — the rights model's view of the page for the slot
//!    ([`page_view`]: block, mode, owners, memory space, the
//!    descriptor the model expects).
//!
//! The lines go to the serial line only (`kprintln!`), never to the screen;
//! nothing allocates, every table pointer is checked before it is read, and
//! the existing report is unchanged. Off by default: with the feature off
//! this module does not exist.
//!
//! ## The hardware probe (ARM ARM DDI 0487 M.d)
//!
//! * `AT S1E0R, <Xt>` — stage 1 translation of `Xt` "with permissions as if
//!   reading from the given virtual address from EL0", EL1&0 regime here
//!   (C5.4.5, C5-1179; encoding `op0=0b01 op1=0b000 CRn=0b0111 CRm=0b1000
//!   op2=0b010`, executable at EL1). A fault is not taken as an exception
//!   but reported in `PAR_EL1`, except a synchronous External abort on the
//!   walk (`RNHWXL`, D8-7842) — not expected for tables in DRAM.
//! * It is IMPLEMENTATION DEFINED whether the result comes from a TLB or
//!   from a table walk (`RNPZWX`, D8-7733): the probe tells what the MMU
//!   *now* answers for the address, not which of the two it used.
//! * A context synchronization event is needed before `PAR_EL1` shows the
//!   result (`IJTYVP`, D8-7734): `isb` between `AT` and `MRS`.
//! * `PAR_EL1`, 64-bit format (the A72 has no FEAT_D128; D24.2.141):
//!   bit 0 `F` = 0 success, then `PA[47:12]` in bits [47:12] (D24-9519);
//!   `F` = 1 abort, then `FST` in bits [6:1] with the Data Abort fault
//!   status encoding — `0b0001LL` translation fault level `LL`,
//!   `0b0010LL` access flag fault, `0b0011LL` permission fault (D24-9526) —,
//!   `S` bit 9 (stage 2) and `PTW` bit 8 (D24-9525). `MRS <Xt>, PAR_EL1` is
//!   `op0=0b11 op1=0b000 CRn=0b0111 CRm=0b0100 op2=0b000` (D24-9528).
//! * `TTBR0_EL1`, 64-bit format: `ASID` in bits [63:48], `BADDR` in
//!   [47:1] (D24.2.208, D24-10043).

use core::fmt::Write as _;

use tos_api::bdos::process::memprot::diag::{page_view, PageView};
use tos_api::bdos::process::mmu::diag::{self as walkdiag, Outcome, TableKind, Walk};
use tos_api::bdos::process::{mmu, PROCESS_TABLE};
use tos_api::bios::crash::CrashReport;

/// The serial writer behind `kprintln!`.
type Serial = crate::hal::uart::UartWriter<'static, crate::console::SerialConsole>;

/// A fresh [`Serial`].
fn uart() -> Serial {
    crate::hal::uart::UartWriter(&crate::console::CONSOLE)
}

/// `TTBR0_EL1.ASID`, bits [63:48] (D24-10043).
const TTBR_ASID_SHIFT: u32 = 48;
/// `TTBR0_EL1.BADDR`, bits [47:1] (D24-10043); our roots are 4-KiB aligned.
const TTBR_BADDR_MASK: u64 = 0x0000_FFFF_FFFF_FFFE;
/// `PAR_EL1.F`, bit 0 (D24-9519, D24-9525).
const PAR_F: u64 = 1;
/// `PAR_EL1.PA[47:12]` on success (D24-9519).
const PAR_PA_MASK: u64 = 0x0000_FFFF_FFFF_F000;

/// `TTBR0_EL1` as it is now — in the crash path still the faulting root.
fn read_ttbr0() -> u64 {
    let v: u64;
    // SAFETY: a system-register read, permitted at EL1.
    unsafe { core::arch::asm!("mrs {v}, ttbr0_el1", v = out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// `AT S1E0R` on `va`, then `PAR_EL1` (module docs: C5.4.5, `IJTYVP`).
fn at_s1e0r(va: u64) -> u64 {
    let par: u64;
    // SAFETY: EL1; the instruction only translates and writes PAR_EL1,
    // which nothing else in the kernel uses. A translation, access flag or
    // permission fault is reported in PAR_EL1, not raised (`RNHWXL`).
    unsafe {
        core::arch::asm!(
            "at s1e0r, {va}",
            "isb",
            "mrs {par}, par_el1",
            va = in(reg) va,
            par = out(reg) par,
            options(nostack, preserves_flags),
        );
    }
    par
}

/// One address the diagnosis looks at, with its hardware probe.
#[derive(Copy, Clone)]
struct Probe {
    label: &'static str,
    va: u64,
    par: u64,
}

/// Print the diagnosis for the crash `report` of `slot` (module docs).
/// Called by [`super::crash::report_and_wait`] right after its `[crash]`
/// line, before the report text and the key wait.
pub fn report(slot: usize, report: &CrashReport<'_>) {
    // The hardware first, closest to the fault: nothing below changes
    // TTBR0_EL1 or the TLB, but the probe should see what the fault saw.
    let ttbr0 = read_ttbr0();
    let el0 = report.pstate & 0xF == 0;
    let mut probes: [Option<Probe>; 2] = [None, None];
    if let Some(far) = report.far {
        probes[0] = Some(Probe { label: "far", va: far, par: at_s1e0r(far) });
    }
    let pc_page = report.pc & !(mmu::PAGE_SIZE - 1);
    if el0 && report.far.is_none_or(|f| f & !(mmu::PAGE_SIZE - 1) != pc_page) {
        probes[1] = Some(Probe { label: "pc", va: report.pc, par: at_s1e0r(report.pc) });
    }

    let mut w = uart();
    let _ = write!(
        w,
        "[crashwalk] slot {} src={} esr=0x{:08x} pc=0x{:x} far=",
        slot,
        if el0 { "el0" } else { "el1-uaccess" },
        report.esr as u32,
        report.pc,
    );
    let _ = match report.far {
        Some(f) => writeln!(w, "0x{f:x}"),
        None => writeln!(w, "none"),
    };

    // Roots.
    let root_table = ttbr0 & TTBR_BADDR_MASK;
    let asid = ttbr0 >> TTBR_ASID_SHIFT;
    let ttbr_slot = walkdiag::slot_of_root(ttbr0);
    let expected = mmu::l0_pa_for(slot);
    let recorded = PROCESS_TABLE.get(slot).map_or(0, |p| p.l0_pa.load(core::sync::atomic::Ordering::Acquire));
    let _ = write!(w, "[crashwalk] ttbr0=0x{ttbr0:016x} asid={asid} root=");
    let _ = match ttbr_slot {
        Some(s) => write!(w, "slot {s}"),
        None if root_table == crate::arch::aarch64::mmu::empty_l0_pa() => write!(w, "empty"),
        None => write!(w, "unknown"),
    };
    let _ = writeln!(
        w,
        "; slot {} expects 0x{:x} records 0x{:x} -> {}",
        slot,
        expected,
        recorded,
        if root_table == expected && recorded == expected { "same" } else { "DIFFERENT" },
    );

    for p in probes.iter().flatten() {
        let walk = walkdiag::walk(slot, p.va);
        print_walk(p.label, &walk);
        if let Some(other) = ttbr_slot.filter(|&s| s != slot) {
            print_walk(p.label, &walkdiag::walk(other, p.va));
        }
        print_par(p);
        // SAFETY: the crash path runs in the trap of the faulting program,
        // under the BDOS lock; the view only reads.
        let view = unsafe { page_view(slot, p.va) };
        print_view(p.label, slot, &view);
        print_summary(p, &walk, &view);
    }
}

/// `L0[i]=desc(kind) … -> outcome` and the region's sharing line.
fn print_walk(label: &str, walk: &Walk) {
    let mut w = uart();
    let _ = write!(w, "[crashwalk] {} va=0x{:x} walk slot {} root 0x{:x}:", label, walk.va, walk.slot, walk.root_pa);
    for (level, step) in walk.steps.iter().enumerate() {
        if let Some(s) = step {
            let _ = write!(w, " L{}[{}]=0x{:016x}(", level, s.index, s.desc);
            write_kind(&mut w, s.kind);
            let _ = write!(w, ")");
        }
    }
    let _ = match walk.outcome {
        Outcome::BadSlot => writeln!(w, " -> bad slot"),
        Outcome::OutOfRange => writeln!(w, " -> above the user tables"),
        Outcome::Leaf { level, oa } => writeln!(w, " -> mapped L{level} oa=0x{oa:x}"),
        Outcome::Invalid { level } => writeln!(w, " -> INVALID at L{level}"),
        Outcome::NotTable { level } => writeln!(w, " -> no table at L{level}"),
        Outcome::BadTable { level, pa } => writeln!(w, " -> BAD TABLE 0x{pa:x} for L{level}, not read"),
    };
    if let Some(t) = walk.l2_template {
        let _ = writeln!(
            w,
            "[crashwalk] {} l2 template=0x{:016x} l3 also named by slots=0x{:x} template={}",
            label,
            t,
            walk.l3_also_slots,
            if walk.l3_also_template { "yes" } else { "no" },
        );
    }
}

fn write_kind(w: &mut Serial, k: TableKind) {
    let _ = match k {
        TableKind::SlotL0(s) => write!(w, "s{s}.L0"),
        TableKind::SlotL1(s) => write!(w, "s{s}.L1"),
        TableKind::SlotL2(s, g) => write!(w, "s{s}.L2g{g}"),
        TableKind::SlotL3Zero(s) => write!(w, "s{s}.L3z"),
        TableKind::SharedL2(g) => write!(w, "sh.L2g{g}"),
        TableKind::SharedL3Zero => write!(w, "sh.L3z"),
        TableKind::SharedL3Static(i) => write!(w, "sh.L3#{i}"),
        TableKind::Arena { idx, used } => write!(w, "arena#{} {}", idx, if used { "used" } else { "FREE" }),
        TableKind::Unknown => write!(w, "unknown"),
    };
}

/// The `AT S1E0R` result (module docs, D24-9519/9525/9526).
fn print_par(p: &Probe) {
    let mut w = uart();
    let _ = write!(w, "[crashwalk] {} AT S1E0R par=0x{:016x}: ", p.label, p.par);
    if p.par & PAR_F == 0 {
        let _ = writeln!(w, "ok pa=0x{:x}", (p.par & PAR_PA_MASK) | (p.va & (mmu::PAGE_SIZE - 1)));
        return;
    }
    let fst = (p.par >> 1) & 0x3F;
    let level = fst & 0b11;
    let what = match fst >> 2 {
        0b0001 => "translation",
        0b0010 => "access flag",
        0b0011 => "permission",
        0b0000 => "address size",
        _ => "other",
    };
    let _ = writeln!(
        w,
        "fault fst=0x{:02x} ({} L{}) s={} ptw={}",
        fst,
        what,
        level,
        (p.par >> 9) & 1,
        (p.par >> 8) & 1,
    );
}

/// The model's view line(s).
fn print_view(label: &str, slot: usize, v: &PageView) {
    let mut w = uart();
    if !v.enabled {
        let _ = writeln!(w, "[crashwalk] {label} model slot {slot}: memprot off");
        return;
    }
    let _ = write!(w, "[crashwalk] {} model slot {}: pool={} ", label, slot, if v.in_pool { "yes" } else { "no" });
    let _ = match v.block {
        Some(b) => {
            let _ = write!(w, "block 0x{:x}-0x{:x} mode=", b.start, b.end);
            let _ = match b.mode {
                Some(m) => write!(w, "{m:?}"),
                None => write!(w, "none"),
            };
            let _ = write!(w, " own=0x{:x} further=", b.own);
            for (i, f) in b.further.iter().enumerate() {
                let _ = write!(w, "{}0x{:x}", if i == 0 { "" } else { "," }, f);
            }
            write!(w, " owners=0x{:x} inherit=0x{:x}", b.owners, b.inherit)
        }
        None => write!(w, "block none"),
    };
    let _ = writeln!(w, "{}", if v.lists_ok { "" } else { " (pool list walk capped)" });
    let _ = write!(
        w,
        "[crashwalk] {} model slot {}: live=0x{:x} slot_live={} slot_kva=0x{:x} space=0x{:x} group=0x{:x} temp=0x{:x} holds={} expect=",
        label,
        slot,
        v.live,
        yes_no(v.slot_live),
        v.slot_kva,
        v.space_root,
        v.group,
        v.temp,
        yes_no(v.holds),
    );
    write_opt(&mut w, v.expected);
    let _ = write!(w, " shared=");
    write_opt(&mut w, v.expected_shared);
    let _ = write!(w, " private needed=");
    let _ = match v.needs_private {
        Some(n) => write!(w, "{}", yes_no(n)),
        None => write!(w, "unknown"),
    };
    let _ = writeln!(
        w,
        " has={} shortfalls={} pending={}",
        yes_no(v.has_private),
        v.shortfalls,
        yes_no(v.pending),
    );
}

/// One line that puts the three answers side by side: the model, the
/// tables (software walk), the hardware (`AT S1E0R`).
fn print_summary(p: &Probe, walk: &Walk, v: &PageView) {
    let model = match v.expected {
        _ if !v.enabled => "n/a",
        Some(d) if d & 1 != 0 => "valid",
        Some(_) => "invalid",
        None => "n/a",
    };
    let table = match walk.outcome {
        Outcome::Leaf { .. } => "valid",
        Outcome::Invalid { .. } => "invalid",
        Outcome::NotTable { .. } | Outcome::BadTable { .. } => "bad",
        Outcome::BadSlot | Outcome::OutOfRange => "n/a",
    };
    let hw = if p.par & PAR_F == 0 { "ok" } else { "fault" };
    // The L3 descriptor against the model's, where both exist.
    let same = match (walk.steps[3], v.expected) {
        (Some(s), Some(e)) if v.enabled => {
            if s.desc == e {
                "yes"
            } else {
                "NO"
            }
        }
        _ => "n/a",
    };
    let mut w = uart();
    let _ = writeln!(
        w,
        "[crashwalk] {} summary: model={} table={} at={} table==model {}",
        p.label, model, table, hw, same,
    );
}

fn write_opt(w: &mut Serial, v: Option<u64>) {
    let _ = match v {
        Some(d) => write!(w, "0x{d:016x}"),
        None => write!(w, "n/a"),
    };
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}
