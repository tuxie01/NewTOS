//! The crash diagnosis of the debug feature `crash-walk` (TASK/PROC08), as
//! data: what the kernel collects once at a program crash
//! ([`CrashDiag`], filled by [`collect`] from the hardware probe the kernel
//! takes first), and its two renderings —
//!
//! * [`write_serial`]: the detailed `[crashwalk] …` lines for the serial
//!   line (they reach a terminal only with `serial=on`);
//! * [`write_screen`]: a compact block for the on-screen crash report, read
//!   off a photo of the screen when the card runs with the serial line off.
//!
//! Both render the same [`CrashDiag`], so the two outputs cannot disagree
//! (the tables are walked once). Formatting only, no allocation; host-tested
//! (`tests/proc08_crash_walk.rs`).
//!
//! What the four views mean and how they are taken (`AT S1E0R`, `PAR_EL1`,
//! `TTBR0_EL1`, ARM ARM DDI 0487 M.d) is documented with the kernel half,
//! `kernel/src/arch/aarch64/crash_walk.rs`.
//!
//! ## The screen block
//!
//! Numbers are hex without `0x` (addresses, descriptors, masks, `par`),
//! except the slot numbers, table indices `[i]` and levels `L<n>`, which are
//! decimal as in the serial lines. One header line, then per probed address
//! (`far`, and `pc` when the PC's page is another one) the walk on two lines
//! (L0–L2, then L3 and where the walk ended; two more for the root in
//! `TTBR0_EL1` when that is another slot's), the hardware answer, the rights
//! model on three lines (one when memory protection is off) and the
//! summary:
//!
//! ```text
//! crash-walk slot 1 el0: ttbr0 2e6000 asid 0 = slot 1, exp 2e6000 rec 2e6000 same
//! far 1a2c000 s1: L0[0]=2e7003 s1.L0  L1[0]=2e8003 s1.L1  L2[13]=11c6003 s1.L2g0
//!   L3[44]=0 arena#454 used -> INVALID at L3  also 0 tpl no
//!   AT S1E0R par=80f: FAULT translation L3 fst=07 s=0 ptw=0
//!   model: pool yes, no block
//!   model: holds no expect 0 shared 0 pend no
//!   model: priv need yes has yes short 0 live 3 group 2 temp 0 space 1a1a000
//!   => model=invalid table=invalid at=fault table==model yes
//! ```
//!
//! For realistic values every line stays within 80 columns (the console at
//! 640×480); the Pi's 1920×1080 console has 240.

use core::fmt::{self, Write};
use core::sync::atomic::Ordering;

use super::memprot::diag::{page_view, PageView};
use super::mmu::diag::{self as walkdiag, Outcome, TableKind, Walk};
use super::{mmu, PROCESS_TABLE};
use crate::bios::crash::CrashReport;

/// `TTBR0_EL1.ASID`, bits [63:48] (ARM ARM D24-10043).
pub const TTBR_ASID_SHIFT: u32 = 48;
/// `TTBR0_EL1.BADDR`, bits [47:1] (D24-10043); our roots are 4-KiB aligned.
pub const TTBR_BADDR_MASK: u64 = 0x0000_FFFF_FFFF_FFFE;
/// `PAR_EL1.F`, bit 0 (D24-9519, D24-9525).
pub const PAR_F: u64 = 1;
/// `PAR_EL1.PA[47:12]` on success (D24-9519).
pub const PAR_PA_MASK: u64 = 0x0000_FFFF_FFFF_F000;

/// One address the hardware was asked about, before anything else ran:
/// `AT S1E0R` on `va`, the `PAR_EL1` it left.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct HwProbe {
    /// `"far"` or `"pc"`.
    pub label: &'static str,
    /// The address.
    pub va: u64,
    /// `PAR_EL1` after `AT S1E0R`.
    pub par: u64,
}

/// What the root in `TTBR0_EL1` is.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RootOwner {
    /// Slot `0`'s root.
    Slot(usize),
    /// The kernel's empty user root.
    Empty,
    /// None of ours.
    Unknown,
}

/// Everything known about one probed address.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ProbeDiag {
    /// The hardware's answer.
    pub hw: HwProbe,
    /// The walk through the faulting slot's tables.
    pub walk: Walk,
    /// The walk through the tables of the slot whose root is in
    /// `TTBR0_EL1`, when that is another slot.
    pub other: Option<Walk>,
    /// The rights model's view of the page for the faulting slot.
    pub view: PageView,
}

