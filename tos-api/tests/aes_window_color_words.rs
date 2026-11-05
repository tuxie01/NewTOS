//! A79 — **`wind_set(WF_COLOR)` / `WF_DCOLOR` colour words reach the pixels,
//! and the frame is the literal rendering of those words.**
//!
//! EmuTOS stores the words (`wm_set`, `aes/gemwmlib.c:1719-1734`), rebuilds
//! the window element tree with `w_bldactive` — `w_setcolor` writes each
//! gadget's word into its `ob_spec` / `te_color` (`:382-397`) — and `ob_draw`
//! then paints border, interior and text from that word (`just_draw`,
//! `aes/gemoblib.c:610-733`; `gr_crack`, `aes/gemgraf.c:612-628`; `gr_rect`,
//! `:547-559`). Before this ticket NewTOS's `draw_frame` never read
//! `w.tcolor`/`w.bcolor` (`docs/api/aes/window-graphics.md`, "`WF_COLOR`/
//! `WF_DCOLOR` → pixel mapping (design, A79)"). Since the user's decision of
//! 2026-09-20 there is one look only: every gadget is painted from its word,
//! so the compiled defaults render the picture EmuTOS itself draws — measured
//! on a Hatari Falcon with the pinned EmuTOS build,
//! `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/` (README + `shots/`).
//! Where a case below hard-codes a colour, the expected pixel was read off
//! `shots/falcon-tc-window-c.png` (the `C:\` window at `(16,96,304,192)`, so
//! frame-relative `(dx,dy)` is screen `(16+dx, 96+dy)` there) or
//! `shots/falcon-tc-two-windows.png` for the untopped window.
//!
//! Cases (spec § (4) worked examples + the lead's test list):
//!
//! * **T7** — `wind_set(h, WF_COLOR, W_NAME, 0x1172, 0x1172)` on an open,
//!   topped window: `gr_crack(0x1172)` = `bc=1, tc=1, md=TRANS, ip=IP_SOLID,
//!   ic=2` → the name band's interior turns colour 2 (RED in the test
//!   palette), its 1px border and the name glyphs stay BLACK, and — `md=0` —
//!   no white glyph-cell boxes. Red before the fix: 0 pixels changed.
//! * **W_VSLIDE `0x1171`** — `ip=IP_SOLID, ic=1` → the vertical slider's
//!   track is solid BLACK outside the thumb.
//! * **`WF_DCOLOR`** — changes only what `wind_create` copies into windows
//!   created *afterwards* (`w_setup`, `gemwmlib.c:220-227`); an existing
//!   window keeps its words and pixels even when its frame is re-emitted.
//! * **Default topped window** — the compiled words rendered as `just_draw`
//!   renders them: hollow (white) faces, 1px black borders, the `IP_2PATT`
//!   stipple on the topped name, `IP_1PATT` on the slider tracks, the bevel
//!   on every `FL3DACT` gadget; expectations written from the spec's
//!   per-gadget table and checked against the reference PNG, not copied from
//!   the output.
//! * **Default untopped window** — the untopped words: name band hollow
//!   white with a black border and black transparent text, no stipple; closer
//!   with border, bevel and white face; info band white.
//! * **Frame hash guard** — a whole-screen FNV-1a hash of two default windows
//!   (one untopped, one topped), pinned so an unintended change to any frame
//!   pixel fails loudly (see [`FRAME_HASH`] for how it was measured).
//!
//! Harness (raster stand-in + host + parameter block) copied from
//! `aes_window_move_blit.rs`, with one change: `draw_glyph_sized` honours
//! `WriteMode::Replace` (clear bits painted in palette colour 0), because the
//! `md` half of a colour word is exactly the difference between an opaque and
//! a transparent glyph cell.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

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

    /// FNV-1a over every device pixel — the "same picture" oracle.
    fn hash(&self) -> u64 {
        let px = self.px.lock().unwrap();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &v in px.iter() {
            for b in v.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
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
    /// Honours `Replace`: clear bits of the `cell_w`×`cell_h` cell are painted
    /// in palette colour 0 (the VDI background pen), set bits in `fg`; every
    /// other mode paints the set bits only.
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
const INFO: i16 = 0x0010;
const SIZER: i16 = 0x0020;
const UPARROW: i16 = 0x0040;
const DNARROW: i16 = 0x0080;
const VSLIDE: i16 = 0x0100;
const LFARROW: i16 = 0x0200;
const RTARROW: i16 = 0x0400;
const HSLIDE: i16 = 0x0800;
/// The A59 survey window kind: everything but `INFO`.
const KIND_0FEF: i16 =
    NAME | CLOSER | FULLER | MOVER | SIZER | UPARROW | DNARROW | VSLIDE | LFARROW | RTARROW | HSLIDE;
/// Every gadget, `INFO` included — the frame with all nineteen elements.
const KIND_ALL: i16 = KIND_0FEF | INFO;

const WF_NAME: i16 = 2;
const WF_INFO: i16 = 3;
const WF_TOP: i16 = 10;
const WF_NEWDESK: i16 = 14;
const WF_COLOR: i16 = 18;
const WF_DCOLOR: i16 = 19;

// gadget indices (`aes/gemlib.h:38-56`)
const W_BOX: i16 = 0;
const W_NAME: i16 = 3;
const W_VSLIDE: i16 = 12;

// palette indices the AES paints with (`obdefs.h:109-124`), as the test
// palette below maps them
const WHITE: Pel = 0;
const BLACK: Pel = 1;
const RED: Pel = 2;
const BLUE: Pel = 4;
const CYAN: Pel = 5;
const YELLOW: Pel = 6;
const LWHITE: Pel = 8;
const LBLACK: Pel = 9;

// frame metrics for the 8×16 system font on a ≥ 640-wide screen
// (`gl_wbox`/`gl_hbox` = 19, `ADJ3DSTD` = 2, `adj_wbox`/`adj_hbox` = 23)
const GL_WBOX: i16 = 19;
const GL_HBOX: i16 = 19;
const ADJ3DSTD: i16 = 2;
const ADJ_WBOX: i16 = GL_WBOX + 2 * ADJ3DSTD;
const ADJ_HBOX: i16 = GL_HBOX + 2 * ADJ3DSTD;

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
/// Index 2 is RED (1000, 0, 0) — the colour T7's `ic=2` names.
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

/// A GEM `OBJECT` at the kernel/LP64 record width (`tos-api/src/aes/object.rs:31`).
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

const G_BOX: u16 = 20;

/// A desktop-background tree, as an application registers one with
/// `wind_set(0, WF_NEWDESK, tree, root)`: one patterned box over the screen.
fn desk_tree() -> &'static [Obj] {
    let bg = 0x0000_0043u64; // pattern 4, interior 3 (green) — a dithered desktop
    Box::leak(Box::new([Obj {
        next: -1,
        head: -1,
        tail: -1,
        otype: G_BOX,
        flags: 0,
        state: 0,
        spec: bg,
        x: 0,
        y: 0,
        w: SCREEN_W as i16,
        h: SCREEN_H as i16,
    }]))
}

/// Fresh AES window state (`wind_new`, EmuTOS `wm_new`, `aes/gemwmlib.c:
/// 1875-1893`), a cleared framebuffer, the registered desktop background
/// tree, and the compiled default colour words restored.
fn fresh() -> (&'static QuietHost, Aespb) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    host.raster.clear();

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;

    // `WF_DCOLOR` is process-wide state; put every slot back to `wm_start`'s
    // values so a case that changed a default cannot leak into the next.
    for g in 0..19 {
        let (t, b) = match g {
            3 => (0x11a1, 0x1101),
            12 | 17 => (0x1111, 0x1111),
            _ => (0x1101, 0x1101),
        };
        set_dcolor(&mut pb, g, t, b);
    }
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
    pb.call(100, 5, 1, 0); // wind_create
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(101, 5, 1, 0); // wind_open
    assert_eq!(pb.intout[0], 1, "wind_open({h})");
    h
}

/// `wind_set(h, WF_NAME | WF_INFO, string)` in the native (`addr_in[0]`) form.
fn set_string(pb: &mut Aespb, handle: i16, field: i16, s: &'static [u8]) {
    assert_eq!(s.last(), Some(&0), "string must be NUL-terminated");
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.addrin[0] = s.as_ptr() as u64;
    pb.call(105, 6, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, {field}, string)");
}

/// `wind_set(h, WF_TOP)`.
fn set_top(pb: &mut Aespb, handle: i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_TOP;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, WF_TOP)");
}

