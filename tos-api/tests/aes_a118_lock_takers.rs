//! A118 — **the remaining takers of the window-tree lock**, and the owner gate
//! of `ev_multi`'s quick check.
//!
//! Spec: `docs/api/aes/window-graphics.md` `wind_update (107)` § "The refusal
//! parks the taker — a real mutex (GH1)", "The AES's own takers (A118)";
//! `docs/api/aes/event.md` `ev_multi` point 3; `docs/api/aes/application.md`
//! `appl_exit`; `docs/api/aes/resource-fileselector.md` `fsel_input`.
//!
//! EmuTOS takes `wm_update(BEG_UPDATE)` — and so waits behind another
//! process's hold (`tak_flag` → `ev_block(MU_MUTEX, …)`,
//! `aes/gemwmlib.c:1785-1786`) — inside `wm_open` (`aes/gemwmlib.c:1422,1429`),
//! `wm_close` (`:1450,1456`, also behind `wm_delete` of an open window,
//! `:1473-1474`), `wm_set` (`:1638,1757`), `gr_rubwind`/`gr_dragbox` behind
//! `graf_rubberbox`/`graf_dragbox` (`aes/gemgrlib.c:205,219,250,267`),
//! `ap_exit` (`aes/gemaplib.c:253,258`) and `fm_do`'s `fm_own(TRUE)` in the
//! file selector (`aes/gemfslib.c:718` → `aes/gemfmlib.c:287`). The quick
//! check of `ev_multi` runs only for the mouse owner itself (`if (rlr ==
//! gl_mowner)`, `aes/gemevlib.c:185`).
//!
//! Harness: `aes_gh1_update_mutex.rs`'s (process slots the host "runs", the
//! S2 re-issue of a call that yielded, input only in `pump_input`), plus a key
//! queue that hands out keys once the mouse script is exhausted.

#![allow(dead_code)]
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
    /// Keys handed out by `console_try_read_key`, only once the mouse script
    /// is exhausted — so a modal loop sees every queued packet first.
    keys: Mutex<VecDeque<u16>>,
}

const IDLE_CAP: usize = 20_000;

impl ScriptHost {
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn pending(&self) -> usize {
        self.script.lock().unwrap().len()
    }
    fn key(&self, k: u16) {
        self.keys.lock().unwrap().push_back(k);
    }
    fn clear(&self) {
        self.keys.lock().unwrap().clear();
        self.script.lock().unwrap().clear();
        self.idle.store(0, Ordering::Relaxed);
    }
}

impl Host for ScriptHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        if self.pending() != 0 {
            return None;
        }
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
            keys: Mutex::new(VecDeque::new()),
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

use tos_api::aes::window::test_update_lock;

const APPL_EXIT: i16 = 19;
const MENU_REGISTER: i16 = 35;
const GRAF_RUBBOX: i16 = 70;
const GRAF_DRAGBOX: i16 = 71;
const FSEL_INPUT: i16 = 90;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;
const WIND_SET: i16 = 105;

const WF_NAME: i16 = 2;
const WF_DCOLOR: i16 = 19;
const AC_CLOSE: i16 = 41;

/// The kernel-context client's identity (`pd::KERNEL_CONTEXT`).
const KERNEL_CONTEXT: usize = MAX_PROCESSES;

fn wind_create(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = 0x000B; // NAME | CLOSER | MOVER
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0).ret.expect("wind_create completes");
    assert!(h > 0, "wind_create: {h}");
    h
}

fn wind_open(pb: &mut Aespb, h: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = 100;
    pb.intin[2] = 100;
    pb.intin[3] = 200;
    pb.intin[4] = 150;
    pb.call(WIND_OPEN, 5, 1, 0)
}

fn wind_close(pb: &mut Aespb, h: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_CLOSE, 1, 1, 0)
}

fn wind_delete(pb: &mut Aespb, h: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_DELETE, 1, 1, 0)
}

