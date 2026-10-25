//! GH1 — **the window-tree lock is a real mutex, the Control Manager holds it
//! until the selecting press is released, and a button event goes to the
//! mouse owner only.**
//!
//! Spec: `docs/api/aes/window-graphics.md` `wind_update (107)` § "The refusal
//! parks the taker — a real mutex (GH1)"; `docs/api/aes/event.md` "The
//! double-click window" NewTOS mapping (`post_button` walks the mouse owner's
//! waits). EmuTOS: `tak_flag`/`amutex`/`unsync` (`aes/gemflag.c:68-130`),
//! `wm_update` (`aes/gemwmlib.c:1778-1797`), `ctlmgr`'s `wm_update(BEG_UPDATE)`
//! … `wm_update(END_UPDATE)` around its event handling and `ct_msgup`'s wait
//! for the button to come up (`aes/gemctrl.c:100-101,463,484`),
//! `post_button(gl_mowner, …)` (`aes/geminput.c:344,463-483`).
//!
//! The defect (`TASK/GH1.md`, `measurements/measurement-2026-10-19-gh1/`): a
//! desk accessory opened from the *Desk* menu ran `wind_update(BEG_UPDATE)` and
//! its dialog's `fm_own(TRUE)` while the picking press was still held, so it
//! never became the mouse owner; every click in its dialog also completed the
//! desktop's parked `MU_BUTTON`, and the desktop repainted its window frame
//! over the dialog.
//!
//! Harness: GEMDOS process slots the host "runs" (`RUN` + `sched::set_running`)
//! and re-issues the parameter block of a call that yielded (the S2 restartable
//! block), as `aes_a112_wake_conditions.rs` does; mouse packets advance only in
//! `Host::pump_input`, as on the board (`aes_menu_click_ownership.rs`, whose
//! raster stand-in and menu tree are reused). Whether a call **completed** is
//! read from `int_out[0]`, pre-set to a sentinel no call returns.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::{self, sched, Abi, MAX_PROCESSES, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_types::BasePage;

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
    /// VDI convention: bit 0 = left.
    buttons: u8,
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
    /// Milliseconds; advanced by `pump_input` alone.
    clock: AtomicU64,
    /// Consecutive pumps on an empty script — a wait that never sees its
    /// event would spin inside the trap; the cap turns that into a failure.
    idle: AtomicUsize,
}

const IDLE_CAP: usize = 20_000;

impl ScriptHost {
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn pending(&self) -> usize {
        self.script.lock().unwrap().len()
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
                    "the AES is still waiting after {n} pumps on an exhausted mouse script \
                     (pointer {:?}, buttons {:#04x}, lock {:?})",
                    tos_api::vdi::mouse_position(),
                    tos_api::vdi::mouse_buttons(),
                    tos_api::aes::window::test_update_lock()
                );
            }
        }
    }
}

// SAFETY: every mutable field is behind a Mutex / atomic.
unsafe impl Sync for ScriptHost {}
unsafe impl Send for ScriptHost {}

static RIG: OnceLock<&'static ScriptHost> = OnceLock::new();
static SERIAL: Mutex<()> = Mutex::new(());

fn rig() -> &'static ScriptHost {
    RIG.get_or_init(|| {
        let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(VecDeque::new()),
            clock: AtomicU64::new(1_000),
            idle: AtomicUsize::new(0),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        host
    })
}

// ── AES parameter block ────────────────────────────────────────────────────

const ABI: Abi = Abi::Lp64;

/// `int_out[0]` before a call: no AES call returns it, so finding it there
/// afterwards means the call did not complete (no copy-back happened).
const NOT_COMPLETED: i16 = -0x5A5A;

struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

/// What one trap did: the slot the dispatcher switched to, if any, and
/// `RET_CODE` when the call completed.
#[derive(Debug)]
struct Outcome {
    switch_to: Option<usize>,
    ret: Option<i16>,
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