/// `wind_set(h, WF_COLOR, gadget, tcol, bcol)`.
fn set_color(pb: &mut Aespb, handle: i16, gadget: i16, tcol: i16, bcol: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_COLOR;
    pb.intin[2] = gadget;
    pb.intin[3] = tcol;
    pb.intin[4] = bcol;
    pb.call(105, 6, 1, 0);
    pb.intout[0]
}

/// `wind_set(0, WF_DCOLOR, gadget, tcol, bcol)`.
fn set_dcolor(pb: &mut Aespb, gadget: i16, tcol: i16, bcol: i16) {
    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_DCOLOR;
    pb.intin[2] = gadget;
    pb.intin[3] = tcol;
    pb.intin[4] = bcol;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set(WF_DCOLOR, {gadget})");
}

/// `wind_get(h, WF_COLOR, gadget)` → `(tcol, bcol)`.
fn get_color(pb: &mut Aespb, handle: i16, gadget: i16) -> (i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_COLOR;
    pb.intin[2] = gadget;
    pb.call(104, 3, 5, 0);
    (pb.intout[2], pb.intout[3])
}

// ── frame geometry the cases read pixels at ───────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct R {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

impl R {
    fn inset(self, n: i16) -> R {
        R { x: self.x + n, y: self.y + n, w: self.w - 2 * n, h: self.h - 2 * n }
    }
    fn contains(self, x: i16, y: i16) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The `W_NAME` object rect for a window whose kind has `CLOSER | FULLER |
/// NAME` — EmuTOS `w_bldactive` (`gemwmlib.c:515-537`): the closer and the
/// fuller each take a full `adj_wbox` of the title box, the name sits
/// `ADJ3DSTD` inside what is left, `gl_hbox` tall at `y = ADJ3DSTD`.
fn name_rect(b: R) -> R {
    R {
        x: b.x + ADJ_WBOX + ADJ3DSTD,
        y: b.y + ADJ3DSTD,
        w: b.w - 2 * ADJ_WBOX - 2 * ADJ3DSTD,
        h: GL_HBOX,
    }
}

/// The 3D-grown rect `just_draw` paints an `FL3DACT` object on
/// (`gemoblib.c:503-508`: `ADJ3DSTD` out on every side).
fn grown(r: R) -> R {
    r.inset(-ADJ3DSTD)
}

/// First work row for a title-bar window without `INFO` (`band_top`).
fn top_of(b: R, has_info: bool) -> i16 {
    b.y + ADJ_HBOX + if has_info { GL_HBOX } else { 0 }
}

/// The vertical slider's track for `KIND_0FEF` at `b` — `vtrack`: the bar
/// column is `adj_wbox` wide against the right edge, from the first work row
/// down to the top of the horizontal band (`vbar_rect`); the up and down
/// arrows each consume `adj_hbox` of it (`w_bldvbar`, `gemwmlib.c:412-425`).
fn vtrack_0fef(b: R) -> R {
    let top = top_of(b, false);
    let bot = b.y + b.h - 1 - ADJ_HBOX;
    let bar = R { x: b.x + b.w - ADJ_WBOX, y: top, w: ADJ_WBOX, h: bot - top + 1 };
    R { x: bar.x, y: bar.y + ADJ_HBOX, w: bar.w, h: bar.h - 2 * ADJ_HBOX }
}

/// The elevator for a default-size (`-1`) thumb at position 0 in `track` —
/// `thumb_geom` + `elev_rect` (`w_bldvbar`, `gemwmlib.c:431-436`): `gl_hbox`
/// long, inset `ADJ3DSTD` on the cross axis, `2*ADJ3DSTD + 1` shorter.
fn velev_default(track: R) -> R {
    R { x: track.x + ADJ3DSTD, y: track.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX - 2 * ADJ3DSTD - 1 }
}

fn px(host: &QuietHost, x: i16, y: i16) -> Pel {
    host.raster.read_pixel(x, y)
}

/// Every pixel of `r`, as `(x, y, pel)`.
fn pixels(host: &QuietHost, r: R) -> Vec<(i16, i16, Pel)> {
    let mut v = Vec::with_capacity(r.w as usize * r.h as usize);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            v.push((x, y, px(host, x, y)));
        }
    }
    v
}

