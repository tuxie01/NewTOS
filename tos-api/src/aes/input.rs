//! AES input layer — mouse ownership and the button-delay window.
//!
//! Source: EmuTOS `aes/geminput.c`. This is the small part of that file our
//! architecture needs: the state the Event Manager consults (`gl_mowner`,
//! `button`, `xrat`/`yrat`, `gl_bdely`) and the two interrupt-side handlers
//! that maintain it, `b_click` (`:110-140`) and `mchange` (`:360-373`).
//!
//! **How EmuTOS gets here.** The AES installs its own handlers into the VDI's
//! mouse-interrupt vectors — `gsx_setmb(far_bcha, far_mcha, &drwaddr)`
//! (`aes/gemgsxif.c:271`) is `vex_butv`/`vex_motv`. The VDI's `mouse_int` then
//! calls `user_but` on a button change and `user_mot` on movement, the latter
//! with the **clipped** new position (`vdi/vdi_asm.S:271-284` and `:339-362`);
//! `far_bcha` calls `b_click`, `far_mcha` forks `mchange`
//! (`aes/gemdosif.S:306-347`). Our equivalent of `mouse_int` is
//! [`crate::vdi::feed_relative`], which calls [`b_click`] and [`mchange`] at the
//! same two points and with the same arguments.
//!
//! **How the delivery half is reproduced.** EmuTOS's `bchange`/`b_delay`
//! deliver a button change through the fork queue into the per-process
//! event-block lists: `post_button` (`:463-483`) walks the mouse owner's
//! `c_bsleep` list and completes every wait the new state satisfies. Our
//! Event Manager has no fork queue, but it keeps that list: one button wait
//! per AES identity ([`abutton`], [`acancel`], [`bwait_take`]), completed by
//! [`post_button`] from inside [`bchange`] — so a sleeping `MU_BUTTON` wait is
//! satisfied by a *published transition*, never by sampling the level, and
//! the click count it returns is the one the transition carried.
//!
//! **The button-delay window runs on the AES tick, not in a caller.** EmuTOS
//! opens it in `b_click` on a press while any waiter asks for more than one
//! click (`gl_bpend`), counts it down by one per AES timer tick (`b_delay(1)`
//! from `tikcod`, `aes/gemdosif.S:389-411`; one tick = `gl_ticktime` = the
//! VDI's `Tickcal`, 20 ms), extends it by three ticks for every further click
//! (`gl_bdely += 3`, `:121-122`) and publishes the gesture only when it
//! closes. Here the countdown is evaluated lazily against the host clock in
//! whole 20 ms ticks ([`b_delay_poll`]) at every point that can observe it —
//! each mouse packet ([`b_click`], [`mchange`]) and each read of the state —
//! which yields the same transitions in the same order as the interrupt-side
//! countdown.
//!
//! **The state at the event (`mtrans`/`pr_*`).** `bchange` records, for every
//! button transition it publishes, the state *before* it — `pr_button`,
//! `pr_mclick`, `pr_xrat`/`pr_yrat` — and counts the transitions in `mtrans`
//! (`aes/geminput.c:333-345`); `ev_rets` reports that recorded state whenever
//! more than one transition has happened since the last event return, and the
//! live state otherwise, then clears the count (`aes/gemevlib.c:42-58`). A
//! click whose release lands inside the double-click window is published as
//! two transitions at once when the window closes (`b_delay`,
//! `aes/geminput.c:155-171`), so its event reports the button **down**. That
//! is [`bchange`], [`ev_rets`] and the window close [`b_delay`] here; see
//! `docs/api/aes/event.md` "The button state an event reports".
//!
//! **The Control Manager's two locks over the mouse.** Beside `gl_mowner` this
//! module also carries the Control Manager's `gl_ctmown` (`aes/gemctrl.c:50`),
//! the flag `mn_do` raises for the whole of a menu-bar track (`ct_mouse(TRUE)`
//! / `ct_mouse(FALSE)`, `aes/gemmnlib.c:277,513`; `aes/gemctrl.c:400-417`) so
//! that a press during the track does not move the mouse away from `ctl_pd`
//! (`bchange`, `aes/geminput.c:334-335`; `ct_chgown`, `aes/gemctrl.c:395`) —
//! and the state of `ct_msgup`'s `while (button & 0x0001) dsptch();`
//! (`aes/gemctrl.c:100-101`), the wait that keeps the Control Manager from
//! looping again until the press that selected a menu item has been released.
//! Together they are the contract of `docs/api/aes/menu.md` "Mouse ownership
//! over the track": the selecting press and its release reach the application
//! only as the one `MN_SELECTED`, never as `MU_BUTTON`.
//!
//! Single boot CPU (the AES is entered only from the GEM trap, these two hooks
//! only from the IKBD decode path), so plain `Relaxed` atomics suffice, matching
//! the rest of the AES state.

use core::sync::atomic::{AtomicBool, AtomicI16, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

use super::pd::{KERNEL_CONTEXT, NUM_PDS};
use tos_types::aes::{WA_DNLINE, WA_UPLINE, WM_ARROWED};

/// EmuTOS `MB_DOWN` (`aes/geminput.c:46`) — the left button, the only state
/// that moves mouse ownership.
const MB_DOWN: u8 = 0x01;

/// EmuTOS `button` — the button state the AES has acted on. Maintained by
/// `bchange` (`aes/geminput.c:342`); here by [`b_click`], which runs
/// `bchange`'s ownership decision inline because our Event Manager has no fork
/// queue to defer it to.
static BUTTON: AtomicU8 = AtomicU8::new(0);

/// EmuTOS `xrat`/`yrat` — the **last reported** (clipped) pointer position, the
/// baseline `mchange` measures its ±2 threshold against (`aes/geminput.c:362`).
/// Deliberately not `crate::vdi::mouse_position()`: that is the *current*
/// position, which by the time `mchange` runs is already the new one.
static XRAT: AtomicI16 = AtomicI16::new(0);
static YRAT: AtomicI16 = AtomicI16::new(0);

/// EmuTOS `gl_mowner == rlr` — `true` while the running application owns the
/// mouse, `false` while the Control Manager does.
///
/// EmuTOS holds an `AESPD *`; with one AES application the only two values it
/// can take are "the app" and `ctl_pd`, so a flag carries the whole state.
/// Initially the app: `geminit.c:888` starts `gl_mowner` at `ctl_pd`, but the
/// Control Manager's very first loop pass calls `w_setactive` →
/// `ct_chgown(D.w_win[w_top()].w_owner, …)` → `set_mown` before anything can
/// observe it (`aes/gemctrl.c:441`), and at single-app that owner is the
/// application.
static GL_MOWNER_APP: AtomicBool = AtomicBool::new(true);

/// The **application half** of EmuTOS `gl_mowner` — *"current mouse/keybd
/// owner"* (`aes/geminput.c:52`): which AES identity the mouse, and with it
/// every key (`kchange` → `post_keybd(gl_mowner->p_cda, ch)`, `:288-296`),
/// belongs to while [`GL_MOWNER_APP`] says "an application". Written at the
/// same places as the flag, under the same guards (`set_mown`, `bchange`'s
/// `chkown`); read by [`owns_keys`]. Identities are compared as EmuTOS's
/// `AESPD`s are, see [`same_pd`]. Power-up value: the application chain,
/// represented by [`KERNEL_CONTEXT`] (the first `w_setactive` hands the mouse to the
/// desktop window's owner, `rlr` at `wm_init`, `aes/gemwmlib.c:1254-1290`).
static GL_MOWNER_PD: AtomicUsize = AtomicUsize::new(KERNEL_CONTEXT);

/// The application half of EmuTOS `gl_cowner` (`aes/geminput.c:63`) — the
/// owner of the control rectangle, set together with `gl_mowner` by
/// `set_mown` (`:225`) and handed out again by `chkown` for a press inside
/// the control rectangle (`:320-322`).
///
/// `set_mown(ctl_pd)` — `fm_own(FALSE)` handing the mouse back to a
/// remembered `ctl_pd` — would make `gl_cowner` the Control Manager too; that
/// half is not carried: a press inside the control rectangle stays an
/// application's, as the flag has always answered it (`chkown`'s `+1`).
static GL_COWNER_PD: AtomicUsize = AtomicUsize::new(KERNEL_CONTEXT);

/// A value of EmuTOS `gl_mowner` as `get_mown`/`set_mown`/`ct_chgown` pass it:
/// the Control Manager (`ctl_pd`) or an application process, named by its AES
/// identity (`appl::current_identity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mown {
    /// `ctl_pd`, the Screen Manager.
    Ctl,
    /// An application process — its AES identity.
    Pd(usize),
}

