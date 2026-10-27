//! Fault-safe kernel access to **user** memory (ARCH03 stage 0c).
//!
//! A TOS syscall argument that is a pointer is a pointer into the *caller's*
//! address space. The kernel reads and writes such memory with the
//! unprivileged load/store instructions `ldtr*`/`sttr*`: executed at EL1 with
//! `PSTATE.UAO = 0` (the Cortex-A72 has no UAO) they are permission-checked
//! "as if executed at EL0" (ARM ARM DDI 0487 M.d C6.2.245; plan
//! `docs/analysis/arch03-implementation-plan.md` Frage 10, 0a/0b notes), so a
//! kernel pointer (`AP = 00`), an address outside the caller's `TTBR0` window
//! and an unmapped user address all fault.
//!
//! What this module adds over the bare instructions is **recovery**. Every
//! `ldtr`/`sttr` below sits in a hand-written leaf routine and is listed in a
//! fixup table `(instruction PC, fixup PC)`. The kernel's synchronous handler
//! consults [`fixup_for`] on a same-EL data abort (`EC 0x25`); if the faulting
//! `ELR_EL1` is one of these instructions it records the fault
//! ([`note_fault`]) and resumes at the fixup, which returns a failure status
//! to the Rust wrapper — instead of halting the machine. The 0b spike measured
//! exactly this picture on QEMU (`docs/analysis/arch03-stage0b-spike.md` §5:
//! `ELR` = the `ldtr`/`sttr` itself, `FAR` = the user VA, DFSC per level).
//! A same-EL abort at any **other** PC is still a kernel fault and halts.
//!
//! ## What a fault means for the caller (EmuTOS behaviour)
//!
//! TOS has no `EFAULT`. On EmuTOS a GEMDOS call that dereferences a bad
//! pointer takes a bus/address error **in supervisor mode**; the default
//! vector (`references/emutos/bios/vectors.S` `any_vec`) runs `dopanic`
//! (`bios/kprint.c:302-596`), which prints the crash report and calls
//! `kill_program` → `Pterm(-1)` (`bios/panicasm.S:96-99`) — and since `run` is
//! still the caller's PD, it is the **calling program** that is terminated
//! (`kprint.c:587-594`). FreeMiNT does the same in substance: a kernel bus
//! error on a user address raises `SIGBUS` on `curproc`
//! (`references/freemint/sys/arch/sig_mach.c:493-502`), whose default action
//! kills it. The recorded fault is therefore not turned into an error code
//! for the program: the kernel's syscall tail picks it up with
//! [`take_pending_fault`] and terminates the caller with the crash report
//! (`docs/api/bios/_exception-crash.md`). The `Err` the wrappers return only
//! lets the Rust call site stop cleanly; its value never reaches the program.
//!
//! On the host (unit tests) there is no EL0/EL1 split and nothing can fault
//! recoverably, so the wrappers use ordinary loads/stores there and the
//! fixup table is empty.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// A user-memory access faulted (the address was not accessible to the
/// calling program). The details are in the record [`note_fault`] stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserFault;

/// What a call returns after a user-access fault stopped it. The value never
/// reaches the program: the syscall tail sees the recorded fault and ends the
/// caller instead (module comment). `EINTRN` only so that a Rust caller that
/// propagates it treats it as a failure.
pub const FAULTED: tos_types::Long = tos_types::error::EINTRN;

/// One fixup-table entry: a faulting instruction and where to resume.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixupEntry {
    /// Address of an `ldtr*`/`sttr*` instruction that may fault.
    pub insn: u64,
    /// Address execution resumes at when that instruction faults.
    pub fixup: u64,
}

/// Look `pc` up in `table`: the fixup address if `pc` is exactly one of the
/// listed instructions. Exact match only — `ELR_EL1` of a synchronous data
/// abort is the faulting instruction itself (ARM ARM D1, measured in 0b §5),
/// so a PC one instruction off is a different (kernel) fault.
pub fn lookup_fixup(table: &[FixupEntry], pc: u64) -> Option<u64> {
    table.iter().find(|e| e.insn == pc).map(|e| e.fixup)
}

/// The fixup for a same-EL data abort at `pc`, if `pc` is one of this
/// module's user-access instructions.
pub fn fixup_for(pc: u64) -> Option<u64> {
    lookup_fixup(fixup_table(), pc)
}

// ---------------------------------------------------------------------------
// Fault record — written by the kernel's abort handler, consumed by the
// syscall tail.
// ---------------------------------------------------------------------------

/// Snapshot of a recovered user-access fault: what EmuTOS saves at `0x380`
/// for `dopanic` (`bios/vectors.S` `any_vec`) — the registers at the fault,
/// the fault address and the PC — in AArch64 terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultRecord {
    /// `FAR_EL1` — the user address the kernel tried to access.
    pub far: u64,
    /// `ESR_EL1` of the abort (`EC 0x25`, DFSC, `WnR`).
    pub esr: u64,
    /// `ELR_EL1` — the faulting `ldtr`/`sttr` (a kernel address).
    pub pc: u64,
    /// `SPSR_EL1` of the abort (the EL1 state the access ran in).
    pub pstate: u64,
    /// `x0..x30` at the fault.
    pub regs: [u64; 31],
}

