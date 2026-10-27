//! AArch64 exception entry: vector table, register save/restore, and
//! Rust-level dispatch.
//!
//! ## Vector layout (`exception_vectors.S`)
//!
//! AArch64 mandates a 2 KiB aligned table with sixteen 128-byte slots
//! grouped into four sets of four entries: synchronous, IRQ, FIQ,
//! SError, replicated for each entry source (current EL/SP0,
//! current EL/SPx, lower-EL/AArch64, lower-EL/AArch32). See ARM ARM
//! D1.10.2 "Exception vectors".
//!
//! Phase 02 only meaningfully implements the "Synchronous, current EL
//! with SPx" entry (used while we still run with EL1 and SP_EL1 ==
//! SP), all other entries trap into [`unhandled`] for diagnostics.
//!
//! ## ABI
//!
//! Synchronous handler stack frame (built by the assembly stub —
//! Phase 05 D6 expanded the frame from 272 B to 288 B for `SP_EL0`):
//!
//! ```text
//!   sp+ 0:   x0   .. x29 (30 × 8 B = 240 B)
//!   sp+240:  lr  (x30)
//!   sp+248:  ELR_EL1
//!   sp+256:  SPSR_EL1
//!   sp+264:  ESR_EL1
//!   sp+272:  SP_EL0   (user-space stack pointer at trap entry)
//!   sp+280:  pad      (reserved — keeps the frame 16-B aligned;
//!                      future TPIDR_EL0 / process tag slot)
//! ```
//!
//! The 16-byte-aligned SP is required by AAPCS64 §5.2.2.1 at every
//! public function call boundary.

use core::arch::global_asm;
use core::sync::atomic::{AtomicBool, Ordering};

use tos_api::trap::{TrapArgs, TrapClass};

global_asm!(include_str!("exception_vectors.s"));

/// Saved register frame layout (matches `exception_vectors.s`). Total
/// 288 bytes = 18 × 16 B (16-byte SP-aligned per AAPCS64 §5.2.2).
///
/// Phase 06 will read [`Self::sp_el0`] in the SVC dispatcher to
/// access user-stack-passed parameters (e.g. m68k-style TOS calls
/// after the marshaling shim, see `docs/ARCHITECTURE.md` § "ABI
/// rules — m68k callers"). Until then, traps originate from EL1
/// and `sp_el0` carries the kernel's main stack pointer (set by the
/// `kernel_main` shim, [`crate::kernel_main`]).
#[repr(C)]
pub struct ExceptionFrame {
    /// `x0..x29` at exception entry.
    pub gpr: [u64; 30],
    /// `x30` (link register).
    pub lr: u64,
    /// Exception link register — the address to return to.
    pub elr: u64,
    /// Saved program status register.
    pub spsr: u64,
    /// Exception syndrome.
    pub esr: u64,
    /// User-space stack pointer (`SP_EL0`) at trap entry.
    pub sp_el0: u64,
    /// Reserved 8-byte pad (keeps `stp`/`ldp` pair-aligned; future
    /// TPIDR_EL0 or per-process tag slot).
    _pad: u64,
}

const _: () = assert!(core::mem::size_of::<ExceptionFrame>() == 288);
const _: () = assert!(core::mem::align_of::<ExceptionFrame>() == 8);

// ---------------------------------------------------------------------------
// ESR_EL1 decoding (ARM ARM D17.2.37)
// ---------------------------------------------------------------------------

/// `ESR_EL1.EC` field for `SVC` from AArch64 EL0 (`0x15`) — used after
/// PHASE-06 introduces user-mode processes.
const EC_SVC_AARCH64: u64 = 0x15;

/// `ESR_EL1.EC` field for `SVC` from the same EL (`0x15` shares the
/// encoding with EL0; bit 0 of EC distinguishes — see ARM ARM table
/// D17-32). PHASE-02 runs in EL1 and uses SVC for self-tests, so we
/// accept both encodings.
const EC_SVC_FROM_EL1: u64 = 0x15;

/// `ESR_EL1.EC` for an Instruction Abort taken from a lower EL.
const EC_IABT_LOWER: u64 = 0x20;
/// `ESR_EL1.EC` for an Instruction Abort taken from the same EL.
const EC_IABT_SAME: u64 = 0x21;
/// `ESR_EL1.EC` for a Data Abort taken from a lower EL.
const EC_DABT_LOWER: u64 = 0x24;
/// `ESR_EL1.EC` for a Data Abort taken from the same EL — what a
/// kernel-side stack overflow into the guard page produces.
const EC_DABT_SAME: u64 = 0x25;

#[inline]
const fn esr_ec(esr: u64) -> u64 {
    (esr >> 26) & 0x3F
}

#[inline]
const fn esr_iss(esr: u64) -> u64 {
    esr & 0x01FF_FFFF
}

#[inline]
const fn is_abort(ec: u64) -> bool {
    matches!(ec, EC_IABT_LOWER | EC_IABT_SAME | EC_DABT_LOWER | EC_DABT_SAME)
}

#[inline]
fn read_far_el1() -> u64 {
    let v: u64;
    // SAFETY: `mrs` of FAR_EL1 is permitted at EL1 and has no side
    // effects.
    unsafe {
        core::arch::asm!(
            "mrs {0}, far_el1",
            out(reg) v,
            options(nomem, nostack, preserves_flags)
        );
    }
    v
}

#[inline]
fn ec_name(ec: u64) -> &'static str {
    match ec {
        EC_SVC_AARCH64 => "SVC (AArch64)",
        EC_IABT_LOWER => "Instruction Abort (lower EL)",
        EC_IABT_SAME => "Instruction Abort (same EL)",
        EC_DABT_LOWER => "Data Abort (lower EL)",
        EC_DABT_SAME => "Data Abort (same EL)",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// VBAR_EL1 setup
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// Symbol exported by `exception_vectors.s` — the start of the
    /// 2 KiB vector table.
    fn __exception_vectors();
}

/// Install the exception vector table by writing its address into
/// `VBAR_EL1`.
///
/// Must be called once during boot before any SVC is executed.
pub fn init() {
    // SAFETY: writing VBAR_EL1 is permitted at EL1 and the symbol
    // address is provided by the linker; the table is `.align 11`.
    unsafe {
        core::arch::asm!(
            "msr vbar_el1, {0}",
            "isb",
            in(reg) __exception_vectors as *const () as usize,
            options(nomem, nostack, preserves_flags)
        );
    }
}

// ---------------------------------------------------------------------------
// Recoverable MMIO probe (optional / possibly-absent controllers)
// ---------------------------------------------------------------------------

/// Set while [`probe_read32`] is doing its single guarded MMIO read, so the
/// synchronous handler knows a Data Abort in that window is expected.
static PROBE_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Set by the handler when the guarded read took a Data Abort — the target
/// device did not respond.
static PROBE_FAULTED: AtomicBool = AtomicBool::new(false);

/// Read a 32-bit MMIO register that may belong to a **possibly-absent**
/// controller, surviving a synchronous external abort. Returns `Some(value)`
/// if the read completed, or `None` if the access faulted — i.e. nothing
/// responds at `va` (e.g. the BCM2711 PCIe root complex under QEMU `raspi4b`,
/// which is not modelled). EL1, single-CPU: the read is a single 4-byte load,
/// and on a Data Abort the handler advances `ELR` past it and sets
/// [`PROBE_FAULTED`]. `va` must be a mapped device VA (a translation fault is
/// **not** recovered — only external aborts on a mapped MMIO page).
pub fn probe_read32(va: u64) -> Option<u32> {
    PROBE_FAULTED.store(false, Ordering::SeqCst);
    PROBE_ACTIVE.store(true, Ordering::SeqCst);
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
    // SAFETY: `va` is a mapped device VA; a fault here is caught and recovered
    // by `rust_sync_handler` (advances ELR, sets PROBE_FAULTED), so the read
    // never escalates to `unhandled`. The loaded value is ignored on fault.
    let v = unsafe { core::ptr::read_volatile(va as *const u32) };
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
    PROBE_ACTIVE.store(false, Ordering::SeqCst);
    if PROBE_FAULTED.load(Ordering::SeqCst) {
        None
    } else {
        Some(v)
    }
}

// ---------------------------------------------------------------------------
// Rust-level handlers (called from exception_vectors.s)
// ---------------------------------------------------------------------------

/// Synchronous-exception handler. Decodes ESR_EL1; if it is an SVC
/// from a recognized class, dispatches into `tos-api`. Updates
/// `frame.gpr[0]` so the assembly stub returns the value in `x0`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_sync_handler(frame: &mut ExceptionFrame) {
    sync_handler(frame);
    // ARCH03 3b: whatever the call changed in memory ownership is in the
    // page tables before the caller (or whoever runs next) continues. A
    // no-op with protection off or nothing pending.
    // `sync_now`: even if a batch were still open (MEMPROT-FAST), nothing
    // runs at EL0 before the tables are in line.
    // SAFETY: trap context, single CPU, BDOS giant lock implicit.
    unsafe { tos_api::bdos::process::memprot::sync_now() };
    // MEMPROT-FAST verification mode (`memprot=verify`): differences found by
    // the full recomputation, and the counters after a process ended.
    while let Some(r) = tos_api::bdos::process::memprot::take_verify_report() {
        report_memprot_verify(r);
    }
    // ARCH03 3c: a process whose rights could not get a private page table
    // (the arena is exhausted) keeps the non-owner rights there — it can lose
    // access, never gain any. Not silent: one serial line per process.
    while let Some(r) = tos_api::bdos::process::memprot::take_shortfall_report() {
        crate::kprintln!(
            "[memprot] page-table arena exhausted: process slot {} (PD 0x{:08x}) keeps non-owner rights in {} region(s); shortfalls since boot {}",
            r.slot,
            r.pd,
            r.of_process,
            r.total,
        );
    }
}

/// One serial line of the memory-protection verification mode
/// (MEMPROT-FAST, boot argument `memprot=verify`).
fn report_memprot_verify(r: tos_api::bdos::process::memprot::VerifyReport) {
    use tos_api::bdos::process::memprot::VerifyReport;
    match r {
        VerifyReport::Mismatch(m) => crate::kprintln!(
            "[memprot] VERIFY MISMATCH {:?} slot {} va 0x{:08x} expected 0x{:016x} table 0x{:016x}",
            m.kind,
            m.slot.map_or(-1, |s| s as i64),
            m.va,
            m.expected,
            m.actual,
        ),
        VerifyReport::Stats(s) => crate::kprintln!(
            "[memprot] verify: {} checks, {} pages compared, {} mismatches, {} tolerated shortfalls; syncs {} regions {} pages {} rebuilds {} fallbacks {}",
            s.verified,
            s.verified_pages,
            s.mismatches,
            s.tolerated,
            s.syncs,
            s.regions,
            s.pages,
            s.rebuilds,
            s.fallbacks,
        ),
    }
}