/// `wind_set(h, field, …)` with the remaining words zero.
fn wind_set(pb: &mut Aespb, h: i16, field: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = field;
    if field == WF_DCOLOR {
        pb.intin[2] = 0; // W_BOX
        pb.intin[3] = -1;
        pb.intin[4] = -1;
    }
    pb.addrin.fill(0);
    pb.call(WIND_SET, 6, 1, 1)
}

/// An accessory that holds the window-tree lock and is ready to run, and a
/// process of the application chain that will ask for the lock: the GH1 pair
/// (`aes_gh1_update_mutex.rs` case 1).
struct Pair {
    a: Proc,
    b: Proc,
    pa: Aespb,
    pbb: Aespb,
}

fn pair() -> Pair {
    let a = Proc::new(true);
    let b = Proc::new(false);
    let (mut pa, mut pbb) = (Aespb::new(), Aespb::new());
    a.run();
    appl_init(&mut pa);
    b.run();
    appl_init(&mut pbb);
    Pair { a, b, pa, pbb }
}

impl Pair {
    /// The accessory takes the lock and goes back on the ready list.
    fn a_holds(&mut self) {
        self.a.run();
        assert_eq!(wind_update(&mut self.pa, BEG_UPDATE).ret, Some(1));
        assert_eq!(test_update_lock(), (1, Some(self.a.slot)), "the accessory holds the lock");
        self.a.ready();
        self.b.run();
    }

    /// `o` — the chain's call — must have parked behind the accessory's hold.
    fn assert_parked(&self, o: &Outcome, what: &str) {
        assert_eq!(o.ret, None, "{what} must not return while another process holds the lock");
        assert_eq!(o.switch_to, Some(self.a.slot), "{what} parks and gives the CPU to the holder");
        assert!(!self.b.is_ready(), "{what}: the caller is off the ready list (WAITIN on sy_wait)");
        assert_eq!(test_update_lock(), (1, Some(self.a.slot)), "{what}: the refused take counts nothing");
    }

    /// The accessory lets go: the lock goes to the parked chain process.
    fn a_releases(&mut self) {
        self.a.run();
        assert_eq!(wind_update(&mut self.pa, END_UPDATE).ret, Some(1));
        assert_eq!(test_update_lock(), (1, Some(self.b.slot)), "unsync hands the lock to the waiter");
        self.b.run();
    }

    /// The re-executed call completes and gives back the lock it was handed.
    fn b_completes(&mut self, what: &str) -> i16 {
        let r = self.pbb.trap();
        let ret = r.ret.unwrap_or_else(|| panic!("{what} completes once the lock is its own"));
        assert_eq!(test_update_lock(), (0, None), "{what} gives the lock back (END_UPDATE)");
        ret
    }
}

// ── 1. wm_open ─────────────────────────────────────────────────────────────

/// `wm_open`'s `wm_update(BEG_UPDATE)` (`aes/gemwmlib.c:1422`): a second
/// process's `wind_open` waits until the holder lets go.
#[test]
fn wind_open_waits_for_another_processs_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    let o = wind_open(&mut p.pbb, h);
    p.assert_parked(&o, "wind_open");
    p.a_releases();
    assert_eq!(p.b_completes("wind_open"), 1);
    assert_eq!(wind_close(&mut p.pbb, h).ret, Some(1));
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

/// `wm_open` refuses before the lock (`aes/gemwmlib.c:1415-1419`): an unknown
/// handle and an open window return FALSE at once, also while it is held.
#[test]
fn a_refused_wind_open_does_not_wait() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    p.a_holds();
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(0), "an open window: FALSE, no wait");
    assert_eq!(wind_open(&mut p.pbb, 30).ret, Some(0), "an unknown handle: FALSE, no wait");
    p.a.run();
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

/// The caller's own hold nests (`tak_flag`, `sy_owner == rlr`): `wind_open`
/// inside its own `BEG_UPDATE` returns, and the count is back at 1.
#[test]
fn wind_open_nests_inside_the_callers_own_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    assert_eq!(wind_update(&mut p.pbb, BEG_UPDATE).ret, Some(1));
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(p.b.slot)), "wm_open's own pair nests and comes off");
    assert_eq!(wind_update(&mut p.pbb, END_UPDATE).ret, Some(1));
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