static FAULT_ARMED: AtomicBool = AtomicBool::new(false);
static FAULT_FAR: AtomicU64 = AtomicU64::new(0);
static FAULT_ESR: AtomicU64 = AtomicU64::new(0);
static FAULT_PC: AtomicU64 = AtomicU64::new(0);
static FAULT_PSTATE: AtomicU64 = AtomicU64::new(0);
static FAULT_REGS: [AtomicU64; 31] = [const { AtomicU64::new(0) }; 31];

/// Record a recovered user-access fault. Called by the kernel's synchronous
/// handler right before it redirects `ELR_EL1` to the fixup. The first fault
/// of a syscall wins: a later one (the Rust caller ignoring the `Err` and
/// trying again) does not overwrite the report of the original.
pub fn note_fault(rec: &FaultRecord) {
    if FAULT_ARMED.load(Ordering::Acquire) {
        return;
    }
    FAULT_FAR.store(rec.far, Ordering::Relaxed);
    FAULT_ESR.store(rec.esr, Ordering::Relaxed);
    FAULT_PC.store(rec.pc, Ordering::Relaxed);
    FAULT_PSTATE.store(rec.pstate, Ordering::Relaxed);
    for (dst, &v) in FAULT_REGS.iter().zip(rec.regs.iter()) {
        dst.store(v, Ordering::Relaxed);
    }
    FAULT_ARMED.store(true, Ordering::Release);
}

/// Consume the pending fault record, if a user access faulted since the last
/// call. The kernel calls this when a syscall returns to its caller.
pub fn take_pending_fault() -> Option<FaultRecord> {
    if !FAULT_ARMED.swap(false, Ordering::AcqRel) {
        return None;
    }
    let mut regs = [0u64; 31];
    for (dst, src) in regs.iter_mut().zip(FAULT_REGS.iter()) {
        *dst = src.load(Ordering::Relaxed);
    }
    Some(FaultRecord {
        far: FAULT_FAR.load(Ordering::Relaxed),
        esr: FAULT_ESR.load(Ordering::Relaxed),
        pc: FAULT_PC.load(Ordering::Relaxed),
        pstate: FAULT_PSTATE.load(Ordering::Relaxed),
        regs,
    })
}

// ---------------------------------------------------------------------------
// Public accessors
// ---------------------------------------------------------------------------

/// Read one byte from user address `va`.
pub fn get_user_u8(va: u64) -> Result<u8, UserFault> {
    let mut v = 0u8;
    // SAFETY: `&mut v` is a valid kernel destination of the right size.
    status(unsafe { raw::ld8(va, &mut v) }).map(|()| v)
}

/// Read a 16-bit value from user address `va` (native byte order).
pub fn get_user_u16(va: u64) -> Result<u16, UserFault> {
    let mut v = 0u16;
    // SAFETY: as in [`get_user_u8`].
    status(unsafe { raw::ld16(va, &mut v) }).map(|()| v)
}

/// Read a 32-bit value from user address `va` (native byte order).
pub fn get_user_u32(va: u64) -> Result<u32, UserFault> {
    let mut v = 0u32;
    // SAFETY: as in [`get_user_u8`].
    status(unsafe { raw::ld32(va, &mut v) }).map(|()| v)
}

/// Read a 64-bit value from user address `va` (native byte order).
pub fn get_user_u64(va: u64) -> Result<u64, UserFault> {
    let mut v = 0u64;
    // SAFETY: as in [`get_user_u8`].
    status(unsafe { raw::ld64(va, &mut v) }).map(|()| v)
}

/// Write one byte to user address `va`.
pub fn put_user_u8(va: u64, v: u8) -> Result<(), UserFault> {
    // SAFETY: the store goes through the unprivileged, fault-recovered path.
    status(unsafe { raw::st8(va, u64::from(v)) })
}

/// Write a 16-bit value to user address `va` (native byte order).
pub fn put_user_u16(va: u64, v: u16) -> Result<(), UserFault> {
    // SAFETY: as in [`put_user_u8`].
    status(unsafe { raw::st16(va, u64::from(v)) })
}

/// Write a 32-bit value to user address `va` (native byte order).
pub fn put_user_u32(va: u64, v: u32) -> Result<(), UserFault> {
    // SAFETY: as in [`put_user_u8`].
    status(unsafe { raw::st32(va, u64::from(v)) })
}

/// Write a 64-bit value to user address `va` (native byte order).
pub fn put_user_u64(va: u64, v: u64) -> Result<(), UserFault> {
    // SAFETY: as in [`put_user_u8`].
    status(unsafe { raw::st64(va, v) })
}

/// Copy `dst.len()` bytes from user address `src` into the kernel buffer
/// `dst`. On a fault the bytes before the faulting one have been copied.
pub fn copy_from_user(dst: &mut [u8], src: u64) -> Result<(), UserFault> {
    // SAFETY: `dst` is a valid kernel buffer of `dst.len()` bytes.
    let left = unsafe { raw::copy_from(dst.as_mut_ptr(), src, dst.len() as u64) };
    if left == 0 { Ok(()) } else { Err(UserFault) }
}

