//! Program crash report — EmuTOS `dopanic` for AArch64 exceptions
//! (ARCH03 stage 0c).
//!
//! Spec: `docs/api/bios/_exception-crash.md`. When a program takes a CPU
//! exception the OS does not handle (on m68k: vectors 2–63 still pointing at
//! the default `any_vec`, `references/emutos/bios/vectors.S`), EmuTOS prints a
//! register dump on the console and in the debug log (`kcprintf`,
//! `bios/kprint.c:302-573`), waits for a key and terminates the program with
//! `Pterm(-1)` (`kprint.c:575-594`, `bios/panicasm.S:96-99`).
//!
//! This module holds the parts that do not touch hardware, so they are host
//! testable: [`classify`] maps an `ESR_EL1` syndrome onto the m68k exception
//! EmuTOS would have reported (`exc_messages`, `kprint.c:283-300`), and
//! [`write_report`] renders the report text. The kernel's synchronous
//! handler supplies the register state, prints, waits and terminates.

use core::fmt;

/// `ESR_ELx.EC` values this module distinguishes (ARM ARM DDI 0487 M.d,
/// D24-8643).
mod ec {
    /// Unknown reason — an undefined/unallocated instruction.
    pub const UNKNOWN: u8 = 0x00;
    /// Trapped `WFI`/`WFE` (`SCTLR_EL1.nTWI/nTWE = 0`).
    pub const WFX: u8 = 0x01;
    /// Illegal Execution state (`PSTATE.IL`).
    pub const ILLEGAL_STATE: u8 = 0x0E;
    /// Trapped `MSR`/`MRS`/system instruction.
    pub const SYSREG: u8 = 0x18;
    /// Instruction Abort from a lower EL.
    pub const IABT_LOWER: u8 = 0x20;
    /// Instruction Abort from the same EL.
    pub const IABT_SAME: u8 = 0x21;
    /// PC alignment fault.
    pub const PC_ALIGN: u8 = 0x22;
    /// Data Abort from a lower EL.
    pub const DABT_LOWER: u8 = 0x24;
    /// Data Abort from the same EL.
    pub const DABT_SAME: u8 = 0x25;
    /// SP alignment fault.
    pub const SP_ALIGN: u8 = 0x26;
}

/// The `DFSC`/`IFSC` value of an Alignment fault (ARM ARM D24-8684).
const FSC_ALIGNMENT: u8 = 0x21;

/// The exception class field `ESR_ELx.EC` (bits 31:26).
#[inline]
pub const fn esr_ec(esr: u64) -> u8 {
    ((esr >> 26) & 0x3F) as u8
}

/// `true` for an Instruction or Data Abort, from either exception level.
#[inline]
pub const fn is_abort(esr: u64) -> bool {
    matches!(
        esr_ec(esr),
        ec::IABT_LOWER | ec::IABT_SAME | ec::DABT_LOWER | ec::DABT_SAME
    )
}

/// The fault status code (`DFSC`/`IFSC`, `ISS[5:0]`) of an abort, `None` for
/// any other exception class.
#[inline]
pub const fn fault_status(esr: u64) -> Option<u8> {
    if is_abort(esr) { Some((esr & 0x3F) as u8) } else { None }
}

/// Whether `FAR_EL1` holds the faulting address for this syndrome: every
/// abort except a synchronous External abort with `FnV` (`ISS[10]`) set, and
/// a PC alignment fault (FAR = the misaligned PC) — ARM ARM D24-8681,
/// D1-7219.
#[inline]
pub const fn far_valid(esr: u64) -> bool {
    if is_abort(esr) {
        esr & (1 << 10) == 0
    } else {
        esr_ec(esr) == ec::PC_ALIGN
    }
}