// ── 2. wm_close, wm_delete ─────────────────────────────────────────────────

/// `wm_close`'s `wm_update(BEG_UPDATE)` (`aes/gemwmlib.c:1450`).
#[test]
fn wind_close_waits_for_another_processs_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    p.a_holds();
    let o = wind_close(&mut p.pbb, h);
    p.assert_parked(&o, "wind_close");
    p.a_releases();
    assert_eq!(p.b_completes("wind_close"), 1);
    assert_eq!(wind_close(&mut p.pbb, h).ret, Some(0), "closed: the close has happened");
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

/// `wm_close` refuses a window that is not open before the lock
/// (`aes/gemwmlib.c:1444-1447`).
#[test]
fn a_refused_wind_close_does_not_wait() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    assert_eq!(wind_close(&mut p.pbb, h).ret, Some(0), "not open: FALSE, no wait");
    p.a.run();
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

/// `wm_delete` of an open window closes it first through `wm_close`
/// (`aes/gemwmlib.c:1473-1474`), and so waits; of a closed one it does not.
#[test]
fn wind_delete_of_an_open_window_waits_for_another_processs_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let open = wind_create(&mut p.pbb);
    let closed = wind_create(&mut p.pbb);
    assert_eq!(wind_open(&mut p.pbb, open).ret, Some(1));
    p.a_holds();
    assert_eq!(wind_delete(&mut p.pbb, closed).ret, Some(1), "a closed window is freed without the lock");
    let o = wind_delete(&mut p.pbb, open);
    p.assert_parked(&o, "wind_delete of an open window");
    p.a_releases();
    assert_eq!(p.b_completes("wind_delete"), 1);
}

// ── 3. wm_set ──────────────────────────────────────────────────────────────

/// `wm_set`'s `wm_update(BEG_UPDATE)` (`aes/gemwmlib.c:1638`), reached for a
/// valid handle — and for `WF_DCOLOR` with any handle (`:1632-1636`).
#[test]
fn wind_set_waits_for_another_processs_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    let o = wind_set(&mut p.pbb, h, WF_NAME);
    p.assert_parked(&o, "wind_set(WF_NAME)");
    p.a_releases();
    assert_eq!(p.b_completes("wind_set(WF_NAME)"), 1);

    p.a_holds();
    let o = wind_set(&mut p.pbb, -1, WF_DCOLOR);
    p.assert_parked(&o, "wind_set(-1, WF_DCOLOR)");
    p.a_releases();
    assert_eq!(p.b_completes("wind_set(WF_DCOLOR)"), 1);

    p.a_holds();
    assert_eq!(wind_set(&mut p.pbb, 30, WF_NAME).ret, Some(0), "an unknown handle: FALSE before the lock");
    p.a.run();
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

// ── 4. gr_rubwind / gr_dragbox ─────────────────────────────────────────────

/// `graf_rubberbox` → `gr_rubbox` → `gr_rubwind`'s `wm_update(BEG_UPDATE)`
/// (`aes/gemgrlib.c:205`) and `graf_dragbox` → `gr_dragbox`'s (`:250`).
#[test]
fn graf_rubberbox_and_graf_dragbox_wait_for_another_processs_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.a_holds();
    p.pbb.intin.fill(0);
    p.pbb.intin[0] = 300;
    p.pbb.intin[1] = 280;
    p.pbb.intin[2] = 10;
    p.pbb.intin[3] = 10;
    let o = p.pbb.call(GRAF_RUBBOX, 4, 3, 0);
    p.assert_parked(&o, "graf_rubberbox");
    p.a_releases();
    assert_eq!(p.b_completes("graf_rubberbox"), 1);

    p.a_holds();
    p.pbb.intin.fill(0);
    p.pbb.intin[0] = 40;
    p.pbb.intin[1] = 30;
    p.pbb.intin[2] = 300;
    p.pbb.intin[3] = 280;
    p.pbb.intin[6] = SCREEN_W as i16;
    p.pbb.intin[7] = SCREEN_H as i16;
    let o = p.pbb.call(GRAF_DRAGBOX, 8, 3, 0);
    p.assert_parked(&o, "graf_dragbox");
    p.a_releases();
    assert_eq!(p.b_completes("graf_dragbox"), 1);
}

