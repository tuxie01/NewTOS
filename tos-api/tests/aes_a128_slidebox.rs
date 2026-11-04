//! A128 — **`graf_slidebox` (76) is EmuTOS `gr_slidebox`**
//! (`aes/gemgrlib.c:358-413`): the dragged feedback is an XOR outline of the
//! (3D-adjusted) child, moved by `gr_dragbox` (`:243-267`), the object itself
//! is neither moved nor redrawn, and the value carries the 3D correction.
//!
//! Spec: `docs/api/aes/resource-fileselector.md` "A128", part A2 and decision 4.
//!
//! ```c
//! ob_actxywh(tree, parent, &c);  ob_relxywh(tree, obj, &t);       /* :365-366 */
//! if (tree[obj].ob_flags & FL3DOBJ) {                             /* :376 */
//!     t.g_w += 2*ADJ3DSTD;  t.g_h += 2*ADJ3DSTD;
//!     if (!(tree[parent].ob_flags & FL3DOBJ)) {
//!         t.g_x -= ADJ3DSTD;  t.g_y -= ADJ3DSTD;  xy_adjust = TRUE; } }
//! gr_dragbox(t.g_w, t.g_h, t.g_x + c.g_x, t.g_y + c.g_y, &c, &t.g_x, &t.g_y);
//! divnd = t.g_y - c.g_y;  divis = c.g_h - t.g_h;                  /* isvert */
//! if (xy_adjust && divnd) divnd += ADJ3DSTD;  if (divnd > divis) divnd = divis;
//! return divis ? mul_div_round(divnd, 1000, divis) : 0;
//! ```
//!
//! Harness (raster stand-in + scripted host + parameter block) copied from
//! `aes_window_slid_rect.rs` (A93): input advances only in `pump_input`.

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

// ── the host: input advances only in `pump_input`, as on the board ─────────

/// One synthetic mouse packet, in the form the VDI's `mouse_int` receives it.
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    /// VDI convention: bit 0 = left, bit 1 = right.
    buttons: u8,
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
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
            clock: AtomicU64::new(0),
            idle: AtomicUsize::new(0),
        }
    }
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn clear(&self) {
        self.script.lock().unwrap().clear();
        self.idle.store(0, Ordering::Relaxed);
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
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
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

// ── the probe tree ─────────────────────────────────────────────────────────

use tos_types::aes::{Object, FL3DACT, G_BOX, G_IBOX, LASTOB};

/// ROOT (an IBOX at (200, 100)), the track (a non-3D G_BOX, rel (10, 10),
/// 24×150) and the thumb (an `FL3DACT` G_BOX, rel (2, 20), 20×30).
const ROOT_XY: (i16, i16) = (200, 100);
const TRACK: (i16, i16, i16, i16) = (10, 10, 24, 150);
const THUMB: (i16, i16, i16, i16) = (2, 20, 20, 30);

fn ob(next: i16, head: i16, tail: i16, ty: u16, flags: u16, spec: u64, r: (i16, i16, i16, i16)) -> Object {
    Object {
        ob_next: next,
        ob_head: head,
        ob_tail: tail,
        ob_type: ty,
        ob_flags: flags,
        ob_state: 0,
        ob_spec: spec,
        ob_x: r.0,
        ob_y: r.1,
        ob_width: r.2,
        ob_height: r.3,
    }
}

fn probe_tree(thumb_flags: u16) -> Box<[Object; 3]> {
    Box::new([
        ob(-1, 1, 1, G_IBOX, 0, 0, (ROOT_XY.0, ROOT_XY.1, 100, 200)),
        ob(0, 2, 2, G_BOX, 0, 0x0000_1141, TRACK),
        ob(1, -1, -1, G_BOX, thumb_flags | LASTOB, 0x00FF_1101, THUMB),
    ])
}

/// Absolute top-left of the track (= `ob_actxywh` of the non-3D parent).
fn track_abs() -> (i16, i16) {
    (ROOT_XY.0 + TRACK.0, ROOT_XY.1 + TRACK.1)
}

/// `graf_slidebox(tree, 1, 2, isvert = 1)`.
fn slidebox(pb: &mut Aespb, tree: &mut [Object; 3]) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.intin[1] = 2;
    pb.intin[2] = 1;
    pb.addrin[0] = tree.as_mut_ptr() as u64;
    pb.call(76, 3, 1, 1);
    pb.addrin[0] = 0;
    pb.intout[0]
}

fn mul_div_round(a: i32, b: i32, c: i32) -> i32 {
    let n = a * b * 2 / c;
    if n < 0 { (n - 1) >> 1 } else { (n + 1) >> 1 }
}

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

/// `graf_mkstate` — one AES call, hence one `dsptch()` → one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) {
    pb.call(79, 0, 5, 0);
}

fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..64 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        graf_mkstate(pb);
    }
}

fn move_to(host: &'static ScriptHost, pb: &mut Aespb, tx: i16, ty: i16, buttons: u8) {
    drain(host, pb);
    for _ in 0..200 {
        let (cx, cy) = tos_api::vdi::mouse_position();
        if cx == tx && cy == ty {
            break;
        }
        let dx = (tx - cx).clamp(-32, 32);
        let dy = (ty - cy).clamp(-32, 32);
        host.push(Packet { dx, dy, buttons });
        graf_mkstate(pb);
    }
    assert_eq!(tos_api::vdi::mouse_position(), (tx, ty), "move_to did not reach its target");
}

