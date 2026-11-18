//! A89 / A90 / A92 — the event-manager and menu-track neighbours of A73,
//! each driven against EmuTOS:
//!
//! * **A89.1 — leaving a submenu into no man's land** (`gemmnlib.c:437-438`):
//!   `mn_do`'s hit-test falls through to "no man's land" with `menu_state ==
//!   SUBMENU_STATE`, so it sets `cur_item = last_item` before switching to
//!   `OUTSIDE_STATE` — the parent item stays selected and the submenu stays
//!   on screen (`item_changed` is false, `:452-461`; the title is unchanged,
//!   `:463-464`). A press there breaks the loop (`:373-374`) and the cleanup
//!   finds `cur_item != NIL` and enabled → it returns `(cur_title, cur_item)`,
//!   the *parent* item (`:482-500`, `menu_state != SUBMENU_STATE` → `*pitem =
//!   cur_item`). The same cleanup makes a press on the parent item itself,
//!   submenu showing (`INITEM_STATE`, `cur_submenu == NIL`), a selection of
//!   the parent.
//! * **A89.2 — a DISABLED title** (`gemmnlib.c:229-244,286-292,429-434`):
//!   `menu_down` drops a box only if `do_chg(tree, ititle, SELECTED, TRUE,
//!   TRUE, TRUE)` succeeds, and `do_chg` refuses a DISABLED object
//!   (`:158-160`) — no hilite, no `menu_sr(TRUE)`, no `ob_draw`. Leaving the
//!   title into no man's land ends `mn_do` without a selection (`cur_menu ==
//!   NIL` → `cur_title = NIL` → `done`, `:406-411`; or the explicit DISABLED
//!   exit, `:429-434`).
//! * **A90 — `evnt_mouse` (22)** (`gemevlib.c:123-128` → `ev_block(MU_M1,
//!   pmo)` → `iasync` → `amouse`, `geminput.c:568-587`): the rectangle is
//!   tested **once at arm time**, ownership-blind (`in_mrect` → `azombie`);
//!   otherwise the wait parks on the caller's `c_msleep`, which only
//!   `post_mouse(gl_mowner, …)` walks (`:446-456`) — from `mchange` on every
//!   move while the caller owns the mouse, and from `set_mown` the moment
//!   ownership comes back to it (`:220-231`). While the Control Manager sits
//!   in `ct_msgup`'s `while (button & 0x0001) dsptch();` (`gemctrl.c:100-101`)
//!   after a select-on-press, the application does not own the mouse, so a
//!   rectangle crossed *then* is not delivered; the release lets `ctlmgr`
//!   loop into `w_setactive` → `set_mown(app)` → `post_mouse(app, xrat,
//!   yrat)` (`gemctrl.c:441-442`), which completes the parked wait against
//!   the live position.
//! * **A92 — the arm-time state includes every packet queued before the
//!   call** (`gemevlib.c:169-170`): `ev_multi` runs `chkkbd(); forker();`
//!   before its quick checks and before `abutton`/`amouse` arm, so `button`,
//!   `xrat`/`yrat` are current as of the call. `ev_mouse` has no such drain
//!   (`:123-128`): its arm-time position is the one the previous dispatch
//!   left (`super()` → `dsptch()`, `gemsuper.c:452-464`).
//! * **A103.1 — a press on the submenu box's own area (border/gap) picks
//!   `smroot`** (`gemmnlib.c:417,494-496`): `cur_submenu = ob_find(smtree,
//!   smroot, 1, x, y)` answers the box itself for a point inside it that no
//!   child covers (`ob_find` returns the last object found,
//!   `gemoblib.c:1034-1113`); `mn_do` treats it as any submenu item —
//!   `SUBMENU_STATE`, `menu_select(smtree, cur_submenu, …)` hilites it
//!   (`:474-475`, the box interior's XOR, `ob_change`), and the cleanup
//!   returns `*pitem = cur_submenu`, `*ptree = smtree` (`:494-496`).
//! * **A103.2 — a DISABLED item with a submenu attached opens nothing**
//!   (`gemmnlib.c:470-475`): `display_submenu` runs only inside `if
//!   (menu_select(tree, cur_item, last_item, TRUE))`, and `menu_select` →
//!   `do_chg(…, chkdisabled = TRUE)` refuses a DISABLED object (`:158-160`).
//!
//! Harness (raster stand-in + scripted host + parameter block) copied from
//! `aes_a73_event_neighbours.rs`; the pointer advances only in
//! `Host::pump_input`, exactly as on the board. New here: a **probe packet**
//! — consumed like any other packet inside the tracker's own pump, it records
//! the tree state and two screen pixels at that instant, so the *intermediate*
//! state of a track (which no AES return exposes) can be asserted.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
    /// Mode-aware (as the A99/A100 harnesses): `Xor` complements the device
    /// pixel where the pattern is set — the one way a hilited box interior
    /// (`ob_change`'s `bb_fill(MD_XOR, …)`) is visible to a probe (A103.1).
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.rgb565_to_device(self.pel_to_device16(fill.fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(fill.bg));
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
                            let v = self.get_device_pixel(xx, yy);
                            self.put(xx, yy, v ^ 0x00FF_FFFF, clip);
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

// ── the host: input advances only in `pump_input`, as on the board ─────────

/// One synthetic mouse packet, in the form the VDI's `mouse_int` receives it.
/// A `probe` packet carries no input: when the pump reaches it, the host
/// records a [`Sample`] of the state at that instant instead.
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    /// VDI convention: bit 0 = left, bit 1 = right.
    buttons: u8,
    probe: bool,
}

const fn pkt(dx: i16, dy: i16, buttons: u8) -> Packet {
    Packet { dx, dy, buttons, probe: false }
}

const PROBE: Packet = Packet { dx: 0, dy: 0, buttons: 0, probe: true };

/// What a probe packet records: the `ob_state` of two objects of the menu
/// tree (`PROBE_OBJ_A`/`PROBE_OBJ_B`) and the pen at two screen points
/// (`PROBE_PX_A`/`PROBE_PX_B`), plus where the pointer was.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Sample {
    state_a: u16,
    state_b: u16,
    pen_a: Pel,
    pen_b: Pel,
    /// The same two points as **device** pixels: an XOR'd white is the
    /// device complement, which no palette entry names (`read_pixel` maps
    /// it to pen 0), so an inverted box interior is only visible here.
    dev_a: u32,
    dev_b: u32,
    at: (i16, i16),
}

/// Probe configuration: the tree the states are read from, two object
/// indices, two pixel positions (packed `x << 16 | y` as i32 in a u64).
static PROBE_TREE: AtomicUsize = AtomicUsize::new(0);
static PROBE_OBJ: AtomicU64 = AtomicU64::new(0);
static PROBE_PX: AtomicU64 = AtomicU64::new(0);