/// Copy the kernel buffer `src` to user address `dst`. On a fault the bytes
/// before the faulting one have been stored.
pub fn copy_to_user(dst: u64, src: &[u8]) -> Result<(), UserFault> {
    // SAFETY: `src` is a valid kernel buffer of `src.len()` bytes.
    let left = unsafe { raw::copy_to(dst, src.as_ptr(), src.len() as u64) };
    if left == 0 { Ok(()) } else { Err(UserFault) }
}

/// Copy a NUL-terminated string from user address `src` into `dst`,
/// returning its length (without the NUL). Reading stops at the NUL or after
/// `dst.len()` bytes — a missing terminator truncates (the returned length is
/// then `dst.len()`); the NUL itself is not stored.
pub fn read_user_cstr(src: u64, dst: &mut [u8]) -> Result<usize, UserFault> {
    // SAFETY: `dst` is a valid kernel buffer of `dst.len()` bytes.
    let n = unsafe { raw::strncpy_from(dst.as_mut_ptr(), src, dst.len() as u64) };
    if n < 0 { Err(UserFault) } else { Ok(n as usize) }
}

// ---------------------------------------------------------------------------
// Caller address space (ARCH03 stage 3a)
// ---------------------------------------------------------------------------

/// Whose address space the pointer arguments of a call live in.
///
/// A trap from an EL0 program hands the kernel **user** addresses, which are
/// accessed with that program's rights — the unprivileged, fault-recovered
/// routines of this module. The in-kernel callers — kernel code that traps
/// with `svc` from EL1 (EmuCON, `kernel/src/shell.rs`), the AES fiber and the kernel's own services calling
/// a GEMDOS leaf directly — hand it **kernel**
/// addresses (high-half VAs, kernel stacks and statics), which an EL0-rights
/// access would refuse; they are accessed as they are, exactly as before.
/// The trap handler derives the space from `SPSR_EL1.M` of the caller
/// ([`crate::trap::TrapArgs::space`]); every TrapArgs the kernel builds for
/// itself says [`Space::Kernel`].
///
/// On the host (unit tests) both are plain host-memory accesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// The calling EL0 program's address space: `ldtr*`/`sttr*` with fixups,
    /// a bad pointer ends the caller (module comment).
    User,
    /// A kernel caller passing kernel pointers: ordinary loads and stores.
    /// A bad pointer here is a kernel bug and stays fatal.
    Kernel,
}

impl Space {
    /// The space of a trap's caller from its saved `SPSR_EL1`: `M[3:0] = 0`
    /// (EL0t) is a program, anything else an EL1 caller.
    #[inline]
    pub const fn from_spsr(spsr: u64) -> Self {
        if spsr & 0xF == 0 { Space::User } else { Space::Kernel }
    }

    /// `true` for [`Space::User`].
    #[inline]
    pub const fn is_user(self) -> bool {
        matches!(self, Space::User)
    }

    /// Read one byte at `va`.
    #[inline]
    pub fn get_u8(self, va: u64) -> Result<u8, UserFault> {
        match self {
            Space::User => get_user_u8(va),
            // SAFETY: a kernel caller passes a valid kernel address (enum doc).
            Space::Kernel => Ok(unsafe { (va as *const u8).read_volatile() }),
        }
    }

    /// Read a native-order 16-bit value at `va` (any alignment).
    #[inline]
    pub fn get_u16(self, va: u64) -> Result<u16, UserFault> {
        match self {
            Space::User => get_user_u16(va),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => Ok(unsafe { (va as *const u16).read_unaligned() }),
        }
    }

    /// Read a native-order 32-bit value at `va` (any alignment).
    #[inline]
    pub fn get_u32(self, va: u64) -> Result<u32, UserFault> {
        match self {
            Space::User => get_user_u32(va),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => Ok(unsafe { (va as *const u32).read_unaligned() }),
        }
    }

    /// Read a native-order 64-bit value at `va` (any alignment).
    #[inline]
    pub fn get_u64(self, va: u64) -> Result<u64, UserFault> {
        match self {
            Space::User => get_user_u64(va),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => Ok(unsafe { (va as *const u64).read_unaligned() }),
        }
    }

    /// Write one byte at `va`.
    #[inline]
    pub fn put_u8(self, va: u64, v: u8) -> Result<(), UserFault> {
        match self {
            Space::User => put_user_u8(va, v),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => {
                unsafe { (va as *mut u8).write_volatile(v) };
                Ok(())
            }
        }
    }

    /// Write a native-order 16-bit value at `va` (any alignment).
    #[inline]
    pub fn put_u16(self, va: u64, v: u16) -> Result<(), UserFault> {
        match self {
            Space::User => put_user_u16(va, v),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => {
                unsafe { (va as *mut u16).write_unaligned(v) };
                Ok(())
            }
        }
    }