/// The whole diagnosis of one crash.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CrashDiag {
    /// The faulting process's slot.
    pub slot: usize,
    /// The fault came from EL0 (otherwise: a kernel user access on its
    /// behalf).
    pub el0: bool,
    /// `ESR_EL1`.
    pub esr: u64,
    /// `ELR_EL1`.
    pub pc: u64,
    /// `FAR_EL1`, when valid.
    pub far: Option<u64>,
    /// `TTBR0_EL1` at the crash.
    pub ttbr0: u64,
    /// Whose root that is.
    pub root: RootOwner,
    /// The root NewTOS expects for the slot ([`mmu::l0_pa_for`]).
    pub expected: u64,
    /// The root the slot records (`Process::l0_pa`).
    pub recorded: u64,
    /// `far`, then `pc` (see [`probe_addresses`]).
    pub probes: [Option<ProbeDiag>; 2],
}

/// The fault came from EL0: `SPSR_EL1.M[3:0]` = `EL0t`.
fn from_el0(report: &CrashReport<'_>) -> bool {
    report.pstate & 0xF == 0
}

/// The addresses to probe for `report`: the fault address, and for an EL0
/// crash the PC when its page is not the fault address's.
pub fn probe_addresses(report: &CrashReport<'_>) -> [Option<(&'static str, u64)>; 2] {
    let page = !(mmu::PAGE_SIZE - 1);
    let pc_page = report.pc & page;
    let far = report.far.map(|f| ("far", f));
    let pc = if from_el0(report) && report.far.is_none_or(|f| f & page != pc_page) {
        Some(("pc", report.pc))
    } else {
        None
    };
    [far, pc]
}

/// Collect the diagnosis for the crash `report` of `slot`, given the
/// hardware state the kernel read first: `ttbr0` and the `AT S1E0R` probes
/// `hw` (for [`probe_addresses`]); `empty_l0_pa` is the kernel's empty user
/// root. Reads only.
///
/// # Safety
///
/// BDOS giant lock held — the crash path: the trap of the faulting
/// program, single CPU ([`page_view`]).
pub unsafe fn collect(
    slot: usize,
    report: &CrashReport<'_>,
    ttbr0: u64,
    hw: [Option<HwProbe>; 2],
    empty_l0_pa: u64,
) -> CrashDiag {
    let ttbr_slot = walkdiag::slot_of_root(ttbr0);
    let root = match ttbr_slot {
        Some(s) => RootOwner::Slot(s),
        None if ttbr0 & TTBR_BADDR_MASK == empty_l0_pa => RootOwner::Empty,
        None => RootOwner::Unknown,
    };
    let mut probes = [None, None];
    for (out, h) in probes.iter_mut().zip(hw) {
        let Some(h) = h else { continue };
        *out = Some(ProbeDiag {
            hw: h,
            walk: walkdiag::walk(slot, h.va),
            other: ttbr_slot.filter(|&s| s != slot).map(|s| walkdiag::walk(s, h.va)),
            // SAFETY: lock held (caller).
            view: unsafe { page_view(slot, h.va) },
        });
    }
    CrashDiag {
        slot,
        el0: from_el0(report),
        esr: report.esr,
        pc: report.pc,
        far: report.far,
        ttbr0,
        root,
        expected: mmu::l0_pa_for(slot),
        recorded: PROCESS_TABLE.get(slot).map_or(0, |p| p.l0_pa.load(Ordering::Acquire)),
        probes,
    }
}

impl CrashDiag {
    /// The table address in `TTBR0_EL1`.
    pub fn root_table(&self) -> u64 {
        self.ttbr0 & TTBR_BADDR_MASK
    }

    /// `TTBR0_EL1.ASID`.
    pub fn asid(&self) -> u64 {
        self.ttbr0 >> TTBR_ASID_SHIFT
    }

    /// `TTBR0_EL1`, the expected and the recorded root all agree.
    pub fn roots_same(&self) -> bool {
        self.root_table() == self.expected && self.recorded == self.expected
    }

    fn source(&self) -> &'static str {
        if self.el0 { "el0" } else { "el1-uaccess" }
    }
}

/// The three answers side by side — model, tables, hardware — and whether
/// the L3 descriptor equals the model's.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    /// `valid` / `invalid` / `n/a`.
    pub model: &'static str,
    /// `valid` / `invalid` / `bad` / `n/a`.
    pub table: &'static str,
    /// `ok` / `fault`.
    pub at: &'static str,
    /// `yes` / `NO` / `n/a`.
    pub same: &'static str,
}

