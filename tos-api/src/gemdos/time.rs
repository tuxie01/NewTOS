//! GEMDOS time / date group (PHASE-07 D1 + D2).
//!
//! Specs: `docs/api/gemdos/time/{2a,2b,2c,2d}-*.md`.
//!
//! NewTOS has no battery RTC; the wall clock is a settable **base
//! epoch** (seconds since the TOS epoch 1980-01-01 00:00:00) plus the
//! monotonic seconds the host reports from the ARM generic timer
//! (`CNTPCT_EL0 / CNTFRQ_EL0`). `Tsetdate` / `Tsettime` recompute the
//! base so the clock reads the requested value; `Tgetdate` /
//! `Tgettime` add the monotonic delta back and pack to the FAT-style
//! WORD encodings. XBIOS `Settime` / `Gettime` use the same clock.
//!
//! At boot [`time_init`] starts the clock at [`DEFAULT_DATETIME`], the
//! kernel's build date at 00:00:00 — EmuTOS's clock value when no RTC
//! answers (`bios/ikbd.h:95`, `bios/clock.c:1211`, `bios/clock.c:1319`).
//!
//! Calendar math uses EmuTOS's simplified leap rule (`bdos/time.c`):
//! every 4th year is a leap year — the 100/400 Gregorian exceptions
//! are dropped, which is exact over the representable 1980..2099 span
//! (2000 is a leap year and no century boundary falls inside).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::host::host;
use crate::trap::TrapArgs;
use tos_types::error::{ERROR, E_OK};
use tos_types::Long;

/// Wall-clock seconds since 1980-01-01 00:00:00 corresponding to
/// monotonic boot time 0. `now = BASE_SECS + monotonic`. `0` until
/// [`time_init`] seeds it at boot (EmuTOS's `current_date`/`current_time`
/// are zero until its `time_init` too); programs set it via `Tsetdate` /
/// `Tsettime` / `Settime`.
static BASE_SECS: AtomicU64 = AtomicU64::new(0);

const SECS_PER_DAY: u64 = 86_400;

/// Days per month, 1-indexed via `month - 1` (February handled by
/// [`days_in_month`] for leap years).
const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

#[inline]
fn monotonic() -> u64 {
    host().map(|h| h.monotonic_seconds()).unwrap_or(0)
}

#[inline]
fn now_secs() -> u64 {
    BASE_SECS.load(Ordering::Acquire).wrapping_add(monotonic())
}

/// EmuTOS leap rule: every 4th year (valid for 1980..2099).
#[inline]
const fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4)
}

#[inline]
fn days_in_month(year: u32, month: u32) -> u32 {
    if month == 2 && is_leap(year) {
        29
    } else {
        MONTH_DAYS[(month - 1) as usize]
    }
}

/// Broken-down calendar time.
struct Dt {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
}

