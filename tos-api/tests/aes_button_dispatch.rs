//! Reproduction of the real-Pi-4 double-click freeze, at the AES trap boundary.
//!
//! **The gesture.** TeraDesk's main loop waits with `ev_mbclicks = 0x102`,
//! `ev_mbmask = 3`, `ev_mbstate = 0` and *no* timer
//! (`sdk/apps/teradesk/main.c:1588-1590,1600-1610`). On a `MU_BUTTON` with two
//! clicks it runs `wd_hndlbutton`'s `n == 2` arm, which selects the item and
//! then spins:
//!
//! ```c
//! if (n == 2) {
//!     itm_select(w, item, 0, TRUE);
//!     /* Wait for a button click */
//!     while (xe_button_state()) ;          /* sdk/apps/teradesk/window.c:4412 */
//!     ...
//! }
//! ```
//!
//! and `xe_button_state()` is a bare `graf_mkstate`
//! (`sdk/apps/teradesk/library/xdialog/xdevent.c:251-258`). When the
//! double-click event is delivered while the *second* press is still held —
//! it is held past the button-delay window, whose close delivers the gesture
//! (`b_delay`, `aes/geminput.c:155-171`) — that spin runs at least one turn.
//!
//! **Why it must terminate.** In EmuTOS every AES call ends in a dispatch:
//! `super()` runs the opcode switch and falls through to `dsptch()`
//! (`aes/gemsuper.c:452-464`) → `disp()`, whose tail is
//! `do { if (fpcnt) forker(); schedule(); } while (fpcnt);`
//! (`aes/gemdisp.c:236-242`). `forker()` runs the queued `bchange`, which is
//! what publishes the new `button` (`aes/geminput.c:333-343`) that
//! `gr_mkstate` then hands out (`aes/gemgrlib.c:458-464`). So an application
//! that does nothing but call `graf_mkstate` still observes the release.
//!
//! **What this test drives.** A [`ScriptHost`] whose `pump_input` — the only
//! thing in this kernel that advances the pointer, exactly as on the board,
//! where the USB-HID mouse is polled from `Host::pump_input` and the AES trap
//! runs with IRQs masked — delivers one scripted mouse packet through
//! [`tos_api::vdi::feed_relative`], the VDI `mouse_int` that calls the AES's
//! `user_but`/`user_mot` vectors. The test then plays a press / release /
//! press / release script, asks `evnt_multi` with TeraDesk's parameters, and
//! reproduces the `while (xe_button_state()) ;` spin with a hard iteration cap
//! so a regression fails the test instead of hanging the runner.
//!
//! Without the trailing dispatch the spin reads the same frozen button byte
//! forever — on the board, inside a trap with interrupts masked, that is the
//! whole machine.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend that is only ever asked for its geometry ──────────────

/// Test double for the framebuffer. Every entry point works on a real linear
/// buffer, so nothing here silently swallows a draw — but nothing in this test
/// draws either: the VDI cursor starts hidden (`HIDE_COUNT` = 1,
/// `tos-api/src/vdi/mouse.rs:33`), so `feed_relative` only moves the cursor
/// state and asks the backend for `width`/`height`.
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
    /// Milliseconds. Advanced by `pump_input` alone, so "time passes" and
    /// "input is polled" are the same event — which is what the cooperative
    /// kernel does: `monotonic_millis` is a counter read, and nothing else on
    /// the trap path can make a packet appear.
    clock: AtomicU64,
}

