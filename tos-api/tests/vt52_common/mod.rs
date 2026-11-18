//! Shared fixture of the CON04 VT52 console tests: a 640×480 store of
//! 16-bit screen words (80×30 cells of 8×16) behind a test host whose
//! `Bconout(2)` device is the VT52 automaton, plus cell helpers and a VDI
//! parameter block.

#![allow(dead_code)]

use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::font;

pub const W: u16 = 640;
pub const H: u16 = 480;
pub const MX: i16 = 79;
pub const MY: i16 = 29;

/// Pens 0..15 as VDI RGB (EmuTOS `videl_palette1`, `vdi/vdi_col.c:130-136`),
/// the device default `v_opnwk` seeds the pseudo-palette from.
#[rustfmt::skip]
const VIDEL_PALETTE1: [(u16, u16, u16); 16] = [
    (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
    (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
    (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
    (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
];

/// Register 0 (pen 0, white) and register 15 (pen 255, black).
pub const WHITE: u16 = 0xffdf;
pub const BLACK: u16 = 0x0000;
/// Register 1 = pen 2 (red), register 4 = pen 4 (blue).
pub const RED: u16 = 0xf800;
pub const BLUE: u16 = 0x001f;

pub struct Screen {
    words: Mutex<Vec<u16>>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl Screen {
    fn idx(x: i16, y: i16) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as u16) < W && (y as u16) < H).then(|| y as usize * W as usize + x as usize)
    }
}

impl RasterBackend for Screen {
    fn width(&self) -> u16 {
        W
    }
    fn height(&self) -> u16 {
        H
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
    }
    fn set_color(&self, index: u16, r: u16, g: u16, b: u16) {
        if let Some(e) = self.req.lock().unwrap().get_mut(index as usize) {
            *e = (r, g, b);
        }
    }
    fn query_color(&self, index: u16) -> (u16, u16, u16) {
        self.req.lock().unwrap().get(index as usize).copied().unwrap_or((0, 0, 0))
    }
    fn set_active_palette(&self, _p: &[u32; 256]) {}
    fn pel_to_device16(&self, _pel: Pel) -> u16 {
        unreachable!("the console draws screen words, not pens")
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
    #[allow(clippy::too_many_arguments)]
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
    fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _fill: PatternFill, _clip: ClipRect) {}
    #[allow(clippy::too_many_arguments)]
    fn draw_glyph_sized(
        &self,
        _x: i16,
        _y: i16,
        _glyph: &[u8],
        _cw: i16,
        _ch: i16,
        _fg: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
    ) {
    }
    fn blit_mono(&self, _dst: Rect, _src: MonoSrc, _fg: Pel, _bg: Pel, _mode: WriteMode, _clip: ClipRect) {}
    /// A plain replace copy (the console only uses op 3).
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, op: u8, _clip: ClipRect) {
        assert_eq!(op, 3, "the console copies with S_ONLY");
        let mut words = self.words.lock().unwrap();
        let mut tmp = Vec::new();
        for r in 0..src.h {
            for c in 0..src.w {
                tmp.push(Self::idx(src.x + c, src.y + r).map(|i| words[i]));
            }
        }
        let mut k = 0;
        for r in 0..src.h {
            for c in 0..src.w {
                if let (Some(i), Some(v)) = (Self::idx(dx + c, dy + r), tmp[k]) {
                    words[i] = v;
                }
                k += 1;
            }
        }
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        Self::idx(x, y).map_or(0, |i| self.words.lock().unwrap()[i] as u32)
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        if let Some(i) = Self::idx(x, y) {
            self.words.lock().unwrap()[i] = argb as u16;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

struct TestHost {
    screen: Screen,
}

impl Host for TestHost {
    /// The kernel's `Bconout(2)` device: the VT52 automaton.
    fn console_write_byte(&self, byte: u8) {
        tos_api::vdi::alpha_conout(&self.screen, byte);
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.screen)
    }
}

static HOST: std::sync::OnceLock<&'static TestHost> = std::sync::OnceLock::new();
static INSTALL: Once = Once::new();
static LOCK: Mutex<()> = Mutex::new(());

fn host() -> &'static TestHost {
    HOST.get_or_init(|| {
        let mut req = [(0u16, 0u16, 0u16); 256];
        req[..16].copy_from_slice(&VIDEL_PALETTE1);
        Box::leak(Box::new(TestHost {
            screen: Screen { words: Mutex::new(vec![0x1234; W as usize * H as usize]), req: Mutex::new(req) },
        }))
    })
}

/// Take the console, install the host — and, with `open_workstation`, open
/// the physical workstation once, as the AES does — then `vt52_init`
/// (cleared screen, power-up state). One test binary uses one mode.
pub fn setup(open_workstation: bool) -> (MutexGuard<'static, ()>, &'static Screen) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    INSTALL.call_once(|| {
        let h: &'static dyn Host = host();
        tos_api::host::install_host(Box::leak(Box::new(h)));
        if open_workstation {
            let mut pb = Pb::new();
            for i in 0..10 {
                pb.intin[i] = 1;
            }
            pb.intin[10] = 2;
            pb.op(1, 0, 0, 11, 0); // v_opnwk
        }
    });
    let s = &host().screen;
    tos_api::vdi::alpha_init(s);
    (g, s)
}

pub fn out(s: &Screen, bytes: &[u8]) {
    for &b in bytes {
        tos_api::vdi::alpha_conout(s, b);
    }
}

pub fn cursor() -> (i16, i16) {
    tos_api::vdi::alpha_cursor()
}

pub fn word(s: &Screen, x: i16, y: i16) -> u16 {
    s.get_device_pixel(x, y) as u16
}

/// The 8×16 words of cell `(col,row)`.
pub fn cell(s: &Screen, col: i16, row: i16) -> Vec<u16> {
    let mut v = Vec::new();
    for y in 0..16 {
        for x in 0..8 {
            v.push(word(s, col * 8 + x, row * 16 + y));
        }
    }
    v
}

/// What cell `(col,row)` must hold for glyph `ch` in `fg` on `bg`.
pub fn glyph_cell(ch: u8, fg: u16, bg: u16) -> Vec<u16> {
    let g = font::glyph(ch as u16);
    let mut v = Vec::new();
    for bits in g.iter() {
        for x in 0..8 {
            v.push(if bits & (0x80 >> x) != 0 { fg } else { bg });
        }
    }
    v
}

pub fn is_char(s: &Screen, col: i16, row: i16, ch: u8, fg: u16, bg: u16) -> bool {
    cell(s, col, row) == glyph_cell(ch, fg, bg)
}

pub fn is_blank(s: &Screen, col: i16, row: i16, bg: u16) -> bool {
    cell(s, col, row).iter().all(|&w| w == bg)
}

pub fn is_cursor_on_blank(s: &Screen, col: i16, row: i16, bg: u16) -> bool {
    cell(s, col, row).iter().all(|&w| w == !bg)
}

pub fn row_text(s: &Screen, row: i16, text: &[u8]) -> bool {
    text.iter().enumerate().all(|(i, &c)| is_char(s, i as i16, row, c, BLACK, WHITE))
}

/// Fill row `row` with `ch` (wrap is off, so 80 glyphs stay on the row).
pub fn fill_row(s: &Screen, row: i16, ch: u8) {
    out(s, &[0x1b, b'Y', 32 + row as u8, 32]);
    out(s, &[ch; 80]);
}

// ── parameter block for the VDI calls ───────────────────────────────────

pub struct Pb {
    pub contrl: Box<[i16; 12]>,
    pub intin: Box<[i16; 128]>,
    pub ptsin: Box<[i16; 64]>,
    pub intout: Box<[i16; 128]>,
    pub ptsout: Box<[i16; 64]>,
}

impl Pb {
    pub fn new() -> Pb {
        Pb {
            contrl: Box::new([0; 12]),
            intin: Box::new([0; 128]),
            ptsin: Box::new([0; 64]),
            intout: Box::new([0; 128]),
            ptsout: Box::new([0; 64]),
        }
    }
    pub fn op(&mut self, opcode: i16, sub: i16, n_pts: i16, n_int: i16, handle: i16) {
        self.contrl.fill(0);
        self.contrl[0] = opcode;
        self.contrl[1] = n_pts;
        self.contrl[3] = n_int;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        let block: [u64; 5] = [
            self.contrl.as_mut_ptr() as u64,
            self.intin.as_mut_ptr() as u64,
            self.ptsin.as_mut_ptr() as u64,
            self.intout.as_mut_ptr() as u64,
            self.ptsout.as_mut_ptr() as u64,
        ];
        // SAFETY: five live arrays owned by `self`, large enough for the
        // escape and workstation calls made here.
        unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, Abi::Lp64) };
    }
    /// `v_escape` sub-function `sub` on the physical workstation (handle 1).
    pub fn escape(&mut self, sub: i16, intin: &[i16]) {
        self.intin[..intin.len()].copy_from_slice(intin);
        self.op(5, sub, 0, intin.len() as i16, 1);
    }
}

