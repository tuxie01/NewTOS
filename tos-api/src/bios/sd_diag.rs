//! The SD diagnosis of the debug feature `sd-diag` (TASK/HW08), as data and as
//! text; no hardware access, so the host tests run it
//! (`tests/hw08_sd_diag.rs`).
//!
//! The kernel fills a [`Report`] from the driver's counters
//! (`kernel/src/drivers/bcm2711/emmc.rs` `error_stats`,
//! `kernel/src/drivers/bcm2711/emmc_diag.rs`) and from
//! [`crate::bdos::fs::cache`], and shows [`write_report`]'s text on the screen,
//! on the serial console and through `Ssystem(S_NEWTOS_SDDIAG)`. Nothing here
//! or below it touches the card: the report has to answer exactly when the
//! card no longer takes writes.
//!
//! Lines are at most [`COLUMNS`] wide and at most [`LINES`] long, a field the
//! report has no value for is left out. Times: `up` = `h:mm:ss.mmm` since the
//! counter started at boot, the time base of the FAT stamps on a Pi without an
//! RTC. Waits are in milliseconds.

use core::fmt::{self, Write};

use tos_types::Long;

/// Width of a report line in characters.
pub const COLUMNS: usize = 80;
/// Most lines one report has.
pub const LINES: usize = 38;
/// Most bytes one report has, line ends included.
pub const TEXT_MAX: usize = (COLUMNS + 1) * LINES;
/// Failures the log keeps behind the first one.
pub const LOG_LEN: usize = 6;
/// Dirty sectors the report lists one by one.
pub const DIRTY_LISTED: usize = 8;

/// Upper bounds of the time classes in milliseconds. The bounds are the limits
/// the driver knows: 250 ms and 500 ms from SD Phys §4.6.2.2, 1000 ms =
/// `WRITE_BUSY_MS` (`docs/api/hw/emmc-audit.md`).
pub const HIST_BOUNDS_MS: [u64; 7] = [1, 4, 16, 64, 250, 500, 1000];
/// Number of time classes: one per bound and one for everything longer.
pub const HIST_LEN: usize = HIST_BOUNDS_MS.len() + 1;

/// The class of a duration in microseconds.
pub const fn hist_index(us: u64) -> usize {
    let mut i = 0;
    while i < HIST_BOUNDS_MS.len() {
        if us < HIST_BOUNDS_MS[i] * 1000 {
            return i;
        }
        i += 1;
    }
    HIST_LEN - 1
}

/// What kind of failure a log entry is.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FailKind {
    /// A wait ran out of time (`BlockError::Timeout`).
    #[default]
    Timeout,
    /// A wait ended on a controller error bit (`BlockError::Io`).
    Io,
    /// The card refused the command, R1 status (`BlockError::Card`).
    Card,
    /// Recovery left an inhibit bit set: SD-HC §3.10.1 "Non-recoverable Error".
    Nonrecoverable,
}

impl FailKind {
    const fn name(self) -> &'static str {
        match self {
            FailKind::Timeout => "timeout",
            FailKind::Io => "error",
            FailKind::Card => "refused",
            FailKind::Nonrecoverable => "NONRECOVERABLE",
        }
    }
}

/// One failure in the driver.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Failure {
    /// Microseconds since boot.
    pub at_us: u64,
    /// The operation it hit: write (else read).
    pub write: bool,
    /// First device LBA of the operation.
    pub lba: u64,
    /// Sectors of the operation.
    pub blocks: u32,
    /// Index into [`DriverReport::phase_names`].
    pub phase: u8,
    /// What happened.
    pub kind: FailKind,
    /// `INTERRUPT` for a wait, the R1 status for a refusal, `STATUS` for a
    /// non-recoverable recovery.
    pub status: u32,
}

/// Counters of one transfer direction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OpStats {
    /// Commands (one `CMD24`/`CMD25` is one).
    pub count: u64,
    /// Sectors.
    pub sectors: u64,
    /// Commands that ended in an error.
    pub failed: u64,
    /// Longest command, microseconds.
    pub max_us: u64,
    /// Commands per time class ([`HIST_BOUNDS_MS`]).
    pub hist: [u32; HIST_LEN],
}