fn set_probe(tree: *const Obj, obj_a: i16, obj_b: i16, px_a: (i16, i16), px_b: (i16, i16)) {
    PROBE_TREE.store(tree as usize, Ordering::Relaxed);
    PROBE_OBJ.store(((obj_a as u16 as u64) << 16) | obj_b as u16 as u64, Ordering::Relaxed);
    let pack = |(x, y): (i16, i16)| ((x as u16 as u64) << 16) | y as u16 as u64;
    PROBE_PX.store((pack(px_a) << 32) | pack(px_b), Ordering::Relaxed);
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
    samples: Mutex<Vec<Sample>>,
    /// Milliseconds; advanced by `pump_input` alone.
    clock: AtomicU64,
    /// Consecutive pumps that found the script empty. A tracking loop that
    /// never sees the event it waits for would otherwise spin forever inside
    /// the trap; the cap turns that into a failing assertion.
    idle: AtomicUsize,
}

/// Pumps on an empty script before the harness gives up. `count_clicks` may
/// legitimately run a whole double-click window (≤ 450 pumps) on an empty
/// script, so the cap sits well above that.
const IDLE_CAP: usize = 20_000;

impl ScriptHost {
    fn new() -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(VecDeque::new()),
            samples: Mutex::new(Vec::new()),
            clock: AtomicU64::new(0),
            idle: AtomicUsize::new(0),
        }
    }
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn clear(&self) {
        self.script.lock().unwrap().clear();
        self.samples.lock().unwrap().clear();
        self.idle.store(0, Ordering::Relaxed);
    }
    fn queued(&self) -> usize {
        self.script.lock().unwrap().len()
    }
    fn samples(&self) -> Vec<Sample> {
        self.samples.lock().unwrap().clone()
    }
    fn take_sample(&self) {
        let tree = PROBE_TREE.load(Ordering::Relaxed) as *const Obj;
        let objs = PROBE_OBJ.load(Ordering::Relaxed);
        let pxs = PROBE_PX.load(Ordering::Relaxed);
        let (oa, ob) = ((objs >> 16) as u16 as i16, objs as u16 as i16);
        let unpack = |v: u64| ((v >> 16) as u16 as i16, v as u16 as i16);
        let (pa, pb) = (unpack(pxs >> 32), unpack(pxs & 0xffff_ffff));
        // SAFETY: the probe tree is the leaked, live menu tree of this case.
        let state = |i: i16| unsafe { (*tree.offset(i as isize)).state };
        let s = Sample {
            state_a: state(oa),
            state_b: state(ob),
            pen_a: self.raster.read_pixel(pa.0, pa.1),
            pen_b: self.raster.read_pixel(pb.0, pb.1),
            dev_a: self.raster.get_device_pixel(pa.0, pa.1),
            dev_b: self.raster.get_device_pixel(pb.0, pb.1),
            at: tos_api::vdi::mouse_position(),
        };
        self.samples.lock().unwrap().push(s);
    }
}

impl Host for ScriptHost {
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
        let next = self.script.lock().unwrap().pop_front();
        match next {
            Some(p) if p.probe => {
                self.idle.store(0, Ordering::Relaxed);
                self.take_sample();
            }
            Some(p) => {
                self.idle.store(0, Ordering::Relaxed);
                tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
            }
            None => {
                let n = self.idle.fetch_add(1, Ordering::Relaxed) + 1;
                assert!(
                    n < IDLE_CAP,
                    "the AES is still waiting after {n} pumps on an exhausted mouse script: \
                     the event it waits for never came (pointer {:?}, buttons {:#04x})",
                    tos_api::vdi::mouse_position(),
                    tos_api::vdi::mouse_buttons()
                );
            }
        }
    }
}

// SAFETY: every mutable field is behind a Mutex / atomic.
unsafe impl Sync for ScriptHost {}
unsafe impl Send for ScriptHost {}

// ── AES parameter block ────────────────────────────────────────────────────

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

    /// Issue one AES call, exactly as the GEM-trap handler does.
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

