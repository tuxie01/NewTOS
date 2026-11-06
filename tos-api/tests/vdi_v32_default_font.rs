//! V32 — the default system font follows `V_REZ_VT`: 8×8 on a screen of fewer
//! than 400 lines, 8×16 from 400 lines up — and everything EmuTOS derives from
//! it follows too.
//!
//! EmuTOS (`references/emutos`, pinned `1a7484ab`):
//!
//! * **VDI.** `text_init` (`vdi/vdi_text.c:592-662`, run by `v_opnwk`,
//!   `vdi_control.c:552`) picks `def_font`: `cell_height = (V_REZ_VT >= 400)
//!   ? 16 : 8` (`:612`), then the `F_DEFAULT` font of the ring whose
//!   `form_height` is that height (`:617-623`; `F_DEFAULT` is set on the 8×8
//!   and the 8×16 face, not on the 6×6, `bios/fnt_st_*.c`). `text_init2`
//!   (`:572-590`, from `init_wk` of both `v_opnwk` and `v_opnvwk`) makes it
//!   `cur_font`, which `vqt_attributes` reports (`:999-1021`).
//! * **AES.** `gsx_start` (`aes/gemgraf.c:405-415`) takes `gl_wchar`/`gl_hchar`
//!   from `vqt_attributes` on its workstation, `gl_hbox = gl_hchar + 3`,
//!   `gl_wbox = max(gl_wchar + 4, gl_hbox * ws_hpixel / ws_wpixel)`, with the
//!   pixel size of `get_std_pixel_size` (`bios/screen.c:872-876`); every IBM
//!   string the AES draws goes through the VDI in that font (`gsx_tblt`,
//!   `:500-527`).
//! * **Console.** `font_set_default` (`bios/font.c:72-88`): `(V_REZ_VT < 400)
//!   ? &fon8x8 : &fon8x16`, `v_cel_my = V_REZ_VT / form_height - 1`, which
//!   `vq_chcells` reports (`vdi/vdi_esc.c:62-67`).
//!
//! Measured (V18 part 2, VDI-TEXT-009): `vqt_attributes` right after
//! `v_opnvwk` at 640×240 — EmuTOS `PTSOUT` 7 6 8 8, NewTOS before V32 7 13 8
//! 16; at 640×480 both 7 13 8 16
//! (`measurements/measurement-2026-11-06-v18-t2/k19/emutos-240/VDPROBE.LOG`).
//!
//! Ticket: `TASK/V32.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

// ── a metadata backend that records the glyph cells drawn ────────────────

struct Profile {
    width: AtomicU16,
    height: AtomicU16,
    /// Every `draw_glyph_sized` cell `(w, h)` since the last [`Profile::take`].
    glyphs: Mutex<Vec<(i16, i16)>>,
}

impl Profile {
    fn take(&self) -> Vec<(i16, i16)> {
        core::mem::take(&mut *self.glyphs.lock().unwrap())
    }
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        self.width.load(Ordering::Relaxed)
    }
    fn height(&self) -> u16 {
        self.height.load(Ordering::Relaxed)
    }
    fn planes(&self) -> u16 {
        16
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {}
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, p: &[u16; 256]) {}
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
        assert_eq!(g.len(), h as usize, "one glyph row per cell row");
        self.glyphs.lock().unwrap().push((w, h));
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {}
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {}
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {}
}

struct TestHost {
    raster: Profile,
}