/// The body of [`rust_sync_handler`].
fn sync_handler(frame: &mut ExceptionFrame) {
    let ec = esr_ec(frame.esr);
    if ec != EC_SVC_AARCH64 && ec != EC_SVC_FROM_EL1 {
        // Fault-safe user access (ARCH03 0c, `tos_api::uaccess`): a same-EL
        // Data Abort on one of the listed `ldtr*`/`sttr*` instructions is a
        // kernel access to a bad *user* pointer, not a kernel bug. Record it
        // and resume at the instruction's fixup, which returns a failure to
        // the Rust caller; the syscall tail then terminates the calling
        // program like EmuTOS does (`complete_call_tail`). The fault is
        // identified by the PC alone (0b §5: `ELR` = the `ldtr`/`sttr`).
        if ec == EC_DABT_SAME {
            if let Some(fixup) = tos_api::uaccess::fixup_for(frame.elr) {
                let mut regs = [0u64; 31];
                regs[..30].copy_from_slice(&frame.gpr);
                regs[30] = frame.lr;
                tos_api::uaccess::note_fault(&tos_api::uaccess::FaultRecord {
                    far: read_far_el1(),
                    esr: frame.esr,
                    pc: frame.elr,
                    pstate: frame.spsr,
                    regs,
                });
                frame.elr = fixup;
                return;
            }
        }
        // Recoverable MMIO probe: a same-EL Data Abort while a `probe_read32`
        // is in flight means the probed device did not respond. Flag it and
        // step `ELR` past the faulting load (all A64 instructions are 4 bytes)
        // so the probe returns `None` instead of panicking. Every other abort
        // still falls through to `unhandled`.
        if ec == EC_DABT_SAME && PROBE_ACTIVE.load(Ordering::SeqCst) {
            PROBE_FAULTED.store(true, Ordering::SeqCst);
            frame.elr = frame.elr.wrapping_add(4);
            return;
        }
        // A synchronous exception taken from an EL0 program (abort,
        // undefined instruction, alignment, trapped system instruction, …)
        // ends that program only — EmuTOS `any_vec` → `dopanic` →
        // `kill_program` (`bios/vectors.S`, `bios/kprint.c:302-596`,
        // `bios/panicasm.S:96-99`). EL1 exceptions stay fatal.
        if frame.spsr & 0x1F == 0 && kill_crashed_program(frame) {
            return;
        }
        unhandled(frame, "synchronous (non-SVC)");
    }

    // A user-access fault recorded during an earlier call must have been
    // consumed by that call's tail; one still pending here belongs to a call
    // that left through a path without the tail (yield/restart). It cannot be
    // attributed to this call, so drop it — loudly.
    if let Some(stale) = tos_api::uaccess::take_pending_fault() {
        crate::kprintln!(
            "[uaccess] stale fault dropped: far=0x{:x} pc=0x{:x} esr=0x{:x}",
            stale.far, stale.pc, stale.esr,
        );
    }

    let svc_imm = (esr_iss(frame.esr) & 0xFFFF) as u16;

    // Phase 05 EC4 sentinel — redirect a from-EL0 `svc #0xEC4` back
    // to the EL1 resume PC captured by `selftest_ec4`. The redirect
    // is gated on `EC4_TEST_RUNNING` so a stray sentinel SVC outside
    // the test window is harmless. The redirect target lives in
    // `PROCESS_TABLE[EC4_SENTINEL_SLOT].parent_frame`, populated by
    // `enter_el0_with_resume_capture` just before the `eret`-to-EL0.
    if svc_imm == crate::selftest_ec4::EC4_SENTINEL_SVC_IMM
        && crate::selftest_ec4::EC4_TEST_RUNNING.load(Ordering::Acquire)
    {
        // Capture diagnostic state for the test driver to assert on.
        crate::selftest_ec4::EC4_CAPTURED_SP_EL0
            .store(frame.sp_el0, Ordering::Release);
        crate::selftest_ec4::EC4_CAPTURED_ELR
            .store(frame.elr, Ordering::Release);
        crate::selftest_ec4::EC4_CAPTURED_SPSR_M
            .store(frame.spsr & 0xF, Ordering::Release);

        // Mark test inactive *before* the redirect so a logic bug
        // that re-fires the sentinel doesn't recurse.
        crate::selftest_ec4::EC4_TEST_RUNNING
            .store(false, Ordering::Release);

        // Redirect: rewrite frame so the post-handler `eret` lands
        // at the EL1 resume PC, on the original kernel SP_EL0.
        let slot_idx = crate::selftest_ec4::EC4_SENTINEL_SLOT
            .load(Ordering::Acquire);
        let pf = &tos_api::bdos::process::PROCESS_TABLE[slot_idx]
            .parent_frame;
        let resume_pc = pf.elr().load(Ordering::Acquire);
        let kernel_sp = pf.sp_el0().load(Ordering::Acquire);
        frame.elr = resume_pc;
        // Restore the kernel's SP_EL0 (the user-stack capture above
        // already saved the diagnostic value); the asm restore will
        // write this back into `SP_EL0`, undoing the user-stack
        // overwrite the test installed before `eret`-ing to EL0.
        frame.sp_el0 = kernel_sp;
        // SPSR.M[3:0] = 0b0100 = EL1t (EL1, SP_EL0 active). Kernel
        // resumes on its own stack.
        frame.spsr = 0b0100;
        return;
    }

    let class = match TrapClass::from_imm(svc_imm) {
        Some(c) => c,
        None => {
            // Unknown trap class → EINVFN.
            frame.gpr[0] = tos_types::error::EINVFN as i64 as u64;
            return;
        }
    };

    // GEMDOS Pterm (0x4C) / Pterm0 (0x00) / Ptermres (0x31) and Pexec
    // (0x4B) need the trap frame mutated rather than just an `x0`
    // return — the dispatcher table can't express that, so we
    // special-case them here.
    if matches!(class, TrapClass::Gemdos) {
        let func = frame.gpr[8] as u16;
        // GEMDOS trap tracer: the frame-mutating calls below never reach
        // `gemdos::dispatch`, so the dispatcher hook cannot see them — and a
        // file-system trace that hides process creation and exit is missing the
        // frame around everything else. Recorded here with their arguments but
        // no result (they return by rewriting the trap frame). The opcode list
        // must stay exactly the set special-cased below, or every ordinary
        // GEMDOS call would be recorded twice. Folds away without
        // `gemdos-trace`. `Pexec` (0x4B) is recorded inside `handle_pexec`
        // instead, where the program path has already been read out of the
        // caller's address space.
        if tos_api::gemdos::trace::ENABLED
            && matches!(func, 0x4C | 0x00 | 0x31 | 0x4D | 0x11A | 0x20)
        {
            let regs = frame_arg_regs(frame);
            tos_api::gemdos::trace::note_frame_call(func, &regs, &[], &[]);
        }
        if func == 0x4C || func == 0x00 || func == 0x31 {
            handle_pterm(frame, func);
            return;
        }
        if func == 0x4B {
            handle_pexec(frame);
            return;
        }
        if func == 0x4D {
            handle_pwait(frame);
            return;
        }
        // Psigreturn (0x11A) — the EL0 upcall trampoline's return syscall
        // (cooperative-kernel building block 1, design §5). Frame-mutating,
        // so it branches here before the leaf table exactly like
        // Pexec/Pterm/Pwait; it is also > MAX_FN (0x57) and would otherwise
        // fall through to `EINVFN`.
        if func == 0x11A {
            handle_psigreturn(frame);
            return;
        }
        // Super (0x20) — the super state (MEM12, `docs/api/_super-state.md`
        // S3–S5). Needs the caller's `SP_EL0` for its token, which the leaf
        // table cannot see; EmuTOS too answers it in the TRAP #1 front end,
        // before the BDOS table (`bdos/rwa.S:159-184`).
        if func == 0x20 {
            handle_super(frame);
            return;
        }
    }

    // Supexec (XBIOS 0x26) from an EL0 program — run the callback at EL0 in
    // super state as an upcall (MEM12 S6); `Psigreturn` hands its `x0` back.
    // An EL1 caller keeps the dispatcher's answer for an absent slot.
    if matches!(class, TrapClass::Xbios)
        && frame.gpr[8] as u16 == 0x26
        && frame.spsr & 0xF == 0
        && handle_supexec(frame)
    {
        return;
    }

    let args = TrapArgs {
        regs: [
            frame.gpr[0],
            frame.gpr[1],
            frame.gpr[2],
            frame.gpr[3],
            frame.gpr[4],
            frame.gpr[5],
            frame.gpr[6],
            frame.gpr[7],
        ],
        func: frame.gpr[8],
        svc_imm,
        // ARCH03 stage 3a: whose address space the pointer arguments are in —
        // an EL0 program's (accessed with its rights, `tos_api::uaccess`) or
        // an EL1 caller's kernel addresses (the kernel's own `svc`s, e.g.
        // EmuCON's GEMDOS calls).
        space: tos_api::uaccess::Space::from_spsr(frame.spsr),
    };

    // `[desk] desktop up`: the kernel reports the desktop program's first
    // `evnt_multi` (DESK43 D4 — the desktop itself prints nothing). One atomic
    // load unless the desktop is running; see `shell_loop`.
    if matches!(class, TrapClass::Gem) {
        crate::shell_loop::observe_desktop_gem_call(&args);
    }

    // TEMPORARY `svc` tracer (debug tooling, not product history). It records
    // **every** `svc`, whichever exception level it came from. This gate also
    // tested `SPSR.M[3:0] == 0` until DESK43/U1, on the stated grounds that
    // "the desktop runs at EL1 and never reaches here"; those grounds were
    // wrong from DESK10 on — the then built-in desktop issued the same two
    // instructions an application does (`svc #2` and `svc #1` through the
    // `newtos_gem` bindings, DESK43 U6b), so an EL0-only gate silently
    // excluded the one caller whose call sequence has to be diffable against
    // `desk/deskmain.c` (`docs/analysis/desktop-userspace-plan-v2.md` §5 U1 and
    // measure 4 of §6.4). `pre_dispatch` flushes the RAM trace to C: at each
    // `evnt_multi` entry; `post_dispatch` records the completed call. Both fold
    // away when `SVC_TRACE` is false (no QED mode, no `svc-seq-trace`), so this
    // is a no-op with the tool disabled — which is its committed state.
    if crate::dbg_qedtrace::SVC_TRACE {
        crate::dbg_qedtrace::pre_dispatch(class, &args);
    }

    // A blocking `evnt_*` AES wait may yield to a runnable peer (S2
    // restartable block, design §4.3 + §8.2 sub-step 3); the AES dispatch
    // returns the `Switch` to yield to, which is applied after the match below.
    let mut pending_switch: Option<tos_api::bdos::process::sched::Switch> = None;

    // A synchronous AES `G_USERDEF` callout may stall the AES fiber mid-draw
    // (AES-as-thread design §3.4(c)); the dispatch returns a `Stalled`
    // descriptor, delivered as an EL0 callout after the match below.
    let mut pending_stall: Option<tos_api::aes::thread::UserdefStall> = None;

    // Result is the full 64-bit `x0` value: GEMDOS returns a sign-
    // extended `Long`, but BIOS `Getbpb` returns a native 64-bit pointer.
    let result: i64 = match class {
        TrapClass::Gemdos => tos_api::gemdos::dispatch(&args) as i64,
        // GEM trap (#2): shared by the VDI and the AES. `x0` is the
        // selector (115 = VDI, 200 = AES), `x1` the parameter block.
        // The VDI returns `flip_y` in `x0` — EmuTOS `vdi_asm.S:124-125`
        // zero-extends the WORD into `d0`; the AES returns a hard 0
        // (`aes/gemsuper.c:454-466`), its contract return being `int_out[0]`.
        //
        // The running process's ABI (ADR-020) tells the readers whether
        // `x1` addresses a block of 4-byte (ILP32) or 8-byte (LP64)
        // pointers; an in-kernel (EL1) caller has no user process and
        // resolves to LP64, keeping the native 64-bit selftest blocks.
        TrapClass::Gem => {
            let abi = tos_api::bdos::process::current_abi();
            /// The `d0 = -1` dispatcher-address query of the m68k GEM trap
            /// (`cmp.w #0xffff,d0`, EmuTOS `vdi_asm.S:40`), as a NewTOS
            /// selector: the sign-extended 64-bit `x0` a caller writing
            /// `long x0 = -1` produces.
            const GEM_SEL_ADDR_QUERY: u64 = -1i64 as u64;
            match args.regs[0] {
                // SAFETY: `x1` is the caller-supplied parameter block.
                115 => i64::from(unsafe { tos_api::vdi::dispatch_in(args.regs[1], abi, args.space) }),
                // SAFETY: `x1` is the caller-supplied parameter block. A
                // blocking `evnt_*` wait may return a peer Switch (S2). The
                // AES now runs on its dedicated EL1 fiber stack (step 1 of the
                // AES-as-thread design, docs/analysis/coop-kernel/
                // 30-aes-thread-design.md §3.4(a)); `call` is a byte-identical
                // stack move over `dispatch` — no behaviour change in step 1.
                200 => {
                    // SAFETY: `x1` is the caller-supplied parameter block.
                    match unsafe { tos_api::aes::thread::call(args.regs[1], abi, args.space) } {
                        tos_api::aes::thread::AesCallOutcome::Complete(sw) => {
                            pending_switch = sw;
                        }
                        // The draw stalled at an app `G_USERDEF`: run its EL0
                        // `ub_code` via the callout stub, resume the fiber on
                        // `Psigreturn` (design §3.4(c)/(e)).
                        tos_api::aes::thread::AesCallOutcome::Stalled(desc) => {
                            pending_stall = Some(desc);
                        }
                    }
                    0
                }
                // Selector −1 — "give me the jsr-able address of the VDI
                // dispatcher" (EmuTOS `move.l #_GSX_ENTRY,d0`,
                // `vdi_asm.S:51-52`). **Documented NewTOS deviation**
                // (docs/COMPATIBILITY.md, "GEM trap selector −1"): there is
                // no such entry here — the VDI is reachable only through
                // `svc #2`, and the dispatcher is EL1 code an EL0 caller can
                // neither branch to nor read. 0 is the honest answer, and the
                // value a conforming caller tests for before using it. It is
                // spelled out as its own arm so it does not silently inherit
                // the "leave `x0` alone" rule below, which would hand back
                // −1 — a value that *looks* like an address.
                GEM_SEL_ADDR_QUERY => 0,
                // Every other selector — including the `−2` GDOS-absence
                // probe behind `vq_gdos()`/`vq_vgdos()` — must come back with
                // `x0` **exactly as the caller set it**: EmuTOS falls through
                // to a bare `rte` that never writes `d0`
                // (`vdi_asm.S:40-49`, *"Just like Atari TOS, we leave d0
                // unchanged, indicating the absence of GDOS if this is called
                // with -2 in the low-order word of d0."*); spec
                // docs/api/vdi/_dispatcher.md, "`d0` special values".
                //
                // We cannot skip the write-back — `complete_call_tail` stores
                // this `result` into `x0` for every trap class — so we return
                // `x0`'s own bits instead. `frame.gpr[0]` is untouched between
                // entry and here, so the store is `x0 = x0`: bit-exact, and it
                // leaves the other arms' return path alone.
                _ => frame.gpr[0] as i64,
            }
        }
        // BIOS (Trap 13): block-device + console functions.
        TrapClass::Bios => tos_api::bios::dispatch(&args),
        // XBIOS (Trap 14): screen group (Physbase/Logbase/Getrez/...).
        TrapClass::Xbios => tos_api::xbios::dispatch(&args),
    };

    // Mid-syscall cooperative yield (S2 restartable block, design §4.3 + §8.2
    // sub-step 3): a blocking `evnt_*` wait found a runnable peer and returned
    // the Switch to yield to. Unlike the leaf `Pwait`, `sleep` sits deep inside
    // `evnt_multi`, so the yield propagates up here via the AES dispatch return
    // value. Save this process's frame with the ELR **rewound to the `svc`**
    // (`save_yield_frame(.., true)`) so the post-handler `eret` re-executes the
    // `svc` on resume — the syscall re-enters, re-checks its predicate, and this
    // time fires — then switch to the peer. This returns early, skipping the
    // result-write + terminate/upcall tails below: nothing is committed for the
    // (restarting) call, the yield path pumped no input so no Ctrl-C terminate
    // can be pending, and a pending upcall for *this* process is delivered on
    // its own resume (deliver-on-resume, §6), not now.
    //
    // **The EL1 caller (ACC03).** When the blocking client is an EL1 kernel
    // context such as the boot path — `RUN` names no slot — there is no
    // `saved_frame` to write and no run-set entry to drop; the frame is parked
    // in [`KERNEL_PARK`] instead, with the same ELR rewind, and restored the
    // moment the accessory it yields to blocks again.
    // A112: the dispatch a *completed* AES call ends with found a ready peer
    // (`Switch::completed`). Not an abort: the call is finished below like any
    // other, and the leaf yield happens after the completion tail.
    let mut post_call_switch: Option<tos_api::bdos::process::sched::Switch> = None;
    if let Some(sw) = pending_switch.take_if(|sw| sw.completed) {
        post_call_switch = Some(sw);
    }

    if let Some(sw) = pending_switch {
        // KEYS02 (O1): not a switch — the blocking wait gave way because this
        // process's `kbdvec` hook has bytes queued. Abort the call like an S2
        // yield (rewind to the `svc`, commit nothing) and run the hook on top
        // of the rewound frame; its `Psigreturn` re-executes the `svc`.
        if sw.upcall_restart {
            restart_for_kbdvec(frame);
            return;
        }
        let cur = tos_api::bdos::process::sched::running_slot();
        if cur < tos_api::bdos::process::MAX_PROCESSES {
            save_yield_frame(cur, frame, true);
            tos_api::bdos::process::sched::mark_yielded(cur);
        } else if !sw.el1_internal {
            park_kernel_frame(frame, true);
            adopt_kernel_park_as_parent(sw.slot);
        }
        apply_switch(frame, &sw);
        return;
    }

    // A synchronous AES `G_USERDEF` callout stalled the fiber mid-draw
    // (AES-as-thread design §3.4(c)): capture the client's origin frame on the
    // first stall and redirect EL0 into the callout stub. Bails out of the
    // result-write tail exactly like the `pending_switch` yield — the original
    // `wind_*` call is not finished; its real result is committed only when the
    // fiber completes (on the userdef `Psigreturn`, `handle_psigreturn`).
    if let Some(desc) = pending_stall {
        deliver_userdef_stall(frame, desc);
        return;
    }

    // TEMPORARY `svc` tracer (debug tooling): record the completed call, from
    // either exception level (see the `pre_dispatch` hook above). Folds away
    // when `SVC_TRACE` is false.
    if crate::dbg_qedtrace::SVC_TRACE {
        crate::dbg_qedtrace::post_dispatch(class, &args, result);
    }

    // AES trap tracer: perform a pending trace flush here — on the kernel
    // exception stack, with the AES dispatch already returned — rather than
    // inside the AES, whose fiber stack (16 KiB) must not carry the FAT write
    // path. A no-op (one atomic swap) when nothing is pending, and compiled
    // out entirely without the `aes-trace` feature.
    tos_api::aes::trace::flush_if_requested();

    // GEMDOS trap tracer: same place, same reason (kernel exception stack, the
    // dispatch already returned). Deliberately called after *every* trap class,
    // not only GEMDOS: that is what lets the tracer notice that the application
    // has gone quiet — a GEM app's idle loop hammers the AES while the file
    // system does nothing, so a GEMDOS-only tail would never tick.
    tos_api::gemdos::trace::flush_if_requested();

    // KEYS02: a single-key blocking read (`Cconin`/`Cnecin`/`Crawcin`/
    // `Bconin(2)`) gave way because this process's `kbdvec` hook has bytes
    // queued (`Host::console_read_key_restartable`). Its `result` is a
    // placeholder: discard it, rewind to the `svc` and run the hook first.
    // Only an EL0 caller can have hook bytes; should an EL1 caller ever see
    // the flag, the read simply completes with its placeholder.
    if tos_api::xbios::kbdvecs::take_restart() && frame.spsr & 0xF == 0 {
        restart_for_kbdvec(frame);
        return;
    }

    if complete_call_tail(frame, result) {
        if let Some(sw) = post_call_switch {
            yield_after_completed_call(frame, &sw);
        }
    }
}

