//! BDOS process table — Phase 06 D2.
//!
//! Mirrors EmuTOS `bdos/proc.c` runtime state: a fixed-size table of
//! per-process kernel slots plus the two distinguished pointers
//! `run` (currently-executing process) and `nextrun` (process queued
//! for the next trap-return / Pexec hand-off).
//!
//! ## Scope of this deliverable
//!
//! D2 introduces only the **data structures** the rest of Phase 06
//! builds on:
//!
//!   * [`Process`] — the kernel-side slot wrapping the user-visible
//!     [`BasePage`].
//!   * [`MAX_PROCESSES`] — bound on concurrent processes (16 for now,
//!     plenty for normal Atari-style sequential `Pexec` chains).
//!   * [`PROCESS_TABLE`] — static array of slots in BSS.
//!   * [`RUN`] / [`NEXTRUN`] — `AtomicPtr<BasePage>` for lock-free
//!     reads from the SVC handler.
//!   * Allocation / free / lookup helpers for the slots.
//!
//! The slot's per-process L0 page-table phys (`l0_pa`) is reserved
//! but not yet populated — Phase 06 D8 fills it during the per-
//! process TTBR0 install. ELF loading (D3), `Pexec` modes (D4/D5),
//! `Pterm` cleanup (D6) and `m_own` integration (D9) all consume
//! this table without altering its shape.
//!
//! ## Concurrency model
//!
//! Each [`Process`] field is atomic so the SVC sync handler — which
//! runs without taking the BDOS giant lock — can read the running
//! process's basepage in a single relaxed-acquire load. Slot
//! allocation uses a CAS on the `state` field so two concurrent
//! `Pexec` calls (Phase-08+ scenario) don't claim the same slot.
//! All other writes happen under the BDOS giant lock that the
//! GEMDOS dispatcher implicitly holds (Phase 06 D6 will replace the
//! "implicit single-CPU" lock with an explicit per-CPU SpinMutex).

use core::ptr;
use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicU64, AtomicU8, Ordering,
};

use tos_types::BasePage;

pub mod arena;
pub mod asid;
pub mod elf;
pub mod loader;
pub mod memprot;
pub mod mmu;
pub mod pterm;
pub mod pexec;
pub mod registry;
pub mod reloc;
pub mod sched;
pub mod standin;
pub mod usermem;

// ─── Bounds + state encoding ────────────────────────────────────────

/// Maximum number of concurrently-tracked processes. EmuTOS does not
/// bound this explicitly (it relies on memory pressure to fail
/// `Pexec`); we pick a small static cap because TOS process chains
/// are sequential and shallow — even AES + a desktop accessory + a
/// running app rarely exceeds five live processes.
pub const MAX_PROCESSES: usize = 16;

/// Maximum EL0-upcall nesting depth per process (cooperative-kernel
/// building block 1, `docs/analysis/coop-kernel/10-upcall-signal-design.md`
/// §6.3). Bounds the per-slot saved-context stack [`Process::upcall_ctx`];
/// a delivery that would exceed it terminates the process with
/// `Pterm(-998)` — the FreeMiNT "stack overflow" ALERT analogue
/// (`references/freemint/sys/arch/sig_mach.c:138`). FreeMiNT instead bounds
/// nesting by its 16-KiB system stack (~12 B/nest, `sig_mach.c:141-144`);
/// NewTOS bounds it by this array. Memory: `ParentFrame` × 8 × 16 slots
/// ≈ 34 KiB BSS (design §6.3).
pub const MAX_UPCALL_NEST: usize = 8;

/// Sentinel value for `parent_slot` indicating "no parent" (the boot
/// process loaded by the kernel itself).
pub const NO_PARENT: u32 = u32::MAX;

/// Process-slot lifecycle. Encoded as `u8` so the field can sit in
/// an `AtomicU8` for lock-free state probes.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    /// Slot is unused; safe to claim via [`allocate_slot`].
    Free = 0,
    /// Slot is reserved, basepage allocated, but the program has not
    /// yet started executing. Set by `Pexec` mode 3 (`PE_LOAD`) and
    /// by mode 0 (`PE_LOADGO`) before the first eret.
    Loaded = 1,
    /// Slot is the active process. Equals [`RUN`]'s target.
    Running = 2,
    /// `Pterm`-marked but not yet reaped — its allocations have not
    /// been freed by `free_all_owned` yet. Transient.
    Terminated = 3,
}

impl State {
    /// Decode a raw `u8` from [`Process::state`]. Defensive: any
    /// value outside `0..=3` is treated as `Free`.
    #[inline]
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Loaded,
            2 => Self::Running,
            3 => Self::Terminated,
            _ => Self::Free,
        }
    }
}

// ─── Per-process ABI (pointer / `long` width) ───────────────────────

/// Application binary interface of a loaded program, derived from its
/// ELF class at load time (ADR-020, the ILP32 pivot). It is the width
/// of a C pointer and `long` in the program's address space, and thus
/// how the shared GEM trap (`svc #2`) must interpret the AES/VDI
/// parameter block the app hands in: an ILP32 app builds the block from
/// **4-byte** pointers, an LP64 app from **8-byte** pointers. The WORD
/// arrays the pointers point at are 16-bit either way; only the pointer
/// reads (the six/five block pointers, and any `long`/pointer inside the
/// AES `addrin`/`addrout` arrays) change width.
///
/// Encoded as `u8` so it can live in an [`AtomicU8`] alongside the rest
/// of the lock-free [`Process`] slot state.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Abi {
    /// Native AArch64 LP64: 8-byte pointers and `long`. Kernel-launched
    /// ELF64 programs, the embedded selftests, and every in-kernel (EL1)
    /// GEM caller (kernel code, the boot VDI/AES selftests) use this.
    Lp64 = 0,
    /// GNU ILP32 (`aarch64-elf-gcc -mabi=ilp32`, `ELFCLASS32`): 4-byte
    /// pointers and `long`. Source-port GEM apps built against the ILP32
    /// SDK (libc-ilp32 / gemlib-ilp32 / cflib-ilp32) run in this ABI.
    Ilp32 = 1,
}

impl Abi {
    /// Decode a raw `u8` from [`Process::abi`]. Defensive: any value
    /// other than `1` (`Ilp32`) is treated as `Lp64`, so a fresh/zeroed
    /// slot reads as the native ABI.
    #[inline]
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Ilp32,
            _ => Self::Lp64,
        }
    }

    /// Read the `i`-th pointer-width word from a parameter-block array
    /// based at `base`, widened to `u64`: an 8-byte load at 8-byte stride
    /// for [`Abi::Lp64`], a 4-byte (zero-extended) load at 4-byte stride
    /// for [`Abi::Ilp32`]. Used for both the AES/VDI block pointers and
    /// the AES `addrin` `long`/pointer elements (a `long` is exactly one
    /// pointer wide in each ABI).
    ///
    /// # Safety
    /// `base + i * width` must address a valid, readable slot.
    #[inline]
    pub unsafe fn read_ptr(self, base: u64, i: usize) -> u64 {
        match self {
            // SAFETY: forwarded to the caller's bounds contract.
            Self::Lp64 => unsafe { (base as *const u64).add(i).read() },
            // SAFETY: forwarded to the caller's bounds contract.
            Self::Ilp32 => unsafe { (base as *const u32).add(i).read() as u64 },
        }
    }

    /// Write `v` into the `i`-th pointer-width slot of a parameter-block
    /// array based at `base` (the inverse of [`Self::read_ptr`]): an
    /// 8-byte store for [`Abi::Lp64`], the low 4 bytes for [`Abi::Ilp32`].
    /// Used by the AES `addrout` writers (`rsrc_gaddr`, …).
    ///
    /// # Safety
    /// `base + i * width` must address a valid, writable slot.
    #[inline]
    pub unsafe fn write_ptr(self, base: u64, i: usize, v: u64) {
        match self {
            // SAFETY: forwarded to the caller's bounds contract.
            Self::Lp64 => unsafe { (base as *mut u64).add(i).write(v) },
            // SAFETY: forwarded to the caller's bounds contract.
            Self::Ilp32 => unsafe { (base as *mut u32).add(i).write(v as u32) },
        }
    }

    /// [`Self::read_ptr`] in the caller's address space `space` (ARCH03
    /// stage 3a): an EL0 program's parameter block is read with its rights
    /// through [`crate::uaccess`], an in-kernel caller's as the kernel
    /// address it is. `Err` when the program may not read the slot.
    #[inline]
    pub fn read_ptr_in(self, space: crate::uaccess::Space, base: u64, i: usize) -> Result<u64, crate::uaccess::UserFault> {
        match self {
            Self::Lp64 => space.get_u64(base + 8 * i as u64),
            Self::Ilp32 => space.get_u32(base + 4 * i as u64).map(u64::from),
        }
    }

    /// [`Self::write_ptr`] in the caller's address space `space`.
    #[inline]
    pub fn write_ptr_in(self, space: crate::uaccess::Space, base: u64, i: usize, v: u64) -> Result<(), crate::uaccess::UserFault> {
        match self {
            Self::Lp64 => space.put_u64(base + 8 * i as u64, v),
            Self::Ilp32 => space.put_u32(base + 4 * i as u64, v as u32),
        }
    }
}