fn install(host: &'static ScriptHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`): EmuTOS'
/// `videl_palette1` for pens 0..15 as RGB555-in-565 words, so `read_pixel`
/// round-trips the pens the menu draws with. Below the bar the AES paints
/// its desktop dither (pen [`DESK`] on the `(x + y)`-even points, white on
/// the others); a drop-down box painted over it reads back as pen 0.
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

const WHITE: Pel = 0;
/// The desktop background's dither pen at an `(x + y)`-even point.
const DESK: Pel = 3;

// ── the menu tree ──────────────────────────────────────────────────────────

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
const G_IBOX: u16 = 25;
const G_STRING: u16 = 28;
const G_TITLE: u16 = 32;
const LASTOB: u16 = 0x0020;
const SELECTED: u16 = 0x0001;
const DISABLED: u16 = 0x0008;

/// Object indices the cases steer by.
const TITLE_FILE: i16 = 4;
const TITLE_EDIT: i16 = 5;
const ITEM_OPEN: i16 = 12;

/// Bar geometry: titles are 56 px wide in the 19 px bar row; the *File*
/// drop-down box sits at (56,19) and holds three 19 px items; *Edit*'s box
/// at (112,19) holds two.
const BAR_H: i16 = 19;
const FILE_X: i16 = 56;
const EDIT_X: i16 = 112;
const ITEM_H: i16 = 19;
const BOX_W: i16 = 112;

/// A writable, NUL-terminated copy of `s` — `menu_attach` writes the
/// submenu marker into the parent item's text (`gemmnext.c:62-63`), so the
/// strings must not live in read-only memory.
fn text(s: &str) -> u64 {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    Box::leak(v.into_boxed_slice()).as_ptr() as u64
}

/// The desktop's menu shape (Desk/File/Edit with a drop-down each), the same
/// tree `aes_a73_event_neighbours.rs` uses, with heap strings. `edit_state`
/// is the *Edit* title's initial `ob_state`.
fn menu_tree(edit_state: u16) -> *mut Obj {
    let t = |ty, fl, spec: u64, x, y, w, h, next, head, tail| Obj {
        next, head, tail, otype: ty, flags: fl, state: 0, spec, x, y, w, h,
    };
    let s = |p: &str, y, next, last| {
        t(G_STRING, if last { LASTOB } else { 0 }, text(p), 0, y, BOX_W, ITEM_H, next, -1, -1)
    };
    let mut tree: Box<[Obj; 17]> = Box::new([
        /* 0 ROOT      */ t(G_IBOX, 0, 0, 0, 0, 640, 480, -1, 1, 6),
        /* 1 THEBAR    */ t(G_BOX, 0, 0, 0, 0, 640, BAR_H, 6, 2, 2),
        /* 2 THEACTIVE */ t(G_IBOX, 0, 0, 0, 0, 640, BAR_H, 1, 3, 5),
        /* 3 Desk      */ t(G_TITLE, 0, text("Desk"), 0, 0, 56, BAR_H, 4, -1, -1),
        /* 4 File      */ t(G_TITLE, 0, text("File"), FILE_X, 0, 56, BAR_H, 5, -1, -1),
        /* 5 Edit      */ t(G_TITLE, 0, text("Edit"), EDIT_X, 0, 56, BAR_H, 2, -1, -1),
        /* 6 themenus  */ t(G_IBOX, 0, 0, 0, 0, 640, 480, 0, 7, 14),
        /* 7 Desk-box  */ t(G_BOX, 0, 1 << 16, 0, BAR_H, BOX_W, 2 * ITEM_H, 10, 8, 9),
        /* 8  About    */ s("About", 0, 9, false),
        /* 9  Info     */ s("Info", ITEM_H, 7, false),
        /*10 File-box  */ t(G_BOX, 0, 1 << 16, FILE_X, BAR_H, BOX_W, 3 * ITEM_H, 14, 11, 13),
        /*11  New      */ s("New", 0, 12, false),
        /*12  Open     */ s("Open    ", ITEM_H, 13, false),
        /*13  Quit     */ s("Quit", 2 * ITEM_H, 10, false),
        /*14 Edit-box  */ t(G_BOX, 0, 1 << 16, EDIT_X, BAR_H, BOX_W, 2 * ITEM_H, 6, 15, 16),
        /*15  Cut      */ s("Cut", 0, 16, false),
        /*16  Copy     */ s("Copy", ITEM_H, 14, true),
    ]);
    tree[TITLE_EDIT as usize].state = edit_state;
    Box::leak(tree).as_mut_ptr()
}

/// A three-item submenu in its own tree: root `G_BOX` 100×57, items at y
/// 0/19/38. `open_submenu` re-homes the box beside the parent item.
const SUB_W: i16 = 100;
const SUB_ALPHA: i16 = 1;

fn submenu_tree() -> *mut Obj {
    let s = |p: &str, y, next, last| Obj {
        next,
        head: -1,
        tail: -1,
        otype: G_STRING,
        flags: if last { LASTOB } else { 0 },
        state: 0,
        spec: text(p),
        x: 0,
        y,
        w: SUB_W,
        h: ITEM_H,
    };
    let tree: Box<[Obj; 4]> = Box::new([
        Obj { next: -1, head: 1, tail: 3, otype: G_BOX, flags: 0, state: 0, spec: 1 << 16, x: 0, y: 0, w: SUB_W, h: 3 * ITEM_H },
        s("Alpha", 0, 2, false),
        s("Beta", ITEM_H, 3, false),
        s("Gamma", 2 * ITEM_H, 0, true),
    ]);
    Box::leak(tree).as_mut_ptr()
}

/// The bar tree of [`menu_tree`] with a submenu box **in the same tree**:
/// object 17, a fourth child of `themenus` (after the *Edit* box), whose
/// three items (18..=20) sit **2 px inside** the box on both sides, so the
/// box's first and last two columns are the box's own area — the "border"
/// `ob_find(smtree, smroot, 1, …)` answers with `smroot` itself
/// (`gemoblib.c:1034-1113`). A same-tree submenu's pick round-trips as
/// `MN_SELECTED`'s `msg[4]` (`menu::track`), which is what A103.1 reads.
const SUB_ROOT: i16 = 17;
const SUB_BETA_SAME: i16 = 19;
const SUB_INSET: i16 = 2;

fn menu_tree_with_same_tree_submenu() -> *mut Obj {
    let base = menu_tree(0);
    let s = |p: &str, y, next, last| Obj {
        next,
        head: -1,
        tail: -1,
        otype: G_STRING,
        flags: if last { LASTOB } else { 0 },
        state: 0,
        spec: text(p),
        x: SUB_INSET,
        y,
        w: SUB_W - 2 * SUB_INSET,
        h: ITEM_H,
    };
    let mut v: Vec<Obj> = (0..17).map(|i| unsafe { *base.offset(i) }).collect();
    // Hang the submenu box after the Edit box (14): 14 → 17 → back to 6.
    v[14].next = SUB_ROOT;
    v[6].tail = SUB_ROOT;
    // The Edit box's *Copy* was the tree's LASTOB; the last submenu item is now.
    v[16].flags &= !LASTOB;
    v.push(Obj { next: 6, head: 18, tail: 20, otype: G_BOX, flags: 0, state: 0, spec: 1 << 16, x: 0, y: 0, w: SUB_W, h: 3 * ITEM_H });
    v.push(s("Alpha", 0, 19, false));
    v.push(s("Beta", ITEM_H, 20, false));
    v.push(s("Gamma", 2 * ITEM_H, SUB_ROOT, true));
    Box::leak(v.into_boxed_slice()).as_mut_ptr()
}

/// The `MENU` struct at the LP64 layout (`mn_tree` @0, four WORDs @8).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct MenuRec {
    mn_tree: u64,
    mn_menu: i16,
    mn_item: i16,
    mn_scroll: i16,
    mn_keystate: i16,
}

// ── the rig ────────────────────────────────────────────────────────────────

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_M1: i16 = 0x0004;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const MN_SELECTED: i16 = 10;

/// The installed host, menu tree and AES state are process-wide statics, so
/// the cases in this binary run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static ScriptHost> = OnceLock::new();

fn rig() -> &'static ScriptHost {
    RIG.get_or_init(|| {
        let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost::new()));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Fresh application state with the menu bar installed and the pointer parked
/// on the desktop, buttons up, nothing queued. Returns the tree too.
fn fresh_with(edit_state: u16) -> (&'static ScriptHost, Aespb, *mut Obj) {
    let host = rig();
    host.clear();
    // A leftover box from a case that ended inside a track would sit in the
    // raster; the cases below only ever end with the track closed, but the
    // raster is cleared so a pixel probe never reads a predecessor's paint.
    host.raster.px.lock().unwrap().fill(0);
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new: closes every window, drains the update locks

    let tree = menu_tree(edit_state);
    pb.intin.fill(0);
    pb.intin[0] = 1; // show
    pb.addrin[0] = tree as u64;
    pb.call(30, 1, 1, 1); // menu_bar
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_bar must install the tree");

    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);

    // Park the pointer on empty desktop, buttons up, and empty the pipe.
    move_to(host, &mut pb, 320, 300, 0);
    drain_messages(&mut pb);
    (host, pb, tree)
}

fn fresh() -> (&'static ScriptHost, Aespb, *mut Obj) {
    fresh_with(0)
}

/// Like [`fresh`], but the bar is [`menu_tree_with_same_tree_submenu`] and
/// the submenu (object 17 of that tree) is attached to *Open*.
fn fresh_same_tree_submenu() -> (&'static ScriptHost, Aespb, *mut Obj) {
    let host = rig();
    host.clear();
    host.raster.px.lock().unwrap().fill(0);
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new

    let tree = menu_tree_with_same_tree_submenu();
    pb.intin.fill(0);
    pb.intin[0] = 1; // show
    pb.addrin[0] = tree as u64;
    pb.call(30, 1, 1, 1); // menu_bar
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_bar must install the tree");

    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);

    move_to(host, &mut pb, 320, 300, 0);
    drain_messages(&mut pb);

    let mut rec = Box::new(MenuRec { mn_tree: tree as u64, mn_menu: SUB_ROOT, mn_item: SUB_ROOT + 1, mn_scroll: 0, mn_keystate: 0 });
    pb.intin.fill(0);
    pb.intin[0] = 1; // ME_ATTACH
    pb.intin[1] = ITEM_OPEN;
    pb.addrin[0] = tree as u64;
    pb.addrin[1] = &mut *rec as *mut MenuRec as u64;
    pb.call(37, 2, 1, 2);
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    assert_eq!(pb.intout[0], 1, "menu_attach must accept the same-tree submenu");
    (host, pb, tree)
}

