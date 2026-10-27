//! Cooperative round-robin scheduler over the process table.
//!
//! The **run-set** is the set of processes marked [`Process::runnable`]
//! (peers in a voluntary round-robin). A peer gives up the CPU with
//! `Pwait` (GEMDOS 0x4D), with every AES call, or by blocking in an AES
//! wait; [`on_yield`] then picks the next runnable peer. On termination,
//! [`on_exit`] drops the peer and picks the next one (or returns `None`,
//! i.e. resume the parent / boot via the existing `Pterm` path).
//!
//! **Which** peer runs next is EmuTOS's ready list (A115, see "The ready
//! list" below): the run-set is ordered like `rlr`, and processes made ready
//! between two dispatches wait on a `drl` stack that every dispatch appends
//! behind the caller. `runnable` is set and cleared **only** through this
//! module ([`ready`], [`unready`], [`admit`], [`wake`], …), never by storing
//! the bit, or the order would not know about it.
//!
//! ## Extensible toward preemptive multitasking
//!
//! Everything scheduling-specific lives here: the run-set membership and
//! the ready-list policy. The SVC handler only ever sees a [`Switch`]
//! (which process to run and how to enter it). Growing this into a
//! richer scheduler — priorities, time-slices via a timer-driven yield,
//! blocked/sleeping states for `evnt_*` waits — means changing this
//! module and adding fields to [`Process`], **not** touching the trap
//! path. The cooperative `Pwait` hand-off is just the first policy.
//!
//! [`Process`]: super::Process
//! [`Process::runnable`]: super::Process::runnable

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::{MAX_PROCESSES, PROCESS_TABLE};
use crate::host::host;

/// Sentinel for "no slot" (the kernel/boot is on the CPU, no peer runs).
const NONE: usize = MAX_PROCESSES;

// ─── Wait-condition keys (sleep/wake substrate, step 3a) ────────────
//
// A `wait_cond` is a **wake-class key**, not a per-waiter predicate (design
// §4.1): `wake(cond)` re-arms every waiter whose key matches, and each woken
// waiter then re-checks its *full* predicate (the FreeMiNT `sleep`/`do_wake`
// model, `references/freemint/sys/proc.c:490-716`). The key is a `u64` with the
// class in the high byte and the argument (pid / deadline) in the low 56 bits
// (§8.1.3). `0` is reserved for "not waiting", so no valid key is ever `0`.

/// Bit position of the wake-class tag within a `wait_cond` key.
const WAIT_CLASS_SHIFT: u32 = 56;
/// Mask for the argument (pid / deadline) carried in the low bits of a key.
const WAIT_ARG_MASK: u64 = (1u64 << WAIT_CLASS_SHIFT) - 1;

/// `WAIT_INPUT` class tag: keyboard / button / mouse activity (§4.1). Woken by
/// the cooperative idle wake-pass, never the ISR (design §5.2).
const CLASS_INPUT: u64 = 1;
/// `WAIT_MESG(idx)` class tag: a message in AES identity `idx`'s per-app FIFO
/// (§4.1). Woken by [`crate::aes::pd::post_msg`] (design §5.2).
const CLASS_MESG: u64 = 2;
/// `WAIT_TIMER(deadline)` class tag: `monotonic_millis >= deadline` (§4.1).
/// Woken by the idle wake-pass deadline check (timer-IRQ deferred, design §5.2).
const CLASS_TIMER: u64 = 3;

/// The `WAIT_INPUT` wake key (design §4.1). Used by `evnt_keybd`/`evnt_button`/
/// `evnt_mouse`/`evnt_multi`, which all wait on host input.
#[inline]
pub const fn wait_input() -> u64 {
    CLASS_INPUT << WAIT_CLASS_SHIFT
}

/// The `WAIT_MESG(idx)` wake key for the message FIFO of AES identity `idx`
/// (the descriptor index: a GEMDOS slot or `pd::KERNEL_CONTEXT`; design §4.1). Used
/// by `evnt_mesag`; the poster ([`crate::aes::pd::post_msg`]) wakes on the
/// same key.
#[inline]
pub const fn wait_mesg(idx: usize) -> u64 {
    (CLASS_MESG << WAIT_CLASS_SHIFT) | (idx as u64 & WAIT_ARG_MASK)
}

/// The `WAIT_TIMER(deadline)` wake key for an absolute `monotonic_millis`
/// deadline (design §4.1). Used by `evnt_timer`; the idle wake-pass wakes it
/// once the deadline passes.
#[inline]
pub const fn wait_timer(deadline_ms: u64) -> u64 {
    (CLASS_TIMER << WAIT_CLASS_SHIFT) | (deadline_ms & WAIT_ARG_MASK)
}

/// `WAIT_MULTI(events, idx)` class tag: a wait on the **union** of several
/// event sources (A112, `docs/api/aes/event.md` "Wake conditions and dispatch"
/// point 1). EmuTOS's `ev_multi` arms one `EVB` per requested event with
/// `iasync` and sleeps on all of them at once (`mwait(flags)`,
/// `aes/gemevlib.c:235-259`, `aes/gemasync.c:117-127`); any one completing
/// wakes the process through `signal()` (`gemasync.c:35-61`), whoever is
/// running. The argument carries the requested [`WAKE_INPUT`]/[`WAKE_MESG`]/
/// [`WAKE_TIMER`] bits and the AES identity whose message FIFO the
/// [`WAKE_MESG`] bit watches; the timer deadline is the waiter's
/// [`super::Process::syscall_deadline`].
const CLASS_MULTI: u64 = 4;

/// [`wait_multi`] event bit: input — keyboard, button, mouse rectangles, and
/// (in NewTOS) the inline Control Manager's menu/frame interception, which
/// turns input into `MN_SELECTED`/`WM_*` messages inside the waiter's own
/// loop. Woken like a [`wait_input`] waiter.
pub const WAKE_INPUT: u64 = 1;
/// [`wait_multi`] event bit: a message written into the waiter's own FIFO by
/// any process — `aqueue(TRUE,…)` servicing the parked reader
/// (`aes/gemqueue.c:104-118`). Woken like a [`wait_mesg`] waiter on the same
/// identity.
pub const WAKE_MESG: u64 = 2;
/// [`wait_multi`] event bit: the waiter's own deadline
/// ([`super::Process::syscall_deadline`]) — EmuTOS `tchange` → `evremove` →
/// `azombie` → `signal` (`aes/gemflag.c:32-68`), run by any dispatcher pass.
pub const WAKE_TIMER: u64 = 4;

/// [`wait_multi`] event bit: the AES window-tree lock `wind_spb` changed hands
/// — `unsync` gave it to the head of its wait line (`azombie(p, 0)`,
/// `aes/gemflag.c:117-124`) after a refused `tak_flag` had parked the waiter
/// with `ev_block(MU_MUTEX, &wind_spb)` (`aes/gemwmlib.c:1785-1786`). Woken
/// like a [`wait_mutex`] waiter (GH1).
pub const WAKE_MUTEX: u64 = 8;

/// `WAIT_MUTEX` class tag: the AES window-tree lock (`wind_spb`) — there is
/// one such lock, so the key carries no argument. Woken by the AES's `unsync`
/// hand-over (`crate::aes::window`, GH1).
const CLASS_MUTEX: u64 = 5;

/// The `WAIT_MUTEX` wake key: the window-tree lock was handed to the head of
/// its wait line. Every waiter re-checks; only the one the lock was handed to
/// proceeds, the others sleep again (FIFO order is kept by the AES's own wait
/// line, not by the wake).
#[inline]
pub const fn wait_mutex() -> u64 {
    CLASS_MUTEX << WAIT_CLASS_SHIFT
}

/// `WAIT_QNQ(writer)` class tag: the `appl_write` that AES identity `writer`
/// parked on a full queue has been serviced (A116). Woken by the read that
/// made room — `aqueue`'s complementary service of `p_qnq`
/// (`aes/gemqueue.c:108-118`), `crate::aes::pd::after_read`.
const CLASS_QNQ: u64 = 6;

