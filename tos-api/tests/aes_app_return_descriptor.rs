//! A29 — **what the desktop loses when a program returns.**
//!
//! The desktop runs in the kernel boot context (`RUN` null) and reports AES
//! id **0**; so does the first program it launches, in GEMDOS slot 0, and so
//! does every `Pexec` child — the whole application chain is EmuTOS's one
//! `rlr`, pid 0 (`docs/api/aes/application.md` § "AES process identity and
//! pid", contract (b), A56; `aes/geminit.c:850-877`). Desktop and first child
//! therefore **report the same AES id**. Until A62 they also shared one AES
//! process descriptor; since A62 the descriptor is keyed by the *identity*
//! (the GEMDOS slot, or the desktop's own reserved index — contract (a)),
//! and only the reported number is shared. This test runs its "child" in the boot context too (no `RUN`), so
//! it exercises the desktop's descriptor across the boundary; the slot-keyed
//! child is `aes_pd_by_slot.rs`'s business. The sharing of the *number* is
//! EmuTOS-faithful: `sh_ldapp` renames the very same `rlr` for the program it
//! is about to start (`references/emutos/aes/gemshlib.c:633-636`).
//!
//! What EmuTOS does to that shared descriptor when the program ends is the
//! point of this file. Both of its end-of-program paths keep the descriptor
//! alive and clear exactly two things:
//!
//! * the application's own `appl_exit` — `ap_exit`
//!   (`references/emutos/aes/gemaplib.c:251-261`): drain the queue
//!   (`if (rlr->p_qindex) ap_rdwr(MU_MESAG, …)`), then
//!   `rlr->p_flags &= ~AP_OPEN`. `p_pid` and `p_name` survive;
//! * the launcher's fallback for a program that never called `appl_exit` —
//!   `sh_ldapp` (`references/emutos/aes/gemshlib.c:650-657`): `mn_cleanup()`,
//!   the same queue drain, the same `p_flags &= ~AP_OPEN`.
//!
//! Neither frees the `AESPD`. In EmuTOS it *cannot* be freed — the descriptors
//! are fixed entries of `D.g_int[]`/`D.g_acc[]` reached by index (`pd_index`,
//! `aes/gempd.c:32-35`), and `fpdnm(NULL, pid)` (`:38-57`) walks that array
//! with no liveness test at all, so a message addressed to a pid always finds
//! its descriptor.
//!
//! The test drives the real launch boundary through the LP64 AES dispatcher,
//! in the order `programs/desktop/src/main.rs` (`run_gem_app` → `desktop_relinquish`
//! / `desktop_resume`) and `kernel/src/launch.rs` execute it, and then asks the
//! only question that matters: **after the program is gone, does the desktop
//! still get its messages?**
//!
//! Harness shape (backend + host + parameter block) follows
//! `aes_window_content.rs`; this file needs no pixels, so the backend is a
//! silent sink.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

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

// ── the host ──────────────────────────────────────────────────────────────

struct SilentHost {
    raster: SinkBackend,
    clock: AtomicU64,
}

impl Host for SilentHost {
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
    }
}

unsafe impl Sync for SilentHost {}
unsafe impl Send for SilentHost {}

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
        assert!(unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) }.is_none());
        self.intout[0]
    }
}

// AES opcodes used here.
const APPL_INIT: i16 = 10;
const APPL_READ: i16 = 11;
const APPL_WRITE: i16 = 12;
const APPL_EXIT: i16 = 19;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;

// Window kinds (`aesdefs.h`).
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const SIZER: i16 = 0x0020;

/// `WM_REDRAW` (`aesdefs.h:228`).
const WM_REDRAW: i16 = 20;

/// The desktop's first directory window (`C:\`), roughly where
/// `desk::main::open_root_window` puts it.
const WR: (i16, i16, i16, i16) = (40, 40, 400, 300);

fn install_host() {
    let host: &'static SilentHost = Box::leak(Box::new(SilentHost {
        raster: SinkBackend,
        clock: AtomicU64::new(0),
    }));
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

