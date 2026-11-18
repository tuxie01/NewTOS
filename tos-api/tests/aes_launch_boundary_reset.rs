//! A28 — **the state reset at the desktop → child-program boundary.**
//!
//! The desktop runs in the kernel boot context (`RUN` null) and reports AES
//! id **0**, as does the first program it launches in GEMDOS slot 0 — the
//! whole application chain is EmuTOS's one `rlr`, pid 0
//! (`docs/api/aes/application.md` § "AES process identity and pid",
//! contract (b), A56). Desktop and first child therefore report the same AES
//! id. Until A62 they also shared one AES process descriptor; since A62 the
//! descriptor is keyed by the identity (the GEMDOS slot, or the desktop's
//! own reserved index — contract (a)). **The sharing was never
//! the defect** — EmuTOS does the same thing, renaming the one `rlr` for each
//! successive program (`references/emutos/aes/gemshlib.c:633-636`). This test
//! runs its "child" in the boot context too (no `RUN`), so phases 1–2 act on
//! the desktop's descriptor; phase 3's flag reset on a slot-keyed descriptor
//! is covered by `aes_pd_by_slot.rs` (c).
//!
//! The defect is that EmuTOS resets that shared descriptor **twice** on the way
//! across the boundary and NewTOS did neither:
//!
//! 1. `desk/deskmain.c:2159-2166` — the desktop's epilog, run before the next
//!    program is loaded: `menu_bar(NULL,0)`, `wind_set(DESKWH, WF_NEWDESK,
//!    NULL,0,0)`, **`appl_exit()`**. `appl_exit` is `ap_exit`
//!    (`aes/gemaplib.c:251-261`), which **drains the message queue**
//!    (`if (rlr->p_qindex) ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, …)`) and clears
//!    `AP_OPEN`.
//! 2. `aes/gemshlib.c:636` — `sh_ldapp`, immediately after `p_nameit` /
//!    `p_setappdir` and before `dos_exec`: **`rlr->p_flags = 0;`**. It does
//!    *not* touch the queue; the queue is the first reset's job.
//!
//! What this file asserts is the **effect**, not the code site:
//!
//! * **phase 1** — the queue is empty *before* the child's `appl_init` runs, so
//!   the child starts with zero inherited messages;
//! * **phase 2** — the control: `appl_init` itself does **not** erase anything
//!   (EmuTOS `ap_init`, `aes/gemaplib.c:54-63`, only ORs `AP_OPEN` — A25). So
//!   phase 1's zero cannot be explained by `appl_init`; only the boundary can
//!   explain it;
//! * **phase 3** — the flag reset, through its one observable consumer: the
//!   launcher's "the program called `appl_init` without `appl_exit`" cleanup
//!   (`aes/gemshlib.c:650-657`, `close_client_if_open`) must judge **the program
//!   that just ran**, not a flag the previous occupant of the descriptor left
//!   behind. A console program (`.NCP`) never calls `appl_init`, so that cleanup
//!   must not fire — and must not throw away the desktop's own pending messages.
//!
//! All three phases live in one `#[test]`: the AES descriptor table and window
//! table are process-wide statics, so two test functions in one binary would
//! race.
//!
//! Harness shape (backend + host + parameter block) follows
//! `aes_app_return_descriptor.rs`; this file needs no pixels, so the backend is
//! a silent sink.

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
const WIND_SET: i16 = 105;

/// `WF_TOP` (`aesdefs.h`) — the field `hndl_button` sets on the first click of
/// a double-click (`programs/desktop/src/main.rs`, EmuTOS `desk/deskmain.c`).
const WF_TOP: i16 = 10;

// Window kinds (`aesdefs.h`).
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const SIZER: i16 = 0x0020;

/// `WM_REDRAW` (`aesdefs.h:228`).
const WM_REDRAW: i16 = 20;

