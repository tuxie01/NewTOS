//! **What survives a program: nothing of its windows** — for the clients the
//! shell loop does not sweep after (A107, A110).
//!
//! EmuTOS sweeps the window table after every program the shell loop ran:
//! `sh_ldapp` → `dos_exec(PE_LOADGO, …)` returns → `if (gl_shgem) wm_new();`
//! (`references/emutos/aes/gemshlib.c:645,676-677`), and `wm_new`
//! (`aes/gemwmlib.c:1875-1893`) drains the `wind_update` locks, then closes
//! every open window and deletes every window in use, handles 1..NUM_WIN. A
//! program that crashed or never bothered to `wind_close`/`wind_delete`
//! leaves nothing behind for the next occupant of the AES.
//!
//! NewTOS's shell loop does that (`kernel/src/shell_loop.rs`). What it cannot
//! reach is an EL0 `Pexec` child that dies **inside** a living parent
//! (TeraDesk → QED): the tests here drive that case and ask, after the child is
//! gone, **is its property gone** — its windows out of the table, invisible to
//! `wind_find` and not one pixel in the whole-screen composite, its background
//! tree, its menu bar and its `wind_update` holds — while the parent's own
//! state is untouched?
//!
//! (Until DESK43 U10 this file also held the A96 tests of
//! `wm_new_keep_desktop`, the sweep of the nested launch from the built-in
//! desktop; that construct ended with DESK43 U3 and the function and its tests
//! went with U10.)
//!
//! Harness: the process fixture of `aes_pd_by_slot.rs`, a pixel-keeping
//! backend so the composite can be inspected.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::aes::object::Object;
use tos_api::bdos::process::{self, Abi, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_types::BasePage;

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend that keeps its pixels ─────────────────────────────────

/// The pseudo-palette the VDI dispatcher installs before a colour-sensitive
/// op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs`): EmuTOS' `videl_palette1`
/// for pens 0..15 as RGB555-in-565 words (`aes_window_content.rs`'s seed).
/// Seeded at construction so pen 0 (white) and pen 1 (black) are distinct
/// device values from the first fill on, whichever test runs first.
fn seed_palette() -> [u16; 256] {
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
    pal
}

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0xFFFF_FFFF; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new(seed_palette()),
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

    /// The device value of VDI pen `pel`.
    fn device(&self, pel: Pel) -> u32 {
        self.rgb565_to_device(self.pel_to_device16(pel))
    }

    /// How many pixels of `r` are **not** `v`.
    fn count_not(&self, r: (i16, i16, i16, i16), v: u32) -> usize {
        let px = self.px.lock().unwrap();
        let mut n = 0;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) && px[Self::idx(x, y)] != v {
                    n += 1;
                }
            }
        }
        n
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
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
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
        let v = self.device(pel);
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
        let fg = self.device(pel);
        let (ax, ay, bx, by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
        let put = |x: i16, y: i16| match mode {
            WriteMode::Xor => {
                let v = self.get_device_pixel(x, y);
                self.put(x, y, v ^ 0x00FF_FFFF, clip);
            }
            _ => self.put(x, y, fg, clip),
        };
        if ay == by {
            for x in ax.min(bx)..=ax.max(bx) {
                put(x, ay);
            }
        } else if ax == bx {
            for y in ay.min(by)..=ay.max(by) {
                put(ax, y);
            }
        } else {
            let (mut x, mut y) = (ax as i32, ay as i32);
            let (xe, ye) = (bx as i32, by as i32);
            let (dx, dy) = ((xe - x).abs(), -(ye - y).abs());
            let sy = if y < ye { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(x as i16, y as i16);
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
        }
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.device(fill.fg);
        let bg = self.device(fill.bg);
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
        let v = self.device(fg);
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
        let f = self.device(fg);
        let b = self.device(bg);
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

struct PixelHost {
    raster: BufBackend,
    clock: AtomicU64,
}

impl Host for PixelHost {
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
        let n = self.clock.fetch_add(1, Ordering::Relaxed);
        assert!(n < 1_000_000, "a trap is spinning");
    }
}

unsafe impl Sync for PixelHost {}
unsafe impl Send for PixelHost {}

static HOST_ONCE: Once = Once::new();
static HOST: Mutex<Option<&'static PixelHost>> = Mutex::new(None);

fn install_host() {
    HOST_ONCE.call_once(|| {
        let host: &'static PixelHost =
            Box::leak(Box::new(PixelHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        *HOST.lock().unwrap() = Some(host);
    });
}

fn host() -> &'static PixelHost {
    HOST.lock().unwrap().expect("host installed by lock()")
}

/// One lock over every process-wide static the tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    install_host();
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

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

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> i16 {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        let sw = unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
        // The call must complete. Since A112 a completed call may come back
        // with the `completed` Switch of the dispatch every AES call ends
        // with (another slot of this test is on the ready list); the host has
        // no trap handler to apply it, and the call's results are already
        // copied back. A mid-call yield (S2, not `completed`) is a failure.
        assert!(sw.is_none_or(|s| s.completed));
        self.intout[0]
    }
}

// AES opcodes used here.
const APPL_INIT: i16 = 10;
const APPL_EXIT: i16 = 19;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;
const WIND_SET: i16 = 105;
const WIND_FIND: i16 = 106;
const WIND_UPDATE: i16 = 107;
const MENU_BAR: i16 = 30;
const EVNT_MULTI: i16 = 25;

/// `evnt_multi` event-flag bits (`aesdefs.h`).
const MU_TIMER: i16 = 0x0020;

/// Window kinds (`aesdefs.h`).
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const MOVER: i16 = 0x0008;

/// `wind_set` field / `wind_update` mode (`aesdefs.h`).
const WF_NEWDESK: i16 = 14;
const BEG_UPDATE: i16 = 1;
const BEG_MCTRL: i16 = 3;

/// The desktop window handle.
const DESKWH: i16 = 0;

/// `G_BOX` (`obdefs.h`) and the `ob_spec` of a solid white box (`gr_crack`:
/// pattern 7 in bits 4..6, interior colour 0, thickness 0).
const G_BOX: u16 = 20;
const WHITE_SOLID: u64 = 0x70;

/// A107: the living EL0 parent's window and its `Pexec` child's — disjoint,
/// both clear of the menu row, and far enough apart that neither the border
/// nor the drop shadow of one reaches the other.
const A107_PARENT_WIN: (i16, i16, i16, i16) = (40, 60, 240, 160);
const A107_CHILD_WIN: (i16, i16, i16, i16) = (330, 260, 240, 160);

fn centre(r: (i16, i16, i16, i16)) -> (i16, i16) {
    (r.0 + r.2 / 2, r.1 + r.3 / 2)
}

/// `r` shrunk by `n` on every side.
fn inset(r: (i16, i16, i16, i16), n: i16) -> (i16, i16, i16, i16) {
    (r.0 + n, r.1 + n, r.2 - 2 * n, r.3 - 2 * n)
}

// ── AES calls ─────────────────────────────────────────────────────────────

fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0)
}