// ── 5. ap_exit ─────────────────────────────────────────────────────────────

/// `ap_exit` holds the lock across `mn_cleanup` and `wait_for_accs(AP_ACCLOSE)`
/// (`aes/gemaplib.c:253-258`): an accessory that answers its `AC_CLOSE` with a
/// `BEG_UPDATE` waits until the exiting program's `END_UPDATE` hands it over.
#[test]
fn appl_exit_holds_the_lock_while_the_accessories_see_ac_close() {
    let (_g, _host) = lock();
    let mut p = pair();
    static TITLE: &[u8] = b"  A118\0";
    p.a.run();
    p.pa.intin.fill(0);
    p.pa.intin[0] = p.pa.global[2];
    p.pa.addrin.fill(0);
    p.pa.addrin[0] = TITLE.as_ptr() as u64;
    assert!(p.pa.call(MENU_REGISTER, 1, 1, 1).ret.is_some_and(|m| m >= 0), "menu_register");
    p.pa.addrin[0] = 0;
    p.a.ready();

    p.b.run();
    p.pbb.intin.fill(0);
    let o = p.pbb.call(APPL_EXIT, 0, 1, 0);
    assert_eq!(o.ret, None, "appl_exit waits for the accessory to see AC_CLOSE");
    assert_eq!(o.switch_to, Some(p.a.slot));
    assert_eq!(
        test_update_lock(),
        (1, Some(p.b.slot)),
        "ap_exit's wm_update(BEG_UPDATE) is held across wait_for_accs (aes/gemaplib.c:253-255)"
    );

    p.a.run();
    let mut msg = Box::new([0i16; 8]);
    p.pa.intin.fill(0);
    p.pa.intin[0] = MU_MESAG;
    p.pa.addrin.fill(0);
    p.pa.addrin[0] = msg.as_mut_ptr() as u64;
    assert_eq!(p.pa.call(EVNT_MULTI, 16, 7, 1).ret, Some(MU_MESAG));
    assert_eq!(msg[0], AC_CLOSE);
    p.pa.addrin[0] = 0;
    let w = wind_update(&mut p.pa, BEG_UPDATE);
    assert_eq!(w.ret, None, "the accessory's BEG_UPDATE waits for the exiting program");
    assert_eq!(w.switch_to, Some(p.b.slot));

    p.b.run();
    let r = p.pbb.trap();
    assert_eq!(r.ret, Some(1), "appl_exit completes");
    assert_eq!(test_update_lock(), (1, Some(p.a.slot)), "its END_UPDATE hands the lock to the accessory");

    p.a.run();
    assert_eq!(p.pa.trap().ret, Some(1), "the accessory's BEG_UPDATE returns");
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

/// `appl_exit` by one process while another holds the lock waits before it
/// does anything — `ap_exit` opens with `wm_update(BEG_UPDATE)`
/// (`aes/gemaplib.c:253`), so no `AC_CLOSE` goes out yet.
#[test]
fn appl_exit_waits_for_another_processs_hold_before_ac_close() {
    let (_g, _host) = lock();
    let mut p = pair();
    static TITLE: &[u8] = b"  A118\0";
    p.a.run();
    p.pa.intin.fill(0);
    p.pa.intin[0] = p.pa.global[2];
    p.pa.addrin.fill(0);
    p.pa.addrin[0] = TITLE.as_ptr() as u64;
    assert!(p.pa.call(MENU_REGISTER, 1, 1, 1).ret.is_some_and(|m| m >= 0), "menu_register");
    p.pa.addrin[0] = 0;
    p.a_holds();
    p.pbb.intin.fill(0);
    let o = p.pbb.call(APPL_EXIT, 0, 1, 0);
    p.assert_parked(&o, "appl_exit");

    // The accessory polls its pipe: nothing there yet.
    p.a.run();
    let mut msg = Box::new([0i16; 8]);
    p.pa.intin.fill(0);
    p.pa.intin[0] = MU_MESAG | MU_TIMER;
    p.pa.addrin.fill(0);
    p.pa.addrin[0] = msg.as_mut_ptr() as u64;
    assert_eq!(p.pa.call(EVNT_MULTI, 16, 7, 1).ret, Some(MU_TIMER), "no AC_CLOSE before ap_exit has the lock");
    // Answer the AC_CLOSE ap_exit posts once it runs: the accessory releases,
    // the exiting program gets the lock, posts, and waits for the accessory.
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (1, Some(p.b.slot)));
    p.a.ready();
    p.b.run();
    let o = p.pbb.trap();
    assert_eq!(o.switch_to, Some(p.a.slot), "appl_exit now waits for AC_CLOSE to be seen");
    p.a.run();
    p.pa.intin[0] = MU_MESAG;
    assert_eq!(p.pa.call(EVNT_MULTI, 16, 7, 1).ret, Some(MU_MESAG));
    assert_eq!(msg[0], AC_CLOSE);
    // …and is not runnable again (back in its message wait, which the host
    // stands in for by taking it off the ready list).
    p.pa.addrin[0] = 0;
    sched::unready(p.a.slot);
    p.b.run();
    assert_eq!(p.pbb.trap().ret, Some(1), "appl_exit completes");
    assert_eq!(test_update_lock(), (0, None));
}

