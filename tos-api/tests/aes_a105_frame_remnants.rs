//! A105 — **the window frame's remnants after A95: `W_INFO` text origin,
//! name centring, `thumb_geom`'s axis, `changecol` on a pressed gadget.**
//!
//! Reference: `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/shots/
//! falcon-tc-window-c.png` — EmuTOS on a Hatari Falcon030, 640x480
//! truecolor, the topped `C:\` window at `(16,96,304,192)` with the info
//! line ` 514 bytes used in 1 items.`; frame-relative `(dx,dy)` = screen
//! `(16+dx, 96+dy)`. Read with `measurements/measurement-2026-09-21-a95/method/dumpwin.py`.
//!
//! 1. **`W_INFO` text origin** — `gl_ainfo` is `TE_LEFT` with `te_thickness
//!    1` (`gl_asamp`, `aes/gemwmlib.c:154-157`), `FL3DNONE` (no growth,
//!    `movetext` FALSE, `aes/gemoblib.c:521-524`); `gr_gtext` gets `c =
//!    gr_inside(&t, 1)` (`:737-738`) and `gr_just` adds `(17-16+1)/2 = 1`
//!    row (`aes/gemgraf.c:574-576`): glyph cells at `(1, 2)` of the info
//!    box, i.e. frame `(1, 25)` — the `5`'s ink on rows `27..38`, columns
//!    `10..15`; the `y`'s descender on rows `39..40`. NewTOS drew at
//!    `(3, 24)`.
//! 2. **Name centring** — `gr_just` on `gr_inside(grown, 1)` (`:735-738`)
//!    after the activator's `y-1` (`:742-755`): `x = name_x - 1 + (name_w +
//!    2 - textw + 1) / 2`, which differs from the old `name_x + (name_w -
//!    textw) / 2` by one column for an odd slack; a text wider than the cell
//!    has no slack (`gsx_tcalc` clamps `textw` to the cell, `:480-497`) and
//!    stays at the cell origin, cut to whole glyphs — left-aligned from
//!    frame column 24 of a `CLOSER` window.
//! 3. **`thumb_geom`** — the horizontal bar's default/minimum elevator is
//!    `gl_wbox`, the vertical one's `gl_hbox` (`gemwmlib.c:431-434,469-471`);
//!    equal on this profile, pinned by a unit test on the helper
//!    (`window.rs`, `thumb_geom_takes_the_axis_minimum`).
//! 4. **`changecol`** — a pressed `FL3DACT` gadget changes colour only with
//!    `actcolchange` set (`objc_sysvar(LK3DACT)`; default 0, `init_3d`,
//!    `:1332`): a hollow interior goes solid black, a solid one takes its
//!    complement (`:681-690`), the glyph takes its complement in `MD_TRANS`
//!    (`:699-706`), and the bevel is judged against the changed interior
//!    (`:950`). The frame gadgets' default word is `0x1101` (`wm_start`,
//!    `gemwmlib.c:1346-1349`): `IP_HOLLOW` with `ic = BLACK`, so the 3D
//!    face substitution (`:619-636`, hollow **and** white only) does not
//!    apply and the face is `gr_rect`'s hollow white. With the default the
//!    press is the sunk bevel and the moved glyph alone.
//!
//! Harness (raster stand-in, scripted host with screen snapshots at a pump,
//! parameter block) copied from `aes_a100_a102_desk_menu_geometry.rs`; the
//! pixel helpers from `aes_window_frame_reference.rs`.

#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

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
    /// Drawing primitives issued so far (`fill_rect`, `fill_pattern`,
    /// `abline`) — A130: a held elevator must not be repainted while the
    /// pointer stands still.
    paints: AtomicUsize,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            paints: AtomicUsize::new(0),
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
        self.paints.fetch_add(1, Ordering::Relaxed);
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
        _last: bool,
    ) -> u16 {
        self.paints.fetch_add(1, Ordering::Relaxed);
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        let (mut x, mut y) = (x0 as i32, y0 as i32);
        let (dx, dy) = ((x1 as i32 - x).abs(), -(y1 as i32 - y).abs());
        let (sx, sy) = (if x < x1 as i32 { 1 } else { -1 }, if y < y1 as i32 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            if m & 0x8000 != 0 {
                // XOR complements the target (the drag outlines, `gsx_xbox`),
                // so drawing a line twice restores what was under it.
                if matches!(mode, WriteMode::Xor) {
                    let old = self.get_device_pixel(x as i16, y as i16);
                    self.put(x as i16, y as i16, old ^ 0x00FF_FFFF, clip);
                } else {
                    self.put(x as i16, y as i16, v, clip);
                }
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
    /// Mode-aware, as the A99 harness (`aes_screen_buffer_save_under.rs`):
    /// the hover toggle is `fill_pattern` in `WriteMode::Xor`, which must
    /// complement the target, not paint it.
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        self.paints.fetch_add(1, Ordering::Relaxed);
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

/// One synthetic mouse packet, in the form the VDI's `mouse_int` receives it,
/// plus two harness-side side effects taken **at the pump, before the packet
/// is fed**: `snap` copies the raster into the host's snapshot list under that
/// index (the screen as the AES left it after the previous packet), and `key`
/// queues one `EV_KRET` word for `console_try_read_key` (a key struck while
/// the pointer rests where the previous packets put it).
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    /// VDI convention: bit 0 = left, bit 1 = right.
    buttons: u8,
    snap: Option<usize>,
    key: Option<u16>,
}

/// A plain motion/button packet.
const fn pkt(dx: i16, dy: i16, buttons: u8) -> Packet {
    Packet { dx, dy, buttons, snap: None, key: None }
}

/// An idle packet that snapshots the screen under `index` first.
const fn snap(index: usize) -> Packet {
    Packet { dx: 0, dy: 0, buttons: 0, snap: Some(index), key: None }
}

/// An idle packet that queues the key `word` (`scancode << 8 | ascii`) first.
const fn key(word: u16) -> Packet {
    Packet { dx: 0, dy: 0, buttons: 0, snap: None, key: Some(word) }
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
    /// Screen copies taken by `snap` packets, by index.
    snaps: Mutex<Vec<(usize, Vec<u32>)>>,
    /// The backend's paint count at each `snap` packet, by index.
    snap_paints: Mutex<Vec<(usize, usize)>>,
    /// Keys queued by `key` packets, handed out by `console_try_read_key`.
    keys: Mutex<VecDeque<u16>>,
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
            snaps: Mutex::new(Vec::new()),
            snap_paints: Mutex::new(Vec::new()),
            keys: Mutex::new(VecDeque::new()),
            clock: AtomicU64::new(0),
            idle: AtomicUsize::new(0),
        }
    }
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn clear(&self) {
        self.script.lock().unwrap().clear();
        self.snaps.lock().unwrap().clear();
        self.snap_paints.lock().unwrap().clear();
        self.keys.lock().unwrap().clear();
        self.idle.store(0, Ordering::Relaxed);
    }
    /// The paint count at the `snap(index)` packet — `None` if never pumped.
    fn paints_at(&self, index: usize) -> Option<usize> {
        let v = self.snap_paints.lock().unwrap();
        v.iter().find(|(i, _)| *i == index).map(|(_, n)| *n)
    }
    /// The screen copy a `snap(index)` packet took, as device pixels (the
    /// XOR complement of the palette's RGB555 white is no palette entry, so
    /// `read_pixel`'s index mapping cannot express a hovered row) — `None` if
    /// that packet was never pumped.
    fn snapshot(&self, index: usize) -> Option<Vec<u32>> {
        let snaps = self.snaps.lock().unwrap();
        let (_, px) = snaps.iter().find(|(i, _)| *i == index)?;
        Some(px.clone())
    }
}