    /// Issue (or re-issue) the call in `contrl` as the running context.
    fn trap(&mut self) -> Outcome {
        self.intout.fill(0);
        self.intout[0] = NOT_COMPLETED;
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        let sw = unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
        Outcome {
            switch_to: sw.map(|s| s.slot),
            ret: (self.intout[0] != NOT_COMPLETED).then_some(self.intout[0]),
        }
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> Outcome {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.trap()
    }
}

const APPL_INIT: i16 = 10;
const EVNT_MULTI: i16 = 25;
const MENU_BAR: i16 = 30;
const GRAF_MOUSE: i16 = 78;
const GRAF_MKSTATE: i16 = 79;
const WIND_UPDATE: i16 = 107;
const WIND_NEW: i16 = 109;

const END_UPDATE: i16 = 0;
const BEG_UPDATE: i16 = 1;
const BEG_MCTRL: i16 = 3;

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const MN_SELECTED: i16 = 10;

fn appl_init(pb: &mut Aespb) {
    pb.intin.fill(0);
    assert!(pb.call(APPL_INIT, 0, 1, 0).ret.is_some(), "appl_init completes");
}

fn wind_update(pb: &mut Aespb, mode: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = mode;
    pb.call(WIND_UPDATE, 1, 1, 0)
}

fn graf_mkstate(pb: &mut Aespb) {
    pb.intin.fill(0);
    let _ = pb.call(GRAF_MKSTATE, 0, 5, 0);
}

/// `evnt_multi(MU_BUTTON)` for one left press (`clicks 1, mask 1, state 1`),
/// plus `extra` flags (no timer unless `MU_TIMER` with 0 ms).
fn evnt_multi_button(pb: &mut Aespb, extra: i16, msg: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = MU_BUTTON | extra;
    pb.intin[1] = 1;
    pb.intin[2] = 1;
    pb.intin[3] = 1;
    pb.addrin.fill(0);
    pb.addrin[0] = msg.as_mut_ptr() as u64;
    pb.call(EVNT_MULTI, 16, 7, 1)
}

/// The desktop's wait (`MU_MESAG | MU_BUTTON | MU_KEYBD`, one or two presses)
/// with an optional 0-ms timer: `(fired, msg)`.
fn evnt_multi_desk(pb: &mut Aespb, poll: bool) -> (Outcome, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD | if poll { MU_TIMER } else { 0 };
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    let o = pb.call(EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    (o, *buf)
}

// ── process slots the host can "run" ───────────────────────────────────────

/// A claimed GEMDOS slot with a basepage page of its own; `acc` registers it
/// as a desk accessory — an `AESPD` of its own (`D.g_acc[]`). Released on drop.
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
    acc: bool,
}

impl Proc {
    fn new(acc: bool) -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot].basepage_kva.store(page.cast::<BasePage>(), Ordering::Release);
        if acc {
            tos_api::aes::acc::register_process(slot);
        }
        Proc { slot, page, acc }
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process for GEMDOS and the scheduler.
    fn run(&self) {
        process::set_current(self.bp());
        sched::set_running(self.slot);
    }

    /// Put this slot on the ready list as a process that has run before.
    fn ready(&self) {
        PROCESS_TABLE[self.slot].has_ctx.store(true, Ordering::Release);
        sched::ready(self.slot);
    }

