//! `wind_set(DESKWH, WF_NEWDESK, tree, root)` — where the **root** companion
//! word travels, per pointer convention (`TASK/A81.md`).
//!
//! EmuTOS's `wm_set` `case WF_NEWDESK` (`references/emutos/aes/gemwmlib.c:
//! 1685-1689`) reads the tree as a 32-bit LONG packed across `int_in[2..3]`
//! and the root from the word after it, `pinwds[2]` = `int_in[4]`. NewTOS
//! accepts that packed form at the **caller's own pointer width**
//! (`tos-api/src/aes/mod.rs::intin_ptr`): two WORDs for an ILP32 caller, four
//! for an LP64 one — so for an LP64 caller the root is `int_in[6]`, the word
//! after the four pointer words (`docs/api/aes/window-graphics.md`, "Pointers
//! in `addr_in[]`", edge case 16). Before A81 the handler read a fixed
//! `int_in[4]` for every width, which for an LP64 caller is the pointer's own
//! bits 32..47.
//!
//! This rig drives the dispatcher end to end (`tos_api::aes::dispatch`, so
//! the `IN_LEN` clamp of `Aespb::from_raw` is in the path) and observes the
//! registered pair the only way a program can: by what the AES paints as the
//! desktop background afterwards. The tree's root candidates are three
//! distinct icon boxes; the AES paints exactly the one whose index it
//! registered. The tree is padded to 32768 objects so that a mis-read root —
//! any 16-bit value the old fixed `int_in[4]` could pick up — lands on a real
//! (empty) object rather than off the array: the old code must *fail* this
//! test, not crash it.
//!
//! The ILP32 packed form (root at `int_in[4]`) cannot be driven through the
//! dispatcher on a 64-bit host — its tree pointer is four bytes and no host
//! allocation lies under 4 GiB — and is pinned by the in-crate unit test
//! `window::newdesk_root_word_tests::ilp32_packed_root_is_int_in_4`, which
//! reads the registered pair back without dereferencing it.
//!
//! Harness shape (backend + host + parameter block) copied from
//! `aes_desk_background.rs`, which copied it from `aes_window_content.rs`.

