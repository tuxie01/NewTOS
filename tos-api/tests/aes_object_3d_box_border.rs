//! A67 — a 3D **box** (`FL3DIND`/`FL3DACT` on `G_BOX`/`G_IBOX`/`G_BOXCHAR`/
//! `G_BOXTEXT`/`G_FBOXTEXT`) keeps its border *and* gets the bevel.
//!
//! EmuTOS `just_draw` draws a box border whenever `th != 0`, with no 3D
//! exception (`aes/gemoblib.c:663-667`), and lays the 3D bevel over the drawn
//! object afterwards (`add_3d_effect`, `:950-951`). `add_3d_effect` itself
//! (`:220-283`) works on the **grown** rectangle (`ADJ3DSTD` each side, `:504-
//! 509`), shrinks it by an *inside* border (`th > 0`, `:231-238`) so the bevel
//! never overwrites that border, and draws two one-pixel polylines that stop
//! one pixel short of the corners: bottom-left → top-left → top-right in
//! WHITE (BLACK when SELECTED), then bottom-left → bottom-right → top-right in
//! LBLACK (LWHITE when SELECTED), falling back to BLACK/WHITE when that colour
//! is the object's own fill colour or beyond the workstation's colour count
//! (`:270-272`).
//!
//! # The reference
//!
//! `docs/analysis/fsel-reference-2026-09-17/fselectr.png` — EmuTOS's own AES
//! resource (`aes/gem_rsc.c`), rendered by `rscview` with 3D objects at the
//! 8×16 cell metric. The FSELECTR drive boxes are `G_BOXCHAR`s with `ob_spec
//! 0x41ff1100` (`gem_rsc.c:211-215`: `th = -1`, black border, hollow-white
//! interior) given `FL3DIND` by `fs_start` (`gemfslib.c:117-128`); drive A
//! lands at root-relative `(220, 100, 24, 16)` and drive C, the SELECTED one,
//! at `(220, 142, 24, 16)` (`tos-api/src/aes/fsel.rs::tests`). The expected
//! rows below are those pixels of the reference, read on 2026-09-18 with the
//! class map `#` = black, `.` = white, `l` = LWHITE, `d` = LBLACK; the
//! objects here are placed at the same coordinates, so the rows compare
//! one-to-one. Only the 8×16 glyph cell is masked (`*`): the character is the
//! system font's business, not the border's.
//!
//! # The profile
//!
//! 640×480 Falcon truecolor, pseudo-palette seeded like `init_wk` does
//! (`aes_object_cursor_bracket.rs`), so pens 0/1/8/9 resolve to four distinct
//! device words and every class in the expected rows is tellable apart.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::aes::object::{Object, TedInfo};
use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse;

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
/// 8 LWHITE (733), 9 LBLACK (533) — the four classes the expected rows use.
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

const G_BOXTEXT: u16 = 22;
const G_BOXCHAR: u16 = 27;
const SELECTED: u16 = 0x0001;
const LASTOB: u16 = 0x0020;
const FL3DIND: u16 = 0x0200;
const NIL: i16 = -1;

/// A device value no palette pen produces: "nothing drew here".
const SLATE: u32 = 0x0012_3456;

fn boxchar(spec: u64, flags: u16, state: u16, x: i16, y: i16) -> [Object; 1] {
    [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type: G_BOXCHAR,
        ob_flags: flags | LASTOB,
        ob_state: state,
        ob_spec: spec,
        ob_x: x,
        ob_y: y,
        ob_width: 24,
        ob_height: 16,
    }]
}

/// `objc_draw` (42) of the whole tree, no clip.
fn objc_draw(pb: &mut Aespb, t: &[Object; 1]) {
    pb.intin.fill(0);
    pb.intin[1] = 8;
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_ptr() as u64;
    pb.call(42, 6, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_draw refused the tree");
}

/// Draw `t` on a slate screen with the cursor out of the way, hand back the
/// backend.
fn draw(t: &[Object; 1]) -> &'static BufBackend {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    mouse::show_reset(b);
    mouse::remove(b);
    b.px.lock().unwrap().fill(SLATE);
    objc_draw(&mut pb, t);
    b
}

/// One screen row as the reference's class string over `x0..=x1`.
fn row(b: &BufBackend, y: i16, x0: i16, x1: i16) -> String {
    (x0..=x1)
        .map(|x| {
            if b.get_device_pixel(x, y) == SLATE {
                return '_';
            }
            match b.read_pixel(x, y) {
                0 => '.',
                1 => '#',
                8 => 'l',
                9 => 'd',
                _ => '?',
            }
        })
        .collect()
}

/// Compare `rows` (`y`, expected) over `x0..=x1`; `*` in the expectation
/// matches anything. Reports every differing row at once.
fn expect_rows(b: &BufBackend, x0: i16, x1: i16, rows: &[(i16, &str)], what: &str) {
    let mut bad = String::new();
    for &(y, want) in rows {
        let got = row(b, y, x0, x1);
        let same = got.chars().zip(want.chars()).all(|(g, w)| w == '*' || g == w);
        if !same || got.len() != want.len() {
            bad.push_str(&format!("  y={y:3}  ours |{got}|\n         ref  |{want}|\n"));
        }
    }
    assert!(bad.is_empty(), "{what}: rows differ from the reference (x {x0}..{x1})\n{bad}");
}

// ── cases ─────────────────────────────────────────────────────────────────

