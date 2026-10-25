//! Desk accessories — the AES half of the `.NAC` lifecycle (ACC02-ACC04).
//!
//! Spec: [`docs/api/aes/accessories.md`] (the end-to-end lifecycle extracted
//! from EmuTOS), with [`docs/api/aes/menu.md`] for `mn_register`/`menu_fixup`
//! and [`docs/api/aes/event.md`] for the `AP_MESAG`/`AP_ACCLOSE` flags.
//!
//! This module owns the two things EmuTOS keeps in `aes/geminit.c` that are
//! not menu state: the set of **accessory processes** that exist (EmuTOS finds
//! them by walking `fpdnm(NULL, pid)` over the reserved DA pid range 2…,
//! `geminit.c:616-624`, because its AESPD slots 2..`num_accs+1` *are* the
//! accessory list) and [`wait_for_accs`], the barrier both `run_accs_and_desktop`
//! (`geminit.c:737`) and `ap_exit` (`gemaplib.c:255`) block on.
//!
//! NewTOS has no reserved descriptor range — an accessory's AES identity is the
//! GEMDOS slot its process got (`aes/pd.rs`, A62) — so the list is explicit
//! here, filled by the kernel-side loader ([`register_process`]) after each
//! successful load, in load order. That is a **mechanism** difference only:
//! what a program can observe is EmuTOS's — which processes the barrier waits
//! for, that it gives up after `WAIT_TIMEOUT` passes, and the pid an accessory
//! reports, `2 + its position in this list` (`appl::reported_pid`, A113;
//! `geminit.c:236-257,819`).
//!
//! The loading half (`count_accs`/`load_one_acc`/`load_accs`/`free_accs`) is
//! kernel-side in `kernel/src/accs.rs`: it needs the ELF loader and the
//! process table, which sit below this layer.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::pd;
use crate::bdos::process::sched;

/// `AP_MESAG` (`aes/struct.h:116`) — "the application has waited for a
/// message". Re-exported so the kernel-side loader can name the barrier it
/// waits on without reaching into the private descriptor module.
pub const AP_MESAG: u16 = pd::AP_MESAG;
/// `AP_ACCLOSE` (`aes/struct.h:117`) — "the application has seen an AC_CLOSE
/// message". Re-exported alongside [`AP_MESAG`].
pub const AP_ACCLOSE: u16 = pd::AP_ACCLOSE;

/// `NUM_ACCS` (`include/sysconf.h:37-40`) — "the maximum number of desk
/// accessory files that will be loaded **AND** the maximum number of desk
/// accessory slots available (one slot per `mn_register()` call)": one shared
/// budget, not two. `sysconf.h`'s own header states these definitions are not
/// overridable, so this is unconditional in every EmuTOS build.
pub const NUM_ACCS: usize = 6;

/// `WAIT_TIMEOUT` (`aes/geminit.c:102`) — "as a precaution against infinite
/// loops, we set a count timeout. in limited testing, the maximum number of
/// loops actually used was less than 100." [`wait_for_accs`] gives up after
/// this many passes and returns as if the flag had been set.
pub const WAIT_TIMEOUT: u32 = 500;

/// Passes already spent by an **in-flight** [`wait_for_accs`], one counter per
/// AES identity ([`pd::NUM_PDS`]); `0` = no wait in flight.
///
/// EmuTOS's `n` (`aes/geminit.c:602,609`) is a C automatic and can be, because
/// its loop never leaves the function: `dsptch()` returns to the very same
/// instruction once the process is rescheduled. A NewTOS pass **does** leave
/// the function — the yield unwinds the AES call and the re-executed `svc`
/// re-enters it from the top (the S2 restartable block,
/// `docs/analysis/coop-kernel/20-sleepwake-substrate-design.md` §4.3) — so a
/// counter on the stack would be reset by every pass that yields, which is
/// precisely the pathological case `WAIT_TIMEOUT` exists to bound. It lives out
/// here instead, for the same reason and in the same shape as the timer
/// deadline ([`sched::arm_or_get_deadline`]): armed on first entry, carried
/// across restarts, cleared on both exits and by [`forget_wait`].
static WAIT_PASSES: [AtomicU32; pd::NUM_PDS] = [const { AtomicU32::new(0) }; pd::NUM_PDS];

