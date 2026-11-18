//! V68 — the XBIOS video answers do not depend on the VDI depth setting:
//! at the default 16 planes and at the 32-plane setting (`vdiplanes=32`)
//! `Getrez`, `VsetMode`, `VgetMonitor`, `VgetSize`, `VgetRGB` and `Vfixmode`
//! answer the same, documented values.
//!
//! Contract: ADR-038 point 9 (`docs/MEMORY.md`). Specs, section "At the
//! 32-plane setting" in `docs/api/xbios/04-getrez.md`, `58-vsetmode.md` and
//! `5b-vgetsize.md`; the values themselves are those of
//! `docs/api/xbios/59-vgetmonitor.md`, `5e-vgetrgb.md` and the bios-r1 cases
//! 87 and 211-215 (`tosdump/cases/bios-r1.tsv`), as
//! `tos-api/tests/xbios22_falcon_video.rs` checks them at the default.
//!
//! The kernel's backend answers `planes()` from the boot argument
//! (`kernel/src/vdi_backend.rs`, `set_planes`); here a metadata backend
//! answers whatever the test stores, beside a framebuffer that can be taken
//! down, so one binary asks every call at both settings. A guard checks that
//! the installed host really reports the setting before the calls are made.
//!
//! Ticket: `TASK/V68.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::host::{Host, ScreenInfo};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const GETREZ: u16 = 0x04;
const VSETMODE: u16 = 0x58;
const VGETMONITOR: u16 = 0x59;
const VGETSIZE: u16 = 0x5B;
const VGETRGB: u16 = 0x5E;
const VFIXMODE: u16 = 0x5F;

/// The two settings of the boot argument `vdiplanes` (ADR-038 point 2).
const SETTINGS: [u16; 2] = [16, 32];

// ── a metadata backend with a settable plane count ───────────────────────