/// Two overlapping directory windows, roughly where the desktop puts them.
const WR1: (i16, i16, i16, i16) = (40, 40, 400, 300);
const WR2: (i16, i16, i16, i16) = (120, 110, 400, 300);

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

/// `appl_exit()` — `ap_exit` (`aes/gemaplib.c:251-261`).
fn appl_exit(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_EXIT, 0, 1, 0)
}

/// `appl_read(pid, 16, buf)` — pop of one message; 1 when one was delivered.
/// **Blocks on an empty FIFO** (A75, EmuTOS `aqueue`, `aes/gemqueue.c:98-124`),
/// so callers that may find the queue empty check [`pending`] first.
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

/// `appl_write(pid, 16, msg)` — 1 when the message was queued.
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
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    assert_eq!(pb.call(WIND_OPEN, 5, 1, 0), 1, "wind_open failed");
    h
}

/// `wind_set(handle, WF_TOP, …)` — the first half of a double-click: the click
/// tops the window it lands in, and the newly exposed area becomes a
/// `WM_REDRAW` in the owner's queue.
fn wind_top(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = WF_TOP;
    pb.call(WIND_SET, 6, 1, 0);
}

/// How many messages are waiting for `pid` (`tos-api/src/aes/window.rs` test
/// hook — reads the FIFO depth without popping, so it can be asked *between*
/// two steps of the boundary).
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

/// The desktop's half of `run_gem_app` **before** the child runs
/// (`programs/desktop/src/main.rs`, `desktop_relinquish`), in its order. EmuTOS's
/// desktop epilog is `menu_bar(NULL,0)`, `wind_set(DESKWH,WF_NEWDESK,NULL,0,0)`,
/// `appl_exit()` (`desk/deskmain.c:2159-2166`); the first two are draws that do
/// not touch the descriptor, and `appl_exit` is last in EmuTOS as it is here —
/// so nothing can re-queue after the drain. (The window-table handoff
/// `save_windows`/`restore_windows` the nested launch also did was removed
/// with its last caller in DESK43 U10; the child here opens no window, so the
/// desktop's windows simply stay in the table.)
fn desktop_relinquish(pb: &mut Aespb) {
    appl_exit(pb);
}

/// The launcher's pre-run step (`kernel/src/launch.rs`,
/// `register_process(child.slot, path)` — EmuTOS `p_nameit` **and**
/// `rlr->p_flags = 0`, `aes/gemshlib.c:633-636`).
fn launcher_register(slot: usize, path: &[u8]) {
    tos_api::aes::register_process(slot, path);
}

/// The launcher's post-run step (`kernel/src/launch.rs`,
/// `close_client_if_open(child.slot)` — EmuTOS `aes/gemshlib.c:650-657`),
/// addressed by the slot the descriptor is keyed by (A62).
fn launcher_cleanup(slot: usize) {
    tos_api::aes::close_client_if_open(slot);
}