/// The leaf yield of a **completed** AES call (A112, `docs/api/aes/event.md`
/// "Wake conditions and dispatch" point 3): `super()`'s closing `dsptch()`
/// (`aes/gemsuper.c:454-466`) put the caller at the end of the ready list and
/// a ready peer runs first. The call's result is already in `frame` (`x0 = 0`
/// for the AES, `int_out` copied back), so the frame is saved **after** the
/// `svc` — the `Pwait` shape, `save_yield_frame(.., false)` — and on resume
/// `apply_switch` writes the same `x0 = 0` the completed call returned.
///
/// Only reached with the frame as the call's plain return (no terminate, no
/// fault, no upcall redirected it — [`complete_call_tail`] returned `true`);
/// a redirected frame keeps the CPU and the peer runs at the caller's next
/// dispatch.
fn yield_after_completed_call(
    frame: &mut ExceptionFrame,
    sw: &tos_api::bdos::process::sched::Switch,
) {
    let cur = tos_api::bdos::process::sched::running_slot();
    if cur >= tos_api::bdos::process::MAX_PROCESSES || frame.spsr & 0xF != 0 {
        return; // only an EL0 caller with a slot dispatches (`dispatch_after_call`)
    }
    save_yield_frame(cur, frame, false);
    tos_api::bdos::process::sched::mark_yielded(cur);
    apply_switch(frame, sw);
}

/// The seven GEMDOS argument registers (`x0..x6`) of a trap frame, in the
/// [`TrapArgs`] order, for the frame-mutating calls that never build a
/// `TrapArgs` of their own.
#[inline]
fn frame_arg_regs(frame: &ExceptionFrame) -> [u64; 7] {
    [
        frame.gpr[0],
        frame.gpr[1],
        frame.gpr[2],
        frame.gpr[3],
        frame.gpr[4],
        frame.gpr[5],
        frame.gpr[6],
    ]
}

/// Shared syscall-completion tail (AES-as-thread design §3.4(e), the extracted
/// `complete_aes_call`). Called by both the ordinary trap path and the
/// post-userdef-stall AES completion ([`handle_psigreturn`]) so the two share
/// byte-identical behaviour:
///
///  1. A blocking console input (Cconin/Cnecin/Cconrs) may have seen a Ctrl-C
///     and recorded a deferred `Pterm(-32)`; consume it and terminate exactly
///     like an explicit `Pterm`, discarding `result`.
///  2. Write the 64-bit `result` into `x0` (error codes already sign-extended;
///     `Getbpb` returns a full pointer; a completed AES call passes `0`).
///  3. On a return to EL0, deliver any pending EL0 upcall for the running
///     process (deliver-on-return, design §7.1 / building block 1 §6.2).
///
/// Returns `true` when `frame` is left as the call's **plain return** (`x0` =
/// `result`, nothing delivered on top) — the one state in which a completed
/// AES call may still yield to a peer (A112, [`yield_after_completed_call`]).
fn complete_call_tail(frame: &mut ExceptionFrame, result: i64) -> bool {
    // The call touched a bad user pointer (`tos_api::uaccess`): on EmuTOS the
    // bus error in the trap handler kills the caller (`any_vec` → `dopanic`
    // → `Pterm(-1)`, `run` still being the caller's PD, `bios/kprint.c:
    // 587-594`); the call's own result never reaches it. Takes precedence
    // over a deferred Ctrl-C terminate, which is then moot.
    if let Some(rec) = tos_api::uaccess::take_pending_fault() {
        let _ = tos_api::bdos::process::take_pending_terminate();
        uaccess_fault_tail(frame, &rec);
        return false;
    }
    if let Some(rc) = tos_api::bdos::process::take_pending_terminate() {
        terminate_and_redirect(frame, rc);
        return false;
    }

    frame.gpr[0] = result as u64;

    if frame.spsr & 0xF == 0 {
        if let Some(slot_idx) = tos_api::bdos::process::slot_for(
            tos_api::bdos::process::current(),
        ) {
            if let Some(u) = tos_api::bdos::process::take_pending_upcall(slot_idx)
            {
                deliver_upcall(frame, u);
                return false;
            }
        }
        // Otherwise the next byte for the process's `kbdvec` hook (KEYS02).
        if deliver_pending_kbdvec(frame) {
            return false;
        }
    }
    true
}

/// Abort the current call for a `kbdvec` delivery (KEYS02, O1): rewind `elr`
/// to the `svc` — the argument registers are still the caller's, nothing was
/// committed — and deliver the hook's next byte on top of that frame. The
/// hook's `Psigreturn` restores the rewound frame and the `svc` re-executes.
/// If the ring turns out empty, the `svc` simply re-executes.
fn restart_for_kbdvec(frame: &mut ExceptionFrame) {
    frame.elr = frame.elr.wrapping_sub(4);
    deliver_pending_kbdvec(frame);
}

/// Deliver the next byte queued for the running process's `kbdvec` hook as an
/// EL0 upcall (KEYS02, `docs/api/xbios/22-kbdvbase.md` B4): `x0` = the
/// scancode (bit 7 = break), `x1` = 0 (D2/D2a), handler = the **current**
/// `kbdvec` slot, return through the trampoline's `Psigreturn`, which comes
/// back here for the next byte — one byte per upcall, in arrival order.
///
/// Only on a return to EL0, and never while a `kbdvec` upcall is already
/// running for this process ([`UPCALL_MASK_KBDVEC`], nesting depth 1): a
/// syscall the hook itself makes — e.g. the `kbdvec_default` stub's — returns
/// normally. A byte whose process put the default back before it could be
/// delivered is decoded directly, like on the fast path. Returns whether an
/// upcall was delivered.
///
/// [`UPCALL_MASK_KBDVEC`]: tos_api::bdos::process::UPCALL_MASK_KBDVEC
fn deliver_pending_kbdvec(frame: &mut ExceptionFrame) -> bool {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{
        current, slot_for, Disposition, UpcallDesc, PROCESS_TABLE, UPCALL_MASK_KBDVEC,
    };
    use tos_api::xbios::kbdvecs;

    if frame.spsr & 0xF != 0 {
        return false;
    }
    let Some(slot) = slot_for(current()) else {
        return false;
    };
    // D9 (MEM12): an accessory's hook is never called — say so, once.
    if kbdvecs::accessory_hook_unwarned(slot) {
        crate::kprintln!(
            "[kbdvec] warning: desk accessory in slot {} installed a kbdvec hook; it is never called \
             (keys go to the innermost started program, docs/api/xbios/22-kbdvbase.md Teil D, D9)",
            slot
        );
    }
    if PROCESS_TABLE[slot].upcall_mask.load(Ordering::Acquire) & UPCALL_MASK_KBDVEC != 0 {
        return false;
    }
    while let Some(byte) = kbdvecs::pop(slot) {
        let handler = kbdvecs::kbdvec_of(slot).unwrap_or(kbdvecs::KBDVEC_DEFAULT_VA);
        if handler == kbdvecs::KBDVEC_DEFAULT_VA {
            crate::ikbd::kbd_int(byte);
            continue;
        }
        // The hook runs in super state, as EmuTOS calls `kbdvec` from the
        // IKBD interrupt in supervisor mode (`bios/aciavecs.S:405-414`;
        // MEM12 S11, O1); `Psigreturn` puts the state back.
        let prev_super = tos_api::bdos::process::superstate::is_super(slot);
        // SAFETY: trap context, BDOS giant lock held implicitly; the slot is
        // the running process, its root built.
        unsafe { tos_api::bdos::process::superstate::set_super(slot, true) };
        deliver_upcall(
            frame,
            UpcallDesc {
                handler_va: handler,
                args: [u64::from(byte), 0, 0, 0],
                mask_bit: UPCALL_MASK_KBDVEC,
                disposition: Disposition::ResumeSuper { prev_super },
            },
        );
        return true;
    }
    false
}

/// First half of an AES userdef callout (AES-as-thread design §3.4(c)): on the
/// **first** stall of the current AES call — detected by
/// [`AES_CALL_ORIGIN_SLOT`] still being [`AES_ORIGIN_NONE`] — snapshot the
/// unchanged client trap frame into [`AES_CALL_ORIGIN`], then deliver the EL0
/// callout stub. `deliver_userdef_upcall` mutates the frame (redirect), so the
/// origin is captured first.
///
/// [`AES_CALL_ORIGIN`]: tos_api::aes::thread::AES_CALL_ORIGIN
/// [`AES_CALL_ORIGIN_SLOT`]: tos_api::aes::thread::AES_CALL_ORIGIN_SLOT
/// [`AES_ORIGIN_NONE`]: tos_api::aes::thread::AES_ORIGIN_NONE
fn deliver_userdef_stall(
    frame: &mut ExceptionFrame,
    desc: tos_api::aes::thread::UserdefStall,
) {
    use core::sync::atomic::Ordering;

    if tos_api::aes::thread::AES_CALL_ORIGIN_SLOT.load(Ordering::Acquire)
        == tos_api::aes::thread::AES_ORIGIN_NONE
    {
        let slot = tos_api::bdos::process::slot_for(
            tos_api::bdos::process::current(),
        )
        .unwrap_or(0);
        save_call_origin(frame, slot);
    }
    deliver_userdef_upcall(frame, desc);
}

/// Redirect the frame into the EL0 `G_USERDEF` callout stub with the app's
/// `ub_code` / `PARMBLK` VAs as arguments (design §3.4(c)/(e)). Reuses
/// [`deliver_upcall`] unchanged; the [`Disposition::UserdefReturn`] tells
/// [`handle_psigreturn`] to resume the AES fiber instead of the interrupted
/// (stub) context.
///
/// [`Disposition::UserdefReturn`]: tos_api::bdos::process::Disposition::UserdefReturn
fn deliver_userdef_upcall(
    frame: &mut ExceptionFrame,
    desc: tos_api::aes::thread::UserdefStall,
) {
    deliver_upcall(
        frame,
        tos_api::bdos::process::UpcallDesc {
            handler_va: tos_api::bdos::process::loader::USERDEF_TRAMPOLINE_VA,
            args: [desc.ub_code_va, desc.parmblk_ptr, 0, 0],
            mask_bit: tos_api::bdos::process::UPCALL_MASK_AES_USERDEF,
            disposition: tos_api::bdos::process::Disposition::UserdefReturn,
        },
    );
}

/// Snapshot the client's trap frame into [`AES_CALL_ORIGIN`] and record its
/// slot (design §3.4(c)). The mirror of [`save_yield_frame`], but into the
/// global AES-call-origin cell rather than a per-process slot.
///
/// [`AES_CALL_ORIGIN`]: tos_api::aes::thread::AES_CALL_ORIGIN
fn save_call_origin(frame: &ExceptionFrame, slot: usize) {
    use core::sync::atomic::Ordering;

    let o = &tos_api::aes::thread::AES_CALL_ORIGIN;
    for i in 0..30 {
        o.gpr(i).store(frame.gpr[i], Ordering::Release);
    }
    o.lr().store(frame.lr, Ordering::Release);
    o.elr().store(frame.elr, Ordering::Release);
    o.spsr().store(frame.spsr, Ordering::Release);
    o.sp_el0().store(frame.sp_el0, Ordering::Release);
    tos_api::aes::thread::AES_CALL_ORIGIN_SLOT.store(slot, Ordering::Release);
}

/// Restore the client's trap frame from [`AES_CALL_ORIGIN`] (captured by
/// [`save_call_origin`]) and clear the origin (design §3.4(e)).
///
/// [`AES_CALL_ORIGIN`]: tos_api::aes::thread::AES_CALL_ORIGIN
fn restore_call_origin(frame: &mut ExceptionFrame) {
    use core::sync::atomic::Ordering;

    let o = &tos_api::aes::thread::AES_CALL_ORIGIN;
    for i in 0..30 {
        frame.gpr[i] = o.gpr(i).load(Ordering::Acquire);
    }
    frame.lr = o.lr().load(Ordering::Acquire);
    frame.elr = o.elr().load(Ordering::Acquire);
    frame.spsr = o.spsr().load(Ordering::Acquire);
    frame.sp_el0 = o.sp_el0().load(Ordering::Acquire);
    o.clear();
    tos_api::aes::thread::AES_CALL_ORIGIN_SLOT
        .store(tos_api::aes::thread::AES_ORIGIN_NONE, Ordering::Release);
}