/// See [`Summary`].
pub fn summary(p: &ProbeDiag) -> Summary {
    let v = &p.view;
    let model = match v.expected {
        _ if !v.enabled => "n/a",
        Some(d) if d & 1 != 0 => "valid",
        Some(_) => "invalid",
        None => "n/a",
    };
    let table = match p.walk.outcome {
        Outcome::Leaf { .. } => "valid",
        Outcome::Invalid { .. } => "invalid",
        Outcome::NotTable { .. } | Outcome::BadTable { .. } => "bad",
        Outcome::BadSlot | Outcome::OutOfRange => "n/a",
    };
    let at = if p.hw.par & PAR_F == 0 { "ok" } else { "fault" };
    let same = match (p.walk.steps[3], v.expected) {
        (Some(s), Some(e)) if v.enabled => {
            if s.desc == e {
                "yes"
            } else {
                "NO"
            }
        }
        _ => "n/a",
    };
    Summary { model, table, at, same }
}

/// The decoded fault side of `PAR_EL1` (D24-9525, D24-9526).
struct ParFault {
    fst: u64,
    what: &'static str,
    level: u64,
    s: u64,
    ptw: u64,
}

fn par_fault(par: u64) -> ParFault {
    let fst = (par >> 1) & 0x3F;
    let what = match fst >> 2 {
        0b0001 => "translation",
        0b0010 => "access flag",
        0b0011 => "permission",
        0b0000 => "address size",
        _ => "other",
    };
    ParFault { fst, what, level: fst & 0b11, s: (par >> 9) & 1, ptw: (par >> 8) & 1 }
}

/// The output address `PAR_EL1` gives for `va` on success.
fn par_pa(par: u64, va: u64) -> u64 {
    (par & PAR_PA_MASK) | (va & (mmu::PAGE_SIZE - 1))
}

