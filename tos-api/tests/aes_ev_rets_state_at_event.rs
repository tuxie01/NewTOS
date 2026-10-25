//! LDG-M3 B1 + O1 — the button state an event *reports*, and the `objc_edit`
//! caret's justification.
//!
//! **B1 — `ev_rets` (`aes/gemevlib.c:42-58`).** Every `evnt_button` /
//! `evnt_multi` / `evnt_mouse` return fills `EV_MX/EV_MY/EV_MB` from the state
//! recorded before the last published button transition (`pr_xrat`/`pr_yrat`/
//! `pr_button`) when more than one transition was published since the previous
//! event return (`mtrans > 1`), else from the live state, and clears the count.
//! `bchange` maintains the record (`aes/geminput.c:329-345`). A waiter that asks
//! for more than one click opens the button-delay window on the press
//! (`b_click`, `:133-140`); a release inside it is published together with the
//! press when the window closes (`b_delay`, `:155-171`), so the click is
//! reported with the button **down** — what WinDom tests before it handles a
//! click (`evnt.mbut & 1`, `sdk/windom/overlay/src/evnt_windom.c:334`).
//! `ev_multi`'s quick check also tries the recorded state first
//! (`(mtrans > 1) && downorup(pr_button, buparm)`, `aes/gemevlib.c:190-194`),
//! so a click made between two event calls is still delivered.
//!
//! **O1 — `pxl_rect` (`aes/gemobed.c:180-190`).** The caret column starts at
//! the field origin justified by `gr_just(te_just, te_font, te_ptmplt, w, h)`
//! (`aes/gemgraf.c:567-589`), plus `ch_pos * gl_wchar`.
//!
//! Spec: `docs/api/aes/event.md` "The button state an event reports";
//! `docs/api/aes/object.md` `objc_edit`. Harness copied from
//! `aes_menu_click_ownership.rs`.

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

const G_FTEXT: u16 = 29;
const EDITABLE: u16 = 0x0008;
const LASTOB: u16 = 0x0020;

const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const MU_KEYBD: i16 = 0x0001;

/// The default double-click index 3 = 275 ms (`gl_dcrates`,
/// `aes/gemevlib.c:33`); every pump advances the clock by 1 ms, so this many
/// empty pumps close the window with room to spare.
const PAST_WINDOW: usize = 400;

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

/// `graf_mkstate` — one AES call, one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) -> (i16, i16, i16) {
    pb.call(79, 0, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Consume every queued packet, one AES call each.
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..256 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        graf_mkstate(pb);
    }
}

/// The application is busy for `n` ms: `n` empty packets, one
/// `graf_mkstate` each.
fn idle(host: &'static ScriptHost, pb: &mut Aespb, n: usize) {
    for _ in 0..n {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    drain(host, pb);
}

/// Walk the pointer to `(tx, ty)`, buttons up.
fn move_to(host: &'static ScriptHost, pb: &mut Aespb, tx: i16, ty: i16) {
    drain(host, pb);
    for _ in 0..200 {
        let (cx, cy) = tos_api::vdi::mouse_position();
        if (cx, cy) == (tx, ty) {
            break;
        }
        host.push(Packet { dx: (tx - cx).clamp(-32, 32), dy: (ty - cy).clamp(-32, 32), buttons: 0 });
        graf_mkstate(pb);
    }
    assert_eq!(tos_api::vdi::mouse_position(), (tx, ty), "move_to did not reach its target");
}

/// A fresh application on an empty desktop (no menu bar, no window), pointer
/// parked at (320, 300), buttons up, every pending transition consumed by one
/// event return.
fn fresh() -> (&'static ScriptHost, Aespb) {
    let host = rig();
    host.clear();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    move_to(host, &mut pb, 320, 300);
    // One 0-ms poll: its `ev_rets` clears whatever the set-up published.
    evnt_multi(&mut pb, MU_TIMER, 0, 0, 0);
    (host, pb)
}

/// `evnt_multi(flags, clicks, mask, state)` with a 0-ms timer (only used when
/// `flags` has `MU_TIMER`). Returns `(fired, EV_MX, EV_MY, EV_MB, EV_BRET)`.
fn evnt_multi(pb: &mut Aespb, flags: i16, clicks: i16, mask: i16, state: i16) -> (i16, i16, i16, i16, i16) {
    evnt_multi_t(pb, flags, clicks, mask, state, 0)
}

/// [`evnt_multi`] with a `ms`-millisecond `MU_TIMER` (`int_in[14]`/`[15]`).
fn evnt_multi_t(pb: &mut Aespb, flags: i16, clicks: i16, mask: i16, state: i16, ms: u16) -> (i16, i16, i16, i16, i16) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = flags;
    pb.intin[1] = clicks;
    pb.intin[2] = mask;
    pb.intin[3] = state;
    pb.intin[14] = ms as i16;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[6])
}