    /// Write a native-order 32-bit value at `va` (any alignment).
    #[inline]
    pub fn put_u32(self, va: u64, v: u32) -> Result<(), UserFault> {
        match self {
            Space::User => put_user_u32(va, v),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => {
                unsafe { (va as *mut u32).write_unaligned(v) };
                Ok(())
            }
        }
    }

    /// Write a native-order 64-bit value at `va` (any alignment).
    #[inline]
    pub fn put_u64(self, va: u64, v: u64) -> Result<(), UserFault> {
        match self {
            Space::User => put_user_u64(va, v),
            // SAFETY: as in [`Self::get_u8`].
            Space::Kernel => {
                unsafe { (va as *mut u64).write_unaligned(v) };
                Ok(())
            }
        }
    }

    /// Copy `dst.len()` bytes from `src` into the kernel buffer `dst`.
    pub fn copy_from(self, dst: &mut [u8], src: u64) -> Result<(), UserFault> {
        match self {
            Space::User => copy_from_user(dst, src),
            Space::Kernel => {
                // SAFETY: as in [`Self::get_u8`]; `dst` is a distinct kernel
                // buffer of `dst.len()` bytes (`copy` tolerates overlap anyway).
                unsafe { core::ptr::copy(src as *const u8, dst.as_mut_ptr(), dst.len()) };
                Ok(())
            }
        }
    }

    /// Copy the kernel buffer `src` to `dst`.
    pub fn copy_to(self, dst: u64, src: &[u8]) -> Result<(), UserFault> {
        match self {
            Space::User => copy_to_user(dst, src),
            Space::Kernel => {
                // SAFETY: as in [`Self::copy_from`].
                unsafe { core::ptr::copy(src.as_ptr(), dst as *mut u8, src.len()) };
                Ok(())
            }
        }
    }

    /// Copy a NUL-terminated string at `src` into `dst` — [`read_user_cstr`]'s
    /// contract (length without the NUL, truncated at `dst.len()`).
    pub fn read_cstr(self, src: u64, dst: &mut [u8]) -> Result<usize, UserFault> {
        match self {
            Space::User => read_user_cstr(src, dst),
            Space::Kernel => {
                let mut n = 0;
                while n < dst.len() {
                    // SAFETY: as in [`Self::get_u8`]; bounded by `dst.len()`.
                    let b = unsafe { ((src + n as u64) as *const u8).read_volatile() };
                    if b == 0 {
                        break;
                    }
                    dst[n] = b;
                    n += 1;
                }
                Ok(n)
            }
        }
    }

    /// The **kernel** address through which the kernel may access the `len`
    /// bytes at caller address `va` with ordinary loads and stores, after
    /// checking that the caller itself may read them (and, with `write`,
    /// write them) — plan §3.6 "kalias + check".
    ///
    /// For [`Space::Kernel`] the pointer is already a kernel address and is
    /// returned as it is. For [`Space::User`] every page of the range is
    /// probed **with the caller's rights** (an unprivileged load; for
    /// `write`, the same byte stored back with interrupts masked, so the
    /// probe changes nothing), and the result is the kernel high-half alias
    /// ([`crate::bdos::process::kalias`], VA = PA) — or, for the private
    /// page 0 (`va < 0x1000`, VA ≠ PA), the running process's own page-0
    /// alias. The access through that address no longer depends on which
    /// `TTBR0` is live or on user rights.
    ///
    /// A probe that faults is a genuine user-access fault: it is recorded
    /// like every other one ([`note_fault`] through the abort handler) and
    /// the syscall tail ends the caller. The one non-faulting refusal is a
    /// range that starts in page 0 and runs past it (page 0 is not
    /// contiguous with the shared space behind it): `Err` without a record.
    ///
    /// Why the caller's rights and not a region-table lookup: this is a
    /// syscall argument, checked in the caller's own trap, so the MMU check
    /// with its rights is the stage-3 answer itself (it also refuses a write
    /// to a `PROT_PR` page the caller may only read, which "is this an
    /// allocated block" cannot). Pointers followed *outside* their owner's
    /// trap keep using [`crate::bdos::process::usermem::resolve_kva`].
    pub fn kptr(self, va: u64, len: u64, write: bool) -> Result<u64, UserFault> {
        match self {
            Space::Kernel => Ok(va),
            Space::User => user_kptr(va, len, write),
        }
    }
}

impl Space {
    /// [`Self::kptr`] that leaves a null pointer null (unchecked), for the
    /// calls whose contract allows 0 for "no buffer".
    pub fn kptr_or_null(self, va: u64, len: u64, write: bool) -> Result<u64, UserFault> {
        if va == 0 { Ok(0) } else { self.kptr(va, len, write) }
    }
}