impl Host for ScriptHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        self.keys.lock().unwrap().pop_front()
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
            Some(p) => {
                self.idle.store(0, Ordering::Relaxed);
                if let Some(i) = p.snap {
                    let px = self.raster.px.lock().unwrap().clone();
                    self.snaps.lock().unwrap().push((i, px));
                    let n = self.raster.paints.load(Ordering::Relaxed);
                    self.snap_paints.lock().unwrap().push((i, n));
                }
                if let Some(k) = p.key {
                    self.keys.lock().unwrap().push_back(k);
                }
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
/// round-trips the pens the menu draws with.
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

// ── the kernel desktop's menu tree ─────────────────────────────────────────

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
/// Every gadget, `INFO` included — the reference window has all of them.
const KIND_ALL: i16 =
    NAME | CLOSER | FULLER | MOVER | INFO | SIZER | UPARROW | DNARROW | VSLIDE | LFARROW | RTARROW | HSLIDE;

const WF_NAME: i16 = 2;
const WF_INFO: i16 = 3;
const WF_NEWDESK: i16 = 14;
const WF_DCOLOR: i16 = 19;

const MU_MESAG: i16 = 0x0010;
const MU_BUTTON: i16 = 0x0002;
const MU_TIMER: i16 = 0x0020;
const WM_CLOSED: i16 = 22;

// palette indices the AES paints with (`obdefs.h:109-124`), as the test
// palette maps them
const WHITE: Pel = 0;
const BLACK: Pel = 1;
const LWHITE: Pel = 8;
const LBLACK: Pel = 9;

// frame metrics for the 8×16 system font on a ≥ 640-wide screen
// (`gl_wbox`/`gl_hbox` = 19, `ADJ3DSTD` = 2, `adj_wbox`/`adj_hbox` = 23)
const GL_WBOX: i16 = 19;
const GL_HBOX: i16 = 19;
const GL_WCHAR: i16 = 8;
const GL_HCHAR: i16 = 16;
const ADJ3DSTD: i16 = 2;
const ADJ_WBOX: i16 = GL_WBOX + 2 * ADJ3DSTD;
const ADJ_HBOX: i16 = GL_HBOX + 2 * ADJ3DSTD;

// ── the rig ───────────────────────────────────────────────────────────────

/// The AES window table and the installed host are process-wide statics, so
/// the cases in this binary must not run concurrently.
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

/// `objc_sysvar(SET, LK3DACT, txtmove, colchange)` (48) — `int_in[0]` = 1
/// (set), `int_in[1]` = 2 (`LK3DACT`), `int_in[2..3]` = the pair.
fn set_lk3dact(pb: &mut Aespb, txtmove: i16, colchange: i16) {
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.intin[1] = 2;
    pb.intin[2] = txtmove;
    pb.intin[3] = colchange;
    pb.call(48, 4, 3, 0);
    assert_eq!(pb.intout[0], 1, "objc_sysvar(SET, LK3DACT, {txtmove}, {colchange})");
}

/// Fresh AES window state (`wind_new`), a cleared framebuffer, the registered
/// desktop background tree, the compiled default colour words and 3D
/// variables restored, the cursor **off**, the pointer parked, nothing queued.
fn fresh() -> (&'static ScriptHost, Aespb) {
    let host = rig();
    host.clear();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    host.raster.clear();

    pb.intin.fill(0);
    pb.intin[0] = 256; // graf_mouse(M_OFF)
    pb.call(78, 1, 1, 0);

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;

    // `WF_DCOLOR` and the 3D variables are process-wide state; put them
    // back to `wm_start`'s / `init_3d`'s values so a case cannot leak into
    // the next.
    for g in 0..19 {
        let (t, b) = match g {
            3 => (0x11a1, 0x1101),
            12 | 17 => (0x1111, 0x1111),
            _ => (0x1101, 0x1101),
        };
        set_dcolor(&mut pb, g, t, b);
    }
    set_lk3dact(&mut pb, 1, 0);

    move_to(host, &mut pb, 600, 440, 0);
    drain_messages(&mut pb);
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

/// `graf_mkstate` — one AES call, hence one `dsptch()` → one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) -> (i16, i16, i16) {
    pb.call(79, 0, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Consume every packet still queued.
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..2000 {
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

/// The application's wait (`MU_MESAG | MU_BUTTON`, TeraDesk's click
/// parameters) with a timer. Returns `(fired, EV_BRET, msg)`.
fn evnt_multi_timed(pb: &mut Aespb, ms: u16) -> (i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_TIMER;
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

/// Empty the owner FIFO.
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

/// `ev_multi` drains the queue once before its arms (A92): one idle packet
/// so a press queued after it is post-arm input.
fn arm_gap(host: &ScriptHost) {
    host.push(pkt(0, 0, 0));
}

/// Packets (1 ms each) a press is held so that the button-delay window it
/// opened has closed: 11 AES ticks of 20 ms (`gl_dclick` at the default
/// index 3, `aes/gemevlib.c:300-309`) plus the tick phase, with margin.
const HOLD_PAST_WINDOW: usize = 240;

/// `n` idle packets with the left button held.
fn hold_down(host: &ScriptHost, n: usize) {
    for _ in 0..n {
        host.push(pkt(0, 0, 1));
    }
}

/// An idle packet, button **held**, that snapshots the screen under `index`
/// first — the screen as the AES left it after the previous packet.
const fn snap_held(index: usize) -> Packet {
    Packet { dx: 0, dy: 0, buttons: 1, snap: Some(index), key: None }
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
    /// Frame-relative → screen x (`dx < 0` counts from the right border pixel).
    fn fx(self, dx: i16) -> i16 {
        if dx < 0 { self.x + self.w + dx } else { self.x + dx }
    }
    /// Frame-relative → screen y (`dy < 0` counts from the bottom border pixel).
    fn fy(self, dy: i16) -> i16 {
        if dy < 0 { self.y + self.h + dy } else { self.y + dy }
    }
}

/// The 3D-grown rect `just_draw` paints an `FL3DACT` object on
/// (`gemoblib.c:503-508`: `ADJ3DSTD` out on every side).
fn grown(r: R) -> R {
    r.inset(-ADJ3DSTD)
}

/// The `W_NAME` object rect for a window with `CLOSER | FULLER | NAME` —
/// `w_bldactive` (`gemwmlib.c:515-537`): the closer and the fuller each take a
/// full `adj_wbox` of the title box, the name sits `ADJ3DSTD` inside what is
/// left, `gl_hbox` tall at `y = ADJ3DSTD`.
fn name_rect(b: R) -> R {
    R { x: b.x + ADJ_WBOX + ADJ3DSTD, y: b.y + ADJ3DSTD, w: b.w - 2 * ADJ_WBOX - 2 * ADJ3DSTD, h: GL_HBOX }
}

/// The `W_INFO` object rect: the full width below the `adj_hbox` title band,
/// `gl_hbox` tall (`gemwmlib.c:561-567`).
fn info_rect(b: R) -> R {
    R { x: b.x, y: b.y + ADJ_HBOX, w: b.w, h: GL_HBOX }
}

/// Where `just_draw` puts the name's glyph cells (A105 (2)): `c =
/// gr_inside(grown(name), 1)`, `c.y--` (`acttxtmove`), then `gr_just`:
/// `textw = min(c.w, n·8)`, `numchs = min(n, textw/8)`, `x = c.x + (c.w -
/// textw + 1)/2` when there is slack, else `c.x`; `y = c.y + (21 - 16 + 1)/2`.
/// Returns `(x, y, numchs)`.
fn name_cells(nr: R, n: i16) -> (i16, i16, i16) {
    let c = grown(nr).inset(1);
    let (cx, cy) = (c.x, c.y - 1);
    let textw = c.w.min(n * GL_WCHAR);
    let numchs = n.min(textw / GL_WCHAR);
    let slack = c.w - textw;
    let x = if slack > 0 { cx + (slack + 1) / 2 } else { cx };
    let y = cy + (c.h - GL_HCHAR + 1) / 2;
    (x, y, numchs)
}

fn px(host: &ScriptHost, x: i16, y: i16) -> Pel {
    host.raster.read_pixel(x, y)
}

/// Every pixel of `r`, as `(x, y, pel)`.
fn pixels(host: &ScriptHost, r: R) -> Vec<(i16, i16, Pel)> {
    let mut v = Vec::with_capacity(r.w as usize * r.h as usize);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            v.push((x, y, px(host, x, y)));
        }
    }
    v
}

/// Assert every pixel of `r` is `pel`, naming the first offender.
fn assert_all(host: &ScriptHost, r: R, pel: Pel, what: &str) {
    for (x, y, p) in pixels(host, r) {
        assert_eq!(p, pel, "{what}: pixel ({x},{y}) in {r:?}");
    }
}

/// Bounding box `(x0, x1, y0, y1)` of the pixels of pen `pen` in `r`, or `None`.
fn pen_bbox(host: &ScriptHost, r: R, pen: Pel) -> Option<(i16, i16, i16, i16)> {
    bbox(pixels(host, r).into_iter().filter(|p| p.2 == pen).map(|p| (p.0, p.1)))
}

/// Bounding box of the BLACK pixels in `r`.
fn ink_bbox(host: &ScriptHost, r: R) -> Option<(i16, i16, i16, i16)> {
    pen_bbox(host, r, BLACK)
}

fn bbox(it: impl Iterator<Item = (i16, i16)>) -> Option<(i16, i16, i16, i16)> {
    let v: Vec<(i16, i16)> = it.collect();
    if v.is_empty() {
        return None;
    }
    Some((
        v.iter().map(|p| p.0).min().unwrap(),
        v.iter().map(|p| p.0).max().unwrap(),
        v.iter().map(|p| p.1).min().unwrap(),
        v.iter().map(|p| p.1).max().unwrap(),
    ))
}

/// The set-bit bounding box `(col0, col1, row0, row1)` of the 8×16 system
/// font glyph of `c` — the same bitmap the AES draws (`vdi::font::glyph`).
fn glyph_bbox(c: u8) -> Option<(i16, i16, i16, i16)> {
    let g = tos_api::vdi::font::glyph(c as u16);
    bbox((0..16).flat_map(|row| (0..8).filter(move |col| g[row] & (0x80 >> col) != 0).map(move |col| (col as i16, row as i16))))
}

/// The expected ink bounding box of `text` drawn with its glyph cells' top-left
/// at `(x, y)` — the union of every glyph's set bits.
fn text_bbox(x: i16, y: i16, text: &[u8]) -> Option<(i16, i16, i16, i16)> {
    bbox(text.iter().enumerate().flat_map(|(i, &c)| {
        let g = tos_api::vdi::font::glyph(c as u16);
        let gx = x + i as i16 * GL_WCHAR;
        (0..16).flat_map(move |row| (0..8).filter(move |col| g[row] & (0x80 >> col) != 0).map(move |col| (gx + col as i16, y + row as i16)))
    }))
}

/// A snapshot's pixel as a device value, and the device value of a pen.
fn at(snap: &[u32], x: i16, y: i16) -> u32 {
    snap[y as usize * SCREEN_W as usize + x as usize]
}
fn dev(host: &ScriptHost, pen: Pel) -> u32 {
    host.raster.rgb565_to_device(host.raster.pel_to_device16(pen))
}

// ── (1) the info line's text origin ───────────────────────────────────────

/// The reference window `(16,96,304,192)` with the reference's info line.
/// `gl_ainfo` is `TE_LEFT`, `te_thickness = 1` (`gemwmlib.c:154-157`) and
/// `W_INFO` is `FL3DNONE` (`:110-129`): `just_draw` neither grows the box
/// nor moves the text (`gemoblib.c:503-524`); `gr_gtext(TE_LEFT, IBM, …,
/// gr_inside(&t, 1))` (`:735-738`) with `gr_just`'s `y += (17 - 16 + 1) / 2`
/// (`gemgraf.c:574-576`) puts the glyph cells at `(1, 2)` of the info box =
/// frame `(1, 25)`. Reference pixels (`falcon-tc-window-c.png`, frame-
/// relative, `dumpwin.py … 0 22 120 43`): the `5` (cell 1, columns `9..16`)
/// inks columns `10..15`, rows `27..38`; the `y` of `bytes` (cell 6, columns
/// `49..56`) inks columns `50..55`, rows `30..40` (descender on `39..40`);
/// rows `24..26` of the info interior are white. NewTOS drew the cells at
/// `(3, 24)`: two columns right, one row up (first ink row 26).
#[test]
fn a105_1_info_text_starts_one_column_inside_the_border_on_the_third_row() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b = R { x: 16, y: 96, w: 304, h: 192 };
    let h = open_window(&mut pb, KIND_ALL, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"C:\\\0");
    set_string(&mut pb, h, WF_INFO, b" 514 bytes used in 1 items.\0");
    let ir = info_rect(b);
    assert_eq!((ir.y, ir.h), (b.fy(23), 19), "the info band: rows 23..41 of the frame");

    // Cell 1 (the `5`): frame columns 9..16, rows 25..40.
    let cell = |i: i16| R { x: b.fx(1 + i * GL_WCHAR), y: b.fy(25), w: GL_WCHAR, h: GL_HCHAR };
    assert_eq!(
        ink_bbox(host, cell(1)),
        Some((b.fx(10), b.fx(15), b.fy(27), b.fy(38))),
        "the `5` of the info line: reference ink columns 10..15, rows 27..38 (cell at frame (9,25))"
    );
    assert_eq!(
        ink_bbox(host, cell(6)),
        Some((b.fx(50), b.fx(55), b.fy(30), b.fy(40))),
        "the `y` of `bytes`: reference ink columns 50..55, rows 30..40 (descender on rows 39..40)"
    );
    // The two rows between the info box's top border and the glyph tops, and
    // the row under the deepest descender, are interior white.
    assert_all(host, R { x: b.fx(1), y: b.fy(24), w: 120, h: 3 }, WHITE, "info interior rows 24..26 (ref: white)");
    // The whole line, every glyph, as the font puts it at (1, 25).
    let want = text_bbox(b.fx(1), b.fy(25), b" 514 bytes used in 1 items.");
    let got = ink_bbox(host, R { x: b.fx(1), y: b.fy(24), w: 27 * GL_WCHAR, h: 17 });
    assert_eq!(got, want, "the info text's ink bounding box at cells from (1,25)");
}

// ── (2) the name's centring ───────────────────────────────────────────────

/// An **odd**-width window: `w = 321`, closer + fuller, name `A A`. The name
/// rect is `(55, 42, 271, 19)`; `gr_inside(grown, 1)` is `(54, 41, 273, 21)`,
/// `c.y--` for the activator; `textw = 24`, slack `249`, so
/// `x = 54 + (249 + 1) / 2 = 179` and `y = 40 + (21 - 16 + 1) / 2 = 43`.
/// The old formula `name_x + (271 - 24) / 2 = 178` was one column left.
/// The name word is set to `0x1101` (hollow, transparent text — the
/// untopped default) on both sides so the interior carries no stipple and
/// the ink is the glyphs alone.
#[test]
fn a105_2_odd_width_name_is_centred_with_gr_justs_rounding() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    set_dcolor(&mut pb, 3, 0x1101, 0x1101);
    let b = R { x: 30, y: 40, w: 321, h: 240 };
    let h = open_window(&mut pb, NAME | CLOSER | FULLER | MOVER, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"A A\0");
    let nr = name_rect(b);
    assert_eq!(nr, R { x: 55, y: 42, w: 271, h: 19 });
    let (x, y, n) = name_cells(nr, 3);
    assert_eq!((x, y, n), (179, 43, 3), "gr_just on gr_inside(grown, 1) after the y-1 nudge");
    let inner = grown(nr).inset(2); // inside border and bevel
    assert_eq!(ink_bbox(host, inner), text_bbox(x, y, b"A A"), "the name's ink at cells from (179,43)");
}

/// A name **wider than its cell**: 40 characters (320 px) in the reference-
/// sized window (`w = 320`, name rect `(55, 42, 270, 19)`, cell `(54, 41,
/// 272, 21)`). `gsx_tcalc` clamps `textw` to 272 and drops the characters
/// that do not fit (`numchs = 272 / 8 = 34`, `gemgraf.c:487-495`); with no
/// slack `gr_just` leaves `x` at the cell's origin (`:578-585`): the text
/// starts at frame column 24, left-aligned, and its 34th glyph is the last
/// — nothing is drawn beyond the cell.
#[test]
fn a105_2_overlong_name_is_left_aligned_and_cut_to_whole_glyphs() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    set_dcolor(&mut pb, 3, 0x1101, 0x1101);
    let b = R { x: 30, y: 40, w: 320, h: 240 };
    let h = open_window(&mut pb, NAME | CLOSER | FULLER | MOVER, (b.x, b.y, b.w, b.h));
    let name: &'static [u8] = b"0123456789012345678901234567890123456789\0";
    set_string(&mut pb, h, WF_NAME, name);
    let nr = name_rect(b);
    let (x, y, n) = name_cells(nr, 40);
    assert_eq!((x, y, n), (b.fx(24), 43, 34), "34 whole glyphs from the cell's origin, frame column 24");
    let inner = grown(nr).inset(2);
    assert_eq!(ink_bbox(host, inner), text_bbox(x, y, &name[..34]), "the first 34 glyphs, left-aligned at column 24");
    // The glyph cells end on the cell's last column (frame 295); the grown
    // box's right border (296) is black, the fuller's slot starts at 297.
    assert_eq!(x + 34 * GL_WCHAR - 1, b.fx(295), "the 34th glyph ends on the cell's last column");
    assert_eq!(px(host, b.fx(296), y + 8), BLACK, "the name box's right border");
}

