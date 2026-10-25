//! A117 — **`MN_SELECTED` goes to the owner of the menu bar (`gl_mnppd`), not
//! to the process whose wait the Control Manager happened to run in.**
//!
//! Spec: `docs/api/aes/menu.md` "`MN_SELECTED` message (the payload)", NewTOS
//! binding (A117); `docs/api/aes/_state.md` `gl_mnppd`. EmuTOS: `hctl_rect`
//! picks `owner = gl_mnppd` for every pick that is not a desk-accessory entry
//! (`aes/gemctrl.c:353`) and posts through `ct_msgup(mesag, owner, …)`
//! (`:365`) → `ap_sendmsg` (`:83-84`); `gl_mnppd` is the descriptor of the
//! process that called `menu_bar(tree, 1)` (`aes/gemmnlib.c:707,712`). The
//! Control Manager is its own process there (`ctl_pd`), so *where* it runs never
//! decides the recipient.
//!
//! The defect (`measurements/measurement-2026-10-25-a117/RUNINFO.TXT`): NewTOS
//! runs the Control Manager inline in every client's `evnt_multi(MU_MESAG)`
//! and posted the pick to that client. A desk accessory idling in
//! `evnt_multi(MU_MESAG)` that happened to run the track received the
//! desktop's *Desk → Desktop info…* as `MN_SELECTED`; the desktop saw nothing.
//!
//! `gl_mnppd` is a *descriptor*: the application chain is one (`D.g_int[0]`,
//! the desktop, every launched program and every `Pexec` child,
//! `aes/geminit.c:851-877`, `gemshlib.c:572`), each accessory another. Cases:
//!
//! 1. bar of the chain (kernel context), pick tracked in an accessory's wait
//!    → the chain receives it, the accessory does not;
//! 2. bar of a chain program, pick tracked in an accessory's wait while the
//!    program's `Pexec` child runs → the child (the chain's running member)
//!    receives it;
//! 3. bar of a chain program, pick tracked in its own `Pexec` child's wait →
//!    the child receives it (one chain descriptor; unchanged by A117).
//!
//! Harness copied from `aes_gh1_update_mutex.rs` (raster stand-in, scripted
//! pointer advancing in `Host::pump_input`, process slots, menu tree).

#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::{self, sched, Abi, State, MAX_PROCESSES, PROCESS_TABLE};
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

use tos_api::aes::window::test_update_lock;

/// `ctl_pd` as the recorded holder of the lock (`window.rs` `CTL_PD`).
const CTL_PD: usize = usize::MAX - 1;
/// The kernel-context client's identity (`pd::KERNEL_CONTEXT`).
const KERNEL_CONTEXT: usize = MAX_PROCESSES;

// ── 1. a second process's BEG_UPDATE parks until the holder lets go ───────


// ── A117 helpers ──────────────────────────────────────────────────────────

impl Proc {
    /// A program the kernel launcher started (`launch.rs`: named, then
    /// `State::Running`) — a live member of the application chain.
    fn program(path: &[u8]) -> Self {
        let p = Proc::new(false);
        tos_api::aes::register_process(p.slot, path);
        PROCESS_TABLE[p.slot].set_state(State::Running);
        p
    }

    /// A started `Pexec` child of `parent` (`handle_pexec`: `parent_slot`,
    /// `State::Running`) — the same chain descriptor.
    fn child_of(parent: &Proc) -> Self {
        let c = Proc::new(false);
        PROCESS_TABLE[c.slot].parent_slot.store(parent.slot as u32, Ordering::Release);
        PROCESS_TABLE[c.slot].set_state(State::Running);
        c
    }
}