fn appl_exit(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_EXIT, 0, 1, 0)
}

/// `evnt_multi(MU_TIMER, …, ms)` — the wait a GEM parent returns to after its
/// `Pexec` child ended. One AES dispatch, so the pending per-slot sweep runs
/// before the opcode does (A107). Returns the fired event mask.
fn evnt_multi_timer(pb: &mut Aespb, ms: i16) -> i16 {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_TIMER;
    pb.intin[14] = ms;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    let fired = pb.call(EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    fired
}

/// `menu_bar(tree, show)`.
fn menu_bar(pb: &mut Aespb, tree: u64, show: i16) {
    pb.intin.fill(0);
    pb.intin[0] = show;
    pb.addrin[0] = tree;
    pb.call(MENU_BAR, 1, 1, 1);
    pb.addrin[0] = 0;
}

fn wind_create(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = NAME | CLOSER | MOVER;
    pb.intin[1] = 0;
    pb.intin[2] = 19;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16 - 19;
    let h = pb.call(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

fn wind_open(pb: &mut Aespb, h: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    assert_eq!(pb.call(WIND_OPEN, 5, 1, 0), 1, "wind_open failed");
}

fn wind_close(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_CLOSE, 1, 1, 0);
}

fn wind_delete(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_DELETE, 1, 1, 0);
}

fn wind_find(pb: &mut Aespb, x: i16, y: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = x;
    pb.intin[1] = y;
    pb.call(WIND_FIND, 2, 1, 0)
}

fn wind_update(pb: &mut Aespb, mode: i16) {
    pb.intin.fill(0);
    pb.intin[0] = mode;
    pb.call(WIND_UPDATE, 1, 1, 0);
}

/// `wind_set(DESKWH, WF_NEWDESK, tree, root)` in the native form the kernel
/// desktop sends (tree in `addr_in[0]`, root in `int_in[2]`); a null tree
/// un-registers.
fn wind_set_newdesk(pb: &mut Aespb, tree: u64, root: i16) {
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = root;
    pb.addrin[0] = tree;
    pb.call(WIND_SET, 3, 1, 1);
    pb.addrin[0] = 0;
}

// ── a process slot the host can "run" ─────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own (the
/// `aes_pd_by_slot.rs` fixture). Released on drop through the kernel's own
/// slot release, so a failing test leaves nothing behind.
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
}

