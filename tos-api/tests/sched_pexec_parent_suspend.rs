//! PORT05 — **a parent suspended inside `Pexec` is not a scheduler peer.**
//!
//! GEMDOS `Pexec` is a synchronous BDOS call: EmuTOS's `proc_go` sets
//! `run = p` and enters the child through `gouser()`
//! (`references/emutos/bdos/proc.c:536-577`); the parent's registers stay
//! frozen in its PD until the child's `xterm` does `run = run->p_parent` and
//! `termuser()` (`:601-618`). Nothing schedules the parent in between — in
//! EmuTOS nothing *can*, because the launching application and the launched
//! one share one AESPD, so the AES round-robin has no second entry to pick
//! (`docs/analysis/coop-kernel/emutos-cooperative-model.md` §5.2).
//!
//! NewTOS gives parent and child separate slots and runs a cooperative
//! round-robin over them, so the rule has to be stated explicitly. It was not:
//! an EL0 program that has been through one blocking `evnt_*` wait is left
//! `runnable` by [`sched::sleep`]'s idle exit, so the moment its `Pexec` child
//! blocked, `on_yield(child)` picked **the parent** — with `has_ctx == false`
//! that is a `fresh` switch, i.e. the suspended parent was re-entered at its
//! entry point and launched the child all over again
//! (`measurements/measurement-2026-09-25-port05/serial/before-*.log`).
//!
//! What is asserted here is the scheduler consequence, not the bookkeeping:
//! with the parent suspended, the blocking child's `on_yield` must reach the
//! *other* peer (an accessory) and never the parent — and after the child's
//! `Pterm` the parent must be back in the run-set exactly as it was.

use core::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard};

use tos_api::bdos::process::{sched, PROCESS_TABLE};

/// The process table is a process-wide static with no locking (single-CPU
/// trap path), and `fixture()` rewrites every slot of it; `cargo test` runs
/// this binary's tests in parallel threads, so they must not overlap (T12).
/// One lock over every process-wide static the tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Slot layout of the fixture — the arrangement the measured run had
/// (accessories 0/1, parent 2, child 3). Since A115 the pick follows the
/// ready list's order, not the slot index.
const ACC: usize = 1;
const PARENT: usize = 2;
const CHILD: usize = 3;

/// Put the slots into the state the measured boot had at the instant
/// `handle_pexec` had launched the child: the desk accessory blocked in its
/// own `evnt_multi` (WAITING, i.e. `runnable == false && wait_cond != 0`), the
/// parent left `runnable` by its last `evnt_*` wait, the child running.
fn fixture() {
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        sched::unready(i);
        p.has_ctx.store(false, Ordering::Release);
        p.wait_cond.store(0, Ordering::Release);
        p.parent_slot
            .store(tos_api::bdos::process::NO_PARENT, Ordering::Release);
        p.parent_runnable.store(false, Ordering::Release);
    }
    // A desk accessory, admitted by the loader and then blocked in its message
    // wait: WAITING, so it is not on the ready list (§3.2).
    sched::admit(ACC);
    sched::unready(ACC);
    PROCESS_TABLE[ACC]
        .wait_cond
        .store(sched::wait_input(), Ordering::Release);
    // The parent: it came out of a blocking AES wait, which leaves the slot
    // READY (`sleep`'s Ausgang (a) re-arms `runnable`).
    sched::ready(PARENT);
    // `handle_pexec`'s parent linkage (EmuTOS `proc_go`, `p->p_parent = run`).
    PROCESS_TABLE[CHILD]
        .parent_slot
        .store(PARENT as u32, Ordering::Release);
}