// ─── Process slot ───────────────────────────────────────────────────

/// Per-process kernel state. Sits next to (but not inside) the
/// user-visible [`BasePage`]: applications never see this struct, so
/// it can carry kernel-only fields (page-table root, parent slot
/// index, etc.) without touching the bit-exact 256-byte basepage
/// ABI.
///
/// All fields are atomic so the SVC handler can probe them lock-free.
/// Mutations during `Pexec` / `Pterm` happen under the BDOS giant
/// lock, but readers don't need to acquire it.
pub struct Process {
    /// Slot lifecycle (see [`State`]).
    pub state: AtomicU8,

    /// Program ABI (pointer / `long` width), recorded from the ELF class
    /// by the loader (ADR-020). Read lock-free by the GEM trap dispatch
    /// via [`current_abi`] so the AES/VDI parameter-block readers use the
    /// caller's pointer width. Encoded as `u8` per [`Abi`]; a zeroed slot
    /// reads as [`Abi::Lp64`].
    pub abi: AtomicU8,

    /// Kernel-side virtual address of the [`BasePage`] living at the
    /// start of this process's TPA — the high-half alias ([`kalias`]) of
    /// the process's **PD pointer** ([`pd_of`]): the user address of the
    /// basepage, which since ARCH03 stage 1 is the start of its TPA block in
    /// the shared address space (VA = PA). The PD pointer is the value
    /// `Pexec(PE_LOAD)` returns, `p_parent` and `os_run` hold, and `RUN` /
    /// `m_own` carry (EmuTOS `run`, `bdos/proc.c`).
    pub basepage_kva: AtomicPtr<BasePage>,

    /// Physical address of the [`BasePage`] — equal to its PD pointer
    /// ([`pd_of`]) since ARCH03 stage 1 (VA = PA).
    pub basepage_pa: AtomicU64,

    /// For a program loaded with `Pexec(PE_LOAD)` and not started yet: the
    /// PD pointer of the process that loaded (and owns) it, so the slot is
    /// released when that process ends ([`pexec::release_unstarted_of`]).
    /// `null` otherwise.
    pub load_owner: AtomicPtr<BasePage>,

    /// For a child started with `Pexec(PE_GO)`: the PD of the memory space
    /// it shares — its caller's (or the caller's own space, for a thread of
    /// a thread). FreeMiNT runs a mode-4 child as a thread in its caller's
    /// memory space (`FORK_SHAREVM`, `sys/k_exec.c:224-231`; plan E6), so it
    /// counts as an owner of the caller's blocks for `Mfree`/`Mshrink`
    /// (E15, [`current_space`]). `null` for every other process.
    pub space_pd: AtomicPtr<BasePage>,

    /// Phys of the per-process L0 page-table root that gets written
    /// to `TTBR0_EL1` when this process becomes [`State::Running`].
    /// `0` until Phase 06 D8 builds the tables for this process.
    pub l0_pa: AtomicU64,

    /// User VA of the program's entry point (the ELF `e_entry`). The
    /// loader records it so `Pexec(PE_GO)` can launch a slot that was
    /// loaded earlier by `Pexec(PE_LOAD)` without re-parsing the image.
    pub entry_va: AtomicU64,

    /// Initial `SP_EL0` for a **fresh** entry of this process — the mapped TPA
    /// top the loader computed (`loader::LoadedImage::sp_el0`). `0` for a slot
    /// whose image was not loaded through `pexec::load` (the embedded selftest
    /// blobs), in which case the entry path keeps its own default.
    ///
    /// The kernel launcher passes the value straight to its EL0-entry helper;
    /// this field is for the other way in — the scheduler switch
    /// (`apply_switch`, `Switch::fresh`), which is how a desk accessory is
    /// first run (ACC02/ACC03).
    pub sp_el0: AtomicU64,

    /// Index of the parent's slot in [`PROCESS_TABLE`], or
    /// [`NO_PARENT`] for the boot-loaded root process. Used by
    /// `Pterm` to return control to the parent's resume PC.
    pub parent_slot: AtomicU32,

    /// The `p_parent` value this process was started with — what
    /// [`pexec::set_parent`] / [`pexec::set_kernel_parent`] wrote (EmuTOS
    /// `proc_go`, `p->p_parent = run`, `bdos/proc.c:541`): the caller's PD,
    /// or the stand-in PD that was the kernel's `run`. `0` for a process
    /// that was never started that way (an accessory, a selftest launch).
    ///
    /// `Pterm` compares the dying process's live `p_parent` with it
    /// (TASK/PROC03 B4, `pterm::return_target`): equal — return to the
    /// recorded parent, the normal case; changed — EmuTOS's `run =
    /// run->p_parent` (`bdos/proc.c:609`) resumes whatever the field names,
    /// which NewTOS honours only for a PD that has a resume point on this
    /// process's launch chain (`_run-ownership-curdir.md` §6.4 D-5).
    ///
    /// [`pexec::set_parent`]: pexec::set_parent
    /// [`pexec::set_kernel_parent`]: pexec::set_kernel_parent
    pub launch_parent: AtomicU32,

    /// Start order of this process — the value of a kernel-wide counter at
    /// the moment it was started through [`pexec::set_parent`] /
    /// [`pexec::set_kernel_parent`] (EmuTOS `proc_go`, which ends with `run =
    /// p`, `bdos/proc.c:575`). `0` for a process that was never started that
    /// way (a desk accessory — never `proc_go`ed — or a selftest launch) and
    /// again from the moment its `Pterm` begins (`run = run->p_parent`,
    /// `:609`, before `ixterm`).
    ///
    /// [`emutos_run`] is the live process with the largest value: EmuTOS's
    /// `run` as an accessory sees it (TASK/PROC03 B5,
    /// `_run-ownership-curdir.md` §1.2/§1.3).
    ///
    /// [`pexec::set_parent`]: pexec::set_parent
    /// [`pexec::set_kernel_parent`]: pexec::set_kernel_parent
    pub start_seq: AtomicU64,

    /// The parent's cooperative run-set membership ([`Self::runnable`]) at the
    /// instant it launched **this** child, so the child's `Pterm` can put it
    /// back exactly as it found it.
    ///
    /// GEMDOS `Pexec` is a *synchronous* call: EmuTOS's `proc_go` sets
    /// `run = p` and enters the child through `gouser()`
    /// (`references/emutos/bdos/proc.c:536-577`); the parent's context is
    /// frozen in its PD and only `xterm`'s `run = run->p_parent` +
    /// `termuser()` (`:601-618`) brings it back. The parent therefore **must
    /// not** be a candidate of NewTOS's cooperative round-robin while the
    /// child lives — in EmuTOS it cannot be, because parent and child share
    /// one AESPD and `dsptch()` has nothing to pick
    /// (`docs/analysis/coop-kernel/emutos-cooperative-model.md` §5.2).
    ///
    /// Written by [`sched::suspend_parent_for_child`], consumed by
    /// [`sched::resume_parent_of_child`]. Kept on the **child** slot for the
    /// same reason [`Self::parent_frame`] / [`Self::parent_l0_pa`] are: it is
    /// per-launch state that must die with the launch.
    ///
    /// [`sched::suspend_parent_for_child`]: sched::suspend_parent_for_child
    /// [`sched::resume_parent_of_child`]: sched::resume_parent_of_child
    pub parent_runnable: AtomicBool,