impl Proc {
    fn new() -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot].basepage_kva.store(page.cast::<BasePage>(), Ordering::Release);
        Proc { slot, page }
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process: every AES call now carries its identity.
    fn run(&self) {
        process::set_current(self.bp());
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
        // SAFETY: single-threaded under `LOCK`; no frame snapshot is consumed
        // on the host. Releasing an already-released slot only re-zeroes it.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` above and is not aliased
        // once the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

/// The kernel boot context: no running process (the role the built-in desktop
/// had until DESK43 U10; here the parents' stand-in between steps).
fn desktop_context() {
    process::set_current(std::ptr::null_mut());
}

/// `Pterm` of an EL0 **child** whose parent is still alive: `parent_slot` is
/// the parent's, so `terminate` makes the parent the running process again —
/// EmuTOS `xterm`'s `run = run->p_parent` (`bdos/proc.c:609`) — exactly as
/// the kernel's `Pterm` handler leaves it before it `eret`s into the parent's
/// saved frame (A107).
fn pterm_child_of(child: &Proc, parent: &Proc) {
    PROCESS_TABLE[child.slot].parent_frame.elr().store(1, Ordering::Release);
    PROCESS_TABLE[child.slot].parent_slot.store(parent.slot as u32, Ordering::Release);
    // SAFETY: single-threaded under `LOCK`; `RUN` is the child's basepage.
    let resume = unsafe { process::pterm::terminate(0) }.expect("Pterm of the running child");
    assert_eq!(resume.parent_frame_slot, child.slot);
    // SAFETY: the frame snapshot is not needed on the host.
    unsafe { process::pterm::release_slot(child.slot) };
    assert_eq!(process::current(), parent.bp(), "the parent is the running process again");
}

// ── the launch boundary, as the kernel drives it ──────────────────────────

/// The desktop's own background tree: one white `G_BOX` over the screen, so
/// every pixel the composite paints as *desktop* is white and anything else
/// inside a rectangle is a window.
fn desktop_tree() -> Box<Object> {
    Box::new(Object {
        ob_next: -1,
        ob_head: -1,
        ob_tail: -1,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: WHITE_SOLID,
        ob_x: 0,
        ob_y: 0,
        ob_width: SCREEN_W as i16,
        ob_height: SCREEN_H as i16,
    })
}

/// Tear down what a test built, whether or not it passed.
fn cleanup(pb: &mut Aespb, desk_win: i16) {
    desktop_context();
    wind_close(pb, desk_win);
    wind_delete(pb, desk_win);
    wind_set_newdesk(pb, 0, 0);
    appl_exit(pb);
    tos_api::aes::window::wm_new();
}

// ── A107: the nested EL0 `Pexec` child of a living GEM parent ──────────────
//
// TeraDesk starts QED: `shel_write(SHW_EXEC, …)` then `x_exec` = `Pexec`
// (`sdk/apps/teradesk/startprg.c:259,309`), and TeraDesk stays alive around
// it. EmuTOS never has this shape — there a GEM program that starts another
// one *ends* and the shell loop runs the child, sweeping after it with
// `wm_new()` (`aes/gemshlib.c:676-677`). Our child dies with its parent
// parked in its own `Pexec`, so the only hook is
// `aes::release_client(slot)` from `pterm::release_slot` — the `Pterm`
// exception stack, where `close_window`'s repaint and `WM_REDRAW` fan-out
// must not run. The slot is therefore *marked* there and swept at the
// parent's next AES dispatch (`aes::dispatch` → `sweep_dead_clients`).

/// The parent has a window open and `Pexec`s a child that opens one and
/// `Pterm`s without `wind_close`. **At the `Pterm` nothing has been swept
/// yet** (the exception stack is not a drawing context) — the slot carries
/// the mark. The parent's next `evnt_multi` must find the child's window out
/// of the table, `wind_find` in its rectangle answering the desktop, its
/// rectangle painted as bare desktop, and its own window untouched.
#[test]
fn a107_a_window_the_pexec_child_left_open_is_gone_at_the_parents_next_aes_call() {
    let _g = lock();
    let mut pb = Aespb::new();
    let tree = desktop_tree();
    tos_api::aes::window::wm_new();

    // The desktop is in the AES with its background registered; it is not
    // part of this case — the launch chain is parent → child, both EL0.
    desktop_context();
    appl_init(&mut pb);
    wind_set_newdesk(&mut pb, &*tree as *const Object as u64, 0);
    appl_exit(&mut pb);

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);
    let pwin = wind_create(&mut pb);
    wind_open(&mut pb, pwin, A107_PARENT_WIN);

    // `Pexec`: the child takes its own slot and runs nested in the parent.
    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    let cwin = wind_create(&mut pb);
    wind_open(&mut pb, cwin, A107_CHILD_WIN);
    assert_eq!(tos_api::aes::window::test_window_owner(cwin), Some(child.slot));
    assert_eq!(wind_find(&mut pb, centre(A107_CHILD_WIN).0, centre(A107_CHILD_WIN).1), cwin);

    // The child ends without closing anything.
    pterm_child_of(&child, &parent);
    let at_pterm = (
        tos_api::aes::window::test_window_owner(cwin),
        tos_api::aes::test_sweep_pending(child.slot),
    );

    // The parent's next AES call.
    let fired = evnt_multi_timer(&mut pb, 2);

    let white = host().raster.device(0);
    let (cx, cy) = centre(A107_CHILD_WIN);
    let (px, py) = centre(A107_PARENT_WIN);
    let verdict = (
        tos_api::aes::window::test_window_owner(cwin),
        wind_find(&mut pb, cx, cy),
        host().raster.count_not(A107_CHILD_WIN, white),
        tos_api::aes::window::test_window_owner(pwin),
        wind_find(&mut pb, px, py),
        host().raster.count_not(inset(A107_PARENT_WIN, 1), white),
        tos_api::aes::test_sweep_pending(child.slot),
    );
    cleanup(&mut pb, pwin);

    assert_eq!(fired & MU_TIMER, MU_TIMER, "the parent's wait must end on its timer, fired = {fired:#06x}");
    assert_eq!(
        at_pterm,
        (Some(child.slot), true),
        "at the `Pterm` itself the child's window is still in the table and the slot is marked \
         (release_slot runs on the exception stack; the sweep draws)"
    );
    assert_eq!(
        verdict.0, None,
        "the dead child's window must be out of the table at the parent's next AES call \
         (wm_new, gemwmlib.c:1886-1890 via gemshlib.c:676-677)"
    );
    assert_eq!(verdict.1, DESKWH, "wind_find inside the dead child's window must answer the desktop");
    assert_eq!(
        verdict.2, 0,
        "closing the child's window must repaint its rectangle as bare desktop (draw_change, \
         gemwmlib.c:1216-1222): {} pixels are not",
        verdict.2
    );
    assert_eq!(verdict.3, Some(parent.slot), "the living parent's window must survive the sweep");
    assert_eq!(verdict.4, pwin, "wind_find inside the parent's window must still answer it");
    assert!(verdict.5 > 0, "the parent's window must still be painted");
    assert!(!verdict.6, "the mark must be consumed by the dispatch that swept");
}

