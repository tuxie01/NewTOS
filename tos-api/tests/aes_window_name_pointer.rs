//! A83 — **`wind_set(WF_NAME/WF_INFO)` keeps the application's pointer and
//! reads the string at every frame build; no length limit; NULL → `""`.**
//!
//! EmuTOS stores the pointer the caller passed — `w_pname = *(char **)pinwds`
//! (`references/emutos/aes/gemwmlib.c:1661`, `w_pinfo` `:1669`, pinned tree
//! `1a7484aba1bfba23e2c0899e243b6d8177fe41df`) — and `w_bldactive` hands that
//! pointer to the `W_NAME`/`W_INFO` `TEDINFO` on **every** frame build
//! (`:501-502`, 3D variant). An application that rewrites its title buffer
//! in place sees the new text on the next frame repaint; a title longer than
//! the gadget is clipped by the object drawer, never truncated by the AES
//! (`docs/api/aes/window-graphics.md`, "`WF_NAME`/`WF_INFO` store a pointer,
//! not a copy; no length limit (contract, A83, 2026-09-20)").
//!
//! NewTOS used to copy the string at `wind_set` time into `name`/`info:
//! [u8; 80]` (`copy_cstr`): a snapshot, capped at 79 characters. The A59
//! survey's case T10 (`TASK/A83.md`) measured it: after the in-place rewrite
//! and a re-top, the title band was pixel-identical to the old text (0 pixels
//! changed, 168 pixels off the reference). Since A83 the AES keeps the
//! pointer and the slot of the process that owns it (`LiveStr` in
//! `tos-api/src/aes/window.rs`) and reads the string on every frame build.
//!
//! Cases:
//!
//! * **T10** — `wind_set(h, WF_NAME, buf = "AAAAAAAA")`, rewrite `buf` in
//!   place to `"BBBBBBBB"`, `WF_TOP` another window and back: the title band
//!   shows `BBBBBBBB` — the same pixels as a window whose title was *set* to
//!   `"BBBBBBBB"` through the same top/untop sequence, and different from the
//!   `AAAAAAAA` band. Red on the copy, green on the pointer.
//! * **No 79-character cap** — a 100-character title on a 1900-px-wide window
//!   paints its 86th glyph; the band differs from the same title cut at 79
//!   characters. Red on the copy, green on the pointer.
//! * **NULL → `""`** — `wind_set(h, WF_NAME, NULL)` after a real title leaves
//!   the band as a never-named window's (EmuTOS: undefined; NewTOS's decision
//!   is the empty string, `w_setup`'s own initial `w_pname = ""`,
//!   `gemwmlib.c:216`). Green before and after.
//!
//! These cases pass **kernel (EL1) pointers**: on the host `process::current()`
//! is null, so the AES records no owner slot and reads the pointer directly.
//! The EL0-owner cases — the string is read through the *owner's* mapping,
//! a dangling pointer renders as `""`, a string stops at its block end and
//! is followed across a page boundary — need the owner's block table, which
//! a host process has no equivalent of; they are modelled at the
//! `usermem::resolve_kva` seam in `window.rs`'s `live_title_tests`.
//!
//! The screen is **1920×1080** (the current hardware profile) so that a title
//! beyond 79 glyphs fits a name gadget at all — at 640 px the gadget clips
//! before the copy's cap could show. Harness after
//! `aes_window_color_words.rs`, reduced to what pixel comparison needs.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 1920;
const SCREEN_H: u16 = 1080;