    /// Snapshot of the parent's trap frame. Pterm copies this back
    /// over the child's trap frame so the post-handler `eret`
    /// resumes the parent at exactly the state it was in when it
    /// dispatched the call that led to this child.
    ///
    /// Two writers populate this:
    ///   * **`Pexec` (real EL0 parent)** — copies the parent's
    ///     trap frame here verbatim before redirecting EL0 to the
    ///     child. The eventual `Pterm` restores the parent at its
    ///     original EL0 PC (the SVC's resume point) with `x0 = rc`.
    ///   * **Kernel-launched test driver (D6 path)** — writes only
    ///     the resume PC, kernel SP, and `SPSR.M = EL1t` via the
    ///     [`crate::bdos::process::loader`]/EL0-entry asm. The
    ///     gpr / lr / spsr.flags fields stay zero; the kernel
    ///     resume code reads its callee-saved regs from the local
    ///     stack frame so junk in `frame.gpr` is harmless.
    pub parent_frame: ParentFrame,

    /// Phys address of the parent's `L0` page-table root. Pterm
    /// installs this in `TTBR0_EL1` (with a TLBI, since we run in
    /// kernel context not eret-adjacent) before resuming the parent.
    /// `0` means "no parent process" — boot is the parent, and the
    /// handler restores the Stage-1 boot tables instead.
    ///
    /// Phase 06 D6 always sees `0` here (boot launches the test
    /// process directly). Phase 06 D3 `Pexec` populates it with the
    /// parent's TTBR0_EL1 read at trap entry, so the child's
    /// `Pterm` brings the parent's address space back online.
    pub parent_l0_pa: AtomicU64,

    /// [`Self::parent_frame`] holds a **whole** EL1 trap frame, not the partial
    /// `elr`/`sp_el0`/`spsr` triple the EL0-entry helper writes — the
    /// desk-accessory case (ACC03). An accessory is not entered through
    /// `enter_el0_with_resume_capture`: it is switched to from a parked EL1
    /// kernel client (the boot path's `wait_for_accs`; until DESK43 U10 also
    /// the built-in desktop inside a blocking `evnt_*`), and that client's frame is copied in here so a
    /// *terminating* accessory resumes it the same way a *blocking* one does.
    /// Such a frame must be restored in full — its `x0`/`x1` carry the GEM-trap
    /// selector and parameter block of an `svc` that is about to re-execute —
    /// whereas the partial kernel-launch frame must not have its `gpr` touched
    /// at all (see `finish_terminate`).
    pub parent_frame_full: AtomicBool,

    /// [`Self::parent_frame`] is the trap frame of a **`Pexec` caller** — an
    /// `svc` that has not returned yet and whose result is the child's exit
    /// code (`docs/api/gemdos/process/4b-pexec.md`, "Trap interface":
    /// "`PE_LOADGO (0)` … the child's `xterm` delivers the child's exit code
    /// … as *this* call's return value via the parent's saved trap frame";
    /// EmuTOS `bdos/proc.c:601-618`). Set by `handle_pexec` for every launch
    /// it redirects, **whichever exception level the caller trapped from**:
    /// the whole register file has to come back, because the caller's
    /// compiler assumed an `svc` preserves the callee-saved registers, and
    /// `x0` has to carry the exit code.
    ///
    /// Distinct from [`Self::parent_frame_full`], which is the adopted
    /// kernel-park of a terminating desk accessory: that frame is restored in
    /// full **without** the exit code (its `svc` re-executes), and consuming
    /// it also disarms the park. The two are never set together, and a
    /// `parent_frame` with neither flag is the partial kernel-launch triple
    /// the EL0-entry helper writes.
    pub parent_frame_pexec: AtomicBool,

    /// The FP/SIMD half of [`Self::parent_frame`] for a **`Pexec` caller** —
    /// `v0..v31`, `FPCR`, `FPSR` as they were at the caller's `svc`
    /// (ARCH02). Written by `handle_pexec` together with the GPR snapshot,
    /// restored by `finish_terminate` on the [`Self::parent_frame_pexec`]
    /// branch, so the caller resumes with the vector file it trapped with and
    /// not the one its child left behind. Only meaningful while
    /// `parent_frame_pexec` is set; the two other kinds of `parent_frame`
    /// (the adopted kernel park, the partial kernel-launch triple) resume EL1
    /// code, which never uses these registers (see [`FpContext`]).
    ///
    /// Kept on the **child** slot like the rest of the parent linkage, and
    /// distinct from [`Self::saved_fp`] of the same slot, which the child
    /// itself fills whenever *it* yields while its parent waits.
    pub parent_fp: FpContext,

    // ── cooperative scheduler (Phase 06 EC4 / `sched`) ──
    /// Full EL0 context saved when this process yields via `Pwait`
    /// (so the round-robin can resume it exactly where it left off).
    /// Distinct from [`Self::parent_frame`], which carries the
    /// parent-resume linkage for `Pexec`/`Pterm`.
    pub saved_frame: ParentFrame,

    /// The FP/SIMD half of [`Self::saved_frame`] — see [`FpContext`]. Written
    /// when this slot yields, restored on a `!fresh` scheduler resume.
    pub saved_fp: FpContext,

    /// In the scheduler's run-set (a peer in the cooperative
    /// round-robin). Cleared when the process terminates.
    ///
    /// **Written only by [`sched`]** ([`sched::ready`], [`sched::unready`],
    /// [`sched::admit`], the wakes): the bit is membership, and the ready
    /// list's order (EmuTOS `rlr`/`drl`, A115) is kept beside it — a bare store
    /// would add a member the order does not know.
    ///
    /// [`sched::ready`]: sched::ready
    /// [`sched::unready`]: sched::unready
    /// [`sched::admit`]: sched::admit
    pub runnable: AtomicBool,

    /// `saved_frame` holds a valid context (the process has run and
    /// yielded). `false` = never entered → start fresh at `entry_va`.
    pub has_ctx: AtomicBool,

    /// Wait-condition key when this process is **WAITING** — cooperative-kernel
    /// step 3a (sleep/wake substrate, `docs/analysis/coop-kernel/`
    /// `20-sleepwake-substrate-design.md` §3.1/§3.2/§4.1). `0` = not waiting
    /// (the process is RUNNING or READY). A nonzero value is a wake-class key
    /// built by [`sched::wait_input`]/[`sched::wait_mesg`]/[`sched::wait_timer`]
    /// (class in the high bits, pid/deadline below, §8.1.3): the process sleeps
    /// in [`sched::sleep`] until [`sched::wake`] — or the idle wake-pass — clears
    /// it (WAITING → READY). Layered over the existing [`Self::runnable`] bit,
    /// not a separate list: WAITING is `runnable == false && wait_cond != 0`,
    /// READY is `runnable == true` (§3.2 "field, not list").
    ///
    /// [`sched::sleep`]: super::process::sched::sleep
    /// [`sched::wake`]: super::process::sched::wake
    /// [`sched::wait_input`]: super::process::sched::wait_input
    /// [`sched::wait_mesg`]: super::process::sched::wait_mesg
    /// [`sched::wait_timer`]: super::process::sched::wait_timer
    pub wait_cond: AtomicU64,

    /// Persisted absolute `monotonic_millis` deadline of a timer-bearing wait
    /// (`evnt_timer`/`wait_ms`/`evnt_multi`-`MU_TIMER`) — S2-restart idempotency
    /// (`docs/analysis/coop-kernel/20-sleepwake-substrate-design.md` §4.3 OPEN,
    /// step-4-prep). `0` = no timer deadline armed. The **first** entry of such a
    /// wait computes `monotonic_millis() + ms` and stores it here; an **S2
    /// restart** (the re-executed `svc` re-enters the wait from the top) finds
    /// the field already set and REUSES it, instead of recomputing `now + ms`
    /// (which would push the deadline forward by `ms` on every restart with a
    /// live peer, so it would never converge). Cleared on **every** completion
    /// path of the wait (it fires and returns — including an input/message event
    /// firing before the timer in `evnt_multi`) so a later unrelated timer wait
    /// recomputes a fresh deadline. Armed / reused / cleared via
    /// [`sched::arm_or_get_deadline`] / [`sched::clear_deadline`]; detection of
    /// first-entry-vs-restart is exactly this field being `0` vs set.
    ///
    /// [`sched::arm_or_get_deadline`]: super::process::sched::arm_or_get_deadline
    /// [`sched::clear_deadline`]: super::process::sched::clear_deadline
    pub syscall_deadline: AtomicU64,

