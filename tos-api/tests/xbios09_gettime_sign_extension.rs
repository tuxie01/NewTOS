//! XBIOS09 — `Gettime` (XBIOS 0x17) returns its packed LONG sign-extended
//! into `x0`.
//!
//! EmuTOS declares `LONG gettime(void)` (`bios/clock.c:1266`), `LONG` being
//! a signed 32-bit type (`include/portab.h:123`); a LONG result reaches `x0`
//! sign-extended (`docs/COMPATIBILITY.md`, "LONG results in `x0`"). Year bit
//! 6 of the date word is bit 31 of the LONG, so from 2044 on the value is
//! negative. Spec: `docs/api/xbios/17-gettime.md` ("Width and extension"):
//! `0x8CCF6000` comes back as `0xFFFFFFFF8CCF6000`, `0x2B8F6000` as
//! `0x000000002B8F6000`.
//!
//! The host's uptime stays fixed, so `Gettime` reads back exactly what
//! `Settime` wrote (both values are valid dates with an even second count, so
//! `Settime`'s normalisation leaves them unchanged).

use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};

/// A host with a frozen uptime of zero seconds.
struct FrozenHost;

impl Host for FrozenHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn monotonic_seconds(&self) -> u64 {
        0
    }
}

static FROZEN: FrozenHost = FrozenHost;
static HOST: &dyn Host = &FROZEN;

const SETTIME: u16 = 0x16;
const GETTIME: u16 = 0x17;

fn xbios(func: u16, arg0: u64) -> i64 {
    let mut regs = [0u64; 8];
    regs[0] = arg0;
    let args = TrapArgs {
        regs,
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

/// Both cases run in one test: the clock is one global, and parallel tests
/// would race on it.
#[test]
fn gettime_sign_extends_the_packed_long() {
    install_host(&HOST);

    // 2050-06-15 12:00:00: year 70 sets bit 31 → negative LONG.
    assert_eq!(xbios(SETTIME, 0x8CCF_6000), 0, "Settime returns void → 0");
    assert_eq!(
        xbios(GETTIME, 0) as u64,
        0xFFFF_FFFF_8CCF_6000,
        "bit 31 set: x0 bits 63..32 are copies of bit 31"
    );
    assert_eq!(xbios(GETTIME, 0), i64::from(0x8CCF_6000u32 as i32));
    assert!(xbios(GETTIME, 0) < 0, "a LONG from 2044 on is negative");

    // 2001-12-15 12:00:00: bit 31 clear → value unchanged, upper half zero.
    assert_eq!(xbios(SETTIME, 0x2B8F_6000), 0);
    assert_eq!(xbios(GETTIME, 0) as u64, 0x0000_0000_2B8F_6000, "bit 31 clear: unchanged");
}