// ── 6. the file selector's fm_do ───────────────────────────────────────────

const KEY_RETURN: u16 = 0x1C0D;

/// `fsel_input` from a desk accessory runs its dialog through `fm_do`, whose
/// `fm_own(TRUE)` (`aes/gemfslib.c:718` → `aes/gemfmlib.c:287,82-96`) waits
/// for the lock and gives the accessory the mouse. A press while the selector
/// is up therefore goes to the accessory, not to the desktop's parked
/// `MU_BUTTON` (the GH1 symptom, for the file selector).
#[test]
fn fsel_input_of_an_accessory_takes_the_mouse_from_the_desktop() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let mut pa = Aespb::new();
    a.run();
    appl_init(&mut pa);
    a.ready();

    // The desktop (the kernel-context chain) waits for a press, 200 ms.
    kernel_context();
    let mut pk = Aespb::new();
    let mut mk = Box::new([0i16; 8]);
    pk.intin.fill(0);
    pk.intin[0] = MU_BUTTON | MU_TIMER;
    pk.intin[1] = 1;
    pk.intin[2] = 1;
    pk.intin[3] = 1;
    pk.intin[14] = 200;
    pk.addrin.fill(0);
    pk.addrin[0] = mk.as_mut_ptr() as u64;
    let w = pk.call(EVNT_MULTI, 16, 7, 1);
    assert_eq!(w.ret, None, "the desktop's wait parks");
    assert_eq!(w.switch_to, Some(a.slot));

    // The accessory's file selector: a click outside the dialog, then Return.
    a.run();
    let mut path = Box::new([0u8; 128]);
    path[..6].copy_from_slice(b"C:\\*.*");
    let mut sel = Box::new([0u8; 32]);
    host.push(Packet { dx: -300, dy: 170, buttons: 0 });
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    host.key(KEY_RETURN);
    pa.intin.fill(0);
    pa.addrin.fill(0);
    pa.addrin[0] = path.as_mut_ptr() as u64;
    pa.addrin[1] = sel.as_mut_ptr() as u64;
    let f = pa.call(FSEL_INPUT, 0, 2, 2);
    assert_eq!(f.ret, Some(1), "fsel_input completes");
    assert_eq!(pa.intout[1], 1, "Return = OK");
    assert_eq!(test_update_lock(), (0, None), "fm_own(FALSE) gave the lock back");

    // The accessory is not runnable now (back in a message wait, which the
    // host stands in for by taking it off the ready list); the desktop runs.
    sched::unready(a.slot);
    kernel_context();
    let r = pk.trap();
    assert_eq!(
        r.ret,
        Some(MU_TIMER),
        "the press in the accessory's file selector must not complete the desktop's MU_BUTTON: \
         fm_own(TRUE) made the accessory the mouse owner (aes/gemfmlib.c:93, aes/geminput.c:463-483)"
    );
    pk.addrin[0] = 0;
    drop(a);
    kernel_context();
    move_to(host, &mut pk, 320, 300, 0);
}