impl Mown {
    /// `ctl_pd` in [`Mown::to_raw`]'s encoding — not an AES identity
    /// (identities are GEMDOS slots and [`KERNEL_CONTEXT`]).
    const RAW_CTL: usize = usize::MAX;

    /// One word for an atomic (`fm_own`'s `ml_pmown`).
    pub(super) fn to_raw(self) -> usize {
        match self {
            Mown::Ctl => Self::RAW_CTL,
            Mown::Pd(p) => p,
        }
    }

    /// The inverse of [`Mown::to_raw`].
    pub(super) fn from_raw(raw: usize) -> Mown {
        if raw == Self::RAW_CTL {
            Mown::Ctl
        } else {
            Mown::Pd(raw)
        }
    }
}

/// Do two AES identities name the same EmuTOS `AESPD`? EmuTOS runs the
/// desktop, every launched program and every `Pexec` child on **one**
/// descriptor, `D.g_int[0]` (`aes/geminit.c:851-877`, `gemshlib.c:572`), and
/// each desk accessory on one of its own (`D.g_acc[]`, `geminit.c:819`).
/// NewTOS gives every GEMDOS slot and the kernel context an identity of its
/// own (A62); for ownership they fold back: a registered accessory
/// ([`super::acc::is_process`]) is itself, every other identity is the one
/// application chain.
pub(super) fn same_pd(a: usize, b: usize) -> bool {
    a == b || (!super::acc::is_process(a) && !super::acc::is_process(b))
}

/// May the AES identity `idx` take a key now? EmuTOS's `kchange` posts every
/// key to `gl_mowner` and a wait reads only its own queue (`ev_multi`'s
/// `rlr->p_cda->c_q`, `aes/gemevlib.c:173-182`; `akbin`,
/// `aes/geminput.c:486-493`), so a key is the waiter's exactly when the
/// waiter owns the mouse: not while `ctl_pd` does, and not while another
/// process does (`docs/api/aes/event.md` "Who gets a key").
///
/// NewTOS reads the key out of the shared ring at that moment instead of
/// moving it into a per-process queue when it arrives (the ring is also the
/// GEMDOS console's; spec, "Where NewTOS differs").
pub(super) fn owns_keys(idx: usize) -> bool {
    owns_mouse(idx)
}

/// EmuTOS `rlr == gl_mowner` for the AES identity `idx` — does `idx`'s own
/// `AESPD` own the mouse ([`same_pd`]), not `ctl_pd` and not another process?
/// The gate of `ev_multi`'s button and rectangle quick checks (`if (rlr ==
/// gl_mowner)`, `aes/gemevlib.c:185`) and of the parked rectangle waits, which
/// only `post_mouse(gl_mowner, …)` walks (`aes/geminput.c:390,446-456`) —
/// A118: they asked [`own_mouse`] ("an application owns it") before.
pub(super) fn owns_mouse(idx: usize) -> bool {
    GL_MOWNER_APP.load(Ordering::Relaxed) && same_pd(GL_MOWNER_PD.load(Ordering::Relaxed), idx)
}

/// EmuTOS `gl_ctmown` (`aes/gemctrl.c:50`) — the Control Manager holds the
/// mouse across a menu-bar track. Raised and lowered by [`ct_mouse`] from
/// [`menu::track`](super::menu::track), EmuTOS `mn_do`'s `ct_mouse(TRUE)` on
/// entry and `ct_mouse(FALSE)` on every exit (`aes/gemmnlib.c:277,513`). While
/// set, a fresh button-down does **not** re-assign the owner ([`b_click`],
/// `bchange`'s `if (!gl_ctmown && …)`, `aes/geminput.c:334-335`) and the
/// Control Manager's loop head does not hand the mouse back ([`w_setactive`],
/// `ct_chgown`'s `if (!gl_ctmown) set_mown(mpd);`, `aes/gemctrl.c:395-396`).
static GL_CTMOWN: AtomicBool = AtomicBool::new(false);

/// The Control Manager is parked in `ct_msgup`'s `while (button & 0x0001)
/// dsptch();` (`aes/gemctrl.c:100-101`): a menu selection has been posted and
/// the press that made it is still held. Until that press is released the
/// Control Manager neither loops (`w_setactive`) nor looks at the button again
/// — so the held button cannot be taken for a fresh press on a window frame,
/// nor re-open the bar. Raised by [`ct_msgup_hold`] (from the menu
/// interception, right after `MN_SELECTED` is posted), lowered by
/// [`ct_msgup_busy`] the first time it sees the button up.
static CT_MSGUP: AtomicBool = AtomicBool::new(false);

/// EmuTOS `gl_bdely` — the ticks left before the open button-delay window
/// closes (`aes/geminput.c:59-60`); 0 = no window open. Set to `gl_dclick` by
/// the press that opens it, `+= 3` per further click (`:121-122`), counted
/// down by [`b_delay`].
static GL_BDELY: AtomicI16 = AtomicI16::new(0);

/// The AES tick ([`now_tick`]) up to which [`GL_BDELY`] has been counted down
/// — the lazy stand-in for EmuTOS calling `b_delay(1)` on every timer tick.
static LAST_TICK: AtomicU64 = AtomicU64::new(0);

/// EmuTOS `gl_bpend` (`aes/geminput.c:64-65`) — "number of pending events
/// desiring more than a single click". `abutton` increments it when it parks a
/// wait for more than one click (`:559-561`); `post_button` decrements it when
/// it completes one (`:477-479`). A wait that `acancel` takes off unsatisfied
/// (`takeoff`, `aes/gemasync.c:96-114,217-243`) does **not** decrement it, so
/// every application whose multi-click wait is ever ended by another event (a
/// timer, a message, a key) leaves it above zero for the rest of the session —
/// and from then on every press opens the delay window. Not touched by an AES
/// restart: `run_accs_and_desktop` resets the four window variables, not this
/// one (`aes/geminit.c:661-664`).
static GL_BPEND: AtomicI16 = AtomicI16::new(0);

