//! VDI seed flood-fill — `v_contourfill` (opcode 103). Spec:
//! `docs/api/vdi/output/_fill.md`.
//!
//! Scanline flood fill from a seed pixel, bounded to the workstation clip
//! rectangle, using the current interior fill pattern. Two modes
//! (EmuTOS `seed_type`):
//! - `INTIN[0] < 0`: fill the connected region **equal** to the seed
//!   colour (`seed_type = 1`).
//! - `INTIN[0] >= 0`: fill the connected region **not equal** to that
//!   boundary colour (`seed_type = 0`), i.e. flood until the boundary.
//!
//! ## The seed queue (EmuTOS `contourfill`, `vdi_fill.c:873-1062`)
//!
//! The fill is EmuTOS' span seed-fill, rebuilt step for step because its
//! seed order is observable: every span is drawn the moment it is taken
//! from the queue, the span search reads the **live** screen (so a span
//! drawn earlier — through a pattern, in XOR — changes what later searches
//! see), and the queue is bounded.
//!
//! - A queue entry is a [`Segment`]: a span `xleft..=xright` on row
//!   `y & 0x7FFF`, with [`DOWN_FLAG`] saying which neighbour row it
//!   explores once drawn (set: `y + 1`, clear: `y - 1`).
//! - A span is a maximal run of **one** screen colour (`end_pts`,
//!   `vdi_fill.c:836-866`), fillable or not by the seed predicate.
//! - `get_seed` (`:888-942`) queues a fillable span, unless the queue holds
//!   the same row and left end going the other way — then that entry is
//!   removed and the span drawn at once (the two searches met). Freed
//!   entries are holes, reused by the next seed; `crunch_queue` lowers the
//!   top over trailing holes.
//! - The queue holds [`QSIZE`] entries. A seed that finds no hole and no
//!   room ends the whole fill on the spot (`:929-932`): what is drawn stays
//!   drawn, the rest is not filled.
//!
//! ## Emergency brake (TASK/V38, NewTOS only)
//!
//! EmuTOS' fill is not proven to end, and it does not always: a fill whose
//! spans stop changing what the search reads (it paints the region's own
//! colour, or red up to a boundary once every span is red) can repeat its
//! queue state for ever — the smallest found is a 5x3 clip with two dots
//! (`tests::dots_5x3`). A VDI trap runs with IRQs masked, so such a fill
//! would stop the machine. NewTOS ends a fill after [`BRAKE_SPANS_PER_PIXEL`]
//! taken spans per screen pixel and returns as from a finished one: what is
//! drawn stays drawn. A fill that ends by itself takes at most one span
//! per pixel in every real case measured, so it never meets the brake
//! (sanctioned deviation, `docs/COMPATIBILITY.md`).
//!
//! The previous NewTOS fill (a 4096-entry point stack with a visited
//! bitmap, 1197d493) dropped seeds silently once the stack was full and
//! left pockets of a dot lattice unfilled where EmuTOS fills them
//! (VDI-DRAW-008, `tosdump/cases/vdi-v18.tsv` 109/154/199).
//!
//! ## Colour model
//!
//! The comparison follows the device's colour model, as EmuTOS' `end_pts`
//! dispatches on `TRUECOLOR_MODE` (`vdi_fill.c:836-849`):
//!
//! - **Truecolor** (`v_planes > 8`): compare **16-bit screen words** with
//!   `OVERLAY_BIT` masked out of the screen side (`end_pts16`/
//!   `search_to_right16`/`search_to_left16`, `vdi_fill.c:694-767`). The
//!   masking is what makes a `v_clrwk`-cleared background (`0xFFFF`) and a
//!   pen-0 fill (`0xFFDF`, the pseudo-palette's white) the same colour.
//! - **Palette** (`v_planes <= 8`): compare colour indices, as the planar
//!   `end_pts` does (`vdi_fill.c:851-866`).
//!
//! Every screen read goes through [`pixelread`], the one function
//! `v_get_pixel` uses as well (EmuTOS reads both through the same
//! addressing, `vdi_fill.c:671-690,746-767`).

use core::cell::UnsafeCell;

use super::backend::{ClipRect, PatternFill, RasterBackend, WriteMode};
use super::fill_patterns;
use super::primitives::MAX_VERTS;
use super::vwk::{self, Vwk};
use super::Vdipb;

/// The Falcon 16-bit overlay bit — bit 5 of the `RRRRRGGGGGOBBBBB` screen
/// word, i.e. the green LSB the VDI pseudo-palette never sets
/// (`vdi_fill.c:26`, `vdi_defs.h:156`, `vdi_col.c:599`).
const OVERLAY_BIT: u16 = 0x0020;

