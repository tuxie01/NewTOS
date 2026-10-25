//! AES Event Manager (`evnt_*`, opcodes 20–26).
//!
//! Spec: [`docs/api/aes/event.md`]; the `int_in`/`int_out` slot layout is
//! the EmuTOS `include/aesdefs.h` mapping (e.g. `T_LOCOUNT = int_in[0]`,
//! `RET_CODE = int_out[0]`).
//!
//! **Wait model (sleep/wake substrate, cooperative-kernel steps 3a + 3b):**
//! EmuTOS' `ev_block` parks the calling AES process and yields to others until
//! an event fires. Each of the six `evnt_*` waits below re-checks its predicate
//! and, when it has not fired, blocks on [`sched::sleep`] with the matching
//! wake-class key ([`sched::wait_input`]/[`sched::wait_mesg`]/
//! [`sched::wait_timer`], and for `evnt_multi` the union key
//! [`sched::wait_multi`], A112) — the single cooperative block primitive replacing the
//! former per-function `pump_input` spin (`docs/analysis/coop-kernel/`
//! `20-sleepwake-substrate-design.md` §5).
//!
//! `sleep` returns a [`sched::SleepOutcome`]: [`Recheck`](sched::SleepOutcome::
//! Recheck) (no peer — the step-3 single-context idle-fallback, identical to the
//! old spin) or [`Yield`](sched::SleepOutcome::Yield)`(Switch)` (a runnable peer
//! exists — step 3b). On `Yield` the wait **aborts and returns the `Switch`**
//! ([`Option<sched::Switch>`]) up through [`super::dispatch`] to
//! `rust_sync_handler`, which saves this process's frame with the ELR rewound to
//! the `svc` (the S2 restartable block) and switches to the peer; on resume the
//! re-executed `svc` re-enters the wait, which re-checks and fires. Because the
//! block path is read-only until the firing pass (design §4.3), an abort commits
//! nothing. The observable `evnt_*` contract (return values, `MU_*` bits,
//! message dequeue-on-firing) is unchanged.

use core::sync::atomic::{AtomicBool, AtomicI16, Ordering};

use super::input::downorup;
use super::pd;
use super::Aespb;
use crate::bdos::process::sched;
use crate::host::host;
use tos_types::aes::{
    AC_CLOSE, AC_OPEN, MN_SELECTED, MU_BUTTON, MU_KEYBD, MU_M1, MU_M2, MU_MESAG, MU_TIMER,
    MU_TOSVALID,
};

