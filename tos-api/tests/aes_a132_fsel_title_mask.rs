//! A132 — the file selector's file-box title (`FTITLE`) shows the current
//! wildcard mask, as EmuTOS draws it.
//!
//! EmuTOS `fs_input` points `FTITLE`'s `te_ptext` at its mask buffer
//! (`set_mask(mask, locstr); tedinfo->te_ptext = mask;`,
//! `aes/gemfslib.c:675-678`), re-reads the mask from every new path
//! (`:889-892`) and redraws the title after each listing (`fs_newdir`,
//! `:502-508`). Spec: `docs/api/aes/resource-fileselector.md`, "The mask in
//! the file-box title `FTITLE`".
//!
//! Driven through the real AES trap against a pixel buffer; the screen is
//! captured each time the selector's modal loop takes a key, i.e. with the
//! dialog up. "Shown" means the mask's glyphs are on the screen twice: once in
//! the `FSDIRECT` path field (which shows the whole path), once in the title.
//! Without the title the mask is there only once.
//!
//! Harness shape (backend + host + parameter block) from
//! `aes_dialog_undraw.rs`.
#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend over a real pixel buffer ─────────────────────────────

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0x0012_3456; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
        }
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn idx(x: i16, y: i16) -> usize {
        y as usize * SCREEN_W as usize + x as usize
    }

    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(SCREEN_W, SCREEN_H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return;
            }
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }

    fn snapshot(&self) -> Vec<u32> {
        self.px.lock().unwrap().clone()
    }
}

impl RasterBackend for BufBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
    }
    fn set_color(&self, index: u16, r: u16, g: u16, b: u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize] = (r, g, b);
        }
    }
    fn query_color(&self, index: u16) -> (u16, u16, u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize]
        } else {
            (0, 0, 0)
        }
    }
    fn set_active_palette(&self, palette: &[u16; 256]) {
        *self.palette.lock().unwrap() = *palette;
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let (r, g, b) = ((argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF);
        (((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)) as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let (r5, g6, b5) = ((px >> 11) & 0x1F, (px >> 5) & 0x3F, px & 0x1F);
        let r = ((r5 << 3) | (r5 >> 2)) as u32;
        let g = ((g6 << 2) | (g6 >> 4)) as u32;
        let b = ((b5 << 3) | (b5 >> 2)) as u32;
        (r << 16) | (g << 8) | b
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        if !Self::inside(x, y) {
            return 0;
        }
        let v = self.px.lock().unwrap()[Self::idx(x, y)];
        let want = self.device_to_rgb565(v);
        let pal = *self.palette.lock().unwrap();
        pal.iter().position(|&p| p == want).unwrap_or(0) as Pel
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, v, clip);
            }
        }
    }
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        let fg = self.rgb565_to_device(self.pel_to_device16(pel));
        let bg = self.rgb565_to_device(self.pel_to_device16(0));
        let (ax, ay, mut bx, mut by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
        if mode == WriteMode::Xor && !last {
            if ax != bx {
                bx -= 1;
            } else if ay != by {
                by -= 1;
            }
        }
        let combine = |x: i16, y: i16, set: bool| match mode {
            WriteMode::Replace => self.put(x, y, if set { fg } else { bg }, clip),
            WriteMode::Transparent => {
                if set {
                    self.put(x, y, fg, clip)
                }
            }
            WriteMode::Xor => {
                if set {
                    let v = self.get_device_pixel(x, y);
                    self.put(x, y, v ^ 0x00FF_FFFF, clip);
                }
            }
            WriteMode::RevTransparent => {
                if !set {
                    self.put(x, y, fg, clip)
                }
            }
        };
        if ay == by {
            let (xlo, xhi) = (ax.min(bx), ax.max(bx));
            for x in xlo..=xhi {
                combine(x, ay, mask & (0x8000u16 >> (x & 0xf)) != 0);
            }
            return mask;
        }
        let (mut x, mut y) = (ax as i32, ay as i32);
        let (xe, ye) = (bx as i32, by as i32);
        let (dx, dy) = ((xe - x).abs(), -(ye - y).abs());
        let sy = if y < ye { 1 } else { -1 };
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            combine(x as i16, y as i16, m & 0x8000 != 0);
            m = m.rotate_left(1);
            if x == xe && y == ye {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += 1;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        m
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.rgb565_to_device(self.pel_to_device16(fill.fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(fill.bg));
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let bit = row & (0x8000u16 >> (xx.rem_euclid(16) as u16));
                self.put(xx, yy, if bit != 0 { fg } else { bg }, clip);
            }
        }
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        glyph: &[u8],
        cell_w: i16,
        cell_h: i16,
        fg: Pel,
        _mode: WriteMode,
        clip: ClipRect,
    ) {
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, _m: WriteMode, clip: ClipRect) {
        let f = self.rgb565_to_device(self.pel_to_device16(fg));
        let b = self.rgb565_to_device(self.pel_to_device16(bg));
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let bit = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16);
                self.put(dst.x + col, dst.y + row, if bit != 0 { f } else { b }, clip);
            }
        }
    }
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, _op: u8, clip: ClipRect) {
        let mut tmp = Vec::with_capacity((src.w as usize) * (src.h as usize));
        {
            let px = self.px.lock().unwrap();
            for row in 0..src.h {
                for col in 0..src.w {
                    let (sx, sy) = (src.x + col, src.y + row);
                    tmp.push(if Self::inside(sx, sy) { px[Self::idx(sx, sy)] } else { 0 });
                }
            }
        }
        for row in 0..src.h {
            for col in 0..src.w {
                let v = tmp[(row as usize) * (src.w as usize) + col as usize];
                self.put(dx + col, dy + row, v, clip);
            }
        }
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)]
        } else {
            0
        }
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)] = argb;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host: keys only, a screen capture at every key the loop takes ─────

