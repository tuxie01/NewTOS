//! KEYOWNER — **a key goes to the process that owns the mouse.**
//!
//! Spec: `docs/api/aes/event.md` § "Who gets a key — the mouse owner
//! (`gl_mowner`)". EmuTOS's `kchange` posts every key to the mouse owner,
//! `post_keybd(gl_mowner->p_cda, ch)` (`aes/geminput.c:288-296`), and a wait
//! reads only its own queue — `ev_multi`'s `rlr->p_cda->c_q`
//! (`aes/gemevlib.c:173-182`), `akbin` (`aes/geminput.c:486-493`). The owner
//! is the top window's owner (`w_setactive`, `aes/gemwmlib.c:322-334`), the
//! caller during `wind_update(BEG_MCTRL)` / `form_do` (`fm_own`,
//! `aes/gemfmlib.c:82-107`), and the application chain is one owner
//! (`aes/geminit.c:851-877`) while each desk accessory is one of its own.
//!
//! The measured defect (`measurements/measurement-2026-10-08-sysinfo2/`,
//! RUNINFO "Esc"): the SYSACC accessory tracks its drop-down inside
//! `BEG_MCTRL` and waits in `evnt_multi(MU_KEYBD | …)`; while it sleeps the
//! built-in desktop's own `evnt_multi(MU_KEYBD | …)` ran and took Esc / Return
//! / the cursor keys out of the one key ring — first poller wins.
//!
//! What is pinned, at the AES trap boundary, with a registered accessory and
//! the kernel-context desktop (or a launched chain program) as the two clients:
//!
//! 1. inside the accessory's `BEG_MCTRL` the keys are the accessory's — the
//!    desktop's `evnt_multi` does not get them, the accessory's does, and after
//!    `END_MCTRL` they are the desktop's again;
//! 2. normal typing goes to the owner of the top window (a chain program), not
//!    to an idle accessory that also waits for `MU_KEYBD`;
//! 3. an accessory window on top makes the accessory the key owner;
//! 4. an accessory's modal dialog (`form_alert`, the kernel's `fm_do` loop)
//!    reads its keys, and the keys go back to the desktop afterwards;
//! 5. desktop shortcuts (Alt-C) reach the desktop with an idle accessory loaded;
//! 6. `evnt_keybd` follows the same owner.
//!
//! Harness shape follows `aes_pd_by_slot.rs` (silent raster, process slots).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, Abi, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_types::BasePage;

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend that accepts every draw and keeps no pixels ───────────

struct SinkBackend;

impl RasterBackend for SinkBackend {
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
    fn set_active_palette(&self, _palette: &[u32; 256]) {}
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        pel as u16
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
    fn abline(
        &self,
        _x0: i16,
        _y0: i16,
        _x1: i16,
        _y1: i16,
        _pel: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        mask
    }
    fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _f: PatternFill, _c: ClipRect) {}
    fn draw_glyph_sized(
        &self,
        _x: i16,
        _y: i16,
        _glyph: &[u8],
        _cell_w: i16,
        _cell_h: i16,
        _fg: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
    ) {
    }
    fn blit_mono(&self, _d: Rect, _s: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _c: ClipRect) {}
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host: one key ring, as the kernel's IKBD ring ─────────────────────

/// A host with no pixels and **one** key ring — the shared IKBD ring the
/// kernel host reads `console_try_read_key` from. Whoever reads first takes
/// the key; the AES decides who may read.
struct KeyHost {
    raster: SinkBackend,
    clock: AtomicU64,
    keys: Mutex<VecDeque<u16>>,
}

impl Host for KeyHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read(&self) -> Option<u8> {
        self.console_try_read_key().map(|w| w as u8)
    }
    fn console_try_read_key(&self) -> Option<u16> {
        self.keys.lock().unwrap().pop_front()
    }
    fn console_peek_key(&self) -> Option<u16> {
        self.keys.lock().unwrap().front().copied()
    }
    fn console_has_input(&self) -> bool {
        !self.keys.lock().unwrap().is_empty()
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        let n = self.clock.fetch_add(1, Ordering::Relaxed);
        assert!(n < 10_000_000, "a trap is spinning");
    }
}

unsafe impl Sync for KeyHost {}
unsafe impl Send for KeyHost {}

static HOST_ONCE: Once = Once::new();
static HOST: Mutex<Option<&'static KeyHost>> = Mutex::new(None);

