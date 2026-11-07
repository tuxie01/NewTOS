//! Debug feature `raster-prof` (TASK/V49 part 3): time spent in the VDI
//! screen backend per raster call, and inside it separately the cache
//! maintenance of `VdiScreen::clean`, so a Pi run tells the raster work from
//! the `dc cvac` / `dc civac` that follows it.
//!
//! Counted, per backend method (`crate::vdi_backend`): calls that reached the
//! drawing (a rectangle clipped away entirely is not counted), pixels of the
//! clipped rectangle (rectangle methods and the glyph box), the method's whole time, and
//! the part of it spent in `clean`. `flush_rect` is its own row (tos-api's
//! own row writers flush through it), so every `clean` belongs to exactly
//! one row.
//!
//! Clock: the generic timer's `CNTPCT_EL0` at `CNTFRQ_EL0` (54 MHz on the
//! Pi 4: 18.5 ns per tick), read after an `ISB` — the counter the kernel
//! already uses (`kernel/src/drivers/timer.rs`). Not the PMU cycle counter:
//! nothing in the boot path sets `MDCR_EL2`, so whether the firmware lets
//! EL1 reach `PMCCNTR_EL0` is unknown, and a trap there would take the test
//! kernel down; the generic timer is reachable at EL1 by construction
//! (`CNTHCTL_EL2`, `kernel/src/arch/aarch64/boot.rs`). A call shorter than
//! one tick still adds up correctly over many calls on average.
//!
//! Ctrl+Alt+F8 shows the report over the top left of the screen (and sends
//! it to the serial console) and **starts the count again**; the next
//! Ctrl+Alt+F8 hides it. So: show+hide before the measured run, show after
//! it. Drawing and hiding the box go through `put_device_pixel` (not
//! counted) and one `flush_rect` each (counted, in the next interval).
//! Runs from the trap-side input pump only, like the `sd-diag` report
//! (`crate::sd_diag`), never from the timer interrupt.
//!
//! Off by default; with it off nothing of this module or its hooks is
//! compiled (`#[cfg]` on every hook line). `cargo xtask build --board rpi4
//! --release --features raster-prof`.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tos_api::bios::shifty::{MODE_ALT, MODE_CTRL};
use tos_api::vdi::backend::RasterBackend;
use tos_api::vdi::font;

use crate::vdi_backend::{self, VDI_SCREEN};

/// The counted backend methods, in report order.
const NAMES: [&str; 7] = ["fill_rect", "fill_pattern", "abline", "glyph", "blit_mono", "copy_rect", "flush_rect"];
const OPS: usize = NAMES.len();

static CALLS: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
static PIXELS: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
static TOTAL: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
static CLEAN: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
/// `clean` ticks of the running method, taken by its [`Scope`] at its end.
static CLEAN_PENDING: AtomicU64 = AtomicU64::new(0);
/// Tick of the last restart of the count (`0` = boot).
static SINCE: AtomicU64 = AtomicU64::new(0);

/// `CNTPCT_EL0` after an `ISB`, so earlier instructions have completed.
#[inline]
fn now() -> u64 {
    let t: u64;
    // SAFETY: `isb` and a read of the EL1-accessible physical counter; no
    // memory or flag effects. Not `nomem`, so the compiler keeps it in
    // program order with the raster `asm!` blocks.
    unsafe { core::arch::asm!("isb", "mrs {0}, cntpct_el0", out(reg) t, options(nostack, preserves_flags)) };
    t
}

fn frequency() -> u64 {
    let f: u64;
    // SAFETY: read-only system-register read, permitted at EL1.
    unsafe { core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) f, options(nomem, nostack, preserves_flags)) };
    f
}

/// One counted backend call: from its creation to its drop.
pub struct Scope {
    op: usize,
    pixels: u64,
    t0: u64,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let dt = now().wrapping_sub(self.t0);
        let c = CLEAN_PENDING.swap(0, Ordering::Relaxed);
        CALLS[self.op].fetch_add(1, Ordering::Relaxed);
        PIXELS[self.op].fetch_add(self.pixels, Ordering::Relaxed);
        TOTAL[self.op].fetch_add(dt, Ordering::Relaxed);
        CLEAN[self.op].fetch_add(c, Ordering::Relaxed);
    }
}

fn scope(op: usize, pixels: u64) -> Scope {
    CLEAN_PENDING.store(0, Ordering::Relaxed);
    Scope { op, pixels, t0: now() }
}