#![allow(clippy::too_many_arguments)]

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

    /// Fill `(x, y, w, h)` with a raw device value, bypassing the AES.
    fn paint(&self, r: (i16, i16, i16, i16), argb: u32) {
        let mut px = self.px.lock().unwrap();
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) {
                    px[Self::idx(x, y)] = argb;
                }
            }
        }
    }

    /// Number of pixels inside `r` still holding [`MARK`] — "the AES did
    /// *not* repaint here".
    fn marked_in(&self, r: (i16, i16, i16, i16)) -> usize {
        let px = self.px.lock().unwrap();
        let mut n = 0;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) && px[Self::idx(x, y)] == MARK {
                    n += 1;
                }
            }
        }
        n
    }

    /// Number of pixels inside `r` but outside `hole` that no longer hold
    /// [`MARK`] — "the AES repainted something it should not have".
    fn unmarked_outside(&self, r: (i16, i16, i16, i16), hole: (i16, i16, i16, i16)) -> usize {
        let px = self.px.lock().unwrap();
        let in_hole = |x: i16, y: i16| x >= hole.0 && x < hole.0 + hole.2 && y >= hole.1 && y < hole.1 + hole.3;
        let mut n = 0;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) && !in_hole(x, y) && px[Self::idx(x, y)] != MARK {
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

// ── the host ──────────────────────────────────────────────────────────────

struct TestHost {
    raster: BufBackend,
    clock: AtomicU64,
}

impl Host for TestHost {
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

unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

// ── AES parameter block ───────────────────────────────────────────────────

/// The caller ABI the block is dispatched at: LP64, the native application
/// ABI — the width at which the packed form's pointer is four WORDs.
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
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

const OP_APPL_INIT: i16 = 10;
const OP_FORM_DIAL: i16 = 51;
const OP_WIND_SET: i16 = 105;
const OP_WIND_NEW: i16 = 109;

const WF_NEWDESK: i16 = 14;
const FMD_FINISH: i16 = 3;

/// EmuTOS `gl_hbox` for the 8x16 system font (`aes/gemgraf.c:412`): the top of
/// `gl_rfull`, above which `w_drawdesk` never paints (`:423-424`).
const GL_HBOX: i16 = 19;

/// A marker written straight into the framebuffer; not a palette colour, so
/// nothing the AES paints can produce it by accident.
const MARK: u32 = 0x0011_2233;

/// Seed the backend's pseudo-palette the way the VDI dispatcher does
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`): EmuTOS'
/// `videl_palette1` for pens 0..15, as RGB555-in-565 words.
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

/// A GEM `OBJECT` at the kernel/LP64 record width (`tos-api/src/aes/object.rs`)
/// — the width `draw_desk_tree` reads a tree registered with no user process
/// current at (`render_tree_lp64`).
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

/// Where the three icon boxes — the root candidates — sit on screen.
const ICON: [(i16, i16, i16, i16); 3] = [(150, 110, 32, 32), (250, 180, 32, 32), (350, 240, 32, 32)];

/// Objects in the tree: 0 = the full-screen background, 1..=3 = the icons,
/// the rest = unlinked empty leaves, present only so that a mis-read root of
/// any non-negative 16-bit value addresses a real object.
const TREE_LEN: usize = 32768;

/// The desktop-background tree: a patterned box over the whole screen with
/// three solid icon boxes as children (what TeraDesk's `wind_set(0,
/// WF_NEWDESK, desk_tree, 0)` registers, `sdk/apps/teradesk/icon.c:2511`),
/// padded as described at [`TREE_LEN`].
fn desk_tree() -> &'static [Obj] {
    // colour word: (border<<12)|(text<<8)|(tmode<<7)|(pattern<<4)|interior
    let bg = 0x0000_0043u64; // pattern 4, interior 3 (green) — a dithered desktop
    let ic = 0x0000_0072u64; // pattern 7 (solid), interior 2 (red) — an "icon"
    let leaf = Obj { next: -1, head: -1, tail: -1, otype: G_BOX, flags: 0, state: 0, spec: 0, x: 0, y: 0, w: 0, h: 0 };
    let mut v = vec![leaf; TREE_LEN];
    v[0] = Obj { next: -1, head: 1, tail: 3, otype: G_BOX, flags: 0, state: 0, spec: bg, x: 0, y: 0, w: SCREEN_W as i16, h: SCREEN_H as i16 };
    for (i, r) in ICON.iter().enumerate() {
        let next = if i + 1 < ICON.len() { (i + 2) as i16 } else { 0 };
        v[i + 1] = Obj { next, head: -1, tail: -1, otype: G_BOX, flags: 0, state: 0, spec: ic, x: r.0, y: r.1, w: r.2, h: r.3 };
    }
    Box::leak(v.into_boxed_slice())
}

static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

/// The one host for this binary (`install_host` accepts a single install).
fn rig() -> &'static TestHost {
    RIG.get_or_init(|| {
        let host: &'static TestHost = Box::leak(Box::new(TestHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        seed_palette(&host.raster);
        host
    })
}

/// Fresh AES window state, no desktop tree registered. `wind_new` (EmuTOS
/// `wm_new`, `aes/gemwmlib.c:1875-1893`) resets the window table.
fn fresh() -> Aespb {
    let mut pb = Aespb::new();
    pb.call(OP_APPL_INIT, 0, 1, 0);
    pb.intin.fill(0);
    pb.call(OP_WIND_NEW, 0, 1, 0);
    pb
}

/// `form_dial(FMD_FINISH)` over the whole screen: `fm_dial` → `w_update`
/// (`aes/gemfmlib.c`, `gemwmlib.c:1004-1048`) repaints the desktop background
/// wherever the desktop window owns the screen — here, everything below the
/// menu row.
fn repaint_desktop(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = FMD_FINISH;
    pb.intin[5] = 0;
    pb.intin[6] = 0;
    pb.intin[7] = SCREEN_W as i16;
    pb.intin[8] = SCREEN_H as i16;
    pb.call(OP_FORM_DIAL, 9, 1, 0);
}

/// Pack `p` little-endian into the four `int_in` WORDs at `at` — gemlib's
/// `*(void**)(aes_intin + 2) = v` on an LP64 little-endian target.
fn pack_lp64(pb: &mut Aespb, at: usize, p: u64) {
    for i in 0..4 {
        pb.intin[at + i] = ((p >> (16 * i)) & 0xFFFF) as u16 as i16;
    }
}

/// The word a fixed `int_in[4]` read picks out of an LP64-packed pointer —
/// its bits 32..47 (`TASK/A81.md`: `0x0000_0001_0177_E4C0` → `0x0001`).
fn upper_alias(tree: u64) -> i16 {
    ((tree >> 32) & 0xFFFF) as u16 as i16
}

/// A root index the old fixed-slot read cannot produce for this tree: the
/// first of the three icons whose index differs from the pointer's aliased
/// word. With a host tree at `0x0000_0001_…` that is 2, the measured A81
/// root; the choice is made at run time because a host allocation's upper
/// half is the allocator's, not the test's.
fn root_for(tree: u64) -> i16 {
    let alias = upper_alias(tree);
    (1..=ICON.len() as i16).find(|&r| r != alias).expect("three candidates, one alias")
}

/// The one observation: after a full-screen background repaint, icon `root`'s
/// box is painted and nothing else below the menu row is — the AES drew the
/// registered tree from the registered root and from nowhere else.
fn assert_root_painted(b: &BufBackend, root: i16, how: &str) {
    let below_menu = (0, GL_HBOX, SCREEN_W as i16, SCREEN_H as i16 - GL_HBOX);
    let want = ICON[(root - 1) as usize];
    let left = b.marked_in(want);
    assert_eq!(
        left, 0,
        "{how}: registered root {root} — its icon box {want:?} must be painted by the background \
         repaint, but {left} of its {} pixels are untouched (the AES drew from a different root)",
        want.2 as usize * want.3 as usize
    );
    let stray = b.unmarked_outside(below_menu, want);
    assert_eq!(
        stray, 0,
        "{how}: registered root {root} — only icon box {want:?} may be painted, but {stray} pixels \
         outside it changed (root 0 = the whole background, or another icon, was drawn)"
    );
}

/// LP64 **packed** form: pointer in `int_in[2..5]`, root in `int_in[6]`,
/// `IN_LEN 7` (the seventh word must be declared or `from_raw` clears it),
/// `AIN_LEN 0`. Red before A81: `set_newdesk` read the root from `int_in[4]`
/// — the pointer's bits 32..47 — and painted from that object instead.
#[test]
fn lp64_packed_form_takes_the_root_from_the_word_after_the_pointer() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh();

    let tree = desk_tree();
    let addr = tree.as_ptr() as u64;
    let root = root_for(addr);

    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pack_lp64(&mut pb, 2, addr);
    pb.intin[6] = root;
    pb.call(OP_WIND_SET, 7, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set(WF_NEWDESK) answers 1");

    b.paint((0, 0, SCREEN_W as i16, SCREEN_H as i16), MARK);
    repaint_desktop(&mut pb);
    assert_root_painted(
        b,
        root,
        &format!(
            "LP64 packed wind_set(0, WF_NEWDESK) — tree {addr:#018x} in int_in[2..5], root {root} in int_in[6] \
             (a fixed int_in[4] read yields {})",
            upper_alias(addr)
        ),
    );
}

/// **Native** form: tree in `addr_in[0]`, root in `int_in[2]`, `IN_LEN 3`,
/// `AIN_LEN 1` — what the native desktop sends
/// (`programs/desktop/src/lib.rs::wind_set_newdesk`) and what gemlib's overlay
/// bindings send after A81. Green before and after.
#[test]
fn native_form_takes_the_root_from_int_in_2() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh();

    let tree = desk_tree();
    let addr = tree.as_ptr() as u64;
    let root = root_for(addr);

    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = root;
    pb.addrin[0] = addr;
    pb.call(OP_WIND_SET, 3, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "wind_set(WF_NEWDESK) answers 1");

    b.paint((0, 0, SCREEN_W as i16, SCREEN_H as i16), MARK);
    repaint_desktop(&mut pb);
    assert_root_painted(b, root, &format!("native wind_set(0, WF_NEWDESK) — tree in addr_in[0], root {root} in int_in[2]"));
}