// ── a raster backend over a real pixel buffer ──────────────────────────────

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
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

    fn clear(&self) {
        self.px.lock().unwrap().fill(0);
    }

    /// FNV-1a over the device pixels of `[x, x+w) × [y, y+h)`.
    fn hash_rect(&self, x: i16, y: i16, w: i16, h: i16) -> u64 {
        let px = self.px.lock().unwrap();
        let mut hsh: u64 = 0xcbf2_9ce4_8422_2325;
        for yy in y..y + h {
            for xx in x..x + w {
                let v = if Self::inside(xx, yy) { px[Self::idx(xx, yy)] } else { 0 };
                for b in v.to_le_bytes() {
                    hsh ^= b as u64;
                    hsh = hsh.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        hsh
    }

    /// How many device pixels of `[x, x+w) × [y, y+h)` equal `v`.
    fn count_rect(&self, x: i16, y: i16, w: i16, h: i16, v: u32) -> usize {
        let px = self.px.lock().unwrap();
        let mut n = 0;
        for yy in y..y + h {
            for xx in x..x + w {
                if Self::inside(xx, yy) && px[Self::idx(xx, yy)] == v {
                    n += 1;
                }
            }
        }
        n
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
    fn set_active_palette(&self, palette: &[u32; 256]) {
        *self.palette.lock().unwrap() = palette.map(|p| p as u16);
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
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        let (mut x, mut y) = (x0 as i32, y0 as i32);
        let (dx, dy) = ((x1 as i32 - x).abs(), -(y1 as i32 - y).abs());
        let (sx, sy) = (if x < x1 as i32 { 1 } else { -1 }, if y < y1 as i32 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            if m & 0x8000 != 0 {
                self.put(x as i16, y as i16, v, clip);
            }
            m = m.rotate_left(1);
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
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
                match fill.mode {
                    WriteMode::Replace => self.put(xx, yy, if bit != 0 { fg } else { bg }, clip),
                    _ => {
                        if bit != 0 {
                            self.put(xx, yy, fg, clip);
                        }
                    }
                }
            }
        }
    }
    /// Honours `Replace`: clear bits of the cell are painted in palette
    /// colour 0 (the VDI background pen), set bits in `fg`.
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        glyph: &[u8],
        cell_w: i16,
        cell_h: i16,
        fg: Pel,
        mode: WriteMode,
        clip: ClipRect,
    ) {
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(0));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                } else if mode == WriteMode::Replace {
                    self.put(x + col, y + row, bg, clip);
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

// ── the host: no input at all — every case is pure `wind_*` calls ──────────

struct QuietHost {
    raster: BufBackend,
    clock: AtomicU64,
}

impl Host for QuietHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        None
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: every mutable field is behind a Mutex / atomic.
unsafe impl Sync for QuietHost {}
unsafe impl Send for QuietHost {}

// ── AES parameter block ───────────────────────────────────────────────────

const ABI: Abi = Abi::Lp64;

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
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

fn install(host: &'static QuietHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

// ── AES constants ─────────────────────────────────────────────────────────

// window kinds (`include/aesdefs.h:519-531`)
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
/// Title bar with name, closer and fuller — the A59 survey's title gadgets.
const KIND_TITLE: i16 = NAME | CLOSER | FULLER | MOVER;

const OP_WIND_CREATE: i16 = 100;
const OP_WIND_OPEN: i16 = 101;
const OP_WIND_SET: i16 = 105;
const OP_WIND_NEW: i16 = 109;

const WF_NAME: i16 = 2;
const WF_TOP: i16 = 10;

// frame metrics for the 8×16 system font on a ≥ 640-wide screen
// (`gl_wbox`/`gl_hbox` = 19, `ADJ3DSTD` = 2, `adj_wbox`/`adj_hbox` = 23)
const GL_WBOX: i16 = 19;
const ADJ3DSTD: i16 = 2;
const ADJ_WBOX: i16 = GL_WBOX + 2 * ADJ3DSTD;
const ADJ_HBOX: i16 = 23;

/// AES palette indices (`obdefs.h:109-110`).
const WHITE: Pel = 0;
const BLACK: Pel = 1;

/// The device word the backend paints palette index `pel` as.
fn dev(host: &QuietHost, pel: Pel) -> u32 {
    host.raster.rgb565_to_device(host.raster.pel_to_device16(pel))
}

// ── the rig ───────────────────────────────────────────────────────────────

/// The AES window table and the installed host are process-wide statics, so
/// the cases in this binary must not run concurrently.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static QuietHost> = OnceLock::new();

fn rig() -> &'static QuietHost {
    RIG.get_or_init(|| {
        let host: &'static QuietHost =
            Box::leak(Box::new(QuietHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
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

/// Fresh AES window state (`wind_new`, EmuTOS `wm_new`, `aes/gemwmlib.c:
/// 1875-1893`) and a cleared framebuffer.
fn fresh() -> (&'static QuietHost, Aespb) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(OP_WIND_NEW, 0, 1, 0);
    host.raster.clear();
    (host, pb)
}

/// `wind_create(kind)` + `wind_open(r)`, returning the handle.
fn open_window(pb: &mut Aespb, kind: i16, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(OP_WIND_CREATE, 5, 1, 0);
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(OP_WIND_OPEN, 5, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_open({h})");
    h
}

/// `wind_set(h, WF_NAME, ptr)` with the pointer packed into `int_in[2..]` at
/// the host's pointer width and `ain_len 0` — gemlib's `wind_set_str`
/// (`a_wind_set_str.c:25`), the convention every ported application uses.
fn set_name_ptr(pb: &mut Aespb, handle: i16, ptr: *const u8) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_NAME;
    let p = ptr as u64;
    for i in 0..(core::mem::size_of::<usize>() / 2) {
        pb.intin[2 + i] = ((p >> (16 * i)) & 0xFFFF) as u16 as i16;
    }
    pb.call(OP_WIND_SET, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, WF_NAME, ptr)");
}

/// `wind_set(h, WF_TOP)`.
fn set_top(pb: &mut Aespb, handle: i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_TOP;
    pb.call(OP_WIND_SET, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, WF_TOP)");
}

/// The title band of the window at `r` — its first `ADJ_HBOX` rows.
fn title_band(host: &QuietHost, r: (i16, i16, i16, i16)) -> u64 {
    host.raster.hash_rect(r.0, r.1, r.2, ADJ_HBOX)
}

/// The T10 stage: window `h1` at `R1` with its title, a second window `h2`
/// elsewhere, and `h1` on top.
const R1: (i16, i16, i16, i16) = (30, 40, 320, 240);
const R2: (i16, i16, i16, i16) = (400, 400, 320, 240);

fn stage(pb: &mut Aespb, title: *const u8) -> (i16, i16) {
    let h2 = open_window(pb, KIND_TITLE, R2);
    let h1 = open_window(pb, KIND_TITLE, R1);
    set_name_ptr(pb, h1, title);
    (h1, h2)
}

/// Untop `h1` (top `h2`) and top it again: the frame repaint the A59 survey
/// used to make the AES rebuild the title band.
fn retop(pb: &mut Aespb, h1: i16, h2: i16) {
    set_top(pb, h2);
    set_top(pb, h1);
}

// ── cases ─────────────────────────────────────────────────────────────────

/// **T10** — the title buffer rewritten in place after `wind_set` shows its
/// new contents on the next frame repaint (EmuTOS reads `w_pname` in
/// `w_bldactive`, `gemwmlib.c:501`, on every build).
#[test]
fn t10_title_rewritten_in_place_shows_on_the_next_frame_repaint() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    // Reference: a window whose title *was set* to "BBBBBBBB", through the
    // same top/untop sequence.
    let (host, mut pb) = fresh();
    let b_title: &'static [u8] = b"BBBBBBBB\0";
    let (h1, h2) = stage(&mut pb, b_title.as_ptr());
    retop(&mut pb, h1, h2);
    let band_b = title_band(host, R1);

    // The case: set "AAAAAAAA", rewrite the buffer in place, re-top.
    let (host, mut pb) = fresh();
    let buf: &'static mut [u8] = Box::leak(Box::new(*b"AAAAAAAA\0"));
    let (h1, h2) = stage(&mut pb, buf.as_ptr());
    retop(&mut pb, h1, h2);
    let band_a = title_band(host, R1);
    assert_ne!(band_a, band_b, "the A and B bands must be told apart at all");

    buf[..8].copy_from_slice(b"BBBBBBBB");
    retop(&mut pb, h1, h2);
    let band_after = title_band(host, R1);

    assert_ne!(
        band_after, band_a,
        "A83 T10: after the in-place rewrite and a re-top the title band still shows AAAAAAAA \
         (the AES copied the string at wind_set time; EmuTOS reads the pointer on every \
         frame build, gemwmlib.c:1661,501)"
    );
    assert_eq!(band_after, band_b, "A83 T10: the band must be the BBBBBBBB rendering");
}

/// **No 79-character cap** — a 100-character title paints its 86th glyph on a
/// window wide enough to hold it, and the band differs from the same title cut
/// at 79 characters (the AES never truncates; the gadget clips,
/// `docs/api/aes/window-graphics.md`, A83 contract).
#[test]
fn a_title_longer_than_79_characters_is_not_truncated() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // 1900 px wide: name area = 1900 − 2·adj_wbox − 2·ADJ3DSTD = 1850 px, room
    // for 231 glyphs — the 100-glyph title fits whole.
    const WIDE: (i16, i16, i16, i16) = (10, 40, 1900, 240);
    let mut long = [0u8; 101];
    for (i, b) in long.iter_mut().take(100).enumerate() {
        *b = b'0' + (i % 10) as u8;
    }
    let long: &'static [u8] = Box::leak(Box::new(long));
    let mut cut = [0u8; 80];
    cut[..79].copy_from_slice(&long[..79]);
    let cut: &'static [u8] = Box::leak(Box::new(cut));

    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_TITLE, WIDE);
    set_name_ptr(&mut pb, h, cut.as_ptr());
    let band_cut = title_band(host, WIDE);

    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_TITLE, WIDE);
    set_name_ptr(&mut pb, h, long.as_ptr());
    let band_long = title_band(host, WIDE);

    assert_ne!(
        band_long, band_cut,
        "A83: a 100-character title renders like its first 79 characters — the AES cut it \
         (copy into name[80]); EmuTOS hands the whole string to the gadget"
    );

    // The 86th glyph cell: text wider than the area is left-aligned at
    // `name_x + 2` (`draw_frame`'s `((name_w − nlen) / 2).max(2)`), each glyph
    // 8 px; the topped name draws in MD_REPLACE, so the cell holds white
    // background *and* black strokes.
    let name_x = WIDE.0 + ADJ_WBOX + ADJ3DSTD;
    let cell_x = name_x + 2 + 8 * 85;
    let cell_y = WIDE.1 + ADJ3DSTD + 1;
    let black = host.raster.count_rect(cell_x, cell_y, 8, 16, dev(host, BLACK));
    let white = host.raster.count_rect(cell_x, cell_y, 8, 16, dev(host, WHITE));
    assert!(
        black > 0 && white > 0,
        "A83: the 86th glyph cell at ({cell_x},{cell_y}) holds no text (black {black}, white {white})"
    );
}

/// **NULL → `""`** — `wind_set(h, WF_NAME, NULL)` after a real title leaves
/// the band as a never-named window's. EmuTOS leaves NULL undefined
/// (`w_bldactive` passes it to the text drawer); NewTOS resolves it to the
/// empty string. Green today; the pointer implementation must keep it so.
#[test]
fn a_null_pointer_is_an_empty_title() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    // Reference: a window that was never named.
    let (host, mut pb) = fresh();
    let (h1, h2) = {
        let h2 = open_window(&mut pb, KIND_TITLE, R2);
        let h1 = open_window(&mut pb, KIND_TITLE, R1);
        (h1, h2)
    };
    retop(&mut pb, h1, h2);
    let band_unnamed = title_band(host, R1);

    let (host, mut pb) = fresh();
    let hello: &'static [u8] = b"HELLO\0";
    let (h1, h2) = stage(&mut pb, hello.as_ptr());
    retop(&mut pb, h1, h2);
    let band_hello = title_band(host, R1);
    assert_ne!(band_hello, band_unnamed, "a named band must differ from an unnamed one");

    set_name_ptr(&mut pb, h1, core::ptr::null());
    retop(&mut pb, h1, h2);
    assert_eq!(
        title_band(host, R1),
        band_unnamed,
        "A83: wind_set(WF_NAME, NULL) must render as the empty title"
    );
}