fn install_host() {
    HOST_ONCE.call_once(|| {
        let host: &'static KeyHost = Box::leak(Box::new(KeyHost {
            raster: SinkBackend,
            clock: AtomicU64::new(0),
            keys: Mutex::new(VecDeque::new()),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        *HOST.lock().unwrap() = Some(host);
    });
}

fn host() -> &'static KeyHost {
    HOST.lock().unwrap().expect("host installed by lock()")
}

/// Type `key` (`scancode << 8 | ascii`) into the ring.
fn type_key(key: u16) {
    host().keys.lock().unwrap().push_back(key);
}

/// Keys still in the ring, oldest first.
fn ring() -> Vec<u16> {
    host().keys.lock().unwrap().iter().copied().collect()
}

static LOCK: Mutex<()> = Mutex::new(());

/// Serialise on the process-wide statics and start from a clean AES state:
/// no window, no `wind_update` hold, an empty ring, the desktop running.
fn lock() -> MutexGuard<'static, ()> {
    install_host();
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    desktop_context();
    tos_api::aes::window::wm_new();
    host().keys.lock().unwrap().clear();
    g
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
        assert!(sw.is_none_or(|s| s.completed), "the call parked (it must fire on its first pass here)");
        self.intout[0]
    }
}

const EVNT_KEYBD: i16 = 20;
const EVNT_MULTI: i16 = 25;
const FORM_ALERT: i16 = 52;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;
const WIND_SET: i16 = 105;
const WIND_UPDATE: i16 = 107;

const MU_KEYBD: i16 = 0x0001;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;

const END_MCTRL: i16 = 2;
const BEG_MCTRL: i16 = 3;

const WF_NEWDESK: i16 = 14;
const DESKWH: i16 = 0;
const NAME: i16 = 0x0001;
const MOVER: i16 = 0x0008;

// Key words, `scancode << 8 | ascii`.
const K_ESC: u16 = 0x011B;
const K_RETURN: u16 = 0x1C0D;
const K_UP: u16 = 0x4800;
const K_DOWN: u16 = 0x5000;
const K_ALT_C: u16 = 0x2E00;
const K_A: u16 = 0x1E61;
const K_B: u16 = 0x3062;

/// `evnt_multi(MU_KEYBD | MU_TIMER, 0 ms)` — one poll: `MU_KEYBD` and the key
/// when the caller may take one, else `MU_TIMER` alone (a 0-ms timer is a poll,
/// `aes/gemevlib.c:216-220`). With `MU_MESAG` too when `mesag` is set (the
/// desktop's and an idle accessory's wait).
fn poll_key(pb: &mut Aespb, mesag: bool) -> Option<u16> {
    let mut msg = [0i16; 8];
    pb.intin.fill(0);
    pb.intin[0] = MU_KEYBD | MU_TIMER | if mesag { MU_MESAG } else { 0 };
    pb.addrin.fill(0);
    pb.addrin[0] = msg.as_mut_ptr() as u64;
    let which = pb.call(EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    (which & MU_KEYBD != 0).then_some(pb.intout[5] as u16)
}

fn wind_update(pb: &mut Aespb, mode: i16) {
    pb.intin.fill(0);
    pb.intin[0] = mode;
    assert_eq!(pb.call(WIND_UPDATE, 1, 1, 0), 1);
}

fn evnt_keybd(pb: &mut Aespb) -> u16 {
    pb.intin.fill(0);
    pb.call(EVNT_KEYBD, 0, 1, 0) as u16
}

fn form_alert(pb: &mut Aespb, s: &[u8], defbut: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = defbut;
    pb.addrin.fill(0);
    pb.addrin[0] = s.as_ptr() as u64;
    let r = pb.call(FORM_ALERT, 1, 1, 1);
    pb.addrin[0] = 0;
    r
}

/// `wind_create` + `wind_open` a window at `(x, y)`; it becomes the top window.
fn open_window(pb: &mut Aespb, x: i16, y: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = NAME | MOVER;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create: {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = x;
    pb.intin[2] = y;
    pb.intin[3] = 200;
    pb.intin[4] = 120;
    assert_eq!(pb.call(WIND_OPEN, 5, 1, 0), 1);
    h
}

fn close_window(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_CLOSE, 1, 1, 0);
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_DELETE, 1, 1, 0);
}