/// The per-identity `c_bsleep` list: the one `MU_BUTTON` wait an AES identity
/// can have parked (`abutton`, `aes/geminput.c:545-565`). [`BW_IDLE`] = none,
/// [`BW_ARMED`] = parked and unsatisfied, a value `>= 0` = completed by
/// [`post_button`] with that click count (`evremove(e, …)` → `azombie`), not
/// yet collected.
static BWAIT: [AtomicI32; NUM_PDS] = [const { AtomicI32::new(BW_IDLE) }; NUM_PDS];
/// The parked wait's `buparm`: the clicks WORD in the high half, mask and
/// state in the low half (`combine_cms`, `aes/gemevlib.h:34-48`).
static BWAIT_PARM: [AtomicU32; NUM_PDS] = [const { AtomicU32::new(0) }; NUM_PDS];
const BW_IDLE: i32 = -1;
const BW_ARMED: i32 = -2;

/// The Control Manager's loop wait (`ctlmgr`'s `ev_multi(MU_BUTTON, …,
/// 0x0001ff01L)`, `aes/gemctrl.c:456-462`) has been completed by a published
/// press while it owned the mouse (`post_button(ctl_pd, …)`) and not yet acted
/// on. Its inline stand-in [`window::intercept`](super::window::intercept)
/// otherwise tests the level of `button`, which misses a click whose press and
/// release a button-delay window published together. Taken by
/// [`ctl_press_take`]; dropped by [`ev_consume`] and [`reinit`].
static CTL_PRESS: AtomicBool = AtomicBool::new(false);

/// EmuTOS `gl_btrue` — the true (physical) button state as `b_click` last saw
/// it (`aes/geminput.c:144-145`). Equal to [`BUTTON`] except while a
/// button-delay window is open (the changes inside it are not published until
/// it closes) and after [`ct_msgup_hotclose`] faked the AES's `button` up.
static GL_BTRUE: AtomicU8 = AtomicU8::new(0);

/// EmuTOS `gl_bdesired` — the state the open delay window counts entries into
/// (`aes/geminput.c:137`): the press that opened it.
static GL_BDESIRED: AtomicU8 = AtomicU8::new(0);

/// EmuTOS `gl_bclick` — the number of times the window's desired state was
/// entered while it was open, the opening press included
/// (`aes/geminput.c:121-122,136`).
static GL_BCLICK: AtomicI16 = AtomicI16::new(0);

/// EmuTOS `mclick` — the click count of the last published transition
/// (`aes/geminput.c:343`).
static MCLICK: AtomicI16 = AtomicI16::new(0);

/// EmuTOS `mtrans` — button transitions published since the last event return
/// (`bchange`'s `mtrans++`, `aes/geminput.c:337`; `ev_rets`'s `mtrans = 0`,
/// `aes/gemevlib.c:57`). Only "more than one" is observable: it selects the
/// recorded `pr_*` state over the live one.
static MTRANS: AtomicI16 = AtomicI16::new(0);

/// EmuTOS `pr_button`/`pr_mclick`/`pr_xrat`/`pr_yrat` — the AES's button,
/// click count and pointer position as they stood just before the last
/// published transition (`aes/geminput.c:338-341`).
static PR_BUTTON: AtomicU8 = AtomicU8::new(0);
static PR_MCLICK: AtomicI16 = AtomicI16::new(0);
static PR_XRAT: AtomicI16 = AtomicI16::new(0);
static PR_YRAT: AtomicI16 = AtomicI16::new(0);

/// The input layer's share of an AES restart (A106, [`super::reinit`]).
///
/// Two EmuTOS steps, in `gem_main`/`run_accs_and_desktop` order:
///
/// * **the button variables** (`geminit.c:661-664`): `gl_btrue = gl_bdesired
///   = gl_bdely = gl_bclick = 0` — here [`BUTTON`] and [`GL_BDELY`], plus
///   [`XRAT`]/[`YRAT`], the last reported pointer position `mchange`
///   measures against (`geminput.c:362`), which is part of the same
///   power-up-zero set (`theglo`-adjacent BSS, `gemstart.S:101-110`).
/// * **the mouse owner** (`geminit.c:888`): `gl_mowner = ctl_pd =
///   iprocess("SCRENMGR", ctlmgr)`, whose first Control-Manager pass hands
///   the mouse to the application before anything can observe it
///   (`gemctrl.c:441`) — which is why [`GL_MOWNER_APP`]'s power-up value is
///   `true`, and why that is what a restart puts back.
///
/// [`GL_CTMOWN`] and [`CT_MSGUP`] are the Control Manager's two track locks
/// (`gemctrl.c:50`, `:100-101`); a session that ended mid-track must not
/// leave either standing for the next one.
///
/// The window's `gl_btrue`/`gl_bdesired`/`gl_bclick` are the same
/// `geminit.c:661-664` set; `mclick`, `mtrans` and the `pr_*` record go back
/// to their power-up zero with `xrat`/`yrat`, so no transition of the ended
/// session is reported by the next one's first event. Every parked button
/// wait ([`BWAIT`]) belonged to a process of the ended session and is taken
/// off; [`GL_BPEND`] is left as it is (see there).
pub(super) fn reinit() {
    BUTTON.store(0, Ordering::Relaxed);
    GL_BDELY.store(0, Ordering::Relaxed);
    for w in BWAIT.iter() {
        w.store(BW_IDLE, Ordering::Relaxed);
    }
    CTL_PRESS.store(false, Ordering::Relaxed);
    GL_BTRUE.store(0, Ordering::Relaxed);
    GL_BDESIRED.store(0, Ordering::Relaxed);
    GL_BCLICK.store(0, Ordering::Relaxed);
    MCLICK.store(0, Ordering::Relaxed);
    MTRANS.store(0, Ordering::Relaxed);
    PR_BUTTON.store(0, Ordering::Relaxed);
    PR_MCLICK.store(0, Ordering::Relaxed);
    PR_XRAT.store(0, Ordering::Relaxed);
    PR_YRAT.store(0, Ordering::Relaxed);
    XRAT.store(0, Ordering::Relaxed);
    YRAT.store(0, Ordering::Relaxed);
    GL_MOWNER_APP.store(true, Ordering::Relaxed);
    GL_MOWNER_PD.store(KERNEL_CONTEXT, Ordering::Relaxed);
    GL_COWNER_PD.store(KERNEL_CONTEXT, Ordering::Relaxed);
    GL_CTMOWN.store(false, Ordering::Relaxed);
    CT_MSGUP.store(false, Ordering::Relaxed);
}

// ── the two VDI mouse-interrupt vectors ────────────────────────────────────