/// Queue a short click at the current position: one empty packet, press,
/// `hold` packets held, release. The script then runs dry, so the window can
/// close on the idle pumps that follow.
///
/// The leading empty packet makes the click arrive **while the application is
/// waiting**: `evnt_multi`'s entry pump (`chkkbd(); forker();`,
/// `aes/gemevlib.c:169-170`) consumes it, the wait is armed (`abutton`,
/// raising `gl_bpend` for a multi-click request, `aes/geminput.c:559-561`),
/// and only then does the press reach `b_click` — which opens the delay
/// window because a waiter wants more than one click (`:133-140`). A press
/// that reaches the AES *before* any multi-click wait was ever armed is
/// published at once (`:141-142`) — a different case, not the one these
/// tests are about.
fn queue_click(host: &'static ScriptHost, hold: usize) {
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..hold {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
}

/// **B1, the zView M3 case.** WinDom's default wait (`bclick = 258`: bflag 1 +
/// up to two clicks, `sdk/windom/windom-1.21.3/demo/main.c:198`) — the same
/// shape as TeraDesk's `0x0102, 3, 0`. A short click whose release falls
/// inside the double-click window must come back as one click **with
/// `EV_MB` = left down**, at the click's position.
#[test]
fn short_click_in_window_reports_button_down() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, 200, 150);
    queue_click(host, 3);
    let (fired, mx, my, mb, bret) = evnt_multi(&mut pb, MU_MESAG | MU_BUTTON | MU_KEYBD, 0x0102, 3, 0);
    eprintln!("short click: fired={fired:#06x} mx={mx} my={my} mb={mb:#x} bret={bret}");
    assert_eq!(fired & MU_BUTTON, MU_BUTTON, "the click must fire MU_BUTTON");
    assert_eq!(bret, 1, "one click inside the window");
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 0, "the release has happened");
    assert_eq!(
        mb & 1,
        1,
        "EV_MB must be the state at the event — pr_button, the press — because the \
         window published the press and the release together (mtrans = 2; \
         aes/gemevlib.c:44-48, aes/geminput.c:155-171, :337-342)"
    );
    assert_eq!((mx, my), (200, 150), "EV_MX/EV_MY = pr_xrat/pr_yrat, the click position");

    // `ev_rets` cleared the count: the next poll reports the live state, and
    // no second MU_BUTTON comes from the recorded one.
    let (fired2, _, _, mb2, _) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 0x0102, 3, 0);
    assert_eq!(fired2, MU_TIMER, "the click was consumed by the first event; fired = {fired2:#06x}");
    assert_eq!(mb2 & 1, 0, "the next event reports the live, released button");
}