/// …and it waits for another process's hold before the dialog is drawn.
#[test]
fn fsel_input_waits_for_another_processs_hold() {
    let (_g, host) = lock();
    let mut p = pair();
    p.a_holds();
    let mut path = Box::new([0u8; 128]);
    path[..6].copy_from_slice(b"C:\\*.*");
    let mut sel = Box::new([0u8; 32]);
    p.pbb.intin.fill(0);
    p.pbb.addrin.fill(0);
    p.pbb.addrin[0] = path.as_mut_ptr() as u64;
    p.pbb.addrin[1] = sel.as_mut_ptr() as u64;
    let o = p.pbb.call(FSEL_INPUT, 0, 2, 2);
    p.assert_parked(&o, "fsel_input");
    p.a_releases();
    host.key(KEY_RETURN);
    assert_eq!(p.b_completes("fsel_input"), 1);
    p.pbb.addrin.fill(0);
}

// ── 7. ev_multi's quick check is the caller's, not any application's ──────

/// `if (rlr == gl_mowner)` (`aes/gemevlib.c:185`): an accessory's
/// `evnt_multi(MU_BUTTON | MU_TIMER, 0 ms)` while the desktop owns the mouse
/// and the button is down runs no button quick check; the 0-ms timer fires in
/// its own quick check (`:216-220`), so nothing is armed and the call returns
/// `MU_TIMER` alone.
#[test]
fn the_button_quick_check_is_the_mouse_owners_only() {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let mut pa = Aespb::new();
    a.run();
    appl_init(&mut pa);

    // The press on the bare desktop makes the desktop the owner
    // (`bchange` → `chkown` → `D.w_win[0].w_owner`, `aes/geminput.c:310-335`).
    kernel_context();
    let mut pk = Aespb::new();
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    graf_mkstate(&mut pk);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1);

    a.run();
    let mut msg = Box::new([0i16; 8]);
    let o = evnt_multi_button(&mut pa, MU_TIMER, &mut msg);
    assert_eq!(
        o.ret,
        Some(MU_TIMER),
        "the accessory does not own the mouse: no button quick check (aes/gemevlib.c:185)"
    );

    // The owner's own quick check does see it.
    kernel_context();
    let mut mk = Box::new([0i16; 8]);
    let o = evnt_multi_button(&mut pk, MU_TIMER, &mut mk);
    assert_eq!(o.ret, Some(MU_BUTTON | MU_TIMER), "the owner's quick check fires");
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    graf_mkstate(&mut pk);
}

// ── 8. a hand-over to a call that is refused on re-execution (Tester, A118 rework) ──
//
// The three cases are the Tester's (`measurements/measurement-2026-10-25-a118-test/method/tester/
// aes_tester_extra-appended-tests.rs.txt`): B parks on a window call, the holder changes the window
// so that B's re-executed call is refused, and ends its hold. EmuTOS has made its checks before the
// wait (`aes/gemwmlib.c:1415-1422,1443-1450,1632-1638`); the woken process is the holder
// (`unsync`, `aes/gemflag.c:117-124`) and ends its hold with its own `END_UPDATE`. The lock must
// not stay with B.