/// The AES identities (process-descriptor indices) of the loaded accessory
/// processes, in load order — EmuTOS's AESPD slots 2..`num_accs+1`.
/// `usize::MAX` is the empty entry.
struct AccPds(UnsafeCell<[usize; NUM_ACCS]>);
// SAFETY: single boot CPU; touched only from the AES/boot path.
unsafe impl Sync for AccPds {}
static ACC_PDS: AccPds = AccPds(UnsafeCell::new([usize::MAX; NUM_ACCS]));

/// Borrow the accessory identity table.
///
/// # Safety
/// Single AES/boot path (no re-entrancy).
#[allow(clippy::mut_from_ref)]
unsafe fn table() -> &'static mut [usize; NUM_ACCS] {
    // SAFETY: see `AccPds`'s Sync impl.
    unsafe { &mut *ACC_PDS.0.get() }
}

/// Record that identity `idx` is a desk-accessory process — the loader's
/// counterpart of EmuTOS reserving AESPD slot `2 + n` for the `n`-th `.ACC`
/// (`geminit.c:819,851-877`). Ignores a repeat registration and anything past
/// the [`NUM_ACCS`] budget.
///
/// Also puts the slot into the cooperative run-set ([`sched::admit`]) as a
/// **fresh** peer: like EmuTOS's `pstart(gotopgm, …)` (`geminit.c:255`) this
/// only makes the process schedulable — it does not run until something
/// dispatches, which for us is the first [`wait_for_accs`] / blocking
/// `evnt_*` (EmuTOS's `dsptch()`, `geminit.c:736`). `pstart` links it on top
/// of `drl` (`gempd.c:125-128`), so accessories registered in load order join
/// the ready list — and are first run — in **reverse** load order (A115).
pub fn register_process(idx: usize) {
    // SAFETY: single AES/boot path.
    let t = unsafe { table() };
    if t.contains(&idx) {
        return;
    }
    if let Some(free) = t.iter_mut().find(|e| **e == usize::MAX) {
        *free = idx;
        sched::admit(idx);
    }
}

/// Forget accessory process `idx` — used when a loaded accessory terminates
/// (EmuTOS's never do, `docs/api/aes/accessories.md` §4 point 6; called from
/// [`super::release_client`] and the kernel's `free_accs`) so the barriers
/// below stop waiting for a process that is gone. The entry is emptied **in
/// place**: the other accessories keep their positions and therefore their
/// pids ([`position_of`]).
pub fn forget_process(idx: usize) {
    // SAFETY: single AES/boot path.
    for e in unsafe { table() }.iter_mut() {
        if *e == idx {
            *e = usize::MAX;
        }
    }
}

/// The identities of the loaded accessory processes, in load order.
pub fn processes() -> impl Iterator<Item = usize> {
    // SAFETY: single AES/boot path; the returned values are plain indices.
    let t = *unsafe { table() };
    t.into_iter().filter(|e| *e != usize::MAX)
}

/// The position of identity `idx` in the accessory list — EmuTOS's
/// `D.g_acc[n]`, i.e. the `n` of pid `2 + n` (`aes/geminit.c:236-257`,
/// `gempd.c:60-70`; spec `docs/api/aes/application.md` "Pid assignment,
/// accessories, and a failed load"). Positions are fixed for the session: an
/// entry [`register_process`] filled keeps its index, and [`forget_process`]
/// empties it in place, so a forgotten accessory never renumbers the others.
/// `None` when `idx` is no loaded accessory.
pub(super) fn position_of(idx: usize) -> Option<usize> {
    // SAFETY: single AES/boot path; read only.
    unsafe { table() }.iter().position(|e| *e == idx)
}

