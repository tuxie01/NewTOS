//! XBIOS22 — the Falcon video queries answer for the Pi screen, and `_VDO`
//! announces them.
//!
//! Owner decisions 2026-11-12 and 2026-11-18 (`TASK/XBIOS22.md`); specs
//! `docs/api/xbios/58-vsetmode.md`, `59-vgetmonitor.md`, `5b-vgetsize.md`,
//! `5e-vgetrgb.md`; ADR-012 amendment 2026-11-18 (`docs/MEMORY.md`).
//!
//! - `VsetMode(-1)` (0x58) answers the fixed word `0x001C`
//!   (`VGA|80COL|TRUECOLOR`, bios-r1 case 211 on `falcontc`); any other
//!   argument would switch the mode and answers the function number, as does
//!   `Vfixmode` (0x5F, case 215).
//! - `VgetMonitor` (0x59) answers 2, `MON_VGA` (case 212); it has no error path.
//! - `VgetSize` (0x5B) answers the byte size of the current 32-bpp screen
//!   (`ScreenInfo::size_bytes`) for every mode word, and the function number
//!   while no framebuffer is up (case 213 differs from the oracle by decision).
//! - `VgetRGB` (0x5E) reads the separate 16-entry Falcon table
//!   (`references/emutos/bios/videl.c:42-46,862-909`, case 214); `-1` for
//!   `index < 0`, `count <= 0` or `index + count > 16`, buffer untouched.
//! - `_VDO` = `0x00030000` (`VDO_FALCON`, `references/emutos/include/cookie.h:67`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::host::{install_host, Host, ScreenInfo};
use tos_api::trap::{TrapArgs, TrapClass};

const VSETMODE: u16 = 0x58;
const VGETMONITOR: u16 = 0x59;
const VGETSIZE: u16 = 0x5B;
const VGETRGB: u16 = 0x5E;
const VFIXMODE: u16 = 0x5F;

/// 1920x1080 XRGB8888 with a padded pitch, two stacked pages: one screen is
/// `stride_bytes * height`, which is what `VgetSize` must report.
const WIDTH: u16 = 1920;
const HEIGHT: u16 = 1080;
const STRIDE: u32 = 1920 * 4 + 64;
const SIZE: u32 = STRIDE * HEIGHT as u32;

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
            mem_bytes: SIZE * 2,
            width: WIDTH,
            height: HEIGHT,
            bpp: 32,
            stride_bytes: STRIDE,
            size_bytes: SIZE,
        })
    }
}

static SCREEN_HOST: ScreenHost = ScreenHost;
static HOST: &dyn Host = &SCREEN_HOST;
static INSTALL: Once = Once::new();
/// Serialises the tests that switch the framebuffer or touch the cookie jar.
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

/// A WORD argument as the trap passes it: sign-extended into the register.
fn w(v: i16) -> u64 {
    v as i64 as u64
}

// ── VsetMode (0x58), query form only ──────────────────────────────────────

/// Case 211: `VsetMode(-1)` = `0x001C` on `falcontc`.
#[test]
fn vsetmode_query_answers_the_falcon_word() {
    let _g = setup();
    assert_eq!(xbios(VSETMODE, [w(-1), 0, 0]), 0x001C);
    assert_eq!(tos_api::xbios::screen::FALCON_MODE_WORD, 0x10 | 0x08 | 0x04, "VGA|80COL|TRUECOLOR");
}

/// A real mode argument would switch the mode; that is not implemented and
/// answers the function number (owner decision 2026-11-12), the query word
/// does not change.
#[test]
fn vsetmode_with_a_mode_answers_the_function_number() {
    let _g = setup();
    for mode in [0x001C_i16, 0x0000, 0x0088, -2, i16::MIN, i16::MAX] {
        assert_eq!(xbios(VSETMODE, [w(mode), 0, 0]), 0x58, "VsetMode({mode:#x})");
    }
    assert_eq!(xbios(VSETMODE, [w(-1), 0, 0]), 0x001C, "the query is unchanged");
}

/// Case 215: `Vfixmode` stays unimplemented, function number.
#[test]
fn vfixmode_answers_the_function_number() {
    let _g = setup();
    assert_eq!(xbios(VFIXMODE, [w(0x1C), 0, 0]), 0x5F);
    assert_eq!(xbios(VFIXMODE, [w(-1), 0, 0]), 0x5F);
}

// ── VgetMonitor (0x59) ─────────────────────────────────────────────────────

/// Case 212: `MON_VGA` = 2. The call has no error path: with or without a
/// framebuffer the answer is 2.
#[test]
fn vgetmonitor_answers_vga() {
    let _g = setup();
    assert_eq!(xbios(VGETMONITOR, [0, 0, 0]), 2);
    SCREEN_UP.store(false, Ordering::SeqCst);
    assert_eq!(xbios(VGETMONITOR, [0, 0, 0]), 2);
}

// ── VgetSize (0x5B) ────────────────────────────────────────────────────────

/// Option B1: the size of one 32-bpp screen as `Physbase` shows it, stride
/// padding included, for every mode word (the argument does not name the
/// screen; case 213 differs from the oracle's 614400 by decision).
#[test]
fn vgetsize_answers_the_current_screen_bytes_for_any_mode() {
    let _g = setup();
    assert_eq!(SIZE, 8_363_520, "test geometry: 7744 * 1080");
    for mode in [0x001C_i16, 0x0000, 0x0088, 0x01FF, -1] {
        assert_eq!(xbios(VGETSIZE, [w(mode), 0, 0]), i64::from(SIZE), "VgetSize({mode:#x})");
    }
}