/// An accessory's idle wait: `evnt_multi(MU_MESAG | MU_TIMER)` with a 0-ms
/// timer (one tick, `adelay`), so the call returns once the pick is over —
/// with `MU_MESAG` only if a message reached the accessory's own queue.
fn evnt_multi_acc(pb: &mut Aespb) -> (Outcome, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_TIMER;
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    let o = pb.call(EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    (o, *buf)
}

/// The Screen Manager's pid in `msg[1]` (`ap_sendmsg` as `ctl_pd`,
/// `aes/gemwmlib.c:855`).
const SCRENMGR_PID: i16 = 1;

fn assert_pick(o: &Outcome, msg: &[i16; 8], who: &str) {
    assert_eq!(
        o.ret.map(|w| w & MU_MESAG),
        Some(MU_MESAG),
        "{who}: the bar owner must receive the pick as a message (aes/gemctrl.c:353,365)"
    );
    assert_eq!((msg[0], msg[1], msg[3], msg[4]), (MN_SELECTED, SCRENMGR_PID, TITLE_FILE, ITEM_OPEN), "{who}");
}

// ── 1. the chain's bar, picked while an accessory's wait runs the track ──

#[test]
fn a_pick_tracked_in_an_accessorys_wait_goes_to_the_bar_owner() {
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

    // The accessory is the one whose wait runs: the inline Control Manager
    // tracks the drop-down there.
    a.run();
    let (o, msg) = evnt_multi_acc(&mut pa);
    assert_eq!(
        o.ret,
        Some(MU_TIMER),
        "the accessory does not own the bar: its wait must not return the pick \
         (got msg {msg:?}; EmuTOS owner = gl_mnppd, aes/gemctrl.c:353)"
    );
    assert_ne!(msg[0], MN_SELECTED);

    // The bar owner (the chain, here the kernel-context desktop) has it.
    kernel_context();
    move_to(host, &mut pk, 320, 300, 0);
    let (o, msg) = evnt_multi_desk(&mut pk, true);
    assert_pick(&o, &msg, "kernel-context desktop");
    assert_eq!(test_update_lock(), (0, None));
    remove_menu(&mut pk);
}

// ── 2. a program's bar, picked in an accessory's wait, the program in Pexec

#[test]
fn a_pick_tracked_in_an_accessorys_wait_reaches_the_chains_running_member() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let mut pa = Aespb::new();
    a.run();
    appl_init(&mut pa);

    let p = Proc::program(b"C:\\OWNER.NAP");
    let mut pp = Aespb::new();
    p.run();
    appl_init(&mut pp);
    install_menu(&mut pp);
    // The program is suspended in a synchronous `Pexec`; its GEM child runs
    // on the same chain descriptor (EmuTOS `D.g_int[0]`).
    let c = Proc::child_of(&p);
    let mut pc = Aespb::new();
    c.run();
    appl_init(&mut pc);
    move_to(host, &mut pc, ON_FILE.0, ON_FILE.1, 0);
    queue_pick_open(host, 6, 4);

    a.run();
    let (o, msg) = evnt_multi_acc(&mut pa);
    assert_eq!(o.ret, Some(MU_TIMER), "the accessory must not receive the program's pick (msg {msg:?})");

    c.run();
    move_to(host, &mut pc, 320, 300, 0);
    let (o, msg) = evnt_multi_desk(&mut pc, true);
    assert_pick(&o, &msg, "the chain's running member");

    p.run();
    move_to(host, &mut pp, 320, 300, 0);
    remove_menu(&mut pp);
    drop(c);
    drop(p);
    kernel_context();
}

// ── 3. a program's bar, picked in its own Pexec child's wait ─────────────

#[test]
fn a_pick_tracked_in_a_chain_childs_wait_stays_with_the_chain() {
    let (_g, host) = lock();
    let p = Proc::program(b"C:\\OWNER.NAP");
    let mut pp = Aespb::new();
    p.run();
    appl_init(&mut pp);
    install_menu(&mut pp);
    let c = Proc::child_of(&p);
    let mut pc = Aespb::new();
    c.run();
    appl_init(&mut pc);
    move_to(host, &mut pc, ON_FILE.0, ON_FILE.1, 0);
    queue_pick_open(host, 6, 4);

    let (o, msg) = evnt_multi_desk(&mut pc, false);
    assert_pick(&o, &msg, "the Pexec child of the bar owner");

    move_to(host, &mut pc, 320, 300, 0);
    p.run();
    remove_menu(&mut pp);
    drop(c);
    drop(p);
    kernel_context();
}