/// The driver half of the report.
#[derive(Clone, Copy, Debug)]
pub struct DriverReport<'a> {
    /// `(emmc=compat, bus width)` while a card is up.
    pub mode: Option<(bool, u8)>,
    /// Waits that ended on an error bit, since boot.
    pub io_errors: u32,
    /// Waits that ran out of time, since boot.
    pub timeouts: u32,
    /// Recovery sequences.
    pub recoveries: u32,
    /// Recoveries that left an inhibit bit set.
    pub nonrecoverable: u32,
    /// Commands the card refused.
    pub card_errors: u32,
    /// `(io_errors, timeouts)` when the base probe of the boot ended; on a
    /// board whose first base has no card those are the probe working.
    pub at_init: (u32, u32),
    /// Phase of the most recent failed wait.
    pub last_phase: u8,
    /// `INTERRUPT` at the most recent failed wait.
    pub last_status: u32,
    /// R1 status of the most recent refusal.
    pub last_card_status: u32,
    /// Names of the phases.
    pub phase_names: &'a [&'a str],
    /// Names of the error bits.
    pub bit_names: &'a [&'a str],
    /// Counts of the error bits.
    pub bits: &'a [u32],
    /// Names of the R1 bits.
    pub r1_names: &'a [&'a str],
    /// Counts of the R1 bits.
    pub r1_bits: &'a [u32],
    /// Timeouts per phase.
    pub timeouts_by_phase: &'a [u32],
    /// Longest wait per phase, microseconds.
    pub wait_max_us: &'a [u64],
    /// Writes.
    pub write: OpStats,
    /// Reads.
    pub read: OpStats,
    /// The first failure after the boot.
    pub first: Option<Failure>,
    /// The latest failures, oldest first.
    pub log: &'a [Failure],
}

/// The whole report.
#[derive(Clone, Copy, Debug)]
pub struct Report<'a> {
    /// Microseconds since boot, now.
    pub now_us: u64,
    /// The driver half; `None` on a board without an SD driver.
    pub driver: Option<DriverReport<'a>>,
    /// `crate::bdos::fs::cache::writeback_stats`.
    pub writeback: (u32, u32, u32, u32),
    /// `crate::bdos::fs::cache::error_codes`.
    pub codes: (Long, u32),
    /// The dirty sectors, as many as were copied.
    pub dirty: &'a [crate::bdos::fs::cache::DirtySlot],
    /// Block cache counters, `None` without one.
    pub bcache: Option<crate::bios::bcache::Stats>,
    /// The bytes of the screen hot key, shown in the title; empty without one.
    pub hot_key: &'a str,
}

/// `h:mm:ss.mmm`.
struct Up(u64);

impl fmt::Display for Up {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = self.0 / 1000;
        let s = ms / 1000;
        write!(
            f,
            "{}:{:02}:{:02}.{:03}",
            s / 3600,
            s / 60 % 60,
            s % 60,
            ms % 1000
        )
    }
}

/// Milliseconds with one decimal.
struct Ms(u64);

impl fmt::Display for Ms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0 / 1000, self.0 / 100 % 10)
    }
}

/// Write `label  name=n name=n …` with the non-zero counts; nothing if all are
/// zero. Runs that would pass [`COLUMNS`] go on on a further line.
fn named_counts<W: Write>(w: &mut W, label: &str, names: &[&str], counts: &[u32]) -> fmt::Result {
    let mut col = 0;
    for (name, n) in names.iter().zip(counts).filter(|(_, n)| **n != 0) {
        let width = name.len() + 1 + decimal_width(u64::from(*n)) + 1;
        if col == 0 || col + width > COLUMNS {
            if col != 0 {
                writeln!(w)?;
            }
            write!(w, " {label:<6}")?;
            col = 7;
        }
        write!(w, " {name}={n}")?;
        col += width;
    }
    if col != 0 {
        writeln!(w)?;
    }
    Ok(())
}