/// The child registered its own `WF_NEWDESK` background tree (a desktop
/// replacement launched from TeraDesk). It is a VA of an address space that
/// is gone, so the sweep drops it — EmuTOS resets `gl_newdesk` before the
/// next program runs (`wm_init`, `gemwmlib.c:1300`). A tree the **parent**
/// registered is not touched when a child dies.
#[test]
fn a107_the_pexec_childs_background_tree_goes_with_it_and_the_parents_stays() {
    let _g = lock();
    let mut pb = Aespb::new();
    let parent_tree = desktop_tree();
    let child_tree = desktop_tree();
    tos_api::aes::window::wm_new();

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);
    wind_set_newdesk(&mut pb, &*parent_tree as *const Object as u64, 0);

    // (a) A child that registers nothing leaves the parent's registration be.
    let quiet = Proc::new();
    quiet.run();
    appl_init(&mut pb);
    pterm_child_of(&quiet, &parent);
    evnt_multi_timer(&mut pb, 2);
    let after_quiet = tos_api::aes::window::test_newdesk_tree();

    // (b) A child that registers its own takes it with it.
    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    wind_set_newdesk(&mut pb, &*child_tree as *const Object as u64, 0);
    let registered = tos_api::aes::window::test_newdesk_tree();
    pterm_child_of(&child, &parent);
    evnt_multi_timer(&mut pb, 2);
    let after_child = tos_api::aes::window::test_newdesk_tree();

    wind_set_newdesk(&mut pb, 0, 0);
    appl_exit(&mut pb);
    desktop_context();
    tos_api::aes::window::wm_new();

    assert_eq!(
        after_quiet,
        &*parent_tree as *const Object as u64,
        "the living parent's WF_NEWDESK tree must survive a child's death"
    );
    assert_eq!(registered, &*child_tree as *const Object as u64, "the child registered its own tree");
    assert_eq!(after_child, 0, "the dead child's WF_NEWDESK tree must be dropped by the sweep");
}