/// `Segment::y` of a free queue entry (`vdi_fill.c:29`). EmuTOS compares
/// the `WORD` against `0xffff` under `-mshort` (16-bit `int`), i.e. as the
/// 16-bit pattern; so does this `u16`.
const EMPTY: u16 = 0xFFFF;

/// `Segment::y` flag: the entry explores the row below (`vdi_fill.c:30`).
const DOWN_FLAG: u16 = 0x8000;

/// The row of a `Segment::y` value, `DOWN_FLAG` stripped (`ABS`,
/// `vdi_fill.c:31`).
#[inline]
fn row(y: u16) -> i16 {
    (y & 0x7FFF) as i16
}

/// The queue's capacity, EmuTOS `QSIZE` = `sizeof(struct vsmain) /
/// sizeof(SEGMENT)` (`include/vdiext.h:33`): the queue shares `vdishare`
/// with `local_ptsin[2*MAX_VERTICES]` and `fill_buffer[MAX_VERTICES]`
/// (`:47-51`), 3 × `MAX_VERTICES` WORDs, and a `SEGMENT` is three WORDs
/// (`:17-21`). At the reference build's `MAX_VERTICES = 1024`
/// ([`MAX_VERTS`]) that is 1024 entries. The capacity is observable: it
/// decides where an over-complex fill stops.
pub(super) const QSIZE: usize = (2 * MAX_VERTS + MAX_VERTS) / 3;

/// The emergency brake: a fill ends after this many taken spans per screen
/// pixel (TASK/V38). Not EmuTOS, which loops on (see the module doc).
///
/// The bound comes from the geometry. A span is at least one pixel, so a
/// fill that draws every span once takes at most one span per screen pixel;
/// the largest real fills measured stay at or below that — the whole
/// 1920x1080 screen 1080 spans, a serpentine maze of one-pixel corridors
/// over it 1,035,841 (0.5 per pixel), a comb of one-pixel teeth 0.5 per
/// pixel (`tests::largest_real_fills_stay_far_below_the_brake`). Sixteen per
/// pixel is sixteen times that: 33,177,600 spans at 1920x1080, 4,915,200 at
/// 640x480. The screen, not the clip, sets it, so a fill confined to a
/// small window has the same headroom.
pub(super) const BRAKE_SPANS_PER_PIXEL: u64 = 16;

/// One queue entry, EmuTOS `SEGMENT` (`include/vdiext.h:17-21`).
#[derive(Clone, Copy)]
struct Segment {
    /// Row plus [`DOWN_FLAG`], or [`EMPTY`].
    y: u16,
    xleft: i16,
    xright: i16,
}

struct SeedQueue(UnsafeCell<[Segment; QSIZE]>);
// SAFETY: accessed only from the single-CPU, IRQ-off VDI trap path, like
// the rest of the VDI mutable state (see `vwk::TABLE`).
unsafe impl Sync for SeedQueue {}

/// The seed queue's storage, EmuTOS `vdishare.queue` (`include/vdiext.h:53`).
/// A static, not a stack array: VDI traps run on the 16 KiB `SP_EL1`
/// exception stack (see `primitives::PolyScratch`).
static QUEUE: SeedQueue = SeedQueue(UnsafeCell::new([Segment { y: EMPTY, xleft: 0, xright: 0 }; QSIZE]));

/// The screen colour at `(x, y)`, EmuTOS `pixelread` (`vdi_fill.c:671-690`):
/// under `TRUECOLOR_MODE` the 16-bit screen word **as stored** (overlay bit
/// included, `:675-681`), on a palette screen the colour index composed
/// from the bit-planes (`:684-689`).
///
/// NewTOS' framebuffer stores 32-bpp XRGB8888 internally
/// (`docs/COMPATIBILITY.md`), so the app-visible word is recovered through
/// `device_to_rgb565`, the same screen→16-bit boundary a memory MFDB
/// crosses — *not* through `read_pixel`, whose reverse map to the nearest
/// CLUT entry is lossy and would fold every non-pen colour onto a pen.
///
/// `v_get_pixel` and `v_contourfill` both read through here, as EmuTOS'
/// `vdi_v_get_pixel` (`:1102`) and `end_pts` (`:746-767`, `:836-866`) read
/// through the same addressing.
///
/// No bounds check, as in EmuTOS: `(x, y)` is the pixel at store index
/// `y * stride + x` — past the right edge the next row, left of 0 the
/// previous one (`get_start_addr16`, `vdi/vdi_inline.h:14-17`; planar
/// `get_start_addr`, `vdi/vdi_misc.c:166-177`, the same linear walk over
/// 16-pixel groups). Only an index outside the logical screen's store reads
/// 0, where EmuTOS reads foreign memory (user decision B, TASK/V24;
/// `docs/COMPATIBILITY.md`). `v_contourfill` reads inside its search
/// bounds only, so the run-on never reaches it.
pub(super) fn pixelread(backend: &dyn RasterBackend, x: i16, y: i16) -> u16 {
    if backend.planes() > 8 {
        backend.device_to_rgb565(backend.get_device_pixel_wrapped(x, y))
    } else {
        // A palette backend reports PELs per `(x, y)` only; its store is
        // taken as `width` pixels per row, which is `v_lin_wr` of a planar
        // screen (16-pixel groups, `width` a multiple of 16).
        let w = backend.width() as u32;
        let store = super::line::LineTarget { width: w, height: backend.height() as u32, stride: w };
        match store.index(x as i32, y as i32) {
            Some(idx) => backend.read_pixel((idx % w as usize) as i16, (idx / w as usize) as i16) as u16,
            None => 0,
        }
    }
}