/// The `WAIT_QNQ(writer)` wake key (A116): sleep until the parked
/// `appl_write` of AES identity `writer` has been copied into its target's
/// queue by a read of that queue. EmuTOS parks the writer's `EVB` on the
/// target's `p_qnq` (`evinsert`, `aes/gemqueue.c:120-124`) and a later
/// successful read services it with `doq` + `azombie(e, 1)` (`:108-118`); the
/// key carries the **writer**, so only the one write that was serviced wakes.
#[inline]
pub const fn wait_qnq(writer: usize) -> u64 {
    (CLASS_QNQ << WAIT_CLASS_SHIFT) | (writer as u64 & WAIT_ARG_MASK)
}

/// Bit position of the AES identity within a [`wait_multi`] argument.
const MULTI_IDX_SHIFT: u32 = 8;
/// Mask of the event bits within a [`wait_multi`] argument.
const MULTI_EVENTS_MASK: u64 = 0xff;

/// The `WAIT_MULTI(events, idx)` wake key (A112): sleep until **any** of the
/// sources in `events` ([`WAKE_INPUT`] | [`WAKE_MESG`] | [`WAKE_TIMER`])
/// fires. `idx` is the AES identity whose FIFO [`WAKE_MESG`] watches (the
/// caller's own). [`wake`] with a [`wait_input`] key wakes it when
/// [`WAKE_INPUT`] is set, a [`wait_mesg`]`(idx)` key when [`WAKE_MESG`] is
/// set; the dispatcher's timer pass when [`WAKE_TIMER`] is set and the slot's
/// [`super::Process::syscall_deadline`] has passed.
#[inline]
pub const fn wait_multi(events: u64, idx: usize) -> u64 {
    (CLASS_MULTI << WAIT_CLASS_SHIFT)
        | (((idx as u64) << MULTI_IDX_SHIFT) & WAIT_ARG_MASK)
        | (events & MULTI_EVENTS_MASK)
}

/// The wake-class tag of a `wait_cond` key.
#[inline]
fn wait_class(cond: u64) -> u64 {
    cond >> WAIT_CLASS_SHIFT
}

/// The argument (pid / deadline) carried by a `wait_cond` key.
#[inline]
fn wait_arg(cond: u64) -> u64 {
    cond & WAIT_ARG_MASK
}

/// The slot currently on the CPU, or [`NONE`].
static CURRENT: AtomicUsize = AtomicUsize::new(NONE);

/// Per-slot restart-kind of the saved context (S2 restartable block, design
/// §4.3): `true` when the slot yielded **mid-syscall** via [`sleep`] with its
/// `saved_frame.elr` rewound to the `svc` (so `apply_switch` must preserve `x0`
/// on resume), `false` for a leaf `Pwait` yield. Written by the exception
/// handler's shared frame-save when a slot yields, read by [`switch_to`] into
/// [`Switch::restartable`]. Parallel to the `runnable`/`has_ctx` bits; a fresh
/// slot is `false` (the value is only consulted on a `!fresh` resume).
static RESUME_RESTARTABLE: [AtomicBool; MAX_PROCESSES] =
    [const { AtomicBool::new(false) }; MAX_PROCESSES];

/// What the SVC handler must do to switch to the next process. The fresh
/// entry's basepage (`x0`) is the slot's PD pointer (`process::pd_of`) — since
/// ARCH03 stage 1 each process's TPA has its own address in the shared space.
pub struct Switch {
    /// Slot to run next.
    pub slot: usize,
    /// `true` = first run (enter at `entry_va` with a fresh stack);
    /// `false` = resume the saved EL0 context.
    pub fresh: bool,
    /// Physical L0 page-table root to install in `TTBR0_EL1`.
    pub l0_pa: u64,
    /// Entry point (user VA), used only when `fresh`.
    pub entry_va: u64,
    /// Initial `SP_EL0` for a `fresh` entry — the slot's
    /// [`super::Process::sp_el0`] (the mapped TPA top the loader computed).
    /// `0` for a slot whose image did not come through `pexec::load` (the
    /// embedded selftest blobs), where the handler keeps its own default.
    pub sp_el0: u64,
    /// Privilege class of the peer being switched to (Option B hybrid model,
    /// design §2). `false` = an EL0 user process (its own TTBR0 → `apply_switch`
    /// installs `l0_pa`); `true` = an EL1-internal cooperative thread
    /// (desktop / screen-manager) for which no TTBR0 install must happen.
    ///
    /// **Prepared but unused in step 3a.** Every peer in the step-3 run-set is
    /// an EL0 process, so this is always `false` and `apply_switch` is
    /// unchanged. Step 4 (`docs/analysis/coop-kernel/`
    /// `20-sleepwake-substrate-design.md` §2/§7.2) reads it to skip the TTBR0
    /// switch for EL1-internal threads.
    pub el1_internal: bool,
    /// Restart-kind of the resume (S2 restartable block, design §4.3 + §8.2
    /// sub-step 3b). Only meaningful when `!fresh` (a resume). `false` = the
    /// slot yielded via the leaf `Pwait`: its `saved_frame.elr` points *after*
    /// the `svc`, so `apply_switch` writes `gpr[0] = 0` (the `Pwait` return
    /// value). `true` = the slot yielded mid-syscall via [`sleep`] with its
    /// `saved_frame.elr` rewound to the `svc`: the `eret` **re-executes** that
    /// `svc`, so `apply_switch` must **preserve `x0`** (the GEM-trap selector,
    /// e.g. 200 = AES) — it skips the `gpr[0] = 0` clobber.
    ///
    /// **This is the second S2 delta.** The design §4.3 originally claimed the
    /// only delta vs. the `Pwait` save was the ELR-rewind; that is incomplete —
    /// without preserving `x0` the re-executed `svc #2` re-enters the GEM trap
    /// with `x0 = 0` and dispatches nothing. [`switch_to`] fills this from the
    /// slot's [`set_resume_restartable`] flag, recorded when the slot yielded.
    pub restartable: bool,
    /// **Not a switch at all** (KEYS02, `docs/api/xbios/22-kbdvbase.md` B4/O1):
    /// the blocking wait gave way because the *running* process has `kbdvec`
    /// bytes queued for its hook. The trap handler must abort the call exactly
    /// like an S2 yield — rewind `elr` to the `svc`, commit nothing — but stay
    /// on this process and deliver the hook upcall on top of the rewound frame.
    /// After the hook's `Psigreturn` the `svc` re-executes and re-checks (a key
    /// the hook chained into the BIOS queue is now there). Built only by
    /// [`switch_restart_for_upcall`]; every other field is meaningless then.
    pub upcall_restart: bool,
    /// **The call has completed** (A112, `docs/api/aes/event.md` "Wake
    /// conditions and dispatch" point 3): the dispatch every AES call ends
    /// with — `super()` falls through to `dsptch()` (`aes/gemsuper.c:454-466`)
    /// — found a ready peer. Unlike an S2 yield nothing is re-executed: the
    /// trap handler finishes the call as usual (result into `x0`, the
    /// terminate/upcall tails) and only then, if the frame is the call's plain
    /// return, saves it as a **leaf** yield (`elr` after the `svc`, the `Pwait`
    /// shape) and switches. On resume the caller continues after its `svc`
    /// with `x0 = 0` — the AES trap's own return value. Built only by
    /// [`dispatch_after_call`].
    pub completed: bool,
}

/// The slot currently running (or [`NONE`]).
#[inline]
pub fn current() -> usize {
    CURRENT.load(Ordering::Acquire)
}