/// `menu_ienable(tree, item, enable)` (32).
fn menu_ienable(pb: &mut Aespb, tree: *mut Obj, item: i16, enable: bool) {
    pb.intin.fill(0);
    pb.intin[0] = item;
    pb.intin[1] = enable as i16;
    pb.addrin[0] = tree as u64;
    pb.call(32, 2, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_ienable({item}, {enable})");
}

/// Attach the submenu to *Open* through `menu_attach(ME_ATTACH)` (37).
fn attach_submenu(pb: &mut Aespb, tree: *mut Obj) -> *mut Obj {
    let sub = submenu_tree();
    let mut rec = Box::new(MenuRec { mn_tree: sub as u64, mn_menu: 0, mn_item: SUB_ALPHA, mn_scroll: 0, mn_keystate: 0 });
    pb.intin.fill(0);
    pb.intin[0] = 1; // ME_ATTACH
    pb.intin[1] = ITEM_OPEN;
    pb.addrin[0] = tree as u64;
    pb.addrin[1] = &mut *rec as *mut MenuRec as u64;
    pb.call(37, 2, 1, 2);
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    assert_eq!(pb.intout[0], 1, "menu_attach must accept the submenu");
    sub
}

/// `graf_mkstate` — one AES call, hence one `dsptch()` → one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) -> (i16, i16, i16) {
    pb.call(79, 0, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Consume every packet still queued.
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..64 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        graf_mkstate(pb);
    }
}

/// Walk the pointer to `(tx, ty)` with `buttons`, one packet per AES call.
fn move_to(host: &'static ScriptHost, pb: &mut Aespb, tx: i16, ty: i16, buttons: u8) {
    drain(host, pb);
    for _ in 0..200 {
        let (cx, cy) = tos_api::vdi::mouse_position();
        if cx == tx && cy == ty {
            break;
        }
        let dx = (tx - cx).clamp(-32, 32);
        let dy = (ty - cy).clamp(-32, 32);
        host.push(pkt(dx, dy, buttons));
        graf_mkstate(pb);
    }
    let (cx, cy) = tos_api::vdi::mouse_position();
    assert_eq!((cx, cy), (tx, ty), "move_to did not reach its target");
}

/// TeraDesk's main-loop wait: `MU_MESAG | MU_BUTTON | MU_KEYBD`, `ev_mbclicks
/// = 0x102` (bflag 1 = wait for a press, up to 2 clicks), `ev_mbmask = 3`,
/// `ev_mbstate = 0`, no timer (`sdk/apps/teradesk/main.c:1588-1610`), with a
/// real message buffer. Returns `(fired, EV_BRET, EV_MB, msg)`.
fn evnt_multi_teradesk(pb: &mut Aespb) -> (i16, i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], pb.intout[3], *buf)
}

/// TeraDesk's wait with a real (non-zero) timer: the loop pumps the script
/// until the deadline and reports `MU_TIMER` alone if nothing else fired.
fn evnt_multi_timed(pb: &mut Aespb, ms: u16) -> (i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD | MU_TIMER;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[14] = ms as i16;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], *buf)
}

/// A plain `evnt_multi(flags)` poll with an `MU_M1` block and a 0 ms timer:
/// pass 0 alone decides. Returns `(fired, EV_MX, EV_MY, EV_MB, EV_BRET)`.
fn evnt_multi_poll(pb: &mut Aespb, flags: i16, m1: (i16, i16, i16, i16, i16)) -> (i16, i16, i16, i16, i16) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = flags | MU_TIMER;
    pb.intin[1] = 0x0102; // wait for a press (TeraDesk's bflag/clicks)
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[4] = m1.0; // MMO1_FLAGS
    pb.intin[5] = m1.1;
    pb.intin[6] = m1.2;
    pb.intin[7] = m1.3;
    pb.intin[8] = m1.4;
    pb.intin[14] = 0;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[6])
}

/// `evnt_mouse` (22): `MO_FLAGS` = 0 enter / 1 leave, then the `GRECT`.
/// Returns `(RET_CODE, EV_MX, EV_MY, EV_MB)`.
fn evnt_mouse(pb: &mut Aespb, flags: i16, rect: (i16, i16, i16, i16)) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = flags;
    pb.intin[1] = rect.0;
    pb.intin[2] = rect.1;
    pb.intin[3] = rect.2;
    pb.intin[4] = rect.3;
    pb.call(22, 5, 5, 0);
    (pb.intout[0], pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Empty the owner FIFO (`MU_MESAG | MU_TIMER(0)` polls until none is left).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        let mut buf = Box::new([0i16; 8]);
        pb.intin.fill(0);
        pb.intin[0] = MU_MESAG | MU_TIMER;
        pb.addrin[0] = buf.as_mut_ptr() as u64;
        pb.call(25, 16, 7, 1);
        pb.addrin[0] = 0;
        if pb.intout[0] & MU_MESAG == 0 {
            return;
        }
    }
}

/// Pointer positions: over the *File* title, over its *Open* item, over the
/// *Edit* title.
const ON_FILE: (i16, i16) = (FILE_X + 28, 8);
const ON_OPEN: (i16, i16) = (FILE_X + 28, BAR_H + ITEM_H + 9);
const ON_EDIT: (i16, i16) = (EDIT_X + 28, 8);

/// Queue a glide of the pointer by `(dx_total, dy_total)` in 10 px packets,
/// `buttons` held throughout.
fn glide(host: &ScriptHost, dx_total: i16, dy_total: i16, buttons: u8) {
    let mut left = dx_total;
    while left != 0 {
        let step = left.clamp(-10, 10);
        host.push(pkt(step, 0, buttons));
        left -= step;
    }
    let mut left = dy_total;
    while left != 0 {
        let step = left.clamp(-10, 10);
        host.push(pkt(0, step, buttons));
        left -= step;
    }
}

/// A press held for a few packets, then released, then idle.
fn click(host: &ScriptHost) {
    for _ in 0..4 {
        host.push(pkt(0, 0, 1));
    }
    for _ in 0..9 {
        host.push(pkt(0, 0, 0));
    }
}

/// `ev_multi`'s `chkkbd(); forker();` (`gemevlib.c:169-170`) drains the
/// fork ring before the quick checks arm: NewTOS pumps once at the head of
/// `evnt_multi` (A92). A case whose script must start *after* the arm queues
/// one idle packet for that drain to consume.
fn arm_gap(host: &ScriptHost) {
    host.push(pkt(0, 0, 0));
}

// ── A89.1: the submenu states ──────────────────────────────────────────────

/// Where `open_submenu` puts the submenu box for *Open*: to the right of the
/// item, overlapping the arrow by one cell (`ix + iw - CELL_W`), start item
/// level with the parent (`gemmnext.c:653-663`).
fn submenu_box() -> (i16, i16, i16, i16) {
    let cw = tos_api::vdi::font::CELL_W;
    (FILE_X + BOX_W - cw, BAR_H + ITEM_H, SUB_W, 3 * ITEM_H)
}