/// Pixels of the inclusive rectangle `clamp_to` returned.
fn area(x0: i32, y0: i32, x1: i32, y1: i32) -> u64 {
    ((x1 - x0 + 1) as u64) * ((y1 - y0 + 1) as u64)
}

/// `RasterBackend::fill_rect`, after clipping.
pub fn fill_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Scope {
    scope(0, area(x0, y0, x1, y1))
}
/// `RasterBackend::fill_pattern`, after clipping.
pub fn fill_pattern(x0: i32, y0: i32, x1: i32, y1: i32) -> Scope {
    scope(1, area(x0, y0, x1, y1))
}
/// `RasterBackend::abline`.
pub fn abline() -> Scope {
    scope(2, 0)
}
/// `RasterBackend::draw_glyph_sized` (pixels: [`Scope::add_area`]).
pub fn glyph() -> Scope {
    scope(3, 0)
}
/// `RasterBackend::blit_mono`, after clipping.
pub fn blit_mono(x0: i32, y0: i32, x1: i32, y1: i32) -> Scope {
    scope(4, area(x0, y0, x1, y1))
}
/// `RasterBackend::copy_rect`, after clipping.
pub fn copy_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Scope {
    scope(5, area(x0, y0, x1, y1))
}
/// `RasterBackend::flush_rect`.
pub fn flush_rect() -> Scope {
    scope(6, 0)
}

/// The cache maintenance of one `VdiScreen::clean`: from creation to drop,
/// added to the running method's [`Scope`].
pub struct CleanScope(u64);

impl Drop for CleanScope {
    fn drop(&mut self) {
        CLEAN_PENDING.fetch_add(now().wrapping_sub(self.0), Ordering::Relaxed);
    }
}

/// Start timing one `clean`.
pub fn clean() -> CleanScope {
    CleanScope(now())
}

// ── The report ─────────────────────────────────────────────────────────────

const KEY_F8: u8 = 0x42;
const COLUMNS: usize = 72;
const LINES: usize = OPS + 5;
const TEXT_MAX: usize = (COLUMNS + 1) * LINES;
const X0: usize = 8;
const Y0: usize = 24;
const MARGIN: usize = 4;
const CELL_W: usize = font::CELL_W as usize;
const CELL_H: usize = font::CELL_H as usize;
const BOX_W: usize = COLUMNS * CELL_W + 2 * MARGIN;
const BOX_H: usize = LINES * CELL_H + 2 * MARGIN;
const BG: u32 = 0x00FF_FFFF;
const FG: u32 = 0x0000_0000;

struct Text<'a> {
    out: &'a mut [u8],
    n: usize,
}

impl Write for Text<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let take = s.len().min(self.out.len() - self.n);
        self.out[self.n..self.n + take].copy_from_slice(&s.as_bytes()[..take]);
        self.n += take;
        Ok(())
    }
}

/// Ticks to microseconds.
fn us(t: u64, frq: u64) -> u64 {
    if frq == 0 {
        return 0;
    }
    (u128::from(t) * 1_000_000 / u128::from(frq)) as u64
}

/// Write the counters since the last restart as text and restart the count.
fn take_report(out: &mut [u8]) -> usize {
    let t = now();
    let frq = frequency();
    let since = SINCE.swap(t, Ordering::Relaxed);
    let mut text = Text { out, n: 0 };
    let _ = write_report(&mut text, t.wrapping_sub(since), frq);
    for i in 0..OPS {
        CALLS[i].store(0, Ordering::Relaxed);
        PIXELS[i].store(0, Ordering::Relaxed);
        TOTAL[i].store(0, Ordering::Relaxed);
        CLEAN[i].store(0, Ordering::Relaxed);
    }
    text.n
}

fn write_report(w: &mut Text<'_>, interval: u64, frq: u64) -> fmt::Result {
    writeln!(w, "raster-prof (V49)  Ctrl+Alt+F8: hide; show again restarts the count")?;
    writeln!(w, "interval {} us, counter {} Hz, times in us", us(interval, frq), frq)?;
    writeln!(w, "{:<12} {:>9} {:>11} {:>11} {:>11} {:>11}", "op", "calls", "pixels", "total", "clean", "raster")?;
    let (mut st, mut sc) = (0u64, 0u64);
    for (i, name) in NAMES.iter().enumerate() {
        let (total, clean) = (TOTAL[i].load(Ordering::Relaxed), CLEAN[i].load(Ordering::Relaxed));
        st += total;
        sc += clean;
        writeln!(
            w,
            "{:<12} {:>9} {:>11} {:>11} {:>11} {:>11}",
            name,
            CALLS[i].load(Ordering::Relaxed),
            PIXELS[i].load(Ordering::Relaxed),
            us(total, frq),
            us(clean, frq),
            us(total.saturating_sub(clean), frq)
        )?;
    }
    writeln!(w, "{:<12} {:>9} {:>11} {:>11} {:>11} {:>11}", "sum", "", "", us(st, frq), us(sc, frq), us(st.saturating_sub(sc), frq))?;
    writeln!(w, "raster = total - clean (clip, colour, kernel or pixel loop)")
}