/// Fresh application, pointer hidden (`graf_mouse(M_OFF)`, so the sprite is
/// not part of the framebuffer comparison) and parked on the thumb with the
/// button up.
fn fresh(on: (i16, i16)) -> (&'static ScriptHost, Aespb) {
    let host = rig();
    host.clear();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.intin[0] = 256; // graf_mouse(M_OFF)
    pb.call(78, 1, 1, 0);
    move_to(host, &mut pb, on.0, on.1, 0);
    (host, pb)
}

/// The press, consumed at once so the button is down when the call starts.
fn press_here(host: &'static ScriptHost, pb: &mut Aespb) {
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    graf_mkstate(pb);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the press must be live");
}

fn snapshot(host: &'static ScriptHost) -> Vec<u32> {
    host.raster.px.lock().unwrap().clone()
}

fn show_pointer(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);
}

/// **Drag.** Press on the 3D thumb, move 30 px down with the button held,
/// release: the value is EmuTOS's for the outline's final place — outline
/// = thumb grown by 2 and moved up/left by 2, `divnd + ADJ3DSTD`, `divis =
/// track_h - outline_h` — the thumb's `ob_y` is unchanged, and the screen
/// is pixel-identical to before the call (the XOR outline is complemented
/// away; nothing was drawn).
#[test]
fn drag_returns_emutos_value_and_leaves_object_and_screen_untouched() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (tx, ty) = track_abs();
    let press = (tx + THUMB.0 + 10, ty + THUMB.1 + 15);
    let (host, mut pb) = fresh(press);
    let mut tree = probe_tree(FL3DACT);
    let before = snapshot(host);

    // Press here (made live first), a still hold for whichever pump comes
    // first, then hold-and-move packets and the release.
    press_here(host, &mut pb);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..6 {
        host.push(Packet { dx: 0, dy: 5, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let v = slidebox(&mut pb, &mut tree);
    let (_, my) = tos_api::vdi::mouse_position();
    assert_eq!(my - press.1, 30, "the drag consumed every move");
    assert!(host.script.lock().unwrap().is_empty(), "the drag ran to the release");

    // outline start (rel 0, 18), 24×34; grip kept → final rel y 18 + 30 = 48.
    let rel = 18 + (my - press.1);
    let divis = TRACK.3 - (THUMB.3 + 4);
    let want = mul_div_round((rel + 2).min(divis) as i32, 1000, divis as i32) as i16;
    assert_eq!(want, 431);
    assert_eq!(v, want, "graf_slidebox value");
    assert_eq!(tree[2].ob_y, THUMB.1, "the object is not moved (EmuTOS drags an outline)");
    assert!(snapshot(host) == before, "the screen is unchanged after the drag (XOR outline erased, nothing drawn)");
    show_pointer(&mut pb);
}

/// **Drag past the end.** The outline is held inside the track
/// (`rc_constrain`): at the bottom `divnd + 2 > divis` is capped → 1000; at
/// the top `divnd = 0` gets no correction → 0.
#[test]
fn drag_is_constrained_to_the_parent_and_saturates() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (tx, ty) = track_abs();
    let press = (tx + THUMB.0 + 10, ty + THUMB.1 + 15);
    for (dy, want) in [(25i16, 1000i16), (-25, 0)] {
        let (host, mut pb) = fresh(press);
        let mut tree = probe_tree(FL3DACT);
        press_here(host, &mut pb);
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
        for _ in 0..8 {
            host.push(Packet { dx: 0, dy, buttons: 1 });
        }
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
        assert_eq!(slidebox(&mut pb, &mut tree), want, "dy {dy}");
        assert!(host.script.lock().unwrap().is_empty(), "the drag ran to the release");
        show_pointer(&mut pb);
    }
}

/// **No motion.** Called with the button already up, `gr_dragbox` returns at
/// once (`gr_stilldn` → `abutton`'s `downorup` holds, `geminput.c:549`) and
/// the value is the start position's: 3D thumb → `(18 + 2) * 1000 / 116`
/// = 172; a non-3D thumb → `20 * 1000 / 120` = 167 (no adjustment).
#[test]
fn button_up_returns_the_start_value_with_and_without_3d() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (tx, ty) = track_abs();
    let on = (tx + THUMB.0 + 10, ty + THUMB.1 + 15);
    for (flags, want) in [(FL3DACT, 172i16), (0, 167)] {
        let (_host, mut pb) = fresh(on);
        let mut tree = probe_tree(flags);
        assert_eq!(slidebox(&mut pb, &mut tree), want, "flags {flags:#06x}");
        show_pointer(&mut pb);
    }
}

/// A null tree or a negative index is refused with 0 before anything runs.
#[test]
fn refused_call_returns_zero() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (_host, mut pb) = fresh((50, 400));
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.intin[1] = 2;
    pb.intin[2] = 1;
    pb.addrin[0] = 0;
    pb.call(76, 3, 1, 1);
    assert_eq!(pb.intout[0], 0);
    show_pointer(&mut pb);
}