/// The child installed a menu bar and died with it installed. EmuTOS clears
/// `gl_mntree` nowhere on the exit path — `mn_cleanup` (`gemmnlib.c:740-760`,
/// called from `ap_exit` `gemaplib.c:254` and `sh_ldapp` `gemshlib.c:652`)
/// only sends `AC_CLOSE` and frees the submenu array, and `wm_new`/`wm_init`
/// do not touch it either; the next program's `menu_bar(tree, 1)` overwrites
/// it. Here the pointer is an EL0 VA the Control Manager would dereference at
/// EL1 under the parent's `TTBR0` (the A97 class of fault), so the NewTOS
/// rule is that a bar dies with the client that installed it — and only with
/// **that** client: the parent's bar is untouched by a child's death.
#[test]
fn a107_the_pexec_childs_menu_bar_goes_with_it_and_the_parents_stays() {
    let _g = lock();
    let mut pb = Aespb::new();
    tos_api::aes::window::wm_new();
    let parent_menu = menu_tree();
    let child_menu = menu_tree();

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);
    menu_bar(&mut pb, parent_menu.as_ptr() as u64, 1);
    assert_eq!(tos_api::aes::menu::active() as u64, parent_menu.as_ptr() as u64);

    // (a) A child that installs no bar leaves the parent's alone.
    let quiet = Proc::new();
    quiet.run();
    appl_init(&mut pb);
    pterm_child_of(&quiet, &parent);
    evnt_multi_timer(&mut pb, 2);
    let after_quiet = tos_api::aes::menu::active() as u64;

    // (b) A child that installs its own takes it with it.
    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    menu_bar(&mut pb, child_menu.as_ptr() as u64, 1);
    let registered = tos_api::aes::menu::active() as u64;
    pterm_child_of(&child, &parent);
    evnt_multi_timer(&mut pb, 2);
    let after_child = tos_api::aes::menu::active() as u64;

    menu_bar(&mut pb, 0, 0);
    appl_exit(&mut pb);
    desktop_context();
    tos_api::aes::window::wm_new();

    assert_eq!(after_quiet, parent_menu.as_ptr() as u64, "the living parent's menu bar must survive a child's death");
    assert_eq!(registered, child_menu.as_ptr() as u64, "the child installed its own bar");
    assert_eq!(after_child, 0, "the dead child's menu tree must be dropped by the sweep");
}