fn count(host: &QuietHost, r: R, pel: Pel) -> usize {
    pixels(host, r).iter().filter(|p| p.2 == pel).count()
}

/// Assert every pixel of `r` is `pel`, naming the first offender.
fn assert_all(host: &QuietHost, r: R, pel: Pel, what: &str) {
    for (x, y, p) in pixels(host, r) {
        assert_eq!(p, pel, "{what}: pixel ({x},{y}) in {r:?}");
    }
}

/// Assert the 1px ring of `r` is `pel`.
fn assert_ring(host: &QuietHost, r: R, pel: Pel, what: &str) {
    assert_all(host, R { x: r.x, y: r.y, w: r.w, h: 1 }, pel, &format!("{what} top row"));
    assert_all(host, R { x: r.x, y: r.y + r.h - 1, w: r.w, h: 1 }, pel, &format!("{what} bottom row"));
    assert_all(host, R { x: r.x, y: r.y, w: 1, h: r.h }, pel, &format!("{what} left column"));
    assert_all(host, R { x: r.x + r.w - 1, y: r.y, w: 1, h: r.h }, pel, &format!("{what} right column"));
}

/// The A59 survey window.
const B7: R = R { x: 30, y: 40, w: 320, h: 240 };

// ── T7: WF_COLOR on W_NAME ────────────────────────────────────────────────

/// `wind_set(h, WF_COLOR, W_NAME, 0x1172, 0x1172)` on the open, topped survey
/// window.
#[test]
fn t7_name_band_turns_red() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF, (B7.x, B7.y, B7.w, B7.h));
    // "A A": two glyph cells with ink and one (the space) without, so the
    // `md=TRANS` claim is testable — that cell must show the fill, not white.
    set_string(&mut pb, h, WF_NAME, b"A A\0");

    let nr = name_rect(B7);
    let before = pixels(host, nr);
    assert_eq!(set_color(&mut pb, h, W_NAME, 0x1172, 0x1172), 1, "wind_set(WF_COLOR) returns 1");
    assert_eq!(get_color(&mut pb, h, W_NAME), (0x1172, 0x1172), "WF_COLOR round-trips");
    let after = pixels(host, nr);

    let changed = before.iter().zip(after.iter()).filter(|(a, b)| a.2 != b.2).count();
    assert!(changed > 0, "A79: no pixel of the name band changed after WF_COLOR (was {changed})");

    // The word is `bc=1, tc=1, md=TRANS, ip=IP_SOLID, ic=2` — interior 100 %
    // colour 2 with black glyph ink, 1px black border, no white anywhere.
    // `W_NAME` is `FL3DACT` (`w_bldactive`, `gemwmlib.c:553`), so `just_draw`
    // paints it on the 3D-grown rect (`gemoblib.c:503-508`) and ends with the
    // 1-px `add_3d_effect` bevel inside the border (`:949-950`; WHITE top/
    // left, LBLACK bottom/right — `LBLACK != icol`, `:271-272`), A95 (2).
    // The colour claims hold inside that bevel.
    let box_r = grown(nr);
    assert_ring(host, box_r, BLACK, "T7 name border (bc=1)");
    assert_bevel(host, box_r.inset(1), "T7 name (FL3DACT)");
    let inner = box_r.inset(2);
    let red = count(host, inner, RED);
    let black = count(host, inner, BLACK);
    let white = count(host, inner, WHITE);
    assert_eq!(white, 0, "T7: md=TRANS leaves no white glyph-cell boxes (white={white})");
    assert!(black > 0, "T7: the name glyphs are drawn in tc=1 (black={black})");
    assert!(
        red > (inner.w as usize * inner.h as usize) * 3 / 4,
        "T7: the interior is colour 2 at 100 % coverage outside the glyph ink (red={red} of {})",
        inner.w as usize * inner.h as usize
    );
    // The middle glyph cell (the space) carries no ink: in transparent mode
    // it is pure fill.
    let nlen = 3 * 8;
    let nx = nr.x + ((nr.w - nlen) / 2).max(2);
    let space = R { x: nx + 8, y: nr.y + 1, w: 8, h: 16 };
    assert_all(host, space, RED, "T7: the space glyph cell shows the red fill through (md=TRANS)");
}

// ── W_VSLIDE 0x1171: solid black track ────────────────────────────────────

#[test]
fn vslide_track_turns_black() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF, (B7.x, B7.y, B7.w, B7.h));
    let track = vtrack_0fef(B7);
    let thumb = grown(velev_default(track));

    let before_black = pixels(host, track).iter().filter(|p| !thumb.contains(p.0, p.1) && p.2 == BLACK).count();
    assert_eq!(set_color(&mut pb, h, W_VSLIDE, 0x1171, 0x1171), 1);

    // `gr_crack(0x1171)` = `bc=1, tc=1, md=TRANS, ip=IP_SOLID, ic=1`: the
    // track is 100 % BLACK (fill and border alike) everywhere the elevator —
    // drawn after it, `w_bldvbar` `gemwmlib.c:428-436` — does not cover.
    let mut outside = 0usize;
    for (x, y, p) in pixels(host, track) {
        if thumb.contains(x, y) {
            continue;
        }
        outside += 1;
        assert_eq!(p, BLACK, "W_VSLIDE 0x1171: track pixel ({x},{y}) outside the thumb {thumb:?}");
    }
    assert!(outside > before_black, "the track had {before_black} black pixels before, {outside} after");
}

// ── WF_DCOLOR: later windows only ─────────────────────────────────────────