impl ScriptHost {
    fn new(script: &[Packet]) -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(script.iter().copied().collect()),
            clock: AtomicU64::new(0),
        }
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
        if let Some(p) = next {
            tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
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

// ── the reproduction ───────────────────────────────────────────────────────

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;

/// TeraDesk's main-loop wait: `MU_MESAG | MU_BUTTON | MU_KEYBD`
/// (`sdk/apps/teradesk/main.c:64`), `ev_mbclicks = 0x102`, `ev_mbmask = 3`,
/// `ev_mbstate = 0`, and no timer while no accessory window is open
/// (`main.c:1588-1590,1600-1610`).
fn evnt_multi_teradesk(pb: &mut Aespb) -> (i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0102; // bflag 1 (wait for a press) + up to 2 clicks
    pb.intin[2] = 3; // both buttons
    pb.intin[3] = 0;
    pb.addrin[0] = 0; // no message buffer needed here
    pb.call(25, 16, 7, 1);
    (pb.intout[0], pb.intout[6], pb.intout[3]) // fired, EV_BRET, EV_MB
}

/// `xe_button_state()` — `graf_mkstate` and nothing else
/// (`sdk/apps/teradesk/library/xdialog/xdevent.c:251-258`).
fn xe_button_state(pb: &mut Aespb) -> i16 {
    pb.call(79, 0, 5, 0);
    pb.intout[3]
}

fn install(host: &'static ScriptHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

/// A double click delivered to a TeraDesk-shaped `evnt_multi`, followed by the
/// `while (xe_button_state()) ;` spin the `n == 2` arm runs
/// (`sdk/apps/teradesk/window.c:4406-4412`).
///
/// The script holds the pointer still (`dx = dy = 0`) so the ±2 movement
/// cancel in `mchange` (`aes/geminput.c:365-368`) never fires and the gesture
/// is a clean double click.
#[test]
fn double_click_spin_on_graf_mkstate_terminates() {
    // ZVIEW02: one empty packet first, so the click arrives while TeraDesk is
    // already waiting — its `evnt_multi`'s entry pump takes the empty packet,
    // the 2-click wait is armed (`abutton` → `gl_bpend++`,
    // `aes/geminput.c:559-561`) and the press then opens the button-delay
    // window (`b_click`, `:133-140`). The double click is delivered when that
    // window closes (`b_delay`, `:155-171`), 11 AES ticks (220 ms) after the
    // first press; the second press is held past that so it is still down at
    // the delivery — the case that makes TeraDesk's spin run.
    let mut script = vec![
        Packet { dx: 0, dy: 0, buttons: 0 }, // the wait is armed
        Packet { dx: 0, dy: 0, buttons: 1 }, // press 1
        Packet { dx: 0, dy: 0, buttons: 0 }, // release 1
        Packet { dx: 0, dy: 0, buttons: 1 }, // press 2  → the double click
    ];
    // The release of the second press arrives after the window has closed,
    // some polls into the spin. Pad with repeats of the same level so the
    // packet queue is not the thing that ends the spin — only the button byte
    // reaching 0 can.
    for _ in 0..300 {
        script.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    script.push(Packet { dx: 0, dy: 0, buttons: 0 }); // release 2
    for _ in 0..64 {
        script.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost::new(&script)));
    install(host);

    let mut pb = Aespb::new();
    let (fired, clicks, mb) = evnt_multi_teradesk(&mut pb);

    assert_eq!(
        fired & MU_BUTTON,
        MU_BUTTON,
        "evnt_multi must report MU_BUTTON for a press with bflag = 1 \
         (downorup, aes/geminput.c:348-357); got 0x{fired:04x}"
    );
    assert_eq!(
        clicks, 2,
        "EV_BRET must be the real click count; 0x0102 asks for 2 \
         (bclicks = LOBYTE(HIWORD(p)), aes/geminput.c:537)"
    );
    assert_eq!(
        mb & 1,
        1,
        "the second press is still held when the double click is delivered — \
         that is what makes TeraDesk's spin run at all"
    );

    // TeraDesk: `while (xe_button_state()) ;`. Bounded so a regression fails
    // instead of hanging the test runner; on the board this loop has no bound
    // and the trap runs with interrupts masked.
    let mut turns = 0;
    while xe_button_state(&mut pb) != 0 {
        turns += 1;
        assert!(
            turns < 1000,
            "graf_mkstate never observed the release: the AES trap did not run \
             EmuTOS' trailing dispatch (super() → dsptch() → disp() → forker(), \
             aes/gemsuper.c:452-464, aes/gemdisp.c:236-242), so the button byte \
             is frozen for the whole spin. On a Pi 4 this is the machine hang."
        );
    }
    assert!(turns > 0, "the spin must actually have run at least one turn");
}