/// `user_but` — the AES's button-change handler, EmuTOS `b_click`
/// (`aes/geminput.c:110-147`).
///
/// `new` is the new button bitmask in VDI convention (bit 0 = left), the `d0`
/// the VDI passes to `user_but` (`vdi/vdi_asm.S:270-284`). Like `b_click` this
/// ignores anything that is not a change of the **true** state `gl_btrue`
/// (`:113`). Three cases, EmuTOS's:
///
/// * **a window is open** (`gl_bdely`): the change is only counted — an entry
///   into the window's desired state increments `gl_bclick` and extends the
///   window by three ticks, `gl_bdely += 3` (`:117-123`); nothing is
///   published until the window closes ([`b_delay`]);
/// * **a press while some waiter asks for more than one click**
///   (`if (gl_bpend && state)`, `:133-140`): the press opens the window —
///   `gl_bclick = 1; gl_bdesired = state; gl_bdely = gl_dclick` — and is
///   *not* published yet;
/// * **otherwise** it is published at once, `forkq(bchange,
///   MAKE_ULONG(state, 1))` (`:141-142`), here a direct [`bchange`] call.
///
/// The countdown is brought up to date first ([`b_delay_poll`]): a window
/// whose last tick has passed is closed before this change is looked at, as
/// the timer interrupt would have closed it.
pub(crate) fn b_click(new: u8) {
    b_delay_poll();
    // "ignore it unless it represents a change" (aes/geminput.c:112-113)
    if new == GL_BTRUE.load(Ordering::Relaxed) {
        return;
    }
    if GL_BDELY.load(Ordering::Relaxed) != 0 {
        // "if the change is into the desired state, increment cnt"
        // (aes/geminput.c:119-123)
        if new == GL_BDESIRED.load(Ordering::Relaxed) {
            GL_BCLICK.fetch_add(1, Ordering::Relaxed);
            GL_BDELY.fetch_add(3, Ordering::Relaxed);
        }
    } else if GL_BPEND.load(Ordering::Relaxed) != 0 && new != 0 {
        // "start click cnt at 1, establish desired state and set wait flag"
        // (aes/geminput.c:135-140)
        GL_BCLICK.store(1, Ordering::Relaxed);
        GL_BDESIRED.store(new, Ordering::Relaxed);
        GL_BDELY.store(super::event::gl_dclick(), Ordering::Relaxed);
    } else {
        let (x, y) = crate::vdi::mouse_position();
        bchange(new, 1, x, y);
    }
    // "update true state of the mouse" (aes/geminput.c:144-145)
    GL_BTRUE.store(new, Ordering::Relaxed);
}

/// The AES tick now: the host clock in whole `gl_ticktime` periods (20 ms,
/// the VDI's `Tickcal` that `gsx_tick` returns, `aes/geminit.c:704`;
/// [`crate::vdi::mouse::TICKCAL_MS`]). `None` without a host.
fn now_tick() -> Option<u64> {
    crate::host::host().map(|h| h.monotonic_millis() / crate::vdi::mouse::TICKCAL_MS as u64)
}

/// Bring the button-delay countdown up to the current tick: EmuTOS calls
/// `b_delay(1)` from its timer handler on every AES tick
/// (`aes/gemdosif.S:406-408`); here the ticks that passed since the last call
/// are applied in one [`b_delay`]. Called before anything reads or changes the
/// button state, so the window closes — and its transitions are published —
/// before any later event is looked at, exactly where the interrupt would have
/// put it.
pub(super) fn b_delay_poll() {
    let Some(t) = now_tick() else {
        return;
    };
    let last = LAST_TICK.swap(t, Ordering::Relaxed);
    let elapsed = t.saturating_sub(last);
    if elapsed > 0 {
        let (x, y) = crate::vdi::mouse_position();
        b_delay(elapsed.min(i16::MAX as u64) as i16, x, y);
    }
}

/// EmuTOS `b_delay(amnt)` (`aes/geminput.c:155-171`): count an open window
/// down by `amnt` ticks; when it reaches zero publish the gesture —
/// `bchange(gl_bdesired, gl_bclick)`, and `bchange(gl_btrue, 1)` as well if
/// the button has since left that state (`:160-170`). `(x, y)` is the AES's
/// pointer position when the forks run (`bchange` reads `xrat`/`yrat`): the
/// VDI cursor for the tick countdown, the new position for `mchange`'s forced
/// close (its forks run after it, see [`mchange`]). (EmuTOS's guard against a full fork ring, `fpcnt +
/// forkq_calls <= NFORKS`, has no counterpart: there is no ring to fill.)
fn b_delay(amnt: i16, x: i16, y: i16) {
    let d = GL_BDELY.load(Ordering::Relaxed);
    if d == 0 {
        return;
    }
    let d = d.saturating_sub(amnt).max(0);
    GL_BDELY.store(d, Ordering::Relaxed);
    if d == 0 {
        let desired = GL_BDESIRED.load(Ordering::Relaxed);
        bchange(desired, GL_BCLICK.load(Ordering::Relaxed), x, y);
        let btrue = GL_BTRUE.load(Ordering::Relaxed);
        if btrue != desired {
            bchange(btrue, 1, x, y);
        }
    }
}

/// EmuTOS `bchange` (`aes/geminput.c:329-345`) — publish one button
/// transition: `new` becomes the AES's `button`, with `clicks` as its click
/// count, and the state it replaces is recorded for [`ev_rets`]. `(x, y)` is
/// the AES's pointer position at that moment (`xrat`/`yrat`); the VDI cursor
/// position is used, which the packet carrying the change has not moved yet
/// (`user_but` runs before the position is touched, `vdi/vdi_asm.S:261-284`).
///
/// On the 0 → left-down edge `bchange` recomputes the mouse owner from the
/// pointer position: `if (!gl_ctmown && (new == MB_DOWN) && (button == 0))
/// gl_mowner = chkown();` — and `chkown` (`:310-322`) resolves
/// [`window::chk_ctrl`](super::window::chk_ctrl) to the active process (`+1`),
/// the Control Manager (`-1`) or the desktop window's owner (`0`); at
/// single-app the first and last are both this application. The owner is
/// latched here, at the press, and does not follow the pointer while the button
/// is held — that is what lets a drag start in a window and leave it.
///
/// The `!gl_ctmown` bracket is load-bearing: this handler runs from
/// [`menu::track`](super::menu::track)'s own `pump_input` while the drop-down
/// is being tracked, so the press that selects an item lands *here*, in the
/// middle of the track. With [`GL_CTMOWN`] raised the owner stays `ctl_pd`, and
/// the Event Manager's quick check for the application (`rlr == gl_mowner`,
/// `aes/gemevlib.c:185`) does not see that press — the H02 leak of the menu
/// click as `MU_BUTTON` (`TASK/H02-STATUS-DONE.MD` §4).
fn bchange(new: u8, clicks: i16, x: i16, y: i16) {
    let button = BUTTON.load(Ordering::Relaxed);
    // `if (!gl_ctmown && (new == MB_DOWN) && (button == 0)) gl_mowner = chkown();`
    // (aes/geminput.c:334-335)
    if !GL_CTMOWN.load(Ordering::Relaxed) && new == MB_DOWN && button == 0 {
        // chkown: +1 → gl_cowner, 0 → D.w_win[0].w_owner, −1 → ctl_pd
        // (aes/geminput.c:310-322).
        let val = super::window::chk_ctrl(x, y);
        GL_MOWNER_APP.store(val >= 0, Ordering::Relaxed);
        match val {
            1 => GL_MOWNER_PD.store(GL_COWNER_PD.load(Ordering::Relaxed), Ordering::Relaxed),
            0 => GL_MOWNER_PD.store(super::window::desk_owner(), Ordering::Relaxed),
            _ => {}
        }
    }
    // `mtrans++; pr_button = button; pr_mclick = mclick; pr_xrat = xrat;
    // pr_yrat = yrat; button = new; mclick = clicks;` (aes/geminput.c:337-343).
    // Saturating: only "> 1" is ever read, and a C `WORD` overflow is no
    // contract.
    let _ = MTRANS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |m| Some(m.saturating_add(1)));
    PR_BUTTON.store(button, Ordering::Relaxed);
    PR_MCLICK.store(MCLICK.load(Ordering::Relaxed), Ordering::Relaxed);
    PR_XRAT.store(x, Ordering::Relaxed);
    PR_YRAT.store(y, Ordering::Relaxed);
    BUTTON.store(new, Ordering::Relaxed);
    MCLICK.store(clicks, Ordering::Relaxed);
    // `post_button(gl_mowner, button, clicks);` (aes/geminput.c:344)
    post_button(new, clicks);
}

