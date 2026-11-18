//! A57 / A71 — the alert and the drop-down save the screen under them into
//! **the** AES screen buffer, device-exact, and put it back byte for byte.
//!
//! EmuTOS keeps one buffer (`gl_tmp`, `aes/gemgsxif.c:80`) that `fm_alert`
//! (`gemfmalt.c:363,390`) and `menu_sr` (`gemmnlib.c:208-223`) both go
//! through — `bb_save`/`bb_restore`, a `vro_cpyfm(S_ONLY)` Screen↔Memory
//! copy of the device pixels (`bb_set`, `gemgsxif.c:320-380`). Nothing on
//! the way is a pen: what is under the box comes back exactly, whatever
//! colour it was.
//!
//! NewTOS before this change:
//!
//! * the alert already copied device pixels, but into a fresh heap `Vec` per
//!   alert (`form.rs`, `BbSave`) — nothing a program could be handed as
//!   `WF_SCREEN`;
//! * the drop-down saved through `read_pixel` — the **pen** under each pixel,
//!   the palette lookup that answers pen 0 for any colour that is no pen —
//!   and restored with `fill_rect(pen)` (`menu.rs`, `save_under` /
//!   `restore_under`). A truecolor image under a menu (zView, QED) came back
//!   as white. That is A71.
//!
//! # What is measured
//!
//! A screen carrying a **truecolor pattern** — every 16-bit RGB565 word in
//! turn, expanded to the device the way the backend expands it, so each
//! pixel round-trips the wireformat exactly and almost none of them is a
//! pen. Over it:
//!
//! * `form_alert` through the AES trap, opened and closed with Enter: the
//!   screen after equals the screen before, and the pixels went through the
//!   screen buffer (the fixture is zeroed first and is not zero afterwards);
//! * a drop-down (`menu::open` / `menu::close`, the tracker's own open and
//!   cancel paths without the mouse): same two checks.
//!
//! The nested submenu and the pop-up take a **transient pool block** per
//! opening (EmuTOS `popup_blit`, `gemmnext.c:136-215`) and cannot run in a
//! host process, which has no BDOS pool; they are target-only.
//!
//! # The profile
//!
//! 640×480 Falcon truecolor (16 planes, 256 pens) with the pseudo-palette
//! `init_wk` seeds under truecolor, as in `aes_form_alert_cursor_bracket.rs`
//! and `aes_menu_click_ownership.rs`; the screen buffer is the fixture
//! `gsxif::install_screen_buffer` provides, sized by `gsx_mcalc` for this
//! screen (614 400 bytes).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::aes::menu;
use tos_api::aes::object::Object;
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
    /// Mode-aware like `abline` above (A99): the menu title's highlight is
    /// `ob_change`'s XOR fill each way (`gemoblib.c:1295-1304`), so an XOR
    /// pattern must complement the destination — a mode-blind opaque paint
    /// would leave the un-highlighted title black.
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

// ── the host ──────────────────────────────────────────────────────────────

/// A host whose console hands out queued key bytes — how the modal alert is
/// closed headlessly (`form.rs` `modal_loop`: Enter fires the DEFAULT button).
/// The first key read also snapshots the screen, the one chance to see the
/// alert while it is up.
struct TestHost {
    raster: BufBackend,
    clock: AtomicU64,
    keys: Mutex<VecDeque<u8>>,
    while_open: Mutex<Option<Vec<u32>>>,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read(&self) -> Option<u8> {
        let mut open = self.while_open.lock().unwrap();
        if open.is_none() {
            *open = Some(self.raster.snapshot());
        }
        self.keys.lock().unwrap().pop_front()
    }
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

/// The AES keeps process-global state (the cursor, the menu hold, the one
/// screen buffer), so the cases in this binary must not overlap.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

/// The fixture screen buffer — `gsx_malloc`'s block on the target. Sized by
/// `gsx_mcalc` for this screen and leaked; read back by the cases to see
/// that the pixels really went through it.
static SCREEN_BUFFER: OnceLock<(usize, usize)> = OnceLock::new(); // (ptr, len)

/// A page-aligned TT-RAM-looking "physical address" for the fixture.
const FIXTURE_PA: u64 = 0x0120_0000;

fn rig() -> &'static TestHost {
    RIG.get_or_init(|| {
        let host: &'static TestHost = Box::leak(Box::new(TestHost {
            raster: BufBackend::new(),
            clock: AtomicU64::new(0),
            keys: Mutex::new(VecDeque::new()),
            while_open: Mutex::new(None),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        seed_palette(&host.raster);

        let len = tos_api::aes::gsxif::gsx_mcalc(&host.raster);
        assert_eq!(len, 614_400, "gsx_mcalc at 640x480: one screen in device pixels");
        let buf: &'static mut [u8] = Box::leak(vec![0u8; len].into_boxed_slice());
        SCREEN_BUFFER.set((buf.as_mut_ptr() as usize, len)).unwrap();
        // SAFETY: the leaked slice lives for the rest of the process and
        // holds exactly `gsx_mcalc` bytes.
        unsafe { tos_api::aes::gsxif::install_screen_buffer(buf.as_mut_ptr(), FIXTURE_PA, len) };
        host
    })
}

