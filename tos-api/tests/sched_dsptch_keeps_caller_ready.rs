//! ACC10 — **the accessory barrier yields, it does not sleep.**
//!
//! EmuTOS's `wait_for_accs` spends each of its (bounded) passes in `all_run()`
//! → `dsptch()` (`aes/geminit.c:559-562,612`), and `dsptch` hands the CPU on
//! through `disp`, which branches on the *caller's own* state:
//!
//! ```text
//! p = rlr;  rlr = p->p_link;                 /* aes/gemdisp.c:225-226 */
//! if (p->p_stat & WAITIN)  mwait_act(p);     /* :230 — park it on nrl */
//! else                     disp_act(p);      /* :231 */
//! ```
//!
//! and `disp_act` is `insert_process(p, &rlr)` (`:74-79`) — straight back onto
//! the ready list. `ap_exit` is in the `else` branch (`aes/gemaplib.c:251-255`:
//! no `ev_block` between entry and the call), so the barrier is a round-robin
//! **yield**: it needs nothing but the dispatcher and makes progress on a
//! machine nobody is touching. Extracted in `docs/api/aes/accessories.md` §3a.
//!
//! NewTOS had it in the other branch. `wait_for_accs` used
//! `sched::sleep(wait_input())`, which is the `WAITIN` case: the caller left
//! the run-set and only `idle_wake_pass` — a key or a pointer movement — could
//! put it back. Headless there is neither, so a program that called
//! `appl_exit()` with a desk accessory loaded never came back from it and never
//! reached its `Pterm` (`measurements/measurement-2026-09-26-acc10/serial/before-*.log`;
//! the same stall for a kernel-launched program, i.e. with no `Pexec` in the
//! picture, in `measurements/measurement-2026-09-25-port05`).
//!
//! What is asserted is the one property that distinguishes the two primitives:
//! after a [`sched::dsptch`] the caller is still on the ready list, after
//! a [`sched::sleep`] it is not. The process table is global and `cargo test`
//! runs a file's tests in parallel threads, so the test serialises on the
//! binary's `LOCK` — the convention of the `aes_*` files that share
//! `PROCESS_TABLE` (T12).

use core::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard};

use tos_api::bdos::process::{sched, set_current, State, PROCESS_TABLE};
use tos_types::BasePage;

/// The process table, `RUN` and the scheduler's `KERNEL_PARKED` are
/// process-wide statics with no locking (single-CPU trap path); `cargo test`
/// runs a binary's tests in parallel threads, so every test in this binary
/// takes the lock first (T12). One lock over every process-wide static the
/// tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The context performing the wait — an EL0 application inside `appl_exit`.
const WAITER: usize = 4;
/// A desk accessory, woken by the `AC_CLOSE` that `mn_cleanup` just posted and
/// therefore the peer the barrier's `dsptch` has to find.
const ACC: usize = 5;

/// Make `slot` the process `running_slot()` reports: `slot_for` matches the
/// BDOS running-basepage `RUN` against a non-`Free` slot's `basepage_kva`
/// (`bdos/process.rs::slot_for`). The pointer is never dereferenced by any of
/// the code under test — it is an identity, not storage.
fn make_running(slot: usize) {
    let fake = (0x1000_0000usize + slot * 0x100) as *mut BasePage;
    PROCESS_TABLE[slot].set_state(State::Running);
    PROCESS_TABLE[slot].basepage_kva.store(fake, Ordering::Release);
    set_current(fake);
}

/// The state the measured boot was in the instant `appl_exit` reached the
/// barrier: the waiter running, one accessory awake with its `AC_CLOSE`
/// pending, everybody else out of the picture.
fn fixture() {
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        sched::unready(i);
        p.has_ctx.store(false, Ordering::Release);
        p.wait_cond.store(0, Ordering::Release);
        p.set_state(State::Free);
        p.basepage_kva
            .store(core::ptr::null_mut(), Ordering::Release);
    }
    sched::set_kernel_parked(false);
    // The waiter came out of an `evnt_*` wait, which leaves the slot READY
    // (`sleep`'s idle exit re-arms `runnable`), and is now on the CPU.
    sched::ready(WAITER);
    make_running(WAITER);
    // The accessory: woken by `pd::post_msg`, not yet scheduled.
    PROCESS_TABLE[ACC].set_state(State::Running);
    sched::admit(ACC);
}

#[test]
fn dsptch_yields_to_the_peer_without_leaving_the_run_set() {
    let _g = lock();
    fixture();

    // `all_run()`'s first `dsptch()`: the woken accessory is on `drl` and
    // joins the ready list behind the caller (A115, `gemdisp.c:231-243`), so
    // the caller keeps the CPU — and stays on the list …
    assert!(matches!(sched::dsptch(), sched::Dispatch::Idle), "the accessory joins behind the caller");
    assert!(PROCESS_TABLE[WAITER].runnable.load(Ordering::Acquire));
    // … and the next `dsptch()` gives the accessory the CPU.
    let sw = match sched::dsptch() {
        sched::Dispatch::Yield(sw) => sw,
        sched::Dispatch::Idle => panic!("a runnable peer exists — must dispatch"),
    };
    assert_eq!(sw.slot, ACC, "the barrier dispatches to the accessory");

    // The property the whole ticket turns on — `disp_act`, not `mwait_act`.
    assert!(
        PROCESS_TABLE[WAITER].runnable.load(Ordering::Acquire),
        "dsptch must leave the caller on the ready list: it is rescheduled by \
         the round-robin, not by an event"
    );
    assert_eq!(
        PROCESS_TABLE[WAITER].wait_cond.load(Ordering::Acquire),
        0,
        "dsptch must not arm a wake key — there is no event to wait for"
    );

    // Therefore the round-robin finds the waiter again the moment the
    // accessory blocks. This is the step that used to be missing: with the
    // waiter out of the run-set, the scheduler skipped it and the CPU went to
    // the idle pass forever.
    sched::unready(ACC);
    PROCESS_TABLE[ACC]
        .wait_cond
        .store(sched::wait_mesg(ACC), Ordering::Release);
    let back = sched::on_yield(ACC).expect("the waiter is still a peer");
    assert_eq!(
        back.slot, WAITER,
        "the blocked accessory hands the CPU back to the waiting application"
    );

    // Contrast, on the same fixture: `sleep` is EmuTOS's `WAITIN` branch. It
    // is the right primitive for a wait a wake class covers and the wrong one
    // for a poll over the DAs' `p_flags`, which none does.
    fixture();
    match sched::sleep(sched::wait_input()) {
        sched::SleepOutcome::Yield(sw) => assert_eq!(sw.slot, ACC),
        sched::SleepOutcome::Recheck => panic!("a runnable peer exists"),
    }
    assert!(
        !PROCESS_TABLE[WAITER].runnable.load(Ordering::Acquire),
        "fixture check: sleep DOES take the caller out of the run-set — that \
         is the difference, and with WAIT_INPUT nothing headless puts it back"
    );

    // With no peer at all, `dsptch` is the idle pass and the caller carries
    // on — EmuTOS's process put straight back on a ready list it is alone on.
    fixture();
    sched::unready(ACC);
    assert!(
        matches!(sched::dsptch(), sched::Dispatch::Idle),
        "nothing runnable: the pass is the idle pass, not a switch"
    );
    assert!(
        PROCESS_TABLE[WAITER].runnable.load(Ordering::Acquire),
        "and the caller is still READY afterwards"
    );

    set_current(core::ptr::null_mut());
}