/// EmuTOS `post_button(gl_mowner, new, numclicks)` (`aes/geminput.c:463-483`)
/// — complete every parked button wait of the mouse owner that `new`
/// satisfies (`downorup(new, e->e_parm)`), with `(numclicks > clicks) ?
/// clicks : numclicks` as its result, the clicks being the wait's own
/// `LOBYTE(HIWORD(e_parm))`; a completed wait for more than one click
/// decrements `gl_bpend` (`:477-479`).
///
/// Only the owner's list is walked: with the Control Manager owning the mouse
/// (`gl_mowner == ctl_pd`) no application wait is completed, and with an
/// application owning it only **that process's** waits are — the identities
/// [`same_pd`] folds into the owner's `AESPD` ([`GL_MOWNER_PD`]: the whole
/// application chain, or one desk accessory). A press inside an accessory's
/// dialog therefore completes the accessory's wait and not the desktop's
/// parked `MU_BUTTON` (GH1). A completed waiter is woken through the
/// scheduler's input class, as `evremove` → `azombie` makes the process
/// runnable.
fn post_button(new: u8, numclicks: i16) {
    if !GL_MOWNER_APP.load(Ordering::Relaxed) {
        // The owner is `ctl_pd`. Outside a menu track (whose own `mn_do` waits
        // are the ones parked then, `aes/gemmnlib.c:597`) that is the Control
        // Manager's loop wait, `ev_multi(…, 0x0001ff01L, …)` — one click, left
        // button down (`aes/gemctrl.c:456-462`). Latch the press for its
        // inline stand-in, [`window::intercept`](super::window::intercept).
        if !GL_CTMOWN.load(Ordering::Relaxed) && downorup(new, 0x0001, 0xff, 0x01) {
            CTL_PRESS.store(true, Ordering::Relaxed);
        }
        return;
    }
    // `for (e = pd->p_cda->c_bsleep; …)` with `pd = gl_mowner`
    // (aes/geminput.c:463-469): the owner's own list.
    let owner = GL_MOWNER_PD.load(Ordering::Relaxed);
    let mut woke = false;
    for (idx, (w, parm)) in BWAIT.iter().zip(BWAIT_PARM.iter()).enumerate() {
        if !same_pd(owner, idx) || w.load(Ordering::Relaxed) != BW_ARMED {
            continue;
        }
        let p = parm.load(Ordering::Relaxed);
        let bflgclks = (p >> 16) as u16 as i16;
        if downorup(new, bflgclks, (p >> 8) as u8, p as u8) {
            let clicks = i16::from(bflgclks as u8);
            if clicks > 1 {
                GL_BPEND.fetch_sub(1, Ordering::Relaxed);
            }
            w.store(i32::from(if numclicks > clicks { clicks } else { numclicks }), Ordering::Relaxed);
            woke = true;
        }
    }
    if woke {
        crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_input());
    }
}

/// The button-wait predicate — EmuTOS `downorup` (`aes/geminput.c:348-357`).
///
/// `bflgclks` is the **clicks WORD** the caller passed (`B_CLICKS` /
/// `MB_CLICKS`), which `combine_cms` packs as the high WORD of the `buparm`
/// LONG (`aes/gemevlib.h:34-48`). It carries two fields: the low byte is the
/// number of clicks to wait for, the **high byte is `bflag`**, the sense bit.
/// `downorup` reads it as `flag = (buparm >> 24) & 0xff`, i.e. exactly that
/// high byte, and returns
///
/// ```text
/// (((mask & (val ^ new)) == 0) != flag)
/// ```
///
/// With `flag = 0` the caller waits to **enter** the masked state, with
/// `flag = 1` to **leave** it (`ev_button`, `aes/gemevlib.c:74-81`). TeraDesk's
/// main loop asks with `clicks = 0x0102` (`sdk/apps/teradesk/main.c:1589`), so
/// `flag = 1`, `mask = 3`, `state = 0`: "tell me when a button **is
/// pressed**" — the inverse of the plain level match.
///
/// The comparison is EmuTOS's literal one: a 0/1 condition compared against the
/// whole flag byte, so a `flag` above 1 makes the predicate constantly true,
/// exactly as it does in C.
pub(super) fn downorup(new: u8, bflgclks: i16, mask: u8, val: u8) -> bool {
    let flag = (bflgclks >> 8) & 0x00ff;
    let cond = i16::from((mask & (val ^ new)) == 0);
    cond != flag
}

/// `user_mot` — the AES's motion handler, EmuTOS `mchange`
/// (`aes/geminput.c:360-373`). `rx`/`ry` are the new **clipped** pointer
/// position, as the VDI passes them (`vdi/vdi_asm.S:355-362`); the VDI calls
/// this only for a packet that actually carried movement (`:305-320`).
///
/// Two things happen, in EmuTOS's order:
///
/// 1. *"zero out button wait if mouse moves more than a little"* — a move of
///    more than ±2 in either axis against the previous reported position forces
///    the pending button delay to expire immediately, `b_delay(gl_bdely)`
///    (`:365-368`). The clicks counted so far are delivered at once; a click
///    after the move starts a new gesture instead of completing a double click.
/// 2. The pointer entering the Control Manager's wait rectangle with the
///    buttons up hands the mouse to the Control Manager (`:387-389`), which is
///    how a hover menu takes over without a click. That rectangle is
///    `gl_ctwait` = the registered bar's `THEACTIVE` extent
///    (`aes/gemmnlib.c:717`; [`menu::in_ctwait`](super::menu::in_ctwait)) —
///    the titles' width, **not** the whole bar row: a pointer in the row right
///    of the last title stays with the application (A87). The same rectangle
///    is what [`event::menu_intercept`](super::event) tracks.
// The threshold test is EmuTOS's line for line (`aes/geminput.c:366-367`);
// folding it into two range checks would read further from the source than it
// is worth.
#[allow(clippy::manual_range_contains)]
pub(crate) fn mchange(rx: i16, ry: i16) {
    // The ticks that passed before this packet close a window first, at the
    // old position (the timer interrupt would have run before it).
    b_delay_poll();
    let dx = XRAT.load(Ordering::Relaxed) - rx;
    let dy = YRAT.load(Ordering::Relaxed) - ry;
    // `b_delay(gl_bdely)` forks the window's `bchange`s; they run after this
    // `mchange` has finished (the fork ring is FIFO), so they see the new
    // `xrat`/`yrat` — hence the expiry is applied at the end, below.
    let bdely = GL_BDELY.load(Ordering::Relaxed);
    let expire = bdely != 0 && (dx > 2 || dx < -2 || dy > 2 || dy < -2);

    // "xrat, yrat hold true" (aes/geminput.c:371)
    XRAT.store(rx, Ordering::Relaxed);
    YRAT.store(ry, Ordering::Relaxed);

    // `if ((gl_mowner != ctl_pd) && (button == 0) && gl_mntree &&
    // in_mrect(&gl_ctwait)) gl_mowner = ctl_pd;` (aes/geminput.c:388-389)
    if GL_MOWNER_APP.load(Ordering::Relaxed)
        && BUTTON.load(Ordering::Relaxed) == 0
        && !super::menu::active().is_null()
        && super::menu::in_ctwait(rx, ry)
    {
        GL_MOWNER_APP.store(false, Ordering::Relaxed);
    }

    if expire {
        // `b_delay(gl_bdely)` (aes/geminput.c:365-368)
        b_delay(bdely, rx, ry);
    }
}