/// A mouse rectangle block (`MOBLK`): `flags` selects enter (0) vs leave
/// (1) of the `{x, y, w, h}` rectangle.
struct MoBlk {
    flags: i16,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

/// `true` when a `MOBLK`'s condition is satisfied (EmuTOS `in_mrect`:
/// the event fires when `flags != inside`) at the live pointer position.
fn rect_event(m: &MoBlk) -> bool {
    rect_event_at(m, crate::vdi::mouse_position())
}

/// [`rect_event`] against an explicit pointer position `(mx, my)` — the
/// arm-time position `amouse` tests (`aes/geminput.c:575-576`), or the
/// `xrat`/`yrat` a `post_mouse` carries (`:446-456`).
fn rect_event_at(m: &MoBlk, (mx, my): (i16, i16)) -> bool {
    let inside = mx >= m.x && mx < m.x + m.w && my >= m.y && my < m.y + m.h;
    (m.flags != 0) != inside
}

/// One flag per AES identity: a blocking `evnt_multi`/`evnt_mouse` of that
/// identity has **armed** its events and is parked (A112 F4,
/// `docs/api/aes/event.md` "Wake conditions and dispatch" point 5).
///
/// EmuTOS tests the immediate conditions once — the quick checks and each
/// `iasync` arm (`aes/gemevlib.c:172-259`) — and after `mwait` returns it looks
/// at nothing but the completed events (`:261-290`). A NewTOS wait that yields
/// to a peer is aborted and its `svc` re-executed on resume (the S2
/// restartable block), so without a record of the arm the re-executed call
/// would run its arm-time tests a second time against a later state. This is
/// that record — armed when the call is about to block, kept across S2
/// restarts, cleared on every completion and when the slot is released
/// ([`forget_wait`]). It lives out here, not on the stack, for the reason
/// `acc::WAIT_PASSES` does.
static EV_ARMED: [AtomicBool; pd::NUM_PDS] = [const { AtomicBool::new(false) }; pd::NUM_PDS];

/// `true` when identity `idx` re-enters a wait it had already armed.
fn ev_armed(idx: usize) -> bool {
    EV_ARMED.get(idx).is_some_and(|f| f.load(Ordering::Acquire))
}

/// Record (`true`) or drop (`false`) identity `idx`'s armed wait.
fn set_ev_armed(idx: usize, armed: bool) {
    if let Some(f) = EV_ARMED.get(idx) {
        f.store(armed, Ordering::Release);
    }
}

/// Drop identity `idx`'s armed-wait record — the slot's tenant is gone
/// ([`super::release_client`]) or the AES restarts ([`super::reinit`]).
pub(super) fn forget_wait(idx: usize) {
    set_ev_armed(idx, false);
}

/// Drop every armed-wait record (AES restart).
pub(super) fn reinit_waits() {
    for f in EV_ARMED.iter() {
        f.store(false, Ordering::Release);
    }
}

/// The AES tick in milliseconds, `gl_ticktime` (`geminit.c:704`).
const TICK_MS: u64 = crate::vdi::mouse::TICKCAL_MS as u64;

/// Milliseconds from `now` until a timer of `ms` milliseconds expires, as
/// `adelay` arms it (A112, `docs/api/aes/event.md` "Wake conditions and
/// dispatch" point 6): a count that divides to **0 ticks** (`ms /
/// gl_ticktime == 0`, `gemevlib.c:136,259`) is raised to **one** tick
/// (`if (c == 0L) c = 1L`, `geminput.c:500-501`), i.e. it expires at the
/// next tick boundary — 0..1 tick periods from now, never immediately.
/// Longer counts keep their millisecond value.
fn timer_span(now: u64, ms: u64) -> u64 {
    if ms / TICK_MS == 0 {
        (now / TICK_MS + 1) * TICK_MS - now
    } else {
        ms
    }
}

/// AES menu-bar interception (mirroring the EmuTOS Control Manager's
/// `hctl_rect`, `gemctrl.c:318`): if a menu bar is installed and the cursor is
/// over it, run the drop-down tracker (`menu::track`) — GEM hover menus, no
/// click needed to open. On a selection, post an `MN_SELECTED` message into
/// the menu owner's per-app FIFO — the EmuTOS `ct_msgup(MN_SELECTED, gl_mnppd,
/// …)`→`ap_sendmsg`→`aqueue`/`doq` path (`gemctrl.c:353-367`,
/// `gemwmlib.c:851-863`, `gemqueue.c:85-125`) — and return `true`; the caller
/// then delivers it through the regular `MU_MESAG` dequeue, so a menu
/// selection can no longer clobber (or be clobbered by) an already-queued
/// message (AUD-P2-002). Returns `false` when there is no menu, the cursor is
/// not over the bar, or the drop-down was cancelled (no message).
///
/// The target is the **bar owner**, `gl_mnppd` (`gemctrl.c:353`,
/// [`menu::mn_selected_target`](super::menu::mn_selected_target)), never the
/// waiter whose `evnt_multi` the inline Control Manager happens to run in — a
/// desk accessory idling in `evnt_multi(MU_MESAG)` runs it as well as the
/// application does (A117). The `ap_sendmsg` sender pid
/// `msg[1] = rlr->p_pid` (`gemwmlib.c:855`) is the **Screen Manager's**,
/// `appl::SCRENMGR_PID` = 1: `ct_msgup` runs inside `ctlmgr()`, `rlr ==
/// ctl_pd` (`gemctrl.c:353-367`; spec § "AES process identity and pid",
/// contract (b)). A stamp, not a process — NewTOS's control manager runs
/// inline in the caller's trap, so the caller's own pid (0) would be wrong
/// here.
///
/// **Entry = the hand-over.** EmuTOS reaches `hctl_rect` only after `mchange`
/// gave the mouse to the Control Manager, which it does for a pointer in the
/// bar row **with the buttons up**: `if ((gl_mowner != ctl_pd) && (button ==
/// 0) && gl_mntree && in_mrect(&gl_ctwait)) gl_mowner = ctl_pd;`
/// (`aes/geminput.c:388-389`). A pointer dragged into the bar with the button
/// held belongs to whoever took the press and opens nothing; the tracker is
/// entered once the button is up. Inside the track the button may of course go
/// down (press on a title, drag to an item) — that is `mn_do`'s business.
///
/// **After a selection** the Control Manager is in `ct_msgup`'s wait for the
/// button to come up (`aes/gemctrl.c:100-101`): the message is posted at once,
/// and [`input::ct_msgup_hold`](super::input::ct_msgup_hold) keeps the
/// interception and `w_setactive` out of the way until the release.
fn menu_intercept() -> bool {
    let tree = super::menu::active();
    if tree.is_null() {
        return false;
    }
    // Hover model: the drop-down opens whenever the cursor is over the
    // Control Manager's wait rectangle with the buttons up — `mchange`'s
    // hand-over condition, no click required. That rectangle is `gl_ctwait`,
    // the registered bar's `THEACTIVE` extent (`mn_bar`, `gemmnlib.c:717`;
    // `ctlmgr`'s `MU_M1` wait, `gemctrl.c:456-460`) — the titles, not the
    // whole `gl_rmenu` row: a pointer right of the last title is the
    // application's (A87).
    let (mx, my) = crate::vdi::mouse_position();
    if !super::menu::in_ctwait(mx, my) || crate::vdi::mouse_buttons() & 0x01 != 0 {
        return false;
    }
    let Some(backend) = host().and_then(|h| h.raster()) else {
        return false;
    };
    // `ctlmgr`'s `ev_multi` fired on `MU_M1`: `wm_update(BEG_UPDATE); /* take
    // the screen */` (`gemctrl.c:463`), then `hctl_rect()` (`:472-473`). With
    // the lock another process's, `ctlmgr` waits in that `wm_update` and the
    // bar is not tracked (the caller gates on `window::ctl_may_run`).
    if !super::window::ctl_begin() {
        return false;
    }
    match super::menu::track(tree, backend) {
        Some((title, item)) => {
            let mut msg = [0i16; pd::MSG_WORDS];
            // ap_sendmsg: msg[1] = rlr->p_pid — the Screen Manager's, 1
            // (gemctrl.c:367 runs as ctl_pd; gemwmlib.c:855).
            msg[1] = super::appl::SCRENMGR_PID;
            // `if ((title == THEDESK) && (item >= gl_dafirst))`
            // (`gemctrl.c:337`) — a pick in the Desk drop-down at or past the
            // first accessory entry belongs to a desk accessory, not to the
            // application that owns the bar.
            let (target, msg0, w3, w4, ext) = match super::menu::desk_accessory_pick(title, item) {
                Some((owner, menu_id)) => {
                    // `do_chg(gl_mntree, title, SELECTED, FALSE, TRUE, TRUE)`
                    // (`gemctrl.c:341`) — un-highlight the *Desk* title, which
                    // the tracker leaves selected; the accessory's own
                    // `form_alert`/window draws next and the title must not
                    // stay inverted behind it.
                    super::menu::normalize_title(title);
                    // `ct_msgup(AC_OPEN, owner, title, item, 0, 0, 0)`
                    // (`gemctrl.c:337-350`): the *title* (THEDESK) lands in
                    // `msg[3]`, the accessory's **menu id** in `msg[4]` — the
                    // asymmetry `AC_CLOSE` mirrors (`docs/api/aes/
                    // accessories.md` §3). `treehi = treelo = parent = 0`
                    // (`gemctrl.c:348`).
                    (Some(owner), AC_OPEN, title, menu_id, [0; 3])
                }
                // `owner = gl_mnppd; mesag = MN_SELECTED;` plus the
                // CONF_WITH_MENU_EXTENSION payload `treehi = HIWORD(tree);
                // treelo = LOWORD(tree); parent = get_par(tree, item);`
                // (`gemctrl.c:353-358,365`) — WinDom routes a pick by `msg[5..6]`
                // (TASK/ZVIEW04).
                //
                // The owner is the bar's, not this waiter's (A117): the inline
                // Control Manager also runs in a desk accessory's wait.
                None => (
                    super::menu::mn_selected_target(),
                    MN_SELECTED,
                    title,
                    item,
                    super::menu::mn_selected_ext(tree, item),
                ),
            };
            msg[0] = msg0;
            msg[3] = w3;
            msg[4] = w4;
            msg[5] = ext[0];
            msg[6] = ext[1];
            msg[7] = ext[2];
            // No target only for a bar whose descriptor has no live member —
            // a dead client's bar is dropped with it (A107, `release_bar_of`).
            if let Some(target) = target {
                // SAFETY: single trap path; `post_msg` hands the message to
                // the live slot (EmuTOS `aqueue`/`doq`, `gemqueue.c:85-125`).
                unsafe {
                    pd::post_msg(target, msg);
                }
            }
            // `ct_msgup`: "send message and wait for the mouse button to come
            // up" (`gemctrl.c:81-104`) — still inside `ctlmgr`'s screen lock,
            // which the end of that wait gives up (`:484`;
            // `input::ct_msgup_busy` → `window::ctl_release`).
            super::input::ct_msgup_hold();
            super::window::ctl_finish(true);
            true
        }
        None => {
            // cancelled — the click is consumed, no message;
            // `wm_update(END_UPDATE)` (`gemctrl.c:484`).
            super::window::ctl_finish(false);
            false
        }
    }
}

/// Fill the shared event return slots — EmuTOS `ev_rets` (`aes/gemevlib.c:
/// 42-58`), the block at `int_out[1..4]`: `EV_MX`/`EV_MY`/`EV_MB` are the
/// pointer position and button state **at the event** — the state recorded
/// before the last button transition when more than one transition was
/// published since the previous event return, else the live state
/// ([`super::input::ev_rets`], which also clears the count) — and `EV_KS` the
/// live keyboard shift state.
///
/// A click whose release falls inside the double-click window therefore
/// reports `EV_MB` = the pressed button, not the released one: the window's
/// close publishes the press and the release together (`b_delay`,
/// `aes/geminput.c:155-171`), `mtrans` is 2, and `pr_button` is the press.
fn fill_rets(pb: &Aespb) {
    let (mx, my, mb) = super::input::ev_rets();
    pb.set_intout(1, mx); // EV_MX
    pb.set_intout(2, my); // EV_MY
    pb.set_intout(3, mb as i16); // EV_MB
    pb.set_intout(4, shift_state()); // EV_KS
}

/// Current keyboard shift-state bitmask (`Kbshift` encoding), or 0 with
/// no host.
fn shift_state() -> i16 {
    host().map(|h| h.keyboard_shift_state()).unwrap_or(0) as i16
}

/// `evnt_button` (21) — EmuTOS `ev_button` (`aes/gemevlib.c:84-94`): block
/// until the mouse buttons satisfy the request, then return the click count
/// and fill the mouse / keyboard return slots.
///
/// `int_in[0]` = flag + clicks to wait for (`B_CLICKS`), `int_in[1]` = button
/// mask (`B_MASK`), `int_in[2]` = desired masked state (`B_STATE`).
/// `RET_CODE` = clicks; `int_out[1..4]` = x, y, buttons, shift.
///
/// `ev_button` is `ev_block(MU_BUTTON, buparm)` → `iasync` → `abutton`
/// ([`super::input::abutton`]): if the AES `button` already satisfies
/// [`downorup`] the call returns **1** at once (`azombie(e, 1)`), whatever was
/// asked for. Otherwise the wait is parked (a request for more than one click
/// raises `gl_bpend`, so the next press opens the button-delay window) and is
/// completed only by a published transition — `post_button` returning
/// `min(clicks of the transition, clicks requested)` (`aes/geminput.c:463-483`).
/// A double click therefore comes back when its window has closed, with 2.
///
/// Deliberately **not** gated on mouse ownership at the arm: `abutton` tests
/// the level unconditionally (`aes/geminput.c:549-551`). Only `ev_multi`'s
/// *quick check* carries the `rlr == gl_mowner` condition (`aes/gemevlib.c:185`);
/// the parked wait is completed only while the caller owns the mouse
/// (`post_button(gl_mowner, …)`).
pub fn evnt_button(pb: &Aespb) -> Option<sched::Switch> {
    let bflgclks = pb.intin(0);
    let mask = pb.intin(1) as u8;
    let state = pb.intin(2) as u8;
    let idx = super::appl::current_identity();
    let mut got = None;
    if host().is_some() {
        super::input::b_delay_poll();
        // A re-executed call (S2) finds its own wait still parked.
        if !super::input::bwait_armed(idx) {
            got = super::input::abutton(idx, super::input::button(), bflgclks, mask, state);
        }
        while got.is_none() {
            super::input::b_delay_poll();
            got = super::input::bwait_take(idx);
            if got.is_some() {
                break;
            }
            // AES trap tracer: one sample per evaluation of the wait.
            // Guarded by the compile-time `ENABLED`, so the call folds away
            // without the feature.
            if super::trace::ENABLED {
                super::trace::note_eval();
                if super::input::bdely_pending() {
                    super::trace::note_click_window();
                }
            }
            if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_input()) {
                return Some(sw); // yield to a peer (S2); the wait stays parked
            }
        }
    } else {
        // No host, no input source: only the arm's level test can answer.
        got = super::input::abutton(idx, super::input::button(), bflgclks, mask, state);
        super::input::acancel(idx);
    }
    fill_rets(pb);
    pb.set_ret(got.unwrap_or(0));
    None
}

