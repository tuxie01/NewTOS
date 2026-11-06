//! What the debug feature `sd-diag` (TASK/HW08) records inside the SD driver,
//! beyond the counters the driver always keeps (`emmc::error_stats`): the
//! failure log with the operation it hit, the longest wait per phase and the
//! write and read times. The report's text is
//! `tos_api::bios::sd_diag`, its screen form `crate::sd_diag`.
//!
//! The driver runs on the single-CPU, IRQ-off trap path, so plain relaxed
//! atomics and one uncontended lock are enough; nothing here allocates or
//! touches the card.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tos_api::bios::sd_diag::{FailKind, Failure, HIST_LEN, LOG_LEN, OpStats, hist_index};

use super::emmc::Phase;
use crate::sync::SpinMutex;

/// Counter ticks to microseconds at `frq` Hz (0 when the machine reports no
/// counter frequency).
pub fn ticks_to_us(ticks: u64, frq: u64) -> u64 {
    if frq == 0 {
        return 0;
    }
    (u128::from(ticks) * 1_000_000 / u128::from(frq)) as u64
}

static WAIT_MAX_TICKS: [AtomicU64; Phase::COUNT] = [const { AtomicU64::new(0) }; Phase::COUNT];

/// A wait in `phase` ended after `ticks` counter ticks.
pub fn wait_done(phase: Phase, ticks: u64) {
    WAIT_MAX_TICKS[phase.index()].fetch_max(ticks, Ordering::Relaxed);
}

/// The longest wait per phase so far, in counter ticks.
pub fn wait_max(out: &mut [u64; Phase::COUNT]) {
    for (o, m) in out.iter_mut().zip(&WAIT_MAX_TICKS) {
        *o = m.load(Ordering::Relaxed);
    }
}

/// The failure log: the first one for good, the latest [`LOG_LEN`] behind it.
struct Log {
    first: Option<Failure>,
    ring: [Failure; LOG_LEN],
    /// Failures after the first one so far.
    behind: usize,
}

static LOG: SpinMutex<Log> = SpinMutex::new(Log {
    first: None,
    ring: [Failure {
        at_us: 0,
        write: false,
        lba: 0,
        blocks: 0,
        phase: 0,
        kind: FailKind::Timeout,
        status: 0,
    }; LOG_LEN],
    behind: 0,
});

/// The operation the driver is on, for the failure log: `(write, lba, blocks)`.
static CURRENT: SpinMutex<(bool, u64, u32)> = SpinMutex::new((false, 0, 0));

/// Set by the first failure after the boot, once.
static FIRST_SEEN: AtomicBool = AtomicBool::new(false);

/// Take the "a failure happened and the report has not shown itself yet" flag.
pub fn take_first_failure() -> bool {
    FIRST_SEEN.swap(false, Ordering::Relaxed)
}

/// Log a failure of the current operation. Only called once the base probe of
/// the boot is over: a slot without a card answering `CTO` is no failure.
pub fn failure(now_us: u64, phase: Phase, kind: FailKind, status: u32) {
    let (write, lba, blocks) = *CURRENT.lock();
    let f = Failure {
        at_us: now_us,
        write,
        lba,
        blocks,
        phase: phase.index() as u8,
        kind,
        status,
    };
    let mut log = LOG.lock();
    if log.first.is_none() {
        log.first = Some(f);
        FIRST_SEEN.store(true, Ordering::Relaxed);
    } else {
        let at = log.behind % LOG_LEN;
        log.ring[at] = f;
        log.behind += 1;
    }
}

/// Copy the log: the first failure, then up to [`LOG_LEN`] latest, oldest
/// first, and their number.
pub fn log(out: &mut [Failure; LOG_LEN]) -> (Option<Failure>, usize) {
    let log = LOG.lock();
    let n = log.behind.min(LOG_LEN);
    for (i, o) in out.iter_mut().take(n).enumerate() {
        *o = log.ring[(log.behind - n + i) % LOG_LEN];
    }
    (log.first, n)
}

const ZERO_OP: OpStats = OpStats {
    count: 0,
    sectors: 0,
    failed: 0,
    max_us: 0,
    hist: [0; HIST_LEN],
};
static OPS: SpinMutex<[OpStats; 2]> = SpinMutex::new([ZERO_OP; 2]);

/// A read (`write == false`) or write command for `blocks` sectors at `lba`
/// starts.
pub fn op_begin(write: bool, lba: u64, blocks: usize) {
    *CURRENT.lock() = (write, lba, blocks as u32);
}

/// The command ended after `us` microseconds.
pub fn op_end(write: bool, us: u64, blocks: usize, ok: bool) {
    let mut ops = OPS.lock();
    let op = &mut ops[usize::from(write)];
    op.count += 1;
    op.sectors += blocks as u64;
    op.failed += u64::from(!ok);
    op.max_us = op.max_us.max(us);
    op.hist[hist_index(us)] += 1;
}

/// `(reads, writes)`.
pub fn ops() -> [OpStats; 2] {
    *OPS.lock()
}