struct KeyHost {
    raster: BufBackend,
    keys: Mutex<VecDeque<u16>>,
    /// The screen as it was when the modal loop took each key.
    shots: Mutex<Vec<Vec<u32>>>,
    clock: AtomicU64,
}

impl Host for KeyHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        let k = self.keys.lock().unwrap().pop_front();
        if k.is_some() {
            self.shots.lock().unwrap().push(self.raster.snapshot());
        }
        k
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        let t = self.clock.fetch_add(1, Ordering::Relaxed);
        assert!(t < 100_000, "the file selector is still waiting with no keys left");
    }
}

unsafe impl Sync for KeyHost {}
unsafe impl Send for KeyHost {}

/// EmuTOS `videl_palette1` pens 0..15 (white, black, …) as the VDI dispatcher
/// seeds them; see `aes_dialog_undraw.rs` `seed_palette`.
fn seed_palette(b: &BufBackend) {
    const VIDEL1: [(u16, u16, u16); 16] = [
        (1000, 1000, 1000), (0, 0, 0), (1000, 0, 0), (0, 1000, 0),
        (0, 0, 1000), (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
        (733, 733, 733), (533, 533, 533), (667, 0, 0), (0, 667, 0),
        (0, 0, 667), (0, 667, 667), (667, 667, 0), (667, 0, 667),
    ];
    let mut pal = [0u16; 256];
    for (i, (r, g, bl)) in VIDEL1.iter().enumerate() {
        let c5 = |v: u16| ((v as u32 * 31 + 500) / 1000) as u16;
        pal[i] = (c5(*r) << 11) | (c5(*g) << 6) | c5(*bl);
    }
    *b.palette.lock().unwrap() = pal;
}

static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static KeyHost> = OnceLock::new();

fn rig() -> &'static KeyHost {
    RIG.get_or_init(|| {
        let host: &'static KeyHost = Box::leak(Box::new(KeyHost {
            raster: BufBackend::new(),
            keys: Mutex::new(VecDeque::new()),
            shots: Mutex::new(Vec::new()),
            clock: AtomicU64::new(0),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        seed_palette(&host.raster);
        host
    })
}

// ── AES parameter block ───────────────────────────────────────────────────

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
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, Abi::Lp64) };
    }
}

// ── keys and the glyph search ─────────────────────────────────────────────

const KEY_RETURN: u16 = 0x1C0D;
const KEY_TAB: u16 = 0x0F09;
const KEY_ESC: u16 = 0x011B;

/// Run `fsel_input` on `start` with `keys` queued; returns the path written
/// back, the button, and the screens captured at each key.
fn run_fsel(start: &[u8], keys: &[u16]) -> (Vec<u8>, i16, Vec<Vec<u32>>) {
    let host = rig();
    host.shots.lock().unwrap().clear();
    host.clock.store(0, Ordering::Relaxed);
    {
        let mut q = host.keys.lock().unwrap();
        q.clear();
        q.extend(keys.iter().copied());
    }
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    let mut path = [0u8; 128];
    path[..start.len()].copy_from_slice(start);
    let mut sel = [0u8; 32];
    pb.intin.fill(0);
    pb.addrin[0] = path.as_mut_ptr() as u64;
    pb.addrin[1] = sel.as_mut_ptr() as u64;
    pb.call(90, 0, 2, 2); // fsel_input
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    assert_eq!(pb.intout[0], 1, "fsel_input ran");
    let n = path.iter().position(|&b| b == 0).unwrap();
    let shots = std::mem::take(&mut *host.shots.lock().unwrap());
    (path[..n].to_vec(), pb.intout[1], shots)
}