// ── (4) changecol on a pressed gadget ─────────────────────────────────────

/// The topped `KIND_ALL` window at `B`; the closer's grown slot is
/// `(30,40)+23x23`, its glyph cell at `(38,43)` unpressed and `(39,44)`
/// pressed (`c.g_x++` for SELECTED, `gemoblib.c:748-753`).
const B: R = R { x: 30, y: 40, w: 320, h: 240 };
const ON_CLOSER: (i16, i16) = (B.x + 8, B.y + 8);
const G_CLOSE: u8 = 5;

/// Press the closer of a fresh `KIND_ALL` window at `B` and snapshot the
/// screen while `gr_watchbox` holds it SELECTED; return the snapshot and the
/// `evnt_multi` result after the release inside (`WM_CLOSED`).
fn pressed_closer(host: &'static ScriptHost, pb: &mut Aespb) -> (Vec<u32>, i16, [i16; 8]) {
    open_window(pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    drain_messages(pb);
    move_to(host, pb, ON_CLOSER.0, ON_CLOSER.1, 0);
    drain_messages(pb);
    arm_gap(host);
    host.push(pkt(0, 0, 1)); // press on the closer
    // Held past the button-delay window: the application's 2-click wait
    // (`gl_bpend`) makes the press open it, and the Control Manager's wait is
    // completed only when it closes (`b_click`/`b_delay`,
    // `aes/geminput.c:133-140,155-171`), 11 AES ticks = 220 ms later.
    hold_down(host, HOLD_PAST_WINDOW);
    host.push(snap_held(0)); // the screen while gr_watchbox shows it SELECTED
    host.push(pkt(0, 0, 1));
    host.push(pkt(0, 0, 0)); // release inside
    for _ in 0..8 {
        host.push(pkt(0, 0, 0));
    }
    let (fired, _bret, msg) = evnt_multi_timed(pb, 300);
    let snap = host.snapshot(0).expect("the snapshot inside the watchbox loop");
    (snap, fired, msg)
}

/// Assert the pressed closer's pixels in `snap`: border black, the sunk
/// bevel BLACK top-left / `lo` bottom-right, interior `face`, the glyph in
/// `ink` at the moved cell `(39,44)`.
fn assert_pressed_closer(host: &ScriptHost, snap: &[u32], face: Pel, ink: Pel, lo: Pel, what: &str) {
    let slot = grown(R { x: B.x + ADJ3DSTD, y: B.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let r = slot.inset(1);
    let (black, face_d, ink_d, lo_d) = (dev(host, BLACK), dev(host, face), dev(host, ink), dev(host, lo));
    // border
    for x in slot.x..slot.x + slot.w {
        assert_eq!(at(snap, x, slot.y), black, "{what}: border top at ({x},{})", slot.y);
        assert_eq!(at(snap, x, slot.y + slot.h - 1), black, "{what}: border bottom at ({x},{})", slot.y + slot.h - 1);
    }
    // bevel: SELECTED → top-left BLACK, bottom-right `lo` (`add_3d_effect`,
    // `gemoblib.c:252,270-272`)
    for y in r.y..r.y + r.h - 1 {
        assert_eq!(at(snap, r.x, y), black, "{what}: sunk bevel left (BLACK) at ({},{y})", r.x);
    }
    for x in r.x..r.x + r.w - 1 {
        assert_eq!(at(snap, x, r.y), black, "{what}: sunk bevel top (BLACK) at ({x},{})", r.y);
    }
    for x in r.x + 1..r.x + r.w {
        assert_eq!(at(snap, x, r.y + r.h - 1), lo_d, "{what}: sunk bevel bottom at ({x},{})", r.y + r.h - 1);
    }
    for y in r.y + 1..r.y + r.h {
        assert_eq!(at(snap, r.x + r.w - 1, y), lo_d, "{what}: sunk bevel right at ({},{y})", r.x + r.w - 1);
    }
    // interior and glyph: the glyph cell moved to (39,44)
    let cell = R { x: 39, y: 44, w: GL_WCHAR, h: GL_HCHAR };
    let g = tos_api::vdi::font::glyph(G_CLOSE as u16);
    let inner = r.inset(1);
    for y in inner.y..inner.y + inner.h {
        for x in inner.x..inner.x + inner.w {
            let is_ink = cell.contains(x, y) && g[(y - cell.y) as usize] & (0x80 >> (x - cell.x)) != 0;
            let want = if is_ink { ink_d } else { face_d };
            assert_eq!(at(snap, x, y), want, "{what}: interior/glyph at ({x},{y}) (ink {is_ink})");
        }
    }
}

/// **`actcolchange = 0` (the default): the press changes no colour.** The
/// interior stays the hollow white of the `0x1101` word, the glyph stays
/// black and moves one column right, the bevel sinks — BLACK top-left,
/// LWHITE bottom-right (`add_3d_effect`, `gemoblib.c:252,270-272`; the
/// interior colour word is BLACK, so LWHITE is not substituted).
#[test]
fn a105_4_pressed_closer_keeps_its_colours_without_actcolchange() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let (snap, fired, msg) = pressed_closer(host, &mut pb);
    assert_pressed_closer(host, &snap, WHITE, BLACK, LWHITE, "actcolchange 0");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "the release inside delivers WM_CLOSED; fired = {fired:#06x}");
    assert_eq!(msg[0], WM_CLOSED, "msg = {msg:?}");
}

/// **`objc_sysvar(SET, LK3DACT, 1, 1)`: the press changes colour.** With
/// `changecol` the SELECTED gadget's hollow interior goes **solid black**
/// (`gemoblib.c:681-690`, `if (ipat == IP_HOLLOW) { ipat = IP_SOLID; icol =
/// BLACK; }`), the glyph is drawn in `xor_color(BLACK) = WHITE` in
/// `MD_TRANS` (`:699-706`, `vdi16comp`, `:83-86`), and the bevel's lower
/// line is LWHITE, judged against the changed `icol` (`:270-272`, `:950`).
/// NewTOS ignored `actcolchange` for the frame gadgets.
#[test]
fn a105_4_pressed_closer_takes_the_complement_colours_with_actcolchange() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    set_lk3dact(&mut pb, 1, 1);
    let (snap, fired, msg) = pressed_closer(host, &mut pb);
    set_lk3dact(&mut pb, 1, 0);
    assert_pressed_closer(host, &snap, BLACK, WHITE, LWHITE, "actcolchange 1");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "the release inside delivers WM_CLOSED; fired = {fired:#06x}");
    assert_eq!(msg[0], WM_CLOSED, "msg = {msg:?}");
    // Released: the closer is back to its normal colours (hollow white
    // interior, glyph black at the unmoved cell (38,43)).
    let slot = grown(R { x: B.x + ADJ3DSTD, y: B.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_eq!(ink_bbox(host, slot.inset(2)), text_bbox(38, 43, &[G_CLOSE]), "the glyph back at (38,43) in black");
}

// ── A109 (2): the pressed scroll arrow ────────────────────────────────────

/// `WM_ARROWED` (24) and the two `WA_*` sub-actions these cases see
/// (`aesdefs.h:245-253`, `gemctrl.c:63-74`).
const WM_ARROWED: i16 = 24;
const WA_DNPAGE: i16 = 1;
const WA_UPLINE: i16 = 2;

/// `G_UP` — the up-arrow glyph of the system font (`obdefs.h` `G_UP`,
/// `aes/gemwmlib.c:61-81` `gl_watype`).
const G_UP: u8 = 0x01;

/// The vertical scroll bar's box for the `KIND_ALL` window at `B` —
/// `w_bldactive`'s `W_VBAR` (`gemwmlib.c:585-610`): `adj_wbox` wide against
/// the window's right edge, from the first work row (title band + info line)
/// down to the top of the horizontal band.
fn vbar_box(b: R) -> R {
    let top = b.y + 1 + (ADJ_HBOX - 1) + GL_HBOX; // TGADGETS + INFO
    let data_h = b.y + b.h - top;
    let work_h = data_h - 2 - (ADJ_HBOX - 1); // HGADGETS / SIZER take the bottom band
    R { x: b.x + b.w - ADJ_WBOX, y: top, w: ADJ_WBOX, h: work_h + 2 }
}

/// The up-arrow cell — `w_adjust(W_VBAR, W_UPARROW, ADJ3DSTD, ADJ3DSTD,
/// gl_wbox, gl_hbox)` (`gemwmlib.c:412-414`).
fn up_arrow_cell(b: R) -> R {
    let bar = vbar_box(b);
    R { x: bar.x + ADJ3DSTD, y: bar.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX }
}

/// Bounding box of the BLACK pixels of `r` **in a snapshot**.
fn snap_ink_bbox(host: &ScriptHost, snap: &[u32], r: R) -> Option<(i16, i16, i16, i16)> {
    let black = dev(host, BLACK);
    let mut v = Vec::new();
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            if at(snap, x, y) == black {
                v.push((x, y));
            }
        }
    }
    bbox(v.into_iter())
}