/// Snapshot `frame` into slot `cur`'s `saved_frame` so the scheduler can
/// resume it, optionally rewinding `elr` to the `svc` instruction. Shared by
/// the leaf `Pwait` yield and the mid-syscall `sleep` yield.
///
///   * `rewind = false` — the leaf `Pwait`: `saved_frame.elr` is the address
///     *after* the `svc`, so the resume returns from the syscall (and
///     `apply_switch` writes the `Pwait` return value `gpr[0] = 0`).
///   * `rewind = true` — the **S2 restartable block** (design §4.3): the
///     `saved_frame.elr` is rewound by one instruction (`elr − 4`) so the
///     post-handler `eret` **re-executes the `svc`** on resume; the syscall
///     re-enters from the top, re-checks its predicate, and fires.
///
/// **S2 needs TWO deltas, not the one the design §4.3 first claimed.** The
/// ELR-rewind here is the first. The second is on the resume side: a rewound
/// `svc #2` (the GEM trap) must re-execute with its original `x0` (the VDI/AES
/// selector), but `apply_switch`'s resume branch would otherwise clobber `x0`
/// to 0 (the `Pwait` return). This helper records `rewind` as the slot's
/// restart-kind via [`sched::set_resume_restartable`]; [`apply_switch`] reads
/// it through [`Switch::restartable`] and skips that clobber when set. The
/// design doc §4.3 is corrected accordingly.
///
/// [`Switch::restartable`]: tos_api::bdos::process::sched::Switch::restartable
/// Save the live FP/SIMD register file (`v0..v31`, `FPCR`, `FPSR`) into `dst`
/// — the vector half of a yielding EL0 context (ACC03, see
/// [`tos_api::bdos::process::FpContext`]).
///
/// The kernel itself is built for `aarch64-unknown-none-softfloat` and never
/// touches these registers, so the values in them at an `svc` from EL0 are
/// still the caller's.
fn save_fp_context(dst: &tos_api::bdos::process::FpContext) {
    let p = dst.ptr();
    // SAFETY: `p` addresses `FP_CONTEXT_LEN` (66) contiguous `u64`s — 512
    // bytes of vector state followed by the two 32-bit status words. FP/SIMD
    // is enabled at EL1 (`CPACR_EL1.FPEN = 0b11`, `boot.rs:296-313`).
    unsafe {
        core::arch::asm!(
            // The kernel builds for `aarch64-unknown-none-softfloat`, which
            // turns the FP/SIMD *assembler* feature off; re-enable it for this
            // block only and put it back afterwards, so nothing else in the
            // translation unit can accidentally emit vector code.
            ".arch armv8-a+fp+simd",
            "stp q0, q1, [{p}, #0x000]",
            "stp q2, q3, [{p}, #0x020]",
            "stp q4, q5, [{p}, #0x040]",
            "stp q6, q7, [{p}, #0x060]",
            "stp q8, q9, [{p}, #0x080]",
            "stp q10, q11, [{p}, #0x0a0]",
            "stp q12, q13, [{p}, #0x0c0]",
            "stp q14, q15, [{p}, #0x0e0]",
            "stp q16, q17, [{p}, #0x100]",
            "stp q18, q19, [{p}, #0x120]",
            "stp q20, q21, [{p}, #0x140]",
            "stp q22, q23, [{p}, #0x160]",
            "stp q24, q25, [{p}, #0x180]",
            "stp q26, q27, [{p}, #0x1a0]",
            "stp q28, q29, [{p}, #0x1c0]",
            "stp q30, q31, [{p}, #0x1e0]",
            "mrs {t}, fpcr",
            "str {t}, [{p}, #0x200]",
            "mrs {t}, fpsr",
            "str {t}, [{p}, #0x208]",
            ".arch armv8-a+nofp+nosimd",
            p = in(reg) p,
            t = out(reg) _,
            options(nostack),
        );
    }
}

/// Restore an FP/SIMD register file saved by [`save_fp_context`].
fn restore_fp_context(src: &tos_api::bdos::process::FpContext) {
    let p = src.ptr();
    // SAFETY: as `save_fp_context`.
    unsafe {
        core::arch::asm!(
            ".arch armv8-a+fp+simd",
            "ldp q0, q1, [{p}, #0x000]",
            "ldp q2, q3, [{p}, #0x020]",
            "ldp q4, q5, [{p}, #0x040]",
            "ldp q6, q7, [{p}, #0x060]",
            "ldp q8, q9, [{p}, #0x080]",
            "ldp q10, q11, [{p}, #0x0a0]",
            "ldp q12, q13, [{p}, #0x0c0]",
            "ldp q14, q15, [{p}, #0x0e0]",
            "ldp q16, q17, [{p}, #0x100]",
            "ldp q18, q19, [{p}, #0x120]",
            "ldp q20, q21, [{p}, #0x140]",
            "ldp q22, q23, [{p}, #0x160]",
            "ldp q24, q25, [{p}, #0x180]",
            "ldp q26, q27, [{p}, #0x1a0]",
            "ldp q28, q29, [{p}, #0x1c0]",
            "ldp q30, q31, [{p}, #0x1e0]",
            "ldr {t}, [{p}, #0x200]",
            "msr fpcr, {t}",
            "ldr {t}, [{p}, #0x208]",
            "msr fpsr, {t}",
            ".arch armv8-a+nofp+nosimd",
            p = in(reg) p,
            t = out(reg) _,
            options(nostack),
        );
    }
}

fn save_yield_frame(cur: usize, frame: &ExceptionFrame, rewind: bool) {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{sched, PROCESS_TABLE};

    let sf = &PROCESS_TABLE[cur].saved_frame;
    for i in 0..30 {
        sf.gpr(i).store(frame.gpr[i], Ordering::Release);
    }
    sf.lr().store(frame.lr, Ordering::Release);
    let elr = if rewind { frame.elr.wrapping_sub(4) } else { frame.elr };
    sf.elr().store(elr, Ordering::Release);
    sf.spsr().store(frame.spsr, Ordering::Release);
    sf.sp_el0().store(frame.sp_el0, Ordering::Release);
    // The vector half of the context (ACC03): `ExceptionFrame` carries only
    // the GPRs, and a hard-float EL0 program scheduled in between would
    // otherwise leave its own `v0..v31` behind for this slot to resume on.
    save_fp_context(&PROCESS_TABLE[cur].saved_fp);
    // Record how this slot yielded so `switch_to` carries it into the resume:
    // a rewound (restartable) frame must have `x0` preserved on resume.
    sched::set_resume_restartable(cur, rewind);
}

// ─── The kernel park (ACC03) ────────────────────────────────────────────────
//
// NewTOS's boot path is an AES client that runs at EL1 and owns no process
// slot (as the built-in desktop did until DESK43 U10; the desktop is now the
// EL0 process `NEWDESK` with a slot of its own), so it has no slot on the
// ready list the way EmuTOS's pid-0 AESPD has (`aes/geminit.c:851-877` gives
// it slot 0 in the same round-robin as the accessories); the scheduler keeps
// it as one extra entry of that list while it is parked (A115,
// `sched::pick_peer_for_kernel`). To let an accessory run *beside* such an EL1 client,
// the EL1 client's trap frame is snapshotted here — the "kernel park" — the
// accessory is switched to, and the park is restored the moment the accessory
// blocks again. One slot is enough: exactly one EL1 context can be inside a
// trap at a time (single CPU, no re-entrant EL1 trap), which is the same
// invariant the AES fiber relies on (`aes/thread.rs`, design §3.3).

/// The parked EL1 kernel client's full trap frame (`x0..x29`, `lr`, `elr`,
/// `spsr`, `sp_el0`). Armed together with [`tos_api::bdos::process::sched::set_kernel_parked`].
static KERNEL_PARK: tos_api::bdos::process::ParentFrame =
    tos_api::bdos::process::ParentFrame::zeroed();

/// The BDOS running-basepage (`RUN`) the parked EL1 client had — normally
/// null, but a kernel context that launched a program and is inside its own
/// nested AES call must come back to the same value.
static KERNEL_PARK_RUN: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

/// Restart-kind of the park, the EL1 twin of [`sched::Switch::restartable`]:
/// `true` when [`park_kernel_frame`] rewound `elr` to the `svc` (a blocking
/// `evnt_*`, which must re-execute and re-check on resume), `false` for the
/// leaf `Pwait` park (which resumes *after* the `svc` with `x0 = 0`).
///
/// [`sched::Switch::restartable`]: tos_api::bdos::process::sched::Switch::restartable
static KERNEL_PARK_RESTARTABLE: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// Snapshot the EL1 kernel client's trap `frame` into [`KERNEL_PARK`] and arm
/// it, so a later [`restore_kernel_park`] brings this context back exactly
/// here. `rewind` has the same meaning as in [`save_yield_frame`]: `true`
/// rewinds `elr` by one instruction so the `svc` re-executes (the S2
/// restartable block for a blocking `evnt_*`), `false` resumes after it.
fn park_kernel_frame(frame: &ExceptionFrame, rewind: bool) {
    use core::sync::atomic::Ordering;

    for i in 0..30 {
        KERNEL_PARK.gpr(i).store(frame.gpr[i], Ordering::Release);
    }
    KERNEL_PARK.lr().store(frame.lr, Ordering::Release);
    let elr = if rewind { frame.elr.wrapping_sub(4) } else { frame.elr };
    KERNEL_PARK.elr().store(elr, Ordering::Release);
    KERNEL_PARK.spsr().store(frame.spsr, Ordering::Release);
    KERNEL_PARK.sp_el0().store(frame.sp_el0, Ordering::Release);
    KERNEL_PARK_RUN.store(
        tos_api::bdos::process::current() as u64,
        Ordering::Release,
    );
    KERNEL_PARK_RESTARTABLE.store(rewind, Ordering::Release);
    tos_api::bdos::process::sched::set_kernel_parked(true);
}

/// Copy the armed [`KERNEL_PARK`] into slot `slot`'s `parent_frame` so that an
/// accessory which **terminates** (EmuTOS accessories never do, `docs/api/aes/
/// accessories.md` §4 point 6, but ours must not hang if one returns from
/// `main`) resumes the same EL1 client the block path would have resumed.
/// `parent_frame_full` marks it as a whole EL1 frame, not the partial
/// `elr`/`sp_el0`/`spsr` triple `enter_el0_with_resume_capture` writes.
fn adopt_kernel_park_as_parent(slot: usize) {
    use core::sync::atomic::Ordering;

    let pf = &tos_api::bdos::process::PROCESS_TABLE[slot].parent_frame;
    for i in 0..30 {
        pf.gpr(i)
            .store(KERNEL_PARK.gpr(i).load(Ordering::Acquire), Ordering::Release);
    }
    pf.lr()
        .store(KERNEL_PARK.lr().load(Ordering::Acquire), Ordering::Release);
    pf.elr()
        .store(KERNEL_PARK.elr().load(Ordering::Acquire), Ordering::Release);
    pf.spsr()
        .store(KERNEL_PARK.spsr().load(Ordering::Acquire), Ordering::Release);
    pf.sp_el0()
        .store(KERNEL_PARK.sp_el0().load(Ordering::Acquire), Ordering::Release);
    tos_api::bdos::process::PROCESS_TABLE[slot]
        .parent_l0_pa
        .store(0, Ordering::Release);
    tos_api::bdos::process::PROCESS_TABLE[slot]
        .parent_frame_full
        .store(true, Ordering::Release);
    // …and it is explicitly *not* a `Pexec` caller's frame: the two
    // classifications are mutually exclusive, and the one that writes the
    // frame states both (DESK46).
    tos_api::bdos::process::PROCESS_TABLE[slot]
        .parent_frame_pexec
        .store(false, Ordering::Release);
}

/// Restore the parked EL1 kernel client over `frame` and disarm the park —
/// `apply_switch`'s `el1_internal` target. TTBR0 goes back to the empty
/// no-process root (an EL1 context needs no low half), `RUN` back to the value
/// the client had, and the scheduler's "running slot" to none.
fn restore_kernel_park(frame: &mut ExceptionFrame) {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::sched;

    // Same TTBR0 bracket as `apply_switch`/`finish_terminate`: TLBI via the
    // boot-restore, then a UART-bearing line (QUIRK-001), then no install —
    // the resume is to EL1, which runs entirely through TTBR1.
    // SAFETY: trap context, single-CPU; no kernel low-half VA is held.
    unsafe {
        crate::arch::aarch64::user_entry::restore_boot_ttbr0();
    }
    crate::kprintln!("[sched] switch -> kernel client");

    for i in 0..30 {
        frame.gpr[i] = KERNEL_PARK.gpr(i).load(Ordering::Acquire);
    }
    frame.lr = KERNEL_PARK.lr().load(Ordering::Acquire);
    frame.elr = KERNEL_PARK.elr().load(Ordering::Acquire);
    frame.spsr = KERNEL_PARK.spsr().load(Ordering::Acquire);
    frame.sp_el0 = KERNEL_PARK.sp_el0().load(Ordering::Acquire);
    if !KERNEL_PARK_RESTARTABLE.load(Ordering::Acquire) {
        frame.gpr[0] = 0; // the resumed leaf `Pwait` returns 0
    }

    tos_api::bdos::process::set_current(
        KERNEL_PARK_RUN.load(Ordering::Acquire) as *mut tos_types::BasePage,
    );
    sched::set_running(tos_api::bdos::process::MAX_PROCESSES);
    sched::set_kernel_parked(false);
}

/// `Pwait` (GEMDOS 0x4D) — cooperative yield. Snapshot the running peer's
/// full EL0 context into its slot, put it at the end of the ready list and
/// resume the list's head (a context switch like `Pexec`/`Pterm`, but
/// peer→peer; the head may be the parked EL1 kernel client, A115). When the
/// caller is the head again at once it is a no-op that returns 0.
///
/// **From an EL1 kernel context** (`RUN` names no slot) this is EmuTOS's
/// `all_run(); dsptch();` — the accessory-start half of `wait_for_accs`
/// (`aes/geminit.c:610,736`): the kernel client goes to the end of the ready
/// list, the EL1 frame is parked and the head gets the CPU; the call returns
/// when the round robin comes back to it. When it is the head itself (its
/// first dispatch after `pstart`, or nothing else ready) it is a no-op.
fn handle_pwait(frame: &mut ExceptionFrame) {
    use tos_api::bdos::process::{sched, MAX_PROCESSES};

    let cur = sched::running_slot();
    if cur >= MAX_PROCESSES {
        match sched::pick_peer_for_kernel() {
            Some(sw) => {
                // Leaf park: resume *after* the `svc`, `x0 = 0` (ACC03).
                park_kernel_frame(frame, false);
                adopt_kernel_park_as_parent(sw.slot);
                apply_switch(frame, &sw);
            }
            None => frame.gpr[0] = 0, // the kernel client is the head → no-op
        }
        return;
    }

    // Leaf yield: resume *after* the svc (`rewind = false`), so this is
    // byte-identical to the previous inline save.
    save_yield_frame(cur, frame, false);
    sched::mark_yielded(cur);

    match sched::on_yield(cur) {
        Some(sw) => apply_switch(frame, &sw),
        None => frame.gpr[0] = 0, // the caller is the head → continue
    }
}