/// [`Space::kptr`] for a user range.
fn user_kptr(va: u64, len: u64, write: bool) -> Result<u64, UserFault> {
    const PAGE: u64 = 0x1000;
    let probe = |a: u64| if write { probe_user_write(a) } else { get_user_u8(a).map(|_| ()) };
    if len == 0 {
        return Ok(alias_of(va));
    }
    // Every page the range touches: its first byte in the range.
    let last = va.checked_add(len - 1);
    probe(va)?;
    let mut page = (va & !(PAGE - 1)).wrapping_add(PAGE);
    match last {
        Some(last) => {
            while page <= last && page != 0 {
                probe(page)?;
                page = page.wrapping_add(PAGE);
            }
        }
        // The range wraps the address space: the first page past the top is
        // not user memory — probe the highest address so the fault is real.
        None => probe(u64::MAX)?,
    }
    // A slot-private page (page 0, the stub page, the private read-write
    // page, MEM12) is not contiguous with the memory around it: a range may
    // not run across its end.
    if crate::bdos::process::mmu::in_private_page(va) {
        let page_end = (va & !(PAGE - 1)) + PAGE;
        if va + len > page_end {
            return Err(UserFault);
        }
    }
    Ok(alias_of(va))
}

/// The kernel alias of user address `va` of the running process: page 0 and
/// the private read-write page through the process's own frames, everything
/// else VA = PA. (The stub page never gets here: it is EL0 execute-only, so
/// the probe of [`user_kptr`] faults on it.)
fn alias_of(va: u64) -> u64 {
    if let Some(slot) = crate::bdos::process::slot_for(crate::bdos::process::current()) {
        if let Some(a) = crate::bdos::process::mmu::private_page_alias(slot, va) {
            return a;
        }
    }
    crate::bdos::process::kalias(va)
}