/// Double-click intervals per speed index 0..4 (EmuTOS `gl_dcrates`, ms).
const DCRATES: [u64; 5] = [450, 330, 275, 220, 165];

/// EmuTOS `gl_dclick` — the button-delay window in AES ticks:
/// `gl_dcrates[gl_dcindex] / gl_ticktime` (`ev_dclick`, `aes/gemevlib.c:
/// 300-309`), 220 ms / 20 ms = 11 ticks at the default index 3. Integer
/// division, as in C.
pub(super) fn gl_dclick() -> i16 {
    let ms = DCRATES[DCLICK_INDEX.load(Ordering::Relaxed).clamp(0, 4) as usize];
    (ms / crate::vdi::mouse::TICKCAL_MS as u64) as i16
}

/// `evnt_mouse` (22) — block until the mouse enters (`flags` = 0) or
/// leaves (`flags` = 1) the rectangle in `int_in[1..4]`, then fill the
/// return slots and return 1.
///
/// MOBLK: `int_in[0]` = flags, `int_in[1..4]` = x, y, w, h. The event
/// fires when `flags != inside` (EmuTOS `in_mrect`).
///
/// **The wait is `amouse`'s, as `evnt_multi`'s `MU_M1`/`MU_M2` are (A90).**
/// `ev_mouse` is `ev_block(MU_M1, pmo)` → `iasync` → `amouse`
/// (`aes/gemevlib.c:123-128`, `aes/gemasync.c:157-162`,
/// `aes/geminput.c:568-587`): `in_mrect` is tested **once, at arm time**,
/// ownership-blind, against the position the previous dispatch left —
/// `ev_mouse` has no `chkkbd(); forker();` of its own, unlike `ev_multi`
/// (`gemevlib.c:169-170`), so nothing is pumped before the arm. If the
/// rectangle does not match at that instant, the wait parks on the caller's
/// `c_msleep`, which only `post_mouse(gl_mowner, …)` walks
/// (`geminput.c:446-456`): from `mchange` on every move *while the caller
/// owns the mouse*, and once from `set_mown` the moment ownership comes
/// (back) to it (`:220-231`). Pass 0 below is the arm, every later pass the
/// parked state — live position, owner only.
///
/// Ownership comes back through the Control Manager's loop head,
/// `w_setactive` (`aes/gemctrl.c:442`), which runs here as it does in
/// [`evnt_multi`] — a no-op while a button is held or a menu track holds
/// the mouse, and not at all while `ct_msgup` still waits for the press that
/// selected a menu item to be released (`:100-101`). A rectangle the pointer
/// crosses during that hold is therefore not delivered; the release lets
/// `w_setactive` → `set_mown(app)` → `post_mouse(app, xrat, yrat)` complete
/// the parked wait against the live position (`gemctrl.c:441-442`,
/// `geminput.c:225-231`). The bar/frame interception itself stays with
/// `evnt_multi` (the inline Control Manager runs for a message wait only,
/// AUD-P2-003); `evnt_mouse` only lets the mouse come back.
pub fn evnt_mouse(pb: &Aespb) -> Option<sched::Switch> {
    let m = MoBlk {
        flags: pb.intin(0),
        x: pb.intin(1),
        y: pb.intin(2),
        w: pb.intin(3),
        h: pb.intin(4),
    };
    let idx = super::appl::current_identity();
    if host().is_some() {
        // The position at arm time — `amouse`'s one ownership-blind test
        // (`aes/geminput.c:575-576`), taken before anything is pumped.
        let arm_pos = crate::vdi::mouse_position();
        // A re-executed call (S2) resumes the parked wait: no second arm-time
        // test (A112 F4).
        let mut pass = if ev_armed(idx) { 1u32 } else { 0u32 };
        loop {
            // The Control Manager's loop head runs only when it could have
            // taken the screen (`window::ctl_may_run`, `gemctrl.c:439-463`).
            if !super::input::ct_msgup_busy() && super::window::ctl_may_run() {
                super::input::w_setactive();
            }
            let fired = (pass == 0 && rect_event_at(&m, arm_pos))
                || (super::input::own_mouse() && rect_event(&m));
            if fired {
                break;
            }
            set_ev_armed(idx, true);
            if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_input()) {
                return Some(sw); // yield to a peer (S2); re-checks on resume
            }
            pass = pass.saturating_add(1);
        }
        set_ev_armed(idx, false);
    }
    fill_rets(pb);
    // `rets[2] = button;   /* always use current button state */`
    // (`ev_mouse`, `aes/gemevlib.c:127`).
    pb.set_intout(3, super::input::button() as i16); // EV_MB
    pb.set_ret(1);
    None
}