/// Install the next peer's address space and rewrite the trap frame so the
/// post-handler `eret` lands in it — fresh at its entry point, or resumed
/// from its saved EL0 context. Shared by `Pwait` and scheduler-aware
/// `Pterm`.
fn apply_switch(frame: &mut ExceptionFrame, sw: &tos_api::bdos::process::sched::Switch) {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{sched, State, PROCESS_TABLE};

    // The EL1-internal target (ACC03): not a process at all but the parked
    // kernel client — no slot, no TTBR0 install, no `saved_frame`.
    if sw.el1_internal {
        restore_kernel_park(frame);
        return;
    }

    // TTBR0 switch with the QUIRK-001 bracket (QEMU cortex-a72 TCG): a
    // TLBI (boot-restore) then UART-bearing prints then the no-TLBI
    // install, so the `eret`-to-EL0 later does not wedge. Harmless on
    // real hardware. See `handle_pexec` / `finish_terminate`.
    // SAFETY: trap context, single-CPU; no kernel low-half VA held.
    unsafe {
        crate::arch::aarch64::user_entry::restore_boot_ttbr0();
    }
    crate::kprintln!("[sched] switch -> slot {}", sw.slot);
    unsafe {
        tos_api::bdos::process::mmu::install_ttbr0(sw.l0_pa);
    }
    crate::kprintln!("[sched] running");

    if sw.fresh {
        for i in 0..30 {
            frame.gpr[i] = 0;
        }
        // The process-entry contract: the basepage (PD pointer) in `x0`
        // (`sdk/crt0.S`), as on the `Pexec` and kernel-launch paths.
        frame.gpr[0] = tos_api::bdos::process::pd_of(sw.slot) as u64;
        frame.lr = 0;
        frame.elr = sw.entry_va;
        frame.spsr = 0; // SPSR.M = EL0t
        // The slot's own initial stack when the loader computed one (a program
        // loaded through `pexec::load`, i.e. every desk accessory — its TPA top
        // is mapped, the fixed constant below is not); the fixed default
        // otherwise, for the embedded selftest blobs that map it.
        frame.sp_el0 = if sw.sp_el0 != 0 {
            sw.sp_el0
        } else {
            crate::arch::aarch64::el0::USER_STACK_TOP_EXCLUSIVE
        };
    } else {
        let sf = &PROCESS_TABLE[sw.slot].saved_frame;
        for i in 0..30 {
            frame.gpr[i] = sf.gpr(i).load(Ordering::Acquire);
        }
        frame.lr = sf.lr().load(Ordering::Acquire);
        frame.elr = sf.elr().load(Ordering::Acquire);
        frame.spsr = sf.spsr().load(Ordering::Acquire);
        frame.sp_el0 = sf.sp_el0().load(Ordering::Acquire);
        // The SECOND S2 delta (design §4.3, corrected — see `save_yield_frame`):
        // a leaf `Pwait` resume returns 0 in `x0`, but a **restartable** resume
        // re-executes the `svc` and must keep its original `x0` (the GEM-trap
        // selector, e.g. 200 = AES) or the re-entered trap dispatches nothing.
        // `switch_to` set `restartable` from the slot's recorded yield kind.
        if !sw.restartable {
            frame.gpr[0] = 0; // the resumed `Pwait` returns 0
        }
        // …and the vector half the `save_yield_frame` above stored (ACC03).
        restore_fp_context(&PROCESS_TABLE[sw.slot].saved_fp);
    }

    tos_api::bdos::process::set_current(tos_api::bdos::process::pd_of(sw.slot));
    PROCESS_TABLE[sw.slot].set_state(State::Running);
    sched::set_running(sw.slot);

    // Deliver-on-resume (design §6, §8.2 sub-step 4): a process resumed from a
    // block may have a pending EL0 upcall — e.g. an event/message for an AES
    // client that was blocked in `evnt_multi` (the sixth upcall consumer of
    // `docs/analysis/coop-kernel/10-upcall-signal-design.md` §1/§8). Deliver it
    // now, *after* `set_current` so `deliver_upcall`'s `slot_for(current())`
    // resolves to this slot: it snapshots the just-restored (intact — including
    // the preserved `x0`) block context onto the kernel-side upcall stack and
    // redirects the `eret` into the EL0 handler, which runs in the
    // freshly-installed TTBR0. On the handler's `Psigreturn` the block context
    // resumes and the restartable `svc` re-executes, now seeing its event.
    // Only on a `!fresh` resume — a fresh process has no prior block context.
    if !sw.fresh {
        if let Some(u) = tos_api::bdos::process::take_pending_upcall(sw.slot) {
            deliver_upcall(frame, u);
        } else {
            // …or the next byte for its `kbdvec` hook (KEYS02).
            deliver_pending_kbdvec(frame);
        }
    }
}

/// Pterm (GEMDOS 0x4C) / Pterm0 (0x00) handler. Rewrites the trap
/// frame to redirect the post-handler `eret` at the dying process's
/// captured parent resume PC, after freeing every BDOS allocation
/// the process owned.
///
/// On error the frame is left unchanged and `x0` is set to
/// [`tos_types::error::EINVFN`] — Pterm has no error code in the
/// EmuTOS ABI, so this is purely a debugging affordance for
/// the D6 selftest.
fn handle_pterm(frame: &mut ExceptionFrame, func: u16) {
    // Capture the user's exit code BEFORE termination runs. ABI per
    // function:
    //   * `Pterm0`   (0x00): rc fixed at 0, no args.
    //   * `Pterm`    (0x4C): rc in `x0` (= `frame.gpr[0]`).
    //   * `Ptermres` (0x31): `Ptermres(blkln, rc)` — `blkln` in `x0`,
    //     `rc` in `x1`. `blkln` is the size the TPA is shrunk to before it
    //     stays resident (EmuTOS `xsetblk(0, run, blkln)`, `bdos/proc.c:626`).
    //
    // The exit code is a **UWORD** (D-005): EmuTOS `xterm(UWORD rc)`
    // (`bdos/proc.c:601,616`) stores `run->p_dreg[0] = rc`, i.e. the
    // low 16 bits **zero-extended** into the LONG the parent reads back
    // from `Pexec`. Mask to 16 bits here so `Pterm(-1)` delivers 65535,
    // `Pterm(0x12345)` delivers 0x2345 — never a sign-extended value.
    let rc: i32 = match func {
        0x00 => 0,
        0x31 => (frame.gpr[1] & 0xFFFF) as i32,
        _ => (frame.gpr[0] & 0xFFFF) as i32,
    };
    // `Ptermres`' keep-resident size: a LONG in `x0` (only read for 0x31).
    let blkln = frame.gpr[0] as u32 as i32;

    // BIOS finding H1 (etv_term): if the app installed a real `etv_term`
    // process-terminate hook via `Setexc(0x102, handler)`, run it at EL0
    // FIRST, then terminate — EmuTOS `xterm` calls `protect_v(userterm)` at
    // the very start of termination, before any teardown, then continues to
    // `ixterm`/`termuser` (`bdos/proc.c:601-618`). `protect_v` is a bare
    // `jsr (a0)` with no arguments (`util/miscasm.S:213-220`), and `etv_term`
    // is `void (*)(void)` (`include/tosvars.h:112`), so the handler receives
    // NO arguments; termination continues unconditionally after it returns.
    //
    // The boot-default is a NON-callable sentinel in NewTOS
    // (`ETV_DEFAULT_JUST_RTS`, a stable opaque token — not a real `just_rts`
    // code address), so the sentinel (and `0` = never installed) is treated
    // as "no handler" and we terminate directly. That is behaviourally
    // identical to EmuTOS calling `just_rts` (a no-op RTS) and also resolves
    // H1's "boot-default non-callable sentinel" concern: the sentinel is
    // never jumped to. Only etv_term is wired here (incremental H1 close);
    // etv_critic (M1) and etv_timer are later cooperative-kernel steps.
    //
    // Guarded on a live process slot: a `Pterm` with no running process
    // (`current() == null`) falls through to the normal path, which reports
    // `EINVFN` via `finish_terminate`'s misuse branch (matching the old
    // behaviour) instead of silently no-op'ing in `deliver_upcall`.
    let etv = tos_api::bios::vectors::vector(tos_api::bios::vectors::ETV_TERM);
    let has_slot = tos_api::bdos::process::slot_for(
        tos_api::bdos::process::current(),
    )
    .is_some();
    if has_slot && etv != 0 && etv != tos_api::bios::vectors::ETV_DEFAULT_JUST_RTS
    {
        // The terminate variant that runs *after* the handler is chosen by
        // `func`: Ptermres (0x31) keeps the process's owned pool blocks
        // resident, Pterm/Pterm0 free them. The handler runs in the
        // terminating process's still-mapped EL0 context (TTBR0 unchanged);
        // on its `Psigreturn` the disposition routes into that teardown.
        deliver_upcall(
            frame,
            tos_api::bdos::process::UpcallDesc {
                handler_va: etv as u64,
                args: [0; 4], // etv_term is void(*)(void) — no arguments
                mask_bit: tos_api::bdos::process::UPCALL_MASK_ETV_TERM,
                disposition: tos_api::bdos::process::Disposition::Terminate {
                    rc,
                    reserve: (func == 0x31).then_some(blkln),
                },
            },
        );
        return;
    }

    // No app handler installed → terminate directly (unchanged path).
    // SAFETY: trap-context single-CPU contract; the BDOS giant lock
    // is held implicitly while we execute the handler.
    let result = unsafe {
        match func {
            0x00 => tos_api::bdos::process::pterm::terminate_zero(),
            0x31 => tos_api::bdos::process::pterm::terminate_resident(blkln, rc),
            _ => tos_api::bdos::process::pterm::terminate(rc),
        }
    };

    finish_terminate(frame, rc, result);
}

/// Terminate the running process with exit code `rc` (cleanup via the
/// normal [`pterm::terminate`]) and redirect the frame to the parent —
/// used by the Ctrl-C self-terminate from blocking console input,
/// which records its request via `request_terminate` rather than a
/// `Pterm` SVC.
fn terminate_and_redirect(frame: &mut ExceptionFrame, rc: i32) {
    // SAFETY: trap context, single-CPU, BDOS lock held implicitly.
    let result = unsafe { tos_api::bdos::process::pterm::terminate(rc) };
    finish_terminate(frame, rc, result);
}

/// A synchronous exception taken from an EL0 program — EmuTOS `any_vec` →
/// `dopanic` → `kill_program` (`bios/vectors.S`, `bios/kprint.c:302-596`,
/// `bios/panicasm.S:96-99`; spec `docs/api/bios/_exception-crash.md`): print
/// the crash report, wait for a key, terminate the program with `Pterm(-1)`.
/// The parent (a `Pexec` caller, or the kernel's launcher for a program the
/// shell loop started) resumes as after any `Pterm`.
///
/// Returns `false` — the caller then halts as before — when the exception
/// cannot be attributed to a process (no live slot behind `RUN`).
fn kill_crashed_program(frame: &mut ExceptionFrame) -> bool {
    use tos_api::bios::crash;

    let Some(slot) =
        tos_api::bdos::process::slot_for(tos_api::bdos::process::current())
    else {
        return false;
    };
    // FAR first, before anything can take another abort.
    let far = crash::far_valid(frame.esr).then(read_far_el1);
    let mut regs = [0u64; 31];
    regs[..30].copy_from_slice(&frame.gpr);
    regs[30] = frame.lr;
    let report = crash::CrashReport {
        kind: crash::classify(frame.esr),
        esr: frame.esr,
        far,
        pc: frame.elr,
        pstate: frame.spsr,
        regs: &regs,
        usp: frame.sp_el0,
        program: super::crash::current_program(),
        violation: crash::mem_violation(
            frame.esr,
            far.and_then(tos_api::bdos::process::memprot::region_kind),
        ),
    };
    super::crash::report_and_wait(&report, slot);
    kill_program(frame);
    true
}

/// The syscall tail for a call that touched a bad user pointer
/// ([`tos_api::uaccess`]). An EL0 caller is terminated with the crash report
/// of the kernel-side fault — what EmuTOS shows for a bus error inside a
/// trap: the supervisor PC, registers and fault address, then `Pterm(-1)` of
/// the caller (`bios/kprint.c:587-594`). An EL1 caller handing a user-pointer
/// API a pointer it cannot reach is a kernel bug and stays fatal, as it was
/// before the access became recoverable.
fn uaccess_fault_tail(frame: &mut ExceptionFrame, rec: &tos_api::uaccess::FaultRecord) {
    use tos_api::bios::crash;

    let slot = tos_api::bdos::process::slot_for(tos_api::bdos::process::current());
    let slot = match slot {
        Some(s) if frame.spsr & 0x1F == 0 => s,
        _ => {
            crate::console::force_serial_tx_on();
            crate::kprintln!(
                "\n[EXC] user-access fault in a kernel-initiated call: \
                 FAR=0x{:016x} PC=0x{:016x} ESR=0x{:016x} (caller SPSR=0x{:x})",
                rec.far, rec.pc, rec.esr, frame.spsr,
            );
            crate::arch::aarch64::halt();
        }
    };
    let report = crash::CrashReport {
        kind: crash::classify(rec.esr),
        esr: rec.esr,
        far: crash::far_valid(rec.esr).then_some(rec.far),
        pc: rec.pc,
        pstate: rec.pstate,
        regs: &rec.regs,
        usp: frame.sp_el0,
        program: super::crash::current_program(),
        violation: crash::mem_violation(
            rec.esr,
            crash::far_valid(rec.esr)
                .then_some(rec.far)
                .and_then(tos_api::bdos::process::memprot::region_kind),
        ),
    };
    super::crash::report_and_wait(&report, slot);
    kill_program(frame);
}

/// EmuTOS `kill_program` (`bios/panicasm.S:96-99`): `Pterm(-1)` on behalf of
/// the running program, through exactly the path of the program's own
/// `Pterm` — `etv_term` hook, resource release, parent resume
/// ([`handle_pterm`]). The `-1` is the WORD EmuTOS pushes; `handle_pterm`
/// takes its low 16 bits (D-005), so the parent's `Pexec` sees `0xFFFF`.
///
/// Should termination be refused (no parent resume context — a kernel
/// bookkeeping bug), the frame comes back unchanged and resuming it would
/// re-run the faulting code: halt instead.
fn kill_program(frame: &mut ExceptionFrame) {
    let (elr, spsr) = (frame.elr, frame.spsr);
    frame.gpr[0] = -1i64 as u64;
    handle_pterm(frame, 0x4C);
    if frame.elr == elr && frame.spsr == spsr {
        unhandled(frame, "Pterm(-1) after a program crash refused");
    }
}

