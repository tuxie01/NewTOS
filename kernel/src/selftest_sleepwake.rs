//! Cooperative-kernel step 3a — sleep/wake substrate self-test.
//!
//! Spec: `docs/analysis/coop-kernel/20-sleepwake-substrate-design.md`
//! (§3 states + `wait_cond`, §4.2 `sleep`/`wake` primitives, §7.3 the
//! wake-on-input test with one context).
//!
//! Step 3a lands only the substrate fields + primitives with the **idle-
//! fallback** path (no real peer switch — that is step 3b). The live no-
//! regression gate for it is the unchanged desktop / launched app (the
//! `event.rs` refactor drives the AES event loop). This gated test adds the
//! deterministic half the live boot cannot show: that a WAITING slot moves to
//! READY when — and only when — [`sched::wake`] is called with the *matching*
//! wake-class key (the "a process sleeps on `WAIT_INPUT`, a simulated input
//! `wake` releases it" case, §7.3), and stays WAITING on a mismatching key.
//!
//! It is gated behind `pub mod` and is **not** wired into normal boot (mirrors
//! [`crate::selftest_sysvars`] / [`crate::selftest_upcall`]). Invoke manually
//! from `kernel_main` (after the host is installed) when validating step 3a.

use core::sync::atomic::Ordering;

use tos_api::bdos::process::sched;
use tos_api::bdos::process::{allocate_slot, free_slot, PROCESS_TABLE};

use crate::kprintln;

/// Put slot `idx` into the WAITING(`cond`) state (design §3.1): record the wake
/// key and drop it out of the round-robin run-set — exactly what
/// [`sched::sleep`] does for a slot caller before the idle pass.
fn set_waiting(idx: usize, cond: u64) {
    PROCESS_TABLE[idx].wait_cond.store(cond, Ordering::Release);
    sched::unready(idx);
}

/// `true` when slot `idx` is WAITING (`runnable == false && wait_cond != 0`).
fn is_waiting(idx: usize) -> bool {
    !PROCESS_TABLE[idx].runnable.load(Ordering::Acquire)
        && PROCESS_TABLE[idx].wait_cond.load(Ordering::Acquire) != 0
}

/// `true` when slot `idx` is READY (`runnable == true && wait_cond == 0`).
fn is_ready(idx: usize) -> bool {
    PROCESS_TABLE[idx].runnable.load(Ordering::Acquire)
        && PROCESS_TABLE[idx].wait_cond.load(Ordering::Acquire) == 0
}

/// Run the step-3a sleep/wake substrate self-test (see the module docs).
/// Prints `[sleepwake] === PASS ===` on success. Gated; call manually after the
/// host is installed — not part of normal boot.
pub fn run() {
    kprintln!("");
    kprintln!("[sleepwake] === step-3a sleep/wake substrate test ===");

    let Some(idx) = allocate_slot() else {
        kprintln!("[sleepwake] === FAIL: no free process slot ===");
        return;
    };
    kprintln!("[sleepwake] using slot {}", idx);

    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[sleepwake]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // (1) WAIT_INPUT: a slot sleeps on WAIT_INPUT (RUNNING → WAITING).
    set_waiting(idx, sched::wait_input());
    check(is_waiting(idx), "WAIT_INPUT slot is WAITING");

    // (2) A mismatching wake must NOT release it (wake matches by exact key).
    sched::wake(sched::wait_mesg(7));
    check(is_waiting(idx), "mismatching wake leaves it WAITING");

    // (3) The matching (simulated input) wake releases it (WAITING → READY).
    sched::wake(sched::wait_input());
    check(is_ready(idx), "matching WAIT_INPUT wake -> READY");

    // (4) WAIT_MESG(pid): the message-FIFO wake class, matched by pid.
    set_waiting(idx, sched::wait_mesg(3));
    check(is_waiting(idx), "WAIT_MESG(3) slot is WAITING");
    sched::wake(sched::wait_mesg(4)); // wrong pid
    check(is_waiting(idx), "wrong-pid wake leaves it WAITING");
    sched::wake(sched::wait_mesg(3)); // right pid
    check(is_ready(idx), "matching WAIT_MESG(3) wake -> READY");

    // (5) wake(0) is a no-op: 0 is "not waiting", never a valid wake key.
    set_waiting(idx, sched::wait_timer(0));
    sched::wake(0);
    check(is_waiting(idx), "wake(0) is a no-op");
    // Release it again so the slot is clean before free.
    sched::wake(sched::wait_timer(0));

    free_slot(idx);

    if fails == 0 {
        kprintln!("[sleepwake] === PASS ===");
    } else {
        kprintln!("[sleepwake] === FAIL: {} check(s) ===", fails);
    }
}