/// The identity of the accessory at list position `n` (the inverse of
/// [`position_of`]), or `None` for a position beyond [`NUM_ACCS`] or one no
/// loaded accessory holds.
pub(super) fn at(n: usize) -> Option<usize> {
    // SAFETY: single AES/boot path; read only.
    unsafe { table() }.get(n).copied().filter(|e| *e != usize::MAX)
}

/// Is identity `idx` a loaded desk accessory — an `AESPD` of its own
/// (`D.g_acc[]`, `aes/geminit.c:819`), as opposed to a member of the
/// application chain? Used by the keyboard/mouse-owner comparison
/// (`input::same_pd`).
pub fn is_process(idx: usize) -> bool {
    processes().any(|e| e == idx)
}

/// Number of loaded accessory processes.
pub fn count() -> usize {
    processes().count()
}

/// Drop the whole accessory list — the AES-restart half (`mn_init`'s
/// neighbourhood, [`super::reinit`]). The processes themselves are freed by
/// the kernel's `free_accs`.
pub(super) fn reinit() {
    // SAFETY: single restart path, no AES call in flight.
    unsafe { table() }.fill(usize::MAX);
    for c in WAIT_PASSES.iter() {
        c.store(0, Ordering::Release);
    }
    for f in IN_WAIT_LINE.iter() {
        f.store(false, Ordering::Release);
    }
}

/// `true` when every loaded accessory has `bitmask` set in its `p_flags` —
/// `wait_for_accs`'s inner loop (`geminit.c:616-624`).
pub fn all_have(bitmask: u16) -> bool {
    processes().all(|idx| {
        // SAFETY: single AES path; `by_index` hands out the live descriptor.
        match unsafe { pd::by_index(idx) } {
            Some(p) => p.flags & bitmask != 0,
            None => true,
        }
    })
}

/// `all_run()` (`aes/geminit.c:554-566`) — "give everyone a chance to run, at
/// least once": one [`sched::dsptch`] per loaded accessory.
///
/// Returns `Some(switch)` as soon as one of those dispatches finds a peer: a
/// NewTOS syscall can only perform **one** context switch, because the switch
/// is the trap handler's and reaching it means unwinding this call. The
/// remaining dispatches of the pass are therefore performed by the re-executed
/// call — the same sequence of switches, with the loop state kept in
/// [`WAIT_PASSES`] instead of on the stack.
///
/// Then EmuTOS's trailing `wm_update(BEG_UPDATE); wm_update(END_UPDATE);`
/// (`:564-565`, "then get in the wait line"): block behind whoever holds the
/// window-tree lock, and let it go at once
/// ([`window::update_wait_line`](super::window::update_wait_line), GH1). A
/// caller parked there is re-executed from the top like any other; it is
/// marked in [`IN_WAIT_LINE`] so the re-executed pass goes straight back to the
/// wait line instead of dispatching the accessories again.
fn all_run() -> Option<sched::Switch> {
    let who = super::appl::current_identity();
    let in_line = IN_WAIT_LINE.get(who).is_some_and(|f| f.load(Ordering::Acquire));
    if !in_line {
        for _ in 0..count() {
            if let sched::Dispatch::Yield(sw) = sched::dsptch() {
                return Some(sw);
            }
        }
        if let Some(f) = IN_WAIT_LINE.get(who) {
            f.store(true, Ordering::Release);
        }
    }
    if let Some(sw) = super::window::update_wait_line() {
        return Some(sw);
    }
    if let Some(f) = IN_WAIT_LINE.get(who) {
        f.store(false, Ordering::Release);
    }
    None
}

/// AES identity `who` has done `all_run`'s dispatches and is in its
/// `wm_update` wait line (`aes/geminit.c:564`) — kept across the S2 restart of
/// a caller parked there, for the reason [`WAIT_PASSES`] is. Cleared when the
/// wait line is passed and with [`forget_wait`].
static IN_WAIT_LINE: [AtomicBool; pd::NUM_PDS] = [const { AtomicBool::new(false) }; pd::NUM_PDS];