/// AES double-click speed index (0..4); EmuTOS startup default is 3
/// (`geminit.c:708`, `ev_dclick(3, TRUE)`).
static DCLICK_INDEX: AtomicI16 = AtomicI16::new(3);

/// `ev_dclick` (`aes/gemevlib.c`) — the internal getter/setter behind
/// [`evnt_dclick`]. Also called at AES boot with `ev_dclick(3, TRUE)`
/// (`geminit.c:708`) and by `process_inf2` from the `.INF` `#E` line's low
/// three bits (`geminit.c:426`).
pub(crate) fn ev_dclick(rate: i16, setit: bool) -> i16 {
    if setit {
        DCLICK_INDEX.store(rate, Ordering::Relaxed);
    }
    DCLICK_INDEX.load(Ordering::Relaxed)
}

/// `evnt_dclick` (26) — get (`int_in[1]` = 0) or set (`int_in[1]` != 0)
/// the double-click speed index `int_in[0]`. Returns the current index.
pub fn evnt_dclick(pb: &Aespb) {
    let rate = pb.intin(0); // EV_DCRATE
    let setit = pb.intin(1); // EV_DCSETIT
    pb.set_ret(ev_dclick(rate, setit != 0));
}

/// `evnt_multi` (25) — the central event multiplexer. Block until **any**
/// of the requested events (`MU_*` in `int_in[0]`) fires, then return the
/// bitmask of those that did and report the mouse / keyboard / button
/// state for all of them at once.
///
/// `int_in[0]` = flags, `int_in[1..3]` = button clicks/mask/state,
/// `int_in[4..8]` = MOBLK 1, `int_in[9..13]` = MOBLK 2,
/// `int_in[14..15]` = timer lo/hi ms, `addr_in[0]` = message buffer.
/// `RET_CODE` = fired bitmask; `int_out[1..4]` = x, y, buttons, shift;
/// `int_out[5]` = key (`EV_KRET`); `int_out[6]` = click count (`EV_BRET`).
///
/// Each single `evnt_*` wait is a special case of this loop. `EV_KRET`
/// carries the full `scancode << 8 | ascii` key word (cursor / function
/// keys included). The `MU_BUTTON` click count (`EV_BRET`) is the count the
/// button event carried — `mclick`/`pr_mclick` from the quick check, or what
/// `post_button` completed the parked wait with — so callers can tell a
/// Select (1) from an Open (2). The double-click window runs on the AES tick,
/// independently of this call: a `MU_TIMER` that ends the call while it is
/// open does not cut it short, and the gesture is reported by whichever call
/// is waiting when it closes (`docs/api/aes/event.md` "The double-click
/// window"). Bring-up limit: single AES application.
pub fn evnt_multi(pb: &Aespb) -> Option<sched::Switch> {
    let flags = (pb.intin(0) as u16) & MU_TOSVALID;

    let b_flgclks = pb.intin(1);
    let b_mask = pb.intin(2) as u8;
    let b_state = pb.intin(3) as u8;
    let m1 = MoBlk {
        flags: pb.intin(4),
        x: pb.intin(5),
        y: pb.intin(6),
        w: pb.intin(7),
        h: pb.intin(8),
    };
    let m2 = MoBlk {
        flags: pb.intin(9),
        x: pb.intin(10),
        y: pb.intin(11),
        w: pb.intin(12),
        h: pb.intin(13),
    };
    let lo = pb.intin(14) as u16 as u32;
    let hi = pb.intin(15) as u16 as u32;
    let ms = ((hi << 16) | lo) as u64;
    let pbuff = pb.addrin(0);
    let idx = super::appl::current_identity();

    // `if (MU_FLAGS & MU_MESAG) rlr->p_flags |= AP_MESAG;` (`gemsuper.c:147-148`)
    // — set when the wait is **issued**, not when a message arrives: that is
    // exactly what `wait_for_accs(AP_MESAG)` tests for ("the accessory has
    // issued at least one wait for a message", `geminit.c:574-578`). Idempotent,
    // so an S2 restart of this call re-sets the same bit.
    if flags & MU_MESAG != 0 {
        // SAFETY: single trap path; `by_index` hands out the live descriptor.
        if let Some(p) = unsafe { pd::by_index(idx) } {
            p.flags |= pd::AP_MESAG;
        }
    }

    let mut fired = 0u16;
    let mut key = 0i16;
    let mut clicks = 0i16;

    if let Some(h) = host() {
        // `ev_multi` opens with `chkkbd(); forker();` (`aes/gemevlib.c:169-170`):
        // the fork ring — every mouse/keyboard packet the interrupt side queued
        // since the last dispatch — is drained *before* the quick checks run
        // and before `abutton`/`amouse` arm, so the `button` level and the
        // `xrat`/`yrat` position they test are current as of the call, not as
        // of the previous trap's `dsptch()`. One `pump_input` is that drain
        // here — the same call `dsptch()` and the wait loop below make — and
        // the arm-time state (`arm_own`/`arm_mb`/`arm_pos`) is sampled after
        // it (A92). The key `chkkbd` would fork is read where it lands
        // (`console_try_read_key`, in the loop). Harmless on an S2 restart:
        // the whole call re-executes and pumps once more, as the wait loop
        // would have.
        h.pump_input();
        // Persist the timer deadline per-slot for S2-restart idempotency
        // (design §4.3 OPEN): the first entry stores `now + ms`, an S2 restart
        // (this whole loop re-executes from the top on resume) reuses it, and
        // any completion below clears it. Only meaningful when `MU_TIMER` is
        // armed; without a timer the value is unused (`cap = None`, no MU_TIMER
        // check), so it is computed but not persisted.
        //
        // `tmcount == 0` is the quick-check poll (`gemevlib.c:216-220`): the
        // deadline is `now` and pass 0 fires it. Any other count is armed the
        // way `adelay` arms it — a count below one tick as one tick
        // ([`timer_span`], A112).
        let now = h.monotonic_millis();
        let span = if ms == 0 { 0 } else { timer_span(now, ms) };
        let deadline = if flags & MU_TIMER != 0 {
            sched::arm_or_get_deadline(now, span)
        } else {
            now.saturating_add(span)
        };
        // Pass counter: pass 0 is the *entry* evaluation (EmuTOS's `abutton` /
        // `amouse` level test taken when `ev_multi` arms the event), every later
        // pass is the "asleep in `mwait`" state. See the MU_BUTTON block below.
        // A call re-executed after a peer yield (S2) is still asleep in `mwait`:
        // it resumes at pass 1 and does not run the quick checks or the arm
        // a second time (A112 F4; `gemevlib.c:261-290` looks only at what
        // completed).
        let mut pass = if ev_armed(idx) { 1u32 } else { 0u32 };
        // The button state and mouse ownership **at arm time** — what `ev_multi`
        // sees when it runs its quick check (`rlr == gl_mowner`,
        // `aes/gemevlib.c:185`) and arms `abutton`'s level test
        // (`aes/geminput.c:549-551`): both happen before the application
        // blocks, i.e. before the Control Manager can act. Our Control Manager
        // runs inline in this very pass, *ahead* of the checks, and may consume
        // a button transition on the way (the drop-down track selects on a
        // press or a release); the pass-0 test must therefore use the level the
        // arm saw, not the level the track left behind. Sampled once, here.
        // The level is EmuTOS's `button` (`downorup(button, buparm)`,
        // `aes/gemevlib.c:196`; `abutton`, `aes/geminput.c:549`) — the state
        // the AES has acted on, which `ct_msgup`'s HOTCLOSE exit fakes to 0
        // for a press the Control Manager has already consumed
        // (`aes/gemctrl.c:92`). It equals the raw VDI byte otherwise.
        // The window countdown is brought to the call's entry first, as the
        // timer interrupt would have run it (`b_delay`, see `input`).
        super::input::b_delay_poll();
        let arm_own = super::input::own_mouse();
        let arm_mb = super::input::button();
        // `mclick` — the count `prets[5]` reports when the live level satisfies
        // the quick check (`aes/gemevlib.c:196-198`).
        let arm_mclick = super::input::mclick();
        // The recorded state the quick check tries first: `(mtrans > 1) &&
        // downorup(pr_button, buparm)` (`aes/gemevlib.c:190`) — a press and
        // its release both published since the previous event return (a click
        // made while the application was busy) still satisfy a wait for the
        // press. Sampled at arm time with the level above.
        let arm_pending = super::input::pending_transition();
        // The pointer position **at arm time**, for the same reason: `amouse`
        // tests a rectangle wait exactly once, against the position at the
        // instant `ev_multi` arms it (`aes/geminput.c:575-576`), before the
        // application blocks and before the Control Manager can move the
        // pointer through the rectangle on its own account. Sampled here,
        // ahead of the inline interception (A73, point 1).
        let arm_pos = crate::vdi::mouse_position();
        loop {
            // The ticks that passed while asleep close a window before this
            // pass looks at anything (`b_delay`, see `input`); its transitions
            // complete our parked wait through `post_button`.
            super::input::b_delay_poll();
            // AES trap tracer: one sample of the button byte per evaluation of
            // the multiplexer predicate, folded into the call record as
            // `passes`/`mb_or`/`mb_and`. Guarded by the compile-time `ENABLED`
            // so the whole statement folds away without the feature. While a
            // button-delay window is open the pass is also a window sample —
            // whether the pointer moved while it stood open, the measurement
            // for `mchange`'s forced close (`aes/geminput.c:360-370`).
            if super::trace::ENABLED {
                super::trace::note_eval();
                if super::input::bdely_pending() {
                    super::trace::note_click_window();
                }
            }
            // The Control Manager — the `menu_intercept`/`window::intercept`
            // pair, mirroring the EmuTOS `ctlmgr` process (`gemctrl.c:427-486`)
            // — runs here, inline, ahead of the application's own checks. It
            // stays out of the way while it is parked in `ct_msgup`'s wait for
            // the press that selected a menu item to come up
            // (`gemctrl.c:100-101`): no interception, no `w_setactive`, until the
            // button is released.
            //
            // Nor does it run while **another process holds the screen lock**
            // (GH1): EmuTOS's `ctlmgr` is then asleep in its `ev_multi` or
            // blocked in the `wm_update(BEG_UPDATE)` that follows it
            // (`gemctrl.c:439-463`) — no bar track, no frame click, no
            // `w_setactive()` until the holder's `END_UPDATE`
            // (`window::ctl_may_run`). The events it would have acted on stay
            // pending: the pointer over the bar, the latched frame press.
            if !super::input::ct_msgup_busy() && super::window::ctl_may_run() {
                // AES intercepts a click in the menu bar or a window frame before
                // the app sees the raw button: it owns the mouse over the
                // menu/frame and posts the resulting MN_SELECTED / WM_* message
                // into the target's per-app FIFO via `ap_sendmsg`
                // (`gemctrl.c:81`, `gemwmlib.c:851`). The message reaches the app
                // through the regular MU_MESAG dequeue below — no direct `pbuff`
                // write, no bypass. Done only when the app actually requested
                // MU_MESAG: EmuTOS sets AP_MESAG (and processes the frame click)
                // solely for MU_MESAG (`gemsuper.c:147`); an app waiting on
                // MU_BUTTON alone is not woken by a frame click and must not
                // receive a spurious MU_MESAG (AUD-P2-003).
                if flags & MU_MESAG != 0 && !menu_intercept() {
                    super::window::intercept();
                }
                // The Control Manager's loop head, `w_setactive()`
                // (`aes/gemctrl.c:442`): after it has dealt with its event it
                // hands the mouse (back) to the owner of the top window —
                // `ct_chgown(ppd, &d)` → `set_mown` (`aes/gemwmlib.c:326-333`,
                // `aes/gemctrl.c:392-397`) — a no-op while a button is held
                // (`aes/geminput.c:222`) or while a menu track holds the mouse
                // (`gemctrl.c:395`). It sits **after** the interception, not
                // before: run first, it undid `mchange`'s hand-over to `ctl_pd`
                // (`aes/geminput.c:388-389`) before the track could inherit it,
                // and the track then returned with the application owning the
                // press it had just consumed.
                super::input::w_setactive();
            }
            // `pc = &rlr->p_cda->c_q; if (pc->c_cnt) …` (`aes/gemevlib.c:
            // 173-182`): the caller reads its own queue, and `kchange` fills
            // only the mouse owner's (`aes/geminput.c:288-296`) — a key is
            // taken only by the process that owns the mouse now, after the
            // Control Manager's `w_setactive` above (`docs/api/aes/event.md`
            // "Who gets a key").
            if flags & MU_KEYBD != 0 && super::input::owns_keys(idx) {
                if let Some(w) = h.console_try_read_key() {
                    key = w as i16; // scancode << 8 | ascii (EV_KRET)
                    fired |= MU_KEYBD;
                }
            }
            if flags & MU_BUTTON != 0 {
                // EmuTOS runs the button/rectangle **quick check** only while
                // the calling process owns the mouse — `if (rlr == gl_mowner)`
                // (`aes/gemevlib.c:185`) — and only once, at the call's entry:
                // our loop pass 0, on the ownership and level sampled at arm
                // time (`arm_own`/`arm_mb`, above). It tries the recorded
                // state first — `if ((mtrans > 1) && downorup(pr_button,
                // buparm)) { … prets[5] = pr_mclick; }` — then the live
                // `button`, `prets[5] = mclick` (`aes/gemevlib.c:188-198`):
                // the count that transition was published with, not capped
                // to the request.
                //
                // If nothing at all fired in the quick checks, EmuTOS arms the
                // wait — `iasync(MU_BUTTON, buparm)` → `abutton`, an
                // ownership-blind level test answering `azombie(e, 1)`
                // (`aes/geminput.c:549-551`), else parked on the caller's
                // list and `gl_bpend++` for a multi-click request — below,
                // after the pass's other checks. Every later pass is the
                // "asleep" state: the wait fires only once `post_button`
                // completed it with a published transition, while we own the
                // mouse (`aes/geminput.c:344,463-483`).
                //
                // This is where a menu selection stays a message: the press
                // that picked the item was published under `gl_ctmown` with
                // the Control Manager owning the mouse, so no application
                // wait was completed by it, and on pass 0 `arm_mb` is the
                // level from before the track. The one exception EmuTOS keeps
                // stays too: an application that re-arms while the selecting
                // press is *still held* sees that level at arm time and gets
                // its one ownership-blind `MU_BUTTON` from `abutton`
                // (`docs/api/aes/menu.md`, "The one documented exception").
                if pass == 0 {
                    if arm_own {
                        if let Some((_, pr_mclick)) =
                            arm_pending.filter(|&(prb, _)| downorup(prb, b_flgclks, b_mask, b_state))
                        {
                            clicks = pr_mclick;
                            fired |= MU_BUTTON;
                        } else if downorup(arm_mb, b_flgclks, b_mask, b_state) {
                            clicks = arm_mclick;
                            fired |= MU_BUTTON;
                        }
                    }
                } else if let Some(c) = super::input::bwait_take(idx) {
                    clicks = c; // apret(MU_BUTTON) (aes/gemevlib.c:276-277)
                    fired |= MU_BUTTON;
                }
            }
            // The rectangle quick checks sit inside the same
            // `if (rlr == gl_mowner)` block as the button one
            // (`aes/gemevlib.c:200-203`), and the blocking path behaves the same
            // way: `amouse` tests `in_mrect` once, ownership-blind, against the
            // position **at arm time** (`aes/geminput.c:568-587`), then parks
            // the wait on `c_msleep`, which only `post_mouse(gl_mowner, …)`
            // walks (`aes/geminput.c:390,446-456`) — from `mchange` on every
            // move while the caller owns the mouse, and once from `set_mown`
            // the moment ownership comes (back) to it (`:220-231`).
            //
            // Pass 0 is the arm: the one ownership-blind test uses `arm_pos`,
            // the position from before the inline Control Manager ran — a
            // rectangle the drop-down track crossed on its own account never
            // reaches the application's parked wait (`post_mouse(ctl_pd, …)`
            // walks `ctl_pd`'s list). If that track handed the mouse back
            // before the application runs (`w_setactive` → `set_mown(app)` →
            // `post_mouse(app, xrat, yrat)`, `aes/gemctrl.c:441-442`; a
            // selection on *release*), the parked wait is completed against
            // the live position — `own_mouse()` is true again, and the live
            // test is that `post_mouse`. Every later pass is the parked state:
            // live position, owner only. An unsatisfied arm that another event
            // (the track's `MN_SELECTED`) wakes the caller from is simply
            // dropped, `acancel` (`aes/gemevlib.c:264`) — nothing re-tests it
            // (A73, point 1; `docs/api/aes/event.md` "tested once, at arm time").
            let own_m = super::input::own_mouse();
            let rect_fires = |m: &MoBlk| {
                (pass == 0 && rect_event_at(m, arm_pos)) || (own_m && rect_event(m))
            };
            if flags & MU_M1 != 0 && rect_fires(&m1) {
                fired |= MU_M1;
            }
            if flags & MU_M2 != 0 && rect_fires(&m2) {
                fired |= MU_M2;
            }
            if flags & MU_MESAG != 0 {
                // SAFETY: single trap path; `by_index` hands out the live slot.
                if let Some(src) = unsafe { pd::by_index(idx) } {
                    // `MME_PBUFF`, checked writable for the 16-byte message
                    // with the caller's rights once there is one to copy —
                    // where EmuTOS's `doq` writes it (ARCH03 3a). One the
                    // program may not write ends the call (and the program)
                    // with the message still queued.
                    if src.queue.pending() > 0
                        && pbuff != 0
                        && pb.ubuf(pbuff, 2 * pd::MSG_WORDS as u64, true).is_none()
                    {
                        sched::clear_deadline();
                        set_ev_armed(idx, false);
                        super::input::acancel(idx);
                        return None;
                    }
                    // Pop the FRONT of the FIFO (EmuTOS `ap_rdwr(MU_RDMSG)`
                    // →`doq` dequeue, `gemqueue.c:76-81`); a single pending
                    // message is just the 1-element case.
                    let had = src.queue.bytes();
                    if let Some(msg) = src.queue.pop() {
                        // `ev_mesag` → `ap_rdwr(MU_MESAG, rlr, 16, …)`
                        // (`gemevlib.c:116`): the read services a parked
                        // writer unless it took the fast path (A116).
                        // SAFETY: single trap path; `idx`'s own queue.
                        unsafe { pd::after_read(idx, &mut src.queue, 2 * pd::MSG_WORDS, had) };
                        if pbuff != 0 {
                            // Checked just above; written in the caller's space.
                            for (i, &w) in msg.iter().enumerate() {
                                if pb.space().put_u16(pbuff + 2 * i as u64, w as u16).is_err() {
                                    sched::clear_deadline();
                                    set_ev_armed(idx, false);
                                    super::input::acancel(idx);
                                    return None;
                                }
                            }
                        }
                        // `if ((ret & MU_MESAG) && (*(WORD *)MME_PBUFF ==
                        // AC_CLOSE)) rlr->p_flags |= AP_ACCLOSE;`
                        // (`gemsuper.c:160-161`) — the AES sets the flag, never
                        // the accessory; `wait_for_accs(AP_ACCLOSE)` reads it.
                        if msg[0] == AC_CLOSE {
                            src.flags |= pd::AP_ACCLOSE;
                        }
                        fired |= MU_MESAG;
                    }
                }
            }
            if flags & MU_TIMER != 0 && h.monotonic_millis() >= deadline {
                fired |= MU_TIMER;
            }

            // `if (what == 0) { … iasync(MU_BUTTON, buparm); … }`
            // (`aes/gemevlib.c:231-239`): nothing fired in the quick checks,
            // so the button wait is armed — `abutton`, after the keyboard and
            // before the rectangles, whose ownership-blind arm-time test
            // (`amouse`) therefore does not prevent it. A call re-executed
            // after a peer yield (S2) finds its wait still parked, or completed
            // in the meantime, instead of arming a second one.
            if pass == 0 && flags & MU_BUTTON != 0 {
                let quick = if arm_own { fired } else { fired & !(MU_M1 | MU_M2) };
                if quick == 0 {
                    let got = if super::input::bwait_armed(idx) {
                        super::input::bwait_take(idx)
                    } else {
                        super::input::abutton(idx, arm_mb, b_flgclks, b_mask, b_state)
                    };
                    if let Some(c) = got {
                        clicks = c;
                        fired |= MU_BUTTON;
                    }
                }
            }

            if fired != 0 {
                break;
            }
            // Not fired: sleep on the **union** of the armed events (A112;
            // `iasync` per flag + `mwait(flags)`, `gemevlib.c:235-263`). Each
            // event has its own wake source, independent of who is running
            // (`docs/api/aes/event.md` "Wake conditions and dispatch"):
            //   * `MU_TIMER` — the slot's own deadline (persisted above),
            //     completed by the dispatcher's timer pass;
            //   * `MU_MESAG` — a write into this identity's FIFO by any
            //     process (`pd::post_msg`/`post_words` wake `WAIT_MESG(idx)`);
            //   * input — `MU_KEYBD`/`MU_BUTTON`/`MU_M1`/`MU_M2`, and for
            //     `MU_MESAG` also the inline Control Manager above, which turns
            //     a menu/frame click into a message inside *this* loop (in
            //     EmuTOS a process of its own); owner gating stays with the
            //     checks above, so a woken non-owner simply sleeps again.
            //
            // The loop above the block is read-only (a message is popped only on
            // the `fired |= MU_MESAG` firing pass), so on a peer yield (S2) it is
            // safe to abort and re-execute on resume (§4.3); the arm itself is
            // recorded so the re-executed call does not repeat it (F4).
            let mut events = 0;
            if flags & (MU_KEYBD | MU_BUTTON | MU_M1 | MU_M2 | MU_MESAG) != 0 {
                events |= sched::WAKE_INPUT;
            }
            if flags & MU_MESAG != 0 {
                events |= sched::WAKE_MESG;
            }
            if flags & MU_TIMER != 0 {
                events |= sched::WAKE_TIMER;
            }
            set_ev_armed(idx, true);
            if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_multi(events, idx)) {
                return Some(sw);
            }
            pass = pass.saturating_add(1);
        }
    }
    set_ev_armed(idx, false);

    // `which |= acancel(flags)` (`aes/gemevlib.c:264`): a button wait still
    // parked is taken off unsatisfied — leaving `gl_bpend` as it is.
    super::input::acancel(idx);
    // Completed (an event fired, or no host): clear the persisted timer deadline
    // so a later timer wait recomputes fresh — any firing path reaches here,
    // whereas a peer yield above returned early and left the deadline armed for
    // the restart (S2-restart idempotency, §4.3).
    sched::clear_deadline();
    fill_rets(pb);
    pb.set_intout(5, key); // EV_KRET
    pb.set_intout(6, clicks); // EV_BRET
    pb.set_ret(fired as i16);
    None
}