/// Why a fill stopped before its queue ran empty.
#[derive(Debug, PartialEq, Eq)]
enum Stop {
    /// A full queue: EmuTOS `get_seed` returns `-1` and `contourfill`
    /// quits (`vdi_fill.c:929-932,1009,1016,1023,1030`).
    Overflow,
    /// The emergency brake: [`BRAKE_SPANS_PER_PIXEL`] spans per screen
    /// pixel taken (NewTOS only, TASK/V38).
    Brake,
}

/// One `contourfill` call's state: EmuTOS' file-scope `search_color`,
/// `seed_type`, `qbottom`/`qtop`/`qptr` (`vdi_fill.c:34-41`; `qbottom` is
/// always index 0 here) plus the draw attributes.
struct Flood<'a> {
    backend: &'a dyn RasterBackend,
    /// The search bounds (`VDI_CLIP(vwk)`), already cut to the device.
    bounds: ClipRect,
    /// The workstation clip, passed to the span draw.
    clip: ClipRect,
    truecolor: bool,
    search_color: u16,
    seed_type: bool,
    fill: PatternFill<'a>,
    queue: &'a mut [Segment; QSIZE],
    /// One above the highest entry in use.
    qtop: usize,
    /// The entry the next `pop` starts its search at.
    qptr: usize,
    /// Spans the fill may still take before the brake ends it.
    budget: u64,
}