struct Profile {
    planes: AtomicU16,
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        WIDTH
    }
    fn height(&self) -> u16 {
        HEIGHT
    }
    fn planes(&self) -> u16 {
        self.planes.load(Ordering::Relaxed)
    }
    /// The 256 pens of both settings (ADR-024 point 2, ADR-038 point 3).
    fn num_colors(&self) -> u16 {
        256
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {}
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, p: &[u32; 256]) {}
    fn pel_to_device16(&self, p: Pel) -> u16 {
        0
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        0
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        0
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {}
    fn abline(
        &self,
        x: i16,
        y: i16,
        xx: i16,
        yy: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {}
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        g: &[u8],
        w: i16,
        h: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
    ) {
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {}
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {}
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {}
}

// ── the host: the backend plus a 32-bpp framebuffer ──────────────────────

/// 1920x1080 XRGB8888 with a padded pitch, two stacked pages: one screen is
/// `stride_bytes * height`, which is what `VgetSize` must report.
const WIDTH: u16 = 1920;
const HEIGHT: u16 = 1080;
const STRIDE: u32 = 1920 * 4 + 64;
const SIZE: u32 = STRIDE * HEIGHT as u32;

struct TestHost {
    raster: Profile,
    screen_up: AtomicBool,
}

impl Host for TestHost {
    fn console_write_byte(&self, byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn screen(&self) -> Option<ScreenInfo> {
        self.screen_up.load(Ordering::SeqCst).then_some(ScreenInfo {
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

static HOST: TestHost = TestHost { raster: Profile { planes: AtomicU16::new(16) }, screen_up: AtomicBool::new(true) };
static HOST_REF: &dyn Host = &HOST;

/// The installed host is process-wide.
static SERIAL: Mutex<()> = Mutex::new(());
static INSTALL: OnceLock<()> = OnceLock::new();

/// Take the lock, set the depth the boot argument would have chosen and
/// whether a framebuffer is up, and check that the installed host reports
/// that depth (so a call that read it would see it).
fn setting(planes: u16, screen_up: bool) -> MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    INSTALL.get_or_init(|| tos_api::host::install_host(&HOST_REF));
    HOST.raster.planes.store(planes, Ordering::Relaxed);
    HOST.screen_up.store(screen_up, Ordering::SeqCst);
    let host = tos_api::host::host().expect("host installed");
    assert_eq!(host.raster().map(|r| r.planes()), Some(planes), "the setting is in force");
    assert_eq!(host.screen().is_some(), screen_up, "the framebuffer state is in force");
    g
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

const SENTINEL: u32 = 0xDEAD_BEEF;

/// `VgetRGB(index, count, buf)` into a 17-long buffer of sentinels: the
/// answer and what was written.
fn vgetrgb(index: i16, count: i16) -> (i64, [u32; 17]) {
    let mut buf = [SENTINEL; 17];
    let rc = xbios(VGETRGB, [w(index), w(count), buf.as_mut_ptr() as u64]);
    (rc, buf)
}

/// The 64 bytes bios-r1 case 214 captured on `falcontc` as 16 longwords
/// (`docs/api/xbios/5e-vgetrgb.md`), written out independently of the
/// implementation's table; entry 16 of the buffer stays the sentinel.
const CASE_214: [u32; 17] = [
    0x00FF_FFFF, 0x00FF_0000, 0x0000_FF00, 0x00FF_FF00, 0x0000_00FF, 0x00FF_00FF, 0x0000_FFFF, 0x00BB_BBBB, //
    0x0088_8888, 0x00AA_0000, 0x0000_AA00, 0x00AA_AA00, 0x0000_00AA, 0x00AA_00AA, 0x0000_AAAA, 0x0000_0000, //
    SENTINEL,
];

/// Every answer of the XBIOS video group at the current setting, named.
/// `VgetRGB` contributes its return code and the buffer it wrote.
#[derive(Debug, PartialEq, Eq)]
struct Answers {
    getrez: i64,
    vsetmode_query: i64,
    vsetmode_0: i64,
    vsetmode_1c: i64,
    vgetmonitor: i64,
    vgetsize_1c: i64,
    vgetsize_0: i64,
    vgetrgb_full: (i64, [u32; 17]),
    vgetrgb_slice: (i64, [u32; 17]),
    vgetrgb_bad: (i64, [u32; 17]),
    vfixmode: i64,
}

fn answers() -> Answers {
    Answers {
        getrez: xbios(GETREZ, [0, 0, 0]),
        vsetmode_query: xbios(VSETMODE, [w(-1), 0, 0]),
        vsetmode_0: xbios(VSETMODE, [w(0), 0, 0]),
        vsetmode_1c: xbios(VSETMODE, [w(0x1C), 0, 0]),
        vgetmonitor: xbios(VGETMONITOR, [0, 0, 0]),
        vgetsize_1c: xbios(VGETSIZE, [w(0x1C), 0, 0]),
        vgetsize_0: xbios(VGETSIZE, [w(0), 0, 0]),
        vgetrgb_full: vgetrgb(0, 16),
        vgetrgb_slice: vgetrgb(3, 2),
        vgetrgb_bad: vgetrgb(0, 17),
        vfixmode: xbios(VFIXMODE, [w(0x1C), 0, 0]),
    }
}

/// The documented answers with a framebuffer up, at either setting.
fn documented_with_screen() -> Answers {
    let mut slice = [SENTINEL; 17];
    slice[0] = 0x00FF_FF00; // entry 3, yellow
    slice[1] = 0x0000_00FF; // entry 4, blue
    Answers {
        getrez: 2,           // ST_HIGH: mode word without VIDEL_COMPAT (case 87)
        vsetmode_query: 0x1C, // VGA|80COL|TRUECOLOR (case 211)
        vsetmode_0: 0x58,    // a mode switch is not implemented: function number
        vsetmode_1c: 0x58,
        vgetmonitor: 2, // MON_VGA (case 212)
        vgetsize_1c: i64::from(SIZE), // one 32-bpp screen, stride padding included
        vgetsize_0: i64::from(SIZE),  // the mode argument is not read
        vgetrgb_full: (0, CASE_214),  // case 214
        vgetrgb_slice: (0, slice),
        vgetrgb_bad: (-1, [SENTINEL; 17]), // index + count > 16: -1, buffer untouched
        vfixmode: 0x5F,                    // not implemented: function number (case 215)
    }
}

/// ADR-038 point 9: with a framebuffer up every answer is the documented
/// one at 16 planes and at 32 planes, and the two settings agree.
#[test]
fn xbios_video_answers_are_the_documented_ones_at_both_settings() {
    let mut seen = Vec::new();
    for planes in SETTINGS {
        let _g = setting(planes, true);
        let a = answers();
        assert_eq!(a, documented_with_screen(), "vdiplanes={planes}");
        seen.push(a);
    }
    assert_eq!(seen[0], seen[1], "the 16- and the 32-plane setting answer alike");
}

/// Without a framebuffer `VgetSize` answers its function number `0x5B` at
/// both settings (`docs/api/xbios/5b-vgetsize.md`); every other answer is the
/// same as with one (none of them reads the screen).
#[test]
fn without_a_framebuffer_vgetsize_answers_0x5b_at_both_settings() {
    let mut seen = Vec::new();
    for planes in SETTINGS {
        let _g = setting(planes, false);
        let a = answers();
        let want = Answers { vgetsize_1c: 0x5B, vgetsize_0: 0x5B, ..documented_with_screen() };
        assert_eq!(a, want, "vdiplanes={planes}, no framebuffer");
        seen.push(a);
    }
    assert_eq!(seen[0], seen[1], "the 16- and the 32-plane setting answer alike");
}

/// `VsetMode(-1)` is the constant the build checks (`FALCON_MODE_WORD`):
/// the same word the query answered above, at both settings (owner choice
/// O4: the word says 16-bit truecolor beside a 32-plane VDI).
#[test]
fn the_mode_word_is_the_falcon_truecolor_word_at_both_settings() {
    for planes in SETTINGS {
        let _g = setting(planes, true);
        assert_eq!(tos_api::xbios::screen::FALCON_MODE_WORD, 0x001C);
        assert_eq!(xbios(VSETMODE, [w(-1), 0, 0]), i64::from(tos_api::xbios::screen::FALCON_MODE_WORD), "vdiplanes={planes}");
    }
}