/// The pixels under the box, from the kernel heap at the first show.
static SAVED: crate::sync::SpinMutex<Option<alloc::boxed::Box<[u32]>>> = crate::sync::SpinMutex::new(None);
static SHOWN: AtomicBool = AtomicBool::new(false);
static BOX: crate::sync::SpinMutex<(usize, usize)> = crate::sync::SpinMutex::new((0, 0));

fn show() {
    let mut buf = [0u8; TEXT_MAX];
    let len = take_report(&mut buf);
    let text = &buf[..len];
    if let Ok(s) = core::str::from_utf8(text) {
        crate::kprintln!("[rasterprof]\n{}[rasterprof] end", s);
    }
    if !vdi_backend::available() {
        return;
    }
    let (sw, sh) = vdi_backend::dims();
    let lines = text.iter().filter(|&&b| b == b'\n').count();
    let w = BOX_W.min((sw as usize).saturating_sub(X0));
    let h = (lines.min(LINES) * CELL_H + 2 * MARGIN).min((sh as usize).saturating_sub(Y0));
    if w == 0 || h == 0 {
        return;
    }
    let mut guard = SAVED.lock();
    let saved = guard.get_or_insert_with(|| alloc::vec![0u32; BOX_W * BOX_H].into_boxed_slice());
    for y in 0..h {
        for x in 0..w {
            saved[y * w + x] = VDI_SCREEN.get_device_pixel((X0 + x) as i16, (Y0 + y) as i16);
            let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            VDI_SCREEN.put_device_pixel((X0 + x) as i16, (Y0 + y) as i16, if edge { FG } else { BG });
        }
    }
    let (mut col, mut row) = (0usize, 0usize);
    for &b in text {
        if b == b'\n' {
            col = 0;
            row += 1;
            continue;
        }
        if col < COLUMNS && row < LINES {
            for (dy, bits) in font::glyph(u16::from(b)).iter().enumerate() {
                for dx in 0..CELL_W {
                    let (px, py) = (X0 + MARGIN + col * CELL_W + dx, Y0 + MARGIN + row * CELL_H + dy);
                    if bits & (0x80 >> dx) != 0 && px < X0 + w && py < Y0 + h {
                        VDI_SCREEN.put_device_pixel(px as i16, py as i16, FG);
                    }
                }
            }
        }
        col += 1;
    }
    VDI_SCREEN.flush_rect(X0 as i16, Y0 as i16, (X0 + w - 1) as i16, (Y0 + h - 1) as i16);
    *BOX.lock() = (w, h);
    SHOWN.store(true, Ordering::Relaxed);
}

fn hide() {
    let (w, h) = *BOX.lock();
    if let Some(saved) = SAVED.lock().as_ref() {
        for y in 0..h {
            for x in 0..w {
                VDI_SCREEN.put_device_pixel((X0 + x) as i16, (Y0 + y) as i16, saved[y * w + x]);
            }
        }
    }
    if w > 0 && h > 0 {
        VDI_SCREEN.flush_rect(X0 as i16, Y0 as i16, (X0 + w - 1) as i16, (Y0 + h - 1) as i16);
    }
    SHOWN.store(false, Ordering::Relaxed);
}

/// One key byte from the decoder, bit 7 = break. `true` if it is the hot key
/// Ctrl+Alt+F8 (make and break), which goes no further.
pub fn hotkey(raw: u8, shifty: u8) -> bool {
    let both = MODE_CTRL | MODE_ALT;
    if raw & 0x7F != KEY_F8 || shifty & both != both {
        return false;
    }
    if raw & 0x80 == 0 {
        if SHOWN.load(Ordering::Relaxed) {
            hide();
        } else {
            show();
        }
    }
    true
}

impl Scope {
    /// Count the pixels of the inclusive rectangle `clamp_to` returned, for
    /// a method that knows them only after its scope began
    /// (`draw_glyph_sized`'s box path, TASK/V50; its clip-off run-on path
    /// counts none).
    pub fn add_area(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.pixels += area(x0, y0, x1, y1);
    }
}