/// FSELECTR drive A as EmuTOS draws it: `G_BOXCHAR 'A'`, `ob_spec 0x41ff1100`
/// (black border, `th = -1`, hollow white → `indbutcol` face), `FL3DIND`,
/// not selected, at root-relative `(220, 100, 24, 16)`. Grown to
/// `(218, 98, 28, 20)`; the outside border is the black ring at `x = 217/246`,
/// `y = 97/118`; the bevel's WHITE polyline is column 218 (rows 98..116) and
/// row 98 (columns 218..244), its LBLACK polyline row 117 (219..245) and
/// column 245 (99..117); the two short-stopped corners `(245, 98)` and
/// `(218, 117)` show the face.
#[test]
fn fl3dind_boxchar_with_outside_border_matches_the_reference_drive_box() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = boxchar(0x41ff_1100, FL3DIND, 0, 220, 100);
    let b = draw(&t);
    // fselectr.png, root-relative rows 97..118 × columns 217..246. The glyph
    // cell is 8×16 at `(218 + (28-8)/2, 98 + (20-16)/2) = (228, 100)`, i.e.
    // offsets 11..18 of rows 100..115 — masked.
    const FACE: &str = "#.lllllllllllllllllllllllllld#";
    let masked: String = FACE
        .chars()
        .enumerate()
        .map(|(i, c)| if (11..=18).contains(&i) { '*' } else { c })
        .collect();
    let mut rows: Vec<(i16, &str)> = vec![
        (97, "##############################"),
        (98, "#...........................l#"),
        (99, FACE),
    ];
    rows.extend((100..=115).map(|y| (y, masked.as_str())));
    rows.extend([
        (116, FACE),
        (117, "#lddddddddddddddddddddddddddd#"),
        (118, "##############################"),
    ]);
    expect_rows(b, 217, 246, &rows, "drive A (FL3DIND, th = -1)");
    // Nothing spills past the border.
    assert_eq!(row(b, 110, 215, 216), "__", "left of the border");
    assert_eq!(row(b, 110, 247, 248), "__", "right of the border");
    assert_eq!(row(b, 96, 217, 246), "_".repeat(30), "above the border");
    assert_eq!(row(b, 119, 217, 246), "_".repeat(30), "below the border");
}

/// The same box SELECTED — FSELECTR drive C at root-relative `(220, 142)`:
/// the indicator changes colour (`indcolchange = 1`), so the face is the XOR
/// of LWHITE = LBLACK, the first polyline is BLACK and the second LWHITE;
/// the border stays.
#[test]
fn selected_fl3dind_boxchar_keeps_the_border_and_inverts_the_bevel() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = boxchar(0x43ff_1100, FL3DIND, SELECTED, 220, 142);
    let b = draw(&t);
    // fselectr.png, root-relative rows 139..141 and 158..160 × columns 217..246.
    #[rustfmt::skip]
    let rows: [(i16, &str); 6] = [
        (139, "##############################"),
        (140, "############################d#"),
        (141, "##ddddddddddddddddddddddddddl#"),
        (158, "##ddddddddddddddddddddddddddl#"),
        (159, "#dlllllllllllllllllllllllllll#"),
        (160, "##############################"),
    ];
    expect_rows(b, 217, 246, &rows, "drive C (FL3DIND, SELECTED, th = -1)");
}

/// An *inside* border (`th = 1`, `ob_spec 0x4101_1100`): the black ring is the
/// grown rectangle's own edge (`x = 218/245`, `y = 98/117`) and the bevel is
/// shrunk by `th` so it sits *inside* the ring (`add_3d_effect`,
/// `gemoblib.c:231-238`) instead of overwriting it.
#[test]
fn fl3dind_boxchar_with_inside_border_draws_the_bevel_inside_the_border() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = boxchar(0x4101_1100, FL3DIND, 0, 220, 100);
    let b = draw(&t);
    #[rustfmt::skip]
    let rows: [(i16, &str); 6] = [
        (97,  "______________________________"),
        (98,  "_############################_"),
        (99,  "_#.........................l#_"),
        (100, "_#.lllllllllllllllllllllllld#_"),
        (116, "_#lddddddddddddddddddddddddd#_"),
        (117, "_############################_"),
    ];
    expect_rows(b, 217, 246, &rows, "inside border (FL3DIND, th = +1)");
}

/// The text half: a `G_BOXTEXT` with `FL3DIND`, `te_thickness = -1` and a
/// hollow-white TEDINFO. EmuTOS only zeroes a text thickness when it promotes
/// a *`G_TEXT`/`G_FTEXT`* to the boxed type (`gemoblib.c:589-598`); a
/// `G_BOXTEXT` keeps its border (`:663-667`) under the bevel.
#[test]
fn fl3dind_boxtext_with_outside_border_keeps_it_under_the_bevel() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    let ted = Box::leak(Box::new(TedInfo {
        te_ptext: TEXT.as_ptr() as u64,
        te_ptmplt: TEXT.as_ptr() as u64,
        te_pvalid: TEXT.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: 2,
        te_color: 0x1180, // black border, black text, REPLACE, hollow, white
        te_junk2: 0,
        te_thickness: -1,
        te_txtlen: 1,
        te_tmplen: 1,
    }));
    let t = [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type: G_BOXTEXT,
        ob_flags: FL3DIND | LASTOB,
        ob_state: 0,
        ob_spec: ted as *const TedInfo as u64,
        ob_x: 220,
        ob_y: 100,
        ob_width: 24,
        ob_height: 16,
    }];
    let b = draw(&t);
    #[rustfmt::skip]
    let rows: [(i16, &str); 6] = [
        (97,  "##############################"),
        (98,  "#...........................l#"),
        (99,  "#.lllllllllllllllllllllllllld#"),
        (116, "#.lllllllllllllllllllllllllld#"),
        (117, "#lddddddddddddddddddddddddddd#"),
        (118, "##############################"),
    ];
    expect_rows(b, 217, 246, &rows, "G_BOXTEXT (FL3DIND, te_thickness = -1)");
}