/// What a wait loop saw while a frame press was held and after it: every
/// message delivered, with whether the button was still down when the wait
/// returned it.
struct Held {
    before: Vec<u32>,
    held: Vec<u32>,
    msgs: Vec<([i16; 8], bool)>,
}

/// Press `(x, y)` on the frame of the `KIND_ALL` window at `B`, hold it, let
/// it go — and keep calling the application's wait (`evnt_multi` with
/// `MU_MESAG`) the whole time, as an application does, until the script is
/// used up and a wait ends on its timer alone. `before` is the screen just
/// before the press, `held` the screen while the button is still down.
fn press_frame(host: &'static ScriptHost, pb: &mut Aespb, x: i16, y: i16) -> Held {
    move_to(host, pb, x, y, 0);
    drain_messages(pb);
    arm_gap(host);
    host.push(snap(0)); // before the press
    host.push(pkt(0, 0, 1)); // press
    hold_down(host, HOLD_PAST_WINDOW); // see `pressed_closer`
    host.push(snap_held(1)); // while it is held
    hold_down(host, 40);
    host.push(pkt(0, 0, 0)); // release
    for _ in 0..8 {
        host.push(pkt(0, 0, 0));
    }
    let mut msgs = Vec::new();
    for _ in 0..2000 {
        let (fired, _bret, msg) = evnt_multi_timed(pb, 300);
        let down = tos_api::vdi::mouse_buttons() & 1 != 0;
        if fired & MU_MESAG != 0 {
            msgs.push((msg, down));
        } else if host.script.lock().unwrap().is_empty() && !down {
            break;
        }
    }
    let before = host.snapshot(0).expect("the pre-press snapshot");
    let held = host.snapshot(1).expect("the held snapshot");
    Held { before, held, msgs }
}