/// Common tail of every process-termination path (`Pterm` / `Pterm0` /
/// `Ptermres`, and the Ctrl-C self-terminate): copy the parent's frame
/// snapshot back over the trap frame, restore the parent's `TTBR0`,
/// release the slot, and let the post-handler `eret` resume the parent.
///
/// `rc` may arrive unmasked (Ctrl-C `-32`, upcall overflow `-998`); the
/// parent receives `rc & 0xFFFF` zero-extended on both branches — the
/// `Pexec` caller's `x0` via [`pterm::exit_code_for_caller`], the
/// kernel-launch path via `record_kernel_child_exit` (PROC06, PROC05).
///
/// [`pterm::exit_code_for_caller`]: tos_api::bdos::process::pterm::exit_code_for_caller
fn finish_terminate(
    frame: &mut ExceptionFrame,
    rc: i32,
    result: Result<
        tos_api::bdos::process::pterm::PtermResume,
        tos_api::bdos::process::pterm::PtermError,
    >,
) {
    use core::sync::atomic::Ordering;

    let mut resume = match result {
        Ok(r) => r,
        Err(_) => {
            // Misuse — leave the frame alone and report EINVFN so
            // the user's SVC observes a recoverable error.
            frame.gpr[0] = tos_types::error::EINVFN as i64 as u64;
            return;
        }
    };

    // `run = run->p_parent` (EmuTOS `xterm`, `bdos/proc.c:609`; TASK/PROC03
    // B4, `docs/api/gemdos/process/_run-ownership-curdir.md` §6.4 D-5): the
    // dying process's `p_parent` decides where this returns
    // (`pterm::return_target`).
    if let Some(v) = resume.ignored_parent {
        crate::kprintln!(
            "[pterm] p_parent 0x{:08x} names no resume point on this launch chain \
             — returning to the parent it was started from",
            v
        );
    }
    if resume.unwind > 0 {
        // The field names an ancestor: the processes in between end without
        // being resumed (EmuTOS never resumes them either). Their frames are
        // discarded, each is terminated in turn (`terminate_unwound`, `RUN`
        // is already it), and the last one's `parent_frame` is the resume
        // point below. The boot tables go in first: the slots released here
        // take their page tables with them while `TTBR0` still names the
        // dying process's (the final restore below re-installs as usual).
        crate::kprintln!(
            "[pterm] p_parent names the PD {} launch(es) further up — unwinding to it",
            resume.unwind
        );
        // SAFETY: trap context, single-CPU; no kernel code holds a low-half
        // VA reference across this point (same as the restore below).
        unsafe { crate::arch::aarch64::user_entry::restore_boot_ttbr0() };
        for _ in 0..resume.unwind {
            tos_api::bdos::process::sched::resume_parent_of_child(resume.parent_frame_slot);
            // SAFETY: BDOS lock (caller contract); this slot's frame is not
            // used — the process it would resume is terminated next.
            unsafe { tos_api::bdos::process::pterm::release_slot(resume.parent_frame_slot) };
            // SAFETY: trap context, BDOS lock; `RUN` is the next process.
            match unsafe { tos_api::bdos::process::pterm::terminate_unwound() } {
                Ok(r) => resume = r,
                Err(e) => {
                    // `return_target` checked every slot on the way (a
                    // started process with a resume record), so this is a
                    // table inconsistency: nothing is left to resume.
                    crate::kprintln!("[pterm] unwind stopped: {:?} — halting", e);
                    crate::arch::aarch64::halt();
                }
            }
        }
    }

    let pf = &tos_api::bdos::process::PROCESS_TABLE[resume.parent_frame_slot]
        .parent_frame;
    let pf_spsr = pf.spsr().load(Ordering::Acquire);

    // Copy the parent's frame snapshot over the current trap frame.
    // After this, the post-handler asm restore + eret resumes the
    // parent at exactly the state captured when `Pexec` redirected
    // it (or, for the kernel-launch path, at the EL1 resume PC the
    // EL0-entry helper stamped into `parent_frame.elr`).
    //
    // Which of the three kinds of `parent_frame` this is, is recorded by
    // whoever wrote it — **not** derived from `SPSR.M`, because two of the
    // three are `EL1t` (DESK46):
    //
    //   * [`parent_frame_pexec`] — a `Pexec` caller's trap frame, written by
    //     [`handle_pexec`] for every launch it redirects. Full register
    //     restore, so the caller's `x0..x29 / lr / sp_el0` — and, from
    //     `parent_fp`, its `v0..v31 / FPCR / FPSR` (ARCH02) — are exactly what
    //     they were at the `Pexec` SVC, with `gpr[0]` overwritten to the
    //     child's exit code (`docs/api/gemdos/process/4b-pexec.md`,
    //     "Trap interface"; EmuTOS `bdos/proc.c:601-618`). This holds for an
    //     **EL1** caller as much as for an EL0 one: an `svc` preserves the
    //     callee-saved registers as far as its caller's compiler is
    //     concerned, and the only EL1 caller — EmuCON's `run` command,
    //     [`crate::shell`] — kept the UART base and its own buffers in
    //     `x19`…`x28` across the trap and read them back as the child's
    //     zeroes. Before DESK46 this arm was gated on `SPSR.M == EL0t` and
    //     that case fell through to the partial restore below.
    //   * [`parent_frame_full`] — a desk accessory that terminated (ACC03).
    //     Its "parent" is the EL1 kernel client whose whole trap frame was
    //     copied in when the accessory was switched to, and whose `svc` is
    //     about to **re-execute**: restore the registers too, but do *not*
    //     write the exit code over `x0` (that would replace the GEM-trap
    //     selector and the re-entered trap would dispatch nothing). Consuming
    //     the frame also disarms the kernel park.
    //   * neither — the kernel-launch path (Phase 05 EC4 / D3 / D6 selftests,
    //     `crate::launch`): the "parent" is a kernel function and only
    //     `elr` / `sp_el0` / `spsr` were written. **Only** those are
    //     restored; the resume function's epilogue restores its callee-saved
    //     registers from its own stack frame, and copying zeroes into `gpr`
    //     would clobber `x29` and crash the post-resume frame-pointer chain.
    //     The exit code goes to `crate::launch::record_kernel_child_exit`.
    //
    // [`parent_frame_full`]: tos_api::bdos::process::Process::parent_frame_full
    // [`parent_frame_pexec`]: tos_api::bdos::process::Process::parent_frame_pexec
    let full = tos_api::bdos::process::PROCESS_TABLE[resume.parent_frame_slot]
        .parent_frame_full
        .swap(false, Ordering::AcqRel);
    let pexec_caller = tos_api::bdos::process::PROCESS_TABLE
        [resume.parent_frame_slot]
        .parent_frame_pexec
        .swap(false, Ordering::AcqRel);
    if pexec_caller || full {
        for i in 0..30 {
            frame.gpr[i] = pf.gpr(i).load(Ordering::Acquire);
        }
        frame.lr = pf.lr().load(Ordering::Acquire);
        if pexec_caller {
            // Exit code lands in the caller's `x0` per the GEMDOS Pexec
            // ABI. Overwrite gpr[0] *after* the bulk copy so the
            // caller's pre-trap value doesn't shadow it. The low 16 bits,
            // zero-extended (EmuTOS `xterm(UWORD rc)`, `bdos/proc.c:601,
            // 616`): masked here, once for every terminate path, so the
            // console Ctrl-C's `-32` arrives as 65504 and the upcall
            // overflow's `-998` as 64538, never as a negative "load error"
            // (PROC06; `docs/api/gemdos/process/4c-pterm.md`).
            frame.gpr[0] = tos_api::bdos::process::pterm::exit_code_for_caller(rc);
            // The caller's FP/SIMD registers as `handle_pexec` saved them
            // (ARCH02); the live ones are the terminating child's. Restored
            // straight into the registers: nothing between here and the
            // `eret` uses them (soft-float kernel). Not for the adopted
            // kernel park (`full`) — its context is EL1 code, which never
            // reads these registers, and it saved none.
            restore_fp_context(
                &tos_api::bdos::process::PROCESS_TABLE[resume.parent_frame_slot].parent_fp,
            );
        }
    } else {
        // The kernel-launch path: no caller register takes the exit code, so
        // it is left for the resumed kernel function — the launcher returns
        // it as `dos_exec`'s value to `sh_ldapp` (PROC05, DESK43 U9-0b).
        crate::launch::record_kernel_child_exit(rc);
    }
    if full {
        // The park has been consumed by this resume; nothing is parked now.
        tos_api::bdos::process::sched::set_kernel_parked(false);
        tos_api::bdos::process::sched::set_running(tos_api::bdos::process::MAX_PROCESSES);
    }
    frame.elr = pf.elr().load(Ordering::Acquire);
    frame.spsr = pf_spsr;
    frame.sp_el0 = pf.sp_el0().load(Ordering::Acquire);

    // The launch this slot stood for is over: put the parent back into the
    // cooperative run-set exactly as `handle_pexec` found it (PORT05, EmuTOS
    // `xterm` → `run = run->p_parent`, `bdos/proc.c:609`). Must run **before**
    // `release_slot`, which clears the `parent_slot` / `parent_runnable` pair
    // this reads. A no-op for a slot with no parent slot — the kernel-launched
    // programs and the desk accessories, whose parent is an EL1 context.
    tos_api::bdos::process::sched::resume_parent_of_child(
        resume.parent_frame_slot,
    );

    // Done with the slot's atomics — release it for re-use.
    // SAFETY: see `terminate` — caller-side BDOS-lock contract.
    unsafe {
        tos_api::bdos::process::pterm::release_slot(resume.parent_frame_slot);
    }

    // Bring the parent's address space back online before the eret.
    //   * `parent_l0_pa == 0` — the dying process was launched by the
    //     kernel boot path (Phase 06 D6 selftest, no real parent).
    //     Restore the Stage-1 boot tables; the resume is to EL1, so
    //     no QEMU wedge applies regardless.
    //   * `parent_l0_pa != 0` — a real parent called `Pexec`. Flush
    //     the TLB via boot-restore (TLBI is safe in kernel context),
    //     then `install_ttbr0` (no TLBI) so the parent's mappings are
    //     fresh-walked.
    //
    // SAFETY: trap context, single-CPU; no kernel code holds a
    // low-half VA reference across this point.
    //
    // The `kprintln!` between the boot-restore (TLBI) and the parent
    // install is INTERIM, QEMU-only, and LOAD-BEARING — see QUIRK-001
    // in docs/MEMORY.md. Parent and child share identical EL0 VAs
    // (both TPAs map at VA 0x100) under a shared ASID 0, so QEMU's
    // cortex-a72 TCG model reuses the *child's* cached translation
    // block for the parent's same-VA code across this TTBR0 switch —
    // the post-handler `eret`-to-EL0 then makes no forward progress
    // and the model wedges. A line of UART traffic (Device-memory
    // writes) between the TLBI and the install forces TCG to end the
    // translation block; a single Device *read* is NOT sufficient
    // (verified empirically), nor is pure arithmetic "distance". This
    // mirrors what `handle_pexec` already relies on. On real hardware
    // it is a harmless diagnostic line. Per-process ASIDs were tried
    // as a "proper" fix and made this QEMU model *worse* (it wedges on
    // the ASID-tagged TTBR0 write itself) — see QUIRK-001. The wedge
    // is a pure QEMU artifact, so this barrier IS the correct
    // long-term solution for the QEMU target; keep it.
    unsafe {
        crate::arch::aarch64::user_entry::restore_boot_ttbr0();
        if resume.parent_l0_pa != 0 {
            tos_api::bdos::process::mmu::install_ttbr0(resume.parent_l0_pa);
        }
    }
}

/// Deliver a pending EL0 upcall (cooperative-kernel building block 1,
/// `docs/analysis/coop-kernel/10-upcall-signal-design.md` §4/§6/§7). Snapshot
/// the current (already-result-bearing, §6.2) EL0 frame onto the running
/// process's kernel-side upcall context stack, then redirect the frame so the
/// post-handler `eret` lands in the EL0 handler — with `args` in `x0..x3`
/// (AAPCS64) and the EL0 trampoline ([`TRAMPOLINE_VA`], which calls
/// `Psigreturn`) in the link register. The handler runs on the interrupted
/// `SP_EL0`, re-aligned to 16 bytes with a red-zone guard (§6.4). Pure frame
/// mutation, identical in style to `apply_switch`'s resume branch. On nesting
/// overflow (`> MAX_UPCALL_NEST`) the process is terminated with `Pterm(-998)`
/// (§6.3).
fn deliver_upcall(
    frame: &mut ExceptionFrame,
    desc: tos_api::bdos::process::UpcallDesc,
) {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{
        current, loader::TRAMPOLINE_VA, slot_for, MAX_UPCALL_NEST, PROCESS_TABLE,
    };

    // The caller established a pending upcall for the running slot, so
    // `current()` is a live process; recompute its slot for the store.
    let slot_idx = match slot_for(current()) {
        Some(i) => i,
        None => return,
    };
    let p = &PROCESS_TABLE[slot_idx];

    let depth = p.upcall_depth.load(Ordering::Acquire) as usize;
    if depth >= MAX_UPCALL_NEST {
        // Nesting overflow (§6.3): terminate like FreeMiNT's "stack
        // overflow" ALERT (`sig_mach.c:138`), exit code -998.
        terminate_and_redirect(frame, -998);
        return;
    }

    // Save the interrupted context — all 30 GPRs (the EL0 code is stopped at
    // an arbitrary point, so any register may be live; §6.2), plus lr / elr /
    // spsr / sp_el0 — so `Psigreturn` can resume it, including the syscall
    // result already in `gpr[0]`.
    let ctx = &p.upcall_ctx[depth];
    for i in 0..30 {
        ctx.gpr(i).store(frame.gpr[i], Ordering::Release);
    }
    ctx.lr().store(frame.lr, Ordering::Release);
    ctx.elr().store(frame.elr, Ordering::Release);
    ctx.spsr().store(frame.spsr, Ordering::Release);
    ctx.sp_el0().store(frame.sp_el0, Ordering::Release);
    // Record which mask bit this level set so `Psigreturn` clears exactly it.
    p.upcall_ctx_mask[depth].store(desc.mask_bit, Ordering::Release);
    // Record the disposition (cooperative-kernel step 2): `Psigreturn` reads
    // it to decide resume (step-1 default) vs terminate-after (etv_term).
    p.upcall_ctx_disp[depth].store(desc.disposition.encode(), Ordering::Release);
    p.upcall_depth.store((depth + 1) as u8, Ordering::Release);
    // Mark the upcall type active (FreeMiNT `p_sigmask`, `sig_mach.c:291`).
    let m = p.upcall_mask.load(Ordering::Acquire);
    p.upcall_mask.store(m | desc.mask_bit, Ordering::Release);

    // Redirect the frame to the EL0 handler.
    frame.gpr[0] = desc.args[0];
    frame.gpr[1] = desc.args[1];
    frame.gpr[2] = desc.args[2];
    frame.gpr[3] = desc.args[3];
    frame.elr = desc.handler_va;
    frame.lr = TRAMPOLINE_VA;
    // AAPCS64 16-byte alignment (an IRQ boundary may leave `SP_EL0`
    // unaligned) + a 0x20 red-zone guard below it (§6.4). `- 0x20` keeps the
    // 16-byte alignment produced by `& !0xF`.
    frame.sp_el0 = (frame.sp_el0 & !0xF).wrapping_sub(0x20);
    // `frame.spsr` is left as-is — the handler runs in the same EL0 context.
}