/// The exception EmuTOS would have reported — `exc_messages[]`
/// (`bios/kprint.c:283-300`) — for an AArch64 synchronous exception.
///
/// The mapping follows what raised the m68k vector, not the AArch64 name:
/// an MMU/bus fault on a data or instruction access is the 68k **bus error**
/// (vector 2), a misaligned access/PC/SP the **address error** (vector 3,
/// odd address), an undefined instruction the **illegal instruction**
/// (vector 4), and an instruction that user mode may not execute
/// (`MSR`/`MRS`/cache maintenance/`WFI` trapped by `SCTLR_EL1`) the
/// **privilege violation** (vector 8). Anything else has no 68k counterpart
/// and is reported by its exception class, the way EmuTOS reports a vector
/// outside its table ("Exception number %d", `kprint.c:494-502`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashKind {
    /// 68k vector 2.
    BusError,
    /// 68k vector 3.
    AddressError,
    /// 68k vector 4.
    IllegalInstruction,
    /// 68k vector 8.
    PrivilegeViolation,
    /// No 68k counterpart; carries `ESR_EL1.EC`.
    Other(u8),
}

impl CrashKind {
    /// The m68k exception vector number (`proc_enum`) this kind stands for,
    /// `None` for [`CrashKind::Other`].
    pub const fn vector(self) -> Option<u8> {
        match self {
            CrashKind::BusError => Some(2),
            CrashKind::AddressError => Some(3),
            CrashKind::IllegalInstruction => Some(4),
            CrashKind::PrivilegeViolation => Some(8),
            CrashKind::Other(_) => None,
        }
    }

    /// The EmuTOS message text (`exc_messages[vector]`).
    pub const fn message(self) -> Option<&'static str> {
        match self {
            CrashKind::BusError => Some("Bus Error"),
            CrashKind::AddressError => Some("Address Error"),
            CrashKind::IllegalInstruction => Some("Illegal Instruction"),
            CrashKind::PrivilegeViolation => Some("Privilege Violation"),
            CrashKind::Other(_) => None,
        }
    }
}

/// Classify a synchronous exception syndrome — see [`CrashKind`].
pub const fn classify(esr: u64) -> CrashKind {
    match esr_ec(esr) {
        ec::IABT_LOWER | ec::IABT_SAME | ec::DABT_LOWER | ec::DABT_SAME => {
            if (esr & 0x3F) as u8 == FSC_ALIGNMENT {
                CrashKind::AddressError
            } else {
                CrashKind::BusError
            }
        }
        ec::PC_ALIGN | ec::SP_ALIGN => CrashKind::AddressError,
        ec::UNKNOWN | ec::ILLEGAL_STATE => CrashKind::IllegalInstruction,
        ec::WFX | ec::SYSREG => CrashKind::PrivilegeViolation,
        other => CrashKind::Other(other),
    }
}

/// The program that was running, from its basepage — EmuTOS prints these
/// from `run` (`kprint.c:565-573`). Addresses are the program's own (user)
/// addresses, as `LONG`s like the basepage stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramInfo {
    /// User address of the basepage.
    pub basepage: u32,
    /// `p_tbase`.
    pub tbase: u32,
    /// `p_tlen`.
    pub tlen: u32,
    /// `p_dbase`.
    pub dbase: u32,
    /// `p_bbase`.
    pub bbase: u32,
}

/// What the memory a faulting access hit was, with memory protection on —
/// FreeMiNT's `berr_msg` classes (`sys/arch/mprot.x:71-80,99-111`): the mode
/// of the block (a global block cannot cause a protection fault, so a fault
/// there is a "hardware" one), free pool memory, or memory that protection
/// does not control at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// `PROT_P` — private memory of another process (or freed).
    Private,
    /// `PROT_S` — supervisor (kernel) memory.
    Super,
    /// `PROT_PR` — readable by everybody, writable by its owner.
    Readable,
    /// Free pool memory, or a `PROT_I` block (FreeMiNT reports both as
    /// "free": `berr_msg[PROT_I]`).
    Free,
    /// Not controlled by memory protection (outside the pools, a global
    /// block).
    Hardware,
    /// Page 0, the classic system variables (MEM12): EL0 read-only outside
    /// the super state, so a write there in user state is the Atari's bus
    /// error (`docs/api/_super-state.md` S9).
    SystemVariables,
    /// The shared stub page (MEM12): EL0 execute-only, no data access.
    StubPage,
    /// The call target of a `Bconmap` `MAPTAB` function slot (XBIOS05): a
    /// program called through one of the table's function pointers, which
    /// are not callable on NewTOS (`docs/api/xbios/2c-bconmap.md`,
    /// `docs/COMPATIBILITY.md`).
    BconmapVector,
}