// ── what the Event Manager consults ────────────────────────────────────────

/// "An application owns the mouse" — `gl_mowner != ctl_pd`, the test the
/// Control Manager makes for itself (`rlr == gl_mowner` with `rlr = ctl_pd`,
/// `aes/gemevlib.c:185`, inverted). Whether a *given* application owns it is
/// [`owns_mouse`].
pub(super) fn own_mouse() -> bool {
    GL_MOWNER_APP.load(Ordering::Relaxed)
}

/// The Control Manager's `w_setactive()` (`aes/gemctrl.c:442`): give the mouse
/// to the owner of the top window — `ct_chgown(ppd, &d)` → `set_mown`
/// (`aes/gemwmlib.c:322-334`, `aes/gemctrl.c:392-397`); the desktop window's
/// owner when no window is open ([`window::top_owner`](super::window::top_owner)).
/// The two guards are [`ct_chgown`]'s and [`set_mown`]'s, so a press latched
/// to the Control Manager stays with it until the release.
///
/// **While `ml_ocnt > 0` the owner is kept.** EmuTOS's `ctlmgr` reaches its
/// loop head only after its own `ev_multi` fired (`aes/gemctrl.c:439-462`),
/// and inside a `BEG_MCTRL` bracket nothing can fire it: keys and presses go to
/// the bracket's holder (the control rectangle is the whole screen, `chkown`
/// answers `gl_cowner`), and `gl_mntree` is held, so there is no bar to track
/// (`aes/gemfmlib.c:89-93`). Our Control Manager runs inline at every wait
/// pass of every client, so the dormancy is stated here: the holder keeps the
/// mouse and the keyboard (`docs/api/aes/event.md` "Who gets a key", point 2).
/// The flag half is set as before.
pub(super) fn w_setactive() {
    let pd = if super::window::mctrl_held() {
        GL_MOWNER_PD.load(Ordering::Relaxed)
    } else {
        super::window::top_owner()
    };
    ct_chgown(Mown::Pd(pd));
}

/// EmuTOS `get_mown(&pmown)` (`aes/geminput.c:209-212`) — the current mouse
/// owner: [`Mown::Ctl`] while `ctl_pd` has it, else the application's
/// identity. `fm_own` saves it on entry (`ml_pmown`, `aes/gemfmlib.c:92`) to
/// hand the mouse back to that owner on exit.
pub(super) fn get_mown() -> Mown {
    if GL_MOWNER_APP.load(Ordering::Relaxed) {
        Mown::Pd(GL_MOWNER_PD.load(Ordering::Relaxed))
    } else {
        Mown::Ctl
    }
}

/// EmuTOS `ct_chgown(mpd, pr)` (`aes/gemctrl.c:392-397`): `set_ctrl(pr); if
/// (!gl_ctmown) set_mown(mpd);` — the mouse changes hands only while no menu
/// track holds it. The control rectangle half is not stored: EmuTOS's `ctrl`
/// has exactly two sources, and [`window::chk_ctrl`](super::window::chk_ctrl)
/// reads both live (`ml_ocnt > 0` → the whole screen; else the top window's
/// work area). `owner` names the owner as [`get_mown`] does.
pub(super) fn ct_chgown(owner: Mown) {
    if !GL_CTMOWN.load(Ordering::Relaxed) {
        set_mown(owner);
    }
}

/// EmuTOS `set_mown(mp)` (`aes/geminput.c:220-235`): `if (!button) {
/// gl_cowner = gl_mowner = mp; post_mouse(…); post_button(gl_mowner, button,
/// 1); }` — "the mouse should only be transferred with the buttons in an up
/// state" (`:216-217`); with a button held nothing changes. The
/// `post_button` completes a parked button wait of the new owner that the
/// (up) state satisfies ([`post_button`]); the `post_mouse` half is covered by
/// the Event Manager re-testing its rectangles against `owns_mouse(idx)` on every
/// pass. `gl_cowner = gl_mowner = mp` (`:225`): for an application both
/// identity halves move ([`GL_MOWNER_PD`], [`GL_COWNER_PD`]); for `ctl_pd`
/// only the flag does.
fn set_mown(owner: Mown) {
    let button = BUTTON.load(Ordering::Relaxed);
    if button == 0 {
        match owner {
            Mown::Ctl => GL_MOWNER_APP.store(false, Ordering::Relaxed),
            Mown::Pd(pd) => {
                GL_MOWNER_APP.store(true, Ordering::Relaxed);
                GL_MOWNER_PD.store(pd, Ordering::Relaxed);
                GL_COWNER_PD.store(pd, Ordering::Relaxed);
            }
        }
        post_button(button, 1);
    }
}

/// EmuTOS `ct_mouse(grabit)` (`aes/gemctrl.c:400-417`), the part that matters
/// to ownership: raise (`grab = true`) or lower [`GL_CTMOWN`]. `mn_do` brackets
/// its whole tracking loop with it (`aes/gemmnlib.c:277,513`); so does
/// [`menu::track`](super::menu::track). The mouse-form bookkeeping EmuTOS does
/// alongside (`set_mouse_to_arrow`, `gl_tmpmoff`) is the VDI cursor's business
/// here and not reproduced.
pub(super) fn ct_mouse(grab: bool) {
    GL_CTMOWN.store(grab, Ordering::Relaxed);
}

/// Enter `ct_msgup`'s wait for the selecting press to be released
/// (`while (button & 0x0001) dsptch();`, `aes/gemctrl.c:100-101`): the Control
/// Manager has posted `MN_SELECTED` and stays out of the way until the button is
/// up. Our Control Manager runs inline in the Event Manager's poll loop, so the
/// wait is a flag the loop consults ([`ct_msgup_busy`]) rather than a spin —
/// the application gets its message at once, exactly as EmuTOS's `dsptch()`
/// inside the spin lets it run.
pub(super) fn ct_msgup_hold() {
    CT_MSGUP.store(true, Ordering::Relaxed);
}

/// Is the Control Manager still inside `ct_msgup`'s wait? `true` while the
/// press that selected a menu item is held; the first call that finds the
/// button up ends the wait (`ct_msgup` returns, `ctlmgr` loops) and answers
/// `false`.
///
/// `ct_msgup` waits **inside** `ctlmgr`'s screen lock (`wm_update(BEG_UPDATE)`
/// … `wm_update(END_UPDATE)`, `aes/gemctrl.c:463,484`), so the end of the wait
/// is also the Control Manager's `END_UPDATE`
/// ([`window::ctl_release`](super::window::ctl_release)): only now does a
/// process that asked for the lock meanwhile — an accessory answering its
/// `AC_OPEN` with `wind_update(BEG_UPDATE)` — get it, with the button up, so
/// its `fm_own(TRUE)` → `set_mown` can give it the mouse (GH1).
pub(super) fn ct_msgup_busy() -> bool {
    if !CT_MSGUP.load(Ordering::Relaxed) {
        return false;
    }
    if BUTTON.load(Ordering::Relaxed) & MB_DOWN != 0 {
        return true;
    }
    CT_MSGUP.store(false, Ordering::Relaxed);
    super::window::ctl_release();
    false
}