#[test]
fn dcolor_reaches_new_windows_only() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    // Two side-by-side windows that do not overlap.
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_0FEF, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"ONE\0");

    // Change the *default* for W_NAME (topped and untopped) — no redraw
    // (`gemwmlib.c:1727-1734`, `gadget = -1`), no effect on `h1`'s words.
    set_dcolor(&mut pb, W_NAME, 0x1172, 0x1172);
    assert_eq!(get_color(&mut pb, h1, W_NAME), (0x11a1, 0x1101), "an existing window keeps its words");
    let n1 = grown(name_rect(b1));
    assert_eq!(count(host, n1, RED), 0, "h1's name band is untouched by WF_DCOLOR");

    // A window created afterwards inherits the new default (`w_setup`,
    // `gemwmlib.c:220-227`) and paints with it.
    let h2 = open_window(&mut pb, KIND_0FEF, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"TWO\0");
    assert_eq!(get_color(&mut pb, h2, W_NAME), (0x1172, 0x1172), "a new window copies gl_wtcolor/gl_wbcolor");
    let n2 = grown(name_rect(b2));
    assert_ring(host, n2, BLACK, "h2 name border");
    let inner2 = n2.inset(1);
    assert!(
        count(host, inner2, RED) > (inner2.w as usize * inner2.h as usize) * 3 / 4,
        "h2's name band renders the inherited word 0x1172"
    );

    // Re-emitting h1's frame (a WF_NAME change redraws the frame, like
    // WF_COLOR does) still paints h1 with its own, unchanged words.
    set_string(&mut pb, h1, WF_NAME, b"ONE!\0");
    assert_eq!(count(host, n1, RED), 0, "h1's re-emitted frame still uses h1's words, not the new default");
}

// ── the drop shadow takes W_BOX's border colour (A131) ────────────────────

/// The two `SHADOWED` bands of a window with border rect `b` — `just_draw`
/// after `gr_inside(&t, th)` with `th = 1` (`W_BOX`'s `ob_spec 0x00011101`,
/// `gemwmlib.c:84-86`): `bb_fill(t.x, t.y+t.h+th, t.w+th, 2*th)` and
/// `bb_fill(t.x+t.w+th, t.y, 2*th, t.h+3*th)` (`gemoblib.c:894-900`). The
/// two `th`-wide corner notches belong to neither band.
fn shadow_bands(b: R) -> [R; 2] {
    [
        R { x: b.x + 1, y: b.y + b.h, w: b.w - 1, h: 2 },
        R { x: b.x + b.w, y: b.y + 1, w: 2, h: b.h + 1 },
    ]
}

/// Assert both shadow bands of `b` — and the stretch of `W_BOX`'s own border
/// no child covers — are `pel`: `just_draw` strokes the `G_IBOX` border in
/// `bcol` (`gemoblib.c:614,665`) and fills the `SHADOWED` bands with the same
/// `bcol` (`:894-900`). The children are drawn after their parent and their
/// `0x1101` rings overdraw the rest of the `W_BOX` ring — `W_TITLE` the top
/// row and the band's sides, `W_VBAR`/`W_HBAR` the right column and the bottom
/// row (`w_bldactive`, `gemwmlib.c:514-517,585-616`); `W_DATA`/`W_WORK` have
/// thickness 0 (`gl_waspec`, `:84-106`), so the left column between the title
/// band and the horizontal bar shows `W_BOX`'s `bc`. Kind `KIND_0FEF` (no
/// `INFO`) only.
fn assert_box_and_shadow(host: &QuietHost, b: R, pel: Pel, what: &str) {
    let left = R { x: b.x, y: b.y + ADJ_HBOX, w: 1, h: b.h - 2 * ADJ_HBOX };
    assert_all(host, left, pel, &format!("{what}: W_BOX left border below the title band"));
    let [bottom, right] = shadow_bands(b);
    assert_all(host, bottom, pel, &format!("{what}: bottom shadow band"));
    assert_all(host, right, pel, &format!("{what}: right shadow band"));
}

/// Paint the whole screen in `pel`, so a shadow band the frame never paints
/// cannot pass for BLACK (the cleared buffer reads back as pen 1).
fn prefill(host: &QuietHost, pel: Pel) {
    host.raster.fill_rect(0, 0, SCREEN_W as i16, SCREEN_H as i16, pel, ClipRect::full(SCREEN_W, SCREEN_H));
}

/// With the compiled default words the shadow is pen 1 = BLACK, topped and
/// untopped alike: `bcol` of `0x1101` (`gl_wtcolor`/`gl_wbcolor[W_BOX]`,
/// `gemwmlib.c:1347-1357`; `gr_crack` bits 15..12, `gemgraf.c:612-615`).
/// Measured on the oracle: `measurements/measurement-2026-11-05-a131/`
/// (`shprobe` s0-s4, shadow `0,0,0` on tt and falcontc). Before A131 NewTOS
/// painted pen 8 here.
#[test]
fn drop_shadow_is_pen_1_by_default() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    prefill(host, CYAN);
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_0FEF, (b1.x, b1.y, b1.w, b1.h));
    assert_eq!(get_color(&mut pb, h1, W_BOX), (0x1101, 0x1101), "default W_BOX words");
    assert_box_and_shadow(host, b1, BLACK, "topped default window");

    // A second window on top repaints h1 untopped (A94); its word is 0x1101
    // too, so the shadow stays BLACK.
    open_window(&mut pb, KIND_0FEF, (b2.x, b2.y, b2.w, b2.h));
    assert_box_and_shadow(host, b1, BLACK, "untopped default window");
    assert_box_and_shadow(host, b2, BLACK, "topped second window");
}

/// `wind_set(h, WF_COLOR, W_BOX, tcol, bcol)` recolours the border **and** the
/// shadow, per state: `w_setcolor(pw, W_BOX, istop)` writes `istop ?
/// w_tcolor : w_bcolor` into `W_BOX`'s `ob_spec` on every `w_bldactive`
/// (`gemwmlib.c:382-397,510`), and `wm_set` re-walks the frame
/// (`do_cpwalk`, `:1719-1726,1754-1755`). Oracle: `shprobe` s5
/// (`0x2101` on `W_BOX` → shadow `255,0,0` on EmuTOS,
/// `measurements/measurement-2026-11-05-a131/`).
#[test]
fn drop_shadow_follows_the_w_box_border_colour() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    prefill(host, CYAN);
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_0FEF, (b1.x, b1.y, b1.w, b1.h));

    // topped word bc=2 (RED), untopped word bc=4 (BLUE)
    assert_eq!(set_color(&mut pb, h1, W_BOX, 0x2101, 0x4101), 1, "wind_set(WF_COLOR, W_BOX) returns 1");
    assert_eq!(get_color(&mut pb, h1, W_BOX), (0x2101, 0x4101), "WF_COLOR round-trips");
    assert_box_and_shadow(host, b1, RED, "topped h1 after WF_COLOR");

    // h2 on top: h1 is repainted with its untopped word.
    let h2 = open_window(&mut pb, KIND_0FEF, (b2.x, b2.y, b2.w, b2.h));
    assert_box_and_shadow(host, b1, BLUE, "untopped h1");
    assert_box_and_shadow(host, b2, BLACK, "h2 keeps the default word");

    // Topping h1 again brings back the topped word; h2 goes untopped, still
    // on its own default word.
    set_top(&mut pb, h1);
    assert_box_and_shadow(host, b1, RED, "re-topped h1");
    assert_box_and_shadow(host, b2, BLACK, "untopped h2");
    let _ = h2;
}

