//! A99 — `objc_change` toggles SELECTED by XOR the way EmuTOS `ob_change`
//! does (`docs/api/aes/object-form.md` § `objc_change` (47) incl. D-093a,
//! EmuTOS `aes/gemoblib.c:1253-1320`, `xor_is_ok` `:107-148`).
//!
//! The rule (`:1280-1316`):
//!
//! * `G_USERDEF` → `ob_user(tree, obj, &t, spec, curr_state, new_state)`,
//!   **no** generic redraw (`:1282-1284`).
//! * `G_ICON` → never XOR, plain `just_draw` (`:1286-1287`).
//! * every other type: if only the `SELECTED` bit toggles and
//!   `xor_is_ok(obtype, flags, spec)` → `bb_fill(MD_XOR, FIS_SOLID,
//!   IP_SOLID, t inset by th)` and **no** `just_draw` (`redraw = !ok`,
//!   `:1295-1304`). A `G_IMAGE` being *deselected* takes the XOR **and**
//!   the redraw (`xor_is_ok(G_IMAGE)` is FALSE, `:114`; D-093a). An
//!   `FL3DOBJ` object is never XOR'd (`:114`) — its press shows by the
//!   bevel, so it is redrawn.
//! * `th` is `ob_sst`'s border thickness clamped at 0 (`:1277-1278`): an
//!   outside (negative) border leaves the full rect, a positive one insets.
//! * unchanged state or `spec == -1` → nothing at all (`:1263-1264`);
//!   `redraw == 0` → the state is stored and nothing is drawn (`:1270-1271`).
//!
//! Why it matters (the visible consequence of A85): with the trailing XOR in
//! `just_draw`, *redrawing* an object without an opaque fill (unboxed
//! `G_TEXT`/`G_FTEXT`, `G_IBOX`, `G_STRING`) to deselect it paints its ink
//! over the still-inverted ground instead of toggling the ground back — the
//! bar stays. TeraDesk's "Set mask" list rows (`desktop.rsc` tree 9,
//! `FTYPE1..4`, `G_FTEXT` `0x1100` on an `FL3DBAK` box) are that class.
//!
//! # The profile
//!
//! 640×480 Falcon truecolor, pseudo-palette seeded like `init_wk` does
//! (`aes_object_selected_xor.rs`, whose backend this file copies and extends
//! with a **draw-op log**). Class letters: `.` white, `#` black, `l` LWHITE,
//! `d` LBLACK, `L` = the 24-bit complement of LWHITE (an XOR over an LWHITE
//! face), `_` the slate ("nothing drew here"). The child object sits at
//! `(220, 100, 24, 16)` inside a parent box at `(200, 90, 70, 40)`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::aes::object::{BitBlk, IconBlk, Object, TedInfo, UserBlk};
use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse;

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── the draw-op log ───────────────────────────────────────────────────────

/// One backend draw call. Reads (`read_pixel`, `get_device_pixel`) are not
/// logged; the cursor sprite's `put_device_pixel` is, so a test can prove the
/// sprite stayed down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    FillRect { x: i16, y: i16, w: i16, h: i16, pel: Pel },
    Line,
    /// `solid` = every pattern word `0xFFFF` (the AES `IP_SOLID` pattern).
    FillPattern { x: i16, y: i16, w: i16, h: i16, mode: WriteMode, solid: bool },
    Glyph,
    BlitMono,
    CopyRect,
    PutDevicePixel,
}

impl Op {
    fn is_xor_fill(self) -> bool {
        matches!(self, Op::FillPattern { mode: WriteMode::Xor, .. })
    }
}