/// The slot of the execution context physically on the CPU, derived from the
/// BDOS running-basepage `RUN` — or [`NONE`] for an **EL1 kernel context**
/// (e.g. the boot path), which owns no process slot.
///
/// This is the same derivation [`crate::aes::appl::current_identity`] uses and
/// it is the one that is *always* maintained: every path that gives the CPU to
/// an EL0 process sets `RUN` ([`crate::bdos::process::set_current`]) — the
/// kernel launcher, `Pexec`, the scheduler switch — whereas [`CURRENT`] is
/// written only by the scheduler's own [`set_running`] and is therefore
/// [`NONE`] for a program the kernel launcher entered directly.
///
/// [`sleep`] must distinguish "an EL0 process is blocking" from "the EL1
/// kernel client is blocking" (they park in different places, §-ACC03), so it
/// keys off this rather than off [`current`].
#[inline]
pub fn running_slot() -> usize {
    match super::slot_for(super::current()) {
        Some(idx) => idx,
        None => NONE,
    }
}

/// `true` while an **EL1 kernel client** (the boot path's accessory wait; until
/// DESK43 U10 also the built-in desktop inside a blocking `evnt_*`) has its
/// trap frame parked
/// by the kernel and is waiting to be resumed — the NewTOS stand-in for
/// EmuTOS's "the desktop's AESPD is one more entry in the round-robin"
/// (`aes/gemasm.S:169-194` `switchto`, over `rlr`/`drl`).
///
/// EmuTOS has no equivalent flag because it has no privilege boundary: its
/// desktop is an ordinary AESPD and `dsptch()` switches to it like to any
/// other. NewTOS's EL1 kernel context owns no process slot (the desktop, for
/// which this was built, is an EL0 process with a slot since DESK43 U9/U10);
/// instead the trap handler snapshots its frame ("the kernel park") before
/// switching to an accessory, and this flag says that such a context is
/// waiting to be resumed — it is then one entry of the ready list, at the
/// position its last dispatch appended it to (A115, [`pick_peer_for_kernel`]).
static KERNEL_PARKED: AtomicBool = AtomicBool::new(false);

/// Record whether an EL1 kernel client's frame is parked and resumable — set
/// by the trap handler when it parks, cleared when it restores.
#[inline]
pub fn set_kernel_parked(parked: bool) {
    KERNEL_PARKED.store(parked, Ordering::Release);
}

/// `true` when [`set_kernel_parked`] armed a parked EL1 kernel client.
#[inline]
pub fn kernel_parked() -> bool {
    KERNEL_PARKED.load(Ordering::Acquire)
}

/// The [`Switch`] that means "resume the parked EL1 kernel client" rather than
/// an EL0 peer — `el1_internal` is the flag reserved for exactly this
/// (design §2/§7.2) and `slot` is [`NONE`], so no process-table entry is read.
/// `apply_switch` restores the kernel park instead of a slot's saved frame.
#[inline]
pub fn switch_to_kernel() -> Switch {
    Switch {
        slot: NONE,
        fresh: false,
        l0_pa: 0,
        entry_va: 0,
        sp_el0: 0,
        el1_internal: true,
        restartable: false,
        upcall_restart: false,
        completed: false,
    }
}

/// The [`Switch`] that means "abort and restart this wait on the running
/// process after its `kbdvec` hook ran" ([`Switch::upcall_restart`]). Returned
/// by [`sleep`] when the running slot has hook bytes queued
/// (`xbios::kbdvecs::pending`), so an `evnt_*` wait that would otherwise only
/// return on an event the hook itself has to produce (a chained key in the
/// BIOS queue) cannot deadlock.
#[inline]
pub fn switch_restart_for_upcall() -> Switch {
    Switch {
        slot: running_slot(),
        fresh: false,
        l0_pa: 0,
        entry_va: 0,
        sp_el0: 0,
        el1_internal: false,
        restartable: true,
        upcall_restart: true,
        completed: false,
    }
}

/// The dispatch of the EL1 kernel client (a context that owns no slot): the
/// next entry of the ready list to give the CPU to, or `None` when the kernel
/// client itself is the head and simply carries on.
///
/// The kernel client is EmuTOS's pid 0 — the AESPD the boot path's
/// accessory barrier runs in (`geminit.c:736-737`, `dsptch(); wait_for_accs`)
/// — and its dispatch is `disp()` like anyone's (A115): it is appended to
/// the **end** of `rlr` (`disp_act`, `gemdisp.c:74-79`), `drl` is drained
/// behind it, and the head is resumed. Desk accessories reach `rlr` through
/// `pstart`'s push onto `drl` ([`admit`], `gempd.c:125-128`), so they start in
/// **reverse** load order, and the kernel client's first dispatch after
/// loading them keeps the CPU (it was appended before the drain) — EmuTOS's
/// `dsptch(); /* off we go */` ahead of `wait_for_accs`, whose `all_run`
/// then hands over.
///
/// The kernel client is on the list while its frame is parked
/// ([`kernel_parked`]); the position it was appended at is
/// `KERNEL_RL_KEY`. Its waits (`sleep` from EL1) are polls — it re-checks
/// on every resume — so it is appended in either case, never put on `nrl`.
pub fn pick_peer_for_kernel() -> Option<Switch> {
    KERNEL_RL_KEY.store(next_rl_key(), Ordering::Release);
    pick_head(NONE)
}

/// Record `slot` as the running process.
#[inline]
pub fn set_running(slot: usize) {
    CURRENT.store(slot, Ordering::Release);
}

/// A launch-mode `Pexec` (`PE_LOADGO` / `PE_GO`) has given the CPU to the
/// process in `child_slot`: take its **parent** out of the cooperative run-set
/// for as long as the child lives.
///
/// GEMDOS `Pexec` is a *synchronous* BDOS call. EmuTOS's `proc_go` sets
/// `run = p` and enters the child through `gouser()`
/// (`references/emutos/bdos/proc.c:536-577`); the parent's registers stay
/// frozen in its PD until the child's `xterm` does `run = run->p_parent` and
/// `termuser()` (`:601-618`). Nothing can schedule the parent in between —
/// EmuTOS's AES round-robin cannot even see it, because the launching
/// application and the launched one share **one** AESPD, so `dsptch()` has no
/// second entry to pick (`docs/analysis/coop-kernel/emutos-cooperative-model.md`
/// §5.2: *"GEMDOS `Pexec` … hat keine Verbindung zum AES-`dsptch()`-Scheduler.
/// Während die Kindanwendung läuft, laufen Accessories/Screen-Manager
/// nicht"*).
///
/// NewTOS gives parent and child **separate** slots, and a parent that ever
/// blocked in an `evnt_*` wait is left `runnable` by [`sleep`]'s idle exit — so
/// without this the child's first block would hand the CPU straight back to its
/// own suspended parent (measured: `measurements/measurement-2026-09-25-port05`, the parent
/// was re-entered `fresh` at its entry point and relaunched the child in a
/// loop). Dropping the bit is what expresses "the parent is inside `Pexec`".
///
/// Accessories keep running while the child is blocked exactly as before: they
/// are separate run-set members, and only the one parent is withdrawn.
///
/// No-op for a child whose `parent_slot` is [`super::NO_PARENT`] — the
/// kernel-launched root program (`kernel/src/launch.rs`) and desk accessories,
/// whose "parent" is an EL1 context that owns no slot and is not in the
/// run-set to begin with.
pub fn suspend_parent_for_child(child_slot: usize) {
    if child_slot >= MAX_PROCESSES {
        return;
    }
    let parent = PROCESS_TABLE[child_slot].parent_slot.load(Ordering::Acquire);
    if parent == super::NO_PARENT {
        return;
    }
    let p = &PROCESS_TABLE[parent as usize];
    PROCESS_TABLE[child_slot]
        .parent_runnable
        .store(p.runnable.load(Ordering::Acquire), Ordering::Release);
    // Off every list: EmuTOS has no entry to take off (parent and child share
    // the one AESPD), the parent is simply not a candidate until it returns.
    unready(parent as usize);
    // A process that reached `Pexec` is RUNNING, so by the WAITING/READY rule
    // (§3.2) its `wait_cond` is already `0`. Store it anyway: a stale wake key
    // would let [`wake`] re-arm `runnable` behind our back and put the
    // suspended parent back on the ready list while its child is still alive —
    // the exact hole this function closes.
    p.wait_cond.store(0, Ordering::Release);
}