/// `WF_DCOLOR` on `W_BOX` reaches the shadow of windows created afterwards
/// (`w_setup` copies `gl_wtcolor`/`gl_wbcolor`, `gemwmlib.c:220-227`; the
/// `SHADOWED` bands use that word's `bcol`, `gemoblib.c:894-900`). Source
/// only — the A131 oracle run read `WF_DCOLOR` but did not set it.
#[test]
fn drop_shadow_of_a_new_window_follows_wf_dcolor() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    prefill(host, CYAN);
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    set_dcolor(&mut pb, W_BOX, 0x6101, 0x6101);
    let h1 = open_window(&mut pb, KIND_0FEF, (b1.x, b1.y, b1.w, b1.h));
    assert_eq!(get_color(&mut pb, h1, W_BOX), (0x6101, 0x6101), "the new window copies the default");
    assert_box_and_shadow(host, b1, YELLOW, "window created after WF_DCOLOR(W_BOX, 0x6101)");
}

// ── frame hash guard ──────────────────────────────────────────────────────

/// Whole-screen FNV-1a hash of [`default_frames`] under the literal look —
/// the one look there is since the user's decision of 2026-09-20. First
/// measured 2026-09-20 on the tree with the `FrameLook` switch removed
/// (`0x259e_dc0b_844d_91a0`). Re-pin it only with an EmuTOS-backed reason
/// for the pixel change (and say which pixels).
///
/// **Re-pinned 2026-09-21 (A94).** `wind_open` of `h2` now repaints the old
/// top `h1` untopped through its own rectangle list — EmuTOS `draw_change`'s
/// `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)` on an open (`gemwmlib.c:1203-1207`),
/// `w_bldactive` with `istop` false (`:495,537`). The pixel change against
/// the previous pin, measured by a scratch diff of the two renderings: 1236
/// pixels, all inside `h1`'s name band (x 65..375, y 61..81), all on odd rows
/// at odd columns — the `IP_2PATT` ink of the topped word `0x11a1` — black
/// → white (the untopped word `0x1101`, hollow). Nothing else moved. Measured
/// on this binary alone, twice, and under `--shuffle-seed 7` / `99` with
/// `--test-threads=1` — the same value every time.
///
/// **Re-pinned 2026-09-21 (A95).** The frame now follows `just_draw` pixel
/// for pixel on the six points of `TASK/A95.md`: the 1-px `add_3d_effect`
/// polylines (`gemoblib.c:238-276`) instead of a 2-px band, the bevel on the
/// `FL3DACT` name box (`:949-950`), the `G_BOXCHAR` glyph centred in the
/// grown cell with `gr_just`'s `(w+1)/2` after the activator's `y-1`
/// (`:735-757`, `gemgraf.c:567-593`), the arrow cells and tracks laid into
/// the `W_VBAR`/`W_HBAR` rects (`gemwmlib.c:412-474`, `:606-616`), and the
/// `DISABLED` overlay on the untopped name (`:537`, `gemoblib.c:917-930`).
/// Scratch diff of the two renderings against the previous pin: 2859
/// pixels, every one inside a gadget cell or name box of `h1`/`h2` (bbox
/// `(41,61)-(578,438)`), plus `h1`'s bottom-right gadgets as first drawn,
/// which survive under `h2`'s never-painted work area; transitions BLACK→
/// WHITE 909 (old glyph columns, the stipple row/column under the name's
/// white bevel), LBLACK→WHITE 674 (the second bevel line), WHITE→LBLACK 505
/// and BLACK→LBLACK 255 (the name box's bevel, the moved arrow slots),
/// WHITE→BLACK 428 (new glyph columns, the track's extra row/column),
/// LBLACK→BLACK 88. Nothing outside the two frames moved. Measured on this
/// binary alone, twice, and under `--shuffle-seed 7` / `99` with
/// `--test-threads=1` — the same value every time.
///
/// **Re-pinned 2026-09-21 (A105).** The info line's text now sits where
/// `just_draw` puts a `G_BOXTEXT`'s: `gr_gtext(TE_LEFT, IBM, …,
/// gr_inside(&t, 1))` with `gr_just`'s `(17 - 16 + 1) / 2` row
/// (`gemoblib.c:735-738`, `gemgraf.c:574-576`; `gl_ainfo` `TE_LEFT`,
/// `te_thickness 1`, `gemwmlib.c:154-157`) — glyph cells at `(1, 2)` of the
/// info box instead of `(3, 1)`. Scratch diff of the two renderings against
/// the previous pin: 2160 pixels, bbox `(43,86)-(385,200)`, every one inside
/// the two info lines (`h1` rows 86..98, `h2` rows 186..200), exactly 1080
/// BLACK→WHITE and 1080 WHITE→BLACK — the same glyph ink two columns left
/// and one row down. Nothing else moved (the name text's `gr_just` centring
/// coincides with the old formula for these even-width windows). Measured on
/// this binary alone, twice, and under `--shuffle-seed 7` / `99` with
/// `--test-threads=1` — the same value every time.
///
/// **Re-pinned 2026-11-05 (A131).** The drop shadow is painted in `W_BOX`'s
/// border colour — the `bcol` `just_draw` cracked from the `G_IBOX`'s word
/// (`gemoblib.c:614,894-900`), `0x1101` → pen 1 — instead of pen 8 (the
/// light-grey decision of 2026-09-10, revoked by the user on 2026-11-05;
/// oracle: `measurements/measurement-2026-11-05-a131/`). Scratch diff of the
/// two renderings against the previous pin: 2474 pixels, bbox
/// `(41,61)-(581,441)`, every one LWHITE→BLACK and every one inside a shadow
/// band — `h2`'s 1320 and the 1154 of `h1`'s 1240 that `h2` does not cover.
/// Nothing else moved. Measured on this binary alone, twice, and under
/// `--shuffle-seed 7` / `99` with `--test-threads=1` — the same value every
/// time.
const FRAME_HASH: u64 = 0xcbe0_43f3_9761_0440;