/// `evnt_keybd` (20) — block until a key is pressed; return it in
/// `RET_CODE` as `scancode << 8 | ascii`.
///
/// The key word comes from the IKBD key ring (`console_try_read_key`): the
/// low byte is the ASCII character (0 for keys with none), the high byte
/// the Atari scancode, so cursor / function keys reach the application.
/// The GEMDOS console path still reads the ASCII byte only.
///
/// `ev_block(MU_KEYBD)` → `akbin` reads the caller's own queue
/// (`aes/geminput.c:486-493`), which `kchange` fills only while the caller owns
/// the mouse (`:288-296`): a key is taken only by the current key owner
/// ([`super::input::owns_keys`]). The Control Manager's loop head runs on each
/// pass, as in [`evnt_mouse`], so ownership a hover over the bar gave to
/// `ctl_pd` comes back (`docs/api/aes/event.md` "Who gets a key").
pub fn evnt_keybd(pb: &Aespb) -> Option<sched::Switch> {
    let idx = super::appl::current_identity();
    let key = match host() {
        Some(h) => loop {
            if !super::input::ct_msgup_busy() && super::window::ctl_may_run() {
                super::input::w_setactive();
            }
            if super::input::owns_keys(idx) {
                if let Some(w) = h.console_try_read_key() {
                    break w as i16; // scancode << 8 | ascii
                }
            }
            if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_input()) {
                return Some(sw); // yield to a peer (S2); re-checks on resume
            }
        },
        None => 0,
    };
    pb.set_ret(key);
    None
}