/// `wait_for_accs(bitmask)` (`aes/geminit.c:599-628`) — let every desk
/// accessory run until each has `bitmask` set in its `p_flags`, or
/// [`WAIT_TIMEOUT`] passes have gone by.
///
/// EmuTOS's body is `all_run(); walk the DAs' p_flags;` in a bounded loop, and
/// the decisive property — extracted in `docs/api/aes/accessories.md` §3a — is
/// that `all_run`'s `dsptch()` **keeps the caller on the ready list**
/// (`disp_act`, `aes/gemdisp.c:74-79,231`). The barrier is a round-robin yield,
/// not a wait on an event: nothing but the dispatcher is needed to satisfy it,
/// and it makes progress on a machine nobody is touching.
///
/// That is why this is [`sched::dsptch`] and not [`sched::sleep`]. Sleeping
/// here is EmuTOS's `WAITIN` branch, which `ap_exit` is not in; it took the
/// caller out of the run-set and left it for `idle_wake_pass` to revive on a
/// key or a pointer movement, so headless — no key, no pointer — a program
/// that called `appl_exit` with an accessory loaded never reached its `Pterm`
/// (ACC10, measured in `measurements/measurement-2026-09-26-acc10/`).
///
/// Returns `Some(switch)` when the caller must abort its syscall and yield —
/// the wait resumes from the top when the caller's `svc` re-executes — and
/// `None` when the barrier is satisfied (or timed out, as EmuTOS's is).
///
/// **Restart-safety.** The function commits nothing but the pass counter: it
/// only reads `p_flags`.
#[must_use]
pub fn wait_for_accs(bitmask: u16) -> Option<sched::Switch> {
    let who = super::appl::current_identity();
    // `for (n = 0; n < WAIT_TIMEOUT; n++)` (`:609`), with `n` in `WAIT_PASSES`
    // because a pass can leave this function (see there).
    while spend_pass(who) <= WAIT_TIMEOUT {
        // `all_run();` (`:612`)
        if let Some(sw) = all_run() {
            return Some(sw);
        }
        // The `fpdnm(NULL, pid)` walk over the DA descriptors (`:617-625`):
        // no more DAs means they all have the bit.
        if all_have(bitmask) {
            forget_wait(who);
            return None;
        }
    }
    // `KDEBUG(("wait_for_accs(): %8.8s took too long\n", pd->p_name))` (`:627`)
    // — EmuTOS gives up and carries on, and so do we.
    forget_wait(who);
    None
}

/// Charge one pass to AES identity `who`'s [`WAIT_PASSES`] budget and report
/// the new total (1 on the first pass of a wait).
fn spend_pass(who: usize) -> u32 {
    match WAIT_PASSES.get(who) {
        Some(c) => c.fetch_add(1, Ordering::AcqRel) + 1,
        // No identity to charge — a caller outside the descriptor table cannot
        // exist (`current_identity` returns a slot or `pd::KERNEL_CONTEXT`), but an
        // out-of-range index must not silently wait forever either.
        None => WAIT_TIMEOUT + 1,
    }
}

/// Drop AES identity `who`'s in-flight [`wait_for_accs`] budget, so its next
/// wait starts with the full [`WAIT_TIMEOUT`].
///
/// Called on both exits of [`wait_for_accs`] and from [`super::release_client`]
/// — a process killed between a yield and its resume would otherwise leave a
/// spent budget behind for the slot's next tenant, which would then skip the
/// `AC_CLOSE` handshake instead of performing it.
pub(super) fn forget_wait(who: usize) {
    if let Some(c) = WAIT_PASSES.get(who) {
        c.store(0, Ordering::Release);
    }
    if let Some(f) = IN_WAIT_LINE.get(who) {
        f.store(false, Ordering::Release);
    }
}
