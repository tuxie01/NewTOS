//! XBIOS21 — the ST(e) palette `Setcolor`/`Setpalette` share starts with
//! EmuTOS' boot palette and stays away from the VDI colour table.
//!
//! EmuTOS: `screen_init` → `initialise_palette_registers`
//! (`bios/screen.c:432-457`) loads the 16 ST(e) palette registers with
//! `dflt_palette[] & 0x0FFF` (`bios/screen.c:54-71`, values
//! `bios/screen.h:56-71`) on every STe/TT/Videl profile; `atari_setcolor`
//! (`bios/screen.c:1015-1036`) returns the previous register word masked
//! `& 0x0FFF`, writes only when `color >= 0`, and masks the index `& 0x0F`.
//! `Setpalette` fills the same registers. Owner decision 2026-11-12
//! (`TASK/XBIOS21.md`): the shadow leads, and an XBIOS palette write does not
//! reach the VDI colour table in the truecolor VDI. Spec:
//! `docs/api/xbios/07-setcolor.md` ("Boot palette", Pi binding),
//! `docs/api/xbios/06-setpalette.md`.
//!
//! The palette is one global; the tests serialise on a lock, and the first
//! one to take it checks the boot state before any test writes.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SETPALETTE: u16 = 0x06;
const SETCOLOR: u16 = 0x07;

/// `dflt_palette[]` of `bios/screen.c:54-59` with the constants of
/// `bios/screen.h:56-71`, `& 0x0FFF` — written out here independently of
/// the implementation's table.
const EMUTOS_BOOT: [u16; 16] = [
    0x0FFF, 0x0F00, 0x00F0, 0x0FF0, 0x000F, 0x0F0F, 0x00FF, 0x0555, //
    0x0333, 0x0F33, 0x03F3, 0x0FF3, 0x033F, 0x0F3F, 0x03FF, 0x0000,
];

// ── a raster backend that counts colour-table writes ───────────────────────

static SET_COLOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static SET_ACTIVE_PALETTE_CALLS: AtomicUsize = AtomicUsize::new(0);

struct CountingBackend;

impl RasterBackend for CountingBackend {
    fn width(&self) -> u16 {
        640
    }
    fn height(&self) -> u16 {
        480
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
    }
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {
        SET_COLOR_CALLS.fetch_add(1, Ordering::SeqCst);
    }
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _palette: &[u16; 256]) {
        SET_ACTIVE_PALETTE_CALLS.fetch_add(1, Ordering::SeqCst);
    }
    fn pel_to_device16(&self, _pel: Pel) -> u16 {
        0
    }
    fn device_to_rgb565(&self, _argb: u32) -> u16 {
        0
    }
    fn rgb565_to_device(&self, _px: u16) -> u32 {
        0
    }
    fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
    fn abline(
        &self,
        _x0: i16,
        _y0: i16,
        _x1: i16,
        _y1: i16,
        _pel: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        mask
    }
    fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _f: PatternFill, _c: ClipRect) {}
    fn draw_glyph_sized(
        &self,
        _x: i16,
        _y: i16,
        _glyph: &[u8],
        _cell_w: i16,
        _cell_h: i16,
        _fg: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
    ) {
    }
    fn blit_mono(&self, _d: Rect, _s: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _c: ClipRect) {}
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

static BACKEND: CountingBackend = CountingBackend;

/// A host whose only display is the counting backend.
struct RasterHost;

impl Host for RasterHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&BACKEND)
    }
}

static RASTER_HOST: RasterHost = RasterHost;
static HOST: &dyn Host = &RASTER_HOST;

// ── harness ────────────────────────────────────────────────────────────────

static LOCK: Mutex<()> = Mutex::new(());
static BOOT_CHECKED: Once = Once::new();

/// Take the palette lock; the first caller asserts the boot state before any
/// test has written the palette.
fn setup() -> MutexGuard<'static, ()> {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    BOOT_CHECKED.call_once(|| {
        install_host(&HOST);
        for (n, &want) in EMUTOS_BOOT.iter().enumerate() {
            assert_eq!(setcolor(n as i64, -1), want as i64, "boot Setcolor({n},-1)");
        }
    });
    guard
}