impl Host for TestHost {
    fn console_write_byte(&self, byte: u8) {
        tos_api::vdi::alpha_conout(&self.raster, byte);
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
}

static HOST: TestHost = TestHost {
    raster: Profile { width: AtomicU16::new(640), height: AtomicU16::new(480), glyphs: Mutex::new(Vec::new()) },
};
static HOST_REF: &dyn Host = &HOST;

/// The installed host, the VDI handle table and the AES state are
/// process-wide.
static SERIAL: Mutex<()> = Mutex::new(());
static INSTALL: OnceLock<()> = OnceLock::new();

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Switch the screen to `w`×`h` (the kernel takes over any VideoCore
/// geometry, `kernel/src/drivers/bcm2711/framebuffer.rs`).
fn screen(w: u16, h: u16) {
    INSTALL.get_or_init(|| tos_api::host::install_host(&HOST_REF));
    HOST.raster.width.store(w, Ordering::Relaxed);
    HOST.raster.height.store(h, Ordering::Relaxed);
    HOST.raster.take();
}

// ── VDI parameter block ──────────────────────────────────────────────────

struct Vpb {
    contrl: Box<[i16; 16]>,
    intin: Box<[i16; 32]>,
    ptsin: Box<[i16; 32]>,
    intout: Box<[i16; 64]>,
    ptsout: Box<[i16; 32]>,
    block: Box<[u64; 5]>,
}

impl Vpb {
    fn new() -> Vpb {
        let mut pb = Vpb {
            contrl: Box::new([0; 16]),
            intin: Box::new([0; 32]),
            ptsin: Box::new([0; 32]),
            intout: Box::new([0; 64]),
            ptsout: Box::new([0; 32]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    fn op(&mut self, opcode: i16, sub: i16, n_pts: i16, n_int: i16, handle: i16) {
        self.contrl.fill(0);
        self.contrl[0] = opcode;
        self.contrl[1] = n_pts;
        self.contrl[3] = n_int;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        // SAFETY: `block` holds five live pointers to arrays owned by `self`,
        // each larger than any opcode issued here writes.
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64) };
    }

    /// `v_opnwk` (1) or `v_opnvwk` (100) with every attribute index 1 and
    /// raster coordinates; returns `CONTRL[6]`.
    fn open(&mut self, opcode: i16, handle: i16) -> i16 {
        for i in 0..10 {
            self.intin[i] = 1;
        }
        self.intin[10] = 2;
        self.op(opcode, 0, 0, 11, handle);
        self.contrl[6]
    }

    /// `vqt_attributes` (38) → `PTSOUT[0..4]`: char width, char height
    /// (`top`), cell width, cell height (`vdi_text.c:1013-1018`).
    fn vqt_attributes(&mut self, handle: i16) -> Attr {
        self.op(38, 0, 0, 0, handle);
        assert_eq!(self.intout[0], 1, "the system face (font id 1)");
        (self.ptsout[0], self.ptsout[1], self.ptsout[2], self.ptsout[3])
    }
}

/// `vqt_attributes`' `PTSOUT[0..4]`.
type Attr = (i16, i16, i16, i16);
/// A screen size `(width, height)`.
type Screen = (u16, u16);

/// `vqt_attributes` of the 8×8 face (`fnt_st_8x8.c`: `max_char_width` 7,
/// `top` 6, `max_cell_width` 8, cell height `top + bottom + 1` = 8) — EmuTOS's
/// measured answer at 640×240.
const ATTR_8X8: Attr = (7, 6, 8, 8);
/// `vqt_attributes` of the 8×16 face (`fnt_st_8x16.c`: 7, 13, 8, 16) —
/// EmuTOS's measured answer at 640×480.
const ATTR_8X16: Attr = (7, 13, 8, 16);

/// The screens the ticket names, plus the two lines around the threshold.
const CASES: &[(Screen, Attr)] = &[
    ((640, 240), ATTR_8X8),
    ((640, 399), ATTR_8X8),
    ((640, 400), ATTR_8X16),
    ((640, 480), ATTR_8X16),
    ((1920, 1080), ATTR_8X16),
];

/// `v_opnwk` → `text_init` → `def_font`; `init_wk` → `text_init2` →
/// `cur_font = def_font`, on the physical workstation and on a virtual one.
#[test]
fn a_fresh_workstation_gets_the_default_font_of_its_screen_height() {
    let _g = lock();
    for &((w, h), want) in CASES {
        screen(w, h);
        let mut pb = Vpb::new();
        let phys = pb.open(1, 0);
        assert_eq!(phys, 1, "{w}x{h}: v_opnwk handle");
        assert_eq!(pb.vqt_attributes(phys), want, "{w}x{h}: vqt_attributes after v_opnwk");
        let virt = pb.open(100, phys);
        assert!(virt > 1, "{w}x{h}: v_opnvwk handle {virt}");
        assert_eq!(pb.vqt_attributes(virt), want, "{w}x{h}: vqt_attributes after v_opnvwk");
        pb.op(101, 0, 0, 0, virt); // v_clsvwk
    }
}

/// `vq_chcells` (escape 1) counts the console's rows in `font_set_default`'s
/// font: 240 / 8 = 30 rows at 640×240 (an 8×16 cell would give 15), 480 / 16
/// = 30 at 640×480, 1080 / 16 = 67 at 1920×1080.
#[test]
fn the_console_cell_grid_follows_the_default_font() {
    let _g = lock();
    for &((w, h), rows) in &[((640u16, 240u16), 30i16), ((640, 480), 30), ((1920, 1080), 67)] {
        screen(w, h);
        let mut pb = Vpb::new();
        let phys = pb.open(1, 0);
        pb.op(5, 1, 0, 0, phys); // vq_chcells
        assert_eq!((pb.intout[0], pb.intout[1]), (rows, (w / 8) as i16), "{w}x{h}: vq_chcells rows, columns");
    }
}

// ── AES ──────────────────────────────────────────────────────────────────

struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

impl Aespb {
    fn new() -> Aespb {
        let mut pb = Aespb {
            contrl: Box::new([0; 8]),
            global: Box::new([0; 16]),
            intin: Box::new([0; 16]),
            intout: Box::new([0; 16]),
            addrin: Box::new([0; 4]),
            addrout: Box::new([0; 4]),
            block: Box::new([0; 6]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.global.as_ptr() as u64;
        pb.block[2] = pb.intin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.addrin.as_ptr() as u64;
        pb.block[5] = pb.addrout.as_ptr() as u64;
        pb
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each larger than any opcode issued here declares.
        assert!(unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, Abi::Lp64) }.is_none());
    }
}

/// Start an AES session on a `w`×`h` screen: the workstation open of
/// `gsx_init` (`geminit.c:675`), the window manager's start, `appl_init`.
fn aes_at(w: u16, h: u16) -> Aespb {
    screen(w, h);
    tos_api::aes::gsxif::gsx_init();
    tos_api::aes::window::wm_init();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    assert!(pb.intout[0] >= 0, "appl_init failed ({})", pb.intout[0]);
    HOST.raster.take();
    pb
}

/// `graf_handle` (77) → `(gl_wchar, gl_hchar, gl_wbox, gl_hbox)`
/// (`gemsuper.c:288-291`). At 640×240 the pixel is twice as tall as wide
/// (`get_std_pixel_size`: 278 × 556), so `gl_wbox` = `11 * 556 / 278` = 22 >
/// `gl_wchar + 4`.
#[test]
fn graf_handle_reports_the_metrics_of_the_default_font() {
    let _g = lock();
    for &((w, h), want) in &[
        ((640u16, 240u16), (8i16, 8i16, 22i16, 11i16)),
        ((640, 480), (8, 16, 19, 19)),
        ((1920, 1080), (8, 16, 19, 19)),
    ] {
        let mut pb = aes_at(w, h);
        pb.call(77, 0, 5, 0);
        assert_eq!((pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4]), want, "{w}x{h}: graf_handle");
    }
}

/// A GEM `OBJECT` at the kernel/LP64 record width (`tos-api/src/aes/object.rs`).
#[repr(C)]
#[derive(Copy, Clone)]
struct Obj {
    next: i16,
    head: i16,
    tail: i16,
    otype: u16,
    flags: u16,
    state: u16,
    spec: u64,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

const G_IBOX: u16 = 25;
const G_STRING: u16 = 28;
const LASTOB: u16 = 0x0020;

/// `objc_draw` of a `G_STRING` draws it in the IBM font (`gsx_tblt(IBM, …)`,
/// `gemoblib.c:845`): one glyph cell per character in the default face of
/// the screen — 8×8 under 400 lines, 8×16 from 400 up.
#[test]
fn objc_draw_writes_ibm_text_in_the_default_font() {
    let _g = lock();
    static TEXT: &[u8] = b"Font\0";
    for &((w, h), cell) in &[((640u16, 240u16), (8i16, 8i16)), ((640, 480), (8, 16)), ((1920, 1080), (8, 16))] {
        let mut pb = aes_at(w, h);
        let tree: Box<[Obj; 2]> = Box::new([
            Obj { next: -1, head: 1, tail: 1, otype: G_IBOX, flags: 0, state: 0, spec: 0, x: 0, y: 0, w: 200, h: 40 },
            Obj {
                next: 0,
                head: -1,
                tail: -1,
                otype: G_STRING,
                flags: LASTOB,
                state: 0,
                spec: TEXT.as_ptr() as u64,
                x: 8,
                y: 8,
                w: 32,
                h: 16,
            },
        ]);
        pb.intin.fill(0);
        pb.intin[0] = 0; // start object
        pb.intin[1] = 8; // depth
        pb.intin[2] = 0;
        pb.intin[3] = 0;
        pb.intin[4] = w as i16;
        pb.intin[5] = h as i16;
        pb.addrin[0] = tree.as_ptr() as u64;
        HOST.raster.take();
        pb.call(42, 6, 1, 1); // objc_draw
        let glyphs = HOST.raster.take();
        assert_eq!(glyphs, vec![cell; 4], "{w}x{h}: the glyph cells of \"Font\"");
    }
}