    // ── cooperative-kernel EL0 upcall / signal delivery (building block 1,
    //    docs/analysis/coop-kernel/10-upcall-signal-design.md §6) ──
    /// Kernel-side stack of interrupted EL0 contexts, one per active
    /// (possibly nested) upcall. `deliver_upcall` pushes the pre-upcall
    /// frame here (with the syscall result already in `gpr[0]`, §6.2);
    /// `Psigreturn` (GEMDOS 0x11A) pops + restores it. Bounded by
    /// [`MAX_UPCALL_NEST`]. Kernel-side rather than on the EL0 stack (design
    /// §6.1: an EL0 app cannot corrupt the resume context). Styled on
    /// [`Self::saved_frame`].
    pub upcall_ctx: [ParentFrame; MAX_UPCALL_NEST],

    /// Mask bit each [`Self::upcall_ctx`] level was delivered with, so
    /// `Psigreturn` clears exactly the bit its level set. FreeMiNT recovers
    /// the signal number from the on-stack frame (`sig_mach.c:277,291`); with
    /// the kernel-side context stack we key it off the nesting depth instead.
    pub upcall_ctx_mask: [AtomicU32; MAX_UPCALL_NEST],

    /// [`Disposition`] each [`Self::upcall_ctx`] level was delivered with
    /// (cooperative-kernel step 2), encoded per [`Disposition::encode`]. On
    /// `Psigreturn`, the top level's word decides resume (step-1 default,
    /// `0`) vs terminate-after (etv_term). A resume level always stores `0`,
    /// so this array is transparent to the step-1 resume path.
    pub upcall_ctx_disp: [AtomicU64; MAX_UPCALL_NEST],

    /// Current upcall nesting depth (= number of live [`Self::upcall_ctx`]
    /// entries). `deliver_upcall` increments, `Psigreturn` decrements.
    pub upcall_depth: AtomicU8,

    /// Set of upcall-type bits currently being handled — the FreeMiNT
    /// `p_sigmask` analogue (`sig_mach.c:291`). A type stays masked while its
    /// handler runs and is cleared on the matching `Psigreturn`.
    pub upcall_mask: AtomicU32,

    /// Pending (posted, not-yet-delivered) upcall handler VA, or `0` for
    /// none — the presence sentinel of the [`UpcallDesc`] posted by
    /// [`post_upcall`] and consumed by [`take_pending_upcall`] (the per-slot
    /// [`request_terminate`]/[`take_pending_terminate`] analogue, design §7.1).
    pub upcall_pending_handler: AtomicU64,

    /// Argument registers (x0..x3) of the pending upcall.
    pub upcall_pending_args: [AtomicU64; 4],

    /// Mask bit of the pending upcall.
    pub upcall_pending_mask: AtomicU32,

    /// [`Disposition`] of the pending upcall (cooperative-kernel step 2),
    /// encoded per [`Disposition::encode`]; `0` (= [`Disposition::Resume`])
    /// for step-1 consumers.
    pub upcall_pending_disp: AtomicU64,
}

/// A pending EL0 upcall: the handler VA to jump to, up to four
/// argument-register values (x0..x3, AAPCS64), the mask bit identifying
/// the upcall type (set on delivery, cleared by `Psigreturn`), and the
/// [`Disposition`] on handler return. The consumers (etv_*, vex,
/// G_USERDEF, eralert, …) supply the handler VA directly — this is
/// disposition-agnostic in the POSIX sense (no signal-number table,
/// design §1), but each delivery carries its own resume-vs-terminate
/// [`Disposition`]. Analogue of a FreeMiNT signal resolved to its
/// `sa_handler` (`sig_mach.c`).
#[derive(Copy, Clone)]
pub struct UpcallDesc {
    /// EL0 VA of the handler the process runs before resuming.
    pub handler_va: u64,
    /// Handler arguments, delivered in x0..x3.
    pub args: [u64; 4],
    /// Upcall-type bit (masked while the handler runs).
    pub mask_bit: u32,
    /// What happens when the handler returns via `Psigreturn`
    /// (cooperative-kernel step 2). Step-1 consumers use
    /// [`Disposition::Resume`]; the etv_term consumer uses
    /// [`Disposition::Terminate`].
    pub disposition: Disposition,
}

/// What the kernel does when an EL0 upcall handler returns via `Psigreturn`
/// (cooperative-kernel step 2, `docs/analysis/coop-kernel/`
/// `10-upcall-signal-design.md`). Extends the step-1 mechanism, which only
/// ever resumed the interrupted context, with a "run the handler, then
/// terminate" disposition — the shape the **etv_term** process-terminate
/// hook needs (EmuTOS `xterm`, `bdos/proc.c:601-618`: `protect_v(userterm)`
/// runs the installed handler, then teardown continues to the parent).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Disposition {
    /// Restore the interrupted EL0 context and resume it (step-1 default).
    Resume,
    /// After the handler returns, terminate the process instead of resuming:
    /// route into the standard `Pterm` teardown with exit code `rc` and,
    /// for `Ptermres`, the keep-resident teardown (`reserve`). This is the
    /// etv_term "notify before I die" contract.
    Terminate {
        /// Original `Pterm` exit code (UWORD, already masked to 16 bits).
        rc: i32,
        /// `Some(blkln)` for `Ptermres` (0x31) — shrink the TPA to `blkln`
        /// and keep the process's memory resident; `None` for
        /// `Pterm`/`Pterm0`.
        reserve: Option<i32>,
    },
    /// The handler was the synchronous AES `G_USERDEF` callout (AES-as-thread
    /// design, `docs/analysis/coop-kernel/30-aes-thread-design.md` §3.4(e)): the
    /// EL0 stub ran the app's `ub_code(PARMBLK*)` and stored its WORD result in
    /// the userdef ret-slot. On `Psigreturn` the kernel reads that WORD, resumes
    /// the stalled AES fiber (`aes::thread::resume`), and — when the fiber
    /// finally completes — restores the original client `wind_*` frame from
    /// `aes::thread::AES_CALL_ORIGIN` instead of the interrupted (stub) context.
    /// Neither resume-in-place (the stub context is discarded) nor terminate.
    UserdefReturn,
}

impl Disposition {
    /// Tag bit marking a [`Disposition::Terminate`] in the `AtomicU64`
    /// encoding used by the pending / per-nest slots. A `0` word decodes to
    /// [`Disposition::Resume`], keeping the step-1 default a plain zero.
    const TERM_TAG: u64 = 1 << 63;
    /// `reserve` (Ptermres) flag in the `AtomicU64` encoding; the `blkln`
    /// LONG sits in bits 16..48, the exit code (a UWORD) in bits 0..16.
    const RESERVE_TAG: u64 = 1 << 62;
    /// Tag bit marking a [`Disposition::UserdefReturn`] (AES-as-thread design
    /// §6.1.6). Distinct from [`Self::TERM_TAG`]; a `0` word still decodes to
    /// [`Disposition::Resume`], keeping the step-1 default a plain zero.
    const USERDEF_TAG: u64 = 1 << 61;

    /// Encode into the per-nest / pending `AtomicU64`. [`Disposition::Resume`]
    /// is `0`, so a freshly-zeroed slot means "resume".
    pub fn encode(self) -> u64 {
        match self {
            Disposition::Resume => 0,
            Disposition::Terminate { rc, reserve } => {
                Self::TERM_TAG
                    | match reserve {
                        Some(blkln) => Self::RESERVE_TAG | ((blkln as u32 as u64) << 16),
                        None => 0,
                    }
                    | (rc as u32 as u64 & 0xFFFF)
            }
            Disposition::UserdefReturn => Self::USERDEF_TAG,
        }
    }

    /// Decode a word written by [`Self::encode`].
    pub fn decode(v: u64) -> Self {
        if v & Self::TERM_TAG != 0 {
            Disposition::Terminate {
                rc: (v & 0xFFFF) as i32,
                reserve: (v & Self::RESERVE_TAG != 0).then_some((v >> 16) as u32 as i32),
            }
        } else if v & Self::USERDEF_TAG != 0 {
            Disposition::UserdefReturn
        } else {
            Disposition::Resume
        }
    }
}

/// Upcall-type mask bit for the **etv_term** process-terminate hook (the
/// FreeMiNT `p_sigmask` per-consumer bit, `sig_mach.c:291`). Each future
/// upcall consumer (etv_critic, vex, …) claims its own bit; etv_term is the
/// first real one.
pub const UPCALL_MASK_ETV_TERM: u32 = 1 << 0;