/// **A130-003: a held scroll gadget sends `WM_ARROWED` for as long as the
/// button is down** (`handle_arrow_msg`, `aes/gemctrl.c:111-139`: `do { send
/// or park; dsptch(); } while (button & 0x0001)`). The first message goes out
/// on the press, not on the release; each later wait of the owner while the
/// button is still down gets the next; after the release there is none.
fn assert_repeats(h: &Held, action: i16) {
    assert!(!h.msgs.is_empty(), "the press must deliver WM_ARROWED");
    for (m, _) in &h.msgs {
        assert_eq!((m[0], m[4]), (WM_ARROWED, action), "only WM_ARROWED/{action}, msg = {m:?}");
        assert_eq!(m[1], 1, "sent by the Screen Manager (pid 1), msg = {m:?}");
    }
    assert!(h.msgs[0].1, "the first WM_ARROWED arrives while the button is still held");
    let while_held = h.msgs.iter().filter(|(_, down)| *down).count();
    assert!(
        while_held >= 10,
        "WM_ARROWED repeats while the button is held: {while_held} of {} messages",
        h.msgs.len()
    );
}

/// **A held scroll arrow is drawn `SELECTED`, and normal again on release.**
/// `hctl_window` brackets `handle_arrow_msg` with `ob_change(gl_awind,
/// gadget, selected, TRUE)` / `(…, normal, TRUE)` (`aes/gemctrl.c:284-292`),
/// `selected = normal | SELECTED` (`:189-194`). With the default
/// `actcolchange = 0` that press shows as `add_3d_effect`'s **sunk** bevel —
/// top/left BLACK instead of WHITE, bottom/right LWHITE instead of LBLACK
/// (`gemoblib.c:238-276`) — and the glyph moved by `acttxtmove`'s `c.g_x++`
/// (`:742-755`), which with the unselected `c.g_y--` puts it one pixel right
/// **and** one down. NewTOS drew no feedback at all (the residual finding of A105).
/// `WM_ARROWED`/`WA_UPLINE` repeats while it is held (A130-003).
#[test]
fn a109_a_held_scroll_arrow_is_drawn_selected_and_restored_on_release() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    open_window(&mut pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    drain_messages(&mut pb);

    let cell = up_arrow_cell(B);
    assert_eq!((cell.x, cell.y, cell.w, cell.h), (329, 84, 19, 19), "the up-arrow cell (gemwmlib.c:412-414)");
    let slot = grown(cell); // (327, 82, 23, 23)
    let bevel = slot.inset(1);
    let probe_y = bevel.y + 8;

    let h = press_frame(host, &mut pb, cell.x + 9, cell.y + 9);
    let (before, held) = (&h.before, &h.held);

    // The bevel: raised before the press, sunk while it is held.
    assert_eq!(at(before, bevel.x, probe_y), dev(host, WHITE), "raised bevel: left line WHITE");
    assert_eq!(
        at(before, bevel.x + bevel.w - 1, probe_y),
        dev(host, LBLACK),
        "raised bevel: right line LBLACK"
    );
    assert_eq!(at(held, bevel.x, probe_y), dev(host, BLACK), "held: the bevel sinks, left line BLACK");
    assert_eq!(
        at(held, bevel.x + bevel.w - 1, probe_y),
        dev(host, LWHITE),
        "held: the bevel sinks, right line LWHITE"
    );

    // The glyph moves one pixel right and one down while it is held.
    let inner = slot.inset(2);
    let normal_cell = (335, 85);
    assert_eq!(
        snap_ink_bbox(host, before, inner),
        text_bbox(normal_cell.0, normal_cell.1, &[G_UP]),
        "the unpressed arrow glyph"
    );
    assert_eq!(
        snap_ink_bbox(host, held, inner),
        text_bbox(normal_cell.0 + 1, normal_cell.1 + 1, &[G_UP]),
        "the held arrow glyph, moved by acttxtmove (gemoblib.c:748-753)"
    );

    // Released: every pixel of the gadget slot is back where it was.
    for y in slot.y..slot.y + slot.h {
        for x in slot.x..slot.x + slot.w {
            assert_eq!(
                host.raster.get_device_pixel(x, y),
                at(before, x, y),
                "after the release the arrow is normal again, pixel ({x},{y})"
            );
        }
    }

    assert_repeats(&h, WA_UPLINE);
    drain(host, &mut pb);
}