/// `wind_set(0, WF_NEWDESK, NULL, 0)` from the running client — the desktop's
/// registration, which records it as the desktop window's owner
/// (`gemwmlib.c:1684-1687`).
fn newdesk(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_NEWDESK;
    pb.addrin.fill(0);
    pb.call(WIND_SET, 6, 1, 1);
}

// ── process slots ─────────────────────────────────────────────────────────

/// A claimed GEMDOS slot with a basepage page of its own; released on drop.
/// `acc` registers it as a desk accessory (`acc::register_process`, the
/// loader's `D.g_acc[]` slot) and forgets it again on drop.
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
    acc: bool,
}

impl Proc {
    fn new(acc: bool) -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot]
            .basepage_kva
            .store(page.cast::<BasePage>(), Ordering::Release);
        if acc {
            tos_api::aes::acc::register_process(slot);
        }
        Proc { slot, page, acc }
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
        if self.acc {
            tos_api::aes::acc::forget_process(self.slot);
        }
        // SAFETY: single-threaded under `LOCK`.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` and the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

/// The kernel boot context: no running process — the kernel-context client
/// (`pd::KERNEL_CONTEXT`).
fn desktop_context() {
    process::set_current(std::ptr::null_mut());
}

// ── 1. the BEG_MCTRL holder gets the keys ─────────────────────────────────

/// The SYSINFO02 drop-down: the accessory takes the mouse with
/// `wind_update(BEG_MCTRL)` and waits for `MU_KEYBD | MU_BUTTON | MU_M1`; the
/// desktop waits in its own `evnt_multi(MU_KEYBD | MU_BUTTON | MU_MESAG)`.
/// Esc, Return and the cursor keys are the accessory's (`fm_own` →
/// `ct_chgown(rlr, …)`, `aes/gemfmlib.c:93`; `kchange` → the owner,
/// `aes/geminput.c:288-296`). Before KEYOWNER the desktop's poll took them.
#[test]
fn keys_go_to_the_accessory_inside_its_beg_mctrl() {
    let _g = lock();
    let mut pb = Aespb::new();
    newdesk(&mut pb);
    let acc = Proc::new(true);

    acc.run();
    wind_update(&mut pb, BEG_MCTRL);

    for key in [K_DOWN, K_UP, K_RETURN, K_ESC] {
        type_key(key);
        desktop_context();
        assert_eq!(
            poll_key(&mut pb, true),
            None,
            "key {key:#06x}: the desktop must not take a key while the accessory holds \
             BEG_MCTRL (gl_mowner = the accessory, aes/gemfmlib.c:93; kchange → \
             post_keybd(gl_mowner->p_cda), aes/geminput.c:288-296)"
        );
        assert_eq!(ring(), vec![key], "the key stays for its owner");
        acc.run();
        assert_eq!(poll_key(&mut pb, false), Some(key), "key {key:#06x}: the accessory's own wait gets it");
        assert!(ring().is_empty());
    }

    acc.run();
    wind_update(&mut pb, END_MCTRL);

    // END_MCTRL hands the mouse back (`ct_chgown(ml_pmown, …)`, gemfmlib.c:102):
    // typing is the desktop's again, and the idle accessory gets nothing.
    type_key(K_A);
    acc.run();
    assert_eq!(poll_key(&mut pb, true), None, "after END_MCTRL the idle accessory owns no key");
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), Some(K_A), "after END_MCTRL the desktop types again");
}

/// `evnt_keybd` is `akbin` on the caller's queue (`aes/geminput.c:486-493`):
/// the BEG_MCTRL holder's `evnt_keybd` gets the key the desktop may not take.
#[test]
fn evnt_keybd_follows_the_owner() {
    let _g = lock();
    let mut pb = Aespb::new();
    newdesk(&mut pb);
    let acc = Proc::new(true);

    acc.run();
    wind_update(&mut pb, BEG_MCTRL);
    type_key(K_ESC);
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), None, "not the desktop's");
    acc.run();
    assert_eq!(evnt_keybd(&mut pb), K_ESC, "evnt_keybd of the owner returns the key in RET_CODE");
    wind_update(&mut pb, END_MCTRL);

    type_key(K_B);
    desktop_context();
    assert_eq!(evnt_keybd(&mut pb), K_B, "the desktop's evnt_keybd after END_MCTRL");
}

// ── 2. normal typing goes to the top window's owner ───────────────────────