#[test]
fn blocking_child_never_yields_to_its_own_parent() {
    let _g = lock();
    fixture();

    // Without the suspend this is the measured defect: the child blocks and
    // the round-robin hands the CPU back to the parent that is sitting inside
    // `Pexec` — `fresh`, because the parent never yielded, i.e. re-entered at
    // its entry point.
    let before = sched::on_yield(CHILD).expect("fixture: a peer must exist");
    assert_eq!(
        before.slot, PARENT,
        "fixture check: without the suspend the parent IS what on_yield picks"
    );
    assert!(
        before.fresh,
        "fixture check: the parent never yielded, so the switch is a fresh \
         entry at its entry point — the destructive half of the defect"
    );

    sched::suspend_parent_for_child(CHILD);

    assert!(
        !PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire),
        "a parent inside Pexec must not be in the cooperative run-set"
    );
    assert!(
        sched::on_yield(CHILD).is_none(),
        "with the parent suspended and the accessory asleep there is no peer: \
         the child falls into the idle pass, exactly like a program the kernel \
         launcher started"
    );
}

#[test]
fn an_awake_accessory_still_gets_the_cpu_while_the_child_blocks() {
    let _g = lock();
    fixture();
    // The other half of the rule: withdrawing the parent must not withdraw
    // anybody else. A woken accessory is still a peer for the blocked child.
    PROCESS_TABLE[ACC].wait_cond.store(0, Ordering::Release);
    sched::ready(ACC);

    sched::suspend_parent_for_child(CHILD);

    let sw = sched::on_yield(CHILD).expect("the accessory is runnable");
    assert_eq!(
        sw.slot, ACC,
        "the blocking child yields to the accessory, never to its parent"
    );
}

#[test]
fn a_stale_wake_key_cannot_re_arm_the_suspended_parent() {
    let _g = lock();
    fixture();
    // Belt and braces: `wake` re-arms every slot whose `wait_cond` matches, so
    // a suspended parent that still carried a wake key would be put back on
    // the ready list behind the scheduler's back.
    PROCESS_TABLE[PARENT]
        .wait_cond
        .store(sched::wait_input(), Ordering::Release);

    sched::suspend_parent_for_child(CHILD);
    sched::wake(sched::wait_input());

    assert!(
        !PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire),
        "wake() must not re-arm a parent that is suspended inside Pexec"
    );
}

#[test]
fn the_childs_pterm_puts_the_parent_back_exactly_as_it_was() {
    let _g = lock();
    fixture();
    sched::suspend_parent_for_child(CHILD);
    sched::resume_parent_of_child(CHILD);

    assert!(
        PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire),
        "after the child's Pterm the parent is a peer again (it was one)"
    );
    let sw = sched::on_yield(CHILD).expect("a peer exists again");
    assert_eq!(sw.slot, PARENT, "the round-robin sees the parent again");
}

#[test]
fn a_parent_that_was_never_a_peer_does_not_become_one() {
    let _g = lock();
    fixture();
    // The kernel launcher's programs start outside the run-set (`launch.rs`
    // does not `admit`), and a program that has not yet blocked in an `evnt_*`
    // wait is still outside it. Launching a child must not change that.
    sched::unready(PARENT);

    sched::suspend_parent_for_child(CHILD);
    sched::resume_parent_of_child(CHILD);

    assert!(
        !PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire),
        "the saved bit is restored, not a forced `true`"
    );
}

#[test]
fn a_child_without_a_parent_slot_is_left_alone() {
    let _g = lock();
    fixture();
    // Desk accessories and the kernel launcher's programs have no parent slot
    // (their "parent" is an EL1 context that owns no slot). Both calls must be
    // no-ops rather than touching slot `NO_PARENT`.
    PROCESS_TABLE[CHILD]
        .parent_slot
        .store(tos_api::bdos::process::NO_PARENT, Ordering::Release);

    sched::suspend_parent_for_child(CHILD);
    assert!(
        PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire),
        "an unrelated slot must not be withdrawn from the run-set"
    );
    sched::resume_parent_of_child(CHILD);
    assert!(PROCESS_TABLE[PARENT].runnable.load(Ordering::Acquire));
}