impl<'a> Flood<'a> {
    /// A fill inside `bounds` (the clip cut to the device), drawing through
    /// `clip`, with the colour model of `backend` (truecolor for
    /// `v_planes > 8`, `vdi_defs.h:22`).
    fn new(
        backend: &'a dyn RasterBackend,
        bounds: ClipRect,
        clip: ClipRect,
        search_color: u16,
        seed_type: bool,
        fill: PatternFill<'a>,
        queue: &'a mut [Segment; QSIZE],
    ) -> Flood<'a> {
        Flood {
            backend,
            bounds,
            clip,
            truecolor: backend.planes() > 8,
            search_color,
            seed_type,
            fill,
            queue,
            qtop: 0,
            qptr: 0,
            budget: BRAKE_SPANS_PER_PIXEL * backend.width() as u64 * backend.height() as u64,
        }
    }

    /// Fill from the seed `(sx, sy)`, which lies inside `bounds`: nothing
    /// unless the seed's own span is fillable (`vdi_fill.c:990-991`), else
    /// the seed queue's main loop.
    fn fill_from(&mut self, sx: i16, sy: i16) -> Result<(), Stop> {
        let Some((oldxleft, oldxright, true)) = self.end_pts(sx, sy) else {
            return Ok(());
        };
        self.run(sy as u16, oldxleft, oldxright)
    }

    /// The colour `end_pts` compares: the screen word with `OVERLAY_BIT`
    /// off under truecolor (`vdi_fill.c:709,733,755`), the index otherwise.
    #[inline]
    fn color(&self, x: i16, y: i16) -> u16 {
        let c = pixelread(self.backend, x, y);
        if self.truecolor {
            c & !OVERLAY_BIT
        } else {
            c
        }
    }

    /// EmuTOS `end_pts` (`vdi_fill.c:836-866`, truecolor `end_pts16`
    /// `:746-767`): the run of `(x, y)`'s colour on row `y`, bounded by the
    /// clip's x range, and whether it is fillable (`seed_type` for a run of
    /// `search_color`, `!seed_type` otherwise). `None` for a row outside
    /// the clip's y range, where EmuTOS returns 0 without writing the ends.
    fn end_pts(&self, x: i16, y: i16) -> Option<(i16, i16, bool)> {
        if y < self.bounds.ymn || y > self.bounds.ymx {
            return None;
        }
        let color = self.color(x, y);
        let mut xr = x;
        while xr < self.bounds.xmx && self.color(xr + 1, y) == color {
            xr += 1;
        }
        let mut xl = x;
        while xl > self.bounds.xmn && self.color(xl - 1, y) == color {
            xl -= 1;
        }
        let fillable = if color == self.search_color { self.seed_type } else { !self.seed_type };
        Some((xl, xr, fillable))
    }

    /// Draw one span with the fill attributes, EmuTOS `draw_rect_common`
    /// on a one-row rectangle (`vdi_fill.c:910-916,1050-1057`).
    fn draw_span(&self, xl: i16, xr: i16, y: i16) {
        self.backend.fill_pattern(xl, y, xr - xl + 1, 1, self.fill, self.clip);
    }

    /// EmuTOS `crunch_queue` (`vdi_fill.c:875-881`): lower `qtop` over
    /// trailing holes; restart `qptr` at the bottom if it is now past the top.
    fn crunch_queue(&mut self) {
        while self.qtop > 0 && self.queue[self.qtop - 1].y == EMPTY {
            self.qtop -= 1;
        }
        if self.qptr >= self.qtop {
            self.qptr = 0;
        }
    }

    /// EmuTOS `get_seed` (`vdi_fill.c:888-942`). Finds the span at
    /// `(xin, row(yin))` and writes its ends to `xleftout`/`xrightout`
    /// (left untouched for a row outside the clip, as in EmuTOS). A fillable
    /// span either meets an opposite-direction entry with the same row and
    /// left end — that entry is freed and the span drawn now — or is queued
    /// in the first hole, else on top. `Ok(true)` = a seed was queued.
    fn get_seed(&mut self, xin: i16, yin: u16, xleftout: &mut i16, xrightout: &mut i16) -> Result<bool, Stop> {
        let Some((xl, xr, fillable)) = self.end_pts(xin, row(yin)) else {
            return Ok(false);
        };
        *xleftout = xl;
        *xrightout = xr;
        if !fillable {
            return Ok(false);
        }

        let mut qhole = None;
        for i in 0..self.qtop {
            let y = self.queue[i].y;
            if y == EMPTY {
                // Skip holes, remembering the first one.
                if qhole.is_none() {
                    qhole = Some(i);
                }
                continue;
            }
            if (y ^ DOWN_FLAG) == yin && self.queue[i].xleft == xl {
                // Ran into the opposite search: remove it and fill the line.
                self.draw_span(xl, xr, row(yin));
                self.queue[i].y = EMPTY;
                if i + 1 == self.qtop {
                    self.crunch_queue();
                }
                return Ok(false);
            }
        }

        let slot = match qhole {
            Some(i) => i,
            None => {
                // No hole: raise `qtop` if there is room, else give up.
                if self.qtop >= QSIZE {
                    return Err(Stop::Overflow);
                }
                self.qtop += 1;
                self.qtop - 1
            }
        };
        self.queue[slot] = Segment { y: yin, xleft: xl, xright: xr };
        Ok(true)
    }

    /// The main loop of EmuTOS `contourfill` (`vdi_fill.c:1003-1062`),
    /// started with the seed span `(oldy, oldxleft..=oldxright)`; `oldy`
    /// carries no `DOWN_FLAG`, so the seed row is searched upwards first
    /// and then, once its queued downward copy is taken, downwards.
    fn run(&mut self, mut oldy: u16, mut oldxleft: i16, mut oldxright: i16) -> Result<(), Stop> {
        self.queue[0] = Segment { y: oldy | DOWN_FLAG, xleft: oldxleft, xright: oldxright };
        self.qptr = 0;
        self.qtop = 1;

        loop {
            let next = if oldy & DOWN_FLAG != 0 { oldy.wrapping_add(1) } else { oldy.wrapping_sub(1) };
            let ny = row(next);
            // A row outside the clip yields no span and writes no ends
            // (`end_pts`, `vdi_fill.c:843-844`), so every `get_seed` on it below
            // returns 0 and the block has no effect: skipped as a whole.
            if ny >= self.bounds.ymn && ny <= self.bounds.ymx {
                let (mut newxleft, mut newxright) = (0, 0);
                let (mut xleft, mut xright) = (0, 0);
                let mut gotseed = self.get_seed(oldxleft, next, &mut newxleft, &mut newxright)?;

                // The new span sticks out left by two or more: seed the
                // spans of this row beyond ours, searching back
                // (`oldy ^ DOWN_FLAG`).
                if newxleft < oldxleft - 1 && gotseed {
                    xleft = oldxleft;
                    while xleft > newxleft {
                        xleft -= 1;
                        self.get_seed(xleft, oldy ^ DOWN_FLAG, &mut xleft, &mut xright)?;
                    }
                }
                // The further spans of the next row under ours.
                while newxright < oldxright {
                    newxright += 1;
                    gotseed = self.get_seed(newxright, next, &mut xleft, &mut newxright)?;
                }
                // The last of them sticks out right by two or more.
                if newxright > oldxright + 1 && gotseed {
                    xright = oldxright;
                    while xright < newxright {
                        xright += 1;
                        self.get_seed(xright, oldy ^ DOWN_FLAG, &mut xleft, &mut xright)?;
                    }
                }
            }

            // Eventually jump out here.
            if self.qtop == 0 {
                return Ok(());
            }
            // The emergency brake (NewTOS only): checked before a span is
            // taken, so a fill that empties its queue within the budget
            // runs exactly as in EmuTOS.
            if self.budget == 0 {
                return Err(Stop::Brake);
            }
            self.budget -= 1;

            // Take the next entry, scanning circularly from `qptr`.
            while self.queue[self.qptr].y == EMPTY {
                self.qptr += 1;
                if self.qptr == self.qtop {
                    self.qptr = 0;
                }
            }
            let seg = self.queue[self.qptr];
            oldy = seg.y;
            oldxleft = seg.xleft;
            oldxright = seg.xright;
            self.queue[self.qptr].y = EMPTY;
            self.qptr += 1;
            if self.qptr == self.qtop {
                self.crunch_queue();
            }

            self.draw_span(oldxleft, oldxright, row(oldy));
            // `SEEDABORT` is `no_abort` for the VDI (`vdi_fill.c:1076-1090`), so no early abort.
        }
    }
}