/// A point on the submenu's *Beta* row, right of its text.
fn on_beta() -> (i16, i16) {
    let (x, y, w, _) = submenu_box();
    (x + w - 10, y + ITEM_H + 9)
}

/// A pixel inside the submenu box that no text reaches (right edge of the
/// *Gamma* row), on an `(x + y)`-even point: pen 0 while the box is on
/// screen, the desktop's [`DESK`] otherwise.
fn submenu_probe_px() -> (i16, i16) {
    let (x, y, w, _) = submenu_box();
    let p = (x + w - 5, y + 2 * ITEM_H + 9);
    assert_eq!((p.0 + p.1) % 2, 0, "probe point on a DESK-pen point of the dither");
    p
}

/// **A89.1a — a press on the parent item, submenu showing, selects the
/// parent.** Hover *File*, slide onto *Open* (its submenu opens beside it,
/// `INITEM_STATE` with `smtree` set), press without ever entering the
/// submenu. EmuTOS: `MU_BUTTON` in `INITEM_STATE` breaks the loop
/// (`gemmnlib.c:373-374`); the cleanup undisplays the submenu, restores the
/// box and, `cur_item` being enabled and `menu_state != SUBMENU_STATE`,
/// returns `(File, Open)` (`:482-500`) → one `MN_SELECTED`.
#[test]
fn press_on_the_parent_item_with_its_submenu_showing_selects_the_parent() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh();
    attach_submenu(&mut pb, tree);

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0); // down onto *Open*
    host.push(pkt(0, 0, 0));
    click(host); // press → the selection (hover mode: buparm 0x00010101)

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("a89.1a: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!(fired & MU_BUTTON, 0, "the selecting press leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(
        fired & MU_MESAG,
        MU_MESAG,
        "a press on a drop-down item whose submenu is showing selects that item in EmuTOS \
         (mn_do INITEM_STATE → MU_BUTTON → break, aes/gemmnlib.c:373-374; cleanup returns \
         cur_item, :482-500) — NewTOS delivered nothing; fired = {fired:#06x}"
    );
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
}

/// **A89.1b — leaving the submenu into no man's land keeps the parent item
/// and the submenu; a press there selects the parent.** Hover *File*, slide
/// onto *Open*, into the submenu (`SUBMENU_STATE`, *Beta* hilited), then
/// right, out of the submenu onto empty desktop. EmuTOS (`gemmnlib.c:437-438`):
/// `if (menu_state == SUBMENU_STATE) cur_item = last_item;` → `OUTSIDE_STATE`
/// with the parent still selected and `smtree` still displayed (only the
/// submenu item is un-hilited, `:446-448`). The probe packet, consumed in
/// the tracker's own pump while the pointer rests outside, records exactly
/// that. A press there breaks the loop and the cleanup returns the retained
/// `cur_item` (`:482-500`): `MN_SELECTED (File, Open)`.
#[test]
fn leaving_the_submenu_into_no_mans_land_keeps_the_parent_and_a_press_selects_it() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh();
    let sub = attach_submenu(&mut pb, tree);
    let beta = on_beta();
    let (sbx, _, sbw, _) = submenu_box();
    let outside = (sbx + sbw + 40, beta.1); // right of the submenu, empty desktop
    set_probe(tree, ITEM_OPEN, ITEM_OPEN, submenu_probe_px(), (outside.0, outside.1));
    let (ppx, ppy) = submenu_probe_px();
    assert_eq!(host.raster.read_pixel(ppx, ppy), DESK, "the probe point shows the desktop dither before the track");
    // SAFETY: `sub` is the leaked, live submenu tree.
    let sub_state = |i: i16| unsafe { (*sub.offset(i as isize)).state };

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0); // down onto *Open* → submenu opens
    host.push(pkt(0, 0, 0));
    glide(host, beta.0 - ON_OPEN.0, beta.1 - ON_OPEN.1, 0); // into the submenu, onto *Beta*
    host.push(pkt(0, 0, 0));
    glide(host, outside.0 - beta.0, 0, 0); // out to the right: no man's land
    host.push(pkt(0, 0, 0));
    host.push(PROBE); // the state while the pointer rests outside
    host.push(pkt(0, 0, 0));
    click(host); // press outside

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    let (mx, my) = tos_api::vdi::mouse_position();
    let samples = host.samples();
    eprintln!("a89.1b: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} samples={samples:?}");
    assert_eq!(samples.len(), 1, "the probe must have been consumed inside the track; samples = {samples:?}");
    let s = samples[0];
    assert_eq!(s.at, outside, "the probe ran with the pointer outside");
    assert_eq!(
        s.state_a & SELECTED,
        SELECTED,
        "leaving the submenu into no man's land must keep the parent item selected \
         (aes/gemmnlib.c:437-438: cur_item = last_item; item_changed false → no un-hilite, :452-459); \
         Open.ob_state = {:#06x}",
        s.state_a
    );
    assert_eq!(
        s.pen_a,
        WHITE,
        "the submenu must still be on screen in OUTSIDE_STATE (no undisplay_submenu: \
         item_changed false, aes/gemmnlib.c:452-457); pen at {:?} = {}",
        submenu_probe_px(),
        s.pen_a
    );
    assert_eq!(sub_state(2) & SELECTED, 0, "the submenu item is un-hilited on leaving (:446-448)");
    assert_eq!(fired & MU_BUTTON, 0, "the press outside leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(
        fired & MU_MESAG,
        MU_MESAG,
        "a press in no man's land after leaving a submenu selects the retained parent item \
         (cleanup: cur_item != NIL, menu_state != SUBMENU_STATE → *pitem = cur_item, \
         aes/gemmnlib.c:482-500); fired = {fired:#06x}"
    );
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
    assert_eq!(sub_state(0) & SELECTED, 0, "nothing of the submenu tree is left selected");
    assert_eq!(host.raster.read_pixel(ppx, ppy), DESK, "the submenu's pixels were put back at the selection");
}