/// The desktop's own wait (`evnt_multi(…, 0x02, 0x01, 0x01, …)`,
/// `desk/deskmain.c:2088`): a single click in the window reports `EV_MB` = 1
/// and one click; a double click reports two, also with the button down.
#[test]
fn desk_wait_single_and_double_click_report_button_down() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    queue_click(host, 2);
    let (fired, _, _, mb, bret) = evnt_multi(&mut pb, MU_MESAG | MU_BUTTON | MU_KEYBD, 2, 1, 1);
    eprintln!("desk single: fired={fired:#06x} mb={mb:#x} bret={bret}");
    assert_eq!((fired & MU_BUTTON, bret, mb & 1), (MU_BUTTON, 1, 1), "single click");

    // Double click: press, release, press, release — all inside the window.
    drain(host, &mut pb);
    queue_click(host, 2);
    queue_click(host, 2);
    let (fired, _, _, mb, bret) = evnt_multi(&mut pb, MU_MESAG | MU_BUTTON | MU_KEYBD, 2, 1, 1);
    eprintln!("desk double: fired={fired:#06x} mb={mb:#x} bret={bret}");
    assert_eq!(fired & MU_BUTTON, MU_BUTTON, "double click fires MU_BUTTON");
    assert_eq!(bret, 2, "a double click must still count as two");
    assert_eq!(mb & 1, 1, "and is reported with the button down");
    drain(host, &mut pb);
    let (fired2, ..) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 2, 1, 1);
    assert_eq!(fired2, MU_TIMER, "no stray button event after the double click; fired = {fired2:#06x}");
}

/// A press held past the window is one transition: `mtrans` stays 1 and the
/// event reports the live state — the button still down (`aes/gemevlib.c:50-53`).
#[test]
fn long_press_reports_live_state() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..PAST_WINDOW {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    let (fired, _, _, mb, bret) = evnt_multi(&mut pb, MU_BUTTON, 0x0102, 3, 0);
    eprintln!("long press: fired={fired:#06x} mb={mb:#x} bret={bret}");
    assert_eq!((fired, bret, mb & 1), (MU_BUTTON, 1, 1));
    host.clear();
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    drain(host, &mut pb);
}

/// A click made while the application was not waiting (between two event
/// calls) is two published transitions; the next `evnt_multi`'s quick check
/// finds the press in the record (`(mtrans > 1) && downorup(pr_button, …)`,
/// `aes/gemevlib.c:190-194`) and delivers it with its recorded click count
/// and `EV_MB` = the press.
#[test]
fn click_between_calls_is_delivered_from_the_record() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, 250, 260);
    queue_click(host, 1);
    drain(host, &mut pb); // the app is busy: graf_mkstate only
    // … and stays busy past the click window, so whatever window the press
    // opened (`gl_bpend` left raised by earlier cancelled waits) has closed
    // and published press and release before the next call.
    idle(host, &mut pb, PAST_WINDOW);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 0);
    let (fired, mx, my, mb, bret) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 0x0102, 3, 0);
    eprintln!("busy click: fired={fired:#06x} mx={mx} my={my} mb={mb:#x} bret={bret}");
    assert_eq!(fired & MU_BUTTON, MU_BUTTON, "the recorded press satisfies the wait");
    assert_eq!(bret, 1, "EV_BRET = pr_mclick");
    assert_eq!((mx, my, mb & 1), (250, 260, 1), "EV_MX/EV_MY/EV_MB = the recorded state");
    let (fired2, ..) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 0x0102, 3, 0);
    assert_eq!(fired2, MU_TIMER, "delivered once; fired = {fired2:#06x}");
}

/// `evnt_button` (`ev_button`, `aes/gemevlib.c:84-94`) ends in the same
/// `ev_rets`: a click in the window reports `EV_MB` = 1 in `int_out[3]`.
#[test]
fn evnt_button_reports_button_down() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    queue_click(host, 2);
    pb.intin.fill(0);
    pb.intin[0] = 2;
    pb.intin[1] = 1;
    pb.intin[2] = 1;
    pb.call(21, 3, 5, 0);
    let (ret, mb) = (pb.intout[0], pb.intout[3]);
    eprintln!("evnt_button: ret={ret} mb={mb:#x}");
    assert_eq!((ret, mb & 1), (1, 1));
}

/// A click the AES consumed itself must not be reported again. Here the press
/// is published while the application is busy and its release inside
/// `graf_rubberbox`, whose `gr_stilldn` returns on it through `ev_multi` →
/// `ev_rets` (`aes/gemgrlib.c:47-73`) — so the next event finds no recorded
/// press.
#[test]
fn release_consumed_by_graf_wait_is_not_reported_again() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    drain(host, &mut pb);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1);
    host.push(Packet { dx: 3, dy: 3, buttons: 1 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    pb.intin.fill(0);
    pb.intin[0] = 320;
    pb.intin[1] = 300;
    pb.call(70, 4, 3, 0); // graf_rubberbox
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 0);
    let (fired, ..) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 1, 1, 1);
    assert_eq!(fired, MU_TIMER, "the press+release was graf_rubberbox's; fired = {fired:#06x}");
}