/// Upcall-type mask bit for the synchronous AES `G_USERDEF` callout
/// (AES-as-thread design, `docs/analysis/coop-kernel/30-aes-thread-design.md`
/// §3.4). Set while the EL0 stub runs the app's `ub_code`, cleared on its
/// `Psigreturn`. Distinct from [`UPCALL_MASK_ETV_TERM`].
pub const UPCALL_MASK_AES_USERDEF: u32 = 1 << 1;

/// Upcall-type mask bit for the VDI mouse-wheel vector `vex_wheelv`
/// (upcall design §1, consumer **vex**). Set while the application's wheel
/// routine runs at EL0, cleared on its `Psigreturn`. Distinct from the other
/// two bits.
pub const UPCALL_MASK_VEX_WHEEL: u32 = 1 << 2;

/// Upcall-type mask bit for the IKBD **`kbdvec`** hook (KEYS02,
/// `docs/api/xbios/22-kbdvbase.md` B4). Set while the program's `kbdvec`
/// routine runs at EL0 for one scancode, cleared on its `Psigreturn`. While it
/// is set no further `kbdvec` byte is delivered (nesting depth stays 1); the
/// next queued byte follows on that `Psigreturn`. Not posted through
/// [`post_upcall`] — whose single pending slot would lose a make when its break
/// arrives in the same poll — but drained from the per-process ring in
/// `xbios::kbdvecs`.
pub const UPCALL_MASK_KBDVEC: u32 = 1 << 3;

// ─── Parent-frame snapshot ──────────────────────────────────────────

/// Index of `lr` (`x30`) in [`ParentFrame::regs`].
pub const PARENT_FRAME_LR: usize = 30;
/// Index of `elr_el1` in [`ParentFrame::regs`].
pub const PARENT_FRAME_ELR: usize = 31;
/// Index of `spsr_el1` in [`ParentFrame::regs`].
pub const PARENT_FRAME_SPSR: usize = 32;
/// Index of `sp_el0` in [`ParentFrame::regs`].
pub const PARENT_FRAME_SP_EL0: usize = 33;
/// Total number of `u64` slots in [`ParentFrame`] — `30` for
/// `x0..x29` plus four one-off slots above.
pub const PARENT_FRAME_LEN: usize = 34;

// ─── FP/SIMD context (ACC03) ────────────────────────────────────────

/// `u64` slots in an [`FpContext`]: `v0..v31` are 128-bit (two slots each),
/// then `FPCR` and `FPSR`.
pub const FP_CONTEXT_LEN: usize = 32 * 2 + 2;
/// Index of `FPCR` in [`FpContext::slots`].
pub const FP_CONTEXT_FPCR: usize = 64;
/// Index of `FPSR` in [`FpContext::slots`].
pub const FP_CONTEXT_FPSR: usize = 65;

/// The **FP/SIMD half** of a saved EL0 context — `v0..v31`, `FPCR`, `FPSR`.
///
/// [`ExceptionFrame`] carries only the general-purpose registers, which is
/// enough for every path that existed before desk accessories: an EL1 caller
/// never touches the vector file (both kernel crates build for
/// `aarch64-unknown-none-softfloat`), and a kernel-launched program runs to its
/// `Pterm` without another EL0 context ever being scheduled in between.
///
/// A desk accessory breaks both assumptions: it is parked mid-function while a
/// *hard-float* EL0 program (anything from `sdk/build.sh`, which targets
/// `aarch64-unknown-none-elf`) runs and rewrites the whole vector file.
/// Measured: the accessory's `evnt_mesag` loop keeps its AES control block
/// `{23, 0, 1, 1}` in `d1` as a loop-invariant constant
/// (`str d1, [x9]` at `sysacc.c`'s call site) — after a launched GEM program
/// had run, the resumed accessory stored *that program's* leftover `d1` into
/// its control array and trapped with a wrong opcode, every pass, forever.
///
/// Saved where the GPRs are saved and restored where they are restored: a
/// slot's yield and its `!fresh` scheduler resume ([`Process::saved_fp`],
/// ACC03), and a `Pexec` caller's launch and its child's termination
/// ([`Process::parent_fp`], ARCH02 — the same failure on the
/// EL0-parent → EL0-child → EL0-parent path: the child rewrites the whole
/// vector file and the parent resumed on it).
#[repr(C)]
pub struct FpContext {
    /// `v0..v31` (low/high `u64` each), then `FPCR`, `FPSR`.
    slots: core::cell::UnsafeCell<[u64; FP_CONTEXT_LEN]>,
}

// SAFETY: written and read only from the single-CPU trap path, with the same
// discipline as the rest of the slot's saved context.
unsafe impl Sync for FpContext {}

impl FpContext {
    /// All-zero.
    #[inline]
    pub const fn zeroed() -> Self {
        Self { slots: core::cell::UnsafeCell::new([0; FP_CONTEXT_LEN]) }
    }

    /// Raw pointer to the first slot — what the save/restore asm addresses.
    #[inline]
    pub fn ptr(&self) -> *mut u64 {
        self.slots.get() as *mut u64
    }
}

/// Atomic snapshot of an [`ExceptionFrame`]'s restorable bits
/// (everything the post-handler `eret` consumes). Layout matches
/// [`crate::bdos::process::PARENT_FRAME_*`] index constants.
///
/// All reads/writes go through atomics for the same single-CPU
/// reason as the rest of [`Process`]; the slot is mutated only
/// under the BDOS giant lock, but other slot fields are atomic
/// anyway and a non-atomic outlier would break the auto-`Sync`
/// derivation.
#[repr(C)]
pub struct ParentFrame {
    /// `x0..x29` (= 30 slots), followed by `lr`, `elr`, `spsr`,
    /// `sp_el0`. Use [`Self::gpr`], [`Self::lr`], [`Self::elr`],
    /// [`Self::spsr`], [`Self::sp_el0`] for named access.
    pub regs: [AtomicU64; PARENT_FRAME_LEN],
}

impl ParentFrame {
    /// All-zero. Sentinel value: `elr() == 0` means "no parent
    /// frame captured" (slot is fresh / freed).
    #[inline]
    pub const fn zeroed() -> Self {
        Self {
            regs: [const { AtomicU64::new(0) }; PARENT_FRAME_LEN],
        }
    }

    /// `&AtomicU64` for a general-purpose register. `i` must be in
    /// `0..30`.
    #[inline]
    pub fn gpr(&self, i: usize) -> &AtomicU64 {
        &self.regs[i]
    }

    /// `&AtomicU64` for `lr` (`x30`).
    #[inline]
    pub fn lr(&self) -> &AtomicU64 {
        &self.regs[PARENT_FRAME_LR]
    }

    /// `&AtomicU64` for `elr_el1`.
    #[inline]
    pub fn elr(&self) -> &AtomicU64 {
        &self.regs[PARENT_FRAME_ELR]
    }

    /// `&AtomicU64` for `spsr_el1`.
    #[inline]
    pub fn spsr(&self) -> &AtomicU64 {
        &self.regs[PARENT_FRAME_SPSR]
    }

    /// `&AtomicU64` for `sp_el0`.
    #[inline]
    pub fn sp_el0(&self) -> &AtomicU64 {
        &self.regs[PARENT_FRAME_SP_EL0]
    }

    /// Reset every slot to zero. Called by [`free_slot`] when a
    /// slot is released.
    #[inline]
    pub fn clear(&self) {
        for r in self.regs.iter() {
            r.store(0, Ordering::Release);
        }
    }
}