    fn is_ready(&self) -> bool {
        PROCESS_TABLE[self.slot].runnable.load(Ordering::Acquire)
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
        sched::set_running(MAX_PROCESSES);
        sched::unready(self.slot);
        PROCESS_TABLE[self.slot].wait_cond.store(0, Ordering::Release);
        if self.acc {
            tos_api::aes::acc::forget_process(self.slot);
        }
        // SAFETY: single-threaded under `SERIAL`.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` and the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

/// The kernel-context client (no running process) — the application chain.
fn kernel_context() {
    process::set_current(std::ptr::null_mut());
    sched::set_running(MAX_PROCESSES);
}

/// Serialise on the process-wide statics and start clean: kernel context,
/// no window, no lock, empty script, pointer parked on the desktop.
fn lock() -> (MutexGuard<'static, ()>, &'static ScriptHost) {
    let host = rig();
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    host.clear();
    kernel_context();
    let mut pb = Aespb::new();
    appl_init(&mut pb);
    pb.intin.fill(0);
    let _ = pb.call(WIND_NEW, 0, 1, 0);
    assert_eq!(tos_api::aes::window::test_update_lock(), (0, None), "wind_new drains the lock");
    move_to(host, &mut pb, 320, 300, 0);
    (g, host)
}

/// Walk the pointer to `(tx, ty)` with `buttons`, one packet per AES call.
fn move_to(host: &'static ScriptHost, pb: &mut Aespb, tx: i16, ty: i16, buttons: u8) {
    for _ in 0..64 {
        if host.pending() == 0 {
            break;
        }
        graf_mkstate(pb);
    }
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

// ── the menu tree (`aes_menu_click_ownership.rs`) ──────────────────────────

/// A GEM `OBJECT` at the kernel/LP64 record width.
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

const TITLE_FILE: i16 = 4;
const ITEM_OPEN: i16 = 12;
const BAR_H: i16 = 19;
const FILE_X: i16 = 56;
const ITEM_H: i16 = 19;

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

/// Install the menu bar from the kernel-context client (the desktop).
fn install_menu(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.addrin.fill(0);
    pb.addrin[0] = menu_tree() as u64;
    assert_eq!(pb.call(MENU_BAR, 1, 1, 1).ret, Some(1), "menu_bar installs the tree");
    pb.addrin[0] = 0;
    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    let _ = pb.call(GRAF_MOUSE, 1, 1, 0);
}

/// Take the bar down again, so the next case starts without one.
fn remove_menu(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.addrin.fill(0);
    pb.addrin[0] = 0;
    let _ = pb.call(MENU_BAR, 1, 1, 1);
}

const ON_FILE: (i16, i16) = (FILE_X + 28, 8);
const ON_OPEN: (i16, i16) = (FILE_X + 28, BAR_H + ITEM_H + 9);

/// Queue the hover-select gesture on *File → Open*: down onto the item with
/// the button up, press (the selection), hold for `hold` packets, release,
/// and `tail` button-up packets after it.
fn queue_pick_open(host: &'static ScriptHost, hold: usize, tail: usize) {
    let steps = (ON_OPEN.1 - ON_FILE.1) / 10;
    for _ in 0..steps {
        host.push(Packet { dx: 0, dy: 10, buttons: 0 });
    }
    host.push(Packet { dx: 0, dy: ON_OPEN.1 - ON_FILE.1 - steps * 10, buttons: 0 });
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..hold {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..tail {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
}

use tos_api::aes::window::{test_mctrl_count, test_update_lock};

/// `ctl_pd` as the recorded holder of the lock (`window.rs` `CTL_PD`).
const CTL_PD: usize = usize::MAX - 1;
/// The kernel-context client's identity (`pd::KERNEL_CONTEXT`).
const KERNEL_CONTEXT: usize = MAX_PROCESSES;

// ── 1. a second process's BEG_UPDATE parks until the holder lets go ───────

/// `tak_flag` refuses a held lock to another process and `wm_update` parks it
/// on `sy_wait` (`aes/gemflag.c:68-101`, `aes/gemwmlib.c:1785-1786`); the
/// holder's last `END_UPDATE` hands it over with the count at 1 and wakes the
/// waiter (`unsync`, `:105-130`), whose call then returns.
#[test]
fn beg_update_of_another_process_parks_until_the_holder_ends() {
    let (_g, _host) = lock();
    let a = Proc::new(true); // a desk accessory: an AESPD of its own
    let b = Proc::new(false); // the application chain (the desktop)
    let mut pa = Aespb::new();
    let mut pbb = Aespb::new();
    a.run();
    appl_init(&mut pa);
    b.run();
    appl_init(&mut pbb);

    a.run();
    assert_eq!(wind_update(&mut pa, BEG_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(a.slot)), "the accessory holds the lock");

    a.ready();
    b.run();
    let w = wind_update(&mut pbb, BEG_UPDATE);
    assert_eq!(w.ret, None, "a second process's BEG_UPDATE must not return while the lock is held");
    assert_eq!(w.switch_to, Some(a.slot), "…it parks and gives the CPU to the holder");
    assert!(!b.is_ready(), "…and is off the ready list (WAITIN on sy_wait)");
    assert_eq!(test_update_lock(), (1, Some(a.slot)), "the refused take counts nothing");

    a.run();
    let e = wind_update(&mut pa, END_UPDATE);
    assert_eq!(e.ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(b.slot)), "unsync hands the lock to the waiter, count 1");
    assert!(b.is_ready(), "…and wakes it (azombie)");
    // A115: the woken waiter is on `drl` and joins `rlr` behind the holder
    // (`disp_act` before `schedule()`, `gemdisp.c:231-243`); the holder,
    // otherwise alone on `rlr`, keeps the CPU for this call.
    assert_eq!(e.switch_to, None, "…and joins the ready list behind the holder");
    pa.intin.fill(0);
    let next = pa.call(GRAF_MKSTATE, 0, 5, 0);
    assert_eq!(next.switch_to, Some(b.slot), "the holder's next AES call gives it the CPU");

    b.run();
    let r = pbb.trap(); // the re-executed svc
    assert_eq!(r.ret, Some(1), "the waiter's BEG_UPDATE returns once the lock is its own");
    assert_eq!(test_update_lock(), (1, Some(b.slot)), "the hand-over is not counted twice");
    assert_eq!(wind_update(&mut pbb, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 2. the application chain is one process ───────────────────────────────

/// EmuTOS runs the desktop and every program it launches on one `AESPD`
/// (`aes/geminit.c:851-877`), so a program's `BEG_UPDATE` nests on the
/// desktop's hold instead of waiting for it.
#[test]
fn beg_update_nests_within_the_application_chain() {
    let (_g, _host) = lock();
    let b = Proc::new(false);
    let c = Proc::new(false);
    let mut pbb = Aespb::new();
    let mut pc = Aespb::new();
    let mut pk = Aespb::new();
    b.run();
    appl_init(&mut pbb);
    c.run();
    appl_init(&mut pc);

    b.run();
    assert_eq!(wind_update(&mut pbb, BEG_UPDATE).ret, Some(1));
    c.run();
    assert_eq!(wind_update(&mut pc, BEG_UPDATE).ret, Some(1), "same AESPD: tak_flag succeeds");
    kernel_context();
    assert_eq!(wind_update(&mut pk, BEG_UPDATE).ret, Some(1), "the kernel-context client is the chain too");
    assert_eq!(test_update_lock(), (3, Some(b.slot)), "one lock, three nested holds, the first taker recorded");
    assert_eq!(wind_update(&mut pk, END_UPDATE).ret, Some(1));
    b.run();
    assert_eq!(wind_update(&mut pbb, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(c.slot)), "the remaining share stays with the chain");
    c.run();
    assert_eq!(wind_update(&mut pc, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 3. waiters are served in arrival order ────────────────────────────────

/// `amutex` appends to `sy_wait`, `unsync` takes the head
/// (`aes/gemflag.c:97-101,118-123`).
#[test]
fn waiters_are_served_in_arrival_order() {
    let (_g, _host) = lock();
    let a = Proc::new(true);
    let c = Proc::new(true);
    let b = Proc::new(false);
    let (mut pa, mut pc, mut pbb) = (Aespb::new(), Aespb::new(), Aespb::new());
    for (p, pb) in [(&a, &mut pa), (&c, &mut pc), (&b, &mut pbb)] {
        p.run();
        appl_init(pb);
    }

    a.run();
    assert_eq!(wind_update(&mut pa, BEG_UPDATE).ret, Some(1));
    a.ready();
    c.run();
    assert_eq!(wind_update(&mut pc, BEG_UPDATE).ret, None, "C waits first");
    a.ready();
    b.run();
    assert_eq!(wind_update(&mut pbb, BEG_UPDATE).ret, None, "B waits second");

    a.run();
    assert_eq!(wind_update(&mut pa, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(c.slot)), "the head of the line gets the lock");

    b.run();
    assert_eq!(pbb.trap().ret, None, "B, woken too, finds the lock C's and waits on");
    c.run();
    assert_eq!(pc.trap().ret, Some(1), "C's call returns");
    assert_eq!(wind_update(&mut pc, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(b.slot)), "then B");
    b.run();
    assert_eq!(pbb.trap().ret, Some(1));
    assert_eq!(wind_update(&mut pbb, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 4. BEG_MCTRL waits as BEG_UPDATE does ─────────────────────────────────

/// `BEG_MCTRL` is `fm_own(TRUE)`, whose first act is `wm_update(BEG_UPDATE)`
/// (`aes/gemfmlib.c:86`): it waits for the lock before it takes the mouse.
#[test]
fn beg_mctrl_waits_for_the_lock_before_it_takes_the_mouse() {
    let (_g, _host) = lock();
    let a = Proc::new(true);
    let b = Proc::new(false);
    let (mut pa, mut pbb) = (Aespb::new(), Aespb::new());
    a.run();
    appl_init(&mut pa);
    b.run();
    appl_init(&mut pbb);

    a.run();
    assert_eq!(wind_update(&mut pa, BEG_UPDATE).ret, Some(1));
    a.ready();
    b.run();
    assert_eq!(wind_update(&mut pbb, BEG_MCTRL).ret, None, "BEG_MCTRL parks behind the holder");
    assert_eq!(test_mctrl_count(), (0, None), "…before ml_ocnt moves");

    a.run();
    assert_eq!(wind_update(&mut pa, END_UPDATE).ret, Some(1));
    b.run();
    assert_eq!(pbb.trap().ret, Some(1));
    assert_eq!(test_mctrl_count(), (1, Some(b.slot)), "fm_own's 0 → 1 edge ran for B");
    assert_eq!(test_update_lock(), (1, Some(b.slot)), "fm_own's BEG_UPDATE is the handed-over count");
    assert_eq!(wind_update(&mut pbb, 2).ret, Some(1)); // END_MCTRL
    assert_eq!(test_mctrl_count(), (0, None));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 5. a dead holder hands the lock on ────────────────────────────────────

/// A client that ends holding the lock gives it up (`sh_ldapp`'s
/// `if (wind_spb.sy_owner == rlr) unsync(&wind_spb)`, `aes/gemshlib.c:668-669`);
/// through `unsync`, so the next waiter gets it.
#[test]
fn a_dead_holder_hands_the_lock_to_the_waiter() {
    let (_g, _host) = lock();
    let b = Proc::new(false);
    let mut pbb = Aespb::new();
    b.run();
    appl_init(&mut pbb);
    {
        let a = Proc::new(true);
        let mut pa = Aespb::new();
        a.run();
        appl_init(&mut pa);
        assert_eq!(wind_update(&mut pa, BEG_UPDATE).ret, Some(1));
        a.ready();
        b.run();
        assert_eq!(wind_update(&mut pbb, BEG_UPDATE).ret, None, "B waits for A");
    } // A's slot is released here (release_slot → the sweep at the next dispatch)
    b.run();
    let r = pbb.trap();
    assert_eq!(r.ret, Some(1), "the next dispatch's sweep releases A's hold and B's call returns");
    assert_eq!(test_update_lock(), (1, Some(b.slot)));
    assert_eq!(wind_update(&mut pbb, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 6. a press completes the mouse owner's wait only (F2) ─────────────────

/// `bchange` → `post_button(gl_mowner, …)` walks the **owner's** `c_bsleep`
/// list (`aes/geminput.c:344,463-483`). With an accessory owning the mouse
/// (its `BEG_MCTRL`, `fm_own` → `ct_chgown(rlr, …)`, `aes/gemfmlib.c:93`) a
/// press completes the accessory's parked `MU_BUTTON` and not the desktop's —
/// in GH1 the desktop acted on every click in the accessory's dialog.
#[test]
fn a_press_completes_only_the_mouse_owners_button_wait() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let b = Proc::new(false);
    let (mut pa, mut pbb) = (Aespb::new(), Aespb::new());
    let (mut ma, mut mb) = ([0i16; 8], [0i16; 8]);
    a.run();
    appl_init(&mut pa);
    b.run();
    appl_init(&mut pbb);

    a.run();
    assert_eq!(wind_update(&mut pa, BEG_MCTRL).ret, Some(1), "the accessory takes the mouse");
    b.ready();
    let wa = evnt_multi_button(&mut pa, 0, &mut ma);
    assert_eq!(wa.ret, None, "the accessory's button wait parks");
    assert_eq!(wa.switch_to, Some(b.slot));

    // The desktop waits for a press too; the press arrives while it is parked.
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // drained by ev_multi's entry
    host.push(Packet { dx: 0, dy: 0, buttons: 1 }); // the press, while parked
    b.run();
    let wb = evnt_multi_button(&mut pbb, 0, &mut mb);
    assert_eq!(
        wb.ret, None,
        "the press must not complete the desktop's wait: the accessory owns the mouse \
         (post_button(gl_mowner, …), aes/geminput.c:463-483)"
    );
    assert_eq!(wb.switch_to, Some(a.slot), "…it completes the accessory's, which runs next");

    a.run();
    assert_eq!(pa.trap().ret, Some(MU_BUTTON), "the accessory gets the press");
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    graf_mkstate(&mut pa);
    assert_eq!(wind_update(&mut pa, 2).ret, Some(1)); // END_MCTRL
}

// ── 7. the Control Manager holds the lock until the selecting press is up ─

/// `ctlmgr` takes the screen before `hctl_rect` and gives it up only after
/// `ct_msgup`'s `while (button & 0x0001) dsptch();`
/// (`aes/gemctrl.c:100-101,463,484`). The menu owner's own `BEG_UPDATE` after
/// `MN_SELECTED` (the desktop's event bracket, `desk/deskmain.c:2094`) returns
/// only with the button up.
#[test]
fn the_menu_owners_beg_update_after_a_pick_waits_for_the_release() {
    let (_g, host) = lock();
    let mut pk = Aespb::new();
    install_menu(&mut pk);
    move_to(host, &mut pk, ON_FILE.0, ON_FILE.1, 0);
    queue_pick_open(host, 6, 4);

    let (o, msg) = evnt_multi_desk(&mut pk, false);
    assert_eq!(o.ret.map(|w| w & MU_MESAG), Some(MU_MESAG), "the pick arrives as a message");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN));
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the selecting press is still held");
    assert_eq!(test_update_lock(), (1, Some(CTL_PD)), "ct_msgup waits inside ctlmgr's BEG_UPDATE");

    let w = wind_update(&mut pk, BEG_UPDATE);
    assert_eq!(w.ret, Some(1));
    assert_eq!(
        tos_api::vdi::mouse_buttons() & 1,
        0,
        "BEG_UPDATE returned while the selecting press was held: the Control Manager \
         must hold the screen until ct_msgup's wait ends (aes/gemctrl.c:100-101,484)"
    );
    assert_eq!(test_update_lock(), (1, Some(KERNEL_CONTEXT)));
    assert_eq!(wind_update(&mut pk, END_UPDATE).ret, Some(1));
    move_to(host, &mut pk, 320, 300, 0);
    remove_menu(&mut pk);
}

/// The GH1 order: the process the pick concerns (here an accessory asking for
/// the screen while the press is held) runs on only after the release — so its
/// `fm_own(TRUE)` then finds the button up and `set_mown` gives it the mouse
/// (`aes/geminput.c:220-235`).
#[test]
fn another_processs_beg_update_during_the_pick_waits_for_the_release() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let mut pa = Aespb::new();
    a.run();
    appl_init(&mut pa);
    kernel_context();
    let mut pk = Aespb::new();
    install_menu(&mut pk);
    move_to(host, &mut pk, ON_FILE.0, ON_FILE.1, 0);
    queue_pick_open(host, 6, 4);
    let (o, msg) = evnt_multi_desk(&mut pk, false);
    assert_eq!(o.ret.map(|w| w & MU_MESAG), Some(MU_MESAG));
    assert_eq!(msg[0], MN_SELECTED);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the selecting press is still held");

    a.run();
    let w = wind_update(&mut pa, BEG_UPDATE);
    assert_eq!(w.ret, Some(1));
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 0, "the accessory's BEG_UPDATE waited for the release");
    assert_eq!(test_update_lock(), (1, Some(a.slot)));
    assert_eq!(wind_update(&mut pa, END_UPDATE).ret, Some(1));
    kernel_context();
    move_to(host, &mut pk, 320, 300, 0);
    remove_menu(&mut pk);
}

// ── 8. the Control Manager is inert while another process holds the lock ──

/// While a process holds the screen, `ctlmgr` is blocked in its
/// `wm_update(BEG_UPDATE)` (`aes/gemctrl.c:463`): the pointer over the bar
/// opens nothing. The desktop's poll returns its timer.
#[test]
fn the_bar_is_not_tracked_while_another_process_holds_the_lock() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let mut pa = Aespb::new();
    a.run();
    appl_init(&mut pa);
    assert_eq!(wind_update(&mut pa, BEG_UPDATE).ret, Some(1));

    kernel_context();
    let mut pk = Aespb::new();
    install_menu(&mut pk);
    move_to(host, &mut pk, ON_FILE.0, ON_FILE.1, 0);
    let (o, _msg) = evnt_multi_desk(&mut pk, true);
    assert_eq!(
        o.ret,
        Some(MU_TIMER),
        "with the accessory holding the screen the bar must not be tracked; the poll returns its timer"
    );
    assert_eq!(test_update_lock(), (1, Some(a.slot)));
    move_to(host, &mut pk, 320, 300, 0);
    remove_menu(&mut pk);
    a.run();
    assert_eq!(wind_update(&mut pa, END_UPDATE).ret, Some(1));
}

// ── 9. the press that cancels a drop-down is the Control Manager's ────────

const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;

/// The application's window, `NAME | CLOSER | MOVER`, opened at `r`.
fn open_window(pb: &mut Aespb, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = 0x000B;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0).ret.expect("wind_create completes");
    assert!(h > 0, "wind_create: {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    assert!(pb.call(WIND_OPEN, 5, 1, 0).ret.is_some(), "wind_open completes");
    h
}

fn close_window(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    let _ = pb.call(WIND_CLOSE, 1, 1, 0);
    pb.intin[0] = h;
    let _ = pb.call(WIND_DELETE, 1, 1, 0);
}

/// A point in the work area of the window [`cancel_setup`] opens.
const IN_WORK: (i16, i16) = (300, 300);

/// The QED gesture (`measurements/measurement-2026-10-21-gh1-reg`, scenario
/// `qedd`): the pointer on *File* opens the drop-down, the pointer leaves it, and a
/// press in the top window's work area cancels it — `mn_do` in
/// `OUTSIDE_STATE` waits for a press and breaks on it (`aes/gemmnlib.c:
/// 300-307,355-361`). The press is still held (`hold` packets) when the
/// application's poll returns.
fn cancel_setup(host: &'static ScriptHost, pk: &mut Aespb, hold: usize) -> i16 {
    install_menu(pk);
    let h = open_window(pk, (200, 150, 400, 300));
    // Read the window's redraw message(s) first.
    let mut buf = Box::new([0i16; 8]);
    loop {
        pk.intin.fill(0);
        pk.intin[0] = MU_MESAG | MU_TIMER;
        pk.addrin.fill(0);
        pk.addrin[0] = buf.as_mut_ptr() as u64;
        let r = pk.call(EVNT_MULTI, 16, 7, 1).ret.expect("the message poll completes");
        if r & MU_MESAG == 0 {
            break;
        }
    }
    pk.addrin[0] = 0;
    move_to(host, pk, ON_FILE.0, ON_FILE.1, 0);
    // Drained by `ev_multi`'s entry; the pointer over the title with the
    // buttons up opens the drop-down (hover, `mchange` → `ctl_pd`).
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let (mut x, mut y) = ON_FILE;
    while (x, y) != IN_WORK {
        let dx = (IN_WORK.0 - x).clamp(-32, 32);
        let dy = (IN_WORK.1 - y).clamp(-32, 32);
        host.push(Packet { dx, dy, buttons: 0 });
        x += dx;
        y += dy;
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..hold {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..4 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    // QED's wait: messages, a press, a timer (here a 0-ms poll).
    let (o, _msg) = evnt_multi_desk(pk, true);
    assert!(o.ret.is_some(), "the application's poll completes");
    assert_eq!(tos_api::vdi::mouse_position(), IN_WORK, "the drop-down was cancelled in the work area");
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the cancelling press is still held");
    h
}

fn cancel_teardown(host: &'static ScriptHost, pk: &mut Aespb, h: i16) {
    move_to(host, pk, 320, 300, 0);
    close_window(pk, h);
    remove_menu(pk);
}

/// `ctlmgr` owns the cancelling press (`bchange` left it with `ctl_pd`: `if
/// (!gl_ctmown && …) gl_mowner = chkown();`, `aes/geminput.c:334-335`). After
/// `mn_do` its loop's `ev_multi` returns on the held button, `hctl_window` on
/// the top window finds no control point and ends in `ct_msgup(0, …)`'s
/// `while (button & 0x0001) dsptch();` — inside `wm_update(BEG_UPDATE)` …
/// `wm_update(END_UPDATE)` (`aes/gemctrl.c:81-102,182,314,463,484`). So the
/// application's `BEG_UPDATE` returns only with the button up, and its
/// `BEG_MCTRL` then gives it the mouse (`fm_own` → `ct_chgown(rlr, …)` →
/// `set_mown`, `aes/gemfmlib.c:93`, `aes/geminput.c:220-235`): its next
/// `MU_BUTTON` wait completes. In the GH1 regression the lock was free, the
/// application took it with the press held, `set_mown` refused, and the
/// application — holding the screen — never got the mouse back (QED hang).
#[test]
fn the_press_that_cancels_a_dropdown_holds_the_screen_until_released() {
    let (_g, host) = lock();
    let mut pk = Aespb::new();
    let h = cancel_setup(host, &mut pk, 8);
    assert_eq!(
        test_update_lock(),
        (1, Some(CTL_PD)),
        "the cancelling press must be the Control Manager's: ct_msgup(0) waits for the release \
         inside ctlmgr's BEG_UPDATE (aes/gemctrl.c:100-101,463,484)"
    );

    let w = wind_update(&mut pk, BEG_UPDATE);
    assert_eq!(w.ret, Some(1));
    assert_eq!(
        tos_api::vdi::mouse_buttons() & 1,
        0,
        "the application's BEG_UPDATE returned while the cancelling press was held"
    );
    assert_eq!(test_update_lock(), (1, Some(KERNEL_CONTEXT)));
    assert_eq!(wind_update(&mut pk, END_UPDATE).ret, Some(1));

    // QED's drag bracket: BEG_MCTRL, wait for the button to be up, END_MCTRL.
    assert_eq!(wind_update(&mut pk, BEG_MCTRL).ret, Some(1));
    let mut m = [0i16; 8];
    pk.intin.fill(0);
    pk.intin[0] = MU_BUTTON;
    pk.intin[1] = 1;
    pk.intin[2] = 1;
    pk.intin[3] = 0; // up
    pk.addrin.fill(0);
    pk.addrin[0] = m.as_mut_ptr() as u64;
    assert_eq!(
        pk.call(EVNT_MULTI, 16, 7, 1).ret,
        Some(MU_BUTTON),
        "inside its BEG_MCTRL the application owns the mouse"
    );
    pk.addrin[0] = 0;
    assert_eq!(wind_update(&mut pk, 2).ret, Some(1)); // END_MCTRL
    assert_eq!(test_update_lock(), (0, None));
    cancel_teardown(host, &mut pk, h);
}

/// The other order EmuTOS allows: the application re-arms a press wait while
/// the cancelling press is held and gets it from `abutton`'s ownership-blind
/// level test (`aes/geminput.c:549-551`, `docs/api/aes/menu.md` "The one
/// documented exception"), then asks for mouse control at once. Its
/// `BEG_MCTRL` waits behind the Control Manager's `ct_msgup` (`fm_own` →
/// `wm_update(BEG_UPDATE)`, `aes/gemfmlib.c:86`, `aes/gemwmlib.c:1785-1786`)
/// and takes the mouse with the button up; the wait for the release that
/// follows completes and keys reach it again.
#[test]
fn beg_mctrl_after_the_cancelling_press_waits_for_the_release() {
    let (_g, host) = lock();
    let mut pk = Aespb::new();
    let h = cancel_setup(host, &mut pk, 8);

    let mut m = [0i16; 8];
    let o = evnt_multi_button(&mut pk, 0, &mut m);
    assert_eq!(o.ret, Some(MU_BUTTON), "abutton's level test sees the held press");
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "…while it is still held");

    assert_eq!(wind_update(&mut pk, BEG_MCTRL).ret, Some(1));
    assert_eq!(
        tos_api::vdi::mouse_buttons() & 1,
        0,
        "BEG_MCTRL returned while the cancelling press was held: the Control Manager must hold \
         the screen in ct_msgup until the release (aes/gemctrl.c:100-101,484)"
    );
    assert_eq!(test_mctrl_count(), (1, Some(KERNEL_CONTEXT)));
    pk.intin.fill(0);
    pk.intin[0] = MU_BUTTON;
    pk.intin[1] = 1;
    pk.intin[2] = 1;
    pk.intin[3] = 0; // up
    pk.addrin.fill(0);
    pk.addrin[0] = m.as_mut_ptr() as u64;
    assert_eq!(pk.call(EVNT_MULTI, 16, 7, 1).ret, Some(MU_BUTTON), "the release wait completes");
    pk.addrin[0] = 0;
    assert_eq!(wind_update(&mut pk, 2).ret, Some(1)); // END_MCTRL
    assert_eq!(test_update_lock(), (0, None));
    cancel_teardown(host, &mut pk, h);
}