/// The key words that type `s` (ASCII in the low byte, as `fm_do` passes it on).
fn typed(s: &[u8]) -> Vec<u16> {
    s.iter().map(|&c| c as u16).collect()
}

const BLACK: u32 = 0x0000_0000;

/// Every screen position (top-left of the first 8x16 cell) at which `s` is
/// drawn in the system font: every set glyph pixel black, and the cells not
/// all black (a solid black area matches any glyph).
fn find_text(shot: &[u32], s: &[u8]) -> Vec<(i16, i16)> {
    let glyphs: Vec<&[u8; 16]> = s.iter().map(|&c| tos_api::vdi::font::glyph(c as u16)).collect();
    let w = 8 * s.len() as i16;
    let mut hits = Vec::new();
    for y in 0..SCREEN_H as i16 - 16 {
        'x: for x in 0..SCREEN_W as i16 - w {
            let mut other = false;
            for (i, g) in glyphs.iter().enumerate() {
                for (row, &bits) in g.iter().enumerate() {
                    for col in 0..8i16 {
                        let (px, py) = (x + 8 * i as i16 + col, y + row as i16);
                        let v = shot[BufBackend::idx(px, py)];
                        if bits & (0x80 >> col) != 0 {
                            if v != BLACK {
                                continue 'x;
                            }
                        } else if v != BLACK {
                            other = true;
                        }
                    }
                }
            }
            if other {
                hits.push((x, y));
            }
        }
    }
    hits
}

/// `s` stands on two different rows: the `FSDIRECT` path field and the
/// `FTITLE` caption.
fn assert_in_field_and_title(shot: &[u32], s: &[u8], when: &str) {
    let hits = find_text(shot, s);
    let mut rows: Vec<i16> = hits.iter().map(|h| h.1).collect();
    rows.dedup();
    assert_eq!(
        rows.len(),
        2,
        "{when}: {:?} must be drawn in the path field and in the file box's title \
         (EmuTOS fs_input/fs_newdir, gemfslib.c:502-508,675-678); found at {hits:?}",
        core::str::from_utf8(s).unwrap()
    );
}

// ── the tests ─────────────────────────────────────────────────────────────

/// On entry: `set_mask(mask, locstr); te_ptext = mask` (`gemfslib.c:677-678`)
/// — the caller's mask is in the title with the dialog's first draw.
#[test]
fn the_title_shows_the_callers_mask_on_open() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (path, button, shots) = run_fsel(b"C:\\*.TXT", &[KEY_RETURN]);
    assert_eq!(button, 1, "Return = OK");
    assert_eq!(path, b"C:\\*.TXT");
    assert_eq!(shots.len(), 1);
    assert_in_field_and_title(&shots[0], b"*.TXT", "on open");
}

/// An edited `FSDIRECT` taken by Return is a new listing: `set_mask` reads the
/// mask from the edited path (`gemfslib.c:873-880,889-892`) and `fs_newdir`
/// redraws the title (`:502-508`). The capture at the second Return is the
/// screen after that listing.
#[test]
fn the_title_follows_an_edited_mask() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut keys = vec![KEY_TAB, KEY_ESC];
    keys.extend(typed(b"C:\\*.C"));
    keys.push(KEY_RETURN); // a changed path: re-list, do not exit
    keys.push(KEY_RETURN); // OK
    let (path, button, shots) = run_fsel(b"C:\\*.TXT", &keys);
    assert_eq!(button, 1);
    assert_eq!(path, b"C:\\*.C", "the edited path is returned");
    assert_in_field_and_title(&shots[0], b"*.TXT", "before the edit");
    let last = shots.last().unwrap();
    assert_in_field_and_title(last, b"*.C", "after the edit");
    assert!(find_text(last, b"*.TXT").is_empty(), "the old mask is gone from the screen");
}

/// A path edited down to its directory has no mask: `set_mask` appends `*.*`
/// to the path itself (`set_all_files`, `gemfslib.c:529-530`) — the field,
/// the title and the path handed back all carry it.
#[test]
fn an_empty_mask_becomes_star_dot_star_in_title_and_path() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut keys = vec![KEY_TAB, KEY_ESC];
    keys.extend(typed(b"C:\\"));
    keys.push(KEY_RETURN);
    keys.push(KEY_RETURN);
    let (path, button, shots) = run_fsel(b"C:\\*.TXT", &keys);
    assert_eq!(button, 1);
    assert_eq!(path, b"C:\\*.*", "set_mask completed the path");
    assert_in_field_and_title(shots.last().unwrap(), b"*.*", "after the edit to C:\\");
}