/// Opcode 103 — `v_contourfill`.
pub fn v_contourfill(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let sx = pb.ptsin(0);
    let sy = pb.ptsin(1);

    // Search bounds: the workstation clip rectangle (`VDI_CLIP(vwk)`)
    // intersected with the **live** device geometry, so no read or draw
    // leaves the framebuffer. `vs_clip` already keeps the clip inside
    // `0..=xres`/`0..=yres` (`vdi_control.c:241-249`).
    let clip = vwk.clip_rect();
    let bounds = ClipRect {
        xmn: clip.xmn.max(0),
        ymn: clip.ymn.max(0),
        xmx: clip.xmx.min(backend.width().saturating_sub(1) as i16),
        ymx: clip.ymx.min(backend.height().saturating_sub(1) as i16),
    };
    if bounds.xmn > bounds.xmx || bounds.ymn > bounds.ymx {
        return;
    }
    // Seed-in-clip guard (silent no-op, `vdi_fill.c:965-967`).
    if sx < bounds.xmn || sx > bounds.xmx || sy < bounds.ymn || sy > bounds.ymx {
        return;
    }

    // Under `TRUECOLOR_MODE` (`v_planes > 8`, `vdi_defs.h:22`) the seed
    // fill works on raw 16-bit screen words, not palette indices —
    // `end_pts` dispatches to `end_pts16` (`vdi_fill.c:846-849`).
    let truecolor = backend.planes() > 8;

    // Determine the search colour + seed mode (`contourfill`,
    // `vdi_fill.c:970-986`).
    let intin0 = pb.intin(0);
    let (search_color, seed_type) = if intin0 < 0 {
        // Fill the region equal to the seed pixel: `search_color =
        // pixelread(xleft, oldy)` (`vdi_fill.c:972`). EmuTOS takes the
        // **unmasked** word here and `end_pts16` compares it against a
        // *masked* screen colour (`:763`). So a seed pixel whose overlay
        // bit is set (a `v_clrwk`-cleared background, `0xFFFF`) never
        // equals any masked screen word and the fill draws nothing. The
        // asymmetry is observable on NewTOS as on a Falcon since the kernel
        // keeps `OVERLAY_BIT` in the device pixel's X byte (VDI-OVL-001,
        // closed by V11/V12, 2026-09-21).
        (pixelread(backend, sx, sy), true)
    } else {
        // `>= numcolors` guard (silent no-op, `vdi_fill.c:976-977`).
        if intin0 as u16 >= backend.num_colors() {
            return;
        }
        // Convert the VDI pen to a device colour: `search_color =
        // MAP_COL[search_color]` and, under truecolor, the physical
        // workstation's pseudo-palette entry — `phys_work.ext->palette[...]`
        // (`vdi_fill.c:978-985`), the *physical* workstation's table, not the
        // calling one's. NewTOS folds `MAP_COL` to the identity
        // (`vwk::VwkExt`), so the pen indexes the pseudo-palette directly.
        let c = if truecolor {
            // SAFETY: single trap path, as for the rest of the VDI state.
            let phys = unsafe { vwk::ext(vwk::PHYS_HANDLE) };
            let Some(word) = phys.and_then(|e| e.palette.get(intin0 as usize).copied()) else {
                return; // pen beyond the pseudo-palette → no such colour
            };
            word
        } else {
            intin0 as u16
        };
        (c, false) // fill region bounded by this colour
    };

    // HOLLOW is not a no-op: EmuTOS `st_fl_ptr` points `patptr` at the
    // all-zero HOLLOW word, so each filled span is drawn through the write
    // mode (Replace → pen 0).
    let pattern = fill_patterns::select(vwk);
    let fill = PatternFill {
        fg: vwk.fill_color,
        bg: 0,
        pattern,
        mode: WriteMode::from_wrt_mode(vwk.wrt_mode),
    };

    // SAFETY: single trap path (see `SeedQueue`); no other borrow of the
    // queue is live, `v_contourfill` does not re-enter the VDI.
    let queue = unsafe { &mut *QUEUE.0.get() };
    // A full queue ends the fill where it stands (`vdi_fill.c:1009-1030`), and so
    // does the emergency brake (TASK/V38); there is no error channel.
    let _ = Flood::new(backend, bounds, clip, search_color, seed_type, fill, queue).fill_from(sx, sy);
}