/// Check that the caller may write the byte at `va`, without changing it:
/// read it and store it back with the caller's rights, interrupts masked so
/// no interrupt handler can change the byte in between.
fn probe_user_write(va: u64) -> Result<(), UserFault> {
    let saved = irq_mask();
    let r = get_user_u8(va).and_then(|b| put_user_u8(va, b));
    irq_restore(saved);
    r
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[inline]
fn irq_mask() -> u64 {
    let daif: u64;
    // SAFETY: reads DAIF and masks IRQs at EL1; restored by `irq_restore`.
    unsafe {
        core::arch::asm!("mrs {d}, daif", "msr daifset, #2", d = out(reg) daif,
            options(nomem, nostack, preserves_flags));
    }
    daif
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[inline]
fn irq_restore(daif: u64) {
    // SAFETY: restores the DAIF value `irq_mask` read.
    unsafe {
        core::arch::asm!("msr daif, {d}", d = in(reg) daif, options(nomem, nostack, preserves_flags));
    }
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[inline]
fn irq_mask() -> u64 {
    0
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[inline]
fn irq_restore(_daif: u64) {}

#[inline]
fn status(rc: u64) -> Result<(), UserFault> {
    if rc == 0 { Ok(()) } else { Err(UserFault) }
}

// ---------------------------------------------------------------------------
// Target implementation: leaf routines + fixup table
// ---------------------------------------------------------------------------

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
mod raw {
    use super::FixupEntry;

    // Every routine is a **leaf** (no frame, `lr` untouched), so a fixup can
    // simply set the return value and `ret` to the Rust caller. Calling
    // convention AAPCS64; the scratch registers used are caller-saved.
    //
    // Status routines return `x0 = 0` on success and `1` after a fault.
    // `copy_*` return the number of bytes **not** copied (0 = all done);
    // `strncpy_from` returns the string length or `-1` after a fault.
    core::arch::global_asm!(
        r#"
    .pushsection .text.newtos_uaccess, "ax", %progbits
    .balign 4

    .global __uaccess_ld8
    .type   __uaccess_ld8, %function
__uaccess_ld8:
.Luacc_ld8:
    ldtrb   w2, [x0]
    strb    w2, [x1]
    mov     x0, #0
    ret

    .global __uaccess_ld16
    .type   __uaccess_ld16, %function
__uaccess_ld16:
.Luacc_ld16:
    ldtrh   w2, [x0]
    strh    w2, [x1]
    mov     x0, #0
    ret

    .global __uaccess_ld32
    .type   __uaccess_ld32, %function
__uaccess_ld32:
.Luacc_ld32:
    ldtr    w2, [x0]
    str     w2, [x1]
    mov     x0, #0
    ret

    .global __uaccess_ld64
    .type   __uaccess_ld64, %function
__uaccess_ld64:
.Luacc_ld64:
    ldtr    x2, [x0]
    str     x2, [x1]
    mov     x0, #0
    ret

    .global __uaccess_st8
    .type   __uaccess_st8, %function
__uaccess_st8:
.Luacc_st8:
    sttrb   w1, [x0]
    mov     x0, #0
    ret

    .global __uaccess_st16
    .type   __uaccess_st16, %function
__uaccess_st16:
.Luacc_st16:
    sttrh   w1, [x0]
    mov     x0, #0
    ret

    .global __uaccess_st32
    .type   __uaccess_st32, %function
__uaccess_st32:
.Luacc_st32:
    sttr    w1, [x0]
    mov     x0, #0
    ret

    .global __uaccess_st64
    .type   __uaccess_st64, %function
__uaccess_st64:
.Luacc_st64:
    sttr    x1, [x0]
    mov     x0, #0
    ret

    // Shared fixup of the status routines: report the fault.
.Luacc_fault_status:
    mov     x0, #1
    ret

    // x0 = dst (kernel), x1 = src (user), x2 = count.
    // Eight bytes per unprivileged load while at least eight remain (an
    // unaligned `ldtr` is permitted on Normal memory, SCTLR_EL1.A = 0), then
    // byte by byte. A fault in an 8-byte load counts all eight as not copied.
    .global __uaccess_copy_from
    .type   __uaccess_copy_from, %function
__uaccess_copy_from:
    cbz     x2, .Luacc_cf_done
.Luacc_cf_wide:
    cmp     x2, #8
    b.lo    .Luacc_cf_loop
.Luacc_cf_insn8:
    ldtr    x3, [x1]
    str     x3, [x0], #8
    add     x1, x1, #8
    subs    x2, x2, #8
    b.ne    .Luacc_cf_wide
    b       .Luacc_cf_done
.Luacc_cf_loop:
.Luacc_cf_insn:
    ldtrb   w3, [x1]
    strb    w3, [x0], #1
    add     x1, x1, #1
    subs    x2, x2, #1
    b.ne    .Luacc_cf_loop
.Luacc_cf_done:
    // Also the fixup: x2 still counts the faulting bytes as not copied.
    mov     x0, x2
    ret

    // x0 = dst (user), x1 = src (kernel), x2 = count; 8-byte stores first,
    // as in `__uaccess_copy_from`.
    .global __uaccess_copy_to
    .type   __uaccess_copy_to, %function
__uaccess_copy_to:
    cbz     x2, .Luacc_ct_done
.Luacc_ct_wide:
    cmp     x2, #8
    b.lo    .Luacc_ct_loop
    ldr     x3, [x1]
.Luacc_ct_insn8:
    sttr    x3, [x0]
    add     x1, x1, #8
    add     x0, x0, #8
    subs    x2, x2, #8
    b.ne    .Luacc_ct_wide
    b       .Luacc_ct_done
.Luacc_ct_loop:
    ldrb    w3, [x1], #1
.Luacc_ct_insn:
    sttrb   w3, [x0]
    add     x0, x0, #1
    subs    x2, x2, #1
    b.ne    .Luacc_ct_loop
.Luacc_ct_done:
    mov     x0, x2
    ret

    // x0 = dst (kernel), x1 = src (user), x2 = max; returns length or -1
    .global __uaccess_strncpy_from
    .type   __uaccess_strncpy_from, %function
__uaccess_strncpy_from:
    mov     x4, #0
.Luacc_sc_loop:
    cmp     x4, x2
    b.hs    .Luacc_sc_done
.Luacc_sc_insn:
    ldtrb   w3, [x1, #0]
    cbz     w3, .Luacc_sc_done
    strb    w3, [x0, x4]
    add     x1, x1, #1
    add     x4, x4, #1
    b       .Luacc_sc_loop
.Luacc_sc_done:
    mov     x0, x4
    ret
.Luacc_sc_fault:
    mov     x0, #-1
    ret
    .popsection

    .pushsection .rodata.newtos_uaccess_fixups, "a", %progbits
    .balign 8
    .global __uaccess_fixups_start
__uaccess_fixups_start:
    .quad .Luacc_ld8,  .Luacc_fault_status
    .quad .Luacc_ld16, .Luacc_fault_status
    .quad .Luacc_ld32, .Luacc_fault_status
    .quad .Luacc_ld64, .Luacc_fault_status
    .quad .Luacc_st8,  .Luacc_fault_status
    .quad .Luacc_st16, .Luacc_fault_status
    .quad .Luacc_st32, .Luacc_fault_status
    .quad .Luacc_st64, .Luacc_fault_status
    .quad .Luacc_cf_insn8, .Luacc_cf_done
    .quad .Luacc_cf_insn, .Luacc_cf_done
    .quad .Luacc_ct_insn8, .Luacc_ct_done
    .quad .Luacc_ct_insn, .Luacc_ct_done
    .quad .Luacc_sc_insn, .Luacc_sc_fault
    .global __uaccess_fixups_end
__uaccess_fixups_end:
    .popsection
"#
    );

    unsafe extern "C" {
        #[link_name = "__uaccess_ld8"]
        pub fn ld8(src: u64, dst: *mut u8) -> u64;
        #[link_name = "__uaccess_ld16"]
        pub fn ld16(src: u64, dst: *mut u16) -> u64;
        #[link_name = "__uaccess_ld32"]
        pub fn ld32(src: u64, dst: *mut u32) -> u64;
        #[link_name = "__uaccess_ld64"]
        pub fn ld64(src: u64, dst: *mut u64) -> u64;
        #[link_name = "__uaccess_st8"]
        pub fn st8(dst: u64, v: u64) -> u64;
        #[link_name = "__uaccess_st16"]
        pub fn st16(dst: u64, v: u64) -> u64;
        #[link_name = "__uaccess_st32"]
        pub fn st32(dst: u64, v: u64) -> u64;
        #[link_name = "__uaccess_st64"]
        pub fn st64(dst: u64, v: u64) -> u64;
        #[link_name = "__uaccess_copy_from"]
        pub fn copy_from(dst: *mut u8, src: u64, n: u64) -> u64;
        #[link_name = "__uaccess_copy_to"]
        pub fn copy_to(dst: u64, src: *const u8, n: u64) -> u64;
        #[link_name = "__uaccess_strncpy_from"]
        pub fn strncpy_from(dst: *mut u8, src: u64, n: u64) -> i64;

        static __uaccess_fixups_start: FixupEntry;
        static __uaccess_fixups_end: FixupEntry;
    }

    /// The fixup table emitted by the assembly above.
    pub fn table() -> &'static [FixupEntry] {
        let start = &raw const __uaccess_fixups_start;
        let end = &raw const __uaccess_fixups_end;
        let len = (end as usize - start as usize) / core::mem::size_of::<FixupEntry>();
        // SAFETY: the assembly emits `len` contiguous, 8-byte aligned
        // `FixupEntry` records between the two symbols, in read-only data
        // that lives for the whole kernel lifetime.
        unsafe { core::slice::from_raw_parts(start, len) }
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[inline]
fn fixup_table() -> &'static [FixupEntry] {
    raw::table()
}

/// Host build: no EL0/EL1 split, no recoverable faults — plain accesses.
#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
mod raw {
    pub unsafe fn ld8(src: u64, dst: *mut u8) -> u64 {
        // SAFETY: host tests pass valid host addresses.
        unsafe { dst.write((src as *const u8).read_volatile()) };
        0
    }
    pub unsafe fn ld16(src: u64, dst: *mut u16) -> u64 {
        // SAFETY: as above.
        unsafe { dst.write((src as *const u16).read_unaligned()) };
        0
    }
    pub unsafe fn ld32(src: u64, dst: *mut u32) -> u64 {
        // SAFETY: as above.
        unsafe { dst.write((src as *const u32).read_unaligned()) };
        0
    }
    pub unsafe fn ld64(src: u64, dst: *mut u64) -> u64 {
        // SAFETY: as above.
        unsafe { dst.write((src as *const u64).read_unaligned()) };
        0
    }
    pub unsafe fn st8(dst: u64, v: u64) -> u64 {
        // SAFETY: as above.
        unsafe { (dst as *mut u8).write_volatile(v as u8) };
        0
    }
    pub unsafe fn st16(dst: u64, v: u64) -> u64 {
        // SAFETY: as above.
        unsafe { (dst as *mut u16).write_unaligned(v as u16) };
        0
    }
    pub unsafe fn st32(dst: u64, v: u64) -> u64 {
        // SAFETY: as above.
        unsafe { (dst as *mut u32).write_unaligned(v as u32) };
        0
    }
    pub unsafe fn st64(dst: u64, v: u64) -> u64 {
        // SAFETY: as above.
        unsafe { (dst as *mut u64).write_unaligned(v) };
        0
    }
    pub unsafe fn copy_from(dst: *mut u8, src: u64, n: u64) -> u64 {
        // SAFETY: as above; `dst` holds `n` bytes.
        unsafe { core::ptr::copy(src as *const u8, dst, n as usize) };
        0
    }
    pub unsafe fn copy_to(dst: u64, src: *const u8, n: u64) -> u64 {
        // SAFETY: as above; `src` holds `n` bytes.
        unsafe { core::ptr::copy(src, dst as *mut u8, n as usize) };
        0
    }
    pub unsafe fn strncpy_from(dst: *mut u8, src: u64, n: u64) -> i64 {
        let mut i = 0u64;
        while i < n {
            // SAFETY: as above; bounded by `n`.
            let b = unsafe { ((src + i) as *const u8).read_volatile() };
            if b == 0 {
                break;
            }
            // SAFETY: `dst` holds `n` bytes and `i < n`.
            unsafe { dst.add(i as usize).write(b) };
            i += 1;
        }
        i as i64
    }
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[inline]
fn fixup_table() -> &'static [FixupEntry] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: [FixupEntry; 3] = [
        FixupEntry { insn: 0xFFFF_0000_0010_0000, fixup: 0xFFFF_0000_0010_0100 },
        FixupEntry { insn: 0xFFFF_0000_0010_0010, fixup: 0xFFFF_0000_0010_0100 },
        FixupEntry { insn: 0xFFFF_0000_0010_0040, fixup: 0xFFFF_0000_0010_0050 },
    ];

    #[test]
    fn lookup_hits_exact_instruction() {
        assert_eq!(lookup_fixup(&TABLE, 0xFFFF_0000_0010_0000), Some(0xFFFF_0000_0010_0100));
        assert_eq!(lookup_fixup(&TABLE, 0xFFFF_0000_0010_0010), Some(0xFFFF_0000_0010_0100));
        assert_eq!(lookup_fixup(&TABLE, 0xFFFF_0000_0010_0040), Some(0xFFFF_0000_0010_0050));
    }

    #[test]
    fn lookup_misses_neighbours_and_unrelated_pcs() {
        // One instruction before/after a listed one is a different PC: a
        // kernel fault there must stay fatal.
        assert_eq!(lookup_fixup(&TABLE, 0xFFFF_0000_0010_0004), None);
        assert_eq!(lookup_fixup(&TABLE, 0xFFFF_0000_0010_003C), None);
        assert_eq!(lookup_fixup(&TABLE, 0), None);
        assert_eq!(lookup_fixup(&[], 0xFFFF_0000_0010_0000), None);
    }

    #[test]
    fn host_table_is_empty() {
        // Nothing can fault recoverably on the host.
        assert_eq!(fixup_for(0xFFFF_0000_0010_0000), None);
    }

    #[test]
    fn fault_record_round_trip_and_first_fault_wins() {
        let mut regs = [0u64; 31];
        regs[0] = 0x1111;
        regs[30] = 0x3030;
        let first = FaultRecord { far: 0xDEAD_0000, esr: 0x9600_0047, pc: 0x10, pstate: 0x3C5, regs };
        let second = FaultRecord { far: 0xBEEF, ..first };
        // Drain anything a previous test left (tests share the statics).
        let _ = take_pending_fault();
        note_fault(&first);
        note_fault(&second);
        assert_eq!(take_pending_fault(), Some(first));
        assert_eq!(take_pending_fault(), None);
    }

    #[test]
    fn space_from_spsr_el0t_is_user() {
        assert_eq!(Space::from_spsr(0x0000_0000), Space::User); // EL0t
        assert_eq!(Space::from_spsr(0x6000_0000), Space::User); // EL0t, flags set
        assert_eq!(Space::from_spsr(0x0000_0004), Space::Kernel); // EL1t
        assert_eq!(Space::from_spsr(0x0000_0005), Space::Kernel); // EL1h
        assert_eq!(Space::from_spsr(0x0000_03C5), Space::Kernel);
    }

    #[test]
    fn both_spaces_read_and_write_the_same_bytes_on_the_host() {
        for sp in [Space::User, Space::Kernel] {
            let mut w = [0u8; 24];
            let base = w.as_mut_ptr() as u64;
            sp.put_u8(base, 0x11).unwrap();
            sp.put_u16(base + 1, 0x2233).unwrap(); // unaligned
            sp.put_u32(base + 3, 0x4455_6677).unwrap();
            sp.put_u64(base + 7, 0x8899_AABB_CCDD_EEFF).unwrap();
            assert_eq!(sp.get_u8(base), Ok(0x11));
            assert_eq!(sp.get_u16(base + 1), Ok(0x2233));
            assert_eq!(sp.get_u32(base + 3), Ok(0x4455_6677));
            assert_eq!(sp.get_u64(base + 7), Ok(0x8899_AABB_CCDD_EEFF));
            let src: [u8; 9] = *b"ABCDEFGHI";
            sp.copy_to(base + 15, &src).unwrap();
            let mut back = [0u8; 9];
            sp.copy_from(&mut back, base + 15).unwrap();
            assert_eq!(back, src);
            let s = b"C:\\X\0rest";
            let mut buf = [0u8; 8];
            assert_eq!(sp.read_cstr(s.as_ptr() as u64, &mut buf), Ok(4));
            let mut two = [0u8; 2];
            assert_eq!(sp.read_cstr(s.as_ptr() as u64, &mut two), Ok(2), "truncates");
        }
    }

    #[test]
    fn kptr_of_a_kernel_pointer_is_the_pointer() {
        assert_eq!(Space::Kernel.kptr(0xFFFF_0000_1234_5678, 64, true), Ok(0xFFFF_0000_1234_5678));
        assert_eq!(Space::Kernel.kptr(0, 0, false), Ok(0));
    }

    #[test]
    fn kptr_of_user_memory_is_its_alias_after_the_probe() {
        // Host: the alias of user memory is the address itself, and the
        // probes are plain accesses that leave the bytes unchanged.
        let mut buf = vec![0x5Au8; 3 * 4096];
        let va = buf.as_mut_ptr() as u64 + 100;
        assert_eq!(Space::User.kptr(va, 2 * 4096, true), Ok(va));
        assert!(buf.iter().all(|&b| b == 0x5A), "a write probe changes nothing");
        assert_eq!(Space::User.kptr(va, 0, false), Ok(va));
    }

    #[test]
    fn host_accessors_read_and_write() {
        let s = b"PATH\0junk";
        let mut buf = [0u8; 16];
        assert_eq!(read_user_cstr(s.as_ptr() as u64, &mut buf), Ok(4));
        assert_eq!(&buf[..4], b"PATH");
        // Truncation without a NUL inside the window.
        let mut small = [0u8; 2];
        assert_eq!(read_user_cstr(s.as_ptr() as u64, &mut small), Ok(2));
        let mut w = 0u32;
        put_user_u32(&mut w as *mut u32 as u64, 0xA5A5_5A5A).unwrap();
        assert_eq!(get_user_u32(&w as *const u32 as u64), Ok(0xA5A5_5A5A));
    }
}