// ── a raster backend over a real pixel buffer ─────────────────────────────

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
    ops: Mutex<Vec<Op>>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            ops: Mutex::new(Vec::new()),
        }
    }

    fn log(&self, op: Op) {
        self.ops.lock().unwrap().push(op);
    }

    fn take_ops(&self) -> Vec<Op> {
        std::mem::take(&mut *self.ops.lock().unwrap())
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn idx(x: i16, y: i16) -> usize {
        y as usize * SCREEN_W as usize + x as usize
    }

    fn clipped(x: i16, y: i16, clip: ClipRect) -> bool {
        if !Self::inside(x, y) {
            return true;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(SCREEN_W, SCREEN_H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return true;
            }
        }
        false
    }

    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if Self::clipped(x, y, clip) {
            return;
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }

    /// VDI `WM_XOR`: complement the 24-bit RGB of the destination (EmuTOS
    /// `vdi_line.c:494` `*work = ~temp`; kernel `VdiScreen::xor_pixel`).
    fn xor(&self, x: i16, y: i16, clip: ClipRect) {
        if Self::clipped(x, y, clip) {
            return;
        }
        let mut px = self.px.lock().unwrap();
        let i = Self::idx(x, y);
        px[i] ^= 0x00FF_FFFF;
    }

    fn dev(&self, pel: Pel) -> u32 {
        self.rgb565_to_device(self.pel_to_device16(pel))
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
    fn set_active_palette(&self, palette: &[u32; 256]) {
        *self.palette.lock().unwrap() = palette.map(|p| p as u16);
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
    }
    /// The kernel's convention (`VdiScreen::device_to_rgb565`): the wire
    /// word is RGB555-in-565 — green in bits 6-10, bit 5 the overlay bit —
    /// as `seed_palette` packs it (EmuTOS `set_color16`, `vdi_col.c`).
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let r5 = ((argb >> 19) & 0x1f) as u16;
        let g5 = ((argb >> 11) & 0x1f) as u16;
        let b5 = ((argb >> 3) & 0x1f) as u16;
        (r5 << 11) | (g5 << 6) | b5
    }
    /// The kernel's `pal555_to_xrgb`: 5→8-bit gun replication, so WHITE is
    /// `0xFFFFFF` and BLACK `0x000000` — the XOR of one is the other.
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let x = |c5: u16| ((c5 << 3) | (c5 >> 2)) as u32;
        (x((px >> 11) & 0x1F) << 16) | (x((px >> 6) & 0x1F) << 8) | x(px & 0x1F)
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
        self.log(Op::FillRect { x, y, w, h, pel });
        let v = self.dev(pel);
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
        self.log(Op::Line);
        let fg = self.dev(pel);
        let bg = self.dev(0);
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
                    self.xor(x, y, clip);
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
    /// Mode-aware: the XOR cells depend on `WriteMode::Xor` complementing the
    /// destination exactly like the kernel's `combine`.
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        self.log(Op::FillPattern {
            x,
            y,
            w,
            h,
            mode: fill.mode,
            solid: fill.pattern.iter().all(|&word| word == 0xFFFF),
        });
        let fg = self.dev(fill.fg);
        let bg = self.dev(fill.bg);
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let set = row & (0x8000u16 >> (xx.rem_euclid(16) as u16)) != 0;
                match fill.mode {
                    WriteMode::Replace => self.put(xx, yy, if set { fg } else { bg }, clip),
                    WriteMode::Transparent => {
                        if set {
                            self.put(xx, yy, fg, clip)
                        }
                    }
                    WriteMode::Xor => {
                        if set {
                            self.xor(xx, yy, clip)
                        }
                    }
                    WriteMode::RevTransparent => {
                        if !set {
                            self.put(xx, yy, fg, clip)
                        }
                    }
                }
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
        self.log(Op::Glyph);
        let v = self.dev(fg);
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect) {
        self.log(Op::BlitMono);
        let f = self.dev(fg);
        let b = self.dev(bg);
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let set = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16) != 0;
                let (dx, dy) = (dst.x + col, dst.y + row);
                match mode {
                    WriteMode::Replace => self.put(dx, dy, if set { f } else { b }, clip),
                    WriteMode::Transparent => {
                        if set {
                            self.put(dx, dy, f, clip)
                        }
                    }
                    WriteMode::Xor => {
                        if set {
                            self.xor(dx, dy, clip)
                        }
                    }
                    WriteMode::RevTransparent => {
                        if !set {
                            self.put(dx, dy, f, clip)
                        }
                    }
                }
            }
        }
    }
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, _op: u8, clip: ClipRect) {
        self.log(Op::CopyRect);
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
        self.log(Op::PutDevicePixel);
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
unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

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
        // SAFETY: `block` holds six live pointers to arrays owned by `self`.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

// ── the rig ───────────────────────────────────────────────────────────────

static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

fn rig() -> &'static TestHost {
    RIG.get_or_init(|| {
        let host: &'static TestHost =
            Box::leak(Box::new(TestHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        seed_palette(&host.raster);
        host
    })
}

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`): pen 0 white, 1 black,
/// 2 red, 3 green, 8 LWHITE (733), 9 LBLACK (533), 13 LCYAN (0,667,667).
fn seed_palette(b: &BufBackend) {
    #[rustfmt::skip]
    const VIDEL1: [(u16, u16, u16); 16] = [
        (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
        (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
        (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
        (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
    ];
    let mut pal = [0u16; 256];
    for (i, (r, g, bl)) in VIDEL1.iter().enumerate() {
        let c5 = |v: u16| ((v as u32 * 31 + 500) / 1000) as u16;
        pal[i] = (c5(*r) << 11) | (c5(*g) << 6) | c5(*bl);
    }
    *b.palette.lock().unwrap() = pal;
}

// ── objects ───────────────────────────────────────────────────────────────

const G_BOX: u16 = 20;
const G_IMAGE: u16 = 23;
const G_USERDEF: u16 = 24;
const G_IBOX: u16 = 25;
const G_BUTTON: u16 = 26;
const G_STRING: u16 = 28;
const G_FTEXT: u16 = 29;
const G_ICON: u16 = 31;
const LASTOB: u16 = 0x0020;
const FL3DBAK: u16 = 0x0400;
const FL3DACT: u16 = 0x0600;
const NORMAL: i16 = 0;
const SELECTED: i16 = 0x0001;
const NIL: i16 = -1;

// `te_color` 0x1100: border 1 (black), text 1 (black), inside pattern 0
// (hollow), inside colour 0 (white), bit 7 clear = TRANSPARENT — TeraDesk's
// SETMASK row word.
const TE_HOLLOW_WHITE_TRANS: i16 = 0x1100;

/// A device value no palette pen produces: "nothing drew here".
const SLATE: u32 = 0x0012_3456;

/// The parent box: `(200, 90, 70, 40)`.
const PX: i16 = 200;
const PY: i16 = 90;
const PW: i16 = 70;
const PH: i16 = 40;

/// The child: `(220, 100, 24, 16)` absolute = `(20, 10)` inside the parent.
const OX: i16 = 220;
const OY: i16 = 100;
const OW: i16 = 24;
const OH: i16 = 16;

const BLACK: Pel = 1;
const LWHITE: Pel = 8;

/// A two-object tree: `[0]` a parent `G_BOX` with `flags`/`spec` (`th = -1`
/// outside border, hollow → WHITE face, or `backgrcol` LWHITE under
/// `FL3DBAK`), `[1]` the child of `ob_type`/`child_flags`/`child_spec`.
fn tree(parent_flags: u16, parent_spec: u64, ob_type: u16, child_flags: u16, child_spec: u64) -> [Object; 2] {
    [
        Object {
            ob_next: NIL,
            ob_head: 1,
            ob_tail: 1,
            ob_type: G_BOX,
            ob_flags: parent_flags,
            ob_state: 0,
            ob_spec: parent_spec,
            ob_x: PX,
            ob_y: PY,
            ob_width: PW,
            ob_height: PH,
        },
        Object {
            ob_next: 0,
            ob_head: NIL,
            ob_tail: NIL,
            ob_type,
            ob_flags: child_flags | LASTOB,
            ob_state: 0,
            ob_spec: child_spec,
            ob_x: OX - PX,
            ob_y: OY - PY,
            ob_width: OW,
            ob_height: OH,
        },
    ]
}

/// The FL3DBAK dialog panel (LWHITE face, black outside border) with a child.
fn on_lwhite_panel(ob_type: u16, child_flags: u16, child_spec: u64) -> [Object; 2] {
    tree(FL3DBAK, 0x00ff_1100, ob_type, child_flags, child_spec)
}

/// A plain hollow-white box (a menu drop-down's ground) with a child.
fn on_white_box(ob_type: u16, child_flags: u16, child_spec: u64) -> [Object; 2] {
    tree(0, 0x00ff_1100, ob_type, child_flags, child_spec)
}

/// A leaked TEDINFO: IBM font, `just`, `color`, `thickness`, `text`, `tmpl`.
fn ted(text: &'static [u8], tmpl: &'static [u8], just: i16, color: i16, thickness: i16) -> u64 {
    static EMPTY: &[u8] = b"\0";
    let t = Box::leak(Box::new(TedInfo {
        te_ptext: text.as_ptr() as u64,
        te_ptmplt: tmpl.as_ptr() as u64,
        te_pvalid: EMPTY.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: just,
        te_color: color,
        te_junk2: 0,
        te_thickness: thickness,
        te_txtlen: text.len() as i16,
        te_tmplen: tmpl.len() as i16,
    }));
    t as *const TedInfo as u64
}

/// A leaked 16×16 mono raster: a diagonal plus a full top row, so the image
/// has set and clear bits in every row but the top.
fn raster16() -> u64 {
    let mut words = [0u16; 16];
    words[0] = 0xFFFF;
    for (row, w) in words.iter_mut().enumerate().skip(1) {
        *w = 0x8000 >> row;
    }
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
    Box::leak(bytes.into_boxed_slice()).as_ptr() as u64
}

/// A leaked BITBLK over [`raster16`], ink BLACK, 16 px wide.
fn bitblk() -> u64 {
    let b = Box::leak(Box::new(BitBlk {
        bi_pdata: raster16(),
        bi_wb: 2,
        bi_hl: 16,
        bi_x: 0,
        bi_y: 0,
        bi_color: BLACK as i16,
    }));
    b as *const BitBlk as u64
}

/// A leaked ICONBLK: 16×16 mask = data = [`raster16`], fg BLACK on bg WHITE,
/// no overlay character, no label.
fn iconblk() -> u64 {
    let r = raster16();
    let ib = Box::leak(Box::new(IconBlk {
        ib_pmask: r,
        ib_pdata: r,
        ib_ptext: 0,
        ib_char: 0x1000,
        ib_xchar: 0,
        ib_ychar: 0,
        ib_xicon: 0,
        ib_yicon: 0,
        ib_wicon: 16,
        ib_hicon: 16,
        ib_xtext: 0,
        ib_ytext: 0,
        ib_wtext: 0,
        ib_htext: 0,
    }));
    ib as *const IconBlk as u64
}

/// A leaked USERBLK. The tree is app-provided (reached through the AES
/// dispatcher), so the AES never *calls* `ub_code` on the host — it defers
/// the EL0 upcall (`ob_user`) — which is exactly what lets this file prove
/// "no generic redraw" without a callback body.
fn userblk() -> u64 {
    let ub = Box::leak(Box::new(UserBlk { ub_code: 0x1000, ub_parm: 0 }));
    ub as *const UserBlk as u64
}

/// `objc_draw` (42) of the whole tree, no clip.
fn objc_draw(pb: &mut Aespb, t: &mut [Object; 2]) {
    pb.intin.fill(0);
    pb.intin[1] = 8;
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_mut_ptr() as u64;
    pb.call(42, 6, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_draw refused the tree");
}

/// `objc_change` (47): `int_in[0]` object, `[2..5]` clip (0 = none), `[6]`
/// new state, `[7]` redraw flag (`aesdefs.h:376-377`).
fn objc_change(pb: &mut Aespb, t: &mut [Object; 2], obj: i16, newstate: i16, redraw: bool) {
    pb.intin.fill(0);
    pb.intin[0] = obj;
    pb.intin[6] = newstate;
    pb.intin[7] = redraw as i16;
    pb.addrin.fill(0);
    // The AES writes `ob_state` through this pointer: it must come from a
    // `&mut` (an `as_ptr()` of a non-`mut` array is UB and lets the optimiser
    // read the stale state).
    pb.addrin[0] = t.as_mut_ptr() as u64;
    pb.call(47, 8, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_change refused the tree");
}

/// Draw `t` on a slate screen with the cursor out of the way (hide depth 1,
/// so the AES's own `gsx_moff`/`gsx_mon` bracket nests without a sprite
/// blit), then clear the op log.
fn draw(t: &mut [Object; 2]) -> &'static BufBackend {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    mouse::show_reset(b);
    mouse::remove(b);
    b.px.lock().unwrap().fill(SLATE);
    objc_draw(&mut pb, t);
    b.take_ops();
    b
}

/// Pixel class at `(x, y)` — see the module doc for the letters.
fn class(b: &BufBackend, x: i16, y: i16) -> char {
    let v = b.get_device_pixel(x, y);
    if v == SLATE {
        return '_';
    }
    if v == b.dev(LWHITE) ^ 0x00FF_FFFF {
        return 'L';
    }
    for (pel, c) in [(0, '.'), (1, '#'), (8, 'l'), (9, 'd')] {
        if v == b.dev(pel) {
            return c;
        }
    }
    '?'
}

/// One screen row as a class string over `x0..=x1`.
fn row(b: &BufBackend, y: i16, x0: i16, x1: i16) -> String {
    (x0..=x1).map(|x| class(b, x, y)).collect()
}

/// The child's rect `(220..=243, 100..=115)` as one class string.
fn cell(b: &BufBackend) -> String {
    (OY..OY + OH).map(|y| row(b, y, OX, OX + OW - 1)).collect()
}

/// The first differing pixel of two whole-screen snapshots.
fn first_difference(a: &[u32], b: &[u32]) -> Option<(i16, i16, u32, u32)> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).map(|i| {
        let x = (i % SCREEN_W as usize) as i16;
        let y = (i / SCREEN_W as usize) as i16;
        (x, y, a[i], b[i])
    })
}

fn assert_same_screen(before: &[u32], after: &[u32], what: &str) {
    if let Some((x, y, was, is)) = first_difference(before, after) {
        panic!("{what}: the screen differs from the reference, first at ({x},{y}): was 0x{was:06X}, is 0x{is:06X}");
    }
}

/// The single XOR-fill op EmuTOS's `bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, t
/// inset by th)` maps to: one `fill_pattern` in `WriteMode::Xor` with the
/// solid pattern over exactly `(x, y, w, h)`, and nothing else.
fn assert_exactly_one_xor_fill(ops: &[Op], x: i16, y: i16, w: i16, h: i16, what: &str) {
    assert_eq!(
        ops,
        &[Op::FillPattern { x, y, w, h, mode: WriteMode::Xor, solid: true }],
        "{what}: a SELECTED toggle on an XOR-able object is exactly one solid XOR fill over ({x},{y},{w},{h})"
    );
}

// ── (a) unboxed G_FTEXT on the LWHITE FL3DBAK panel — TeraDesk's SETMASK row ──

/// `G_FTEXT`, `te_color 0x1100`, `th = 0`, no 3D flag, child of an `FL3DBAK`
/// box (LWHITE face). Select via `objc_change(SELECTED, 1)`: `xor_is_ok`
/// (`:129-131`, interior 0 / text 1 ≤ BLACK) → one XOR over the full rect —
/// the ground `L`, the glyphs WHITE. Deselect via `objc_change(NORMAL, 1)`:
/// the second XOR restores the parent's ground and the glyphs **exactly** as
/// before the selection. Red before A99: the deselect *redrew* the text over
/// the inverted ground, and the bar stayed.
#[test]
fn a99_unboxed_g_ftext_deselect_restores_the_parent_ground_and_glyphs() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    static TMPL: &[u8] = b"__\0";
    let mut t = on_lwhite_panel(G_FTEXT, 0, ted(TEXT, TMPL, 0, TE_HOLLOW_WHITE_TRANS, 0));
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();
    assert_eq!(row(b, 100, 236, 243), "l".repeat(8), "precondition: the row lies on the LWHITE face:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert_eq!(t[1].ob_state, SELECTED as u16, "the state is stored");
    assert_eq!(row(b, 100, 236, 243), "L".repeat(8), "SELECTED: the row's ground is XOR-inverted:\n{}", cell(b));
    let glyphs: String = (100..116).map(|y| row(b, y, 220, 235)).collect();
    assert!(glyphs.contains('.'), "SELECTED: the glyphs are XOR'd BLACK → WHITE: {glyphs}");
    assert!(!glyphs.contains('l'), "SELECTED: no untouched LWHITE inside the row: {glyphs}");
    // Outside the child the panel is untouched.
    assert_eq!(row(b, 99, 220, 243), "l".repeat(24));
    assert_eq!(row(b, 116, 220, 243), "l".repeat(24));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_eq!(t[1].ob_state, 0, "the state is stored");
    assert_same_screen(&normal, &b.snapshot(), "G_FTEXT deselected by objc_change");
}

// ── (b) G_IBOX on the LWHITE panel ────────────────────────────────────────

/// A non-3D `G_IBOX` (`th = -1`, no fill): `xor_is_ok(G_IBOX)` is TRUE
/// (`:112-113`); `th < 0` is clamped to 0 (`:1277-1278`), so the XOR covers
/// the whole object rect (the outside ring is not in it). Round trip = the
/// panel as before.
#[test]
fn a99_g_ibox_deselect_restores_the_ground() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut t = on_lwhite_panel(G_IBOX, 0, 0x00ff_1100);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();
    assert_eq!(row(b, 108, 221, 242), "l".repeat(22), "precondition: hollow interior on the face:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    // Interior complemented, the inner ring (on the rect's edge) BLACK → WHITE,
    // the outer ring (outside the rect) untouched.
    assert_eq!(row(b, 108, 217, 246), format!("ll#.{}.#ll", "L".repeat(22)), "SELECTED G_IBOX:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_same_screen(&normal, &b.snapshot(), "G_IBOX deselected by objc_change");
}

// ── (c) G_STRING (a menu item) round trip ─────────────────────────────────

/// A `G_STRING` in a hollow-white box (a drop-down): select → black bar with
/// WHITE glyphs (`xor_is_ok(G_STRING)` TRUE, `:112-113`); deselect →
/// pixel-identical to the box before the selection. This is what
/// `menu_select` → `do_chg` → `ob_change` (`gemmnlib.c:170`) relies on: no
/// white pre-fill, no repaint of the item, two XORs.
#[test]
fn a99_g_string_select_deselect_round_trip_is_pixel_identical() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let mut t = on_white_box(G_STRING, 0, TEXT.as_ptr() as u64);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    let c = cell(b);
    assert_eq!(row(b, 100, 236, 243), "#".repeat(8), "SELECTED G_STRING: black bar beyond the glyphs:\n{c}");
    assert!(c.contains('.'), "SELECTED G_STRING: WHITE glyphs:\n{c}");

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_same_screen(&normal, &b.snapshot(), "G_STRING deselected by objc_change");
}

// ── (d) the op counter: exactly one XOR fill, nothing else ────────────────

/// `ob_change` `:1295-1304`: when only `SELECTED` toggles and `xor_is_ok`,
/// the object is **not** redrawn — `bb_fill(MD_XOR, FIS_SOLID, IP_SOLID,
/// t.g_x+th, t.g_y+th, t.g_w-2th, t.g_h-2th)` is the whole draw. For the
/// `G_FTEXT` row (`th = 0`) that is one solid XOR fill over `(220, 100, 24,
/// 16)`; no glyph, no border, no fill_rect, no sprite blit — on the select
/// and on the deselect alike.
#[test]
fn a99_selected_toggle_on_an_xor_able_object_is_exactly_one_xor_fill() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    static TMPL: &[u8] = b"__\0";
    let mut t = on_lwhite_panel(G_FTEXT, 0, ted(TEXT, TMPL, 0, TE_HOLLOW_WHITE_TRANS, 0));
    let b = draw(&mut t);
    let mut pb = Aespb::new();

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert_exactly_one_xor_fill(&b.take_ops(), OX, OY, OW, OH, "G_FTEXT select");

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_exactly_one_xor_fill(&b.take_ops(), OX, OY, OW, OH, "G_FTEXT deselect");
}

/// The same for a `G_STRING` (`th = 0`): the menu item's highlight is one
/// XOR fill each way.
#[test]
fn a99_g_string_toggle_is_exactly_one_xor_fill() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let mut t = on_white_box(G_STRING, 0, TEXT.as_ptr() as u64);
    let b = draw(&mut t);
    let mut pb = Aespb::new();

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert_exactly_one_xor_fill(&b.take_ops(), OX, OY, OW, OH, "G_STRING select");

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_exactly_one_xor_fill(&b.take_ops(), OX, OY, OW, OH, "G_STRING deselect");
}