/// **A page click in the track gets no feedback.** EmuTOS guards both
/// `ob_change`s with `if ((gadget != W_HSLIDE) && (gadget != W_VSLIDE))`
/// (`aes/gemctrl.c:285,290`): for a click in the track `gadget` — `ob_find`'s
/// answer — *is* the slide box, while `cpt` was moved on to the elevator
/// (`:262-278`), so nothing is redrawn. The message is still sent
/// (`WA_DNPAGE` for a point below the thumb), repeated while the button is
/// held (A130-003).
#[test]
fn a109_a_page_click_in_the_track_draws_no_feedback() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    open_window(&mut pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    drain_messages(&mut pb);

    let bar = vbar_box(B);
    // Below the thumb: with no `WF_VSLSIZ`/`WF_VSLIDE` set the elevator is one
    // gadget cell at the top of the track (`gemwmlib.c:431-434`).
    let (px, py) = (bar.x + ADJ3DSTD + GL_WBOX / 2, bar.y + bar.h - ADJ_HBOX - 20);
    let h = press_frame(host, &mut pb, px, py);

    for y in bar.y..bar.y + bar.h {
        for x in bar.x..bar.x + bar.w {
            assert_eq!(
                at(&h.held, x, y),
                at(&h.before, x, y),
                "a page click must redraw nothing in the bar, pixel ({x},{y})"
            );
        }
    }
    assert_repeats(&h, WA_DNPAGE);
    drain(host, &mut pb);
}