/// Two default windows with every gadget, names and info lines: `h1`
/// untopped (partly covered, repainted untopped by `h2`'s open — A94), `h2`
/// topped.
fn default_frames(pb: &mut Aespb) {
    let h1 = open_window(pb, KIND_ALL, (40, 60, 360, 260));
    set_string(pb, h1, WF_NAME, b"Untopped window\0");
    set_string(pb, h1, WF_INFO, b"info of the lower window\0");
    let h2 = open_window(pb, KIND_ALL, (200, 160, 380, 280));
    set_string(pb, h2, WF_NAME, b"Topped window\0");
    set_string(pb, h2, WF_INFO, b"12 items in 34567 bytes\0");
}

#[test]
fn default_frames_are_pixel_identical_to_the_pinned_hash() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    default_frames(&mut pb);
    let got = host.raster.hash();
    assert_eq!(
        got, FRAME_HASH,
        "the default frames must render exactly as pinned (got {got:#018x}, pinned {FRAME_HASH:#018x})"
    );
}

// ── the default topped window against the gadget table ────────────────────

/// The `IP_2PATT` dither (`vdi/vdi_fill.c` DITHER row 2, `[0x0000, 0x5555,
/// 0x0000, 0x5555]`) at screen-aligned `(x, y)`: ink on odd rows at odd
/// columns. Reference: `falcon-tc-window-c.png` name band, rows 99/101/…
/// carry `#` at x = 41, 43, … (odd), rows 98/100/… none.
fn ip2_is_ink(x: i16, y: i16) -> bool {
    y.rem_euclid(4) % 2 == 1 && x.rem_euclid(16) % 2 == 1
}

/// The `IP_1PATT` dither (`[0x0000, 0x4444, 0x0000, 0x1111]`): row 1 inks
/// columns 1, 5, 9, 13; row 3 inks 3, 7, 11, 15.
fn ip1_is_ink(x: i16, y: i16) -> bool {
    let row = match y.rem_euclid(4) {
        1 => 0x4444u16,
        3 => 0x1111u16,
        _ => 0,
    };
    row & (0x8000 >> (x.rem_euclid(16) as u16)) != 0
}

/// Assert the bevel `add_3d_effect` leaves inside a 1px border: WHITE along
/// the top and left, LBLACK along the bottom and right (`gemoblib.c:238-276`,
/// colours fixed at `:252,270-272`; reference `falcon-tc-window-c.png`
/// closer: white at x=17 / y=97, `(140,138,140)` at x=37 / y=117).
///
/// Samples the outermost pixel row/column of each edge, two pixels clear of
/// the corners; the exact 1-px width and the corner ownership are pinned by
/// `aes_window_frame_reference.rs` (A95).
fn assert_bevel(host: &QuietHost, ci: R, what: &str) {
    assert_all(host, R { x: ci.x + 2, y: ci.y, w: ci.w - 4, h: 1 }, WHITE, &format!("{what} bevel top"));
    assert_all(host, R { x: ci.x, y: ci.y + 2, w: 1, h: ci.h - 4 }, WHITE, &format!("{what} bevel left"));
    assert_all(host, R { x: ci.x + 2, y: ci.y + ci.h - 1, w: ci.w - 4, h: 1 }, LBLACK, &format!("{what} bevel bottom"));
    assert_all(host, R { x: ci.x + ci.w - 1, y: ci.y + 2, w: 1, h: ci.h - 4 }, LBLACK, &format!("{what} bevel right"));
}

