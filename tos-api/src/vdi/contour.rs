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

/// Signals a full queue: EmuTOS `get_seed` returns `-1` and `contourfill`
/// quits (`vdi_fill.c:929-932,1009,1016,1023,1030`).
struct Overflow;

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
}

impl Flood<'_> {
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
    fn get_seed(&mut self, xin: i16, yin: u16, xleftout: &mut i16, xrightout: &mut i16) -> Result<bool, Overflow> {
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
                    return Err(Overflow);
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
    fn run(&mut self, mut oldy: u16, mut oldxleft: i16, mut oldxright: i16) -> Result<(), Overflow> {
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
    let mut flood = Flood {
        backend,
        bounds,
        clip,
        truecolor,
        search_color,
        seed_type,
        fill,
        queue,
        qtop: 0,
        qptr: 0,
    };

    // Check if anything to do (`vdi_fill.c:990-991`): the seed's own span
    // must be fillable.
    let Some((oldxleft, oldxright, true)) = flood.end_pts(sx, sy) else {
        return;
    };
    // A full queue ends the fill where it stands (`vdi_fill.c:1009-1030`); there is
    // no error channel.
    let _ = flood.run(sy as u16, oldxleft, oldxright);
}