// ── A130: the elevator drag (`gr_slidebox`) and its hit test ──────────────

const WM_HSLID: i16 = 25;
const WM_VSLID: i16 = 26;
const WF_HSLIDE: i16 = 8;
const WF_VSLIDE: i16 = 9;
const WF_HSLSIZ: i16 = 15;
const WF_VSLSIZ: i16 = 16;

/// `wind_set(h, field, v)`.
fn set_slider(pb: &mut Aespb, handle: i16, field: i16, v: i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.intin[2] = v;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, {field}, {v})");
}

/// The window of the A130 probe (`tosdump/probes/slprobe.c`): border
/// `(80, 60, 400, 320)`, kind `0x0FEF` (every gadget but `INFO`) — the
/// geometry the oracle's messages were measured on.
const SLPROBE: R = R { x: 80, y: 60, w: 400, h: 320 };
const KIND_SLPROBE: i16 = 0x0FEF;

/// `W_VBAR` of a window **without** `INFO` and with both bars: as
/// [`vbar_box`], the work area starting right below the title band.
fn vbar_box_noinfo(b: R) -> R {
    let top = b.y + 1 + (ADJ_HBOX - 1);
    let data_h = b.y + b.h - top;
    let work_h = data_h - 2 - (ADJ_HBOX - 1);
    R { x: b.x + b.w - ADJ_WBOX, y: top, w: ADJ_WBOX, h: work_h + 2 }
}

/// The vertical slide box and its un-grown elevator in the bar `bar` —
/// `w_bldvbar` (`gemwmlib.c:427-436`): the slide box is the bar between the
/// two arrows (`adj_hbox` each), `size = max(gl_hbox, mul_div(h, vslsiz,
/// 1000))` (or `gl_hbox` for −1), `posn = mul_div(h - size, vslide, 1000)`,
/// the elevator at `(ADJ3DSTD, posn + ADJ3DSTD)`, `gl_wbox` × `size -
/// 2*ADJ3DSTD - 1`.
fn v_slide_elev_in(bar: R, size_pm: i16, pos_pm: i16) -> (R, R) {
    let slide = R { x: bar.x, y: bar.y + ADJ_HBOX, w: ADJ_WBOX, h: bar.h - 2 * ADJ_HBOX };
    let size = if size_pm == -1 { GL_HBOX } else { GL_HBOX.max((slide.h as i32 * size_pm as i32 / 1000) as i16) };
    let posn = ((slide.h - size) as i32 * pos_pm as i32 / 1000) as i16;
    let e = R { x: slide.x + ADJ3DSTD, y: slide.y + posn + ADJ3DSTD, w: GL_WBOX, h: size - 2 * ADJ3DSTD - 1 };
    (slide, e)
}

/// [`v_slide_elev_in`] for the `KIND_ALL` window at `B`.
fn v_slide_elev(size_pm: i16, pos_pm: i16) -> (R, R) {
    v_slide_elev_in(vbar_box(B), size_pm, pos_pm)
}

/// EmuTOS `mul_div_round` (`util/miscasm.S:151-189`).
fn mul_div_round(a: i32, b: i32, d: i32) -> i32 {
    let n = a * b * 2 / d;
    if n < 0 { (n - 1) >> 1 } else { (n + 1) >> 1 }
}