fn decimal_width(mut n: u64) -> usize {
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

fn op_lines<W: Write>(w: &mut W, label: &str, op: &OpStats) -> fmt::Result {
    writeln!(
        w,
        "{label:<7} n {} sec {} failed {} max {} ms",
        op.count,
        op.sectors,
        op.failed,
        Ms(op.max_us),
    )?;
    write!(w, " time  ")?;
    for (bound, n) in HIST_BOUNDS_MS.iter().zip(&op.hist) {
        write!(w, " <{bound}:{n}")?;
    }
    writeln!(w, " more:{}", op.hist[HIST_LEN - 1])
}

fn failure_line<W: Write>(w: &mut W, label: &str, f: &Failure, phases: &[&str]) -> fmt::Result {
    writeln!(
        w,
        "{label:<7} {} {} lba {} n {} {} {} {:08x}",
        Up(f.at_us),
        if f.write { "write" } else { "read" },
        f.lba,
        f.blocks,
        phases.get(f.phase as usize).copied().unwrap_or("?"),
        f.kind.name(),
        f.status,
    )
}

fn driver_lines<W: Write>(w: &mut W, d: &DriverReport<'_>) -> fmt::Result {
    match d.mode {
        Some((compat, width)) => write!(
            w,
            "driver  {}-bit {}",
            width,
            if compat { "compat" } else { "multi" }
        )?,
        None => write!(w, "driver  no card")?,
    }
    writeln!(
        w,
        " errors {} timeouts {} recover {} nonrecover {} refused {}",
        d.io_errors, d.timeouts, d.recoveries, d.nonrecoverable, d.card_errors,
    )?;
    writeln!(
        w,
        " boot    errors {} timeouts {} (base probe, in the counts above)",
        d.at_init.0, d.at_init.1,
    )?;
    writeln!(
        w,
        " last    {} int {:08x} r1 {:08x}",
        d.phase_names
            .get(d.last_phase as usize)
            .copied()
            .unwrap_or("?"),
        d.last_status,
        d.last_card_status,
    )?;
    named_counts(w, "bits", d.bit_names, d.bits)?;
    named_counts(w, "r1", d.r1_names, d.r1_bits)?;
    named_counts(w, "tmo", d.phase_names, d.timeouts_by_phase)?;
    write!(w, " wait ms")?;
    let mut col = 8;
    for (name, us) in d
        .phase_names
        .iter()
        .zip(d.wait_max_us)
        .filter(|(_, us)| **us != 0)
    {
        let width = name.len() + 2 + decimal_width(*us / 1000) + 2;
        if col + width > COLUMNS {
            write!(w, "\n        ")?;
            col = 8;
        }
        write!(w, " {name} {}", Ms(*us))?;
        col += width;
    }
    writeln!(w)?;
    op_lines(w, "write", &d.write)?;
    op_lines(w, "read", &d.read)?;
    match &d.first {
        Some(f) => failure_line(w, "first", f, d.phase_names)?,
        None => writeln!(w, "first   no failure since the boot")?,
    }
    for f in d.log {
        failure_line(w, "fail", f, d.phase_names)?;
    }
    Ok(())
}

/// Write the report. At most [`LINES`] lines of at most [`COLUMNS`] characters
/// (host test `tests/hw08_sd_diag.rs`).
pub fn write_report<W: Write>(w: &mut W, r: &Report<'_>) -> fmt::Result {
    write!(w, "SD diagnosis  up {}", Up(r.now_us))?;
    if !r.hot_key.is_empty() {
        write!(w, "   {}: hide / show again", r.hot_key)?;
    }
    writeln!(w)?;
    match &r.driver {
        Some(d) => driver_lines(w, d)?,
        None => writeln!(w, "driver  none on this board")?,
    }
    let (failed, retried, gave_up, dirty) = r.writeback;
    writeln!(
        w,
        "cache   writeback failed {failed} retried {retried} gave up {gave_up} dirty {dirty}; last code {} of {}",
        r.codes.0, r.codes.1,
    )?;
    for s in r.dirty.iter().take(DIRTY_LISTED) {
        writeln!(
            w,
            " dirty  {}: lba {} {} tries {} code {}",
            char::from(b'A' + (s.drv as u8).min(25)),
            s.lba,
            if s.fat { "FAT" } else { "dir/data" },
            s.fails,
            s.err,
        )?;
    }
    if dirty as usize > DIRTY_LISTED {
        writeln!(w, " dirty  +{} more", dirty as usize - DIRTY_LISTED)?;
    }
    if let Some(b) = &r.bcache {
        writeln!(
            w,
            "bcache  dev r {} w {} hit {} miss {} sectors",
            b.dev_reads, b.dev_writes, b.hit_sectors, b.miss_sectors,
        )?;
    }
    Ok(())
}
