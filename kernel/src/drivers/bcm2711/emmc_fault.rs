//! SD fault injection of the debug feature `sd-fault` (TASK/HW08), for test
//! kernels: a program arms a fault through `Ssystem(S_NEWTOS_SDFAULT)`, and the
//! next write commands in the armed LBA range take the driver's real failure
//! paths — `note_failure`, `recover` with its `CMD13`, `CMD12` and software
//! resets, the `BlockError` that follows — so everything above the driver sees
//! what a real failure gives. Reads are only hit by [`Mode::Wedge`] and
//! [`Mode::ReadErr`].
//!
//! What the injection cannot be: it does not make a card busy or a controller
//! stuck, it makes the driver see that. Whether the recovery helps on EMMC2 is
//! measured by running it there ([`Mode::Dcrc`], [`Mode::TimeoutRdy`], M-C of
//! TASK/HW03), not by these modes in QEMU.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// The modes of `Ssystem(S_NEWTOS_SDFAULT)`, first word of its argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    /// 0: disarm.
    Off,
    /// 1: the card refuses `CMD24`/`CMD25`: no command is sent, the R1 path
    /// sees `WP_VIOLATION` (`BlockError::Card`, `EWRITF`).
    R1,
    /// 2: the command goes out, then the data phase ends on `Data CRC Error`
    /// (`BlockError::Io`, `EWRITF`, CMD12 + both resets, no data written).
    Dcrc,
    /// 3: the command goes out, Buffer Write Ready never comes: the full
    /// `WRITE_BUSY_MS` wait, then the timeout path (`BlockError::Timeout`,
    /// `EDRVNR`, CMD12 + both resets, no data written).
    TimeoutRdy,
    /// 4: the data goes out, Transfer Complete is withheld for `ms`: below
    /// `WRITE_BUSY_MS` a slow success, from there on the timeout path; the
    /// data has then reached the card even though the write failed, and for
    /// the rest of the `ms` `STATUS` keeps reporting Command Inhibit (DAT), as
    /// a card still holding DAT0 low would make it.
    Busy,
    /// 5: `STATUS` reports Command Inhibit (DAT) until disarmed, for reads and
    /// writes: a controller the recovery does not bring back. `skip`, `count`
    /// and the LBA range do not apply.
    Wedge,
    /// 6: the command really goes out with an address behind the end of the
    /// card: what the card answers (M-A of TASK/HW03).
    OutOfRange,
    /// 8: the data goes out, Transfer Complete does not come, and the card
    /// goes on programming for `ms` from there: `CMD13` answers `prg`, a data
    /// command gets no answer. What a real card does when it holds the busy
    /// past `WRITE_BUSY_MS`, which QEMU's card never does.
    Prg,
    /// 9: reads in the LBA range end on a data error (`BlockError::Io`,
    /// `EREADF`). No command goes out.
    ReadErr,
}

impl Mode {
    const fn from_word(w: u32) -> Mode {
        match w {
            1 => Mode::R1,
            2 => Mode::Dcrc,
            3 => Mode::TimeoutRdy,
            4 => Mode::Busy,
            5 => Mode::Wedge,
            6 => Mode::OutOfRange,
            8 => Mode::Prg,
            9 => Mode::ReadErr,
            _ => Mode::Off,
        }
    }
}

static MODE: AtomicU32 = AtomicU32::new(0);
static SKIP: AtomicU32 = AtomicU32::new(0);
/// Faults still to fire; `u32::MAX` = until disarmed.
static LEFT: AtomicU32 = AtomicU32::new(0);
static BUSY_MS: AtomicU32 = AtomicU32::new(0);
static LBA_LO: AtomicU32 = AtomicU32::new(0);
static LBA_HI: AtomicU32 = AtomicU32::new(0);
static FIRED: AtomicU32 = AtomicU32::new(0);
static BUSY_UNTIL: AtomicU64 = AtomicU64::new(0);
static PRG_UNTIL: AtomicU64 = AtomicU64::new(0);