/// GEMDOS `Super` (0x20) — the super state (MEM12,
/// `docs/api/_super-state.md` S3–S5, approved 2026-10-27). For an EL0
/// program: `superstate::super_call` with the caller's `SP_EL0` (the token
/// `Super(0L)` hands back); the result goes through the ordinary completion
/// tail. An EL1 caller (kernel code, no process) runs privileged by
/// definition: the inquiry answers `-1` like EmuTOS's `x20_inq` in supervisor
/// mode; there is no state to switch, so any other argument is `EINVFN`.
fn handle_super(frame: &mut ExceptionFrame) {
    use tos_api::bdos::process::{current, slot_for, superstate};

    if frame.spsr & 0xF != 0 {
        frame.gpr[0] = if frame.gpr[0] == 1 {
            -1i64 as u64
        } else {
            tos_types::error::EINVFN as i64 as u64
        };
        return;
    }
    let Some(slot) = slot_for(current()) else {
        frame.gpr[0] = tos_types::error::EINVFN as i64 as u64;
        return;
    };
    // SAFETY: trap context under the BDOS giant lock; the running slot's root
    // is built.
    let result = unsafe { superstate::super_call(slot, frame.gpr[0], frame.sp_el0) };
    complete_call_tail(frame, result);
}

/// XBIOS `Supexec` (0x26) from an EL0 program (MEM12 S6): enter the super
/// state and deliver `codeptr` (`x0`) as an EL0 upcall with no arguments
/// (EmuTOS `jmp (a0)`, `bios/vectors.S:1376-1378`). The interrupted context
/// is the caller right after its `svc`; the callback's `ret` lands in the
/// trampoline, whose `Psigreturn` ([`Disposition::SupexecReturn`]) resumes
/// it with `x0` = the callback's value and the super state of before.
/// Unlike EmuTOS, the callback may make system calls. Returns `false` when
/// there is no running process (the caller then takes the dispatcher path).
///
/// [`Disposition::SupexecReturn`]: tos_api::bdos::process::Disposition::SupexecReturn
fn handle_supexec(frame: &mut ExceptionFrame) -> bool {
    use tos_api::bdos::process::{current, slot_for, superstate, Disposition, UpcallDesc};

    let Some(slot) = slot_for(current()) else {
        return false;
    };
    let codeptr = frame.gpr[0];
    let prev_super = superstate::is_super(slot);
    // SAFETY: trap context under the BDOS giant lock; running slot.
    unsafe { superstate::set_super(slot, true) };
    deliver_upcall(
        frame,
        UpcallDesc {
            handler_va: codeptr,
            args: [0; 4],
            mask_bit: 0,
            disposition: Disposition::SupexecReturn { prev_super },
        },
    );
    true
}

/// GEMDOS `Psigreturn` (0x11A) — the EL0 upcall trampoline's return syscall
/// (design §5). Pops the top saved EL0 context off the running process's
/// kernel-side upcall stack and acts on its [`Disposition`]
/// (cooperative-kernel step 2):
///
///   * [`Disposition::Resume`] (step-1 default) — restore the saved context
///     over the trap frame (GPRs + lr + elr + spsr + sp_el0), so the
///     post-handler `eret` resumes the interrupted EL0 code exactly where the
///     upcall diverted it — including the original syscall's `x0` (§7.2).
///   * [`Disposition::Terminate`] (etv_term) — the handler has run its
///     "notify before I die" course; now terminate the process with the
///     original `Pterm` exit code instead of resuming, routing into the
///     standard teardown ([`finish_terminate`], EmuTOS `xterm` continuing
///     past `protect_v(userterm)`, `bdos/proc.c:609-617`).
///
/// Either way the delivered upcall's mask bit is cleared and the nesting
/// level popped. Same restore style as `finish_terminate`. An empty stack is
/// caller misuse: leave the frame and report `EINVFN` (like
/// `finish_terminate`'s misuse branch, §5.2).
fn handle_psigreturn(frame: &mut ExceptionFrame) {
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{current, slot_for, Disposition, PROCESS_TABLE};

    let slot_idx = match slot_for(current()) {
        Some(i) => i,
        None => {
            frame.gpr[0] = tos_types::error::EINVFN as i64 as u64;
            return;
        }
    };
    let p = &PROCESS_TABLE[slot_idx];

    let depth = p.upcall_depth.load(Ordering::Acquire) as usize;
    if depth == 0 {
        // Misuse: `Psigreturn` with no upcall in flight.
        frame.gpr[0] = tos_types::error::EINVFN as i64 as u64;
        return;
    }
    let top = depth - 1;
    let disp = Disposition::decode(p.upcall_ctx_disp[top].load(Ordering::Acquire));
    // The handler's `x0` at its `Psigreturn`: the trampoline and the
    // `G_USERDEF` stub leave it alone, so it is the routine's return value
    // (`Supexec`'s result, the `G_USERDEF` WORD).
    let handler_x0 = frame.gpr[0];

    // Pop the nesting level bookkeeping first (FreeMiNT `p_sigmask &= ~…`,
    // `sig_mach.c:291`), so the process state is clean regardless of
    // disposition. Common to both branches.
    let bit = p.upcall_ctx_mask[top].load(Ordering::Acquire);
    let m = p.upcall_mask.load(Ordering::Acquire);
    p.upcall_mask.store(m & !bit, Ordering::Release);
    p.upcall_ctx_mask[top].store(0, Ordering::Release);
    p.upcall_ctx_disp[top].store(0, Ordering::Release);
    p.upcall_depth.store(top as u8, Ordering::Release);

    if disp == Disposition::UserdefReturn {
        // AES `G_USERDEF` callout return (AES-as-thread design §3.4(e)). The
        // stub's interrupted snapshot is discarded — the AES call is resumed on
        // the fiber, not this EL0 context. Read the WORD the stub stored, feed
        // it to the fiber, and either deliver the next userdef (a further stall)
        // or restore the client's original `wind_*` frame and finish the call.
        p.upcall_ctx[top].clear();
        // `ub_code`'s WORD, which the stub leaves in `w0` (MEM12: it no longer
        // stores it in page 0).
        let word = handler_x0 as i16;
        match tos_api::aes::thread::resume(word) {
            tos_api::aes::thread::AesCallOutcome::Stalled(next) => {
                deliver_userdef_upcall(frame, next);
            }
            tos_api::aes::thread::AesCallOutcome::Complete(sw) => {
                // The AES draw finished. Restore the client's captured origin
                // frame, then the shared completion tail writes `x0 = 0` (GEM
                // trap convention — the real result is in the app's `intout[]`).
                // A userdef-stalling draw (`compose`/`paint_bg_clip`) never
                // blocks, so the only `Switch` it can return is the
                // `completed` one of the dispatch every AES call ends with.
                restore_call_origin(frame);
                if complete_call_tail(frame, 0) {
                    // …and that dispatch's leaf yield (A112), as on the
                    // ordinary trap path.
                    if let Some(sw) = sw.filter(|sw| sw.completed) {
                        yield_after_completed_call(frame, &sw);
                    }
                }
            }
        }
        return;
    }

    if let Disposition::Terminate { rc, reserve } = disp {
        // Terminate-after (etv_term): the saved context is discarded — the
        // process dies rather than resuming. Route into the standard Pterm
        // teardown with the original exit code; `finish_terminate` redirects
        // the frame to the parent (or the kernel resume, boot-launched).
        p.upcall_ctx[top].clear();
        // SAFETY: trap-context single-CPU contract; BDOS giant lock held
        // implicitly — same contract as `handle_pterm`.
        let result = unsafe {
            match reserve {
                Some(blkln) => tos_api::bdos::process::pterm::terminate_resident(blkln, rc),
                None => tos_api::bdos::process::pterm::terminate(rc),
            }
        };
        finish_terminate(frame, rc, result);
        return;
    }

    // Resume (step-1 default): restore the interrupted context and scrub the
    // reused slot. Byte-identical to step 1 apart from clearing the disp word
    // above (which is `0` for a resume level, so no behavioural change).
    let ctx = &p.upcall_ctx[top];
    for i in 0..30 {
        frame.gpr[i] = ctx.gpr(i).load(Ordering::Acquire);
    }
    frame.lr = ctx.lr().load(Ordering::Acquire);
    frame.elr = ctx.elr().load(Ordering::Acquire);
    frame.spsr = ctx.spsr().load(Ordering::Acquire);
    frame.sp_el0 = ctx.sp_el0().load(Ordering::Acquire);
    ctx.clear();

    // MEM12: a `Supexec` callback's value is the `Supexec`'s result (S6), and
    // an upcall delivered in super state leaves it as it found it (S6, S11).
    match disp {
        Disposition::SupexecReturn { prev_super } => {
            frame.gpr[0] = handler_x0;
            // SAFETY: trap context under the BDOS giant lock; running slot.
            unsafe { tos_api::bdos::process::superstate::set_super(slot_idx, prev_super) };
        }
        Disposition::ResumeSuper { prev_super } => {
            // SAFETY: as above.
            unsafe { tos_api::bdos::process::superstate::set_super(slot_idx, prev_super) };
        }
        _ => {}
    }

    // KEYS02 (B4): more bytes for the `kbdvec` hook — chain the next upcall
    // onto the context just restored instead of resuming it, so a make and
    // its break that arrived in one poll both reach the hook, in order.
    deliver_pending_kbdvec(frame);
}

/// Read a NUL-terminated string from a **kernel** pointer (ordinary
/// loads). Used when an EL1 caller passes `Pexec` a kernel-VA path.
fn read_kernel_cstr(va: u64, buf: &mut [u8]) -> usize {
    let p = va as *const u8;
    let mut n = 0;
    while n < buf.len() {
        // SAFETY: bounded by buf.len(); provenance is the caller's path.
        let b = unsafe { p.add(n).read_volatile() };
        if b == 0 {
            break;
        }
        buf[n] = b;
        n += 1;
    }
    n
}

/// Read a NUL-terminated string from a user-space VA into `buf`, returning
/// the number of bytes read (excluding the terminator). The read goes
/// through [`tos_api::uaccess::read_user_cstr`] — unprivileged `ldtrb`, so it
/// is permission-checked as if EL0 issued it (the correct contract for a TOS
/// syscall pointer), and a bad pointer comes back as `Err` instead of
/// halting the machine (ARCH03 0c). The caller's `TTBR0_EL1` must be active.
/// Bounded by `buf.len()`; a missing NUL truncates.
fn read_user_cstr(
    user_va: u64,
    buf: &mut [u8],
) -> Result<usize, tos_api::uaccess::UserFault> {
    tos_api::uaccess::read_user_cstr(user_va, buf)
}

/// Byte length of the Pexec environment scratch buffer. Bounds a
/// caller-supplied environment block; generous versus real Atari
/// environments (a handful of `KEY=value` records). A larger block is
/// truncated to a double-NUL-terminated prefix.
const ENV_BUF_LEN: usize = 512;

/// Read a `Pexec` environment block (a sequence of NUL-terminated
/// records ended by a **double NUL**) from a pointer into `buf`,
/// returning the byte count **including** the terminating double NUL.
/// `from_el0` selects user (`tos_api::uaccess`, unprivileged `ldtrb`,
/// fault-recovered) vs ordinary (kernel pointer) loads, exactly like the
/// `path`/tail readers. On overflow the last two bytes are forced to NUL so
/// the copied env is always double-NUL terminated. `Err` when a user read
/// faulted.
fn read_env_block(
    va: u64,
    buf: &mut [u8],
    from_el0: bool,
) -> Result<usize, tos_api::uaccess::UserFault> {
    let mut p = va;
    let mut n = 0;
    let mut prev_nul = false;
    while n < buf.len() {
        let b: u8 = if from_el0 {
            tos_api::uaccess::get_user_u8(p)?
        } else {
            // SAFETY: bounded by buf.len(); provenance is the caller's env.
            unsafe { (p as *const u8).read_volatile() }
        };
        buf[n] = b;
        n += 1;
        p += 1;
        if b == 0 {
            if prev_nul {
                return Ok(n); // double NUL: env complete, both NULs included
            }
            prev_nul = true;
        } else {
            prev_nul = false;
        }
    }
    // Overflow without a natural double-NUL terminator: force one so the
    // downstream verbatim copy stays well-formed.
    if n >= 2 {
        buf[n - 1] = 0;
        buf[n - 2] = 0;
    }
    Ok(n)
}

/// A `Pexec` argument (path, tail, environment) could not be read from the
/// caller's memory. `Pexec` never reaches the ordinary syscall tail, so it
/// hands the recorded fault to [`uaccess_fault_tail`] itself.
fn pexec_user_fault(frame: &mut ExceptionFrame) {
    match tos_api::uaccess::take_pending_fault() {
        Some(rec) => uaccess_fault_tail(frame, &rec),
        // The reader reported a fault the abort handler did not record —
        // impossible by construction; never resume the caller as if the
        // call had succeeded.
        None => unhandled(frame, "Pexec user-access fault without a record"),
    }
}