/// The child in `child_slot` has terminated: put its parent back into the
/// cooperative run-set exactly as [`suspend_parent_for_child`] found it.
///
/// Called from the `Pterm` tail **before** the slot is released (releasing it
/// clears `parent_slot` and `parent_runnable`). Restores the saved bit rather
/// than forcing `true`, so a parent that was never a run-set member — a
/// program the kernel launcher started that has not yet blocked in an `evnt_*`
/// wait — does not become one by having launched a child.
pub fn resume_parent_of_child(child_slot: usize) {
    if child_slot >= MAX_PROCESSES {
        return;
    }
    let parent = PROCESS_TABLE[child_slot].parent_slot.load(Ordering::Acquire);
    if parent == super::NO_PARENT {
        return;
    }
    let was = PROCESS_TABLE[child_slot]
        .parent_runnable
        .load(Ordering::Acquire);
    let parent = parent as usize;
    if !was {
        unready(parent);
        return;
    }
    // The parent takes the child's place on `rlr`: in EmuTOS the two are one
    // AESPD, so the entry the child ran as *is* the parent's. A child that
    // is not on `rlr` (it never dispatched, or is waiting) leaves no place to
    // take; the parent then joins at the end. Either way it is the process
    // the `Pterm` resumes, and its next dispatch re-queues it.
    let child_on_rlr = PROCESS_TABLE[child_slot].runnable.load(Ordering::Acquire)
        && DRL_KEY[child_slot].load(Ordering::Acquire) == 0;
    if child_on_rlr {
        DRL_KEY[parent].store(0, Ordering::Release);
        RL_KEY[parent].store(RL_KEY[child_slot].load(Ordering::Acquire), Ordering::Release);
        PROCESS_TABLE[parent].runnable.store(true, Ordering::Release);
    } else {
        ready(parent);
    }
}

/// Add `slot` to the cooperative run-set as a fresh (never-entered) peer —
/// EmuTOS `pstart` (`aes/gempd.c:110-131`), which links the new process **on
/// top of `drl`** (`:125-128`). The next dispatch appends it to `rlr`, behind
/// the dispatching process and in reverse order of admission (A115).
pub fn admit(slot: usize) {
    if slot < MAX_PROCESSES {
        PROCESS_TABLE[slot].has_ctx.store(false, Ordering::Release);
        // A fresh peer has no saved frame; clear any stale restart-kind left by
        // a previous tenant of this slot so a later resume can't misread it.
        RESUME_RESTARTABLE[slot].store(false, Ordering::Release);
        unready(slot);
        push_drl(slot);
    }
}

/// Record how slot `slot` yielded, for the resume side (S2 restartable block,
/// design §4.3): `restartable = true` for a mid-syscall [`sleep`] yield (the
/// `saved_frame.elr` is rewound to the `svc`, so the resume preserves `x0`),
/// `false` for a leaf `Pwait` yield. Called by the exception handler's shared
/// frame-save; read by [`switch_to`] into [`Switch::restartable`].
#[inline]
pub fn set_resume_restartable(slot: usize, restartable: bool) {
    if slot < MAX_PROCESSES {
        RESUME_RESTARTABLE[slot].store(restartable, Ordering::Release);
    }
}

/// Mark `slot`'s saved context valid (the handler has snapshotted its
/// frame into `saved_frame`); a later switch will *resume* it.
#[inline]
pub fn mark_yielded(slot: usize) {
    if slot < MAX_PROCESSES {
        PROCESS_TABLE[slot].has_ctx.store(true, Ordering::Release);
    }
}

// ─── The ready list (A115) ──────────────────────────────────────────────────
//
// EmuTOS keeps three lists of AESPDs (`aes/geminit.c:131`): `rlr`, the ready
// list, whose head is the running process; `nrl`, the processes waiting for an
// event; and `drl`, the processes made ready since the last dispatch. `disp()`
// (`aes/gemdisp.c:221-253`) takes the running process off the head of `rlr`,
// appends it to the **end** (`disp_act` → `insert_process`, `:74-79`,
// `aes/gempd.c:133-145`) or parks it on `nrl` (`mwait_act`, `:82-93`), runs the
// fork ring (`forker`: timer and input completions → `signal`), and
// `schedule()` drains `drl` from its head onto the end of `rlr` (`:181-205`);
// then the **head** of `rlr` runs. `drl` is a push-front stack — `signal`
// (`aes/gemasync.c:56-57`) and `pstart` (`aes/gempd.c:125-128`) link at its
// head — so what became ready in one pass joins `rlr` in reverse order, and
// always behind the dispatching process.
//
// Membership stays the `runnable` bit ("field, not list", design §3.2):
// READY = on `rlr` or `drl`, WAITING (`nrl`) = `runnable == false` with a wake
// key. The **order** is two per-slot keys: [`RL_KEY`], the slot's position on
// `rlr` (a smaller key is nearer the head; appending takes the next value of a
// counter), and [`DRL_KEY`], non-zero while the slot is on `drl` (a larger key
// was pushed later, i.e. is nearer the head of the stack). The EL1 kernel
// client (EmuTOS pid 0, no slot) is the one entry without a slot: it is on
// `rlr` while its frame is parked ([`kernel_parked`]), at [`KERNEL_RL_KEY`].

/// Position of each slot on `rlr` — valid while the slot is `runnable` and not
/// on `drl` ([`DRL_KEY`] `== 0`). Smaller = nearer the head.
static RL_KEY: [AtomicU64; MAX_PROCESSES] = [const { AtomicU64::new(0) }; MAX_PROCESSES];

/// Non-zero while the slot is on `drl`: the push stamp (larger = pushed later
/// = nearer the head of the push-front stack).
static DRL_KEY: [AtomicU64; MAX_PROCESSES] = [const { AtomicU64::new(0) }; MAX_PROCESSES];

/// The kernel client's position on `rlr` while it is parked.
static KERNEL_RL_KEY: AtomicU64 = AtomicU64::new(0);

/// Counter behind [`RL_KEY`] / [`KERNEL_RL_KEY`]: each append takes the next.
static RL_SEQ: AtomicU64 = AtomicU64::new(1);

/// Counter behind [`DRL_KEY`]: each push takes the next.
static DRL_SEQ: AtomicU64 = AtomicU64::new(1);

/// Per-slot arm stamp of the timer deadline ([`arm_or_get_deadline`]), the
/// tie-break of [`wake_timers`]: `adelay` inserts a new delay **before** an
/// equal one (`if (c <= p->e_parm) break;`, `aes/geminput.c:525-530`), so of
/// two equal deadlines the later-armed one completes first.
static TIMER_ARM_SEQ: [AtomicU64; MAX_PROCESSES] = [const { AtomicU64::new(0) }; MAX_PROCESSES];

/// Counter behind [`TIMER_ARM_SEQ`].
static TIMER_SEQ: AtomicU64 = AtomicU64::new(1);

#[inline]
fn next_rl_key() -> u64 {
    RL_SEQ.fetch_add(1, Ordering::AcqRel)
}

/// `disp_act(p)` (`aes/gemdisp.c:74-79`): put `slot` on the **end** of `rlr`
/// — it is READY. Taken off `drl` if it was there. The wake key is left alone
/// (a READY slot has none, design §3.2; [`sleep`]'s idle exit clears its own).
///
/// The one way to make a slot runnable other than a wake ([`wake`],
/// [`wake_timers`]) and [`admit`]; tests and fixtures use it too, so the order
/// always knows every member.
pub fn ready(slot: usize) {
    if slot < MAX_PROCESSES {
        DRL_KEY[slot].store(0, Ordering::Release);
        RL_KEY[slot].store(next_rl_key(), Ordering::Release);
        PROCESS_TABLE[slot].runnable.store(true, Ordering::Release);
    }
}

/// Take `slot` off every ready list (`rlr` and `drl`) — it waits (`nrl`), is
/// suspended in a `Pexec`, or is gone. The wake key is the caller's business.
pub fn unready(slot: usize) {
    if slot < MAX_PROCESSES {
        PROCESS_TABLE[slot].runnable.store(false, Ordering::Release);
        DRL_KEY[slot].store(0, Ordering::Release);
    }
}