/// `Ssystem(S_NEWTOS_SDFAULT)`: `[mode, skip, count, ms, lba_lo, lba_hi]`:
/// after `skip` matching write commands, the next `count` (0 = until disarmed)
/// take the fault; a command matches if it overlaps `lba_lo..lba_hi` (`lba_hi`
/// 0 = no upper end). Mode 0xFFFF only answers. Returns the faults fired since
/// the last arming.
pub fn arm(a: &[u32; 6]) -> u32 {
    let fired = FIRED.load(Ordering::Relaxed);
    if a[0] == 0xFFFF {
        return fired;
    }
    SKIP.store(a[1], Ordering::Relaxed);
    LEFT.store(if a[2] == 0 { u32::MAX } else { a[2] }, Ordering::Relaxed);
    BUSY_MS.store(a[3], Ordering::Relaxed);
    LBA_LO.store(a[4], Ordering::Relaxed);
    LBA_HI.store(a[5], Ordering::Relaxed);
    FIRED.store(0, Ordering::Relaxed);
    MODE.store(a[0], Ordering::Relaxed);
    crate::kprintln!(
        "[sdfault] armed mode {} skip {} count {} ms {} lba {}..{}",
        a[0],
        a[1],
        a[2],
        a[3],
        a[4],
        a[5],
    );
    fired
}

/// The bits `STATUS` reports on top of what the controller says, at counter
/// value `now`.
pub fn forced_status(dat_inhibit: u32, now: u64) -> u32 {
    let wedged = Mode::from_word(MODE.load(Ordering::Relaxed)) == Mode::Wedge;
    if wedged || now < BUSY_UNTIL.load(Ordering::Relaxed) {
        dat_inhibit
    } else {
        0
    }
}

/// The card stays busy until counter value `until` ([`Mode::Busy`]).
pub fn busy_until(until: u64) {
    BUSY_UNTIL.store(until, Ordering::Relaxed);
}

/// The card programs until counter value `until` ([`Mode::Prg`]).
pub fn prg_until(until: u64) {
    PRG_UNTIL.store(until, Ordering::Relaxed);
}

/// `prg` while [`Mode::Prg`] holds the card, at counter value `now`: what a
/// `CMD13` is answered, and that a data command is not.
pub fn forced_state(now: u64) -> Option<u32> {
    (now < PRG_UNTIL.load(Ordering::Relaxed)).then_some(super::emmc_xfer::CARD_PRG)
}

/// Does the read of `blocks` sectors at `lba` take [`Mode::ReadErr`]?
pub fn take_read(lba: u64, blocks: usize) -> bool {
    Mode::from_word(MODE.load(Ordering::Relaxed)) == Mode::ReadErr && take_in_range(lba, blocks)
}

/// The fault for the write command of `blocks` sectors at `lba`:
/// [`Mode::Off`] for none. The wait of [`Mode::Busy`] is [`busy_ms`].
pub fn take_write(lba: u64, blocks: usize) -> Mode {
    let mode = Mode::from_word(MODE.load(Ordering::Relaxed));
    if matches!(mode, Mode::Off | Mode::Wedge | Mode::ReadErr) || !take_in_range(lba, blocks) {
        return Mode::Off;
    }
    crate::kprintln!("[sdfault] fires {:?} at lba {} n {}", mode, lba, blocks);
    mode
}

/// Does a command at `lba` take the armed fault? Counts `skip` and `count`.
fn take_in_range(lba: u64, blocks: usize) -> bool {
    let lo = u64::from(LBA_LO.load(Ordering::Relaxed));
    let hi = match LBA_HI.load(Ordering::Relaxed) {
        0 => u64::MAX,
        h => u64::from(h),
    };
    if lba + blocks as u64 <= lo || lba >= hi {
        return false;
    }
    let skip = SKIP.load(Ordering::Relaxed);
    if skip > 0 {
        SKIP.store(skip - 1, Ordering::Relaxed);
        return false;
    }
    let left = LEFT.load(Ordering::Relaxed);
    if left == 0 {
        return false;
    }
    if left != u32::MAX {
        LEFT.store(left - 1, Ordering::Relaxed);
    }
    FIRED.fetch_add(1, Ordering::Relaxed);
    true
}

/// How long [`Mode::Busy`] withholds Transfer Complete.
pub fn busy_ms() -> u32 {
    BUSY_MS.load(Ordering::Relaxed)
}