/// `evnt_mesag` (23) — block until a 16-byte message is in the calling
/// application's pipe, copy it to `addr_in[0]` (`ME_PBUFF`), set the
/// `AP_MESAG` flag, and return 1.
///
/// Bring-up note: with a single AES application and no window/menu
/// managers posting messages yet, the only writer is `appl_write`; the
/// wait returns as soon as a message is pending. A lone application
/// waiting on a message that no one will post blocks by design — it
/// asked to wait. Multi-application posting lands with the window/menu
/// managers (later D-deliverables).
pub fn evnt_mesag(pb: &Aespb) -> Option<sched::Switch> {
    let dest = pb.addrin(0);
    let idx = super::appl::current_identity();
    // `rlr->p_flags |= AP_MESAG;` **before** `ev_mesag` (`gemsuper.c:137`) —
    // "the accessory has issued at least one wait for a message"
    // (`geminit.c:574-578`), which is what `wait_for_accs(AP_MESAG)` waits for.
    // It must therefore be set on *entry*, not on delivery: at boot no message
    // has been sent yet and the barrier would never lift.
    // SAFETY: single trap path; `by_index` hands out the live descriptor.
    if let Some(p) = unsafe { pd::by_index(idx) } {
        p.flags |= pd::AP_MESAG;
    }
    if host().is_none() {
        pb.set_ret(1);
        return None;
    }
    loop {
        // SAFETY: single trap path; `by_index` hands out the live slot.
        let Some(src) = (unsafe { pd::by_index(idx) }) else {
            // No descriptor — only an identity outside the table, which
            // `current_identity` never produces: nothing to wait on; report
            // the message event and bail.
            pb.set_ret(1);
            return None;
        };
        // `ME_PBUFF`, checked writable for the 16-byte message with the
        // caller's rights once there is one to copy (ARCH03 3a); one the
        // program may not write ends the call with the message still queued.
        if src.queue.pending() > 0
            && dest != 0
            && pb.ubuf(dest, 2 * pd::MSG_WORDS as u64, true).is_none()
        {
            return None;
        }
        // Pop the FRONT of the FIFO (EmuTOS `ap_rdwr(MU_RDMSG)`→`doq` dequeue,
        // `gemqueue.c:76-81`).
        let had = src.queue.bytes();
        if let Some(msg) = src.queue.pop() {
            // `ap_rdwr(MU_MESAG, rlr, 16, …)` (`gemevlib.c:116`): the read
            // services a parked writer unless it took the fast path (A116).
            // SAFETY: single trap path; `idx`'s own queue.
            unsafe { pd::after_read(idx, &mut src.queue, 2 * pd::MSG_WORDS, had) };
            if dest != 0 {
                // Checked just above; written in the caller's space.
                for (i, &w) in msg.iter().enumerate() {
                    if pb.space().put_u16(dest + 2 * i as u64, w as u16).is_err() {
                        return None;
                    }
                }
            }
            // `if (*(WORD *)ME_PBUFF == AC_CLOSE) rlr->p_flags |= AP_ACCLOSE;`
            // (`gemsuper.c:139-140`).
            if msg[0] == AC_CLOSE {
                src.flags |= pd::AP_ACCLOSE;
            }
            pb.set_ret(1);
            return None;
        }
        // No message yet: block until one is posted to this identity's FIFO
        // (`pd::post_msg` wakes `WAIT_MESG(idx)`, design §5.2). The pop above
        // runs only when a message is present, so aborting to a peer (S2) and
        // re-executing on resume never double-pops (§4.3).
        if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_mesg(idx)) {
            return Some(sw);
        }
    }
}