#[test]
fn t_wind_set_parked_then_window_deleted_does_not_leak_lock() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    let o = wind_set(&mut p.pbb, h, WF_NAME);
    p.assert_parked(&o, "wind_set");
    p.a.run();
    assert_eq!(wind_delete(&mut p.pa, h).ret, Some(1));
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    let r = p.pbb.trap();
    assert_eq!(r.ret, Some(0), "the deleted window: FALSE");
    assert_eq!(test_update_lock(), (0, None), "lock leaked to B after refusal on re-execution");
}

#[test]
fn t_wind_open_parked_then_window_opened_by_other_does_not_leak_lock() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    let o = wind_open(&mut p.pbb, h);
    p.assert_parked(&o, "wind_open");
    p.a.run();
    assert_eq!(wind_open(&mut p.pa, h).ret, Some(1)); // A's own hold nests
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    let r = p.pbb.trap();
    assert_eq!(r.ret, Some(0), "already open: FALSE");
    assert_eq!(test_update_lock(), (0, None), "lock leaked to B after refusal on re-execution");
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

#[test]
fn t_wind_close_parked_then_window_closed_by_other_does_not_leak_lock() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    p.a_holds();
    let o = wind_close(&mut p.pbb, h);
    p.assert_parked(&o, "wind_close");
    p.a.run();
    assert_eq!(wind_close(&mut p.pa, h).ret, Some(1));
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    let r = p.pbb.trap();
    assert_eq!(r.ret, Some(0), "not open any more: FALSE");
    assert_eq!(test_update_lock(), (0, None), "lock leaked to B after refusal on re-execution");
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
}