/// A launched program (a chain member, e.g. QED) with its window on top; an
/// accessory loaded and waiting for keys too. Typing is the program's
/// (`w_setactive`, `aes/gemwmlib.c:322-334`) — the accessory gets nothing.
/// Before KEYOWNER whichever of the two polled first took the key.
#[test]
fn typing_goes_to_the_top_window_owner_not_to_an_idle_accessory() {
    let _g = lock();
    let mut pb = Aespb::new();
    let acc = Proc::new(true);
    let app = Proc::new(false);

    app.run();
    let wh = open_window(&mut pb, 40, 60);

    for key in [K_A, K_B, K_RETURN] {
        type_key(key);
        acc.run();
        assert_eq!(
            poll_key(&mut pb, true),
            None,
            "key {key:#06x}: an accessory that owns no window and holds no BEG_MCTRL takes no key"
        );
        app.run();
        assert_eq!(poll_key(&mut pb, true), Some(key), "key {key:#06x}: the top window's owner types");
    }

    app.run();
    close_window(&mut pb, wh);
    // No window open: the desktop window's owner (the chain) still types.
    type_key(K_A);
    acc.run();
    assert_eq!(poll_key(&mut pb, true), None);
    app.run();
    assert_eq!(poll_key(&mut pb, true), Some(K_A), "no window: the chain owns the keys");
}

// ── 3. an accessory window on top ─────────────────────────────────────────

/// An accessory's window on top makes the accessory the key owner
/// (`w_setactive` → `D.w_win[w_top()].w_owner`); closing it gives the keys
/// back to the desktop.
#[test]
fn an_accessory_window_on_top_gets_the_keys() {
    let _g = lock();
    let mut pb = Aespb::new();
    newdesk(&mut pb);
    let acc = Proc::new(true);

    acc.run();
    let wh = open_window(&mut pb, 100, 100);

    type_key(K_A);
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), None, "the accessory's window is on top: not the desktop's key");
    acc.run();
    assert_eq!(poll_key(&mut pb, true), Some(K_A), "the top window's owner is the accessory");

    close_window(&mut pb, wh);
    type_key(K_B);
    acc.run();
    assert_eq!(poll_key(&mut pb, true), None, "its window closed: the accessory owns no key");
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), Some(K_B), "back to the desktop window's owner");
}

// ── 4. an accessory's modal dialog ────────────────────────────────────────

/// An accessory's `form_alert` (the kernel's `fm_do` loop, `fm_own(TRUE)`)
/// reads its own keys — Return fires the default button — and afterwards the
/// desktop types again (`fm_own(FALSE)` → `ml_pmown`).
#[test]
fn an_accessory_dialog_reads_its_keys_and_gives_them_back() {
    let _g = lock();
    let mut pb = Aespb::new();
    newdesk(&mut pb);
    let acc = Proc::new(true);

    acc.run();
    type_key(K_RETURN);
    assert_eq!(form_alert(&mut pb, b"[1][Save the settings?][Save|Cancel]\0", 1), 1, "Return = default");
    assert!(ring().is_empty(), "the dialog took its Return");

    type_key(K_A);
    acc.run();
    assert_eq!(poll_key(&mut pb, true), None, "dialog over: the accessory owns no key");
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), Some(K_A), "dialog over: the desktop types");
}

// ── 5. desktop shortcuts ──────────────────────────────────────────────────

/// The desktop's keyboard shortcuts (Alt-C, Esc, Return) with an idle
/// accessory loaded: the desktop gets every one, the accessory none.
#[test]
fn desktop_shortcuts_reach_the_desktop_with_an_accessory_loaded() {
    let _g = lock();
    let mut pb = Aespb::new();
    newdesk(&mut pb);
    let acc = Proc::new(true);

    for key in [K_ALT_C, K_ESC, K_RETURN] {
        type_key(key);
        acc.run();
        assert_eq!(poll_key(&mut pb, true), None, "key {key:#06x}: not the idle accessory's");
        desktop_context();
        assert_eq!(poll_key(&mut pb, true), Some(key), "key {key:#06x}: the desktop's shortcut");
    }

    // A desktop window of the desktop's own on top changes nothing.
    let wh = open_window(&mut pb, 20, 40);
    type_key(K_ALT_C);
    acc.run();
    assert_eq!(poll_key(&mut pb, true), None);
    desktop_context();
    assert_eq!(poll_key(&mut pb, true), Some(K_ALT_C));
    close_window(&mut pb, wh);
}