/// **A89.1c — the ticket's literal case: out of the submenu, back onto the
/// parent item, press.** After the excursion into no man's land the pointer
/// re-enters `cur_menu` on the parent (`MU_M2`, `INITEM_STATE`, `cur_item`
/// unchanged → the submenu stays, `:422-426,452`) and presses: `(File,
/// Open)`.
#[test]
fn leaving_the_submenu_and_returning_to_the_parent_item_a_press_selects_the_parent() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh();
    attach_submenu(&mut pb, tree);
    let beta = on_beta();
    let (sbx, _, sbw, _) = submenu_box();
    let outside = (sbx + sbw + 40, beta.1);

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0);
    host.push(pkt(0, 0, 0));
    glide(host, beta.0 - ON_OPEN.0, beta.1 - ON_OPEN.1, 0);
    host.push(pkt(0, 0, 0));
    glide(host, outside.0 - beta.0, 0, 0);
    host.push(pkt(0, 0, 0));
    // Back onto *Open* around the outside: down below the boxes, left past
    // the *File* box, up to the *Open* row, then right into the box — never
    // through the submenu nor over another item, so `cur_item` stays *Open*
    // all the way and the submenu stays displayed.
    glide(host, 0, 120 - outside.1, 0);
    glide(host, 30 - outside.0, 0, 0);
    glide(host, 0, ON_OPEN.1 - 120, 0);
    glide(host, ON_OPEN.0 - 30, 0, 0);
    host.push(pkt(0, 0, 0));
    click(host);

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("a89.1c: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!((mx, my), ON_OPEN, "the press must land on Open");
    assert_eq!(fired & MU_BUTTON, 0, "the selecting press leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "a press on the parent item selects it; fired = {fired:#06x}");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
}

// ── A89.2: a DISABLED title ────────────────────────────────────────────────

/// A pixel inside the *Edit* box area that no item text reaches, on an
/// `(x + y)`-even point of the desktop dither.
const EDIT_BOX_PX: (i16, i16) = (EDIT_X + BOX_W - 6, BAR_H + 9);

/// **A89.2 — hovering a DISABLED title drops no box and hilites nothing.**
/// *Edit* is DISABLED. The pointer enters the bar on *Edit* (`START_STATE`
/// → `INTITLE_STATE`, `gemmnlib.c:398-403`); `menu_select(tree, cur_title,
/// last_title, TRUE)` → `do_chg(…, chkdisabled = TRUE)` refuses
/// (`:158-160`), so `menu_down` is never called (`:466-467`): no `SELECTED`,
/// no `menu_sr(TRUE)`, no `ob_draw`. A click on the title does nothing
/// (`INTITLE_STATE` → `buparm ^= 1`, `:375`); leaving the title downward ends
/// the track without a selection (`cur_menu == NIL` → `cur_title = NIL` →
/// `done`, `:406-411`). The probe records the state while the pointer rests
/// on the title.
#[test]
fn hovering_a_disabled_title_drops_no_box_and_hilites_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh_with(DISABLED);
    set_probe(tree, TITLE_EDIT, TITLE_EDIT, EDIT_BOX_PX, EDIT_BOX_PX);
    // The bar as drawn by menu_bar: nothing in the Edit box area yet.
    assert_eq!(host.raster.read_pixel(EDIT_BOX_PX.0, EDIT_BOX_PX.1), DESK, "the probe point shows the desktop dither");

    move_to(host, &mut pb, ON_EDIT.0, ON_EDIT.1, 0);
    arm_gap(host);
    host.push(pkt(0, 0, 0));
    host.push(PROBE); // on the DISABLED title, buttons up
    click(host); // a click on the disabled title: nothing
    host.push(PROBE);
    glide(host, 0, 100 - ON_EDIT.1, 0); // down, out of the bar → the track ends
    for _ in 0..3 {
        host.push(pkt(0, 0, 0));
    }

    let (fired, bret, msg) = evnt_multi_timed(&mut pb, 200);
    let (mx, my) = tos_api::vdi::mouse_position();
    let samples = host.samples();
    eprintln!("a89.2: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} samples={samples:?}");
    assert_eq!(samples.len(), 2, "both probes must have run inside the track; samples = {samples:?}");
    for (i, s) in samples.iter().enumerate() {
        assert_eq!(s.at, ON_EDIT, "probe {i} ran on the Edit title");
        assert_eq!(
            s.state_a & SELECTED,
            0,
            "probe {i}: a DISABLED title is never hilited (do_chg chkdisabled, aes/gemmnlib.c:158-160, \
             via menu_select :466); Edit.ob_state = {:#06x}",
            s.state_a
        );
        assert_eq!(
            s.pen_a,
            DESK,
            "probe {i}: no box may be dropped under a DISABLED title (menu_down runs menu_sr/ob_draw \
             only if do_chg succeeds, aes/gemmnlib.c:229-244); pen at {EDIT_BOX_PX:?} = {}",
            s.pen_a
        );
    }
    assert_eq!(fired, MU_TIMER, "nothing but the timer; fired = {fired:#06x}, msg = {msg:?}");
    // SAFETY: `tree` is the leaked, live menu tree.
    let edit_state = unsafe { (*tree.offset(TITLE_EDIT as isize)).state };
    assert_eq!(edit_state, DISABLED, "the title's state is untouched after the track");
    assert_eq!(host.raster.read_pixel(EDIT_BOX_PX.0, EDIT_BOX_PX.1), DESK, "the box area still shows the desktop");
}

// ── A103: submenu remnants ─────────────────────────────────────────────────

/// **A103.1 — a press on the submenu box's own area picks `smroot`.** The
/// same-tree submenu's items sit 2 px inside the box, so its first column
/// is box-only. Hover *File*, onto *Open* (the submenu opens beside it),
/// glide into the submenu onto that column on the *Beta* row. EmuTOS:
/// `cur_submenu = ob_find(smtree, smroot, 1, x, y)` = `smroot`
/// (`gemmnlib.c:417`; `ob_find` returns the last object found,
/// `gemoblib.c:1034-1113`) → `SUBMENU_STATE` (`:418-422`), and
/// `menu_select(smtree, cur_submenu, last_submenu, TRUE)` hilites the box
/// (`:474-475`) — `do_chg` → `ob_change`, `xor_is_ok` for a black-and-white
/// `G_BOX`, one XOR of the interior (`gemoblib.c:1295-1304`). The probe
/// packet records that: `smroot` SELECTED, no item SELECTED, the gap pixel
/// the device complement of white. A press there breaks the loop
/// (`:373-374`) and the cleanup, `menu_state == SUBMENU_STATE`, returns
/// `*pitem = cur_submenu = smroot` (`:494-496`): `MN_SELECTED (File, 17)`.
#[test]
fn a103_press_on_the_submenu_boxs_own_area_picks_smroot() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh_same_tree_submenu();
    let (sbx, sby, _, _) = submenu_box();
    // The box's first column (box-only: the items start at `SUB_INSET`), on
    // the *Beta* row; inside the 1-px border, so inside the XOR'd interior.
    let gap = (sbx + 1, sby + ITEM_H + 9);
    // The probe pixel: the same column on the *Alpha* row — inside the
    // XOR'd interior too, and clear of the 16×16 cursor sprite whose hot
    // spot sits at `gap`.
    let probe_px = (sbx + 1, sby + 6);
    let white = host.raster.rgb565_to_device(host.raster.pel_to_device16(WHITE));
    set_probe(tree, SUB_ROOT, SUB_BETA_SAME, probe_px, probe_px);

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0); // down onto *Open* → submenu opens
    host.push(pkt(0, 0, 0));
    glide(host, gap.0 - ON_OPEN.0, gap.1 - ON_OPEN.1, 0); // into the submenu's gap column
    host.push(pkt(0, 0, 0));
    host.push(PROBE); // the state while the pointer rests on the box's own area
    host.push(pkt(0, 0, 0));
    click(host); // press there

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    let (mx, my) = tos_api::vdi::mouse_position();
    let samples = host.samples();
    eprintln!("a103.1: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} samples={samples:?} gap={gap:?}");
    assert_eq!(samples.len(), 1, "the probe must have been consumed inside the track; samples = {samples:?}");
    let s = samples[0];
    assert_eq!(s.at, gap, "the probe ran with the pointer on the box's own column");
    assert_eq!(
        s.state_a & SELECTED,
        SELECTED,
        "on the submenu box's own area the box itself is cur_submenu and hilited \
         (ob_find answers smroot, aes/gemmnlib.c:417; menu_select :474-475); smroot.ob_state = {:#06x}",
        s.state_a
    );
    assert_eq!(s.state_b & SELECTED, 0, "no submenu item is hilited on the box's own area; Beta.ob_state = {:#06x}", s.state_b);
    assert_eq!(
        s.dev_a,
        white ^ 0x00FF_FFFF,
        "the box interior is XOR'd for the hilite (ob_change, aes/gemoblib.c:1295-1304); pixel {probe_px:?} = {:#08x}",
        s.dev_a
    );
    assert_eq!(fired & MU_BUTTON, 0, "the press leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(
        fired & MU_MESAG,
        MU_MESAG,
        "a press on the submenu box's own area selects smroot in EmuTOS \
         (*pitem = cur_submenu, aes/gemmnlib.c:494-496) — NewTOS delivered nothing; fired = {fired:#06x}"
    );
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, SUB_ROOT), "msg = {msg:?}");
    // SAFETY: `tree` is the leaked, live menu tree.
    let state = |i: i16| unsafe { (*tree.offset(i as isize)).state };
    assert_eq!(state(SUB_ROOT) & SELECTED, 0, "the cleanup clears smroot's SELECTED (do_chg, :488)");
    assert_eq!(state(ITEM_OPEN) & SELECTED, 0, "the parent item is un-hilited by the cleanup (:493)");
}