// ── ZVIEW02 / A111 (1): the double-click window runs on the AES tick ───────

/// Push `n` packets with the button state `b`.
fn hold(host: &'static ScriptHost, b: u8, n: usize) {
    for _ in 0..n {
        host.push(Packet { dx: 0, dy: 0, buttons: b });
    }
}

/// zView's main loop (`EvntWindom(MU_MESAG|MU_TIMER|MU_KEYBD|MU_BUTTON)`,
/// `sdk/zview/z-tools-04805b7/zview/main.c:168`, with `evnt.bclick = 258,
/// bmask = 3, bstate = 0` and `TimerInit(20)`, `:138-143`): `evnt_multi`
/// with a 20-ms timer, again and again, until `MU_BUTTON` comes. Returns every
/// `(fired, EV_MB, EV_BRET)` up to and including the one with `MU_BUTTON`.
fn zview_loop(pb: &mut Aespb) -> Vec<(i16, i16, i16)> {
    let mut out = Vec::new();
    for _ in 0..200 {
        let (fired, _, _, mb, bret) =
            evnt_multi_t(pb, MU_MESAG | MU_TIMER | MU_KEYBD | MU_BUTTON, 0x0102, 3, 0, 20);
        out.push((fired, mb, bret));
        if fired & MU_BUTTON != 0 {
            break;
        }
    }
    out
}

/// **ZVIEW02.** A double click on a folder tile: press 15 ms, release 25 ms,
/// press 15 ms (the QEMU injector's 3/5/3 hz_200 ticks) — while the
/// application polls with a 20-ms timer. In EmuTOS the button-delay window is
/// counted down by the AES tick (`b_delay(1)` from `tikcod`,
/// `aes/gemdosif.S:389-411`), not by the waiting call: the timer returns in
/// between report nothing of the withheld press, and the gesture is published
/// when the window closes — `bchange(1, 2)`, `bchange(0, 1)`
/// (`aes/geminput.c:155-171`) — so the wait comes back with **two clicks and
/// `EV_MB` = down** (`pr_button`), what zView needs to open the folder
/// (`nb_click == 2 && evnt.mbut == 1`, `catalog/catalog_mouse.c:132`).
#[test]
fn timer_poll_loop_double_click_reports_two_clicks_button_down() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, 470, 318);
    hold(host, 0, 1);
    hold(host, 1, 15);
    hold(host, 0, 25);
    hold(host, 1, 15);
    hold(host, 0, 1);
    let rets = zview_loop(&mut pb);
    eprintln!("zView loop, short double click: {rets:?}");
    let (last, before) = rets.split_last().unwrap();
    assert_eq!(last.0 & MU_BUTTON, MU_BUTTON, "the double click must come back as MU_BUTTON");
    assert_eq!(last.2, 2, "EV_BRET: two clicks, not one per press (the window is not cut by MU_TIMER)");
    assert_eq!(last.1 & 1, 1, "EV_MB = the press (pr_button): released inside the window");
    assert!(before.len() >= 5, "the timer keeps firing while the window is open: {} returns", before.len());
    for &(fired, mb, _) in before {
        assert_eq!((fired, mb & 1), (MU_TIMER, 0), "a withheld press is not reported before the window closes");
    }
    let (fired2, ..) = evnt_multi(&mut pb, MU_BUTTON | MU_TIMER, 0x0102, 3, 0);
    assert_eq!(fired2, MU_TIMER, "the gesture is delivered once; fired = {fired2:#06x}");
}