/// `signal`'s `p1->p_link = drl; drl = p1;` (`aes/gemasync.c:56-57`): push a
/// slot that has become ready onto the head of `drl`. A slot that is already
/// ready keeps its place — `signal` moves a process only off `nrl`
/// (`:42-59`).
fn push_drl(slot: usize) {
    if slot >= MAX_PROCESSES || PROCESS_TABLE[slot].runnable.load(Ordering::Acquire) {
        return;
    }
    DRL_KEY[slot].store(DRL_SEQ.fetch_add(1, Ordering::AcqRel), Ordering::Release);
    PROCESS_TABLE[slot].runnable.store(true, Ordering::Release);
}

/// `schedule()`'s `while (drl) { drl = (p = drl)->p_link; disp_act(p); }`
/// (`aes/gemdisp.c:193-197`): move `drl` onto the end of `rlr`, head of the
/// stack (the latest push) first.
fn drain_drl() {
    loop {
        let mut top: Option<(u64, usize)> = None;
        for (i, k) in DRL_KEY.iter().enumerate() {
            let k = k.load(Ordering::Acquire);
            if k != 0 && top.is_none_or(|(t, _)| k > t) {
                top = Some((k, i));
            }
        }
        let Some((_, i)) = top else {
            return;
        };
        DRL_KEY[i].store(0, Ordering::Release);
        if PROCESS_TABLE[i].runnable.load(Ordering::Acquire) {
            RL_KEY[i].store(next_rl_key(), Ordering::Release);
        }
    }
}

/// An entry of `rlr`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    Slot(usize),
    /// The parked EL1 kernel client.
    Kernel,
}

/// The head of `rlr`: the ready entry with the smallest key. `kernel_listed`
/// says whether the kernel client is on the list.
fn rlr_head(kernel_listed: bool) -> Option<Entry> {
    let mut best: Option<(u64, Entry)> = None;
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        if !p.runnable.load(Ordering::Acquire) || DRL_KEY[i].load(Ordering::Acquire) != 0 {
            continue;
        }
        let k = RL_KEY[i].load(Ordering::Acquire);
        if best.is_none_or(|(b, _)| k < b) {
            best = Some((k, Entry::Slot(i)));
        }
    }
    if kernel_listed {
        let k = KERNEL_RL_KEY.load(Ordering::Acquire);
        if best.is_none_or(|(b, _)| k < b) {
            best = Some((k, Entry::Kernel));
        }
    }
    best.map(|(_, e)| e)
}

/// The end of every dispatch: drain `drl` behind what is already on `rlr`
/// and resume the head (`schedule()` + `switchto(rlr->p_uda)`,
/// `aes/gemdisp.c:240-252`). `cur` is the dispatching context — a slot, or
/// [`NONE`] for the kernel client — whose own disposition (end of `rlr`, or
/// `nrl`) the caller has already made. `None` = the head is `cur` itself (or
/// nothing is ready): no switch.
fn pick_head(cur: usize) -> Option<Switch> {
    drain_drl();
    let kernel_listed = cur >= MAX_PROCESSES || kernel_parked();
    match rlr_head(kernel_listed)? {
        Entry::Slot(s) if s == cur => None,
        Entry::Slot(s) => Some(switch_to(s)),
        Entry::Kernel if cur >= MAX_PROCESSES => None,
        Entry::Kernel => Some(switch_to_kernel()),
    }
}

fn switch_to(slot: usize) -> Switch {
    let p = &PROCESS_TABLE[slot];
    Switch {
        slot,
        fresh: !p.has_ctx.load(Ordering::Acquire),
        l0_pa: p.l0_pa.load(Ordering::Acquire),
        entry_va: p.entry_va.load(Ordering::Acquire),
        sp_el0: p.sp_el0.load(Ordering::Acquire),
        // Step 3a: every scheduler peer is an EL0 process. EL1-internal-thread
        // classification lands in step 4 (design §2/§7.2).
        el1_internal: false,
        // How the slot last yielded — consulted only when `!fresh` to decide
        // whether the resume preserves `x0` (S2) or writes the `Pwait` 0.
        restartable: RESUME_RESTARTABLE[slot].load(Ordering::Acquire),
        upcall_restart: false,
        completed: false,
    }
}

/// A peer yielded (`Pwait`, or a dispatch whose caller has already made its
/// own disposition). A `cur` that is READY on `rlr` goes to its end
/// (`disp_act`); one that is not runnable stays off (`nrl`); one on `drl`
/// keeps its place there. Then `drl` is drained and the head of `rlr` is
/// picked — a slot, or the parked kernel client ([`switch_to_kernel`]).
/// `None` means "stay on the current process" (it is the head).
pub fn on_yield(cur: usize) -> Option<Switch> {
    if cur >= MAX_PROCESSES {
        return pick_peer_for_kernel();
    }
    if PROCESS_TABLE[cur].runnable.load(Ordering::Acquire)
        && DRL_KEY[cur].load(Ordering::Acquire) == 0
    {
        ready(cur);
    }
    pick_head(cur)
}

/// The current peer is terminating: drop it from the run-set and pick the
/// head of the ready list. `None` means nothing is ready — the handler should
/// fall back to the ordinary `Pterm` parent/boot resume.
pub fn on_exit(cur: usize) -> Option<Switch> {
    unready(cur);
    let next = pick_head(cur);
    if next.is_none() {
        CURRENT.store(NONE, Ordering::Release);
    }
    next
}

// ─── sleep / wake (sleep/wake substrate, steps 3a + 3b) ─────────────
//
// The block primitive `sleep(cond)` replaces the six per-`evnt_*` busy-poll
// loops in `aes/event.rs` (design §5); `wake(cond)` is the FreeMiNT `do_wake`
// analogue that re-arms matching waiters. `sleep` has the two exits of design
// §4.4: **Ausgang (a)**, the no-peer idle-fallback (step 3a) — with exactly one
// runnable context there is no peer to switch to, so `sleep` degrades to
// exactly today's pump-and-recheck spin — and **Ausgang (b)**, the real peer
// switch (step 3b): when a runnable peer exists, `sleep` returns
// [`SleepOutcome::Yield`] so the caller aborts its (non-leaf) syscall and the
// trap handler performs the S2 restartable-block save + `apply_switch`.

/// The outcome of a [`sleep`] call — its two exits (design §4.4).
#[must_use]
pub enum SleepOutcome {
    /// **Ausgang (a).** The idle-fallback ran (no runnable peer — the step-3
    /// single-context case): host input was pumped and the wake sources polled.
    /// The caller re-checks its full predicate and re-sleeps if the event has
    /// not fired.
    Recheck,
    /// **Ausgang (b).** A runnable peer exists (step 3b): the caller must
    /// **abort its syscall** and propagate this `Switch` up to the trap handler,
    /// which saves the caller's frame with the ELR rewound to the `svc` (S2
    /// restartable block, §4.3) and switches to the peer. The caller must **not**
    /// commit any results — the syscall re-executes from the top on resume.
    Yield(Switch),
}