/// A three-object menu tree: ROOT, `THEBAR` (the white strip `menu_bar`
/// draws and stretches to the screen) and `THEACTIVE` (the title box whose
/// extent becomes `gl_ctwait`, `gemmnlib.c:717`). No titles — nothing here
/// tracks the bar.
fn menu_tree() -> Box<[Object; 3]> {
    let mk = |otype: u16, next: i16, head: i16, tail: i16, x: i16, w: i16| Object {
        ob_next: next,
        ob_head: head,
        ob_tail: tail,
        ob_type: otype,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: WHITE_SOLID,
        ob_x: x,
        ob_y: 0,
        ob_width: w,
        ob_height: 19,
    };
    const G_IBOX: u16 = 25;
    Box::new([
        mk(G_IBOX, -1, 1, 1, 0, SCREEN_W as i16),
        mk(G_BOX, 0, 2, 2, 0, SCREEN_W as i16),
        mk(G_IBOX, 1, -1, -1, 16, 216),
    ])
}

// ── A110 — the two `wind_update` counters carry their owner ────────────────
//
// EmuTOS's screen lock has one: `wind_spb.sy_owner`, written by `tak_flag`
// (`aes/gemflag.c:77-82`), cleared by `unsync` on the 1 → 0 edge (`:125-129`),
// and read in exactly one place — `sh_ldapp` takes the lock off a program that
// has ended only `if (wind_spb.sy_owner == rlr)` (`aes/gemshlib.c:668-669`).
// `ml_ocnt` has none (`aes/gemfmlib.c:51`): the only force-drain of it is
// `wm_new`'s (`aes/gemwmlib.c:1879-1883`), which runs when no other client is
// alive. Our per-slot sweep (A107) runs while the parent *is* alive, so both
// counters carry the slot that took them and the sweep drains only that slot's.

/// The parent holds the screen lock; a child is started, takes nothing and
/// dies. `sh_ldapp`'s owner test (`gemshlib.c:668-669`) fails for the child,
/// so the parent's `BEG_UPDATE` must still be in force after the sweep — a
/// blanket drain would unlock the parent's screen in the middle of its own
/// redraw.
#[test]
fn a110_a_living_clients_screen_lock_survives_a_childs_death() {
    let _g = lock();
    let mut pb = Aespb::new();
    tos_api::aes::window::wm_new();

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);
    wind_update(&mut pb, BEG_UPDATE);
    wind_update(&mut pb, BEG_UPDATE); // nested, as a redraw inside a redraw
    let held = tos_api::aes::window::test_update_lock();

    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    pterm_child_of(&child, &parent);
    evnt_multi_timer(&mut pb, 2); // the parent's next AES call: the sweep
    let after = tos_api::aes::window::test_update_lock();

    appl_exit(&mut pb);
    desktop_context();
    tos_api::aes::window::wm_new();

    assert_eq!(held, (2, Some(parent.slot)), "the parent's two BEG_UPDATEs, owner recorded");
    assert_eq!(
        after,
        (2, Some(parent.slot)),
        "a child's death must not touch a lock the living parent owns \
         (gemshlib.c:668-669 releases only `sy_owner == rlr`)"
    );
}