/// **A103.2 — a DISABLED item with a submenu attached opens nothing.**
/// *Open* carries the submenu (attached, then `menu_ienable(Open, 0)`).
/// Hover *File*, onto *Open*, rest: the probe finds *Open* not SELECTED
/// and the submenu's probe pixel still the desktop — `display_submenu` runs
/// only inside `if (menu_select(tree, cur_item, last_item, TRUE))`
/// (`gemmnlib.c:470-475`) and `menu_select` → `do_chg(…, chkdisabled)`
/// refuses the DISABLED item (`:158-160`). A press there cancels: the
/// return is gated on `do_chg(tree, cur_item, SELECTED, FALSE, FALSE,
/// TRUE)` (`:493`), which refuses it too — no `MN_SELECTED`.
#[test]
fn a103_a_disabled_item_with_a_submenu_attached_opens_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, tree) = fresh();
    let sub = attach_submenu(&mut pb, tree);
    menu_ienable(&mut pb, tree, ITEM_OPEN, false);
    let (ppx, ppy) = submenu_probe_px();
    set_probe(tree, ITEM_OPEN, ITEM_OPEN, (ppx, ppy), (ppx, ppy));
    assert_eq!(host.raster.read_pixel(ppx, ppy), DESK, "the probe point shows the desktop dither before the track");
    // SAFETY: `sub` is the leaked, live submenu tree.
    let sub_state = |i: i16| unsafe { (*sub.offset(i as isize)).state };

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0); // down onto the DISABLED *Open*
    host.push(pkt(0, 0, 0));
    host.push(PROBE); // resting on it
    host.push(pkt(0, 0, 0));
    click(host); // press on the disabled item: the cancel

    let (fired, bret, msg) = evnt_multi_timed(&mut pb, 200);
    let (mx, my) = tos_api::vdi::mouse_position();
    let samples = host.samples();
    eprintln!("a103.2: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} samples={samples:?}");
    assert_eq!(samples.len(), 1, "the probe must have been consumed inside the track; samples = {samples:?}");
    let s = samples[0];
    assert_eq!(s.at, ON_OPEN, "the probe ran on the item");
    assert_eq!(s.state_a & DISABLED, DISABLED, "Open is DISABLED for this case");
    assert_eq!(s.state_a & SELECTED, 0, "a DISABLED item is never hilited (do_chg chkdisabled, :158-160)");
    assert_eq!(
        s.pen_a,
        DESK,
        "no submenu may open under a DISABLED item (display_submenu only after menu_select succeeds, \
         aes/gemmnlib.c:470-475); pen at ({ppx},{ppy}) = {}",
        s.pen_a
    );
    assert_eq!(fired & MU_MESAG, 0, "no selection from a DISABLED item (:493); fired = {fired:#06x}, msg = {msg:?}");
    assert_eq!(fired & MU_BUTTON, 0, "the cancelling press leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(sub_state(0) & SELECTED, 0, "nothing of the submenu tree was touched");
    assert_eq!(host.raster.read_pixel(ppx, ppy), DESK, "the submenu area still shows the desktop after the track");
}

// ── A90: evnt_mouse through amouse ─────────────────────────────────────────

/// **A90a — a rectangle crossed while the Control Manager still holds the
/// mouse is not delivered; the release delivers it.** Select *Open* on a
/// press (as A73 1a): `evnt_multi` returns `MN_SELECTED` with the press still
/// held and the Control Manager parked in `ct_msgup`
/// (`gemctrl.c:100-101`). Now the app calls `evnt_mouse(leave, 11×11 around
/// the pointer)`. Script: 30 px to the right while held (out of the
/// rectangle), dwell, release, dwell. EmuTOS: `amouse` finds the pointer
/// inside → parks on the app's `c_msleep`; the move's `post_mouse(ctl_pd, …)`
/// never visits it; the release ends `ct_msgup`, `ctlmgr` loops into
/// `w_setactive` → `set_mown(app)` → `post_mouse(app, xrat, yrat)`
/// (`gemctrl.c:441-442`, `geminput.c:225-231`) → the wait completes against
/// the live position. `evnt_mouse` therefore returns **after the release**:
/// `EV_MB` (the live `button`, `gemevlib.c:127`) is 0.
///
/// The same script also pins that `ev_mouse` has **no** head drain (A92):
/// the first packet is queued before the call, and a `forker()` ahead of
/// `amouse` would make it the arm-time position — an ownership-blind fire
/// with the press still held (`EV_MB = 1`). `ev_mouse` arms straight away
/// (`gemevlib.c:123-128`); only `ev_multi` drains first (`:169-170`).
#[test]
fn evnt_mouse_rectangle_crossed_while_the_selecting_press_is_held_is_delivered_at_the_release() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, _tree) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0);
    host.push(pkt(0, 0, 0));
    host.push(pkt(0, 0, 1)); // press → the selection, held
    host.push(pkt(0, 0, 1));

    let (fired, _bret, mb, msg) = evnt_multi_teradesk(&mut pb);
    eprintln!("a90a setup: fired={fired:#06x} mb={mb} msg={msg:?} queued={}", host.queued());
    assert_eq!(fired, MU_MESAG, "the selection alone; fired = {fired:#06x}");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the selecting press must still be held");
    let (px, py) = tos_api::vdi::mouse_position();

    // Consumed inside evnt_mouse: out of the rectangle while held, dwell,
    // release, dwell.
    glide(host, 30, 0, 1);
    host.push(pkt(0, 0, 1));
    host.push(pkt(0, 0, 1));
    host.push(pkt(0, 0, 0)); // the release
    for _ in 0..3 {
        host.push(pkt(0, 0, 0));
    }

    let (rc, mx, my, ev_mb) = evnt_mouse(&mut pb, 1, (px - 5, py - 5, 11, 11));
    eprintln!("a90a: rc={rc} at=({mx},{my}) EV_MB={ev_mb:#04x} queued={}", host.queued());
    assert_eq!(rc, 1);
    assert_eq!(
        ev_mb & 1,
        0,
        "evnt_mouse returned while the selecting press was still held (pointer at ({mx},{my})): \
         the rectangle was crossed while ctl_pd owned the mouse (ct_msgup, aes/gemctrl.c:100-101), \
         so post_mouse(gl_mowner, …) never reached the app's parked amouse wait \
         (aes/geminput.c:446-456); only the release (w_setactive → set_mown(app) → \
         post_mouse(app, xrat, yrat), :220-231) completes it — EV_MB = {ev_mb:#04x}"
    );
    assert_eq!((mx, my), (px + 30, py), "the pointer left the rectangle");
}