/// The same double click with the **second press held 360 ms** — past the
/// window (the injector's 72-tick variant). The window publishes only the
/// press, `bchange(gl_bdesired, 2)`, the button being still down
/// (`gl_bdesired == gl_btrue`, `aes/geminput.c:166-169`): one transition,
/// so the event reports the live state — down — with two clicks, while the
/// button is still held.
#[test]
fn timer_poll_loop_double_click_second_press_held() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, 470, 110);
    hold(host, 0, 1);
    hold(host, 1, 15);
    hold(host, 0, 25);
    hold(host, 1, 360);
    let rets = zview_loop(&mut pb);
    eprintln!("zView loop, held double click: {rets:?}");
    let last = *rets.last().unwrap();
    assert_eq!((last.0 & MU_BUTTON, last.2, last.1 & 1), (MU_BUTTON, 2, 1));
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "delivered at the window's close, the press still held");
    host.clear();
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    drain(host, &mut pb);
}

/// **A111 (1).** The window is `gl_dclick` ticks (220 ms / 20 ms = 11 at the
/// default index 3, `ev_dclick`, `aes/gemevlib.c:300-309`) from the opening
/// press, **plus three ticks per further click** (`gl_bdely += 3`,
/// `aes/geminput.c:121-122`) — not a fresh full window per click — and the
/// count is reported only when it closes. A wait for three clicks gets a
/// second press 100 ms after the first (window now 14 ticks, 260..280 ms) and
/// a third at 300 ms: outside, so the answer is **2**, delivered 260..280 ms
/// after the first press. With a full window per click the third press would
/// have counted (100 + 220 = 320 ms).
#[test]
fn window_extends_three_ticks_per_click_and_reports_at_its_end() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    hold(host, 0, 1);
    hold(host, 1, 10); // press at t0 (the 2nd pump of the call)
    hold(host, 0, 90);
    hold(host, 1, 10); // second press at t0 + 100
    hold(host, 0, 190);
    hold(host, 1, 10); // third press at t0 + 300
    hold(host, 0, 1);
    let c0 = host.clock.load(Ordering::Relaxed);
    let (fired, _, _, mb, bret) = evnt_multi(&mut pb, MU_BUTTON, 0x0003, 1, 1);
    let at = host.clock.load(Ordering::Relaxed) - (c0 + 2);
    eprintln!("three-click wait: fired={fired:#06x} mb={mb:#x} bret={bret} delivered at t0+{at} ms");
    assert_eq!(fired, MU_BUTTON);
    assert_eq!(bret, 2, "the third press (t0+300) is outside the extended window (t0+260..280)");
    assert_eq!(mb & 1, 1, "EV_MB = the press: the release was published with it");
    assert!((255..=285).contains(&at), "reported at the window's end, t0+{at} ms");
    // The third press is a gesture of its own: published at once (no waiter
    // wants more than one click now), and seen by the next wait.
    drain(host, &mut pb);
    idle(host, &mut pb, PAST_WINDOW);
}

/// `evnt_button` (`ev_button`, `aes/gemevlib.c:84-94`) for two clicks: a
/// double click inside the window returns **2**, delivered when the window
/// closes (`post_button` → `min(2, 2)`, `aes/geminput.c:463-483`), with
/// `EV_MB` = down.
#[test]
fn evnt_button_double_click_returns_two_at_window_end() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    hold(host, 0, 1);
    hold(host, 1, 15);
    hold(host, 0, 25);
    hold(host, 1, 15);
    hold(host, 0, 1);
    let c0 = host.clock.load(Ordering::Relaxed);
    pb.intin.fill(0);
    pb.intin[0] = 2;
    pb.intin[1] = 1;
    pb.intin[2] = 1;
    pb.call(21, 3, 5, 0);
    let at = host.clock.load(Ordering::Relaxed) - c0;
    let (ret, mb) = (pb.intout[0], pb.intout[3]);
    eprintln!("evnt_button double: ret={ret} mb={mb:#x} at +{at} ms");
    assert_eq!((ret, mb & 1), (2, 1));
    assert!(at >= 200, "delivered at the window's end (14 ticks after the press), +{at} ms");
}

// ── O1: the objc_edit caret ────────────────────────────────────────────────

/// Byte layout of the LP64 `TEDINFO`: three 8-byte pointers, then eight WORDs.
#[repr(C)]
struct Ted {
    ptext: u64,
    ptmplt: u64,
    pvalid: u64,
    font: i16,
    fontid: i16,
    just: i16,
    color: i16,
    fontsize: i16,
    thickness: i16,
    txtlen: i16,
    tmplen: i16,
}