/// A positive `th` insets the XOR (`:1301-1302`): a hollow-white `G_BOX`
/// with an *inside* border of 2 (`spec 0x0002_1100`) toggles `(222, 102,
/// 20, 12)` — the border is not inverted. `xor_is_ok(G_BOX, 0x1100)` is
/// TRUE (interior 0, text 1, `:140-147`).
#[test]
fn a99_inside_border_insets_the_xor_by_th() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut t = on_white_box(G_BOX, 0, 0x0002_1100);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert_exactly_one_xor_fill(&b.take_ops(), OX + 2, OY + 2, OW - 4, OH - 4, "G_BOX th=2 select");
    assert_eq!(row(b, 108, 220, 243), format!("##{}##", "#".repeat(20)), "SELECTED: interior black inside the border:\n{}", cell(b));
    assert_eq!(row(b, 101, 220, 243), "#".repeat(24), "the 2-px border row stays black:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert_same_screen(&normal, &b.snapshot(), "G_BOX th=2 deselected by objc_change");
}

// ── (e) G_IMAGE deselect: XOR *and* redraw (D-093a) ───────────────────────

/// `xor_is_ok(G_IMAGE)` is FALSE (`:114`). Select: no XOR in `ob_change`
/// (`ok` FALSE, `new_state & SELECTED`), `redraw` stays TRUE → `just_draw`
/// inverts the ground and blits the image in `xor_color` (`:766-770`).
/// Deselect: `(obtype == G_IMAGE) && !(new_state & SELECTED)` → the XOR
/// **and** `redraw = !ok = TRUE` → `just_draw` re-blits the image normal
/// (D-093a). The ops prove both steps; the pixels prove the round trip.
#[test]
fn a99_g_image_deselect_xors_and_redraws() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut t = on_white_box(G_IMAGE, 0, bitblk());
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();
    assert_eq!(row(b, 100, 220, 235), "#".repeat(16), "precondition: the image's top row is BLACK ink:\n{}", cell(b));
    assert_eq!(row(b, 108, 236, 243), ".".repeat(8), "precondition: white ground beside the image:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    let ops = b.take_ops();
    assert!(
        !ops.first().is_some_and(|op| op.is_xor_fill() && ops.len() == 1),
        "G_IMAGE select is a redraw, not a bare ob_change XOR: {ops:?}"
    );
    assert_eq!(row(b, 108, 236, 243), "#".repeat(8), "SELECTED G_IMAGE: inverted ground:\n{}", cell(b));
    assert_eq!(row(b, 100, 220, 235), ".".repeat(16), "SELECTED G_IMAGE: the ink is xor_color'd WHITE:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    let ops = b.take_ops();
    assert_eq!(
        ops.first(),
        Some(&Op::FillPattern { x: OX, y: OY, w: OW, h: OH, mode: WriteMode::Xor, solid: true }),
        "G_IMAGE deselect starts with ob_change's XOR over the object rect: {ops:?}"
    );
    assert!(
        ops.iter().skip(1).any(|op| matches!(op, Op::BlitMono)),
        "G_IMAGE deselect then redraws the image (D-093a): {ops:?}"
    );
    assert_same_screen(&normal, &b.snapshot(), "G_IMAGE deselected by objc_change");
}

// ── (f) FL3DACT button: full redraw, no bare XOR ──────────────────────────

/// `xor_is_ok` is FALSE for any `FL3DOBJ` (`:114`), so an activator's press
/// is a `just_draw` (sunk bevel, nudged label) and its release another
/// (raised bevel) — never an XOR fill of the rect. The release repaints
/// the grown rect completely, so the round trip is pixel-identical.
#[test]
fn a99_fl3dact_button_toggle_redraws_without_a_bare_xor() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static LABEL: &[u8] = b"X\0";
    let mut t = on_lwhite_panel(G_BUTTON, FL3DACT, LABEL.as_ptr() as u64);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();
    assert_eq!(class(b, 218, 108), '.', "precondition: raised bevel, left line WHITE:\n{}", row(b, 108, 217, 246));

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    let ops = b.take_ops();
    assert!(!ops.iter().any(|op| op.is_xor_fill()), "FL3DACT press must not XOR: {ops:?}");
    assert!(ops.iter().any(|op| matches!(op, Op::Glyph)), "FL3DACT press is a full redraw (label drawn): {ops:?}");
    assert_eq!(class(b, 218, 108), '#', "SELECTED: sunk bevel, left line BLACK:\n{}", row(b, 108, 217, 246));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    let ops = b.take_ops();
    assert!(!ops.iter().any(|op| op.is_xor_fill()), "FL3DACT release must not XOR: {ops:?}");
    assert_same_screen(&normal, &b.snapshot(), "FL3DACT G_BUTTON released by objc_change");
}

