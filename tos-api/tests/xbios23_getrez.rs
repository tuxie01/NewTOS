//! XBIOS23 — `Getrez` (0x04) answers `ST_HIGH` (2), the code EmuTOS gives a
//! non-ST-compatible Falcon mode (truecolor included).
//!
//! Owner decision 2026-11-12 (`TASK/XBIOS23.md`, option A); ADR-017
//! Revision 3 (`docs/MEMORY.md`); spec `docs/api/xbios/04-getrez.md`.
//!
//! - `Getrez()` = 2 with and without a framebuffer: the call has no error
//!   path (bios-r1 case 87 = 2 on `falcontc`).
//! - It agrees with `VsetMode(-1)` by EmuTOS's own mapping
//!   (`references/emutos/bios/screen.c:955-972`): the word has
//!   `VIDEL_COMPAT` (0x80) clear, so the answer is 2.
//! - `Setscreen`'s `rez` is accepted and ignored (`05-setscreen.md`):
//!   `Getrez` does not follow it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::host::{install_host, Host, ScreenInfo};
use tos_api::trap::{TrapArgs, TrapClass};

const GETREZ: u16 = 0x04;
const SETSCREEN: u16 = 0x05;
const VSETMODE: u16 = 0x58;

/// `references/emutos/include/biosdefs.h:126,147,152`.
const ST_HIGH: i64 = 2;
const VIDEL_COMPAT: i64 = 0x0080;
const VIDEL_BPPMASK: i64 = 0x0007;

/// Whether the test host currently has a framebuffer.
static SCREEN_UP: AtomicBool = AtomicBool::new(true);

struct ScreenHost;

impl Host for ScreenHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn screen(&self) -> Option<ScreenInfo> {
        SCREEN_UP.load(Ordering::SeqCst).then_some(ScreenInfo {
            base: 0x1000_0000,
            phys_base: 0x1000_0000,
            mem_base: 0x1000_0000,
            mem_bytes: 1920 * 4 * 1080 * 2,
            width: 1920,
            height: 1080,
            bpp: 32,
            stride_bytes: 1920 * 4,
            size_bytes: 1920 * 4 * 1080,
        })
    }
}

static SCREEN_HOST: ScreenHost = ScreenHost;
static HOST: &dyn Host = &SCREEN_HOST;
static INSTALL: Once = Once::new();
/// Serialises the tests that switch the framebuffer.
static LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    INSTALL.call_once(|| install_host(&HOST));
    SCREEN_UP.store(true, Ordering::SeqCst);
    guard
}

fn xbios(func: u16, regs: [u64; 3]) -> i64 {
    let args = TrapArgs {
        regs: [regs[0], regs[1], regs[2], 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

/// A WORD/LONG argument as the trap passes it: sign-extended into the register.
fn w(v: i64) -> u64 {
    v as u64
}

/// EmuTOS `atari_getrez` on Videl (`references/emutos/bios/screen.c:955-972`),
/// the ST-compatible branch included, as the independent reference for the
/// coherence check.
fn emutos_videl_getrez(vmode: i64) -> i64 {
    if vmode & VIDEL_COMPAT != 0 {
        match vmode & VIDEL_BPPMASK {
            0 => 2,
            1 => 1,
            _ => 0,
        }
    } else {
        2
    }
}

/// Case 87: `Getrez()` = 2 (`ST_HIGH`), the oracle's answer on `falcontc`.
#[test]
fn getrez_answers_st_high() {
    let _g = setup();
    assert_eq!(xbios(GETREZ, [0, 0, 0]), ST_HIGH);
    assert_eq!(tos_api::xbios::screen::getrez(), ST_HIGH);
}

/// `Getrez` cannot fail: the same 2 while no framebuffer is up.
#[test]
fn getrez_answers_st_high_without_a_framebuffer() {
    let _g = setup();
    SCREEN_UP.store(false, Ordering::SeqCst);
    assert_eq!(xbios(GETREZ, [0, 0, 0]), ST_HIGH);
    SCREEN_UP.store(true, Ordering::SeqCst);
}

/// `Getrez` and `VsetMode(-1)` describe the same mode: the word is not
/// ST-compatible, and EmuTOS's mapping of it is 2 (cases 87 and 211).
#[test]
fn getrez_agrees_with_the_vsetmode_query() {
    let _g = setup();
    let word = xbios(VSETMODE, [w(-1), 0, 0]);
    assert_eq!(word & VIDEL_COMPAT, 0, "VsetMode(-1) = {word:#x} has VIDEL_COMPAT clear");
    assert_eq!(xbios(GETREZ, [0, 0, 0]), emutos_videl_getrez(word));
}

/// `Setscreen(-1, -1, rez)` returns 0 and leaves `Getrez` at 2 for every
/// code (the `rez` is ignored, no in-system mode switch, ADR-017).
#[test]
fn setscreen_rez_does_not_change_getrez() {
    let _g = setup();
    for rez in [0_i64, 1, 2, 3, 4, 6, 7, 9, -1] {
        assert_eq!(xbios(SETSCREEN, [w(-1), w(-1), w(rez)]), 0, "Setscreen(-1,-1,{rez})");
        assert_eq!(xbios(GETREZ, [0, 0, 0]), ST_HIGH, "Getrez after Setscreen(-1,-1,{rez})");
    }
}