fn xbios(func: u16, regs: [u64; 8]) -> i64 {
    let args = TrapArgs {
        regs,
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

/// `Setcolor(n, c)`, both WORD arguments sign-extended as the trap passes them.
fn setcolor(n: i64, c: i64) -> i64 {
    xbios(SETCOLOR, [n as u64, c as u64, 0, 0, 0, 0, 0, 0])
}

fn setpalette(words: &[u16; 16]) -> i64 {
    xbios(SETPALETTE, [words.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0])
}

/// Put the boot palette back so the next test starts from it.
fn restore_boot() {
    assert_eq!(setpalette(&EMUTOS_BOOT), 0);
}

// ── documented return values ───────────────────────────────────────────────

/// After boot, `Setcolor(n,-1)` returns EmuTOS' default word for every n
/// (cases bios-r1 93/94/95: `0x0FFF`, `0x0F00`, `0x0000`), and a query
/// changes nothing: asking twice gives the same table.
#[test]
fn boot_query_returns_the_emutos_default_palette() {
    let _g = setup();
    for _round in 0..2 {
        for (n, &want) in EMUTOS_BOOT.iter().enumerate() {
            assert_eq!(setcolor(n as i64, -1), want as i64, "Setcolor({n},-1)");
        }
    }
    assert_eq!(setcolor(0, -1), 0x0FFF, "case 93");
    assert_eq!(setcolor(1, -1), 0x0F00, "case 94");
    assert_eq!(setcolor(15, -1), 0x0000, "case 95");
}

/// A set returns the previous word; the following query returns the set word
/// `& 0x0FFF`; the query leaves it in place.
#[test]
fn set_returns_the_old_word_and_query_returns_the_new() {
    let _g = setup();
    assert_eq!(setcolor(1, 0x0700), 0x0F00, "old entry-1 word");
    assert_eq!(setcolor(1, -1), 0x0700);
    assert_eq!(setcolor(1, -1), 0x0700, "a query does not write");
    // Bits above 0x0FFF are not kept (STe-class mask, owner decision 3).
    assert_eq!(setcolor(2, 0x7ABC), 0x00F0, "old entry-2 word");
    assert_eq!(setcolor(2, -1), 0x0ABC);
    // Neighbours are untouched.
    assert_eq!(setcolor(0, -1), 0x0FFF);
    assert_eq!(setcolor(3, -1), 0x0FF0);
    restore_boot();
}

/// `Setpalette` fills the store `Setcolor` reads.
#[test]
fn setpalette_feeds_setcolor_queries() {
    let _g = setup();
    let mut words = [0u16; 16];
    for (n, w) in words.iter_mut().enumerate() {
        *w = 0xF000 | (n as u16 * 0x0111);
    }
    assert_eq!(setpalette(&words), 0);
    for n in 0..16 {
        assert_eq!(setcolor(n, -1), n * 0x0111, "entry {n} after Setpalette");
    }
    restore_boot();
    assert_eq!(setcolor(1, -1), 0x0F00);
}

/// Owner decision 2: neither `Setcolor` nor `Setpalette` writes the VDI
/// colour table (no `set_color`, no `set_active_palette` on the backend),
/// although a display is up.
#[test]
fn palette_writes_do_not_reach_the_vdi_colour_table() {
    let _g = setup();
    let before = (
        SET_COLOR_CALLS.load(Ordering::SeqCst),
        SET_ACTIVE_PALETTE_CALLS.load(Ordering::SeqCst),
    );
    assert_eq!(setcolor(1, 0x0123), 0x0F00);
    assert_eq!(setcolor(1, 0x0F00), 0x0123);
    restore_boot();
    let after = (
        SET_COLOR_CALLS.load(Ordering::SeqCst),
        SET_ACTIVE_PALETTE_CALLS.load(Ordering::SeqCst),
    );
    assert_eq!(after, before, "the VDI colour table was written");
}

// ── documented edge input (Setcolor has no error return) ──────────────────

/// An out-of-range index is masked `& 0x0F`, not rejected
/// (`bios/screen.c:1024`): `Setcolor(16, c)` writes entry 0, `Setcolor(-1,
/// -1)` reads entry 15. Any negative colour is the query form
/// (`bios/screen.c:1032`), not only `-1`.
#[test]
fn index_wraps_and_any_negative_colour_queries() {
    let _g = setup();
    assert_eq!(setcolor(16, 0x0123), 0x0FFF, "entry 0 via index 16");
    assert_eq!(setcolor(0, -1), 0x0123);
    assert_eq!(setcolor(-1, -1), 0x0000, "index -1 & 0x0F = 15");
    assert_eq!(setcolor(0x7FF1, -1), 0x0F00, "index 0x7FF1 & 0x0F = 1");
    assert_eq!(setcolor(4, -2), 0x000F, "-2 queries");
    assert_eq!(setcolor(4, -32768), 0x000F, "-32768 queries");
    assert_eq!(setcolor(4, -1), 0x000F, "entry 4 unchanged");
    restore_boot();
}