/// Block the current context on wake-class key `cond` until its predicate
/// fires (design §4.2/§4.4). Returns one of the two [`SleepOutcome`] exits.
///
/// Mark the caller WAITING(`cond`) (drop out of the round-robin run-set, §3.1),
/// then:
///
///   * **Ausgang (b)** — if a runnable **peer** exists, return
///     [`SleepOutcome::Yield`] carrying the [`Switch`] to it. `sleep` sits deep
///     inside a non-leaf syscall (`evnt_multi`) and does **not** own the trap
///     frame, so it cannot switch here: the caller propagates the `Yield` up to
///     `rust_sync_handler`, which does the S2 frame-save (ELR rewound to the
///     `svc`) + `apply_switch`. The caller left the slot WAITING; on resume the
///     re-executed `svc` re-enters the syscall, re-checks, and fires.
///   * **Ausgang (a)** — otherwise (no peer, the step-3 single-context case):
///     pump host input, run the idle wake-pass ([`idle_wake_pass`], §5.2), mark
///     the caller READY again, and return [`SleepOutcome::Recheck`]. This is
///     behaviorally identical to the former per-`evnt_*` `host().pump_input()`
///     spin — only the wait *mechanism* moves here.
///
/// **The EL1 kernel client (ACC03).** A caller that owns no slot — the boot
/// path; until DESK43 U10 also the built-in desktop, which reached `evnt_*`
/// from EL1 — has
/// no slot to mark WAITING; its wait is a poll, appended to the ready list
/// like a yield ([`pick_peer_for_kernel`]). It still has the two exits,
/// mirrored:
///
///   * **Ausgang (b)** — a runnable EL0 peer exists (a desk accessory that was
///     woken by [`crate::aes::pd::post_msg`]): yield to it. The handler parks
///     the EL1 frame (the "kernel park") and switches; when the accessory
///     blocks again it comes straight back here and the rewound `svc`
///     re-enters this wait. This is EmuTOS's `dsptch()` from inside `ev_block`
///     (`aes/gemevlib.c`, `gemasm.S:169-194`), for the one client that is not
///     a process.
///   * **Ausgang (a)** — nothing runnable: the idle pass, exactly as before.
///
/// **Yielding *to* the parked kernel client.** The symmetric case: an EL0
/// accessory blocks, no EL0 peer is runnable, and an EL1 client is parked
/// ([`kernel_parked`]). Without this exit the accessory would spin in the idle
/// fallback forever and the desktop — which is not in the run-set and can
/// therefore never be picked — would never run again. `sleep` returns
/// [`switch_to_kernel`], which `apply_switch` turns into the kernel-park
/// restore.
///
/// `cond` is built with [`wait_input`]/[`wait_mesg`]/[`wait_timer`]/
/// [`wait_multi`]/[`wait_mutex`]/[`wait_qnq`].
pub fn sleep(cond: u64) -> SleepOutcome {
    let cur = running_slot();
    // KEYS02 (O1): bytes for this process's `kbdvec` hook are waiting. They
    // are delivered only on a return to EL0, so give way before blocking —
    // the trap handler restarts this call after the hook ran.
    if cur < MAX_PROCESSES && crate::xbios::kbdvecs::pending(cur) != 0 {
        return SleepOutcome::Yield(switch_restart_for_upcall());
    }
    if cur < MAX_PROCESSES {
        // RUNNING → WAITING(cond): record the wake key and drop out of the
        // round-robin run-set (§3.1). Done before the pick so the head of the
        // ready list is never this slot — a returned `Switch` is a real peer.
        PROCESS_TABLE[cur].wait_cond.store(cond, Ordering::Release);
        // `mwait_act`: off `rlr`, onto `nrl` (`aes/gemdisp.c:82-93`).
        unready(cur);

        // `disp()` drains the fork ring on **every** dispatch (`forker()`,
        // `aes/gemdisp.c:239-244`), and `tchange` there completes every
        // expired timer wait whoever is running (A112, `docs/api/aes/
        // event.md` "Wake conditions and dispatch" points 2 and 4). After the
        // caller's own disposition and before `schedule()`'s drain, so a peer
        // whose deadline has passed is on `drl` when it is drained.
        wake_timers();

        // Ausgang (b) — a ready entry exists (§4.4): yield to the head of
        // `rlr` — an EL0 peer, or the parked EL1 kernel client (ACC03), one
        // more entry of the list. The handler performs the S2 frame-save (ELR
        // rewound to the svc) + switch; we do NOT touch the trap frame here —
        // `sleep` is deep in a non-leaf syscall and does not own it.
        if let Some(sw) = pick_head(cur) {
            return SleepOutcome::Yield(sw);
        }
    } else {
        wake_timers();
        if let Some(sw) = pick_peer_for_kernel() {
            // The EL1 kernel client blocks and an accessory is runnable: `dsptch()`.
            return SleepOutcome::Yield(sw);
        }
    }

    // Ausgang (a) — the no-peer idle-fallback (§4.4): EmuTOS `schedule()`-idle.
    // Pump input, then run the wake sources. Behaviorally identical to today's
    // per-`evnt_*` pump-and-recheck spin.
    if let Some(h) = host() {
        h.pump_input();
    }
    idle_wake_pass();

    if cur < MAX_PROCESSES {
        // WAITING → READY: the caller now re-evaluates its full predicate. It
        // is the head of `rlr` (nothing else was ready), so it goes back on
        // it; anything the idle pass woke is on `drl` and joins behind it at
        // the next dispatch.
        PROCESS_TABLE[cur].wait_cond.store(0, Ordering::Release);
        ready(cur);
        // KEYS02 (O1): the pump above may have queued hook bytes for this
        // process — deliver them now rather than after the wait.
        if crate::xbios::kbdvecs::pending(cur) != 0 {
            return SleepOutcome::Yield(switch_restart_for_upcall());
        }
    }
    SleepOutcome::Recheck
}

/// Wake every context WAITING on exactly `cond` (WAITING → READY, design §4.2):
/// clear its [`super::Process::wait_cond`] and push it onto `drl` (EmuTOS
/// `signal`, `aes/gemasync.c:46-59`; A115) — the next dispatch appends it to
/// the ready list behind the dispatching process. Several waiters woken by one
/// key are pushed in slot order and so join the list in reverse slot order. The
/// FreeMiNT `do_wake` analogue (`references/freemint/sys/proc.c:665-701`) — it
/// does **not** switch immediately; a woken peer runs only when the current
/// context next [`sleep`]s or exits (design §4.2).
///
/// In step 3a (one runnable context) `wake` is not yet load-bearing — the
/// caller's own idle re-check already observes the event. It is wired now so
/// the step-4 peer-switch path and the message/input/timer posters do not have
/// to touch `event.rs` later (design §1.3).
///
/// **Union waits (A112).** A [`wait_multi`] waiter is woken by every key its
/// event bits cover: a [`wait_input`] key wakes it when it asked for
/// [`WAKE_INPUT`], a [`wait_mesg`]`(idx)` key when it asked for [`WAKE_MESG`]
/// on that same identity — EmuTOS's `signal()` ORs the completed event into
/// `p_evflg` and wakes the owner if `p_evflg & p_evwait` (`aes/gemasync.c:
/// 35-61`), whatever the owner waits for besides.
pub fn wake(cond: u64) {
    if cond == 0 {
        return; // 0 is "not waiting", never a valid wake key.
    }
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        let c = p.wait_cond.load(Ordering::Acquire);
        if c == cond || multi_covers(c, cond) {
            p.wait_cond.store(0, Ordering::Release);
            // `signal`: off `nrl`, onto the head of `drl` (A115).
            push_drl(i);
        }
    }
}

/// Bytes for `slot`'s `kbdvec` hook were queued while another process had
/// the CPU (KEYS02 D9, MEM12: the hook owner is the innermost started
/// process, not the running one). A blocking wait gives way only when its own
/// process runs (`sleep`'s ring check), so a WAITING owner is made READY
/// (onto `drl`, as [`wake`] does) — it then re-enters its wait, finds the
/// bytes and yields for the delivery. A READY or RUNNING owner needs nothing.
pub fn wake_for_upcall(slot: usize) {
    let Some(p) = PROCESS_TABLE.get(slot) else {
        return;
    };
    if p.wait_cond.load(Ordering::Acquire) != 0 {
        p.wait_cond.store(0, Ordering::Release);
        push_drl(slot);
    }
}

/// `true` when the [`wait_multi`] key `waiter` asked for the source the
/// single-class key `cond` names ([`wait_input`], [`wait_mesg`] or
/// [`wait_mutex`]).
fn multi_covers(waiter: u64, cond: u64) -> bool {
    if wait_class(waiter) != CLASS_MULTI {
        return false;
    }
    let events = wait_arg(waiter) & MULTI_EVENTS_MASK;
    match wait_class(cond) {
        CLASS_INPUT => events & WAKE_INPUT != 0,
        CLASS_MUTEX => events & WAKE_MUTEX != 0,
        CLASS_MESG => {
            events & WAKE_MESG != 0
                && (wait_arg(waiter) >> MULTI_IDX_SHIFT) == wait_arg(cond)
        }
        _ => false,
    }
}