/// `ct_msgup`'s HOTCLOSE exit (`aes/gemctrl.c:86-96`): `WM_CLOSED` has been
/// posted for a window whose kind carries `HOTCLOSE`, and instead of parking
/// in the `while (button & 0x0001)` wait the Control Manager **fakes the
/// button up** — `button = 0; return;` — "otherwise a slow click will cause
/// us to stay in ctlmgr(), eating keys". From here on the AES's `button` says
/// *up* while the raw byte still says *down*: `ctlmgr` is not re-triggered
/// by the level it has already acted on, `set_mown` hands the mouse back to
/// the top window's owner at once ([`w_setactive`]), and the application's
/// `ev_multi` quick check / `abutton` (`aes/gemevlib.c:185-200`,
/// `aes/geminput.c:549`) see no press. The physical release is then a
/// change of the true state `gl_btrue` for [`b_click`], which publishes it,
/// and `bchange` finds `button` already 0 — the release is counted in
/// `mtrans` with `pr_button = 0`, nothing else moves; the next press
/// re-synchronises everything.
pub(super) fn ct_msgup_hotclose() {
    BUTTON.store(0, Ordering::Relaxed);
}

/// EmuTOS `button` — the button state the AES has acted on
/// (`aes/geminput.c:49`); the level `ev_multi`'s quick check, `abutton` and
/// the Control Manager's wait test read (`aes/gemevlib.c:185-200`,
/// `aes/geminput.c:549`, `aes/gemctrl.c:100`). Equal to the raw VDI byte
/// (`crate::vdi::mouse_buttons`) except while a button-delay window holds a
/// press back ([`b_click`]) and after [`ct_msgup_hotclose`].
pub(super) fn button() -> u8 {
    BUTTON.load(Ordering::Relaxed)
}

/// EmuTOS `mclick` — the click count of the last published transition
/// (`aes/geminput.c:343`), what `ev_multi`'s quick check reports when the live
/// `button` satisfies the wait (`prets[5] = mclick`, `aes/gemevlib.c:196-198`).
pub(super) fn mclick() -> i16 {
    MCLICK.load(Ordering::Relaxed)
}

/// Is a button-delay window open? EmuTOS `if (gl_bdely)`
/// (`aes/geminput.c:117,366`).
pub(super) fn bdely_pending() -> bool {
    GL_BDELY.load(Ordering::Relaxed) != 0
}

/// EmuTOS `abutton(e, p)` (`aes/geminput.c:545-565`) — arm the button wait of
/// AES identity `idx` (`iasync(MU_BUTTON, buparm)`, `aes/gemasync.c:154-156`).
/// `level` is the AES `button` the arm tests: if it already satisfies
/// `downorup` the wait completes at once with **one** click — `azombie(e, 1)`,
/// "'nuff said", ownership-blind — and `Some(1)` is returned with nothing
/// parked. Otherwise the wait is parked on the identity's list and, if it asks
/// for more than one click, `gl_bpend` is incremented: from now on a press
/// opens the delay window ([`b_click`]). It is completed only by
/// [`post_button`] and collected with [`bwait_take`].
pub(super) fn abutton(idx: usize, level: u8, bflgclks: i16, mask: u8, state: u8) -> Option<i16> {
    if downorup(level, bflgclks, mask, state) {
        return Some(1);
    }
    let w = BWAIT.get(idx)?;
    // `bclicks = LOBYTE(HIWORD(p)); if (bclicks > 1) gl_bpend++;` (:559-561)
    if bflgclks & 0x00ff > 1 {
        GL_BPEND.fetch_add(1, Ordering::Relaxed);
    }
    let parm = (u32::from(bflgclks as u16) << 16) | (u32::from(mask) << 8) | u32::from(state);
    BWAIT_PARM[idx].store(parm, Ordering::Relaxed);
    w.store(BW_ARMED, Ordering::Relaxed);
    None
}

/// Does identity `idx` have a button wait parked or completed but not yet
/// collected? An Event Manager call that gave the CPU to a peer and is
/// re-executed from the top (S2) finds its own wait still there and must not
/// arm a second one — EmuTOS's process would simply still be asleep on it.
pub(super) fn bwait_armed(idx: usize) -> bool {
    BWAIT.get(idx).is_some_and(|w| w.load(Ordering::Relaxed) != BW_IDLE)
}

/// Collect a completed button wait of identity `idx`: `Some(clicks)` — the
/// `apret(MU_BUTTON)` value, `aes/gemevlib.c:276-277` — and the wait is gone;
/// `None` while it is still parked (or there is none).
pub(super) fn bwait_take(idx: usize) -> Option<i16> {
    let w = BWAIT.get(idx)?;
    let v = w.load(Ordering::Relaxed);
    if v < 0 {
        return None;
    }
    w.store(BW_IDLE, Ordering::Relaxed);
    Some(v as i16)
}

/// EmuTOS `acancel` for the button wait (`aes/gemasync.c:217-243`): take a
/// parked wait of identity `idx` off unsatisfied. Through `takeoff`
/// (`:96-114`) — which does **not** touch `gl_bpend`, so a cancelled
/// multi-click wait leaves its increment behind ([`GL_BPEND`]). A completed
/// wait that was not collected is dropped too: the call that armed it is
/// over.
pub(super) fn acancel(idx: usize) {
    if let Some(w) = BWAIT.get(idx) {
        w.store(BW_IDLE, Ordering::Relaxed);
    }
}

/// An AES-internal wait returned on a button event — `fm_do`, `gr_stilldn`,
/// `mn_do`, the pop-up, the Control Manager — and consumes it the way its
/// EmuTOS `ev_multi`/`ev_button` does: [`ev_rets`] after the return.
///
/// Those EmuTOS waits block on the *published* `button`, so while a
/// button-delay window is open they cannot return before it has closed and
/// published the gesture to the mouse owner of that moment. Ours act on the
/// raw VDI level as soon as it changes; so a window still open at this point
/// is closed here — `b_delay(gl_bdely)`, the same forced close `mchange`
/// applies (`aes/geminput.c:365-368`) — and its transitions are published
/// under the current owner (the Control Manager inside a menu track or a
/// frame gadget, the application in its own `form_do`) before `ev_rets`
/// consumes them. Without it the press the AES has acted on would be
/// published only later, when the application could take it for its own
/// click. Key returns use plain [`ev_rets`]: EmuTOS leaves the window running
/// across them.
pub(super) fn ev_consume() {
    b_delay_poll();
    let d = GL_BDELY.load(Ordering::Relaxed);
    if d != 0 {
        let (x, y) = crate::vdi::mouse_position();
        b_delay(d, x, y);
    }
    let _ = ev_rets();
    CTL_PRESS.store(false, Ordering::Relaxed);
}

/// Take the Control Manager's latched press ([`CTL_PRESS`]): `true` once per
/// press its loop wait was completed with — the `MU_BUTTON` return of
/// `ctlmgr`'s `ev_multi` (`aes/gemctrl.c:456-470`).
pub(super) fn ctl_press_take() -> bool {
    CTL_PRESS.swap(false, Ordering::Relaxed)
}