/// No framebuffer: the function number `0x5B` (owner decision 2026-11-18,
/// item 4; EmuTOS' no-Videl answer, `bios/videl.c:730-731`).
#[test]
fn vgetsize_without_a_framebuffer_answers_the_function_number() {
    let _g = setup();
    SCREEN_UP.store(false, Ordering::SeqCst);
    assert_eq!(xbios(VGETSIZE, [w(0x1C), 0, 0]), 0x5B);
    SCREEN_UP.store(true, Ordering::SeqCst);
    assert_eq!(xbios(VGETSIZE, [w(0x1C), 0, 0]), i64::from(SIZE));
}

// ── VgetRGB (0x5E) ─────────────────────────────────────────────────────────

/// The 64 bytes bios-r1 case 214 captured on `falcontc`
/// (`measurements/measurement-2026-11-12-xbios-full/runs/bios-r1/tosdiff-EMUTOS-1a7484ab-BIOS-falcontc.txt`),
/// as the 16 big-endian longwords they spell — written out here independently
/// of the implementation's table.
const CASE_214: [u32; 16] = [
    0x00FF_FFFF, 0x00FF_0000, 0x0000_FF00, 0x00FF_FF00, 0x0000_00FF, 0x00FF_00FF, 0x0000_FFFF, 0x00BB_BBBB, //
    0x0088_8888, 0x00AA_0000, 0x0000_AA00, 0x00AA_AA00, 0x0000_00AA, 0x00AA_00AA, 0x0000_AAAA, 0x0000_0000,
];

const SENTINEL: u32 = 0xDEAD_BEEF;

fn vgetrgb(index: i16, count: i16, buf: &mut [u32]) -> i64 {
    xbios(VGETRGB, [w(index), w(count), buf.as_mut_ptr() as u64])
}

/// Case 214: `VgetRGB(0, 16, buf)` = 0 and the Falcon default table.
#[test]
fn vgetrgb_reads_the_falcon_table() {
    let _g = setup();
    let mut buf = [SENTINEL; 17];
    assert_eq!(vgetrgb(0, 16, &mut buf), 0);
    assert_eq!(buf[..16], CASE_214);
    assert_eq!(buf[16], SENTINEL, "nothing past count is written");
}

/// A slice of the table lands at the start of the buffer.
#[test]
fn vgetrgb_reads_a_slice_from_index() {
    let _g = setup();
    let mut buf = [SENTINEL; 3];
    assert_eq!(vgetrgb(3, 2, &mut buf), 0);
    assert_eq!(buf, [0x00FF_FF00, 0x0000_00FF, SENTINEL], "yellow, blue");
    let mut one = [SENTINEL; 1];
    assert_eq!(vgetrgb(15, 1, &mut one), 0);
    assert_eq!(one, [0x0000_0000], "entry 15 is black");
}

/// `-1` for `index < 0`, `count <= 0`, `index + count > 16`
/// (`bios/videl.c:872-879`; the limit is 16 for the truecolor word), and the
/// buffer stays untouched.
#[test]
fn vgetrgb_rejects_bad_ranges_without_writing() {
    let _g = setup();
    for (index, count) in [(-1, 1), (i16::MIN, 16), (0, 0), (0, -1), (0, 17), (15, 2), (16, 1), (8, 9), (i16::MAX, i16::MAX)] {
        let mut buf = [SENTINEL; 20];
        assert_eq!(vgetrgb(index, count, &mut buf), -1, "VgetRGB({index}, {count})");
        assert!(buf.iter().all(|&v| v == SENTINEL), "VgetRGB({index}, {count}) wrote the buffer");
    }
}

/// `Setcolor` does not reach the Falcon table (two tables, as in EmuTOS,
/// owner decision 2026-11-12 option A).
#[test]
fn setcolor_does_not_change_vgetrgb() {
    let _g = setup();
    let old = xbios(0x07, [1, 0x0123, 0]);
    let mut buf = [SENTINEL; 16];
    assert_eq!(vgetrgb(0, 16, &mut buf), 0);
    assert_eq!(buf, CASE_214);
    xbios(0x07, [1, old as u64, 0]);
}

// ── _VDO ───────────────────────────────────────────────────────────────────

/// `Ssystem(S_GETCOOKIE, '_VDO', &v)` after the boot seeding: `E_OK` and
/// `0x00030000`; the value form answers the same number.
#[test]
fn the_seeded_vdo_is_vdo_falcon() {
    let _g = setup();
    tos_api::cookie::seed_defaults(0x0000_0100);
    assert_eq!(tos_api::cookie::VDO_FALCON, 0x0003_0000, "references/emutos/include/cookie.h:67");
    let tag = u32::from_be_bytes(*b"_VDO");
    let mut v: u32 = 0x5A5A_5A5A;
    let gemdos = |a0: u64, a1: u64, a2: u64| {
        let args = TrapArgs {
            regs: [a0, a1, a2, 0, 0, 0, 0, 0],
            func: 0x154,
            svc_imm: TrapClass::Gemdos as u16,
            space: tos_api::uaccess::Space::Kernel,
        };
        tos_api::gemdos::dispatch(&args)
    };
    assert_eq!(gemdos(8, tag as i32 as i64 as u64, &raw mut v as u64), 0, "E_OK");
    assert_eq!(v, 0x0003_0000);
    assert_eq!(gemdos(8, tag as i32 as i64 as u64, 0), 0x0003_0000, "value form");
}