/// The timer half of the dispatcher's fork-ring pass (A112): every WAITING
/// context whose deadline has passed is made READY — a [`wait_timer`] key by
/// the deadline it carries, a [`wait_multi`] key with [`WAKE_TIMER`] by its
/// slot's [`super::Process::syscall_deadline`].
///
/// EmuTOS: the tick handler counts the head delay down and, at zero, queues
/// `tchange` (`aes/gemdosif.S:389-401`); `forker()` runs it on every
/// `disp()` (`aes/gemdisp.c:239-244`) and at the head of `ev_multi`, and it
/// completes each expired `EVB` through `evremove` → `azombie` → `signal`
/// (`aes/gemflag.c:32-68`) — independent of the mouse owner and of the
/// process that happens to be running. Called from every dispatch here:
/// [`sleep`], [`dsptch`], [`dispatch_after_call`] and the idle pass.
pub fn wake_timers() {
    let Some(h) = host() else {
        return;
    };
    let now = h.monotonic_millis();
    // The expired waits, completed in `dlr` order (A115): `tchange` takes the
    // delay list from its head (`gemflag.c:40-52`) and `adelay` keeps it
    // sorted by deadline, a new delay before an equal one
    // (`geminput.c:520-540`). Each completion pushes onto `drl`, so the order
    // here decides the order on `rlr`.
    let mut expired: [(u64, u64, usize); MAX_PROCESSES] = [(0, 0, 0); MAX_PROCESSES];
    let mut n = 0;
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        let c = p.wait_cond.load(Ordering::Acquire);
        let deadline = match wait_class(c) {
            CLASS_TIMER if wait_arg(c) <= now => Some(wait_arg(c)),
            CLASS_MULTI if wait_arg(c) & WAKE_TIMER != 0 => {
                let d = p.syscall_deadline.load(Ordering::Acquire);
                (d != 0 && d <= now).then_some(d)
            }
            _ => None,
        };
        if let Some(d) = deadline {
            // Sort key: deadline ascending, then the later arm first.
            expired[n] = (d, u64::MAX - TIMER_ARM_SEQ[i].load(Ordering::Acquire), i);
            n += 1;
        }
    }
    let expired = &mut expired[..n];
    expired.sort_unstable();
    for &(_, _, i) in expired.iter() {
        PROCESS_TABLE[i].wait_cond.store(0, Ordering::Release);
        push_drl(i);
    }
}

// ─── dsptch: the yield that keeps the caller on the ready list ──────────────
//
// `sleep` above is EmuTOS's `WAITIN` branch. This is the other one, and the AES
// needs both: a poll loop whose condition no wake class covers must yield, not
// sleep, or nothing will ever wake it (ACC10).

/// The outcome of a [`dsptch`] pass — the two exits of [`SleepOutcome`], with
/// the caller left **on** the ready list.
#[must_use]
pub enum Dispatch {
    /// A ready **peer** got the CPU. Like [`SleepOutcome::Yield`] the caller
    /// must abort its syscall and propagate the [`Switch`] to the trap handler
    /// (the S2 restartable block, design §4.3); unlike it, the caller is READY,
    /// so the round-robin comes back on its own.
    Yield(Switch),
    /// Nobody else was ready: the idle pass ran (host input pumped, wake
    /// sources polled) and the caller simply carries on.
    Idle,
}

/// `dsptch()` (`aes/gemasm.S:129-167` → `disp`, `aes/gemdisp.c:220-254`) — give
/// the CPU to the next ready context **without leaving the ready list**.
///
/// The second block primitive next to [`sleep`], and the difference is the
/// entire point. `disp` takes the running process off the head of `rlr` and
/// then branches on its *own* state (`gemdisp.c:229-231`):
///
/// ```text
/// if (p->p_stat & WAITIN)  mwait_act(p);   /* park it on nrl  -> `sleep`  */
/// else                     disp_act(p);    /* back on rlr     -> `dsptch` */
/// ```
///
/// and `disp_act` is `insert_process(p, &rlr)` (`gemdisp.c:74-79`) — straight
/// back onto the ready list. A process that calls `dsptch()` outside an
/// `ev_block` has not set `WAITIN`, so it is rescheduled unconditionally, with
/// no wake source, no event and no input involved.
///
/// **It is an insert, not a promise.** `disp_act` *puts* the caller on the
/// ready list; it does not assume it is there. That distinction is the whole
/// of SCHED01: NewTOS has no `pstart`-equivalent for a launched program — in
/// EmuTOS one is not needed, because `sh_ldapp` runs the program under the
/// *shell's own* AESPD (`aes/gemshlib.c:604-605,634-636`: it renames `rlr`
/// and `dos_exec`s), and `rlr` is by definition the ready list — so the only
/// thing that ever made a NewTOS program a ready-list member was the
/// `runnable.store(true)` in [`sleep`]'s idle exit. A program that never
/// blocks in an AES wait therefore reached `wait_for_accs` outside the
/// run-set and hung in `appl_exit`. This function establishes the membership
/// that EmuTOS has by construction.
///
/// **When to use which.** [`sleep`] is right for a wait that a wake class
/// covers (`evnt_keybd`, `evnt_mesag`, `evnt_timer`): the caller is parked and
/// the poster wakes it. `dsptch` is right for a poll loop over state no wake
/// class covers — `wait_for_accs`'s walk over the desk accessories' `p_flags`
/// (`aes/geminit.c:609-626`) is the AES's one instance. Sleeping on an
/// unrelated class there parks the caller until something unrelated happens,
/// which on a machine nobody is touching is never (ACC10; the barrier used to
/// sleep on `wait_input()` and a headless `appl_exit` did not return).
///
/// The counterpart for the *other* half of `disp` — the `forker(); schedule();`
/// input drain every completed AES call ends with — is `aes::dsptch`.
pub fn dsptch() -> Dispatch {
    let cur = running_slot();
    if cur < MAX_PROCESSES {
        // `disp_act(p)` = `insert_process(p, &rlr)` (`gemdisp.c:74-79`): the
        // caller is **put on** the ready list. Establishing the membership is
        // the point — EmuTOS's running process is on `rlr` by construction
        // (`disp` takes it off the head at `:225-226`, `disp_act` links it
        // back in at the tail), and a NewTOS slot has no such guarantee:
        // `admit` is called for desk accessories only (`aes/acc.rs`), and a
        // launched program acquires the bit solely as a side effect of
        // [`sleep`]'s idle exit, i.e. only once it has blocked in some AES
        // wait. A full-screen program with no event loop never blocks, so it
        // reached this line with `runnable == false`, handed the CPU to the
        // accessories, and the scheduler then had no candidate to hand it back
        // to: measured, the ball demo never returned from `appl_exit` with an
        // accessory loaded (SCHED01, `measurements/measurement-2026-09-29-port08`). Set
        // before the pick, so the peer we switch to already sees the caller on
        // the list.
        //
        // `wait_cond` is deliberately left alone. EmuTOS's `disp_act` clears
        // `WAITIN` because `mwait_act` calls it too, for a waiter whose event
        // flags are already set (`gemdisp.c:82-90`); our counterpart of that
        // shortcut is [`sleep`]'s idle exit, which clears the key itself. In
        // the `else` branch — the one this function is — the caller is RUNNING
        // and its key is `0` by the WAITING/READY rule (§3.2), so the clear is
        // the no-op it is in EmuTOS.
        ready(cur);
        // `forker()` after `disp_act` (`aes/gemdisp.c:231-244`): a process
        // whose timer expired goes onto `drl` and so joins `rlr` *behind* the
        // caller and the ones already on it (A112, A115).
        wake_timers();
        if let Some(sw) = pick_head(cur) {
            return Dispatch::Yield(sw);
        }
    } else {
        // The EL1 kernel client dispatches: appended, timers, drain, head.
        wake_timers();
        if let Some(sw) = pick_peer_for_kernel() {
            return Dispatch::Yield(sw);
        }
    }

    // Nothing to switch to. `disp`'s `schedule()` still runs (`gemdisp.c:240`),
    // which is our idle pass; then the caller carries on, exactly as EmuTOS's
    // would after being put straight back on a ready list it is alone on.
    if let Some(h) = host() {
        h.pump_input();
    }
    idle_wake_pass();
    Dispatch::Idle
}