/// `appl_init()` — returns the caller's AES id.
fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0)
}

/// `appl_exit()` — what a well-behaved GEM application calls on its way out
/// (TeraDesk `sdk/apps/teradesk/main.c:2124`, QED through cflib's `appexitg`).
fn appl_exit(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_EXIT, 0, 1, 0)
}

/// `appl_read(pid, 16, buf)` — pop of one message; 1 when one was delivered,
/// 0 when **the descriptor is gone**. **Blocks on an empty FIFO** (A75, EmuTOS
/// `aqueue`, `aes/gemqueue.c:98-124`), so callers that may find the queue
/// empty check [`pending`] first.
fn appl_read(pb: &mut Aespb, pid: i16) -> (i16, [i16; 8]) {
    let mut buf = [0i16; 8];
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    let r = pb.call(APPL_READ, 2, 1, 1);
    pb.addrin[0] = 0;
    (r, buf)
}

/// `appl_write(pid, 16, msg)` — 1 when the message was queued, 0 when it was
/// not (no descriptor for `pid`, or a full FIFO).
fn appl_write(pb: &mut Aespb, pid: i16, msg: &[i16; 8]) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin[0] = msg.as_ptr() as u64;
    let r = pb.call(APPL_WRITE, 2, 1, 1);
    pb.addrin[0] = 0;
    r
}

/// `wind_create` + `wind_open` at `r`, returning the handle.
fn open_window(pb: &mut Aespb, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = NAME | CLOSER | FULLER | MOVER | SIZER;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create must give a handle, got {h}");
    reopen_window(pb, h, r);
    h
}

/// `wind_open(handle, r)` — the desktop opening a directory window. EmuTOS's
/// `wm_open`→`draw_change` ends in `w_update`→`w_redraw`, which posts one
/// `WM_REDRAW` for the new work area to the window's owner
/// (`references/emutos/aes/gemwmlib.c:1420-1436,1004-1048,887-907`).
fn reopen_window(pb: &mut Aespb, h: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    assert_eq!(pb.call(WIND_OPEN, 5, 1, 0), 1, "wind_open failed");
}

fn close_window(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_CLOSE, 1, 1, 0);
}

/// How many messages are waiting for `pid` (`tos-api/src/aes/window.rs` test
/// hook — reads the FIFO depth without popping).
fn pending(pid: i16) -> usize {
    tos_api::aes::window::test_msg_count(pid)
}

/// Read everything the desktop's event loop would have read by now (depth
/// checked first: `appl_read` blocks on an empty FIFO).
fn drain(pb: &mut Aespb, pid: i16) -> usize {
    let mut n = 0;
    while pending(pid) > 0 {
        assert_eq!(appl_read(pb, pid).0, 1, "a queued message must be delivered");
        n += 1;
        assert!(n < 64, "runaway queue");
    }
    n
}

/// The desktop's half of `run_gem_app` before the child runs
/// (`programs/desktop/src/main.rs`, `desktop_relinquish`), in its order. The
/// menu-bar removal and the `WF_NEWDESK` un-registration are draws that do not
/// touch the descriptor; the one that does is `appl_exit()`, last — EmuTOS's
/// desktop epilog `desk/deskmain.c:2159-2166` (A28). (The window-table
/// handoff `save_windows`/`restore_windows` of the nested launch was removed
/// with its last caller in DESK43 U10; the child here opens no window, so the
/// desktop's window simply stays in the table.) This file is about what
/// `appl_exit` must *not* do: it drains the queue and clears `AP_OPEN`, and
/// the desktop is still addressable afterwards.
fn desktop_relinquish(pb: &mut Aespb) {
    appl_exit(pb);
}

/// The desktop's half of `run_gem_app` after the child `Pterm`s
/// (`programs/desktop/src/main.rs`, `desktop_resume`): the desktop re-enters
/// the AES, as EmuTOS's re-entered `deskmain` does in its first statement
/// (`desk/deskmain.c:1953`).
fn desktop_resume(pb: &mut Aespb) {
    appl_init(pb);
}