/// EmuTOS `ev_rets` (`aes/gemevlib.c:42-58`), the mouse half: the pointer
/// position and button state an event reports — the state recorded before the
/// last published transition (`pr_xrat`/`pr_yrat`/`pr_button`) when more than
/// one transition has been published since the last event return
/// (`mtrans > 1`), else the live `xrat`/`yrat`/`button` — then `mtrans = 0`.
///
/// Every AES-internal wait whose EmuTOS counterpart is an `ev_multi`/
/// `ev_button` (the Form Manager's `fm_do`, `gr_stilldn`, `mn_do`, the
/// pop-up's `handle_popup`, the Control Manager's own loop) consumes the
/// transitions it returns on in EmuTOS; those call this too and drop the
/// result, so a click the AES itself acted on is not reported again by the
/// application's next event. The live position is the VDI cursor's, as
/// everywhere else in the Event Manager.
pub(super) fn ev_rets() -> (i16, i16, u8) {
    let r = if MTRANS.load(Ordering::Relaxed) > 1 {
        (
            PR_XRAT.load(Ordering::Relaxed),
            PR_YRAT.load(Ordering::Relaxed),
            PR_BUTTON.load(Ordering::Relaxed),
        )
    } else {
        let (x, y) = crate::vdi::mouse_position();
        (x, y, BUTTON.load(Ordering::Relaxed))
    };
    MTRANS.store(0, Ordering::Relaxed);
    r
}

/// `ev_multi`'s quick check on the recorded state, `if ((mtrans > 1) &&
/// downorup(pr_button, buparm)) { … prets[5] = pr_mclick; }`
/// (`aes/gemevlib.c:190-194`): `Some((pr_button, pr_mclick))` while more than
/// one transition is pending, else `None`. Read-only — the count is cleared
/// by the [`ev_rets`] that ends the call.
pub(super) fn pending_transition() -> Option<(u8, i16)> {
    (MTRANS.load(Ordering::Relaxed) > 1)
        .then(|| (PR_BUTTON.load(Ordering::Relaxed), PR_MCLICK.load(Ordering::Relaxed)))
}

// ── the mouse wheel (CONF_WITH_EXTENDED_MOUSE) ─────────────────────────────

/// EmuTOS `wheel_change(wheel_number, wheel_amount)` (`aes/geminput.c:394-
/// 420`) — the AES's end of the wheel chain, reached through the VDI's
/// `user_wheel` vector ([`crate::vdi::mouse::install_aes_wheel`],
/// [`super::aes_wheel`]).
///
/// EmuTOS's body, complete:
///
/// ```c
/// /* Ignore the wheel messages if the menu is active */
/// if (gl_mowner == ctl_pd)
///     return;
/// if (wheel_amount > 0)       type = WA_DNLINE;
/// else if (wheel_amount < 0)  type = WA_UPLINE;
/// else                        return;
/// /* We have a problem here.
///  * This function is called by forker(), where rlr is set to -1.
///  * As a result, we can't call ap_sendmsg() which internally calls iasync().
///  * Uncomment the following code when some solution has been found.
///  */
/// /*
///     assert(rlr != (AESPD *)-1);
///     wh = wm_find(xrat, yrat);
///     ap_sendmsg(appl_msg, WM_ARROWED, D.w_win[wh].w_owner, wh, type, 0,0,0);
/// */
/// ```
///
/// ## The named deviation: NewTOS sends the message
///
/// **What EmuTOS does here.** It computes the action and then throws the
/// event away. Its own comment names the reason: `wheel_change` runs from
/// `forker()`, which parks the running-process pointer at a sentinel —
/// `oldrl = rlr; rlr = (AESPD *) -1;` (`aes/gemdisp.c:105-106`) — for the
/// whole fork-ring drain. `ap_sendmsg` dereferences exactly that pointer
/// (`ap_msg[1] = rlr->p_pid`, `aes/gemwmlib.c:855`) and then enters
/// `ap_rdwr`/`iasync`, the asynchronous send that reschedules. Neither is
/// legal against `rlr == -1`, so the three lines stay commented out and a
/// mouse wheel on EmuTOS does nothing.
///
/// **Why we go further.** The user's decision of 2026-09-25 is that the wheel
/// must scroll (`TASK/GUI04.md`). This is not the "rationalised EmuTOS
/// deviation" the project forbids: EmuTOS states in its own source that a
/// solution is *missing*, not that the behaviour is intended.
///
/// **How, and why EmuTOS's obstacle is not ours.** NewTOS has no fork ring
/// and no `forker()`. A wheel detent travels `Host::pump_input` →
/// `usb::poll` → `ikbd::feed` → `mousexvec` → `call_user_wheel` → here, and
/// `pump_input` is only ever called **synchronously** from EL1 code running
/// on behalf of a well-defined caller: an AES/GEMDOS trap of the running EL0
/// process, or a wait loop of kernel code (the blocking console reads, the
/// boot path). (INPUT01's timer-interrupt pump
/// does not decode wheel detents — they are held for the trap-side pump,
/// `kernel::input_gate`.) There is no point at which a
/// "no running process" sentinel is installed, so the precondition of
/// EmuTOS's comment simply does not arise. Its two concrete obstacles are
/// gone with it:
///
/// * `rlr->p_pid` — our message poster takes `msg[1]` as an explicit
///   argument. `wheel_change` is control-manager-side code (EmuTOS's
///   `aes_wheel` is forked exactly like `far_mcha`/`mchange`), so it stamps
///   [`super::appl::SCRENMGR_PID`], the same stamp
///   [`window::intercept`](super::window::intercept) uses for every `WM_*` it
///   posts (spec § "AES process identity and pid", contract (b)).
/// * `iasync` — [`pd::post_msg`](super::pd::post_msg) enqueues into the
///   owner's FIFO and flags a sleeping waiter through `sched::wake`; it does
///   not dispatch and cannot re-enter the caller.
///
/// So the delivery is the ordinary one: the same `ap_sendmsg` → `aqueue` →
/// `doq` routing that `send_redraw` and the frame interception already use,
/// including `doq`'s `WM_ARROWED` coalescing (`aes/gemqueue.c:63-67`) — a
/// burst of detents the application has not read yet collapses to the newest,
/// exactly as a burst of scroll-arrow clicks does.
///
/// The menu guard is EmuTOS's, unchanged: while the Control Manager owns the
/// mouse (`gl_mowner == ctl_pd`, i.e. a menu is being tracked) the wheel is
/// ignored.
pub(crate) fn wheel_change(_wheel_number: i16, wheel_amount: i16) {
    // `if (gl_mowner == ctl_pd) return;` (`aes/geminput.c:400-402`)
    if !GL_MOWNER_APP.load(Ordering::Relaxed) {
        return;
    }
    // `if (wheel_amount > 0) WA_DNLINE; else if (< 0) WA_UPLINE; else return;`
    // (`:404-409`)
    let ty = match wheel_amount {
        a if a > 0 => WA_DNLINE,
        a if a < 0 => WA_UPLINE,
        _ => return,
    };
    // `wh = wm_find(xrat, yrat)` (`:418`) — the window under the **last
    // reported** pointer position, the same `xrat`/`yrat` `mchange` maintains
    // (`:370-371`), not a fresh sample.
    let wh = super::window::wm_find(XRAT.load(Ordering::Relaxed), YRAT.load(Ordering::Relaxed));
    let Some(owner) = super::window::owner_of_window(wh) else {
        return;
    };
    // `ap_sendmsg(appl_msg, WM_ARROWED, D.w_win[wh].w_owner, wh, type, 0,0,0)`
    // (`:419`): msg[0] type, msg[1] sender pid, msg[2] 0, msg[3] the window
    // handle, msg[4] the action, msg[5..7] 0 (`aes/gemwmlib.c:851-862`).
    let mut msg = [0i16; 8];
    msg[0] = WM_ARROWED;
    msg[1] = super::appl::SCRENMGR_PID;
    msg[3] = wh;
    msg[4] = ty;
    // SAFETY: single trap path (see the module note); `post_msg` hands the
    // message to the live descriptor of `owner`.
    unsafe {
        super::pd::post_msg(owner, msg);
    }
}