// ── (g) G_ICON: redraw, never XOR ─────────────────────────────────────────

/// `case G_ICON: /* never use XOR for icons */` (`:1286-1287`): the icon is
/// redrawn with its state (`gr_gicon` swaps fg/bg itself), both ways.
#[test]
fn a99_g_icon_toggle_redraws_without_xor() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut t = on_white_box(G_ICON, 0, iconblk());
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();
    assert_eq!(row(b, 100, 220, 235), "#".repeat(16), "precondition: the icon's top row is BLACK:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    let ops = b.take_ops();
    assert!(!ops.iter().any(|op| op.is_xor_fill()), "G_ICON select must not XOR: {ops:?}");
    assert!(ops.iter().any(|op| matches!(op, Op::BlitMono)), "G_ICON select redraws the icon: {ops:?}");
    assert_eq!(row(b, 100, 220, 235), ".".repeat(16), "SELECTED G_ICON: fg/bg swapped by gr_gicon:\n{}", cell(b));

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    let ops = b.take_ops();
    assert!(!ops.iter().any(|op| op.is_xor_fill()), "G_ICON deselect must not XOR: {ops:?}");
    assert_same_screen(&normal, &b.snapshot(), "G_ICON deselected by objc_change");
}

// ── (h) redraw = 0: state stored, nothing drawn ───────────────────────────