/// The **child** holds the screen lock and dies without an `END_UPDATE`.
/// `sh_ldapp`'s owner test matches, so the lock goes — count and owner —
/// and the parent can take it again from zero.
#[test]
fn a110_a_dead_clients_screen_lock_is_released() {
    let _g = lock();
    let mut pb = Aespb::new();
    tos_api::aes::window::wm_new();

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);

    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    wind_update(&mut pb, BEG_UPDATE);
    wind_update(&mut pb, BEG_UPDATE);
    let held = tos_api::aes::window::test_update_lock();

    pterm_child_of(&child, &parent);
    let at_pterm = tos_api::aes::window::test_update_lock();
    evnt_multi_timer(&mut pb, 2); // the parent's next AES call: the sweep
    let after = tos_api::aes::window::test_update_lock();

    // The screen is usable again: the parent's own take starts a fresh count
    // and puts the parent in the owner field.
    wind_update(&mut pb, BEG_UPDATE);
    let retaken = tos_api::aes::window::test_update_lock();

    appl_exit(&mut pb);
    desktop_context();
    tos_api::aes::window::wm_new();

    assert_eq!(held, (2, Some(child.slot)), "the child's two BEG_UPDATEs, owner recorded");
    assert_eq!(
        at_pterm,
        (2, Some(child.slot)),
        "the `Pterm` itself releases nothing — `release_slot` runs on the exception stack (A107)"
    );
    assert_eq!(
        after,
        (0, None),
        "the dead child's screen lock must be drained at the parent's next AES call \
         (gemshlib.c:668-669 + gemwmlib.c:1882-1883)"
    );
    assert_eq!(retaken, (1, Some(parent.slot)), "and the parent can take the lock again");
}

/// The child dies holding `BEG_MCTRL`. `wm_new` drains `ml_ocnt` **through**
/// `wm_update(END_MCTRL)` (`gemwmlib.c:1880-1881`), so `fm_own`'s 1 → 0 edge
/// body runs: `gl_mntree = ml_mnhold` (`gemfmlib.c:103`) puts the parent's
/// menu bar back. Zeroing the counter instead would leave the bar parked for
/// the rest of the parent's life — the observable "the screen is usable
/// again". The `BEG_UPDATE` `fm_own` took with it (`:86`) comes off too.
#[test]
fn a110_a_dead_clients_mouse_control_hold_is_released_and_unparks_the_menu() {
    let _g = lock();
    let mut pb = Aespb::new();
    tos_api::aes::window::wm_new();
    let parent_menu = menu_tree();

    let parent = Proc::new();
    parent.run();
    appl_init(&mut pb);
    menu_bar(&mut pb, parent_menu.as_ptr() as u64, 1);

    let child = Proc::new();
    child.run();
    appl_init(&mut pb);
    wind_update(&mut pb, BEG_MCTRL);
    let held = (
        tos_api::aes::window::test_mctrl_count(),
        tos_api::aes::window::test_update_lock(),
        tos_api::aes::menu::active() as u64,
    );

    pterm_child_of(&child, &parent);
    evnt_multi_timer(&mut pb, 2); // the parent's next AES call: the sweep
    let after = (
        tos_api::aes::window::test_mctrl_count(),
        tos_api::aes::window::test_update_lock(),
        tos_api::aes::menu::active() as u64,
    );

    menu_bar(&mut pb, 0, 0);
    appl_exit(&mut pb);
    desktop_context();
    tos_api::aes::window::wm_new();

    assert_eq!(held.0, (1, Some(child.slot)), "the child's BEG_MCTRL, owner recorded");
    assert_eq!(held.1, (1, Some(child.slot)), "fm_own's own BEG_UPDATE (gemfmlib.c:86)");
    assert_eq!(held.2, 0, "while the child owns the mouse the menu tree is parked (gemfmlib.c:89-90)");
    assert_eq!(after.0, (0, None), "the dead child's mouse-control hold must be drained");
    assert_eq!(after.1, (0, None), "and the BEG_UPDATE fm_own took with it");
    assert_eq!(
        after.2,
        parent_menu.as_ptr() as u64,
        "the drain must run fm_own's 1 -> 0 edge, which puts the parent's menu bar back \
         (gemfmlib.c:103)"
    );
}