#[cfg(test)]
mod tests {
    //! TASK/V38: the emergency brake. A memory screen of 16-bit words (the
    //! truecolor model, `planes() == 16`) whose `fill_pattern` paints, so the
    //! fill reads back what it drew; `draws` counts every span draw, and
    //! the harness guard `cap` ends a fill that would not stop with a panic
    //! instead of hanging the test run.
    use super::super::backend::{MonoSrc, Pel, Rect};
    use super::*;
    use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};

    /// Pens 0..3 as screen words: white (`0xFFDF`, the pseudo-palette's
    /// pen 0 without `OVERLAY_BIT`), black, red, green.
    const PENS: [u16; 4] = [0xFFDF, 0x0000, 0xF800, 0x07C0];
    const WHITE: u16 = PENS[0];
    const BLACK: u16 = PENS[1];

    /// The brake the tests expect: 16 spans per screen pixel.
    const SPANS_PER_PIXEL: u64 = 16;

    struct Canvas {
        w: u16,
        h: u16,
        px: Vec<AtomicU16>,
        /// The read-back never shows a draw (an artificial screen).
        frozen: bool,
        draws: AtomicU64,
        cap: u64,
    }

    impl Canvas {
        fn new(w: u16, h: u16, frozen: bool, cap: u64) -> Canvas {
            let px = (0..w as usize * h as usize).map(|_| AtomicU16::new(WHITE)).collect();
            Canvas { w, h, px, frozen, draws: AtomicU64::new(0), cap }
        }
        fn set(&self, x: i32, y: i32, c: u16) {
            self.px[y as usize * self.w as usize + x as usize].store(c, Ordering::Relaxed);
        }
        fn get(&self, x: i32, y: i32) -> u16 {
            self.px[y as usize * self.w as usize + x as usize].load(Ordering::Relaxed)
        }
        fn draws(&self) -> u64 {
            self.draws.load(Ordering::Relaxed)
        }
        fn count(&self, c: u16) -> usize {
            self.px.iter().filter(|p| p.load(Ordering::Relaxed) == c).count()
        }
    }

    impl RasterBackend for Canvas {
        fn width(&self) -> u16 {
            self.w
        }
        fn height(&self) -> u16 {
            self.h
        }
        fn num_colors(&self) -> u16 {
            256
        }
        fn planes(&self) -> u16 {
            16
        }
        fn set_color(&self, _: u16, _: u16, _: u16, _: u16) {}
        fn query_color(&self, _: u16) -> (u16, u16, u16) {
            (0, 0, 0)
        }
        fn set_active_palette(&self, _: &[u16; 256]) {}
        fn pel_to_device16(&self, pel: Pel) -> u16 {
            PENS[pel as usize & 3]
        }
        fn device_to_rgb565(&self, argb: u32) -> u16 {
            argb as u16
        }
        fn rgb565_to_device(&self, px: u16) -> u32 {
            px as u32
        }
        fn read_pixel(&self, _: i16, _: i16) -> Pel {
            0
        }
        fn fill_rect(&self, _: i16, _: i16, _: i16, _: i16, _: Pel, _: ClipRect) {}
        fn abline(&self, _: i16, _: i16, _: i16, _: i16, _: Pel, _: WriteMode, _: ClipRect, mask: u16, _: bool) -> u16 {
            mask
        }
        fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, f: PatternFill, clip: ClipRect) {
            let n = self.draws.fetch_add(1, Ordering::Relaxed) + 1;
            assert!(n <= self.cap, "harness guard: the fill has not ended after {} span draws", self.cap);
            assert_eq!(f.mode, WriteMode::Replace, "the tests fill in replace mode only");
            if self.frozen {
                return;
            }
            let (fg, bg) = (PENS[f.fg as usize & 3], PENS[f.bg as usize & 3]);
            for yy in y as i32..y as i32 + h as i32 {
                for xx in x as i32..x as i32 + w as i32 {
                    let in_clip = (clip.xmn as i32..=clip.xmx as i32).contains(&xx)
                        && (clip.ymn as i32..=clip.ymx as i32).contains(&yy);
                    if !in_clip || xx < 0 || yy < 0 || xx >= self.w as i32 || yy >= self.h as i32 {
                        continue;
                    }
                    // Replace mode: pen `fg` on set pattern bits, `bg` on clear ones.
                    let set = f.pattern[yy as usize & (f.pattern.len() - 1)] & (0x8000 >> (xx & 15)) != 0;
                    self.set(xx, yy, if set { fg } else { bg });
                }
            }
        }
        fn draw_glyph_sized(&self, _: i16, _: i16, _: &[u8], _: i16, _: i16, _: Pel, _: WriteMode, _: ClipRect) {}
        fn blit_mono(&self, _: Rect, _: MonoSrc, _: Pel, _: Pel, _: WriteMode, _: ClipRect) {}
        fn copy_rect(&self, _: Rect, _: i16, _: i16, _: u8, _: ClipRect) {}
        fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
            if x < 0 || y < 0 || x as u16 >= self.w || y as u16 >= self.h {
                return 0;
            }
            self.get(x as i32, y as i32) as u32
        }
        fn put_device_pixel(&self, _: i16, _: i16, _: u32) {}
        fn flush_rect(&self, _: i16, _: i16, _: i16, _: i16) {}
    }

    static SOLID: [u16; 1] = [0xFFFF];

    /// The fill `v_contourfill` runs once the seed, the clip and the colour
    /// are checked: solid, replace mode, pen `fill_color`, `search_color`
    /// a screen word, `seed_type` as in EmuTOS (true = fill-while-same,
    /// `INTIN[0] < 0`), in a queue of its own.
    fn flood(c: &Canvas, x: i16, y: i16, search_color: u16, seed_type: bool, fill_color: Pel, clip: ClipRect) {
        let mut queue = Box::new([Segment { y: EMPTY, xleft: 0, xright: 0 }; QSIZE]);
        let fill = PatternFill { fg: fill_color, bg: 0, pattern: &SOLID, mode: WriteMode::Replace };
        let _ = Flood::new(c, clip, clip, search_color, seed_type, fill, &mut queue).fill_from(x, y);
    }

    /// The smallest never-ending input found (TASK/V38): a 5x3 clip with
    /// black dots at (3,1) and (1,2), seed (0,2). The fill circles the dot
    /// at (3,1); once nothing on the screen changes any more, the queue
    /// repeats a cycle of 4 taken spans. EmuTOS' model does the same
    /// (`measurements/measurement-2026-11-07-v38/RUNINFO.TXT`).
    fn dots_5x3(c: &Canvas, x0: i32, y0: i32) {
        c.set(x0 + 3, y0 + 1, BLACK);
        c.set(x0 + 1, y0 + 2, BLACK);
    }

    #[test]
    fn brake_ends_a_fill_that_never_ends() {
        // A real input: fill-while-same (INTIN[0] = -1) with pen 0, so the
        // fill paints the region's own colour and never sees its work.
        let budget = SPANS_PER_PIXEL * 5 * 3;
        let c = Canvas::new(5, 3, false, 100 * budget);
        dots_5x3(&c, 0, 0);
        flood(&c, 0, 2, WHITE, true, 0, ClipRect::full(5, 3));
        // The brake fires after `budget` taken spans; every pass may add
        // collision draws (`get_seed`), at most one per span searched.
        assert!(c.draws() >= budget, "stopped early: {} draws", c.draws());
        assert!(c.draws() <= 2 * budget, "{} draws for a budget of {budget}", c.draws());
        // What was drawn stays: white on white, the dots untouched.
        assert_eq!(c.count(BLACK), 2);
    }

    #[test]
    fn brake_ends_a_boundary_fill_that_never_ends() {
        // The same dots filled red up to the black boundary (INTIN[0] = 1):
        // a red span stays fillable, and once the region is red the fill
        // circles as on the white screen.
        let budget = SPANS_PER_PIXEL * 5 * 3;
        let c = Canvas::new(5, 3, false, 100 * budget);
        dots_5x3(&c, 0, 0);
        flood(&c, 0, 2, BLACK, false, 2, ClipRect::full(5, 3));
        assert!(c.draws() >= budget, "stopped early: {} draws", c.draws());
        assert!(c.draws() <= 2 * budget, "{} draws for a budget of {budget}", c.draws());
        // What was drawn stays drawn: the whole region red.
        assert_eq!((c.count(PENS[2]), c.count(BLACK)), (13, 2));
    }

    #[test]
    fn brake_counts_the_screen_not_the_clip() {
        // An artificial screen whose read-back never shows a draw: the
        // same dots in a 5x3 clip window of a 64x48 screen, filled red.
        // The brake is set by the screen (64 x 48), not by the window.
        let budget = SPANS_PER_PIXEL * 64 * 48;
        let c = Canvas::new(64, 48, true, 100 * budget);
        dots_5x3(&c, 10, 20);
        flood(&c, 10, 22, WHITE, true, 2, ClipRect { xmn: 10, ymn: 20, xmx: 14, ymx: 22 });
        assert!(c.draws() >= budget, "stopped early: {} draws", c.draws());
        assert!(c.draws() <= 2 * budget, "{} draws for a budget of {budget}", c.draws());
    }

    #[test]
    fn a_terminating_fill_of_the_same_dots_is_unchanged() {
        // The same 5x3 dots filled red while-same: drawn spans turn red and
        // stop being fillable, so the fill ends by itself as before.
        let c = Canvas::new(5, 3, false, 1000);
        dots_5x3(&c, 0, 0);
        flood(&c, 0, 2, WHITE, true, 2, ClipRect::full(5, 3));
        assert_eq!(c.count(PENS[2]), 13);
        assert!(c.draws() < 20, "{} draws", c.draws());
    }

    /// Fill a white region red and return the span draws: while-same
    /// (`INTIN[0] = -1`), or up to the black boundary (`INTIN[0] = 1`),
    /// where the red spans stay fillable and the fronts end by meeting.
    fn real_fill(c: &Canvas, x: i16, y: i16, to_boundary: bool) -> u64 {
        let (search, seed_type) = if to_boundary { (BLACK, false) } else { (WHITE, true) };
        flood(c, x, y, search, seed_type, 2, ClipRect::full(c.w, c.h));
        c.draws()
    }

    /// The screens of the largest real fills, all white regions bounded by
    /// black, filled red.
    #[derive(Clone, Copy, Debug)]
    enum Shape {
        /// The whole screen: one span per row.
        Full,
        /// Corridors one pixel wide between walls on every odd column,
        /// open at the bottom and the top in turn: every row of every
        /// corridor is a span of its own.
        Serpentine,
        /// A spine along the top row, teeth one pixel wide down every even
        /// column: all teeth are filled side by side, the queue holds one
        /// front per tooth.
        Comb,
        /// The V21 lattice (`vdi-v18.tsv` K8): 40 dotted rows (`0x8888`),
        /// rows 200..317 step 3, x 20..619.
        Lattice,
    }

    fn real_screen(shape: Shape, w: u16, h: u16) -> (Canvas, i16, i16) {
        let c = Canvas::new(w, h, false, w as u64 * h as u64);
        let (w, h) = (w as i32, h as i32);
        match shape {
            Shape::Full => return (c, (w / 2) as i16, (h / 2) as i16),
            Shape::Serpentine => {
                for x in (1..w).step_by(2) {
                    let gap = if (x / 2) % 2 == 0 { h - 1 } else { 0 };
                    (0..h).filter(|&y| y != gap).for_each(|y| c.set(x, y, BLACK));
                }
            }
            Shape::Comb => {
                for x in (1..w).step_by(2) {
                    (1..h).for_each(|y| c.set(x, y, BLACK));
                }
            }
            Shape::Lattice => {
                for y in (200..=317).step_by(3) {
                    (20..=619).filter(|x| 0x8888u16 & (0x8000 >> (x & 15)) != 0).for_each(|x| c.set(x, y, BLACK));
                }
                return (c, 320, 340);
            }
        }
        (c, 0, h as i16 - 1)
    }

    #[test]
    fn largest_real_fills_stay_far_below_the_brake() {
        // The comb's 320/960 fronts make every seed search scan the queue:
        // 640x480 keeps the test short; the serpentine and the full screen
        // run at 1920x1080.
        let cases = [
            (Shape::Full, 1920, 1080),
            (Shape::Serpentine, 1920, 1080),
            (Shape::Comb, 640, 480),
            (Shape::Lattice, 640, 480),
        ];
        for (shape, w, h) in cases {
            for to_boundary in [false, true] {
                let (c, x, y) = real_screen(shape, w, h);
                let spans = real_fill(&c, x, y, to_boundary);
                let area = w as u64 * h as u64;
                eprintln!("{shape:?} {w}x{h} to_boundary={to_boundary}: {spans} spans drawn, {area} pixels");
                if !matches!(shape, Shape::Lattice) {
                    assert_eq!(c.count(WHITE), 0, "{shape:?} not filled");
                }
                // At most one span per screen pixel: the brake (16 per
                // pixel) lies 16 times further out.
                assert!(spans <= area, "{shape:?}: {spans} spans on {area} pixels");
            }
        }
    }
}