/// The launcher's pre-run step (`kernel/src/launch.rs`,
/// `register_process(child.slot, path)` — EmuTOS `p_nameit`,
/// `aes/gemshlib.c:633`).
fn launcher_register(slot: usize, path: &[u8]) {
    tos_api::aes::register_process(slot, path);
}

/// The launcher's post-run step (`kernel/src/launch.rs`,
/// `close_client_if_open(child.slot)` — EmuTOS `aes/gemshlib.c:650-657`),
/// addressed by the slot the descriptor is keyed by (A62).
fn launcher_cleanup(slot: usize) {
    tos_api::aes::close_client_if_open(slot);
}

/// One trip through the launch boundary. `polite` selects which of the two
/// end-of-program paths the child takes: `true` = it calls `appl_exit()`
/// itself (TeraDesk, QED), `false` = it just terminates and the launcher's
/// `sh_ldapp` fallback has to clean up.
fn run_child(pb: &mut Aespb, desk_pid: i16, slot: usize, path: &[u8], polite: bool) {
    desktop_relinquish(pb);
    launcher_register(slot, path);
    let child_pid = appl_init(pb);
    assert_eq!(
        child_pid, desk_pid,
        "slot {slot} shares the desktop's AES id — the premise of this test"
    );
    if polite {
        appl_exit(pb);
    }
    launcher_cleanup(slot);
    desktop_resume(pb);
}

/// The whole point: after a program has come and gone, the desktop must still
/// be an addressable AES application. Both end-of-program paths are exercised.
///
/// One test owns all AES statics (the descriptor table and the window table are
/// process-wide), so the two boundary trips run in sequence inside it.
#[test]
fn the_desktop_still_receives_messages_after_a_program_returns() {
    install_host();
    let mut pb = Aespb::new();

    // ── the desktop comes up (`programs/desktop/src/lib.rs`, `run`) ────────────
    let desk_pid = appl_init(&mut pb);
    assert_eq!(desk_pid, 0, "the desktop's AES id is the chain's, 0 (gemsuper.c:94, rlr->p_pid)");
    let win = open_window(&mut pb, WR);
    drain(&mut pb, desk_pid); // its event loop answered the open's WM_REDRAW

    for (round, polite) in [(1, true), (2, false)] {
        let who: &[u8] = if polite {
            b"C:\\TERADESK.NAP"
        } else {
            b"C:\\CRASHED.NAP"
        };
        run_child(&mut pb, desk_pid, 0, who, polite);

        // 1. The desktop is still an addressable application. This is the
        //    literal symptom A29 names: `appl_write` to its own pid returning 0.
        let probe = [0x4711, desk_pid, 0, 1, 2, 3, 4, 5];
        assert_eq!(
            appl_write(&mut pb, desk_pid, &probe),
            1,
            "round {round}: appl_write to the desktop was refused — its \
             descriptor did not survive the program"
        );
        let (got, body) = appl_read(&mut pb, desk_pid);
        assert_eq!(got, 1, "round {round}: the desktop could not read it back");
        assert_eq!(body, probe, "round {round}: message body");

        // 2. And it is still *wired up*: a window operation the desktop
        //    performs after the return must reach its event loop. This is the
        //    effect the user sees — without it every WM_REDRAW / WM_CLOSED /
        //    WM_MOVED / WM_SIZED / WM_ARROWED / MN_SELECTED is dropped on the
        //    floor and the desktop is inert.
        close_window(&mut pb, win);
        drain(&mut pb, desk_pid);
        reopen_window(&mut pb, win, WR);
        let (got, body) = appl_read(&mut pb, desk_pid);
        assert_eq!(
            got, 1,
            "round {round}: wind_open posted no WM_REDRAW to the desktop — \
             its descriptor is gone, so every window message is dropped"
        );
        assert_eq!(body[0], WM_REDRAW, "round {round}: message type");
        assert_eq!(body[3], win, "round {round}: WM_REDRAW window handle");
        drain(&mut pb, desk_pid);
    }
}
