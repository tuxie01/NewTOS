//! The SD diagnosis of the debug feature `sd-diag` (TASK/HW08) on the screen:
//! Ctrl+Alt+F9 shows the report over the top left of the screen and hides it
//! again, the first driver failure after the boot shows it once by itself, and
//! the same text goes to the serial console and to
//! `Ssystem(S_NEWTOS_SDDIAG)`. Text: [`tos_api::bios::sd_diag`].
//!
//! Made for the moment nothing can be written to the card any more, on a
//! machine with only HDMI and a USB keyboard: it reads RAM and the driver's
//! counters and writes pixels, it makes no GEMDOS call, opens no file and
//! sends no command to the card. Its one allocation, the pixel save area, comes
//! from the kernel heap at the first show.
//!
//! The pixels under the report are saved and put back on hide. The system runs
//! on while the report is up, so what a program draws there in the meantime is
//! overwritten on hide; the next redraw puts it right. Hide and show again to
//! refresh the numbers.
//!
//! Runs only from the trap-side input pump ([`hotkey`], [`poll`]) and from a
//! trap: never inside a cache or driver operation, and never in the timer
//! interrupt, whose pump decodes pointer motion only (`crate::input_gate`).

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use tos_api::bdos::fs::cache;
use tos_api::bios::bcache;
use tos_api::bios::sd_diag::{
    COLUMNS, DIRTY_LISTED, DriverReport, LINES, Report, TEXT_MAX, write_report,
};
use tos_api::bios::shifty::{MODE_ALT, MODE_CTRL};
use tos_api::vdi::backend::RasterBackend;
use tos_api::vdi::font;

use crate::vdi_backend::{self, VDI_SCREEN};

/// F9, IKBD scancode.
const KEY_F9: u8 = 0x43;
/// The hot key as the title names it.
const HOT_KEY: &str = "Ctrl+Alt+F9";

/// Top left corner of the report on the screen.
const X0: usize = 8;
const Y0: usize = 24;
/// Margin inside the box.
const MARGIN: usize = 4;
const CELL_W: usize = font::CELL_W as usize;
const CELL_H: usize = font::CELL_H as usize;
/// Most pixels of the box.
const BOX_W: usize = COLUMNS * CELL_W + 2 * MARGIN;
const BOX_H: usize = LINES * CELL_H + 2 * MARGIN;

const BG: u32 = 0x00FF_FFFF;
const FG: u32 = 0x0000_0000;

/// Bytes writer over a slice; what does not fit is cut.
struct Text<'a> {
    out: &'a mut [u8],
    n: usize,
}

impl Write for Text<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = self.out.len() - self.n;
        let take = s.len().min(room);
        self.out[self.n..self.n + take].copy_from_slice(&s.as_bytes()[..take]);
        self.n += take;
        Ok(())
    }
}

/// Microseconds since boot.
fn now_us() -> u64 {
    let (now, frq): (u64, u64);
    // SAFETY: read-only system-register reads, permitted at EL1.
    unsafe {
        core::arch::asm!("mrs {0}, cntpct_el0", out(reg) now, options(nomem, nostack, preserves_flags));
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) frq, options(nomem, nostack, preserves_flags));
    }
    if frq == 0 {
        return 0;
    }
    (u128::from(now) * 1_000_000 / u128::from(frq)) as u64
}

/// The whole report as text: its length.
pub fn render(out: &mut [u8]) -> usize {
    let mut dirty = [cache::DirtySlot::default(); DIRTY_LISTED];
    let n = cache::dirty_slots(&mut dirty).min(DIRTY_LISTED);
    let mut text = Text { out, n: 0 };
    #[cfg(feature = "board-rpi4")]
    let _ = crate::drivers::bcm2711::emmc::with_diag_report(|d| {
        write_text(&mut text, Some(*d), &dirty[..n])
    });
    #[cfg(not(feature = "board-rpi4"))]
    let _ = write_text(&mut text, None, &dirty[..n]);
    text.n
}

fn write_text(
    text: &mut Text<'_>,
    driver: Option<DriverReport<'_>>,
    dirty: &[cache::DirtySlot],
) -> fmt::Result {
    write_report(
        text,
        &Report {
            now_us: now_us(),
            driver,
            writeback: cache::writeback_stats(),
            codes: cache::error_codes(),
            dirty,
            bcache: bcache::stats(),
            hot_key: HOT_KEY,
        },
    )
}

