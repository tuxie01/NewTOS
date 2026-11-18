//! `graf_growbox` (73) as the desktop's `do_wopen` calls it (DESK50).
//!
//! EmuTOS's desktop zooms from the icon a window or a program was opened from
//! out to the target rectangle: `do_wopen` (`references/emutos/desk/
//! desksupp.c:294-322`) calls `graf_growbox_grect(&d, &t)` (`:315`), whose
//! binding is `GR_I1..GR_I8` = from x/y/w/h, to x/y/w/h and
//! `AES_CTRL_CODE(GRAF_GROWBOX, 8, 1, 0)` (`desk/gembind.c:589-600`). The
//! NewTOS desktop gained that binding in `programs/desktop/src/lib.rs`
//! (`graf_growbox`) with exactly that shape; these tests hold the AES side of
//! the contract it relies on, through the same dispatcher the trap reaches:
//!
//! * the call answers 1 (`crysbind`'s `ret = TRUE`, `aes/gemsuper.c:85`, which
//!   the `GRAF_GROWBOX` arm `:275-277` leaves standing);
//! * the outline starts **at the source** rectangle and ends **at the target**,
//!   growing in between — the zoom goes from the icon to the window, not the
//!   other way round (that is `graf_shrinkbox`);
//! * every outline is drawn in XOR and taken off again by a second identical
//!   XOR (`gsx_xbox`, `aes/gemgraf.c:274-278`), so nothing is left on screen.
//!
//! The kernel's rasterizer cannot run on the host, so the backend here only
//! **records** the line segments the AES asks for. An outline is the four
//! segments of `gsx_bxpts` (`aes/gemgraf.c:246-258`); the tests rebuild each
//! rectangle from them.
//!
//! The desktop half — which rectangles `do_wopen` passes, the deselect, the
//! `curr > 0` gate — runs only on the target (`kernel` is forced to
//! `aarch64-unknown-none-softfloat`) and is measured in QEMU
//! (`measurements/measurement-2026-10-01-desk50/`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const OP_GRAF_GROWBOX: i16 = 73;
const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

/// One `abline` request.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seg {
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    mode: WriteMode,
}

/// A backend that draws nothing and remembers every line segment.
struct RecBackend {
    segs: Mutex<Vec<Seg>>,
}

impl RasterBackend for RecBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        16
    }
    fn planes(&self) -> u16 {
        4
    }
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _palette: &[u32; 256]) {}
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
    #[allow(clippy::too_many_arguments)]
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        _pel: Pel,
        mode: WriteMode,
        _clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        self.segs.lock().unwrap().push(Seg { x0, y0, x1, y1, mode });
        mask
    }
    fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _fill: PatternFill, _clip: ClipRect) {}
    #[allow(clippy::too_many_arguments)]
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
    fn blit_mono(&self, _dst: Rect, _src: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _clip: ClipRect) {}
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

/// The host: the recording raster and a clock that advances on every read, so
/// the animation's per-frame dwell (`monotonic_millis() + 15`) terminates.
struct RecHost {
    raster: RecBackend,
    clock: AtomicU64,
}

impl Host for RecHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: every field is behind a Mutex or an atomic.
unsafe impl Sync for RecHost {}
unsafe impl Send for RecHost {}

static LOCK: Mutex<()> = Mutex::new(());
static HOST: std::sync::OnceLock<&'static RecHost> = std::sync::OnceLock::new();

fn host() -> &'static RecHost {
    HOST.get_or_init(|| {
        let h: &'static RecHost = Box::leak(Box::new(RecHost {
            raster: RecBackend { segs: Mutex::new(Vec::new()) },
            clock: AtomicU64::new(0),
        }));
        let dynref: &'static &'static dyn Host = Box::leak(Box::new(h as &'static dyn Host));
        tos_api::host::install_host(dynref);
        h
    })
}

/// `(x, y, w, h)`.
type R = (i16, i16, i16, i16);