#[test]
fn the_launch_boundary_hands_the_child_a_reset_descriptor() {
    install_host();
    let mut pb = Aespb::new();

    // ── the desktop comes up and the user clicks ──────────────────────────
    let desk_pid = appl_init(&mut pb);
    assert_eq!(desk_pid, 0, "the desktop's AES id is the chain's, 0 (gemsuper.c:94, rlr->p_pid)");
    let w1 = open_window(&mut pb, WR1);
    let w2 = open_window(&mut pb, WR2);
    drain(&mut pb, desk_pid); // its event loop answered the opens

    // First click of the double-click that launches a program: the window the
    // click landed in comes to the top, and the AES posts the desktop a
    // WM_REDRAW for the area that just became visible.
    wind_top(&mut pb, w1);
    assert_eq!(
        pending(desk_pid),
        1,
        "premise: topping a partly covered window must leave the desktop one \
         unread WM_REDRAW — without it there is no inheritance to prevent"
    );

    // ── phase 1: the boundary resets, so the child starts empty ───────────
    desktop_relinquish(&mut pb);
    assert_eq!(
        pending(desk_pid),
        0,
        "phase 1: the desktop's queue must already be empty when it gives up \
         the machine — EmuTOS drains it in appl_exit (deskmain.c:2166 -> \
         ap_exit, gemaplib.c:256-257), before the child exists"
    );

    launcher_register(0, b"C:\\QED.NAP");
    let child_pid = appl_init(&mut pb);
    assert_eq!(
        child_pid, desk_pid,
        "slot 0 shares the desktop's AES id — the premise of this test"
    );
    // Checked by depth, since `appl_read` blocks on an empty FIFO (A75); the
    // front message, if any, names the leak.
    let leaked = tos_api::aes::window::test_pending_msg(child_pid);
    assert_eq!(
        (pending(child_pid), leaked.map(|m| m[0]).unwrap_or(0)),
        (0, 0),
        "phase 1: the child must start with an empty message queue; it holds a \
         message of type {} — the desktop's, for a window handle that is not \
         its own",
        leaked.map(|m| m[0]).unwrap_or(0)
    );

    // ── phase 2: the control — appl_init is not the eraser ────────────────
    // If appl_init cleared the queue, phase 1 would pass for the wrong reason
    // and A25 would be undone. EmuTOS's ap_init (gemaplib.c:54-63) is three
    // statements and the only state change is `rlr->p_flags |= AP_OPEN`.
    let probe = [0x4711, child_pid, 0, 1, 2, 3, 4, 5];
    assert_eq!(appl_write(&mut pb, child_pid, &probe), 1, "phase 2: setup");
    assert_eq!(
        appl_init(&mut pb),
        child_pid,
        "phase 2: a second appl_init returns the same id"
    );
    let (got, body) = appl_read(&mut pb, child_pid);
    assert_eq!(
        (got, body),
        (1, probe),
        "phase 2: appl_init must NOT discard a pending message (EmuTOS ap_init, \
         aes/gemaplib.c:54-63) — so phase 1's empty queue can only come from \
         the boundary reset"
    );

    // the child ends and the desktop comes back
    appl_exit(&mut pb);
    launcher_cleanup(0);

    // ── phase 3: the flag reset, seen through its one consumer ────────────
    // The desktop is an AES application again (EmuTOS re-enters deskmain, whose
    // first step is appl_init, `desk/deskmain.c:1953`), so AP_OPEN is set on the
    // shared descriptor, and it has a message waiting.
    assert_eq!(appl_init(&mut pb), desk_pid, "phase 3: setup, desktop AP_OPEN");
    drain(&mut pb, desk_pid);
    wind_top(&mut pb, w2);
    drain(&mut pb, desk_pid);
    wind_top(&mut pb, w1);
    assert_eq!(pending(desk_pid), 1, "phase 3: setup, one pending WM_REDRAW");

    // Now a console program (`.NCP`) is launched. `programs/desktop/src/main.rs`
    // sends it straight to `launch::run_program_with_tail` — no GEM screen
    // handoff, so no `appl_exit` — and it never calls `appl_init` itself.
    // `sh_ldapp`'s cleanup must therefore find AP_OPEN *clear* and do nothing:
    // `rlr->p_flags = 0` (gemshlib.c:636) wiped the previous occupant's flag
    // when the descriptor was handed over.
    launcher_register(0, b"C:\\HELLO.NCP");
    launcher_cleanup(0);
    assert_eq!(
        pending(desk_pid),
        1,
        "phase 3: the launcher's 'appl_init without appl_exit' cleanup fired \
         for a program that never called appl_init, and drained the desktop's \
         own queue — the descriptor was handed over with a stale AP_OPEN \
         (EmuTOS clears every flag at the handover, aes/gemshlib.c:636)"
    );
    let (got, body) = appl_read(&mut pb, desk_pid);
    assert_eq!(
        (got, body[0]),
        (1, WM_REDRAW),
        "phase 3: and the surviving message is the desktop's own WM_REDRAW"
    );
}