impl Process {
    /// All-zero / "Free" slot. The `const fn` form lets us
    /// `[const { Process::empty() }; MAX_PROCESSES]` in static-array
    /// initialisation.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            state: AtomicU8::new(State::Free as u8),
            abi: AtomicU8::new(Abi::Lp64 as u8),
            basepage_kva: AtomicPtr::new(ptr::null_mut()),
            basepage_pa: AtomicU64::new(0),
            load_owner: AtomicPtr::new(ptr::null_mut()),
            space_pd: AtomicPtr::new(ptr::null_mut()),
            l0_pa: AtomicU64::new(0),
            entry_va: AtomicU64::new(0),
            sp_el0: AtomicU64::new(0),
            parent_slot: AtomicU32::new(NO_PARENT),
            launch_parent: AtomicU32::new(0),
            start_seq: AtomicU64::new(0),
            parent_runnable: AtomicBool::new(false),
            parent_frame: ParentFrame::zeroed(),
            parent_l0_pa: AtomicU64::new(0),
            parent_frame_full: AtomicBool::new(false),
            parent_frame_pexec: AtomicBool::new(false),
            parent_fp: FpContext::zeroed(),
            saved_frame: ParentFrame::zeroed(),
            saved_fp: FpContext::zeroed(),
            runnable: AtomicBool::new(false),
            has_ctx: AtomicBool::new(false),
            wait_cond: AtomicU64::new(0),
            syscall_deadline: AtomicU64::new(0),
            upcall_ctx: [const { ParentFrame::zeroed() }; MAX_UPCALL_NEST],
            upcall_ctx_mask: [const { AtomicU32::new(0) }; MAX_UPCALL_NEST],
            upcall_ctx_disp: [const { AtomicU64::new(0) }; MAX_UPCALL_NEST],
            upcall_depth: AtomicU8::new(0),
            upcall_mask: AtomicU32::new(0),
            upcall_pending_handler: AtomicU64::new(0),
            upcall_pending_args: [const { AtomicU64::new(0) }; 4],
            upcall_pending_mask: AtomicU32::new(0),
            upcall_pending_disp: AtomicU64::new(0),
        }
    }

    /// Read the current lifecycle state.
    #[inline]
    pub fn read_state(&self) -> State {
        State::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Set the lifecycle state. Caller must own the slot (i.e. have
    /// claimed it via [`allocate_slot`] or already be transitioning
    /// it under the BDOS giant lock).
    #[inline]
    pub fn set_state(&self, s: State) {
        self.state.store(s as u8, Ordering::Release);
    }

    /// Read the program ABI recorded for this slot.
    #[inline]
    pub fn read_abi(&self) -> Abi {
        Abi::from_u8(self.abi.load(Ordering::Acquire))
    }

    /// Record the program ABI for this slot (loader, at load time).
    #[inline]
    pub fn set_abi(&self, a: Abi) {
        self.abi.store(a as u8, Ordering::Release);
    }
}

// ─── Static table + run/nextrun ────────────────────────────────────

/// Kernel-wide process table. 16 slots × ~24 bytes each = ~384 B in
/// BSS — well below any meaningful overhead.
pub static PROCESS_TABLE: [Process; MAX_PROCESSES] =
    [const { Process::empty() }; MAX_PROCESSES];

/// The **PD pointer** of the currently-running process — the user
/// address of its [`BasePage`] ([`pd_of`], EmuTOS `run`) —
/// or `null` while only kernel code is on the stack (boot, between `Pterm`
/// and the next `Pexec`). Stamped into new BDOS allocations as their owner
/// ([`crate::bdos::mem::traps::current_owner`], EmuTOS `m_own = run`).
///
/// A user address: kernel code reads the basepage through
/// [`current_kva`], never by dereferencing this value.
pub static RUN: AtomicPtr<BasePage> = AtomicPtr::new(ptr::null_mut());

/// Pointer to the [`BasePage`] of the process that should run next
/// after the current trap returns. Set by `Pexec` modes that load
/// without immediately entering EL0 (`PE_LOAD`); consumed by the
/// trap-exit path which moves it into [`RUN`] just before `eret`.
pub static NEXTRUN: AtomicPtr<BasePage> = AtomicPtr::new(ptr::null_mut());

/// The running process's PD pointer (a **user** address, see [`RUN`]), or
/// `null` if no user process is active. An identity, not a reference:
/// compare it, pass it to [`slot_for`], stamp it as an owner — read the
/// basepage through [`current_kva`].
#[inline]
pub fn current() -> *mut BasePage {
    RUN.load(Ordering::Acquire)
}

/// The kernel high-half alias of the running process's basepage
/// (`run->…` in EmuTOS), or `null` if no user process is active. This is
/// the pointer kernel code dereferences: `p_flags`, `p_xdta`, `p_uft`,
/// `p_env`, ….
#[inline]
pub fn current_kva() -> *mut BasePage {
    let bp = current();
    if bp.is_null() {
        bp
    } else {
        kalias(bp as u64) as *mut BasePage
    }
}

/// Set the running-process pointer to the PD pointer `bp` (a user
/// address, [`pd_of`]; `null` = no process). Atomic so the
/// SVC handler that reads [`current`] sees a consistent value at every
/// moment.
///
/// Every way a process gets the CPU ends here (a launch, a `Pexec`, a
/// `Pterm` return, a scheduler switch, the return from a kernel park). When
/// `bp` is a **desk accessory**, its `os_run` cell is set to [`emutos_run`]
/// first (TASK/PROC03 B5, D-7): in EmuTOS an accessory is entered by a plain
/// `jmp` from an AES context and `run` stays whatever it was
/// (`aes/gemasm.S:83-110`, `_run-ownership-curdir.md` §1.2-§1.3), so
/// `*os_run` — one global word there (`bios/startup.S:110-111`) — names the
/// program in front, or the accdesk PD between programs. NewTOS's cell is
/// per process (page 0, [`loader::OS_RUN_CELL_VA`]); `run` can only change
/// by a `Pexec` or `Pterm`, each of which ends here again, so refreshing the
/// cell on entry keeps it right for as long as the accessory runs. Only the
/// cell follows `run`: [`RUN`] — and with it memory ownership, protection,
/// the current directory, the DTA, the handles and the files — stays the
/// accessory (user decision 2026-10-18, `docs/COMPATIBILITY.md`).
#[inline]
pub fn set_current(bp: *mut BasePage) {
    if let Some(slot) = slot_for(bp) {
        if is_accessory(slot) {
            write_os_run_cell(slot, emutos_run());
        }
    }
    RUN.store(bp, Ordering::Release);
}

/// Is `slot` a loaded desk accessory (`crate::aes::acc`, the list
/// `kernel/src/accs.rs` `load_one_acc` registers into)?
#[inline]
pub fn is_accessory(slot: usize) -> bool {
    crate::aes::acc::is_process(slot)
}

/// Kernel-wide start counter behind [`Process::start_seq`].
static START_SEQ: AtomicU64 = AtomicU64::new(0);

/// Record that `slot` has been started (EmuTOS `proc_go`, `run = p`,
/// `bdos/proc.c:575`): it is the innermost started process from now on
/// ([`emutos_run`]). Called by `pexec::write_parent`, i.e. for every process
/// that gets a `p_parent` at its start.
pub(crate) fn note_started(slot: usize) {
    if let Some(p) = PROCESS_TABLE.get(slot) {
        let n = START_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
        p.start_seq.store(n, Ordering::Release);
    }
}

/// EmuTOS's `run` as the GEMDOS of an executing desk accessory sees it
/// (TASK/PROC03 B5, `_run-ownership-curdir.md` §1.2/§1.3): the PD of the
/// **innermost started process** — the program in front, or its running
/// `Pexec` child, or the child an accessory started — or, with no started
/// process alive, the stand-in [`standin::kernel_run`] (the accdesk PD
/// between programs, the EmuCON PD while EmuCON runs).
///
/// EmuTOS's `run` is a stack: `proc_go` pushes (`run = p`, `bdos/proc.c:575`)
/// and `xterm` pops (`run = run->p_parent`, `:609`); accessories never touch
/// it. NewTOS keeps no such variable — its [`RUN`] is the process that has
/// the CPU, an accessory included — and derives the value instead: the live
/// process started last ([`Process::start_seq`]). For processes ending in
/// the reverse order of their starts, which a synchronous `Pexec` gives, the
/// two are the same. When a program ends while a child an accessory started
/// still lives (EmuTOS: `run` names the child's `p_parent`, the dead
/// program, after the child's end), this names the live child and then the
/// stand-in — never a freed PD.
pub fn emutos_run() -> *mut BasePage {
    let mut best: Option<(u64, usize)> = None;
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        let seq = p.start_seq.load(Ordering::Acquire);
        if seq == 0 || p.read_state() == State::Free || p.basepage_kva.load(Ordering::Acquire).is_null() {
            continue;
        }
        if best.is_none_or(|(b, _)| seq > b) {
            best = Some((seq, i));
        }
    }
    match best {
        Some((_, i)) => pd_of(i),
        None => standin::kernel_run(),
    }
}