fn write_kind<W: Write>(w: &mut W, k: TableKind) -> fmt::Result {
    match k {
        TableKind::SlotL0(s) => write!(w, "s{s}.L0"),
        TableKind::SlotL1(s) => write!(w, "s{s}.L1"),
        TableKind::SlotL2(s, g) => write!(w, "s{s}.L2g{g}"),
        TableKind::SlotL3Zero(s) => write!(w, "s{s}.L3z"),
        TableKind::SharedL2(g) => write!(w, "sh.L2g{g}"),
        TableKind::SharedL3Zero => write!(w, "sh.L3z"),
        TableKind::SharedL3Static(i) => write!(w, "sh.L3#{i}"),
        TableKind::Arena { idx, used } => write!(w, "arena#{} {}", idx, if used { "used" } else { "FREE" }),
        TableKind::Unknown => write!(w, "unknown"),
    }
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

// ─── Serial lines ───────────────────────────────────────────────────────

/// The detailed serial lines, each prefixed `[crashwalk] ` (the reading of
/// every line: TASK/PROC08 "Stand Debug-Feature").
pub fn write_serial<W: Write>(w: &mut W, d: &CrashDiag) -> fmt::Result {
    write!(
        w,
        "[crashwalk] slot {} src={} esr=0x{:08x} pc=0x{:x} far=",
        d.slot,
        d.source(),
        d.esr as u32,
        d.pc,
    )?;
    match d.far {
        Some(f) => writeln!(w, "0x{f:x}")?,
        None => writeln!(w, "none")?,
    }
    write!(w, "[crashwalk] ttbr0=0x{:016x} asid={} root=", d.ttbr0, d.asid())?;
    match d.root {
        RootOwner::Slot(s) => write!(w, "slot {s}")?,
        RootOwner::Empty => write!(w, "empty")?,
        RootOwner::Unknown => write!(w, "unknown")?,
    }
    writeln!(
        w,
        "; slot {} expects 0x{:x} records 0x{:x} -> {}",
        d.slot,
        d.expected,
        d.recorded,
        if d.roots_same() { "same" } else { "DIFFERENT" },
    )?;
    for p in d.probes.iter().flatten() {
        let label = p.hw.label;
        serial_walk(w, label, &p.walk)?;
        if let Some(o) = &p.other {
            serial_walk(w, label, o)?;
        }
        serial_par(w, &p.hw)?;
        serial_view(w, label, d.slot, &p.view)?;
        let s = summary(p);
        writeln!(
            w,
            "[crashwalk] {} summary: model={} table={} at={} table==model {}",
            label, s.model, s.table, s.at, s.same,
        )?;
    }
    Ok(())
}

/// `L0[i]=desc(kind) … -> outcome` and the region's sharing line.
fn serial_walk<W: Write>(w: &mut W, label: &str, walk: &Walk) -> fmt::Result {
    write!(w, "[crashwalk] {} va=0x{:x} walk slot {} root 0x{:x}:", label, walk.va, walk.slot, walk.root_pa)?;
    for (level, step) in walk.steps.iter().enumerate() {
        if let Some(s) = step {
            write!(w, " L{}[{}]=0x{:016x}(", level, s.index, s.desc)?;
            write_kind(w, s.kind)?;
            write!(w, ")")?;
        }
    }
    match walk.outcome {
        Outcome::BadSlot => writeln!(w, " -> bad slot")?,
        Outcome::OutOfRange => writeln!(w, " -> above the user tables")?,
        Outcome::Leaf { level, oa } => writeln!(w, " -> mapped L{level} oa=0x{oa:x}")?,
        Outcome::Invalid { level } => writeln!(w, " -> INVALID at L{level}")?,
        Outcome::NotTable { level } => writeln!(w, " -> no table at L{level}")?,
        Outcome::BadTable { level, pa } => writeln!(w, " -> BAD TABLE 0x{pa:x} for L{level}, not read")?,
    }
    if let Some(t) = walk.l2_template {
        writeln!(
            w,
            "[crashwalk] {} l2 template=0x{:016x} l3 also named by slots=0x{:x} template={}",
            label,
            t,
            walk.l3_also_slots,
            yes_no(walk.l3_also_template),
        )?;
    }
    Ok(())
}

/// The `AT S1E0R` result.
fn serial_par<W: Write>(w: &mut W, p: &HwProbe) -> fmt::Result {
    write!(w, "[crashwalk] {} AT S1E0R par=0x{:016x}: ", p.label, p.par)?;
    if p.par & PAR_F == 0 {
        return writeln!(w, "ok pa=0x{:x}", par_pa(p.par, p.va));
    }
    let f = par_fault(p.par);
    writeln!(w, "fault fst=0x{:02x} ({} L{}) s={} ptw={}", f.fst, f.what, f.level, f.s, f.ptw)
}

/// The model's view line(s).
fn serial_view<W: Write>(w: &mut W, label: &str, slot: usize, v: &PageView) -> fmt::Result {
    if !v.enabled {
        return writeln!(w, "[crashwalk] {label} model slot {slot}: memprot off");
    }
    write!(w, "[crashwalk] {} model slot {}: pool={} ", label, slot, yes_no(v.in_pool))?;
    match v.block {
        Some(b) => {
            write!(w, "block 0x{:x}-0x{:x} mode=", b.start, b.end)?;
            match b.mode {
                Some(m) => write!(w, "{m:?}")?,
                None => write!(w, "none")?,
            }
            write!(w, " own=0x{:x} further=", b.own)?;
            for (i, f) in b.further.iter().enumerate() {
                write!(w, "{}0x{:x}", if i == 0 { "" } else { "," }, f)?;
            }
            write!(w, " owners=0x{:x} inherit=0x{:x}", b.owners, b.inherit)?;
        }
        None => write!(w, "block none")?,
    }
    writeln!(w, "{}", if v.lists_ok { "" } else { " (pool list walk capped)" })?;
    write!(
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
    )?;
    serial_opt(w, v.expected)?;
    write!(w, " shared=")?;
    serial_opt(w, v.expected_shared)?;
    write!(w, " private needed=")?;
    match v.needs_private {
        Some(n) => write!(w, "{}", yes_no(n))?,
        None => write!(w, "unknown")?,
    }
    writeln!(w, " has={} shortfalls={} pending={}", yes_no(v.has_private), v.shortfalls, yes_no(v.pending))
}

fn serial_opt<W: Write>(w: &mut W, v: Option<u64>) -> fmt::Result {
    match v {
        Some(d) => write!(w, "0x{d:016x}"),
        None => write!(w, "n/a"),
    }
}

// ─── Screen block ───────────────────────────────────────────────────────

/// The compact block for the on-screen crash report (module docs), after
/// the report text and before the key prompt; it starts with a blank line.
/// Lines end in `\n` (the console writer adds the `\r`).
pub fn write_screen<W: Write>(w: &mut W, d: &CrashDiag) -> fmt::Result {
    write!(
        w,
        "\ncrash-walk slot {} {}: ttbr0 {:x} asid {} = ",
        d.slot,
        d.source(),
        d.root_table(),
        d.asid(),
    )?;
    match d.root {
        RootOwner::Slot(s) => write!(w, "slot {s}")?,
        RootOwner::Empty => write!(w, "empty")?,
        RootOwner::Unknown => write!(w, "unknown")?,
    }
    writeln!(
        w,
        ", exp {:x} rec {:x} {}",
        d.expected,
        d.recorded,
        if d.roots_same() { "same" } else { "DIFFERENT" },
    )?;
    for p in d.probes.iter().flatten() {
        let label = p.hw.label;
        screen_walk(w, label, &p.walk)?;
        if let Some(o) = &p.other {
            screen_walk(w, label, o)?;
        }
        write!(w, "  AT S1E0R par={:x}: ", p.hw.par)?;
        if p.hw.par & PAR_F == 0 {
            writeln!(w, "ok pa={:x}", par_pa(p.hw.par, p.hw.va))?;
        } else {
            let f = par_fault(p.hw.par);
            writeln!(w, "FAULT {} L{} fst={:02x} s={} ptw={}", f.what, f.level, f.fst, f.s, f.ptw)?;
        }
        screen_view(w, &p.view)?;
        let s = summary(p);
        writeln!(w, "  => model={} table={} at={} table==model {}", s.model, s.table, s.at, s.same)?;
    }
    Ok(())
}

/// Two lines: `<label> <va> s<slot>:` with L0–L2, then L3 (if read), where
/// the walk ended and who else names the region's L3 (`also` = the other
/// slots, a mask; `tpl` = the shared template).
fn screen_walk<W: Write>(w: &mut W, label: &str, walk: &Walk) -> fmt::Result {
    write!(w, "{} {:x} s{}:", label, walk.va, walk.slot)?;
    for (level, step) in walk.steps.iter().enumerate().take(3) {
        if let Some(s) = step {
            write!(w, "{}L{}[{}]={:x} ", if level == 0 { " " } else { "  " }, level, s.index, s.desc)?;
            write_kind(w, s.kind)?;
        }
    }
    w.write_str("\n  ")?;
    if let Some(s) = walk.steps[3] {
        write!(w, "L3[{}]={:x} ", s.index, s.desc)?;
        write_kind(w, s.kind)?;
        w.write_str(" ")?;
    }
    match walk.outcome {
        Outcome::BadSlot => w.write_str("-> bad slot")?,
        Outcome::OutOfRange => w.write_str("-> above the user tables")?,
        Outcome::Leaf { level, oa } => write!(w, "-> mapped L{level} oa={oa:x}")?,
        Outcome::Invalid { level } => write!(w, "-> INVALID at L{level}")?,
        Outcome::NotTable { level } => write!(w, "-> no table at L{level}")?,
        Outcome::BadTable { level, pa } => write!(w, "-> BAD TABLE {pa:x} for L{level}, not read")?,
    }
    if walk.l2_template.is_some() {
        write!(w, "  also {:x} tpl {}", walk.l3_also_slots, yes_no(walk.l3_also_template))?;
    }
    w.write_str("\n")
}

/// The model in three lines (one with memory protection off).
fn screen_view<W: Write>(w: &mut W, v: &PageView) -> fmt::Result {
    if !v.enabled {
        return w.write_str("  model: memprot off\n");
    }
    write!(w, "  model: pool {}", yes_no(v.in_pool))?;
    match v.block {
        Some(b) => {
            write!(w, " block {:x}-{:x} mode ", b.start, b.end)?;
            match b.mode {
                Some(m) => write!(w, "{m:?}")?,
                None => w.write_str("none")?,
            }
            write!(w, " own {:x}", b.own)?;
            let further = b.further.iter().filter(|&&f| f != 0).count();
            if further > 0 {
                write!(w, " +{further} further")?;
            }
            write!(w, " owners {:x} inh {:x}", b.owners, b.inherit)?;
        }
        None => w.write_str(", no block")?,
    }
    if !v.lists_ok {
        w.write_str(" (list walk capped)")?;
    }
    write!(w, "\n  model: holds {} expect ", yes_no(v.holds))?;
    screen_opt(w, v.expected)?;
    w.write_str(" shared ")?;
    screen_opt(w, v.expected_shared)?;
    writeln!(w, " pend {}", yes_no(v.pending))?;
    w.write_str("  model: priv need ")?;
    match v.needs_private {
        Some(n) => w.write_str(yes_no(n))?,
        None => w.write_str("?")?,
    }
    writeln!(
        w,
        " has {} short {} live {:x} group {:x} temp {:x} space {:x}",
        yes_no(v.has_private),
        v.shortfalls,
        v.live,
        v.group,
        v.temp,
        v.space_root,
    )
}

fn screen_opt<W: Write>(w: &mut W, v: Option<u64>) -> fmt::Result {
    match v {
        Some(d) => write!(w, "{d:x}"),
        None => w.write_str("n/a"),
    }
}