/// Seconds-since-1980 → broken-down calendar time.
fn decompose(secs: u64) -> Dt {
    let tod = (secs % SECS_PER_DAY) as u32;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;

    let mut days = (secs / SECS_PER_DAY) as u32;
    let mut year = 1980u32;
    loop {
        let yd = if is_leap(year) { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let mut month = 1u32;
    loop {
        let md = days_in_month(year, month);
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    Dt { year, month, day: days + 1, hour, min, sec }
}

/// Broken-down calendar time → seconds-since-1980.
fn compose(year: u32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> u64 {
    let mut days: u64 = 0;
    for y in 1980..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_month(year, m) as u64;
    }
    days += (day - 1) as u64;
    days * SECS_PER_DAY + (hour as u64) * 3600 + (min as u64) * 60 + sec as u64
}

/// Pack to the GEMDOS date WORD: `[15..9]=year-1980, [8..5]=month,
/// [4..0]=day`.
#[inline]
fn pack_date(dt: &Dt) -> u16 {
    (((dt.year - 1980) as u16) << 9) | ((dt.month as u16) << 5) | (dt.day as u16)
}

/// Pack to the GEMDOS time WORD: `[15..11]=hour, [10..5]=minute,
/// [4..0]=second/2`.
#[inline]
fn pack_time(dt: &Dt) -> u16 {
    ((dt.hour as u16) << 11) | ((dt.min as u16) << 5) | ((dt.sec as u16) / 2)
}

/// Current wall-clock time/date as GEMDOS-packed WORDs `(time, date)`.
/// Used by the file system to time-stamp new directory entries.
pub fn dos_now() -> (u16, u16) {
    let dt = decompose(now_secs());
    (pack_time(&dt), pack_date(&dt))
}

/// `Tgetdate` (0x2A) — packed current date (zero-extended LONG).
pub fn tgetdate(_: &TrapArgs) -> Long {
    pack_date(&decompose(now_secs())) as Long
}

/// `Tgettime` (0x2C) — packed current time (zero-extended LONG).
pub fn tgettime(_: &TrapArgs) -> Long {
    pack_time(&decompose(now_secs())) as Long
}

/// `Tsetdate(d)` (0x2B) — set the date part of the wall clock. Returns
/// `E_OK`, or `ERROR` for an out-of-range field (EmuTOS `xsetdate`:
/// year offset > 119, month > 12, day > days-in-month). NewTOS also
/// rejects month/day `< 1` (avoids the EmuTOS day-0 quirk and the
/// underflow it would cause). See `docs/api/gemdos/time/2b-tsetdate.md`.
pub fn tsetdate(args: &TrapArgs) -> Long {
    let d = args.arg_u16(0);
    let year_off = (d >> 9) as u32;
    let month = ((d >> 5) & 0x0F) as u32;
    let day = (d & 0x1F) as u32;

    if year_off > 119 || !(1..=12).contains(&month) {
        return ERROR;
    }
    let year = 1980 + year_off;
    if day < 1 || day > days_in_month(year, month) {
        return ERROR;
    }

    let cur = decompose(now_secs());
    let target = compose(year, month, day, cur.hour, cur.min, cur.sec);
    BASE_SECS.store(target.wrapping_sub(monotonic()), Ordering::Release);
    E_OK
}

/// `Tsettime(t)` (0x2D) — set the time part of the wall clock. Returns
/// `E_OK`, or `ERROR` if any of the three fields is out of range: the
/// second/2 field ≥ 30, the minute field ≥ 60, **or the hour field ≥ 24**.
/// All three are validated in EmuTOS `xsettime` (`bdos/time.c:144-146`,
/// including `(t & HRS_BM) >= (24 << HRS_SHIFT)`; D-074). See
/// `docs/api/gemdos/time/2d-tsettime.md`.
pub fn tsettime(args: &TrapArgs) -> Long {
    let t = args.arg_u16(0);
    let hour = (t >> 11) as u32; // bits 15..11
    let min = ((t >> 5) & 0x3F) as u32;
    let dsec = (t & 0x1F) as u32; // second / 2

    if dsec >= 30 || min >= 60 || hour >= 24 {
        return ERROR;
    }

    let cur = decompose(now_secs());
    let target = compose(cur.year, cur.month, cur.day, hour, min, dsec * 2);
    BASE_SECS.store(target.wrapping_sub(monotonic()), Ordering::Release);
    E_OK
}

/// `Settime` (XBIOS 0x16) — set the whole wall clock from a packed LONG
/// (`date << 16 | time`, the Tgetdate/Tgettime encoding). See
/// `docs/api/xbios/16-settime.md`.
///
/// On the NewTOS profile no RTC backend is enabled, so — like EmuTOS's
/// last-resort path — this only updates the cached wall clock (the same
/// `BASE_SECS` epoch GEMDOS `Tgetdate`/`Tgettime` read); nothing persists
/// across boots. Settime itself does no validation (spec edge case 1);
/// NewTOS's seconds-based clock cannot represent an impossible calendar
/// date the way EmuTOS's raw WORD cache can, so an out-of-range month/day
/// is clamped to the nearest valid date — a documented deviation. The
/// unvalidated hour/minute/second fields pass through unchanged (they
/// only roll the clock forward, never panic).
pub fn xbios_settime(packed: u32) {
    let date_w = (packed >> 16) as u16;
    let time_w = packed as u16;
    let year = 1980 + (date_w >> 9) as u32;
    let month = (((date_w >> 5) & 0x0F) as u32).clamp(1, 12);
    let day = ((date_w & 0x1F) as u32).clamp(1, days_in_month(year, month));
    let hour = (time_w >> 11) as u32; // hour/min/sec not validated (EmuTOS parity)
    let min = ((time_w >> 5) & 0x3F) as u32;
    let sec = ((time_w & 0x1F) as u32) * 2; // stored as second/2
    let target = compose(year, month, day, hour, min, sec);
    BASE_SECS.store(target.wrapping_sub(monotonic()), Ordering::Release);
}

/// `Gettime` (XBIOS 0x17) — the current wall clock as a packed LONG
/// (`date << 16 | time`). See `docs/api/xbios/17-gettime.md`. With no RTC
/// on the NewTOS profile this reads the same cached clock as GEMDOS
/// (EmuTOS's `MAKE_ULONG(current_date, current_time)` fallback path).
pub fn xbios_gettime() -> u32 {
    let dt = decompose(now_secs());
    ((pack_date(&dt) as u32) << 16) | pack_time(&dt) as u32
}

/// The clock value when no RTC answers: the OS build date at 00:00:00,
/// EmuTOS `DEFAULT_DATETIME = MAKE_ULONG(os_header.os_dosdate, 0)`
/// (`bios/ikbd.h:95`). The date WORD is [`crate::build::OS_DOSDATE`], the
/// value the loader writes into the OSHEADER's `os_dosdate` (+0x1E), so the
/// clock and the header carry one build date (`SOURCE_DATE_EPOCH` pins it).
pub const DEFAULT_DATETIME: u32 = (crate::build::OS_DOSDATE as u32) << 16;

/// Boot-time clock start — EmuTOS `time_init` (`bdos/time.c:160-165`), which
/// seeds GEMDOS's clock from `Gettime()`. With no RTC that is
/// [`DEFAULT_DATETIME`]: the IKBD clock is set to it when it powers up
/// unset (`bios/clock.c:1211`), and a build without one returns it directly
/// (`bios/clock.c:1319`). NewTOS has no RTC (TASK/RTC01 adds one), so the
/// clock reads the build date at 00:00:00 at this call and runs on from
/// there with the generic timer. Call once at boot, after the host is
/// installed (the monotonic count is taken here).
pub fn time_init() {
    xbios_settime(DEFAULT_DATETIME);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `DEFAULT_DATETIME` is the build date GEMDOS-packed in the high word
    /// and midnight (`0`) in the low word, like `MAKE_ULONG(os_dosdate, 0)`.
    #[test]
    fn default_datetime_is_the_build_date_at_midnight() {
        use crate::build::{BUILD_DAY, BUILD_MONTH, BUILD_YEAR};
        assert_eq!(DEFAULT_DATETIME & 0xFFFF, 0, "time word is 00:00:00");
        let date = (DEFAULT_DATETIME >> 16) as u16;
        assert_eq!(date, crate::build::OS_DOSDATE);
        assert_eq!(1980 + (date >> 9), BUILD_YEAR);
        assert_eq!((date >> 5) & 0x0F, BUILD_MONTH as u16);
        assert_eq!(date & 0x1F, BUILD_DAY as u16);
    }

    /// The packed value round-trips through the seconds clock: composing
    /// the build date at 00:00:00 and decomposing it gives the same WORDs.
    #[test]
    fn default_datetime_round_trips_through_the_clock_math() {
        let date = (DEFAULT_DATETIME >> 16) as u16;
        let secs = compose(
            1980 + (date >> 9) as u32,
            ((date >> 5) & 0x0F) as u32,
            (date & 0x1F) as u32,
            0,
            0,
            0,
        );
        let dt = decompose(secs);
        assert_eq!(pack_date(&dt), date);
        assert_eq!(pack_time(&dt), 0);
    }
}