/// `run` for the `p_parent` of a process `caller` starts (EmuTOS `proc_go`,
/// `p->p_parent = run`, `bdos/proc.c:541`): the caller's PD for a program;
/// [`emutos_run`] for a desk accessory, whose calls execute with `run`
/// unchanged (`_run-ownership-curdir.md` §1.3, TASK/PROC03 B5); the stand-in
/// [`standin::kernel_run`] for a kernel caller (`null`).
pub fn run_for_caller(caller: *mut BasePage) -> *mut BasePage {
    if caller.is_null() {
        return standin::kernel_run();
    }
    match slot_for(caller) {
        Some(slot) if is_accessory(slot) => emutos_run(),
        _ => caller,
    }
}

/// Write `pd` into `slot`'s `os_run` cell (page 0, 8 bytes, upper half 0 —
/// the form the loader writes, so a 4- and an 8-byte read both yield it).
fn write_os_run_cell(slot: usize, pd: *mut BasePage) {
    if let Some(p0) = mmu::page0_kva(slot) {
        let off = loader::OS_RUN_CELL_VA as usize;
        // SAFETY: `p0` is the slot's 4-KiB page 0 (kernel alias); the cell
        // lies inside it (`loader`'s compile-time bound). Single CPU, trap /
        // boot context, as every `set_current` caller.
        unsafe { (p0.add(off) as *mut u64).write_unaligned(pd as u64 & 0xFFFF_FFFF) };
    }
}

/// The second identity the running process owns memory under: the PD of the
/// memory space it shares when it is a `Pexec(PE_GO)` child
/// ([`Process::space_pd`]), else `null`. `Mfree`/`Mshrink` accept a block
/// owned by either identity (ARCH03 2d, E6/E15).
#[inline]
pub fn current_space() -> *mut BasePage {
    slot_for(current())
        .map(|s| PROCESS_TABLE[s].space_pd.load(Ordering::Acquire))
        .unwrap_or(ptr::null_mut())
}

/// The kernel alias of the running process's **page 0** — its private
/// `[0, 0x1000)`, where the system variables (`$484` `conterm`, `$4BA`
/// `_hz_200`), the trampolines and the KEYS02 vector table live
/// ([`mmu::page0_kva`]) — or `None` when no user process runs.
#[inline]
pub fn current_page0_kva() -> Option<*mut u8> {
    slot_for(current()).and_then(mmu::page0_kva)
}

/// The PD pointer of `slot` — the user address of its basepage, what
/// [`set_current`] takes for it — or `null` for an out-of-range slot or one
/// without a loaded image. Derived from [`Process::basepage_kva`]
/// ([`kva_to_user_va`]), the single record of where the basepage is.
#[inline]
pub fn pd_of(slot: usize) -> *mut BasePage {
    match PROCESS_TABLE.get(slot) {
        Some(p) => {
            let kva = p.basepage_kva.load(Ordering::Acquire);
            if kva.is_null() {
                ptr::null_mut()
            } else {
                kva_to_user_va(kva as u64) as *mut BasePage
            }
        }
        None => ptr::null_mut(),
    }
}

// ─── Address translation (ARCH03 stage 1: VA = PA) ─────────────────

/// High-half kernel VA offset (mirror of `kernel::arch::aarch64::mmu`).
#[cfg(all(target_arch = "aarch64", target_os = "none"))]
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

/// The kernel alias of user address `va`. Every user address is its own
/// physical address (ARCH03 stage 1, [`mmu`]), and the kernel maps all of
/// DRAM at `phys + VA_OFFSET`, so this is an addition — the AArch64
/// counterpart of FreeMiNT's `prot_temp`, without a table change (plan
/// Frage 10). It says nothing about whether `va` is *live* user memory; that
/// is [`usermem::resolve_kva`]'s question.
///
/// On the host (unit tests) there is no high half: test "user" memory is
/// ordinary host memory, so the alias is the address itself.
#[inline]
pub const fn kalias(va: u64) -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    {
        va.wrapping_add(VA_OFFSET)
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    {
        va
    }
}

/// The user address (= physical address) of kernel high-half VA `kva` —
/// the inverse of [`kalias`]. Identity on the host, like [`kalias`].
#[inline]
pub const fn kva_to_user_va(kva: u64) -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    {
        kva.wrapping_sub(VA_OFFSET)
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    {
        kva
    }
}

/// ABI (pointer / `long` width) of the currently-running process, or
/// [`Abi::Lp64`] when no user process owns the running basepage.
///
/// The GEM trap dispatch (`svc #2`) reads this to interpret the caller's
/// AES/VDI parameter block at the right pointer width. An EL0 app's slot
/// carries the ABI its ELF class implied; every in-kernel (EL1) GEM
/// caller — kernel code and the boot-time VDI/AES selftests, which run
/// while [`RUN`] is null — resolves to [`Abi::Lp64`] and keeps its
/// native 64-bit blocks. (The desktop is the EL0 program `NEWDESK` since
/// DESK43 U9/U10 and takes its slot's ABI, LP64, like any native program.)
#[inline]
pub fn current_abi() -> Abi {
    match slot_for(current()) {
        Some(idx) => PROCESS_TABLE[idx].read_abi(),
        None => Abi::Lp64,
    }
}

// ─── Deferred terminate (Ctrl-C from blocking console input) ────────

/// Sentinel meaning "no terminate pending". `i32::MIN` is never a
/// valid TOS exit code (rc is a WORD), so it cannot collide.
const NO_PENDING_TERMINATE: i32 = i32::MIN;

/// Exit code of a pending self-terminate request, or
/// [`NO_PENDING_TERMINATE`].
static PENDING_TERMINATE: AtomicI32 = AtomicI32::new(NO_PENDING_TERMINATE);

/// Request that the **current** process self-terminate with exit code
/// `rc` once the current SVC returns. The blocking console input
/// functions use this for Ctrl-C (`rc = -32`, EmuTOS `terminate()`):
/// a leaf GEMDOS function returns a value in `x0` and cannot redirect
/// the trap frame to the parent, so it records the request here and
/// the kernel SVC handler performs the Pterm-style redirect via
/// [`take_pending_terminate`].
pub fn request_terminate(rc: i32) {
    PENDING_TERMINATE.store(rc, Ordering::Release);
}

/// Consume a pending self-terminate request set by
/// [`request_terminate`], returning its exit code if one is pending.
pub fn take_pending_terminate() -> Option<i32> {
    let rc = PENDING_TERMINATE.swap(NO_PENDING_TERMINATE, Ordering::AcqRel);
    if rc == NO_PENDING_TERMINATE {
        None
    } else {
        Some(rc)
    }
}

// ─── Deferred EL0 upcall (cooperative-kernel building block 1) ──────
//
// The `request_terminate`/`take_pending_terminate` analogue for the generic
// EL0→EL1 upcall primitive (design §7.1), but per-slot: a consumer posts a
// handler + arguments to a target process's slot, and the SVC handler
// delivers it at that process's next syscall return to EL0. Lock-free over
// the slot atomics; `upcall_pending_handler` (VA, `0` = none) is the
// presence sentinel, written last on post and swapped to `0` on take so a
// reader that observes it set also observes the args/mask stored before it.

/// Post an [`UpcallDesc`] to `slot`, to be delivered at that process's next
/// syscall return to EL0 (design §7.1). A previously-posted, not-yet-
/// delivered upcall is overwritten (last writer wins); the current consumers
/// post at most one pending upcall per delivery point. Out-of-range `slot`
/// is a no-op.
pub fn post_upcall(slot: usize, desc: UpcallDesc) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let p = &PROCESS_TABLE[slot];
    for (i, a) in p.upcall_pending_args.iter().enumerate() {
        a.store(desc.args[i], Ordering::Release);
    }
    p.upcall_pending_mask.store(desc.mask_bit, Ordering::Release);
    p.upcall_pending_disp
        .store(desc.disposition.encode(), Ordering::Release);
    // Presence sentinel last (release): a `take` that sees it nonzero also
    // sees the args/mask/disposition stored above.
    p.upcall_pending_handler
        .store(desc.handler_va, Ordering::Release);
}