impl RegionKind {
    /// FreeMiNT's word for it (`berr_msg`, without the padding).
    pub const fn name(self) -> &'static str {
        match self {
            RegionKind::Private => "private",
            RegionKind::Super => "super",
            RegionKind::Readable => "readable",
            RegionKind::Free => "free",
            RegionKind::Hardware => "hardware",
            RegionKind::SystemVariables => "page-0 system-variable",
            RegionKind::StubPage => "execute-only stub",
            RegionKind::BconmapVector => "Bconmap MAPTAB function (not callable)",
        }
    }
}

/// The MMU's reason for a memory abort (ARM ARM D24-8672/8673 IFSC,
/// D24-8683…8685 DFSC: `0b0001LL` translation, `0b0010LL` access flag,
/// `0b0011LL` permission, LL = level).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    /// No valid translation — memory protection makes another process's
    /// `PROT_P`/`PROT_S` pages and freed pages invalid, so this is what a
    /// protection violation of those looks like (plan §3.3, 0a).
    Translation,
    /// Access flag (never set clear by NewTOS — a kernel bug).
    AccessFlag,
    /// The translation forbids the access — a write to `PROT_PR` memory of
    /// another process.
    Permission,
}

/// The kind of access that faulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// A data read.
    Read,
    /// A data write (`ESR_EL1.WnR`, D24-8682).
    Write,
    /// An instruction fetch.
    Execute,
}

/// The memory-protection line of the report: what was hit and why the MMU
/// refused (ARCH03 stage 3b, plan §3.15 / Frage 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemViolation {
    /// What the address was (from the protection bookkeeping, not the MMU:
    /// a translation fault alone cannot tell "protected" from "unmapped").
    pub region: RegionKind,
    /// The MMU's reason.
    pub fault: FaultType,
    /// The table level the fault was reported at (0–3).
    pub level: u8,
    /// Read, write or instruction fetch.
    pub access: Access,
}

/// The protection line for an abort syndrome `esr` whose address lies in
/// `region` — `None` unless `esr` is a translation, access-flag or
/// permission fault of an Instruction/Data Abort and `region` is known
/// (protection on).
pub const fn mem_violation(esr: u64, region: Option<RegionKind>) -> Option<MemViolation> {
    let Some(region) = region else {
        return None;
    };
    if !is_abort(esr) {
        return None;
    }
    let fsc = (esr & 0x3F) as u8;
    let fault = match fsc >> 2 {
        0b0001 => FaultType::Translation,
        0b0010 => FaultType::AccessFlag,
        0b0011 => FaultType::Permission,
        _ => return None,
    };
    let access = match esr_ec(esr) {
        ec::IABT_LOWER | ec::IABT_SAME => Access::Execute,
        _ => {
            if esr & (1 << 6) != 0 {
                Access::Write
            } else {
                Access::Read
            }
        }
    };
    Some(MemViolation { region, fault, level: fsc & 3, access })
}

/// Everything the report shows.
#[derive(Debug, Clone, Copy)]
pub struct CrashReport<'a> {
    /// The classified exception.
    pub kind: CrashKind,
    /// `ESR_EL1` (EmuTOS shows the 68030 frame's format/status words here).
    pub esr: u64,
    /// The fault address, when the syndrome provides one.
    pub far: Option<u64>,
    /// The program counter at the exception (`ELR_EL1`).
    pub pc: u64,
    /// The saved processor state (`SPSR_EL1`, EmuTOS: `sr`).
    pub pstate: u64,
    /// `x0..x30` at the exception (EmuTOS: `D0-D7`, `A0-A7`).
    pub regs: &'a [u64; 31],
    /// The program's stack pointer `SP_EL0` (EmuTOS: `USP`).
    pub usp: u64,
    /// The running program, if there is one.
    pub program: Option<ProgramInfo>,
    /// With memory protection on: what the faulting access hit
    /// ([`mem_violation`]).
    pub violation: Option<MemViolation>,
}