/// The fixture's bytes.
fn screen_buffer() -> &'static mut [u8] {
    let (ptr, len) = *SCREEN_BUFFER.get().expect("rig() first");
    // SAFETY: the leaked fixture; `SERIAL` serialises the cases.
    unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, len) }
}

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`) as the RGB555-in-565
/// words the pseudo-palette holds — pen 0 white, pen 1 black, the sixteen the
/// kernel backend seeds too.
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

// ── the pattern ──────────────────────────────────────────────────────────

/// Paint every 16-bit RGB565 word in turn — all 32 red levels, 64 green,
/// 32 blue, mixed — expanded to the device exactly as the backend expands
/// it, so each pixel round-trips `bb_save`'s wireformat and only a lossy
/// path (a pen lookup) can change it. Rows below `from_y` only.
fn paint_pattern(b: &BufBackend, from_y: i16) {
    let mut px = b.px.lock().unwrap();
    for y in from_y..SCREEN_H as i16 {
        for x in 0..SCREEN_W as i16 {
            let word = ((y as u32 * 977 + x as u32 * 131) & 0xFFFF) as u16;
            px[BufBackend::idx(x, y)] = b.rgb565_to_device(word);
        }
    }
}

/// How many pattern pixels are no pen at all: the pen lookup that A71's
/// `read_pixel` path did would have answered pen 0 for each of them.
fn non_pen_pixels(b: &BufBackend, from_y: i16) -> usize {
    let pal = *b.palette.lock().unwrap();
    let px = b.px.lock().unwrap();
    let mut n = 0;
    for y in from_y..SCREEN_H as i16 {
        for x in 0..SCREEN_W as i16 {
            let w = b.device_to_rgb565(px[BufBackend::idx(x, y)]);
            if !pal.contains(&w) {
                n += 1;
            }
        }
    }
    n
}

/// A known cursor state: form installed, sprite off the screen.
fn reset_cursor(b: &BufBackend) {
    mouse::show_reset(b);
    mouse::remove(b);
}

fn first_difference(before: &[u32], after: &[u32]) -> Option<(usize, usize, u32, u32)> {
    before.iter().zip(after).enumerate().find(|(_, (a, b))| a != b).map(|(i, (a, b))| {
        (i % SCREEN_W as usize, i / SCREEN_W as usize, *a, *b)
    })
}

// ── the alert ─────────────────────────────────────────────────────────────

/// `form_alert` (52): `int_in[0]` = default button, `addr_in[0]` = the string.
fn form_alert(pb: &mut Aespb, s: &[u8], defbut: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = defbut;
    pb.addrin.fill(0);
    pb.addrin[0] = s.as_ptr() as u64;
    pb.call(52, 1, 1, 1);
    pb.intout[0]
}

/// `fm_alert`: `bb_save` into the screen buffer before the draw, `bb_restore`
/// out of it after (`gemfmalt.c:363,390`). The screen under the alert comes
/// back byte for byte, and the copy went through **the** buffer.
#[test]
fn form_alert_over_a_truecolor_pattern_restores_it_byte_for_byte_through_the_screen_buffer() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    reset_cursor(b);
    paint_pattern(b, 0);
    assert!(non_pen_pixels(b, 0) > 250_000, "the pattern must be truecolor, not pens");
    let before = b.snapshot();
    screen_buffer().fill(0);
    let clipped_before = tos_api::aes::gsxif::clipped_saves();

    *h.while_open.lock().unwrap() = None;
    h.keys.lock().unwrap().push_back(b'\r');
    let chosen = form_alert(&mut pb, b"[1][Hello, truecolor world][OK]\0", 1);
    assert_eq!(chosen, 1, "Enter must fire the default button");

    // Precondition: the alert really drew over the pattern while it was up.
    let open = h.while_open.lock().unwrap().take().expect("the modal loop never asked for a key");
    assert!(first_difference(&before, &open).is_some(), "the alert did not paint anything");

    let after = b.snapshot();
    if let Some((x, y, was, is)) = first_difference(&before, &after) {
        panic!(
            "the screen after the alert differs from the screen before it, first at ({x},{y}): \
             was 0x{was:06X}, is 0x{is:06X} -- bb_restore did not reproduce the pattern"
        );
    }
    assert!(
        screen_buffer().iter().any(|&byte| byte != 0),
        "the alert's save-under did not go through the AES screen buffer (gl_tmp): the \
         fixture is still all zero -- a private buffer is not what WF_SCREEN hands out"
    );
    assert_eq!(
        tos_api::aes::gsxif::clipped_saves(),
        clipped_before,
        "nothing may need clipping at the screen-sized length"
    );
}

// ── the drop-down ─────────────────────────────────────────────────────────

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
const NIL: i16 = -1;

const TITLE_FILE: i16 = 4;
const FILE_BOX: i16 = 10;

const BAR_H: i16 = 19;
const FILE_X: i16 = 56;
const ITEM_H: i16 = 19;

/// The desktop's menu shape (Desk/File/Edit with a drop-down each), the tree
/// `aes_menu_click_ownership.rs` and the objc selftest use.
fn menu_tree() -> *mut Obj {
    static DESK: &[u8] = b"Desk\0";
    static FILE: &[u8] = b"File\0";
    static EDIT: &[u8] = b"Edit\0";
    static ABOUT: &[u8] = b"About\0";
    static INFO: &[u8] = b"Info\0";
    static NEW: &[u8] = b"New\0";
    static OPEN: &[u8] = b"Open\0";
    static QUIT: &[u8] = b"Quit\0";
    static CUT: &[u8] = b"Cut\0";
    static COPY: &[u8] = b"Copy\0";

    let t = |ty, fl, spec: u64, x, y, w, h, next, head, tail| Obj {
        next, head, tail, otype: ty, flags: fl, state: 0, spec, x, y, w, h,
    };
    let s = |p: &[u8], y, next, last| {
        t(G_STRING, if last { LASTOB } else { 0 }, p.as_ptr() as u64, 0, y, 112, ITEM_H, next, -1, -1)
    };
    let tree: Box<[Obj; 17]> = Box::new([
        /* 0 ROOT      */ t(G_IBOX, 0, 0, 0, 0, 640, 480, -1, 1, 6),
        /* 1 THEBAR    */ t(G_BOX, 0, 0, 0, 0, 640, BAR_H, 6, 2, 2),
        /* 2 THEACTIVE */ t(G_IBOX, 0, 0, 0, 0, 640, BAR_H, 1, 3, 5),
        /* 3 Desk      */ t(G_TITLE, 0, DESK.as_ptr() as u64, 0, 0, 56, BAR_H, 4, -1, -1),
        /* 4 File      */ t(G_TITLE, 0, FILE.as_ptr() as u64, FILE_X, 0, 56, BAR_H, 5, -1, -1),
        /* 5 Edit      */ t(G_TITLE, 0, EDIT.as_ptr() as u64, 112, 0, 56, BAR_H, 2, -1, -1),
        /* 6 themenus  */ t(G_IBOX, 0, 0, 0, 0, 640, 480, 0, 7, 14),
        /* 7 Desk-box  */ t(G_BOX, 0, 1 << 16, 0, BAR_H, 112, 2 * ITEM_H, 10, 8, 9),
        /* 8  About    */ s(ABOUT, 0, 9, false),
        /* 9  Info     */ s(INFO, ITEM_H, 7, false),
        /*10 File-box  */ t(G_BOX, 0, 1 << 16, FILE_X, BAR_H, 112, 3 * ITEM_H, 14, 11, 13),
        /*11  New      */ s(NEW, 0, 12, false),
        /*12  Open     */ s(OPEN, ITEM_H, 13, false),
        /*13  Quit     */ s(QUIT, 2 * ITEM_H, 10, false),
        /*14 Edit-box  */ t(G_BOX, 0, 1 << 16, 112, BAR_H, 112, 2 * ITEM_H, 6, 15, 16),
        /*15  Cut      */ s(CUT, 0, 16, false),
        /*16  Copy     */ s(COPY, ITEM_H, 14, true),
    ]);
    Box::leak(tree).as_mut_ptr()
}

/// `menu_sr`: `bb_save(&t)` into the screen buffer when the box drops
/// (`gemmnlib.c:219,239`), `bb_restore(&t)` when it goes up (`:221`). The
/// pattern under the *File* box comes back byte for byte — before A71 the
/// pen path brought it back white — and the copy went through **the**
/// buffer.
#[test]
fn drop_down_over_a_truecolor_pattern_restores_it_byte_for_byte_through_the_screen_buffer() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    reset_cursor(b);
    let tree = menu_tree();
    pb.intin.fill(0);
    pb.intin[0] = 1; // show
    pb.addrin[0] = tree as u64;
    pb.call(30, 1, 1, 1); // menu_bar
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_bar must install the tree");

    // The pattern under the bar, where the drop-down will land.
    paint_pattern(b, BAR_H);
    assert!(non_pen_pixels(b, BAR_H) > 250_000, "the pattern must be truecolor, not pens");
    let before = b.snapshot();
    screen_buffer().fill(0);
    let clipped_before = tos_api::aes::gsxif::clipped_saves();

    let imenu = menu::open(tree as *mut Object, TITLE_FILE, NIL, b);
    assert_eq!(imenu, FILE_BOX, "menu_sub walks to the File box");
    // Precondition: the box really drew over the pattern.
    let open = b.snapshot();
    let inside = |px: &[u32]| px[BufBackend::idx(FILE_X + 8, BAR_H + 8)];
    assert_ne!(inside(&before), inside(&open), "the drop-down did not paint over the pattern");

    menu::close(tree as *mut Object, TITLE_FILE, imenu, b);

    let after = b.snapshot();
    if let Some((x, y, was, is)) = first_difference(&before, &after) {
        panic!(
            "the screen after the drop-down differs from the screen before it, first at ({x},{y}): \
             was 0x{was:06X}, is 0x{is:06X} -- the restore did not reproduce the pattern \
             (A71: a pen-based save-under answers pen 0 for a colour that is no pen)"
        );
    }
    assert!(
        screen_buffer().iter().any(|&byte| byte != 0),
        "the drop-down's save-under did not go through the AES screen buffer (gl_tmp): the \
         fixture is still all zero"
    );
    assert_eq!(
        tos_api::aes::gsxif::clipped_saves(),
        clipped_before,
        "nothing may need clipping at the screen-sized length"
    );
}

// ── the guard: a rectangle the buffer cannot hold ────────────────────────

/// Reinstalls the full fixture when dropped, so a failing guard case cannot
/// leave the other cases of this binary with a toy buffer.
struct RestoreFixture;
impl Drop for RestoreFixture {
    fn drop(&mut self) {
        let (ptr, len) = *SCREEN_BUFFER.get().expect("rig() first");
        // SAFETY: the leaked full-size fixture.
        unsafe { tos_api::aes::gsxif::install_screen_buffer(ptr as *mut u8, FIXTURE_PA, len) };
    }
}

/// `bb_set`'s "buffer too small" arm (`gemgsxif.c:334-343`): the height is
/// clipped to `gl_mlen * sh / size` and nothing is written past the buffer.
/// Unreachable at the screen-sized length — forced here with a toy buffer of
/// ten rows — and counted, since `tos-api` has no `KINFO`.
#[test]
fn bb_save_clips_the_height_to_the_buffer_and_writes_nothing_past_it() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let _restore = RestoreFixture;

    reset_cursor(b);
    paint_pattern(b, 0);

    // A buffer for a 64 x 10 rectangle, followed by a page of sentinel.
    const W: i16 = 64;
    const ROWS: i16 = 10;
    const ASK: i16 = 100;
    let len = tos_api::vdi::bb_save_words(b, W, ROWS) * 4;
    let mut toy = vec![0xA5u8; len + 4096];
    // SAFETY: `toy` outlives the case; `_restore` puts the real fixture back.
    unsafe { tos_api::aes::gsxif::install_screen_buffer(toy.as_mut_ptr(), FIXTURE_PA, len) };
    let clipped_before = tos_api::aes::gsxif::clipped_saves();

    let saved = tos_api::aes::gsxif::bb_save(b, 16, 40, W, ASK).expect("a clipped save still saves");
    let (x, y, w, hh) = saved.rect();
    assert_eq!((x, y, w), (16, 40, W));
    let need = tos_api::vdi::bb_save_words(b, W, ASK) * 4;
    assert_eq!(
        hh as usize,
        len * ASK as usize / need,
        "the saved height is gl_mlen * sh / size (gemgsxif.c:336)"
    );
    assert_eq!(hh, ROWS, "ten rows fit");
    assert_eq!(
        tos_api::aes::gsxif::clipped_saves(),
        clipped_before + 1,
        "the clip is counted (EmuTOS' KINFO)"
    );
    assert!(
        toy[len..].iter().all(|&s| s == 0xA5),
        "bb_save wrote past the buffer's length"
    );
    assert!(toy[..len].iter().any(|&s| s != 0xA5), "bb_save wrote nothing into the buffer");

    // The restore puts back exactly the clipped rows; the rest of the asked
    // rectangle is untouched (EmuTOS: "this will leave droppings").
    let before = b.snapshot();
    b.fill_rect(16, 40, W, ASK, 1, ClipRect::full(SCREEN_W, SCREEN_H));
    saved.restore(b);
    let after = b.snapshot();
    for row in 0..ASK {
        for col in 0..W {
            let (px, py) = (16 + col, 40 + row);
            let i = BufBackend::idx(px, py);
            if row < ROWS {
                assert_eq!(after[i], before[i], "row {row} inside the clipped height must be restored");
            } else {
                assert_ne!(after[i], before[i], "row {row} past the clipped height is not restored");
            }
        }
    }
    assert!(toy[len..].iter().all(|&s| s == 0xA5), "restore read nothing it should not have written");
}