/// `evnt_timer` (24) — block for `ms` milliseconds, then return.
///
/// `int_in[0]` = low WORD of the millisecond count, `int_in[1]` = high
/// WORD (`T_LOCOUNT`/`T_HICOUNT`). `RET_CODE` is 1 (the timer event).
pub fn evnt_timer(pb: &Aespb) -> Option<sched::Switch> {
    let lo = pb.intin(0) as u16 as u32;
    let hi = pb.intin(1) as u16 as u32;
    let ms = ((hi << 16) | lo) as u64;

    if let Some(sw) = wait_ms(ms) {
        return Some(sw); // yield to a peer (S2); reuses the deadline on resume
    }
    // Completed (deadline reached): clear the persisted deadline
    // so a later timer wait recomputes fresh (S2-restart idempotency, §4.3).
    sched::clear_deadline();
    pb.set_ret(1);
    None
}

/// Block for `ms` milliseconds against the host's monotonic clock. A count
/// below one AES tick — **0 included** — waits for the next tick boundary:
/// `ev_timer` → `ev_block(MU_TIMER, ms / gl_ticktime)` → `adelay`, which
/// raises 0 ticks to 1 (`gemevlib.c:134-137`, `geminput.c:500-501`;
/// [`timer_span`], A112). `evnt_timer(0)` is therefore not a poll. With no
/// host installed the clock stands still, so this returns at once.
fn wait_ms(ms: u64) -> Option<sched::Switch> {
    if let Some(h) = host() {
        let now = h.monotonic_millis();
        let ms = timer_span(now, ms);
        // Persist the absolute deadline per-slot so an S2 restart REUSES it
        // instead of recomputing `now + ms` — which, re-entered from the top on
        // every restart with a live peer, would creep forward by `ms` and never
        // converge (design §4.3 OPEN). First entry stores; a restart reuses;
        // `evnt_timer` clears it on completion. `monotonic_millis` is monotonic
        // so a plain comparison is safe (no wraparound within any realistic
        // uptime).
        let deadline = sched::arm_or_get_deadline(now, ms);
        // Block until the deadline. The idle wake-pass wakes `WAIT_TIMER` once
        // `monotonic_millis >= deadline` (design §5.2); the caller's own
        // deadline check drives the loop in the single-context step-3a case.
        while h.monotonic_millis() < deadline {
            if let sched::SleepOutcome::Yield(sw) = sched::sleep(sched::wait_timer(deadline)) {
                return Some(sw); // yield to a peer (S2); reuses the deadline on resume
            }
        }
    }
    None
}