/// Render the report text — the `kcprintf` lines of `dopanic`
/// (`kprint.c:302-573`), up to but not including the
/// "Press any key to continue" prompt, which the caller prints itself (it
/// goes to the screen only, `kprint.c:581-585`). Lines end in `\n`; the
/// console writer adds the `\r` (EmuTOS `cprintf_outc`, `kprint.c:53-60`).
pub fn write_report<W: fmt::Write>(w: &mut W, r: &CrashReport<'_>) -> fmt::Result {
    // `cprintf("\033f\033v\n\n")` (`kprint.c:312`): hide the cursor, turn
    // line wrap on, two blank lines. `dopanic` also clears `M_CEOL` around
    // the register dump and restores it after (`kprint.c:525-526,562-563`);
    // the dump lines here are 75 columns wide and never reach the last
    // column of the console (80 columns at 640×480, more on larger
    // screens), so that bracket has nothing to change.
    w.write_str("\x1bf\x1bv\n\n")?;
    match r.kind.message() {
        Some(m) => writeln!(w, "Panic: {m}")?,
        None => writeln!(w, "Panic: Exception class 0x{:02x}", esr_ec(r.esr))?,
    }
    // The 68030 bus/address error frame line ("fw=… ir=… ssr=…",
    // `kprint.c:432-435`) — the AArch64 syndrome register stands in for it.
    writeln!(w, "esr={:08x}", r.esr as u32)?;
    match r.far {
        Some(a) => writeln!(w, "addr={:016x} sr={:08x} pc={:016x}", a, r.pstate as u32, r.pc)?,
        None => writeln!(w, "sr={:08x} pc={:016x}", r.pstate as u32, r.pc)?,
    }
    // Memory protection (ARCH03 3b): FreeMiNT's classification of the
    // address (`report_buserr`, `sys/arch/mprot.x:89-124`: "MEMORY
    // VIOLATION", type, r/w) plus the MMU's reason, which tells a
    // protected-and-invalid page (translation) from a read-only one
    // (permission).
    if let Some(v) = r.violation {
        let access = match v.access {
            Access::Read => "read",
            Access::Write => "write",
            Access::Execute => "execute",
        };
        let fault = match v.fault {
            FaultType::Translation => "translation",
            FaultType::AccessFlag => "access flag",
            FaultType::Permission => "permission",
        };
        writeln!(
            w,
            "Memory violation: {} memory, {} ({} fault, level {})",
            v.region.name(),
            access,
            fault,
            v.level
        )?;
    }
    // Register dump (`kprint.c:527-537`): four per line, like "D0-3:".
    w.write_str("\n")?;
    let mut i = 0usize;
    while i < 31 {
        let last = core::cmp::min(i + 3, 30);
        let mut label = [0u8; 8];
        let lab = fmt_label(&mut label, i, last);
        write!(w, "{lab:<8}")?;
        for (k, v) in r.regs[i..=last].iter().enumerate() {
            if k > 0 {
                w.write_str(" ")?;
            }
            write!(w, "{v:016x}")?;
        }
        w.write_str("\n")?;
        i += 4;
    }
    writeln!(w, "{:<8}{:016x}", " USP:", r.usp)?;
    w.write_str("\n")?;
    if let Some(p) = r.program {
        writeln!(w, "basepage={:08x}", p.basepage)?;
        writeln!(w, "text={:08x} data={:08x} bss={:08x}", p.tbase, p.dbase, p.bbase)?;
        let tbase = u64::from(p.tbase);
        if r.pc >= tbase && r.pc < tbase + u64::from(p.tlen) {
            writeln!(w, "Crash at text+{:08x}", r.pc - tbase)?;
        }
    }
    Ok(())
}