/// `if (!redraw) return;` (`:1270-1271`) after the state write.
#[test]
fn a99_redraw_zero_stores_the_state_and_draws_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let mut t = on_white_box(G_STRING, 0, TEXT.as_ptr() as u64);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();

    objc_change(&mut pb, &mut t, 1, SELECTED, false);
    assert_eq!(t[1].ob_state, SELECTED as u16, "the state is stored");
    assert!(b.take_ops().is_empty(), "redraw = 0 draws nothing");
    assert_same_screen(&normal, &b.snapshot(), "objc_change with redraw = 0");
}

// ── (i) unchanged state: a no-op ──────────────────────────────────────────

/// `if ((curr_state == new_state) || (spec == -1L)) return;` (`:1263-1264`):
/// re-asserting the state the object already has neither draws nor XORs.
#[test]
fn a99_unchanged_state_is_a_no_op() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let mut t = on_white_box(G_STRING, 0, TEXT.as_ptr() as u64);
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();

    objc_change(&mut pb, &mut t, 1, NORMAL, true);
    assert!(b.take_ops().is_empty(), "an unchanged state draws nothing");
    assert_same_screen(&normal, &b.snapshot(), "objc_change to the same state");

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    b.take_ops();
    let selected = b.snapshot();
    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert!(b.take_ops().is_empty(), "re-selecting a SELECTED object draws nothing (no double XOR)");
    assert_same_screen(&selected, &b.snapshot(), "objc_change SELECTED → SELECTED");
}

// ── (j) G_USERDEF: the user routine, no generic redraw ────────────────────

/// `case G_USERDEF: ob_user(...); redraw = FALSE;` (`:1282-1284`) — the AES
/// itself draws nothing for a userdef on `objc_change`: no XOR of the rect,
/// no `just_draw`. (The tree is app-provided, so the callback is the EL0
/// upcall `ob_user` defers on the host; the state is still stored.)
#[test]
fn a99_g_userdef_toggle_issues_no_generic_draw() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut t = on_white_box(G_USERDEF, 0, userblk());
    let b = draw(&mut t);
    let mut pb = Aespb::new();
    let normal = b.snapshot();

    objc_change(&mut pb, &mut t, 1, SELECTED, true);
    assert_eq!(t[1].ob_state, SELECTED as u16, "the state is stored");
    assert!(b.take_ops().is_empty(), "G_USERDEF: the AES draws nothing itself (no XOR, no just_draw)");
    assert_same_screen(&normal, &b.snapshot(), "G_USERDEF objc_change");
}
