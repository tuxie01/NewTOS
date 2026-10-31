//! XBIOS12 — the boot clock without an RTC starts at the kernel's build date.
//!
//! EmuTOS: `DEFAULT_DATETIME = MAKE_ULONG(os_header.os_dosdate, 0)`
//! (`bios/ikbd.h:95`) is the clock value when no RTC answers
//! (`bios/clock.c:1211`, `bios/clock.c:1319`), and GEMDOS `time_init`
//! (`bdos/time.c:160-165`) seeds its clock from `Gettime()` at boot. NewTOS:
//! `tos_api::gemdos::time::time_init` seeds the one clock that XBIOS `Gettime`
//! and GEMDOS `Tgetdate`/`Tgettime` read. Spec: `docs/api/xbios/17-gettime.md`
//! ("Pi / BCM2711 binding").
//!
//! The host's monotonic seconds (the generic timer's uptime in the kernel) are
//! set by the test: the seed has to read 00:00:00 at the moment `time_init`
//! runs, whatever the uptime then, and run on with it afterwards.

use std::sync::atomic::{AtomicU64, Ordering};

use tos_api::build::{BUILD_DAY, BUILD_MONTH, BUILD_YEAR, OS_DOSDATE};
use tos_api::gemdos::time::{time_init, DEFAULT_DATETIME};
use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};

/// A host with nothing but an uptime the test sets.
struct UptimeHost {
    secs: AtomicU64,
}

impl Host for UptimeHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn monotonic_seconds(&self) -> u64 {
        self.secs.load(Ordering::Relaxed)
    }
}

static UPTIME: UptimeHost = UptimeHost { secs: AtomicU64::new(0) };
static HOST: &dyn Host = &UPTIME;

fn call(class: TrapClass, func: u16) -> i64 {
    let args = TrapArgs {
        regs: [0; 8],
        func: func as u64,
        svc_imm: class as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    match class {
        TrapClass::Xbios => tos_api::xbios::dispatch(&args),
        TrapClass::Gemdos => i64::from(tos_api::gemdos::dispatch(&args)),
        _ => unreachable!("only XBIOS and GEMDOS are called here"),
    }
}

const GETTIME: u16 = 0x17; // XBIOS
const TGETDATE: u16 = 0x2A; // GEMDOS
const TGETTIME: u16 = 0x2C; // GEMDOS

/// The seed is the build date (`build.rs`, `SOURCE_DATE_EPOCH` when set) at
/// 00:00:00, packed like `MAKE_ULONG(os_dosdate, 0)`.
#[test]
fn seed_is_the_build_date_at_midnight() {
    let expect_date =
        ((BUILD_YEAR - 1980) << 9) | ((BUILD_MONTH as u16) << 5) | BUILD_DAY as u16;
    assert_eq!(OS_DOSDATE, expect_date);
    assert_eq!(DEFAULT_DATETIME, (expect_date as u32) << 16);
}

/// After `time_init`, XBIOS `Gettime` and GEMDOS `Tgetdate`/`Tgettime` all
/// read the build date at 00:00:00 — one source for all three — and the
/// clock then runs on with the uptime.
#[test]
fn boot_clock_reads_the_build_date_everywhere() {
    install_host(&HOST);
    // Seven seconds of uptime before the seed (firmware, early boot).
    UPTIME.secs.store(7, Ordering::Relaxed);
    time_init();
    assert_eq!(call(TrapClass::Xbios, GETTIME), DEFAULT_DATETIME as i64, "Gettime");
    assert_eq!(call(TrapClass::Gemdos, TGETDATE), OS_DOSDATE as i64, "Tgetdate");
    assert_eq!(call(TrapClass::Gemdos, TGETTIME), 0, "Tgettime");

    // 61 s later: 00:01:01 → time WORD 0:1:(1/2 = 0) = 1 << 5.
    UPTIME.secs.store(7 + 61, Ordering::Relaxed);
    assert_eq!(call(TrapClass::Gemdos, TGETTIME), 1 << 5, "Tgettime after 61 s");
    assert_eq!(call(TrapClass::Xbios, GETTIME), (DEFAULT_DATETIME | (1 << 5)) as i64);
    assert_eq!(call(TrapClass::Gemdos, TGETDATE), OS_DOSDATE as i64, "same day");
}