/// "X<first>-<last>:" into `buf` (at most 7 bytes: "X28-30:").
fn fmt_label(buf: &mut [u8; 8], first: usize, last: usize) -> &str {
    let mut n = 0;
    let mut put = |b: u8, n: &mut usize| {
        buf[*n] = b;
        *n += 1;
    };
    put(b'X', &mut n);
    for v in [first, last] {
        if v >= 10 {
            put(b'0' + (v / 10) as u8, &mut n);
        }
        put(b'0' + (v % 10) as u8, &mut n);
        put(if v == first { b'-' } else { b':' }, &mut n);
    }
    core::str::from_utf8(&buf[..n]).unwrap_or("X:")
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::String;

    /// Build an ESR from EC + ISS (IL = 1, as for every A64 instruction).
    const fn esr(ec: u8, iss: u32) -> u64 {
        ((ec as u64) << 26) | (1 << 25) | iss as u64
    }

    #[test]
    fn aborts_classify_as_bus_error_except_alignment() {
        // 0b measurements (S0b §5): translation L0..L3, permission L2/L3.
        for fsc in [0x04u32, 0x05, 0x06, 0x07, 0x0E, 0x0F, 0x0B, 0x10] {
            assert_eq!(classify(esr(0x24, fsc)), CrashKind::BusError, "DABT fsc {fsc:#x}");
            assert_eq!(classify(esr(0x20, fsc)), CrashKind::BusError, "IABT fsc {fsc:#x}");
            assert_eq!(classify(esr(0x25, fsc | 0x40)), CrashKind::BusError, "same-EL write");
        }
        assert_eq!(classify(esr(0x24, 0x21)), CrashKind::AddressError);
        assert_eq!(classify(esr(0x25, 0x21)), CrashKind::AddressError);
    }

    #[test]
    fn other_classes_map_onto_68k_vectors() {
        assert_eq!(classify(esr(0x00, 0)), CrashKind::IllegalInstruction);
        assert_eq!(classify(esr(0x0E, 0)), CrashKind::IllegalInstruction);
        assert_eq!(classify(esr(0x22, 0)), CrashKind::AddressError);
        assert_eq!(classify(esr(0x26, 0)), CrashKind::AddressError);
        assert_eq!(classify(esr(0x18, 0)), CrashKind::PrivilegeViolation);
        assert_eq!(classify(esr(0x01, 0)), CrashKind::PrivilegeViolation);
        assert_eq!(classify(esr(0x3C, 0)), CrashKind::Other(0x3C));
        assert_eq!(CrashKind::BusError.vector(), Some(2));
        assert_eq!(CrashKind::AddressError.vector(), Some(3));
        assert_eq!(CrashKind::IllegalInstruction.vector(), Some(4));
        assert_eq!(CrashKind::PrivilegeViolation.vector(), Some(8));
        assert_eq!(CrashKind::Other(0x3C).message(), None);
    }

    #[test]
    fn syndrome_fields() {
        // sttr on an EL0-RO page, 0b §5: ESR ISS 0x4F (WnR + permission L3).
        let e = esr(0x25, 0x4F);
        assert!(is_abort(e));
        assert_eq!(fault_status(e), Some(0x0F));
        assert!(far_valid(e));
        // External abort with FnV: FAR not valid.
        assert!(!far_valid(esr(0x24, 0x10 | (1 << 10))));
        assert!(far_valid(esr(0x22, 0)));
        assert!(!far_valid(esr(0x00, 0)));
        assert_eq!(fault_status(esr(0x00, 0)), None);
    }

    fn render(r: &CrashReport<'_>) -> String {
        let mut s = String::new();
        write_report(&mut s, r).unwrap();
        s
    }

    #[test]
    fn report_has_emutos_shape() {
        let mut regs = [0u64; 31];
        for (i, r) in regs.iter_mut().enumerate() {
            *r = i as u64;
        }
        let r = CrashReport {
            kind: CrashKind::BusError,
            esr: esr(0x24, 0x46),
            far: Some(0xDEAD_0000),
            pc: 0x1234,
            pstate: 0,
            regs: &regs,
            usp: 0x3F_FF00,
            program: Some(ProgramInfo {
                basepage: 0,
                tbase: 0x1000,
                tlen: 0x800,
                dbase: 0x2000,
                bbase: 0x2100,
            }),
            violation: None,
        };
        let s = render(&r);
        let lines: std::vec::Vec<&str> = s.lines().collect();
        // `cprintf("\033f\033v\n\n")` (`kprint.c:312`): cursor off, wrap on.
        assert_eq!(lines[0], "\x1bf\x1bv");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "Panic: Bus Error");
        assert_eq!(lines[3], "esr=92000046");
        assert_eq!(lines[4], "addr=00000000dead0000 sr=00000000 pc=0000000000001234");
        assert_eq!(lines[5], "");
        assert_eq!(
            lines[6],
            "X0-3:   0000000000000000 0000000000000001 0000000000000002 0000000000000003"
        );
        assert!(lines[8].starts_with("X8-11:  0"), "{}", lines[8]);
        assert_eq!(
            lines[13],
            "X28-30: 000000000000001c 000000000000001d 000000000000001e"
        );
        assert_eq!(lines[14], " USP:   00000000003fff00");
        assert_eq!(lines[15], "");
        assert_eq!(lines[16], "basepage=00000000");
        assert_eq!(lines[17], "text=00001000 data=00002000 bss=00002100");
        assert_eq!(lines[18], "Crash at text+00000234");
        assert_eq!(lines.len(), 19);
        // Every line fits an 80-column console.
        assert!(lines.iter().all(|l| l.len() <= 80));
    }

    #[test]
    fn protection_line_tells_translation_from_permission() {
        // EL0 write to an invalid L3 entry (another process's PROT_P page).
        let v = mem_violation(esr(0x24, 0x47), Some(RegionKind::Private)).unwrap();
        assert_eq!(v, MemViolation {
            region: RegionKind::Private,
            fault: FaultType::Translation,
            level: 3,
            access: Access::Write,
        });
        // EL0 write to a read-only L3 entry (PROT_PR of another process).
        let v = mem_violation(esr(0x24, 0x4F), Some(RegionKind::Readable)).unwrap();
        assert_eq!((v.fault, v.level, v.access), (FaultType::Permission, 3, Access::Write));
        // A read; the kernel's ldtr on behalf of a caller (same EL).
        let v = mem_violation(esr(0x25, 0x07), Some(RegionKind::Free)).unwrap();
        assert_eq!((v.fault, v.access), (FaultType::Translation, Access::Read));
        // An instruction fetch.
        let v = mem_violation(esr(0x20, 0x07), Some(RegionKind::Private)).unwrap();
        assert_eq!(v.access, Access::Execute);
        // Protection off: no line. Alignment / external: no line.
        assert_eq!(mem_violation(esr(0x24, 0x47), None), None);
        assert_eq!(mem_violation(esr(0x24, 0x21), Some(RegionKind::Hardware)), None);
        assert_eq!(mem_violation(esr(0x24, 0x10), Some(RegionKind::Hardware)), None);
        assert_eq!(mem_violation(esr(0x00, 0), Some(RegionKind::Hardware)), None);
    }

    #[test]
    fn report_shows_the_protection_line_after_the_address() {
        let regs = [0u64; 31];
        let r = CrashReport {
            kind: CrashKind::BusError,
            esr: esr(0x24, 0x47),
            far: Some(0x0100_2000),
            pc: 0x0100_5000,
            pstate: 0,
            regs: &regs,
            usp: 0,
            program: None,
            violation: mem_violation(esr(0x24, 0x47), Some(RegionKind::Private)),
        };
        let s = render(&r);
        let lines: std::vec::Vec<&str> = s.lines().collect();
        assert_eq!(lines[4], "addr=0000000001002000 sr=00000000 pc=0000000001005000");
        assert_eq!(lines[5], "Memory violation: private memory, write (translation fault, level 3)");
        assert_eq!(lines[6], "");
    }

    #[test]
    fn report_without_address_or_program_or_text_hit() {
        let regs = [0u64; 31];
        let r = CrashReport {
            kind: CrashKind::Other(0x3C),
            esr: esr(0x3C, 0),
            far: None,
            pc: 0xFFFF_0000_0008_0000,
            pstate: 0x3C5,
            regs: &regs,
            usp: 0,
            program: Some(ProgramInfo { basepage: 0, tbase: 0x1000, tlen: 0x10, dbase: 0, bbase: 0 }),
            violation: None,
        };
        let s = render(&r);
        assert!(s.contains("Panic: Exception class 0x3c\n"));
        assert!(s.contains("\nsr=000003c5 pc=ffff000000080000\n"));
        assert!(!s.contains("addr="));
        // A kernel PC is not inside the program's text: no "Crash at" line.
        assert!(!s.contains("Crash at"));
        let r2 = CrashReport { program: None, ..r };
        assert!(!render(&r2).contains("basepage="));
    }
}