/// The pixels under the box, taken from the kernel heap at the first show
/// (the image has no room for them: `STRAM_OS_RESERVE`) and kept. Touched only
/// from the trap side of the input pump and from a trap (module docs): single
/// CPU, one at a time, hence the plain lock.
static SAVED: crate::sync::SpinMutex<Option<alloc::boxed::Box<[u32]>>> =
    crate::sync::SpinMutex::new(None);
/// The report is on the screen.
static SHOWN: AtomicBool = AtomicBool::new(false);
/// Box of the report now on the screen: `(w, h)`, valid while [`SHOWN`].
static BOX: crate::sync::SpinMutex<(usize, usize)> = crate::sync::SpinMutex::new((0, 0));

/// Show the report: save the pixels under it, draw it, copy it to the serial
/// console.
fn show() {
    let mut buf = [0u8; TEXT_MAX];
    let len = render(&mut buf);
    let text = &buf[..len];
    if let Ok(s) = core::str::from_utf8(text) {
        crate::kprintln!("[sddiag]\n{}[sddiag] end", s);
    }
    if !vdi_backend::available() {
        return;
    }
    let (sw, sh) = vdi_backend::dims();
    let lines = text.iter().filter(|&&b| b == b'\n').count();
    let w = BOX_W.min((sw as usize).saturating_sub(X0));
    let h = (lines.min(LINES) * CELL_H + 2 * MARGIN).min((sh as usize).saturating_sub(Y0));
    let mut guard = SAVED.lock();
    let saved = guard.get_or_insert_with(|| alloc::vec![0u32; BOX_W * BOX_H].into_boxed_slice());
    for y in 0..h {
        for x in 0..w {
            saved[y * w + x] = VDI_SCREEN.get_device_pixel((X0 + x) as i16, (Y0 + y) as i16);
            let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            VDI_SCREEN.put_device_pixel(
                (X0 + x) as i16,
                (Y0 + y) as i16,
                if edge { FG } else { BG },
            );
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
            let glyph = font::glyph(u16::from(b));
            for (dy, bits) in glyph.iter().enumerate() {
                for dx in 0..CELL_W {
                    if bits & (0x80 >> dx) != 0 {
                        let (px, py) = (
                            X0 + MARGIN + col * CELL_W + dx,
                            Y0 + MARGIN + row * CELL_H + dy,
                        );
                        if px < X0 + w && py < Y0 + h {
                            VDI_SCREEN.put_device_pixel(px as i16, py as i16, FG);
                        }
                    }
                }
            }
        }
        col += 1;
    }
    VDI_SCREEN.flush_rect(
        X0 as i16,
        Y0 as i16,
        (X0 + w - 1) as i16,
        (Y0 + h - 1) as i16,
    );
    *BOX.lock() = (w, h);
    SHOWN.store(true, Ordering::Relaxed);
}

/// Hide the report: put the saved pixels back.
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
        VDI_SCREEN.flush_rect(
            X0 as i16,
            Y0 as i16,
            (X0 + w - 1) as i16,
            (Y0 + h - 1) as i16,
        );
    }
    SHOWN.store(false, Ordering::Relaxed);
}

/// Show the report if it is hidden, hide it if it is shown. The new state.
pub fn toggle() -> bool {
    if SHOWN.load(Ordering::Relaxed) {
        hide();
    } else {
        show();
    }
    SHOWN.load(Ordering::Relaxed)
}

/// One key byte from the decoder, bit 7 = break. `true` if it is the hot key
/// (make and break), which goes no further.
pub fn hotkey(raw: u8, shifty: u8) -> bool {
    let both = MODE_CTRL | MODE_ALT;
    if raw & 0x7F != KEY_F9 || shifty & both != both {
        return false;
    }
    if raw & 0x80 == 0 {
        toggle();
    }
    true
}

/// A pass of the trap-side input pump: the first failure since the boot shows
/// the report, once. Here and not at the failure, so the cache counters of the
/// failing call are in it.
pub fn poll() {
    #[cfg(feature = "board-rpi4")]
    if crate::drivers::bcm2711::emmc_diag::take_first_failure() && !SHOWN.load(Ordering::Relaxed) {
        show();
    }
}