/// A default `KIND_ALL` window rendered from its compiled colour words the
/// way `just_draw` paints the `W_ACTIVE` tree (spec § (2) per-gadget table).
/// Every colour named here was read off the reference PNG at the frame-
/// relative position given in the comment (`falcon-tc-window-c.png`, frame
/// origin `(16,96)`).
#[test]
fn default_topped_window_matches_the_gadget_table() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b = R { x: 30, y: 40, w: 320, h: 240 };
    let h = open_window(&mut pb, KIND_ALL, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"A A\0");
    set_string(&mut pb, h, WF_INFO, b"i\0");

    // W_BOX (0): 0x1101, th=1 → bc=1 border ring. Reference: row 96 and
    // column 16 are black over the whole frame.
    assert_ring(host, b, BLACK, "W_BOX border");

    // W_TITLE (1): G_BOX 0x1101 at (0, 0, w, adj_hbox) → black ring, hollow
    // (= white, `gr_rect` FIS_HOLLOW in MD_REPLACE) inside. Reference: the
    // title's bottom border is row 118 = 96 + 23 - 1; the info box's top
    // border follows at row 119 (a double black line is EmuTOS-correct).
    let title = R { x: b.x, y: b.y, w: b.w, h: ADJ_HBOX };
    assert_ring(host, title, BLACK, "W_TITLE border");
    assert_all(host, R { x: b.x, y: b.y + ADJ_HBOX - 1, w: b.w, h: 1 }, BLACK, "title bottom border (ref row 118)");
    assert_all(host, R { x: b.x, y: b.y + ADJ_HBOX, w: b.w, h: 1 }, BLACK, "info top border (ref row 119)");

    // W_CLOSER (2): G_BOXCHAR 0x1101, FL3DACT → grown to the full adj cell at
    // the title's top-left: black ring (reference: column 38 = 16 + 22 is
    // black from row 96 to 118), bevel inside the border, white face, black
    // glyph.
    let closer = grown(R { x: b.x + ADJ3DSTD, y: b.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, closer, BLACK, "W_CLOSER border");
    assert_all(host, R { x: b.x + ADJ_WBOX - 1, y: b.y, w: 1, h: ADJ_HBOX }, BLACK, "closer right border (ref col 38)");
    let ci = closer.inset(1);
    assert_bevel(host, ci, "W_CLOSER");
    let face = ci.inset(2);
    assert!(count(host, face, BLACK) > 0, "W_CLOSER glyph in tc=1");
    assert_eq!(count(host, face, LWHITE), 0, "W_CLOSER face is hollow-white, not the 3D grey");
    assert_eq!(count(host, face, BLACK) + count(host, face, WHITE), face.w as usize * face.h as usize);

    // W_NAME (3), topped: G_BOXTEXT 0x11a1 → bc=1 ring on the grown rect —
    // its left column stands right next to the closer's right border
    // (reference: columns 38 and 39 both black, the EmuTOS double line) —
    // IP_2PATT in ic=1 inside, glyphs in tc=1 with md=REPLACE (opaque cells).
    let nr = name_rect(b);
    let nbox = grown(nr);
    assert_ring(host, nbox, BLACK, "W_NAME border");
    assert_eq!(nbox.x, closer.x + closer.w, "name box starts on the column after the closer's border");
    let nlen = 3 * 8;
    let nx = nr.x + ((nr.w - nlen) / 2).max(2);
    let cells = R { x: nx, y: nr.y + 1, w: nlen, h: 16 };
    // The reference's name band also carries the FL3DACT bevel (white row 97
    // / column 40, LBLACK row 117 / right column) that NewTOS does not draw
    // on the name yet (A95); the stipple is sampled two pixels inside the
    // border so the assertion holds for both renderings.
    for (x, y, p) in pixels(host, nbox.inset(3)) {
        if cells.contains(x, y) {
            continue;
        }
        let want = if ip2_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "W_NAME IP_2PATT at ({x},{y})");
    }
    // The space cell: md=REPLACE paints its clear bits white — no stipple.
    assert_all(host, R { x: nx + 8, y: nr.y + 1, w: 8, h: 16 }, WHITE, "W_NAME md=REPLACE glyph cell");

    // W_INFO (5): G_BOXTEXT 0x1101 at (0, adj_hbox, w, gl_hbox) → black
    // ring, white inside, glyph in black transparent. Reference: rows 120-136
    // white apart from the text, row 137 black.
    let info = R { x: b.x, y: b.y + ADJ_HBOX, w: b.w, h: GL_HBOX };
    assert_ring(host, info, BLACK, "W_INFO border");
    let ii = info.inset(1);
    assert_eq!(count(host, ii, LWHITE), 0, "W_INFO is hollow-white, not grey");
    assert!(count(host, ii, BLACK) > 0, "W_INFO glyph");
    assert_all(host, R { x: ii.x + ii.w - 40, y: ii.y, w: 40, h: ii.h }, WHITE, "W_INFO interior right of the text");

    // W_VBAR (9): G_BOX 0x1101 — `w_bldactive`'s bar column, `adj_wbox` wide
    // against the right edge, from the first work row down to (and sharing)
    // the horizontal bar's top row (`gemwmlib.c:585-616`).
    let top = top_of(b, true);
    let row_y = b.y + b.h - ADJ_HBOX;
    let vbar = R { x: b.x + b.w - ADJ_WBOX, y: top, w: ADJ_WBOX, h: row_y - top + 1 };
    assert_all(host, R { x: vbar.x, y: vbar.y, w: 1, h: vbar.h }, BLACK, "W_VBAR left border");
    assert_all(host, R { x: vbar.x, y: vbar.y, w: vbar.w, h: 1 }, BLACK, "W_VBAR top border");
    // W_UPARROW (10) grown fills the column below the bar's top row.
    let up = grown(R { x: vbar.x + ADJ3DSTD, y: vbar.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, up, BLACK, "W_UPARROW border");
    assert_bevel(host, up.inset(1), "W_UPARROW");
    assert_eq!(count(host, up.inset(3), LWHITE), 0, "W_UPARROW face is white");

    // W_VSLIDE (12): G_BOX 0x1111 → black ring, IP_1PATT in ic=1 inside,
    // outside the (grown) elevator. `w_bldvbar` gives it what is left of the
    // `W_VBAR` rect after two `adj_hbox` arrows (`gemwmlib.c:415-429`): its
    // bottom border is the down arrow's grown top border (`:424`; reference
    // row 242 = 96 + 192 - 46), A95 (4).
    let track = R { x: vbar.x, y: vbar.y + ADJ_HBOX, w: vbar.w, h: vbar.h - 2 * ADJ_HBOX };
    assert_ring(host, track, BLACK, "W_VSLIDE border");
    let elev = grown(velev_default(track));
    for (x, y, p) in pixels(host, track.inset(1)) {
        if elev.contains(x, y) {
            continue;
        }
        let want = if ip1_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "W_VSLIDE IP_1PATT at ({x},{y})");
    }
    // W_VELEV (13): 0x1101 FL3DACT → black ring on the grown rect, bevel,
    // white face.
    assert_ring(host, elev, BLACK, "W_VELEV border");
    assert_bevel(host, elev.inset(1), "W_VELEV");
    assert_all(host, elev.inset(3), WHITE, "W_VELEV face");

    // W_HBAR (14): G_BOX 0x1101 — from the left edge to (and sharing) the
    // vertical bar's left column, `adj_hbox` tall against the bottom edge.
    let hbar = R { x: b.x, y: row_y, w: b.w - (ADJ_WBOX - 1), h: ADJ_HBOX };
    assert_all(host, R { x: hbar.x, y: hbar.y, w: hbar.w, h: 1 }, BLACK, "W_HBAR top border");
    assert_all(host, R { x: hbar.x + hbar.w - 1, y: hbar.y, w: 1, h: hbar.h }, BLACK, "W_HBAR right border");
    // W_LFARROW (15) at `(x+ADJ3DSTD, ADJ3DSTD)` of the band, `x = 0`
    // (`w_bldhbar`, `gemwmlib.c:452`): its grown slot starts on the window's
    // left border column (reference columns 16..38), A95 (5).
    let lf = grown(R { x: hbar.x + ADJ3DSTD, y: hbar.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, lf, BLACK, "W_LFARROW border");
    assert_bevel(host, lf.inset(1), "W_LFARROW");

    // W_SIZER (8): 0x1101 FL3DACT with the sizer glyph → grown to the whole
    // corner: black ring, bevel, white face, black glyph.
    let sizer = grown(R { x: b.x + b.w - GL_WBOX - ADJ3DSTD, y: b.y + b.h - GL_HBOX - ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, sizer, BLACK, "W_SIZER border");
    assert_bevel(host, sizer.inset(1), "W_SIZER");
    let sf = sizer.inset(3);
    assert!(count(host, sf, BLACK) > 0, "W_SIZER glyph");
    assert_eq!(count(host, sf, LWHITE), 0, "W_SIZER face is white");

    // Nothing in the frame is the 3D grey: no default word has ic=WHITE, so
    // the `actbutcol`/`backgrcol` substitution never fires (spec § (1)).
    // Reference: the only greys in the whole PNG are the bevel shadow
    // `(140,138,140)` and the desktop's icon text.
    let frame_rows = [
        R { x: b.x, y: b.y, w: b.w, h: ADJ_HBOX + GL_HBOX },
        R { x: b.x, y: row_y, w: b.w, h: ADJ_HBOX },
        R { x: vbar.x, y: vbar.y, w: vbar.w, h: vbar.h },
    ];
    for r in frame_rows {
        assert_eq!(count(host, r, LWHITE), 0, "no LWHITE anywhere in the frame {r:?}");
    }
}

// ── the default untopped window ───────────────────────────────────────────

/// The untopped words (`gl_wbcolor`, every slot `0x1101`, sliders `0x1111`)
/// on a window another window has topped. Reference:
/// `falcon-tc-two-windows.png` / `falcon-tc-untopped-title-zoom5x.png` — the
/// untopped `C:\` window keeps white faces and bands, a black-bordered hollow
/// name band without the `IP_2PATT` stipple, its closer with border and bevel.
///
/// EmuTOS additionally draws the untopped name `DISABLED` (`w_bldactive`,
/// `gemwmlib.c:537`; `just_draw` overlays `IP_4PATT` in WHITE, transparent,
/// `gemoblib.c:917-930`), which thins the glyphs to a dither — pinned by
/// `aes_window_frame_reference.rs` (A95 (6)); the assertions below only
/// require the glyph cells to be black/white.
#[test]
fn default_untopped_window_renders_the_untopped_words() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    // Two side-by-side windows that do not overlap, h1 opened last (on top);
    // a `WF_TOP` on h2 then repaints h1 "covered with deactivated borders"
    // (`draw_change`, `gemwmlib.c:1120-1122`) — the untopped frame in full.
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    set_top(&mut pb, h2);
    assert_eq!(get_color(&mut pb, h1, W_NAME), (0x11a1, 0x1101), "h1 carries the compiled words");

    // W_BOX / W_TITLE: the same black boxes as when topped.
    assert_ring(host, b1, BLACK, "untopped W_BOX border");
    assert_ring(host, R { x: b1.x, y: b1.y, w: b1.w, h: ADJ_HBOX }, BLACK, "untopped W_TITLE border");

    // W_CLOSER: `0x1101`, FL3DACT — border, bevel, white face, black glyph
    // (reference: the untopped closer at the top-left of the two-windows
    // shot looks exactly like the topped one).
    let closer = grown(R { x: b1.x + ADJ3DSTD, y: b1.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, closer, BLACK, "untopped W_CLOSER border");
    let ci = closer.inset(1);
    assert_bevel(host, ci, "untopped W_CLOSER");
    let face = ci.inset(2);
    assert!(count(host, face, BLACK) > 0, "untopped W_CLOSER glyph");
    assert_eq!(count(host, face, BLACK) + count(host, face, WHITE), face.w as usize * face.h as usize);

    // W_NAME untopped: `0x1101` = bc=1, tc=1, md=TRANS, IP_HOLLOW, ic=1 →
    // black ring on the grown rect, the `FL3DACT` bevel inside it
    // (`add_3d_effect`, `gemoblib.c:949-950`; reference
    // `falcon-tc-two-windows.png` row 85 LBLACK), white inside, black
    // transparent glyphs, and **no** stipple: every non-glyph pixel inside
    // the bevel is white.
    let nr = name_rect(b1);
    let nbox = grown(nr);
    assert_ring(host, nbox, BLACK, "untopped W_NAME border");
    assert_bevel(host, nbox.inset(1), "untopped W_NAME");
    let nlen = 3 * 8;
    let nx = nr.x + ((nr.w - nlen) / 2).max(2);
    let cells = R { x: nx, y: nr.y + 1, w: nlen, h: 16 };
    let inner = nbox.inset(2);
    for (x, y, p) in pixels(host, inner) {
        if cells.contains(x, y) {
            continue;
        }
        assert_eq!(p, WHITE, "untopped W_NAME is hollow white outside the glyphs, at ({x},{y})");
    }
    assert!(count(host, cells, BLACK) > 0, "untopped W_NAME glyphs in tc=1");
    assert_eq!(count(host, inner, LWHITE), 0, "untopped W_NAME has no grey");
    assert_eq!(count(host, inner, BLACK) + count(host, inner, WHITE), inner.w as usize * inner.h as usize);

    // W_INFO untopped: the same `0x1101` as topped — black ring, white
    // inside.
    let info = R { x: b1.x, y: b1.y + ADJ_HBOX, w: b1.w, h: GL_HBOX };
    assert_ring(host, info, BLACK, "untopped W_INFO border");
    let ii = info.inset(1);
    assert_eq!(count(host, ii, LWHITE), 0, "untopped W_INFO is hollow-white, not grey");
    assert_all(host, R { x: ii.x + ii.w - 40, y: ii.y, w: 40, h: ii.h }, WHITE, "untopped W_INFO interior");

    // W_VSLIDE untopped: `0x1111` (3D objects on, `gemwmlib.c:1355-1357`) →
    // the same IP_1PATT track as topped.
    let top = top_of(b1, true);
    let row_y = b1.y + b1.h - ADJ_HBOX;
    let vbar = R { x: b1.x + b1.w - ADJ_WBOX, y: top, w: ADJ_WBOX, h: row_y - top + 1 };
    let track = R { x: vbar.x, y: vbar.y + ADJ_HBOX, w: vbar.w, h: vbar.h - 2 * ADJ_HBOX };
    assert_ring(host, track, BLACK, "untopped W_VSLIDE border");
    let elev = grown(velev_default(track));
    for (x, y, p) in pixels(host, track.inset(1)) {
        if elev.contains(x, y) {
            continue;
        }
        let want = if ip1_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "untopped W_VSLIDE IP_1PATT at ({x},{y})");
    }
}