const FIELD: (i16, i16, i16, i16) = (100, 100, 200, 21);

/// The caret's top-left pixel after `objc_edit(ED_INIT)` on a `G_FTEXT` field
/// with text "AB" and template "________" (8 cells = 64 px), found as the
/// pixels `ED_INIT` changes. Since A111 `ED_INIT` draws the caret alone — EmuTOS
/// `curfld`'s XOR line, no field redraw (`aes/gemobed.c:363-365,459-461`) —
/// so the changed pixels are the caret column; `ED_END` would switch it off
/// again and is not used as a "field only" primer any more.
fn caret_origin(just: i16) -> (i16, i16) {
    let host = rig();
    host.clear();
    let mut text = Box::new(*b"AB\0\0\0\0\0\0\0");
    let tmpl: &'static [u8] = b"________\0";
    let valid: &'static [u8] = b"XXXXXXXX\0";
    let ted = Box::new(Ted {
        ptext: text.as_mut_ptr() as u64,
        ptmplt: tmpl.as_ptr() as u64,
        pvalid: valid.as_ptr() as u64,
        font: 3,
        fontid: 0,
        just,
        color: 0x1180,
        fontsize: 0,
        thickness: 0,
        txtlen: 9,
        tmplen: 9,
    });
    let (fx, fy, fw, fh) = FIELD;
    let mut tree = Box::new([
        Obj { next: -1, head: 1, tail: 1, otype: 20, flags: 0, state: 0, spec: 0, x: 0, y: 0, w: 640, h: 480 },
        Obj {
            next: 0,
            head: -1,
            tail: -1,
            otype: G_FTEXT,
            flags: EDITABLE | LASTOB,
            state: 0,
            spec: &*ted as *const Ted as u64,
            x: fx,
            y: fy,
            w: fw,
            h: fh,
        },
    ]);
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let mut edit = |pb: &mut Aespb, kind: i16, idx: i16| {
        pb.intin.fill(0);
        pb.intin[0] = 1;
        pb.intin[2] = idx;
        pb.intin[3] = kind;
        pb.addrin[0] = tree.as_mut_ptr() as u64;
        pb.call(46, 4, 2, 1);
        pb.intout[1]
    };
    // A sentinel ground, so a transparent field still shows the caret.
    host.raster.px.lock().unwrap().fill(0x00AB_CDEF);
    let before = host.raster.px.lock().unwrap().clone();
    let idx = edit(&mut pb, 1, 0); // ED_INIT: the caret at the end of "AB"
    assert_eq!(idx, 2, "ED_INIT puts the index at the end of the text");
    let after = host.raster.px.lock().unwrap().clone();
    let mut min = (i16::MAX, i16::MAX);
    for (i, (a, b)) in before.iter().zip(after.iter()).enumerate() {
        if a != b {
            let (x, y) = ((i % SCREEN_W as usize) as i16, (i / SCREEN_W as usize) as i16);
            min = (min.0.min(x), min.1.min(y));
        }
    }
    drop(text);
    min
}

/// `pxl_rect` (`aes/gemobed.c:180-190`): `gr_just(te_just, te_font,
/// te_ptmplt, w, h)` then `+ ch_pos * gl_wchar`. Template width 64 in a
/// 200-wide field leaves 136 px of slack; the 21-px height leaves 5, centred
/// rounding up: `(5 + 1) / 2 = 3` (`aes/gemgraf.c:571-584`). `curfld` starts
/// the caret line 3 rows above that cell (`t.g_y -= 3`, `aes/gemobed.c:203`).
#[test]
fn objc_edit_caret_follows_te_just() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (fx, fy, ..) = FIELD;
    let ch_pos = 2 * 8;
    for (just, name, dx) in [(0, "TE_LEFT", 0), (1, "TE_RIGHT", 136), (2, "TE_CNTR", 68)] {
        let got = caret_origin(just);
        eprintln!("{name}: caret at {got:?}");
        assert_eq!(got, (fx + dx + ch_pos, fy + 3 - 3), "{name}: caret origin");
    }
}