/// **A130-001/002: an elevator drag is `gr_slidebox`'s.** `hctl_window`
/// draws the pressed elevator `SELECTED`, drags a dotted XOR outline of its
/// 3D-grown rect inside the slide box, draws it normal again and sends
/// `WM_VSLID` with `gr_slidebox`'s value (`aes/gemctrl.c:295-312`,
/// `aes/gemgrlib.c:358-413`). While held:
///
/// * nothing is painted while the pointer stands still (`gr_wait` blocks in
///   `gr_stilldn`, `:169-191`) — NewTOS repainted slide box and elevator on
///   every pass, and the slide box's pattern showed through (GitHub #2);
/// * the elevator itself stays where it was, its bevel sunk;
/// * after a move the outline sits 40 rows lower, the elevator has not moved.
///
/// After the release the screen over the bar is what it was (the test
/// application does not answer with `wind_set`), and the value is
/// `mul_div_round(divnd + ADJ3DSTD, 1000, slide.h - outline.h)`: with size
/// 300 at 500 and a 40-row move, `(80 + 40 + 2) * 1000 / 162` → 753 — the
/// value the pinned oracle sent in the A130 measurement
/// (`measurements/measurement-2026-11-05-a130/analysis/msgtable-v3-1.txt`;
/// NewTOS sent 745).
#[test]
fn a130_elevator_drag_is_gr_slidebox() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let w = SLPROBE;
    let wh = open_window(&mut pb, KIND_SLPROBE, (w.x, w.y, w.w, w.h));
    set_slider(&mut pb, wh, WF_VSLSIZ, 300);
    set_slider(&mut pb, wh, WF_VSLIDE, 500);
    drain_messages(&mut pb);

    let bar = vbar_box_noinfo(w);
    let (slide, e) = v_slide_elev_in(bar, 300, 500);
    assert_eq!((slide.y, slide.h), (106, 229), "the probe's slide box (RUNINFO: track y ~105..334)");
    let g = grown(e);
    let (cx, cy) = (e.x + e.w / 2, e.y + e.h / 2);
    assert_eq!(tos_api::aes::window::gadget_name_at(wh, cx, cy), "velev");

    move_to(host, &mut pb, cx, cy, 0);
    drain_messages(&mut pb);
    arm_gap(host);
    host.push(snap(0));
    host.push(pkt(0, 0, 1)); // press
    hold_down(host, HOLD_PAST_WINDOW);
    host.push(snap_held(1));
    hold_down(host, 60);
    host.push(snap_held(2)); // still standing: nothing painted since (1)
    for _ in 0..4 {
        host.push(pkt(0, 10, 1));
    }
    hold_down(host, 20);
    host.push(snap_held(3)); // 40 rows lower
    host.push(pkt(0, 0, 0)); // release
    for _ in 0..8 {
        host.push(pkt(0, 0, 0));
    }
    let mut got = None;
    for _ in 0..50 {
        let (fired, _b, msg) = evnt_multi_timed(&mut pb, 300);
        if fired & MU_MESAG != 0 && msg[0] == WM_VSLID {
            got = Some(msg);
            break;
        }
    }
    let msg = got.expect("the drag must deliver WM_VSLID");
    let before = host.snapshot(0).unwrap();
    let held1 = host.snapshot(1).unwrap();
    let held2 = host.snapshot(2).unwrap();
    let held3 = host.snapshot(3).unwrap();

    assert_eq!(
        host.paints_at(1),
        host.paints_at(2),
        "nothing may be painted while the held pointer stands still"
    );
    assert_eq!(held1, held2, "the held screen is steady");

    // The elevator is drawn SELECTED (sunk bevel) and stays put.
    let bevel = g.inset(1);
    let probe_y = bevel.y + bevel.h / 2;
    assert_eq!(at(&before, bevel.x, probe_y), dev(host, WHITE), "raised bevel before the press");
    assert_eq!(at(&held1, bevel.x, probe_y), dev(host, BLACK), "the held elevator's bevel sinks");
    assert_eq!(at(&held3, bevel.x, probe_y), dev(host, BLACK), "the elevator has not moved with the pointer");

    // The outline's bottom edge: 40 rows below the grown elevator's last row,
    // a dotted XOR line (every other pixel complemented).
    let row = g.y + g.h - 1 + 40;
    let changed = (g.x..g.x + g.w).filter(|&x| at(&held3, x, row) != at(&before, x, row)).count();
    assert!(changed >= (g.w as usize) / 2 - 1, "the dotted outline 40 rows lower: {changed} pixels on row {row}");

    // Released: the bar is what it was.
    for y in bar.y..bar.y + bar.h {
        for x in bar.x..bar.x + bar.w {
            assert_eq!(host.raster.get_device_pixel(x, y), at(&before, x, y), "after the release, pixel ({x},{y})");
        }
    }

    // `gr_slidebox`'s value for the outline 40 rows lower.
    let (oh, divis) = (g.h, slide.h - g.h);
    let divnd = (g.y - slide.y + 40 + ADJ3DSTD).min(divis);
    assert_eq!((oh, divis, divnd), (67, 162, 122));
    assert_eq!(msg[4] as i32, mul_div_round(divnd as i32, 1000, divis as i32));
    assert_eq!(msg[4], 753, "the oracle's WM_VSLID value");
    drain(host, &mut pb);
}

/// **A130-002: a full-size elevator reports 1000 once moved.** With
/// `WF_VSLSIZ` 1000 the elevator's 3D-grown outline is one row shorter than
/// the slide box (`size - 2*ADJ3DSTD - 1 + 2*ADJ3DSTD`), so it can move one
/// row: `divnd = 1 + ADJ3DSTD` capped at `divis = 1` → 1000
/// (`aes/gemgrlib.c:391-412`). NewTOS sent 0 (no travel); the oracle 1000.
#[test]
fn a130_full_size_elevator_drag_reports_1000() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let wh = open_window(&mut pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    set_slider(&mut pb, wh, WF_VSLSIZ, 1000);
    drain_messages(&mut pb);
    let (_, e) = v_slide_elev(1000, 0);
    let (cx, cy) = (e.x + e.w / 2, e.y + e.h / 2);
    move_to(host, &mut pb, cx, cy, 0);
    drain_messages(&mut pb);
    arm_gap(host);
    host.push(pkt(0, 0, 1));
    hold_down(host, HOLD_PAST_WINDOW);
    for _ in 0..4 {
        host.push(pkt(0, 10, 1));
    }
    host.push(pkt(0, 0, 0));
    for _ in 0..8 {
        host.push(pkt(0, 0, 0));
    }
    let mut got = None;
    for _ in 0..50 {
        let (fired, _b, msg) = evnt_multi_timed(&mut pb, 300);
        if fired & MU_MESAG != 0 && msg[0] == WM_VSLID {
            got = Some(msg);
            break;
        }
    }
    assert_eq!(got.expect("WM_VSLID")[4], 1000);
    drain(host, &mut pb);
}

/// **A130-004: the elevator answers for its 3D-grown rect.** `ob_find` tests
/// an `FL3DACT` object against `ob_actxywh` — `ADJ3DSTD` out on every side
/// (`aes/gemoblib.c:1062-1080`, `:313-335`). So the two rows above and below
/// the elevator rect, and the two columns either side of the horizontal one,
/// are elevator drags, not page clicks; one pixel further is a page click.
#[test]
fn a130_the_elevator_hit_rect_is_the_grown_one() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (_host, mut pb) = fresh();
    let wh = open_window(&mut pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    set_slider(&mut pb, wh, WF_VSLSIZ, 300);
    set_slider(&mut pb, wh, WF_VSLIDE, 500);
    set_slider(&mut pb, wh, WF_HSLSIZ, 1000);
    set_slider(&mut pb, wh, WF_HSLIDE, 0);
    drain_messages(&mut pb);
    let name = |x, y| tos_api::aes::window::gadget_name_at(wh, x, y);

    let (_, e) = v_slide_elev(300, 500);
    let cx = e.x + e.w / 2;
    assert_eq!(name(cx, e.y - ADJ3DSTD), "velev", "top rim");
    assert_eq!(name(cx, e.y - ADJ3DSTD - 1), "vuppage", "above the rim");
    assert_eq!(name(cx, e.y + e.h - 1 + ADJ3DSTD), "velev", "bottom rim");
    assert_eq!(name(cx, e.y + e.h + ADJ3DSTD), "vdnpage", "below the rim");

    // The horizontal bar at size 1000: the slide box between the arrows, the
    // elevator `size - 5` wide at `ADJ3DSTD`; its grown right column is the
    // slide box's last-but-one column (the A130 measurement's x 433 case).
    let hbar_y = B.y + B.h - ADJ_HBOX;
    let slide_x = B.x + ADJ_WBOX;
    let slide_w = (B.w - 2 - (ADJ_WBOX - 1) + 2) - 2 * ADJ_WBOX;
    let ex = slide_x + ADJ3DSTD;
    let ew = slide_w - 2 * ADJ3DSTD - 1;
    let cy = hbar_y + ADJ_HBOX / 2;
    assert_eq!(name(ex + ew - 1 + ADJ3DSTD, cy), "helev", "right rim");
    assert_eq!(name(ex + ew + ADJ3DSTD, cy), "hrtpage", "the slide box's last column");
    assert_eq!(name(slide_x, cy), "helev", "left rim = the slide box's first column");
}