/// Consume a pending upcall for `slot`, if any (design §7.1). Swaps the
/// `handler_va` sentinel to `0` and, if it was set, reads back the
/// args/mask posted before it. Out-of-range `slot` yields `None`.
pub fn take_pending_upcall(slot: usize) -> Option<UpcallDesc> {
    if slot >= MAX_PROCESSES {
        return None;
    }
    let p = &PROCESS_TABLE[slot];
    let handler_va = p.upcall_pending_handler.swap(0, Ordering::AcqRel);
    if handler_va == 0 {
        return None;
    }
    let mut args = [0u64; 4];
    for (i, a) in p.upcall_pending_args.iter().enumerate() {
        args[i] = a.load(Ordering::Acquire);
    }
    let mask_bit = p.upcall_pending_mask.load(Ordering::Acquire);
    let disposition =
        Disposition::decode(p.upcall_pending_disp.load(Ordering::Acquire));
    Some(UpcallDesc {
        handler_va,
        args,
        mask_bit,
        disposition,
    })
}

/// The "queued for next eret" basepage, or `null`.
#[inline]
pub fn pending() -> *mut BasePage {
    NEXTRUN.load(Ordering::Acquire)
}

/// Set the queued-next basepage.
#[inline]
pub fn set_pending(bp: *mut BasePage) {
    NEXTRUN.store(bp, Ordering::Release);
}

// ─── Slot allocation / lookup ──────────────────────────────────────

/// Claim the first free slot. CAS-based so concurrent `Pexec`s
/// don't double-claim. Returns the slot index, or `None` when the
/// table is full (`Pexec` should then return `ENSMEM`).
///
/// The claimed slot is left in [`State::Loaded`]; the caller fills
/// in the basepage / parent / `l0_pa` fields before transitioning
/// to [`State::Running`].
pub fn allocate_slot() -> Option<usize> {
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        // Try to flip Free → Loaded. Stronger compare_exchange
        // because we don't want spurious failures on platforms
        // where weak CAS may fail without contention.
        if p.state
            .compare_exchange(
                State::Free as u8,
                State::Loaded as u8,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            return Some(i);
        }
    }
    None
}

/// Release a slot, zeroing every field. Paired with
/// [`allocate_slot`] (and the late-`Pterm` cleanup that runs
/// `free_all_owned` against this slot's basepage pointer first).
pub fn free_slot(idx: usize) {
    if idx >= MAX_PROCESSES {
        return;
    }
    // Its private page tables and its inherited/temporary rights go first,
    // while the slot still names its root (ARCH03 3b).
    // SAFETY: slot release runs under the BDOS giant lock (every caller is
    // a trap or boot path).
    unsafe { memprot::on_slot_released(idx) };
    let p = &PROCESS_TABLE[idx];
    p.abi.store(Abi::Lp64 as u8, Ordering::Release);
    p.basepage_kva.store(ptr::null_mut(), Ordering::Release);
    p.basepage_pa.store(0, Ordering::Release);
    p.load_owner.store(ptr::null_mut(), Ordering::Release);
    p.space_pd.store(ptr::null_mut(), Ordering::Release);
    p.l0_pa.store(0, Ordering::Release);
    p.entry_va.store(0, Ordering::Release);
    p.sp_el0.store(0, Ordering::Release);
    p.parent_slot.store(NO_PARENT, Ordering::Release);
    p.launch_parent.store(0, Ordering::Release);
    p.start_seq.store(0, Ordering::Release);
    p.parent_runnable.store(false, Ordering::Release);
    p.parent_frame.clear();
    p.parent_l0_pa.store(0, Ordering::Release);
    p.parent_frame_full.store(false, Ordering::Release);
    p.parent_frame_pexec.store(false, Ordering::Release);
    // Drop the slot out of the cooperative run-set: an accessory slot is
    // admitted by the loader and would otherwise stay on the ready list after
    // the process behind it is gone (ACC02/ACC03, A115).
    sched::unready(idx);
    p.has_ctx.store(false, Ordering::Release);
    p.syscall_deadline.store(0, Ordering::Release);
    // Clear any stale WAITING key so a reused slot does not get spuriously
    // woken by a `sched::wake` matching a dead process's wait condition.
    p.wait_cond.store(0, Ordering::Release);
    // Reset the EL0 upcall state so a reused slot starts clean.
    p.upcall_depth.store(0, Ordering::Release);
    p.upcall_mask.store(0, Ordering::Release);
    p.upcall_pending_handler.store(0, Ordering::Release);
    for a in p.upcall_pending_args.iter() {
        a.store(0, Ordering::Release);
    }
    p.upcall_pending_mask.store(0, Ordering::Release);
    p.upcall_pending_disp.store(0, Ordering::Release);
    for c in p.upcall_ctx.iter() {
        c.clear();
    }
    for m in p.upcall_ctx_mask.iter() {
        m.store(0, Ordering::Release);
    }
    for d in p.upcall_ctx_disp.iter() {
        d.store(0, Ordering::Release);
    }
    // Give the VDI mouse-wheel vector back if this process owned it: a
    // `vex_wheelv` handler is an EL0 VA and means nothing once the address
    // space is gone (see `vdi::mouse::release_wheel_vector`).
    crate::vdi::mouse::release_wheel_vector(idx);
    // Drop any `kbdvec` bytes still queued for this process (KEYS02, D8): the
    // hook lived in the address space that is going away; its table and stubs
    // in page 0 go with the TPA.
    crate::xbios::kbdvecs::release_slot(idx);
    // State last so that any concurrent reader sees `Free` only once
    // every other field is observably reset.
    p.state.store(State::Free as u8, Ordering::Release);
}

/// Find the slot whose PD pointer is `bp` (a user address, as [`RUN`]
/// holds it), if any — matched through its kernel alias against
/// [`Process::basepage_kva`]. Linear scan — the table is small and the
/// lookup runs at most once per `Pterm` / per per-process trap, so the cost
/// is negligible.
pub fn slot_for(bp: *mut BasePage) -> Option<usize> {
    if bp.is_null() {
        return None;
    }
    let kva = kalias(bp as u64) as *mut BasePage;
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        if p.read_state() != State::Free
            && p.basepage_kva.load(Ordering::Acquire) == kva
        {
            return Some(i);
        }
    }
    None
}

/// Borrow a slot by index.
#[inline]
pub fn slot(idx: usize) -> Option<&'static Process> {
    PROCESS_TABLE.get(idx)
}

#[cfg(test)]
mod fp_context_tests {
    use super::*;

    /// The kernel's `save_fp_context`/`restore_fp_context`
    /// (`kernel/src/arch/aarch64/exceptions.rs`) address an `FpContext` with
    /// fixed immediates: `stp q0..q31` at `#0x000..#0x1e0`, `FPCR` at
    /// `#0x200`, `FPSR` at `#0x208`. The structure has to be exactly that.
    #[test]
    fn fp_context_layout_matches_the_save_restore_asm() {
        assert_eq!(core::mem::size_of::<FpContext>(), 0x210);
        assert_eq!(FP_CONTEXT_LEN * 8, 0x210);
        assert_eq!(FP_CONTEXT_FPCR * 8, 0x200);
        assert_eq!(FP_CONTEXT_FPSR * 8, 0x208);
        // The status words are loaded/stored with 64-bit `ldr`/`str`.
        assert_eq!(core::mem::align_of::<FpContext>() % 8, 0);
    }

    /// ARCH02: a slot carries two FP contexts that are live at the same time
    /// — `parent_fp` (its Pexec caller's vector file, held until this slot
    /// terminates) and `saved_fp` (its own, written whenever it yields while
    /// that caller waits). They must never overlap, in any slot, or the
    /// child's first yield would overwrite the state its parent resumes on.
    #[test]
    fn parent_fp_and_saved_fp_are_disjoint_in_every_slot() {
        let len = core::mem::size_of::<FpContext>();
        for (i, p) in PROCESS_TABLE.iter().enumerate() {
            let a = p.parent_fp.ptr() as usize;
            let b = p.saved_fp.ptr() as usize;
            assert!(a + len <= b || b + len <= a, "slot {i}: parent_fp overlaps saved_fp");
        }
        // …and no two slots share one (the parent's own `saved_fp` lives in
        // the parent's slot, the `parent_fp` in the child's).
        for i in 0..MAX_PROCESSES {
            for j in 0..MAX_PROCESSES {
                if i != j {
                    assert_ne!(
                        PROCESS_TABLE[i].parent_fp.ptr(),
                        PROCESS_TABLE[j].saved_fp.ptr()
                    );
                }
            }
        }
    }
}