/// **A90b (guard) — the arm-time test is ownership-blind.** Same hold as
/// A90a (`ctl_pd` owns the mouse, press held); `evnt_mouse(enter, a rectangle
/// the pointer is already in)` returns at once — `amouse` → `in_mrect` →
/// `azombie` (`geminput.c:575-576`) — without consuming a single packet, and
/// reports the live button (held).
#[test]
fn evnt_mouse_satisfied_at_arm_time_returns_at_once_regardless_of_ownership() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, _tree) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    arm_gap(host);
    glide(host, 0, ON_OPEN.1 - ON_FILE.1, 0);
    host.push(pkt(0, 0, 0));
    host.push(pkt(0, 0, 1));
    host.push(pkt(0, 0, 1));
    let (fired, _, _, msg) = evnt_multi_teradesk(&mut pb);
    assert_eq!((fired, msg[0]), (MU_MESAG, MN_SELECTED), "setup: the selection, press held");
    let (px, py) = tos_api::vdi::mouse_position();

    host.push(pkt(0, 0, 1)); // would be consumed only by a wait that pumps
    host.push(pkt(0, 0, 0));
    let before = host.queued();
    let (rc, mx, my, ev_mb) = evnt_mouse(&mut pb, 0, (px - 5, py - 5, 11, 11));
    eprintln!("a90b: rc={rc} at=({mx},{my}) EV_MB={ev_mb:#04x} queued {before} -> {}", host.queued());
    assert_eq!(rc, 1);
    assert_eq!((mx, my), (px, py));
    assert_eq!(ev_mb & 1, 1, "the live button is reported (gemevlib.c:127)");
    // One packet goes to the call's own dsptch() (every AES call ends there);
    // the wait itself pumped nothing.
    assert_eq!(host.queued(), before - 1, "the wait must not have pumped: satisfied at arm time");
    drain(host, &mut pb);
}

// ── A92: the arm-time state after the head drain ───────────────────────────

/// **A92a — a move queued before the call counts for the arm-time
/// position.** Pointer parked at (320,300); one packet moving it 20 px right
/// is queued but not yet pumped — the board's case of a packet that arrived
/// between two traps. `evnt_multi(MU_M1 leave 21×21 around (320,300) |
/// MU_TIMER 0)`: EmuTOS drains the fork ring first (`chkkbd(); forker();`,
/// `gemevlib.c:169-170`), so `xrat/yrat` are (340,300) when `in_mrect` runs
/// — `MU_M1` fires with the timer. Sampling before the drain misses it.
#[test]
fn a_move_queued_before_the_call_counts_for_the_arm_time_position() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, _tree) = fresh();
    let (px, py) = tos_api::vdi::mouse_position();
    assert_eq!((px, py), (320, 300));

    host.push(pkt(20, 0, 0)); // queued, not pumped
    let (fired, mx, my, _mb, _bret) = evnt_multi_poll(&mut pb, MU_M1, (1, px - 10, py - 10, 21, 21));
    eprintln!("a92a: fired={fired:#06x} at=({mx},{my})");
    assert_eq!((mx, my), (px + 20, py), "the queued move was applied by the call");
    assert_eq!(
        fired & MU_M1,
        MU_M1,
        "a packet queued before the call belongs to the arm-time position: ev_multi runs \
         chkkbd(); forker(); before its quick checks and before amouse arms \
         (aes/gemevlib.c:169-170, aes/geminput.c:575-576) — fired = {fired:#06x}"
    );
    assert_eq!(fired & MU_TIMER, MU_TIMER, "the 0 ms timer fires on the same pass");
}

/// **A92b — a press queued before the call counts for the arm-time level.**
/// Same drain, for `button`: a press queued but not pumped, `evnt_multi(wait
/// for a press | MU_TIMER 0)` while the app owns the mouse (desktop, no
/// track) → the quick check (`downorup(button, buparm)`, `gemevlib.c:196`)
/// sees the press after `forker()`: `MU_BUTTON` fires.
///
/// ZVIEW02: the wait carries a 300-ms timer instead of a 0-ms poll. Whether the
/// press is published by the head drain depends on `gl_bpend`, which earlier
/// cases of this binary leave raised (a multi-click wait taken off by
/// `acancel` does not lower it, `aes/gemasync.c:96-114,217-243`): then the
/// press opens the button-delay window and is published only when it closes
/// (`b_click`/`b_delay`, `aes/geminput.c:133-140,155-171`), 220 ms later —
/// still within the timer, still `MU_BUTTON` with the button down. With
/// `gl_bpend == 0` the quick check takes it at once; sampling the arm-time
/// level *before* the drain would then leave the wait parked behind a press
/// already published (no `post_button` reaches it) and only the timer would
/// fire.
#[test]
fn a_press_queued_before_the_call_counts_for_the_arm_time_level() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, _tree) = fresh();

    host.push(pkt(0, 0, 1)); // queued, not pumped
    let (fired, _mx, _my, mb, bret) = {
        let mut buf = Box::new([0i16; 8]);
        pb.intin.fill(0);
        pb.intin[0] = MU_BUTTON | MU_TIMER;
        pb.intin[1] = 0x0102; // wait for a press (TeraDesk's bflag/clicks)
        pb.intin[2] = 3;
        pb.intin[3] = 0;
        pb.intin[14] = 300;
        pb.addrin[0] = buf.as_mut_ptr() as u64;
        pb.call(25, 16, 7, 1);
        pb.addrin[0] = 0;
        (pb.intout[0], pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[6])
    };
    eprintln!("a92b: fired={fired:#06x} mb={mb} bret={bret}");
    assert_eq!(mb & 1, 1, "the queued press was applied by the call");
    assert_eq!(
        fired & MU_BUTTON,
        MU_BUTTON,
        "a press queued before the call belongs to the arm-time level (aes/gemevlib.c:169-170, \
         :196); fired = {fired:#06x}"
    );
    // Release, so the next case starts with the button up.
    host.push(pkt(0, 0, 0));
    drain(host, &mut pb);
}