/// The refused re-execution hands the lock on to the next waiter, as B's own `END_UPDATE` would.
#[test]
fn a_refused_re_execution_hands_the_lock_to_the_next_waiter() {
    let (_g, _host) = lock();
    let mut p = pair();
    let c = Proc::new(true); // a second accessory, waiting behind B
    let mut pc = Aespb::new();
    c.run();
    appl_init(&mut pc);
    sched::unready(c.slot); // not runnable until it asks for the lock below
    p.b.run();
    let h = wind_create(&mut p.pbb);
    p.a_holds();
    let o = wind_set(&mut p.pbb, h, WF_NAME);
    p.assert_parked(&o, "wind_set");
    c.run();
    assert_eq!(wind_update(&mut pc, BEG_UPDATE).ret, None, "C waits behind A and B");
    p.a.run();
    assert_eq!(wind_delete(&mut p.pa, h).ret, Some(1));
    assert_eq!(wind_update(&mut p.pa, END_UPDATE).ret, Some(1));
    p.b.run();
    assert_eq!(p.pbb.trap().ret, Some(0));
    assert_eq!(test_update_lock(), (1, Some(c.slot)), "B's refused call passes the lock on to C");
    c.run();
    assert_eq!(pc.trap().ret, Some(1));
    assert_eq!(wind_update(&mut pc, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 9. the brackets nest in the caller's own hold (each taker takes, not only releases) ──

/// Inside its own `BEG_UPDATE` every taker's `wm_update` pair nests (`tak_flag`, `sy_owner ==
/// rlr`) and leaves the count where it was: a body that only ended the lock would drop it to 0.
#[test]
fn every_taker_nests_inside_the_callers_own_hold() {
    let (_g, _host) = lock();
    let mut p = pair();
    p.b.run();
    let h = wind_create(&mut p.pbb);
    assert_eq!(wind_update(&mut p.pbb, BEG_UPDATE).ret, Some(1));
    let held = (1, Some(p.b.slot));
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    assert_eq!(test_update_lock(), held, "wind_open");
    assert_eq!(wind_set(&mut p.pbb, h, WF_NAME).ret, Some(1));
    assert_eq!(test_update_lock(), held, "wind_set");
    assert_eq!(wind_close(&mut p.pbb, h).ret, Some(1));
    assert_eq!(test_update_lock(), held, "wind_close");
    assert_eq!(wind_open(&mut p.pbb, h).ret, Some(1));
    assert_eq!(wind_delete(&mut p.pbb, h).ret, Some(1));
    assert_eq!(test_update_lock(), held, "wind_delete of an open window");
    p.pbb.intin.fill(0);
    p.pbb.intin[0] = 300;
    p.pbb.intin[1] = 280;
    p.pbb.intin[2] = 10;
    p.pbb.intin[3] = 10;
    assert_eq!(p.pbb.call(GRAF_RUBBOX, 4, 3, 0).ret, Some(1));
    assert_eq!(test_update_lock(), held, "graf_rubberbox");
    p.pbb.intin.fill(0);
    p.pbb.intin[0] = 40;
    p.pbb.intin[1] = 30;
    p.pbb.intin[2] = 300;
    p.pbb.intin[3] = 280;
    p.pbb.intin[6] = SCREEN_W as i16;
    p.pbb.intin[7] = SCREEN_H as i16;
    assert_eq!(p.pbb.call(GRAF_DRAGBOX, 8, 3, 0).ret, Some(1));
    assert_eq!(test_update_lock(), held, "graf_dragbox");
    assert_eq!(wind_update(&mut p.pbb, END_UPDATE).ret, Some(1));
    assert_eq!(test_update_lock(), (0, None));
}

// ── 10. a parked rectangle wait is completed only for the mouse owner ──────

const EVNT_MOUSE: i16 = 22;
const MU_M1: i16 = 0x0004;
/// A rectangle right of the parked pointer (320, 300).
const RECT: (i16, i16, i16, i16) = (380, 290, 20, 20);

/// The accessory parks a rectangle wait (pointer outside), the desktop owns the mouse and moves it
/// into the rectangle, the accessory's wait is re-executed: only `post_mouse(gl_mowner, …)`
/// completes a parked wait (`aes/geminput.c:390,446-456`), so it must stay parked.
fn parked_rect_wait_of_a_non_owner(call: impl Fn(&mut Aespb) -> Outcome) {
    let (_g, host) = lock();
    let a = Proc::new(true);
    let b = Proc::new(false);
    let (mut pa, mut pbb) = (Aespb::new(), Aespb::new());
    a.run();
    appl_init(&mut pa);
    b.run();
    appl_init(&mut pbb);
    b.ready();
    a.run();
    let o = call(&mut pa);
    assert_eq!(o.ret, None, "pointer outside: the wait parks");
    assert_eq!(o.switch_to, Some(b.slot));

    b.run();
    move_to(host, &mut pbb, RECT.0 + 5, RECT.1 + 5, 0);
    b.ready();
    a.run();
    let r = pa.trap();
    assert_eq!(
        r.ret, None,
        "the accessory does not own the mouse: its parked rectangle wait is not completed (rlr == gl_mowner)"
    );
    assert_eq!(r.switch_to, Some(b.slot));
    drop(a);
    drop(b);
    kernel_context();
    let mut pk = Aespb::new();
    move_to(host, &mut pk, 320, 300, 0);
}

#[test]
fn evnt_multi_parked_rectangle_is_the_mouse_owners_only() {
    parked_rect_wait_of_a_non_owner(|pa| {
        pa.intin.fill(0);
        pa.intin[0] = MU_M1;
        pa.intin[4] = 0; // enter
        (pa.intin[5], pa.intin[6], pa.intin[7], pa.intin[8]) = RECT;
        pa.addrin.fill(0);
        pa.call(EVNT_MULTI, 16, 7, 1)
    });
}

#[test]
fn evnt_mouse_parked_rectangle_is_the_mouse_owners_only() {
    parked_rect_wait_of_a_non_owner(|pa| {
        pa.intin.fill(0);
        pa.intin[0] = 0; // enter
        (pa.intin[1], pa.intin[2], pa.intin[3], pa.intin[4]) = RECT;
        pa.call(EVNT_MOUSE, 5, 5, 0)
    });
}