/// The `dsptch()` a **completed** AES call ends with (A112,
/// `docs/api/aes/event.md` "Wake conditions and dispatch" point 3): `super()`
/// runs the opcode switch and then falls through to `dsptch()` for every AES
/// call (`aes/gemsuper.c:454-466`), and `disp()` puts the caller at the **end**
/// of the ready list (`disp_act` → `insert_process`, `aes/gemdisp.c:74-79`,
/// `aes/gempd.c:133-145`) before resuming its head — so a ready peer runs
/// before the caller continues, round robin.
///
/// The input half of that `disp()` (the `forker()` drain of the mouse and
/// keyboard packets) is `aes::dsptch`, which has already run when this is
/// called; this is the scheduling half. Returns the [`Switch`] to the next
/// ready context with [`Switch::completed`] set — the trap handler finishes
/// the call and then yields as a leaf (the caller resumes after its `svc`) —
/// or `None` when the caller is alone on the ready list and simply goes on.
///
/// Order (A115): the caller is appended to the end of the ready list
/// ([`ready`], `disp_act`), the timer pass runs ([`wake_timers`], `forker()`),
/// and `drl` — the timer completions and everything made ready during the call
/// itself, e.g. the reader an `appl_write` serviced — is appended **behind the
/// caller** in reverse wake order (`schedule()`, `gemdisp.c:231-244`). A
/// caller that is otherwise alone is therefore resumed at once, and what it
/// woke runs at its *next* dispatch.
///
/// Only an EL0 caller with a slot dispatches here: the EL1 kernel client owns
/// no slot to save a leaf frame into and makes no AES calls since DESK43 U10.
pub fn dispatch_after_call() -> Option<Switch> {
    let cur = running_slot();
    if cur >= MAX_PROCESSES {
        return None;
    }
    // `disp_act(p)`: the caller is put (back) on the ready list — see
    // [`dsptch`] for why the membership has to be established, not assumed.
    ready(cur);
    wake_timers();
    pick_head(cur).map(|mut sw| {
        sw.completed = true;
        sw
    })
}

// ─── timer deadline persistence (S2-restart idempotency, §4.3 OPEN) ─────────
//
// A timer-bearing wait (`evnt_timer`/`wait_ms`/`evnt_multi`-`MU_TIMER`) computes
// its deadline as `monotonic_millis() + ms`. Under the S2 restartable block
// (§4.3) the whole syscall re-executes from the top on resume, so recomputing
// the deadline at the function head would push it forward by `ms` on every
// restart — with a live peer it would never converge (irrelevant with one
// context/no peer, real with step-4 peers). Persisting the deadline per-slot in
// [`super::Process::syscall_deadline`] makes a restart REUSE the original
// deadline; first-entry-vs-restart is detected by the field being `0` vs set.

/// The slot whose [`super::Process::syscall_deadline`] a timer wait uses: the
/// slot [`sleep`] parks — [`running_slot`], derived from `RUN` and maintained
/// on every path that enters a process — and only when that names none, the
/// scheduler's [`current`]. (A112: a program the kernel launcher entered
/// directly has `current() == NONE` until its first switch, so keying the
/// deadline off `current()` alone left it unpersisted and invisible to the
/// timer pass, [`wake_timers`].)
fn deadline_slot() -> usize {
    let r = running_slot();
    if r < MAX_PROCESSES {
        r
    } else {
        current()
    }
}

/// Arm (first entry) or reuse (S2 restart) the running slot's timer deadline
/// (design §4.3). When the slot's [`super::Process::syscall_deadline`] is `0`
/// (first entry), computes `now + ms`, stores it, and returns it; when it is
/// already set (an S2 restart re-entered the wait), returns the STORED value
/// unchanged — so the elapsed time while a peer ran does not move the deadline.
/// Cleared on completion by [`clear_deadline`].
///
/// With no running scheduler slot — an EL1 kernel context, whose timer waits run with
/// [`current`]` == NONE` and never restart (idle-fallback only, §4.4) — there is
/// nothing to persist, so it just returns `now + ms` (today's behavior).
pub fn arm_or_get_deadline(now: u64, ms: u64) -> u64 {
    let cur = deadline_slot();
    if cur < MAX_PROCESSES {
        let field = &PROCESS_TABLE[cur].syscall_deadline;
        let stored = field.load(Ordering::Acquire);
        if stored != 0 {
            return stored; // S2 restart: reuse the original deadline
        }
        let deadline = now.saturating_add(ms);
        field.store(deadline, Ordering::Release);
        TIMER_ARM_SEQ[cur].store(TIMER_SEQ.fetch_add(1, Ordering::AcqRel), Ordering::Release);
        deadline
    } else {
        now.saturating_add(ms)
    }
}

/// Clear the running slot's persisted timer deadline (design §4.3). Called on
/// EVERY completion path of a timer-bearing wait — it fires and returns, or
/// another event (input/message) fires first in `evnt_multi` — so a later,
/// unrelated timer wait recomputes a fresh deadline via [`arm_or_get_deadline`].
/// A no-op when no slot is running or no deadline was armed.
pub fn clear_deadline() {
    let cur = deadline_slot();
    if cur < MAX_PROCESSES {
        PROCESS_TABLE[cur].syscall_deadline.store(0, Ordering::Release);
    }
}

/// Pointer state seen by the previous [`idle_wake_pass`] — `x`, `y` and the
/// button mask packed into one word, so one atomic compare detects any
/// mouse-borne event. Seeded to a value no real pointer state can take
/// (`u64::MAX`) so the first pass always reports a change.
static LAST_POINTER: AtomicU64 = AtomicU64::new(u64::MAX);

/// The live pointer state as one word: `x`, `y` and the button mask.
fn mouse_state_word() -> u64 {
    let (mx, my) = crate::vdi::mouse_position();
    ((mx as u16 as u64) << 32) | ((my as u16 as u64) << 16) | (crate::vdi::mouse_buttons() as u64)
}

/// The cooperative idle wake-pass (design §5.2): the single point where the
/// input and timer wake sources are polled and turned into WAITING → READY
/// transitions — EmuTOS's `schedule()`/`chkkbd` idle, **not** the ISR (the IRQ
/// handler only fills the ring, design §1.2). Runs once per [`sleep`] pass.
fn idle_wake_pass() {
    let Some(h) = host() else {
        return;
    };
    // WAIT_INPUT: console/keyboard input has landed in the ring → wake input
    // waiters (§5.2, EmuTOS `chkkbd`).
    //
    // **Mouse/button changes wake them too** (ACC03). The previous rule — "they
    // reach the VDI state through `pump_input` and are caught by the waiter's
    // own predicate re-check" — holds only while the waiter *is* the context
    // running the idle pass. With a second client in the run-set the idle pass
    // runs in whichever context happens to be on the CPU, and a peer WAITING on
    // `MU_BUTTON`/`MU_M1` can only re-check its predicate once it is scheduled
    // again — which needs a wake. `pump_input` above has already folded the
    // ring into the VDI pointer state, so comparing that state against the
    // previous pass is the cheapest faithful "something happened" test; a
    // spurious wake is harmless (the waiter re-checks and re-sleeps), a missing
    // one is a lost event.
    let pointer = mouse_state_word();
    if h.console_has_input() || pointer != LAST_POINTER.swap(pointer, Ordering::AcqRel) {
        wake(wait_input());
    }
    // WAIT_TIMER / WAIT_MULTI(WAKE_TIMER): per-waiter deadline reached → wake.
    wake_timers();
}