/// Pexec (GEMDOS 0x4B) handler. Sets up a child process and
/// rewrites the trap frame so the post-handler `eret` lands in the
/// child at EL0 instead of returning to the parent.
///
/// On error the frame is left untouched except for `gpr[0]` which
/// receives the GEMDOS error code; the parent's SVC observes a
/// negative `Long`, just like every other GEMDOS error path.
fn handle_pexec(frame: &mut ExceptionFrame) {
    use core::sync::atomic::Ordering;

    use tos_api::bdos::process::loader::EnvSpec;
    use tos_api::bdos::process::pexec::{PE_BASEPAGE, PE_BASEPAGEFLAGS, PE_LOAD, PE_LOADGO};

    let mode = frame.gpr[0] as u16;
    let path_user_va = frame.gpr[1];
    let tail_arg = frame.gpr[2];
    let env_arg = frame.gpr[3];
    let from_el0 = frame.spsr & 0xF == 0;

    // Read the NUL-terminated `path` string from the parent's user
    // space. The parent's TTBR0 is still active (we switch to the
    // child only after a successful load), so its user VAs resolve —
    // but we read through **unprivileged loads** (`ldtrb`, see
    // [`read_user_cstr`]) because `path` is a *user* pointer per the
    // TOS ABI: the access must be permission-checked as EL0, exactly
    // like `Cconws` reads its string (a kernel-VA pointer here would
    // correctly fault, and the future m68k emulator passes user VAs
    // too). A missing NUL is bounded by the buffer size. A bad pointer is
    // the caller's bug: the read faults recoverably (`tos_api::uaccess`)
    // and the caller is terminated with the crash report, like the bus
    // error EmuTOS takes in the trap (ARCH03 0c, `uaccess_fault_tail`).
    let mut path_buf = [0u8; 128];
    // A user (EL0) caller passes a user-VA `path` read with unprivileged
    // loads; a kernel (EL1) caller — e.g. the built-in shell — passes a
    // kernel pointer that must be read with ordinary loads (an `ldtrb`
    // would fault as EL0 on a kernel page). Only the load modes take a
    // path: for `PE_BASEPAGEFLAGS` the argument is the `PRGFLAGS` value, for
    // the go modes it is unused (EmuTOS `xexec`, `bdos/proc.c:210-248`), and
    // reading either as a string would fault on a value like 0x10000007.
    let loads = mode == PE_LOADGO || mode == PE_LOAD;
    let path_len = if !loads {
        0
    } else if from_el0 {
        match read_user_cstr(path_user_va, &mut path_buf) {
            Ok(n) => n,
            Err(_) => return pexec_user_fault(frame),
        }
    } else {
        read_kernel_cstr(path_user_va, &mut path_buf)
    };
    let path_bytes = &path_buf[..path_len];

    // Command tail + environment. The load modes and the basepage modes
    // (5/7) consume them; for `PE_GO`/`PE_GOTHENFREE` the `x2` argument is
    // the PD pointer (a value here, not a string) so we must not read it.
    // `x2` = command-tail image (`init_pd_fields`), `x3` = environment
    // (`env == 0` → inherit `run->p_env`). Both are read from the caller's
    // address space with the same EL0/EL1 discipline as `path`.
    let mut tail_buf = [0u8; 128];
    let mut env_buf = [0u8; ENV_BUF_LEN];
    let (tail_bytes, env_spec): (&[u8], EnvSpec) =
        if loads || mode == PE_BASEPAGE || mode == PE_BASEPAGEFLAGS {
            let tlen = if tail_arg == 0 {
                0
            } else if from_el0 {
                match read_user_cstr(tail_arg, &mut tail_buf) {
                    Ok(n) => n,
                    Err(_) => return pexec_user_fault(frame),
                }
            } else {
                read_kernel_cstr(tail_arg, &mut tail_buf)
            };
            let env_spec = if env_arg == 0 {
                EnvSpec::Inherit
            } else {
                let Ok(elen) = read_env_block(env_arg, &mut env_buf, from_el0) else {
                    return pexec_user_fault(frame);
                };
                EnvSpec::Explicit(&env_buf[..elen])
            };
            (&tail_buf[..tlen], env_spec)
        } else {
            (&[][..], EnvSpec::None)
        };

    // GEMDOS trap tracer: record the launch *before* it happens — a
    // `PE_LOADGO` never comes back to this frame, so a post-hook would lose
    // exactly the interesting case. `path_bytes`/`tail_bytes` were read a few
    // lines above with this handler's own bounded, EL0-permission-checked
    // reader, so the tracer performs no caller-memory access of its own.
    if tos_api::gemdos::trace::ENABLED {
        let regs = frame_arg_regs(frame);
        tos_api::gemdos::trace::note_frame_call(0x4B, &regs, path_bytes, tail_bytes);
    }

    // SAFETY: trap context, BDOS giant lock implicit, single-CPU.
    let result = unsafe {
        tos_api::bdos::process::pexec::dispatch(
            mode, path_bytes, path_user_va, tail_arg, tail_bytes, env_spec,
        )
    };

    let child = match result {
        Ok(c) => c,
        Err(e) => {
            // `PexecError::errno`: a memory/size shortfall is `ENSMEM`
            // (EmuTOS `bdos/proc.c:295-300`), a malformed image `EPLFMT`
            // (`kpgmld.c:47-63`), an unknown mode `EINVFN`, a missing file
            // `EFILNF`, a PD the go modes do not accept `EIMBA`.
            let err = e.errno();
            frame.gpr[0] = err as i64 as u64;
            return;
        }
    };

    // `PE_LOAD` (D4) and the basepage modes 5/7: nothing is launched. The
    // parent keeps running; it receives the child's PD pointer — the
    // basepage address, readable in its own address space (ARCH03 stage 1,
    // EmuTOS `return (long)cur_p` / `return (long)p`, `bdos/proc.c:360,237`)
    // — and hands it back later to `PE_GO` / `PE_GOTHENFREE`. No TTBR0 switch, no frame redirect — just return to
    // the parent. The slot stays `Loaded` until `PE_GO` launches it (or the
    // parent terminates, `pexec::release_unstarted_of`).
    if !child.launch {
        frame.gpr[0] = child.handle;
        return;
    }

    // Launch path (PE_LOADGO / PE_GO): redirect execution to the child.
    // Snapshot the caller's full trap frame into the child slot's
    // `parent_frame` and mark it as a `Pexec` caller's frame, so
    // [`finish_terminate`]'s full-frame restore brings the caller back to
    // exactly this state with `gpr[0]` overwritten to the child's exit code.
    // The mark is what makes that restore independent of the exception level
    // the caller trapped from (DESK46): an EL1 caller — EmuCON's `run`
    // command — needs its callee-saved registers back just as much as an EL0
    // one, and `SPSR.M` cannot tell its frame from the partial kernel-launch
    // triple, which is `EL1t` too.
    let pf = &tos_api::bdos::process::PROCESS_TABLE[child.slot]
        .parent_frame;
    for i in 0..30 {
        pf.gpr(i).store(frame.gpr[i], Ordering::Release);
    }
    pf.lr().store(frame.lr, Ordering::Release);
    pf.elr().store(frame.elr, Ordering::Release);
    pf.spsr().store(frame.spsr, Ordering::Release);
    pf.sp_el0().store(frame.sp_el0, Ordering::Release);
    // …and the vector half of the caller's state (ARCH02): `v0..v31`, `FPCR`,
    // `FPSR` (ARM ARM DDI 0487M.d C5.2.8, C5.2.10) still hold what the caller
    // had at its `svc` — the kernel builds soft-float and nothing on the way
    // here touched them — and the child is about to rewrite all of them.
    // AAPCS64 lets the caller keep live values in `d8..d15` across the call,
    // and a TOS caller expects to come back intact (`docs/api/gemdos/process/
    // 4b-pexec.md`, native ABI). `finish_terminate` puts them back.
    save_fp_context(&tos_api::bdos::process::PROCESS_TABLE[child.slot].parent_fp);
    tos_api::bdos::process::PROCESS_TABLE[child.slot]
        .parent_frame_pexec
        .store(true, Ordering::Release);

    // Stamp the child slot's `parent_l0_pa` with the parent's
    // current `TTBR0_EL1`. Pterm's TTBR0 restore lands here on
    // exit so the parent's user mappings come back online.
    let parent_ttbr0: u64;
    // SAFETY: read-only system register access at EL1.
    unsafe {
        core::arch::asm!(
            "mrs {out}, ttbr0_el1",
            out = out(reg) parent_ttbr0,
            options(nomem, nostack, preserves_flags),
        );
    }
    // Only the root's address (BADDR): with ARCH03 stage-4 ASIDs on,
    // TTBR0_EL1[63:48] carries the parent's ASID, which the restore
    // (`install_ttbr0`) derives from the root again. With ASIDs off the
    // upper bits are 0 and this is the value as read.
    tos_api::bdos::process::PROCESS_TABLE[child.slot]
        .parent_l0_pa
        .store(parent_ttbr0 & 0x0000_FFFF_FFFF_FFFF, Ordering::Release);

    // Record the kernel-side parent linkage (EmuTOS `proc_go`,
    // bdos/proc.c:541 — `p->p_parent = run`). `RUN` still points at
    // the parent here (we set it to the child below), so `current()`
    // yields the parent's basepage. `Pterm` reads this slot index to
    // restore `RUN = parent` when the child terminates (xterm's
    // `run = run->p_parent`).
    let parent_bp = tos_api::bdos::process::current();
    // `p->p_parent = run` (EmuTOS `proc_go`, `bdos/proc.c:541`): the caller's
    // PD pointer; for a desk accessory EmuTOS's `run` — the program in front
    // or the accdesk PD (TASK/PROC03 B5) — while the rights, `parent_slot`
    // (the `Pterm` return) and `RUN` after it stay the accessory's; when the
    // caller is no process (an EL1 caller such as EmuCON's `run`), the
    // stand-in PD that is EmuTOS's `run` there (TASK/PPARENT,
    // `pexec::set_parent`).
    // SAFETY: trap context, BDOS lock implicit; the child holds a loaded image.
    unsafe { tos_api::bdos::process::pexec::set_parent(child.slot, parent_bp) };
    match tos_api::bdos::process::slot_for(parent_bp) {
        Some(parent_slot_idx) => {
            tos_api::bdos::process::PROCESS_TABLE[child.slot]
                .parent_slot
                .store(parent_slot_idx as u32, Ordering::Release);
        }
        None => {
            // No parent slot (boot-launched parent). Leave the
            // child's `parent_slot` at `NO_PARENT`; Pterm then clears
            // `RUN` to null on the child's exit.
        }
    }

    // …and take that parent **out of the cooperative run-set** for as long as
    // the child lives (PORT05). `Pexec` is synchronous — EmuTOS's `proc_go`
    // enters the child through `gouser()` and the parent's registers stay
    // frozen in its PD until the child's `xterm` (`bdos/proc.c:536-577`,
    // `:601-618`) — so a parent suspended here must not be on the ready
    // list. It would otherwise be one: every EL0 program that has been
    // through a blocking `evnt_*` wait is left `runnable` by `sleep`'s idle
    // exit, and the child's first block would switch straight back into its
    // own parent (`fresh`, i.e. re-entered at its entry point) instead of to an
    // accessory. Paired with `resume_parent_of_child` in `finish_terminate`.
    // A no-op when the parent owns no slot (the kernel launcher's programs).
    tos_api::bdos::process::sched::suspend_parent_for_child(child.slot);

    // Set RUN to the child's PD pointer so subsequent BDOS-pool
    // allocations from the child get tagged with the right owner
    // cookie (D9 wiring; EmuTOS `run = p`).
    tos_api::bdos::process::set_current(tos_api::bdos::process::pd_of(child.slot));
    // …and the slot is a started process from here on: `Pexec` 4/6 must not
    // take its PD for one still waiting to be started (ARCH03 2b — before,
    // a running `Pexec` child stayed `Loaded` and `Pexec(4, its PD)` started
    // it a second time). The kernel launcher and the scheduler mark their
    // processes the same way (`launch.rs`, `apply_switch`).
    tos_api::bdos::process::PROCESS_TABLE[child.slot]
        .set_state(tos_api::bdos::process::State::Running);

    // Switch TTBR0 from parent to child *with* a TLBI. We are in
    // kernel context here; the eret to EL0 is many instructions
    // away (handler return + EXC_BODY restore), so the QEMU
    // cortex-a72 wedge condition does not apply.
    //
    // Switch TTBR0 from parent to child. We can NOT use
    // [`switch_ttbr0_kernel`] here even though we're in kernel
    // context: this handler returns through EXC_BODY which `eret`s
    // to EL0 ~50 instructions later, and the QEMU cortex-a72
    // wedge condition (TLBI followed by eret-to-EL0) still triggers
    // at that distance. Two-step instead:
    //   1. `restore_boot_ttbr0()` — flushes TLB while installing
    //      the (otherwise-unused) boot tables. Safe: the boot
    //      install path was the original "TLBI works in kernel
    //      context" carve-out.
    //   2. `install_ttbr0(child.l0_pa)` — no TLBI, but the TLB is
    //      now empty from step 1 so the walker does fresh walks
    //      via the new TTBR0.
    //
    // SAFETY: trap context, single-CPU; no kernel code holds a
    // low-half VA reference across this point.
    // QUIRK-001 (QEMU cortex-a72 TCG only): the model wedges unless
    // enough work (specifically UART-MMIO-bearing prints) sits between
    // the TLBI in `restore_boot_ttbr0` and the `eret` to EL0 below. The
    // two prints below are kept deliberately to bracket the TTBR0 switch
    // and avoid the wedge; real hardware is unaffected.
    unsafe {
        crate::arch::aarch64::user_entry::restore_boot_ttbr0();
    }
    crate::kprintln!("[exec] preparing program (slot {})", child.slot);
    unsafe {
        tos_api::bdos::process::mmu::install_ttbr0(child.l0_pa);
    }
    crate::kprintln!("[exec] launching");

    // Rewrite the current frame so EXC_BODY's `eret` lands at the
    // child entry in EL0, on the child's user stack. The child
    // receives its basepage VA in `x0` per NewTOS's AArch64
    // process ABI.
    for i in 0..30 {
        frame.gpr[i] = 0;
    }
    frame.gpr[0] = child.basepage_va;
    frame.lr = 0;
    frame.elr = child.entry_va;
    frame.spsr = 0; // SPSR.M = EL0t, DAIF = 0
    // Initial user stack = the mapped TPA top (multi-page TPA), 4-KiB /
    // 16-byte aligned. The SDK crt0 re-derives the same value from the
    // basepage's p_hitpa, so a compiled program lands on a mapped stack.
    frame.sp_el0 = child.sp_el0;
}

/// Catch-all for IRQ / FIQ / SError and unimplemented synchronous
/// vectors. PHASE-02 has no interrupt support; reaching here is a
/// configuration bug and we halt with a diagnostic.
#[unsafe(no_mangle)]
pub extern "C" fn rust_unhandled_handler(frame: &mut ExceptionFrame) {
    unhandled(frame, "unhandled");
}

fn unhandled(frame: &ExceptionFrame, kind: &str) -> ! {
    // The fault report always leaves the machine, whatever the serial
    // switch says (SER01: output is off on board-rpi4 without `serial=on`).
    // Set first, so nothing printed on the way down is swallowed.
    crate::console::force_serial_tx_on();

    // AES trap tracer: an unhandled fault ends the run, so this is the last
    // chance to get the trace off the machine — the same reason the panic
    // handler flushes. Runs before the diagnostics so a fault *inside* the
    // print path cannot lose it. No-op — not even a call — without the
    // `aes-trace` feature; best-effort otherwise (every FS error swallowed,
    // never waits on input, QUIRK-002).
    if tos_api::aes::trace::ENABLED {
        tos_api::aes::trace::flush_on_exit();
    }
    if tos_api::gemdos::trace::ENABLED {
        tos_api::gemdos::trace::flush_on_exit();
    }

    let ec = esr_ec(frame.esr);
    let far = if is_abort(ec) { Some(read_far_el1()) } else { None };

    // Stack-overflow recognition (Phase 03 D6): a Data Abort whose
    // FAR_EL1 falls inside the stack guard region means the running
    // kernel code stored below the bottom of its main stack. Print a
    // dedicated banner before the generic dump so the cause is
    // unmistakable in the boot log.
    if let Some(addr) = far {
        let guard_start = crate::bios::machine::layout::stack_guard_start();
        let guard_end = crate::bios::machine::layout::stack_guard_end();
        if addr >= guard_start && addr < guard_end {
            crate::kprintln!(
                "\n[EXC] *** STACK OVERFLOW *** FAR=0x{:016x} \
                 (guard 0x{:016x}..0x{:016x})",
                addr, guard_start, guard_end,
            );
            crate::kprintln!(
                "[EXC] ESR=0x{:016x} ELR=0x{:016x} SPSR=0x{:016x}",
                frame.esr, frame.elr, frame.spsr,
            );
            crate::arch::aarch64::halt();
        }
    }

    crate::kprintln!(
        "\n[EXC] {} exception ({}): ESR=0x{:016x} ELR=0x{:016x} SPSR=0x{:016x}",
        kind, ec_name(ec), frame.esr, frame.elr, frame.spsr,
    );
    if let Some(addr) = far {
        crate::kprintln!("[EXC] FAR=0x{:016x}", addr);
    }
    crate::arch::aarch64::halt();
}