/// `graf_growbox(from, to)` laid exactly as `programs/desktop/src/lib.rs` lays it:
/// eight `int_in`, one `int_out`, no addresses. Returns `int_out[0]` and the
/// segments the AES drew.
fn growbox(from: R, to: R) -> (i16, Vec<Seg>) {
    let h = host();
    h.raster.segs.lock().unwrap().clear();
    let mut control = [0i16; 8];
    let global = [0i16; 16];
    let mut intin = [0i16; 16];
    let mut intout = [0x5A5Ai16; 8];
    let addrin = [0u64; 4];
    let addrout = [0u64; 4];
    control[0] = OP_GRAF_GROWBOX;
    control[1] = 8;
    control[2] = 1;
    control[3] = 0;
    intin[..8].copy_from_slice(&[from.0, from.1, from.2, from.3, to.0, to.1, to.2, to.3]);
    let block: [u64; 6] = [
        control.as_ptr() as u64,
        global.as_ptr() as u64,
        intin.as_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_ptr() as u64,
        addrout.as_ptr() as u64,
    ];
    // SAFETY: the six arrays outlive the call and are at least as long as the
    // control words declare.
    unsafe { tos_api::aes::dispatch(block.as_ptr() as u64, Abi::Lp64) };
    let segs = h.raster.segs.lock().unwrap().clone();
    (intout[0], segs)
}

/// Rebuild the outlines from the segment stream: four segments per box, in
/// `gsx_bxpts` order — top edge left→right, then right edge top→bottom.
fn outlines(segs: &[Seg]) -> Vec<R> {
    assert_eq!(segs.len() % 4, 0, "an outline is four segments: {segs:?}");
    segs.chunks(4)
        .map(|c| {
            let (top, right, bottom, left) = (c[0], c[1], c[2], c[3]);
            // The four corners must close into one rectangle.
            assert_eq!((top.x1, top.y1), (right.x0, right.y0));
            assert_eq!((right.x1, right.y1), (bottom.x0, bottom.y0));
            assert_eq!((bottom.x1, bottom.y1), (left.x0, left.y0));
            assert_eq!((left.x1, left.y1), (top.x0, top.y0));
            (top.x0, top.y0, top.x1 - top.x0 + 1, right.y1 - right.y0 + 1)
        })
        .collect()
}

/// Every outline is shown and taken off again by the very same XOR box,
/// frame after frame: `r0 r0 r1 r1 … rn rn`. Returns the frames `r0..rn`.
fn frames(segs: &[Seg]) -> Vec<R> {
    assert!(segs.iter().all(|s| s.mode == WriteMode::Xor), "growbox draws in XOR only: {segs:?}");
    let boxes = outlines(segs);
    assert_eq!(boxes.len() % 2, 0, "each outline is drawn and erased: {boxes:?}");
    boxes
        .chunks(2)
        .map(|p| {
            assert_eq!(p[0], p[1], "an outline is erased before the next is drawn: {boxes:?}");
            p[0]
        })
        .collect()
}

/// The `do_diropen` case: a drive icon zooms out to the new window's frame.
#[test]
fn desk_binding_zooms_from_the_icon_out_to_the_window() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let icon = (16, 40, 64, 40);
    let window = (80, 40, 400, 300);
    let (ret, segs) = growbox(icon, window);
    assert_eq!(ret, 1, "graf_growbox answers 1");

    let f = frames(&segs);
    assert!(f.len() >= 2, "an animation, not a single box: {f:?}");
    assert_eq!(f.first(), Some(&icon), "the zoom starts at the source icon");
    assert_eq!(f.last(), Some(&window), "the zoom ends at the window rectangle");
    // It grows: no frame is smaller than the one before it.
    for w in f.windows(2) {
        assert!(w[1].2 >= w[0].2 && w[1].3 >= w[0].3, "growing, not shrinking: {f:?}");
    }
}

/// The `pro_run` case (`do_wopen(FALSE, wh, curr, &G.g_desk)`): the target is
/// the whole desktop, which reaches the screen's right and bottom edges, and
/// the source may be an object with no extent (`get_xywh` of an empty
/// object). The call still answers 1, the degenerate frames draw nothing, and
/// the zoom still lands on the desktop rectangle with nothing left behind.
#[test]
fn desk_binding_zooms_to_the_full_desktop_from_an_empty_source() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let empty = (300, 200, 0, 0);
    let desk = (0, 19, SCREEN_W as i16, SCREEN_H as i16 - 19);
    let (ret, segs) = growbox(empty, desk);
    assert_eq!(ret, 1, "graf_growbox answers 1");

    let f = frames(&segs);
    assert!(!f.is_empty(), "the non-empty frames are drawn");
    assert!(f.iter().all(|r| r.2 > 0 && r.3 > 0), "no zero-sized outline is drawn: {f:?}");
    assert_eq!(f.last(), Some(&desk), "the zoom ends at the desktop rectangle");
    let right = desk.0 + desk.2 - 1;
    let bottom = desk.1 + desk.3 - 1;
    assert_eq!((right, bottom), (SCREEN_W as i16 - 1, SCREEN_H as i16 - 1));
}