/// LP64 packed form with root 0 — TeraDesk's actual call
/// (`sdk/apps/teradesk/icon.c:2511`): the whole background below the menu
/// row is painted. On the target this case was never wrong (every EL0 tree
/// lies under 4 GiB, so the old fixed `int_in[4]` read found 0 there by
/// accident); on the host the tree's upper half is the allocator's, and the
/// old code was red here too — the root it read was the allocation's bits
/// 32..47, an empty padding leaf, and nothing was painted at all.
#[test]
fn lp64_packed_form_with_root_0_paints_the_whole_background() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh();

    let tree = desk_tree();
    let addr = tree.as_ptr() as u64;

    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_NEWDESK;
    pack_lp64(&mut pb, 2, addr);
    pb.intin[6] = 0;
    pb.call(OP_WIND_SET, 7, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set(WF_NEWDESK) answers 1");

    b.paint((0, 0, SCREEN_W as i16, SCREEN_H as i16), MARK);
    repaint_desktop(&mut pb);
    let left = b.marked_in((0, GL_HBOX, SCREEN_W as i16, SCREEN_H as i16 - GL_HBOX));
    assert_eq!(left, 0, "root 0: everything below the menu row is repainted ({left} px left)");
    let menu = b.marked_in((0, 0, SCREEN_W as i16, GL_HBOX));
    assert_eq!(menu, (SCREEN_W as usize) * (GL_HBOX as usize), "the menu row is never part of the background repaint");
}
