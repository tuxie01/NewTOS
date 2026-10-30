//! AES Menu Manager (`menu_*`, opcodes 30–34) — Batch 1.
//!
//! Spec: [`docs/api/aes/menu.md`]. A menu **is** an OBJECT tree: the root
//! (`THESCREEN`, `G_IBOX`) holds the bar (`THEBAR`, object 1) as its
//! `ob_head`, and the titles (`G_TITLE`) hang under `THEACTIVE`. A title's
//! drop-down box is **not** its child (D-085): the drop-down `G_BOX`es are the
//! `ob_next` sibling chain rooted at `THESCREEN`'s hidden *second* child
//! (`ob_tail`, the `THEMENUS` container), the *N*-th box matching the *N*-th
//! title — this is what `menu_sub` walks. Each box's own children are the
//! items (`G_STRING`). So `menu_bar` just draws the bar subtree, and the
//! item/title state calls (`icheck`/`ienable`/`tnormal`) flip one `ob_state`
//! bit through the shared `do_chg` helper — each with its own redraw /
//! disabled-guard contract (`gemsuper.c:171-180`): `icheck` never repaints,
//! `ienable` repaints only when item bit 15 is set, `tnormal` always repaints
//! and refuses a `DISABLED` title.
//!
//! **Scope:** the bar + the four original-TOS state/text calls, the
//! interactive drop-down tracking (mouse over titles → open the box →
//! highlight item → post `MN_SELECTED`), and the AES 3.30 extensions
//! `menu_popup`/`menu_attach`/`menu_istart`/`menu_settings`. Submenus are
//! backed by a fixed-size **SMIB** registry ([`SubmenuTable`]) modelled on
//! EmuTOS `SMIB`/`gl_submenu`; `menu_attach` records a submenu for a
//! `(tree, item)` pair (and reproduces the EmuTOS `ob_type`/`SUBMENU`/marker
//! encoding of D-087 as contract), and the drop-down tracker opens an
//! attached submenu as a nested box beside the highlighted parent item
//! (EmuTOS `display_submenu`/`mn_do` `SUBMENU_STATE`).

use super::graf::gl_hbox;
use super::gsxif;
use super::object::{self, ObjTree, Object};
use super::Aespb;
use crate::bdos::process::Abi;
use crate::host::host;
use crate::vdi::backend::{ClipRect, Pel, RasterBackend};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI16, AtomicU64, AtomicUsize, Ordering};
use tos_types::aes::{
    AC_CLOSE, ADJ3DSTD, CHECKED, DISABLED, G_STRING, G_TITLE, NIL, SELECTED, SUBMENU,
};

/// `THESCREEN` — the menu tree root; its `ob_tail` is the box that holds
/// all drop-down menus.
const THESCREEN: i16 = 0;
/// `THEBAR` — the menu-bar object (child of the root).
const THEBAR: i16 = 1;
/// `THEACTIVE` — the box holding the title objects (children of the bar).
const THEACTIVE: i16 = 2;
const MAX_DEPTH: i16 = 16;

/// The active menu tree (`gl_mntree`), set by `menu_bar(show=1)` and
/// cleared by `menu_bar(show=0)`. Stored as a `usize` for `Sync` without
/// `unsafe`; only the single boot CPU touches it.
static MNTREE: AtomicUsize = AtomicUsize::new(0);

/// The client that installed [`MNTREE`] — EmuTOS `gl_mnppd`
/// (`aes/gemmnlib.h:17`), set beside `gl_mntree` in `mn_bar`: `p =
/// fpdnm(NULL, rlr->p_pid); … gl_mnppd = p; gl_mntree = tree;`
/// (`aes/gemmnlib.c:707,712-713`). The AES identity of the caller
/// (`appl::current_identity`: a GEMDOS slot, or `pd::KERNEL_CONTEXT` for kernel
/// code), `usize::MAX` when no bar is installed.
///
/// EmuTOS uses it as the message target for a menu selection
/// (`owner = gl_mnppd; … ct_msgup(mesag, owner, …)`, `aes/gemctrl.c:353,365`):
/// [`mn_selected_target`] (A117). It also answers **whose** tree the pointer
/// in `gl_mntree` is ([`release_bar_of`], [`mntree_accessor`]).
static MNPPD: AtomicUsize = AtomicUsize::new(usize::MAX);

/// The currently installed menu tree, or null if none.
pub fn active() -> *mut Object {
    MNTREE.load(Ordering::Relaxed) as *mut Object
}

/// The AES identity an application-menu pick is posted to — EmuTOS `owner =
/// gl_mnppd` (`aes/gemctrl.c:353`), the descriptor that called `menu_bar(tree,
/// 1)` (`gemmnlib.c:707,712`), whatever process the Control Manager runs in.
///
/// EmuTOS's Control Manager is a process of its own (`ctl_pd`); NewTOS runs it
/// inline in the `evnt_multi(MU_MESAG)` of whichever client is waiting — a
/// desk accessory included — so the waiter must never be the target (A117,
/// `docs/api/aes/menu.md` "`MN_SELECTED` message", NewTOS binding).
///
/// `gl_mnppd` is a *descriptor*, not a process: the application chain is one
/// (`D.g_int[0]`, `aes/geminit.c:851-877`), each accessory another. The
/// recorded identity is therefore mapped to the descriptor's pid and back to
/// the member that runs on it, exactly as `appl_write` addresses that pid
/// ([`appl::identity_of`](super::appl::identity_of)): a chain member waiting is
/// the chain itself, an accessory waiting reaches the chain's innermost started
/// member (the program a `Pexec` parent is suspended for). `None` when no bar
/// is installed or the descriptor has no live member.
pub(super) fn mn_selected_target() -> Option<usize> {
    let owner = MNPPD.load(Ordering::Relaxed);
    if owner >= super::pd::NUM_PDS {
        return None;
    }
    super::appl::identity_of(super::appl::reported_pid(owner))
}

/// EmuTOS `gl_ctwait.m_gr` (`aes/gemctrl.c:48`, `MOBLK gl_ctwait`) — the
/// Control Manager's mouse-wait rectangle, packed `(x, y, w, h)` as four `i16`
/// in one word so a reader always sees one consistent rectangle. It has
/// exactly two values: `THEACTIVE`'s absolute extent while a bar is installed
/// (`ob_actxywh(tree, THEACTIVE, &gl_ctwait.m_gr)`, `aes/gemmnlib.c:717`),
/// and `gl_rmenu` — the whole menu-bar row, `(0, 0, gl_width, gl_hbox)`
/// (`aes/gemgraf.c:427`) — before any bar (`ctlmgr`'s init,
/// `aes/gemctrl.c:438`) and after `mn_bar(…, FALSE)` (`aes/gemmnlib.c:727`).
/// [`CTWAIT_BAR`] says which; the `gl_rmenu` state is not stored as numbers
/// because its width is the device's (see [`in_ctwait`]).
///
/// `m_out` is never set (`gemctrl.c:437`), so `in_mrect(&gl_ctwait)` is a
/// plain inside test (`aes/geminput.c:76-79`).
static CTWAIT_RECT: AtomicU64 = AtomicU64::new(0);
static CTWAIT_BAR: AtomicBool = AtomicBool::new(false);

/// Pack a `(x, y, w, h)` rectangle for [`CTWAIT_RECT`].
fn pack_rect((x, y, w, h): (i16, i16, i16, i16)) -> u64 {
    (x as u16 as u64) | ((y as u16 as u64) << 16) | ((w as u16 as u64) << 32) | ((h as u16 as u64) << 48)
}

/// Unpack a [`CTWAIT_RECT`] word into `(x, y, w, h)`.
fn unpack_rect(v: u64) -> (i16, i16, i16, i16) {
    (v as u16 as i16, (v >> 16) as u16 as i16, (v >> 32) as u16 as i16, (v >> 48) as u16 as i16)
}

/// `in_mrect(&gl_ctwait)` (`aes/geminput.c:76-79`): is `(mx, my)` inside the
/// Control Manager's wait rectangle? This is the rectangle — and the only one
/// — that hands the mouse to `ctl_pd` on a move with the buttons up
/// (`mchange`, `aes/geminput.c:388-389`, [`input::mchange`](super::input::mchange))
/// and that the Control Manager's `MU_M1` wait covers (`ctlmgr`,
/// `aes/gemctrl.c:456-460`, [`event::menu_intercept`](super::event)). A pointer
/// in the bar row **right of the last title** is outside it, and stays with
/// the application (A87).
///
/// With no bar the rectangle is `gl_rmenu = (0, 0, gl_width, gl_hbox)`
/// (`aes/gemgraf.c:427`); the pointer is clipped to the screen, so
/// `inside(mx, my, &gl_rmenu)` reduces to `my < gl_hbox`. Both consumers gate
/// on `gl_mntree` first (`geminput.c:388`, `gemctrl.c:451-452`), so this arm
/// is never what decides a hand-over — it is kept for what `gl_ctwait` *is*.
pub(super) fn in_ctwait(mx: i16, my: i16) -> bool {
    if !CTWAIT_BAR.load(Ordering::Relaxed) {
        return my < gl_hbox();
    }
    let (x, y, w, h) = unpack_rect(CTWAIT_RECT.load(Ordering::Relaxed));
    mx >= x && mx < x + w && my >= y && my < y + h
}

/// The Control Manager's wait rectangle `(x, y, w, h)` while a bar is
/// installed, `None` in the `gl_rmenu` state. For tests and diagnostics.
#[cfg(test)]
fn ctwait_bar_rect() -> Option<(i16, i16, i16, i16)> {
    CTWAIT_BAR.load(Ordering::Relaxed).then(|| unpack_rect(CTWAIT_RECT.load(Ordering::Relaxed)))
}

/// `mn_bar(tree, TRUE)`'s state half (`aes/gemmnlib.c:712-717`): `gl_mntree =
/// tree` and `ob_actxywh(tree, THEACTIVE, &gl_ctwait.m_gr)` — the wait
/// rectangle becomes `THEACTIVE`'s absolute extent in the tree as registered
/// (after `THEBAR` was stretched to the screen; `THEACTIVE` itself is not
/// resized, its width is the titles'). `THEACTIVE` is a plain `G_IBOX`, so
/// `ob_actxywh`'s `FL3DOBJ` growth (`gemoblib.c:322-335`) does not apply and
/// [`object::abs_rect`] is the whole of it. No drawing here: [`menu_bar`] draws.
fn install_bar_state(tree: *mut Object) {
    // `gl_mnppd = p` (`gemmnlib.c:712`), `p` = the calling client.
    MNPPD.store(super::appl::current_identity(), Ordering::Relaxed);
    MNTREE.store(tree as usize, Ordering::Relaxed);
}

/// The second half of [`install_bar_state`] — `ob_actxywh(tree, THEACTIVE,
/// &gl_ctwait.m_gr)` (`gemmnlib.c:717`), taken after `menu_fixup` and the
/// `THEBAR` stretch, exactly where EmuTOS takes it.
fn install_bar_ctwait(tree: *mut Object) {
    CTWAIT_RECT.store(pack_rect(object::abs_rect(tree as *const Object, THEACTIVE)), Ordering::Relaxed);
    CTWAIT_BAR.store(true, Ordering::Relaxed);
}

/// `mn_bar(…, FALSE)`'s state half (`aes/gemmnlib.c:726-727`): `gl_mntree =
/// NULL` and `rc_copy(&gl_rmenu, &gl_ctwait.m_gr)`. Independent of the screen,
/// as EmuTOS's is: dropping the bar must forget the tree with no raster up.
fn remove_bar() {
    MNTREE.store(0, Ordering::Relaxed);
    CTWAIT_BAR.store(false, Ordering::Relaxed);
    // EmuTOS leaves `gl_mnppd` standing on the hide path (`gemmnlib.c:726-727`
    // writes `gl_mntree` and `gl_ctwait` only) — it is only ever read while a
    // tree is installed (`gemctrl.c:353`). Dropped here as well so
    // [`release_bar_of`] cannot match a slot against a bar that is gone.
    MNPPD.store(usize::MAX, Ordering::Relaxed);
}

/// Drop the installed menu bar if the client that installed it is `slot` —
/// the Menu-Manager half of the per-slot sweep [`window::wm_new_for_slot`]
/// runs when an EL0 `Pexec` child has ended (A107).
///
/// **EmuTOS does not do this, and the reason it need not is structural.**
/// `gl_mntree` is written in exactly two places: `mn_bar`
/// (`aes/gemmnlib.c:713,726`) and `fm_own`'s dialog park/restore
/// (`aes/gemfmlib.c:89-90,103`). Neither `wm_new` (`gemwmlib.c:1875-1893`),
/// nor `wm_init` (`:1247-1310`), nor `sh_ldapp`'s `AP_OPEN` cleanup →
/// `mn_cleanup` (`gemshlib.c:650-657`, `gemmnlib.c:740-760`: AC_CLOSE to the
/// accessories and the submenu array, nothing else), nor `ap_exit`
/// (`gemaplib.c:251-261`, which calls the same `mn_cleanup`) clears it. A
/// program that ends with its bar installed leaves the pointer standing, and
/// the next program's `menu_bar(tree, 1)` overwrites it — the desktop does
/// exactly that when the shell loop restarts it (`desk/deskmain.c:1904`).
///
/// On EmuTOS that dangling pointer is inert: there is no MMU, the dead TPA is
/// still readable memory, and the Control Manager would at worst track
/// garbage. Here it is a **fault**: the tree is an EL0 VA of an address space
/// that is gone, and the Control Manager reads it at EL1 under whichever
/// `TTBR0` the next caller has ([`track`] → `ObjTree::new`) — the A97 class of
/// defect. So the NewTOS rule is: a bar dies with the client that installed
/// it. Named deviation, `docs/api/aes/menu.md` § "A dead client's menu bar".
///
/// Nothing is drawn: this is the state half of `mn_bar(…, FALSE)`
/// (`gemmnlib.c:726-727`), not the erase — the pixels in the bar row belong
/// to whatever paints next (the parent's own `menu_bar(tree, 1)`, TeraDesk
/// `regen_desktop`, `sdk/apps/teradesk/icon.c:2509-2518`).
pub(super) fn release_bar_of(slot: usize) {
    // `MNPPD` is `usize::MAX` once the bar has been taken down, so a match
    // here means this slot's tree is either installed or parked.
    if MNPPD.load(Ordering::Relaxed) != slot {
        return;
    }
    // A tree parked by `fm_own` (`gemfmlib.c:89-90`) would be handed back by
    // the matching `END_MCTRL` (`:103`); the dead client's must not be.
    MNHOLD.store(0, Ordering::Relaxed);
    remove_bar();
}

/// The Menu Manager's share of an AES restart (A106, [`super::reinit`]) —
/// the three EmuTOS calls that bring the menu state up for a session, in
/// `gem_main`/`run_accs_and_desktop` order:
///
/// 1. **`mn_init()`** (`geminit.c:808` → `gemmnlib.c:821-832`): the desk
///    accessory registry (`desk_ppd[]`, `D.g_acctitle[]`, `D.g_accreg`) goes
///    empty — here [`ACCS`], whose `title` pointers are the strings a
///    `menu_register` caller supplied and whose address space is gone.
/// 2. **`mn_cleanup()`**'s submenu half (`gemmnlib.c:751-756`): `gl_submenu`
///    is freed and nulled — here [`SUBMENU_TAB`], back to all-[`Smib::EMPTY`],
///    dropping the `parent_tree`/`sub_tree` pointers with it.
/// 3. **`mnext_init()`** (`geminit.c:672`): the `menu_settings` block
///    ([`mnext_init`]).
///
/// Plus the bar and track state, which EmuTOS reaches on the same path by
/// other routes: `gl_mntree`/`gl_mnppd`/`gl_ctwait` are re-established by the
/// Control Manager's own start-up (`ctlmgr`'s `gl_ctwait = gl_rmenu`,
/// `gemctrl.c:438`, after `iprocess("SCRENMGR", ctlmgr)`, `geminit.c:888`) —
/// [`remove_bar`] is exactly that state — and `ml_mnhold` (`gemfmlib.c:56`)
/// belongs to a modal dialog of the session that ended.
///
/// The two [`gsxif::Saved`] slots are taken and dropped, which releases the
/// transient pool block a `popup_blit` save holds (`gemmnext.c:212`) if a
/// drop-down or submenu was still open when the session ended. Nothing is
/// drawn: the screen belongs to the next session.
pub(super) fn reinit() {
    // `mn_init()` (gemmnlib.c:821-832).
    // SAFETY: single-CPU restart path, no AES call in flight.
    unsafe {
        (*ACCS.title.get()).fill(0);
        (*ACCS.owner.get()).fill(usize::MAX);
        (*ACCS.display.get()).fill(-1);
        (*TITLES.0.get()).0 = [[0; ACC_TITLE_LEN]; NUM_ACCS];
        DAFIRST.store(NIL, Ordering::Relaxed);
        // `mn_cleanup`'s `dos_free(gl_submenu); gl_submenu = NULL;`
        // (gemmnlib.c:751-756).
        *SUBMENU_TAB.0.get() = [Smib::EMPTY; NUM_SMIBS];
    }
    mnext_init(); // geminit.c:672
    MNHOLD.store(0, Ordering::Relaxed);
    remove_bar();
    // Release a drop-down / submenu save the dead session never restored.
    drop(MENU_SAVE.take());
    drop(SUBMENU_SAVE.take());
}

/// Parked menu tree during a modal dialog (`ml_mnhold`, EmuTOS gemfmlib.c:56).
/// A single slot suffices because `fm_own` only saves on the outermost
/// `ml_ocnt == 0` → 1 transition and restores on 1 → 0 (nested dialogs do not
/// re-save), so at most one tree is ever parked (see [`window::fm_own`]).
static MNHOLD: AtomicUsize = AtomicUsize::new(0);

/// Park the active menu tree while a modal dialog owns the screen — EmuTOS
/// `fm_own`'s `ml_mnhold = gl_mntree; gl_mntree = NULL;` (gemfmlib.c:89-90).
/// With `gl_mntree` NULL, [`active`] returns null so the Control Manager's menu
/// interception (`menu_intercept`, the `if (gl_mntree)` guard of `hctl_rect`,
/// gemctrl.c:328) finds no menu to track: a menu-bar click is inert for the
/// duration of the dialog. Called on the outermost 0 → 1 mouse-control-ownership
/// transition from [`window::fm_own`].
pub(super) fn hold_menu() {
    MNHOLD.store(MNTREE.load(Ordering::Relaxed), Ordering::Relaxed);
    MNTREE.store(0, Ordering::Relaxed);
}

/// Restore the menu tree parked by [`hold_menu`] — EmuTOS `fm_own`'s
/// `gl_mntree = ml_mnhold;` (gemfmlib.c:103). Called on the outermost 1 → 0
/// mouse-control-ownership transition from [`window::fm_own`].
pub(super) fn release_menu() {
    MNTREE.store(MNHOLD.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// The y-coordinate where the desktop work area begins: **always** the bottom
/// edge of the menu-bar row (`gl_hbox`), whether or not an application has
/// installed a menu via `menu_bar`.
///
/// This mirrors EmuTOS `gl_rfull`, set **once** at `gsx_start` to
/// `(0, gl_hbox, gl_width, gl_height - gl_hbox)`
/// (`references/emutos/aes/gemgraf.c:424`), and copied into the desktop
/// pseudo-window's work rect + owner rectangle list at `w_start`
/// (`gemwmlib.c:1289,1294`) — before any `menu_bar` call. The AES desktop
/// unconditionally reserves the top `gl_hbox` pixels for the menu row
/// (`gl_rmenu = (0,0,gl_width,gl_hbox)`, gemgraf.c:427); the desktop background
/// is never painted into that row, and windows are never moved above it.
///
/// The row is `gl_hbox` = `gl_hchar + 3` (gemgraf.c:412), derived from the
/// system-font cell height in [`graf::gl_hbox`](super::graf::gl_hbox) — 19
/// for the 8×16 font — and read from there by every bar drawing / tracking
/// site in this module, not held as a number of its own.
///
/// The window manager uses this to seed the desktop pseudo-window's work
/// rectangle and its rectangle list (the desk area = screen minus the menu
/// row); see `window::desktop_work`.
pub(super) fn desk_top() -> i16 {
    gl_hbox()
}

const WHITE: Pel = 0;
const BLACK: Pel = 1;

/// EmuTOS `MENU_THICKNESS` (`gemmnlib.h:14`): the per-side margin `menu_sr` adds
/// around a drop-down box before saving/restoring the screen under it
/// (`gemmnlib.c:215-217`). It exists precisely to cover a box's 1-px outward
/// border and overscan.
const MENU_THICKNESS: i16 = 1;

/// `menu_bar` (30) — install (`int_in[0]`=1) or remove (0) the menu bar
/// `addr_in[0]` at the top of the screen.
///
/// EmuTOS `mn_bar` (`gemmnlib.c:703-732`) branches on `showit` **first** and
/// never touches `tree` on the hide path — it only drops the tree pointer
/// (`gl_mntree = NULL`, `:724-728`). `menu_bar(NULL, 0)` is therefore a legal
/// call, and the one an application makes when it gives its bar back: after a
/// `Pterm` the AES must not keep a pointer into the dead process's tree.
///
/// The **show** path is the only one that dereferences `tree`; a NULL tree
/// there is a caller bug that faults EmuTOS outright (it has no guard). NewTOS
/// returns 0 instead of faulting the whole system — the single deviation in
/// this function, on a path no conforming caller takes.
pub fn menu_bar(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let show = pb.intin(0);

    if show == 0 {
        // `gl_mntree = NULL; rc_copy(&gl_rmenu, &gl_ctwait.m_gr)`
        // (`gemmnlib.c:726-727`) — unconditional, and independent of the
        // screen: dropping the bar must forget the tree even with no raster
        // backend up.
        remove_bar();
        if let Some(backend) = host().and_then(|h| h.raster()) {
            let sw = backend.width() as i16;
            let clip = ClipRect::full(backend.width(), backend.height());
            // Erase the bar area.
            backend.fill_rect(0, 0, sw, gl_hbox(), WHITE, clip);
        }
        // `xif` seeds `ret = TRUE` and the `MENU_BAR` arm never changes it
        // (`gemsuper.c:85,168-170`), so a hide always reports 1.
        pb.set_ret(1);
        return;
    }

    if tree.is_null() {
        pb.set_ret(0);
        return;
    }
    let Some(backend) = host().and_then(|h| h.raster()) else {
        pb.set_ret(0);
        return;
    };
    let sw = backend.width() as i16;
    let clip = ClipRect::full(backend.width(), backend.height());

    // The bar is an opaque white strip. THEBAR (the G_BOX background) is
    // hollow (`gr_crack(0)` → IP_HOLLOW), so it relies on the strip being
    // clean underneath; clear it to white first, mirroring the `show == 0`
    // erase, so drawing the bar always yields a clean strip even when a
    // full-screen program (e.g. a `.NCP` console) left content there.
    backend.fill_rect(0, 0, sw, gl_hbox(), WHITE, clip);
    // `gl_mnppd = p; gl_mntree = tree;` (`gemmnlib.c:712-713`) — before
    // `menu_fixup`, which works on `gl_mntree`.
    install_bar_state(tree);
    // `menu_fixup();` (`gemmnlib.c:714`) — splice the registered desk
    // accessories into this tree's Desk drop-down, whoever's tree it is. This
    // is what makes an accessory's title appear under *Desk* in **every**
    // application's menu bar, not only the desktop's.
    menu_fixup();
    // The bar spans the full screen width. THEBAR is object 1; read/write
    // its geometry at the caller's ABI width (24-byte ILP32 / 32-byte LP64).
    let t = ObjTree::new(tree);
    t.set_width(THEBAR, sw - t.x(THEBAR));
    // `ob_actxywh(tree, THEACTIVE, &gl_ctwait.m_gr)` (`gemmnlib.c:717`): the
    // Control Manager waits on `THEACTIVE`'s extent from now on, not on the
    // whole bar row (A87).
    install_bar_ctwait(tree);
    object::draw_obj(tree as *const Object, THEBAR, MAX_DEPTH, backend);
    // Separator line under the bar (`gsx_cline(0, gl_hbox-1, …)`, gemmnlib.c:722).
    backend.fill_rect(0, gl_hbox() - 1, sw, 1, BLACK, clip);
    pb.set_ret(1);
}

/// Interactive drop-down tracking — the only genuinely new Menu-Manager
/// logic. Entered (from `evnt_multi`) as soon as the cursor is over the bar
/// with the buttons up. Follows the mouse: the title under the cursor opens
/// its drop-down (`menu_sub`), items highlight on hover, a left-button
/// **edge** over an item is the selection. Leaving the title *and* the open
/// box does **not** close it — EmuTOS `mn_do`'s `OUTSIDE_STATE`
/// (`aes/gemmnlib.c:293-305,415-443`) keeps the box down and the title
/// hilited until the pointer re-enters (tracking resumes) or a button
/// transition falls outside (the cancel; A73). Returns `Some((title,
/// item))` or `None` (cancelled).
///
/// **Which edge selects.** EmuTOS `mn_do` derives its button wait in
/// `INITEM_STATE` from the live button each time round: `buparm = (button &
/// 0x0001) ? 0x00010100L : 0x00010101L;` (`aes/gemmnlib.c:314`) — button up
/// on the item → wait for the **press**; already down (the user pressed on
/// the title, or in the bar, and dragged onto the item) → wait for the
/// **release**. Either transition breaks the loop (`:369-376`) and the item
/// is returned. In the title itself a press only flips the wait
/// (`buparm ^= 1`, `:375`), so edges over a title select nothing.
///
/// **Mouse ownership.** The whole loop runs under `ct_mouse(TRUE)` …
/// `ct_mouse(FALSE)` (`gemmnlib.c:277,513`): while it tracks, the press that
/// selects does not hand the mouse to the application
/// ([`input::b_click`](super::input::b_click)), so the Event Manager delivers
/// the selection as the one `MN_SELECTED` and not additionally as
/// `MU_BUTTON` (`docs/api/aes/menu.md` "Mouse ownership over the track").
///
/// Behaviour matches GEM's hover menus (open-on-enter, click-to-pick). The
/// screen under each box is saved on open and restored on close (the AES
/// screen buffer, `gsxif::bb_save` — GEM's `menu_sr`), so it works over
/// windows too; the mouse cursor is hidden across every box/highlight draw
/// so it neither smears nor pollutes the backing.
///
/// **Nested submenus** (EmuTOS `mn_do` `SUBMENU_STATE` + `display_submenu`):
/// when the highlighted item has an attached submenu ([`SubmenuTable`]), it
/// opens as a nested box beside the parent item (right, or left if it would
/// run off-screen), saved under [`SUBMENU_SAVE`]. Hovering into the submenu keeps
/// the parent item highlighted; a press over a submenu item selects it; a
/// press on the parent item itself, submenu showing, selects the *parent*
/// (`mn_do` cleanup, `gemmnlib.c:482-500`). Leaving the submenu into no
/// man's land keeps the parent selected and the submenu up (`:437-438`,
/// `OUTSIDE_STATE`), and a press there selects the retained parent too. A
/// submenu that lives in the **same** OBJECT tree as the bar returns its item
/// through the existing `(title, item)` result (the item index is valid in the
/// caller's tree, so it round-trips as `MN_SELECTED`'s `msg[4]`); a submenu in
/// a *separate* tree needs the extended `msg[5..7]` tree-pointer return path
/// and is left as a scoped follow-on (the tracking/rendering still work, only
/// the cross-tree *selection delivery* is deferred).
pub fn track(tree: *mut Object, backend: &dyn RasterBackend) -> Option<(i16, i16)> {
    // `ct_mouse(TRUE)` on entry, `ct_mouse(FALSE)` on every way out
    // (`aes/gemmnlib.c:277,513`).
    super::input::ct_mouse(true);
    let r = track_owned(tree, backend);
    // `mn_do` leaves on the `ev_multi` that returned the selecting or
    // cancelling event (`aes/gemmnlib.c:597-690`), still under
    // `ct_mouse(TRUE)`; every one of its returns ran `ev_rets`
    // (`aes/gemevlib.c:268`), so the transitions of the track are consumed and
    // none of them is reported by the application's next event. A press the
    // track acted on while a button-delay window held it back is published
    // here, before the bracket closes, to the Control Manager
    // (`input::ev_consume`).
    super::input::ev_consume();
    super::input::ct_mouse(false);
    r
}

/// The body of [`track`], run with the Control Manager holding the mouse.
///
/// The installed bar — and every submenu attached to it — is read and drawn
/// through [`mntree_accessor`]: at the ABI and through the address space of
/// the client that installed it (`MNPPD`), not of whoever happens to be in
/// the AES wait while the pointer crosses the bar (ARCH03 3a, F3; the A97
/// class of defect).
fn track_owned(tree: *mut Object, backend: &dyn RasterBackend) -> Option<(i16, i16)> {
    let h = host()?;
    let mt = mntree_accessor(tree);
    // A submenu tree of the installed bar, through the same owner accessor.
    let sub_t = |sub: u64| mt.with_base(sub as *const Object);
    let mut cur_title = NIL;
    let mut cur_box = NIL;
    let mut cur_item = NIL;
    // Nested submenu state (EmuTOS smtree/smroot/cur_submenu): `sub_tree == 0`
    // means no submenu is open.
    let mut sub_tree: u64 = 0;
    let mut sub_box = NIL;
    let mut sub_item = NIL;
    // Hover model (GEM): the menu opens as the cursor enters a title; an item
    // is chosen on a left-button *edge* over it (press if the button was up
    // on entering the item, release if it was already held — `gemmnlib.c:314`);
    // an edge with the pointer outside title, box and submenu cancels
    // (`OUTSIDE_STATE`, below).
    let mut was_down = crate::vdi::mouse_buttons() & 0x01 != 0;
    // Where the previous pass left `mn_do`'s state machine: in the submenu box
    // (`SUBMENU_STATE`) or in no man's land (`OUTSIDE_STATE`). Both are what
    // the `:437-438` retention keys on (below).
    let mut was_in_sub = false;
    let mut was_outside = false;

    loop {
        let (mx, my) = crate::vdi::mouse_position();
        let down = crate::vdi::mouse_buttons() & 0x01 != 0;

        // Title under the cursor (children of THEACTIVE).
        let t = object::child_at_t(mt, THEACTIVE, mx, my);
        if t != NIL && t != cur_title {
            if cur_title != NIL {
                if sub_tree != 0 {
                    close_submenu(sub_t(sub_tree), sub_box, sub_item, backend);
                    sub_tree = 0;
                    sub_box = NIL;
                    sub_item = NIL;
                }
                // Clear the previously-hovered item's SELECTED bit before pulling
                // the box up, matching EmuTOS `mn_do` (gemmnlib.c:459
                // `menu_select(tree, last_item, cur_item, FALSE)` →
                // `do_chg(tree, last_item, SELECTED, FALSE, TRUE, TRUE)`). Without
                // this the bit persists in the app's gl_mntree, so the menu reopens
                // pre-highlighted and two items invert at once. `do_chg` runs with
                // chkdisabled=TRUE, and a DISABLED item was never given SELECTED
                // (`rehilite` guards on DISABLED, gemmnlib.c:471 does the same), so
                // guard the clear identically to avoid the spurious redraw
                // `set_state_bit` (dodraw=true) would emit on a disabled item.
                if cur_item != NIL && (mt.state(cur_item) & DISABLED) == 0 {
                    object::set_state_bit_t(mt, cur_item, SELECTED, false);
                }
                close_box(mt, cur_title, cur_box, backend);
                cur_item = NIL;
            }
            cur_title = t;
            cur_box = open_box(mt, t, backend);
        }

        if cur_box != NIL {
            // The submenu box overlaps the parent box's right edge, so check
            // the submenu first (EmuTOS gives `SUBMENU_STATE` priority).
            let in_sub = sub_tree != 0 && point_in(sub_t(sub_tree), sub_box, mx, my);
            if in_sub {
                // Track the highlight *inside* the submenu; the parent item
                // stays selected (cur_item is untouched). `cur_submenu =
                // ob_find(smtree, smroot, 1, x, y)` (`gemmnlib.c:417`) answers
                // the box **itself** for a point inside it that no child
                // covers — its border/gap (`ob_find` returns the last object
                // found, `gemoblib.c:1034-1113`) — and `mn_do` then treats
                // `smroot` like any submenu item: `SUBMENU_STATE` (`:418-422`),
                // hilited by `menu_select(smtree, cur_submenu, …)` (`:474-475`,
                // the box's interior XOR, `ob_change`) and, on a press,
                // returned as `*pitem` (`:494-496`; A103 (1)).
                let hit = object::child_at_t(sub_t(sub_tree), sub_box, mx, my);
                let sit = if hit == NIL { sub_box } else { hit };
                if sit != sub_item {
                    rehilite(sub_t(sub_tree), sub_item, sit, backend);
                    sub_item = sit;
                }
            } else {
                // Not over the submenu: drop any submenu-item highlight but
                // keep the submenu open while we are still on its parent item.
                if sub_item != NIL {
                    rehilite(sub_t(sub_tree), sub_item, NIL, backend);
                    sub_item = NIL;
                }
                let it = object::child_at_t(mt, cur_box, mx, my);
                // Out of the *submenu* into no man's land — no title, no item
                // — `mn_do` keeps the parent: `if (menu_state == SUBMENU_STATE)
                // cur_item = last_item;` (`gemmnlib.c:437-438`), so
                // `item_changed` is false and neither `undisplay_submenu` nor
                // the parent's un-hilite runs (`:452-459`); only the submenu
                // item was un-hilited above (`:446-448`). The state is then
                // `OUTSIDE_STATE`, which no event leaves until re-entry or a
                // button (`:293-305`) — so the retention holds for as long as
                // the pointer stays outside. Out of the parent *item* into no
                // man's land (`INITEM_STATE` → outside) is the plain change
                // below: `cur_item = NIL`, submenu closed, parent un-hilited.
                let keeps_parent = it == NIL && t == NIL && sub_tree != 0 && (was_in_sub || was_outside);
                if it != cur_item && !keeps_parent {
                    // Item changed: close the old item's submenu, move the
                    // parent highlight, then open the new item's submenu.
                    if sub_tree != 0 {
                        close_submenu(sub_t(sub_tree), sub_box, sub_item, backend);
                        sub_tree = 0;
                        sub_box = NIL;
                        sub_item = NIL;
                    }
                    rehilite(mt, cur_item, it, backend);
                    cur_item = it;
                    // `display_submenu` runs only inside `if (menu_select(tree,
                    // cur_item, last_item, TRUE))` (`gemmnlib.c:470-475`), and
                    // `menu_select` → `do_chg(…, chkdisabled = TRUE)` refuses a
                    // DISABLED item (`:158-160`): a disabled item with a
                    // submenu attached neither hilites nor opens it (A103 (2)).
                    if it != NIL && (mt.state(it) & DISABLED) == 0 {
                        if let Some((st, sb)) = open_submenu(mt, it, backend) {
                            sub_tree = st;
                            sub_box = sb;
                            sub_item = NIL;
                        }
                    }
                }
            }
        }

        // Where the pointer is, in `mn_do`'s terms (`aes/gemmnlib.c:398-443`):
        // over a title (`ob_find(tree, THEACTIVE, …)`), in the open box
        // (`ob_find(tree, cur_menu, …)`), in the open submenu, or — none of
        // these — in "no man's land", `OUTSIDE_STATE`. `THEBAR` (the bar row)
        // only matters while no box has been dropped yet (`START_STATE`).
        let in_bar = my < gl_hbox();
        let in_box = cur_box != NIL && point_in(mt, cur_box, mx, my);
        let in_sub_box = sub_tree != 0 && point_in(sub_t(sub_tree), sub_box, mx, my);
        let outside = cur_title != NIL && t == NIL && !in_box && !in_sub_box;

        // Left-button edge over an item → the selection. `INITEM_STATE` waits
        // for whichever transition the live button allows (`gemmnlib.c:314`),
        // so a press with the button up and a release with it held both end
        // the track here; over a title (`cur_item == NIL`) an edge does
        // nothing, as `INTITLE_STATE`'s `buparm ^= 1` does nothing visible.
        // In `OUTSIDE_STATE` the same transition **cancels**: `MU_BUTTON` in
        // any state but `START`/`INTITLE` breaks the loop (`:369-374`), and
        // the cleanup pulls the box up and un-hilites the title without a
        // selection (`cur_item == NIL`, `:482-511`) — `mn_do` returns FALSE.
        // That press is consumed here, under `gl_ctmown`, exactly like a
        // selecting one (`docs/api/aes/menu.md` "Mouse ownership over the
        // track", point 5).
        if down != was_down {
            if sub_tree != 0 && sub_item != NIL {
                if sub_tree == tree as u64 {
                    // Submenu selection in the same tree: the item index is valid
                    // in the caller's tree (round-trips as MN_SELECTED msg[4]).
                    // `sub_item` is the submenu's root box when the press lands
                    // on its border/gap — `*pitem = cur_submenu` with
                    // `cur_submenu == smroot` (`gemmnlib.c:417,494-496`; the
                    // return is gated on the *parent* item being enabled,
                    // `:493`, which it is: a DISABLED parent never opened it).
                    crate::vdi::cursor_hide(backend);
                    object::set_state_bit_t(sub_t(sub_tree), sub_item, SELECTED, false);
                    close_submenu(sub_t(sub_tree), sub_box, NIL, backend);
                    erase_box_inner(cur_box, backend);
                    // The parent item's SELECTED bit is cleared by the same
                    // `do_chg(tree, cur_item, SELECTED, FALSE, FALSE, TRUE)`
                    // that gates the return (`gemmnlib.c:493`) — no draw, the
                    // box's pixels are already back (A103 (1)).
                    let st = mt.state(cur_item);
                    object::change_t(mt, cur_item, st & !SELECTED, false);
                    crate::vdi::cursor_show(backend);
                    return Some((cur_title, sub_item));
                }
                // A submenu in a separate tree: its selection needs the
                // `msg[5..7]` tree-pointer return path (deferred, see the doc
                // of [`track`]); the press is not acted on.
            } else if cur_item != NIL && !in_sub_box {
                // `INITEM_STATE` — on the item, its submenu showing or not —
                // or `OUTSIDE_STATE` reached from the submenu with the parent
                // retained (`gemmnlib.c:437-438`): `MU_BUTTON` breaks the loop
                // (`:373-374`) and the cleanup undisplays the submenu
                // (`:486-490`), restores the box (`:491`) and, the item
                // enabled and `menu_state != SUBMENU_STATE`, returns
                // `(cur_title, cur_item)` (`:492-500`) — the parent item when
                // a submenu was showing.
                //
                // A DISABLED item is never delivered as a selection: the return
                // is gated on `do_chg(tree, cur_item, SELECTED, FALSE, FALSE,
                // TRUE)` (`:493`), which refuses a DISABLED object
                // (`:159-160`), so `done` stays FALSE and mn_do returns 0 — the
                // menu ABORTS, un-hiliting the title (`:510`) after pulling the
                // box up (`:491`): the no-selection cancel path.
                if (mt.state(cur_item) & DISABLED) != 0 {
                    if sub_tree != 0 {
                        close_submenu(sub_t(sub_tree), sub_box, sub_item, backend);
                    }
                    close_box(mt, cur_title, cur_box, backend);
                    return None;
                }
                // Undisplay the submenu first (its save holds the box's pixels
                // in the overlap, the box's save the desktop), then clear the
                // item's highlight bit and restore the screen under the box,
                // cursor hidden so nothing smears. The title stays highlighted:
                // the app un-highlights it via menu_tnormal after acting on
                // MN_SELECTED.
                crate::vdi::cursor_hide(backend);
                if sub_tree != 0 {
                    close_submenu(sub_t(sub_tree), sub_box, NIL, backend);
                }
                object::set_state_bit_t(mt, cur_item, SELECTED, false);
                erase_box_inner(cur_box, backend);
                crate::vdi::cursor_show(backend);
                return Some((cur_title, cur_item));
            } else if outside {
                // `OUTSIDE_STATE` + `MU_BUTTON` → break → cleanup without a
                // selection (`gemmnlib.c:373-374,482-511`): `menu_sr(FALSE)`,
                // `do_chg(tree, cur_title, SELECTED, FALSE, …)`, return FALSE.
                if sub_tree != 0 {
                    close_submenu(sub_t(sub_tree), sub_box, sub_item, backend);
                }
                close_box(mt, cur_title, cur_box, backend);
                return None;
            }
        }

        if cur_title == NIL {
            // `START_STATE`: no box dropped yet. The pointer wandering the bar
            // row off any title keeps waiting (`MU_M1` enter `THEACTIVE`);
            // leaving `THEBAR` (`MU_M2`) ends the track — `cur_menu == NIL` →
            // `cur_title = NIL` → `done` (`gemmnlib.c:286-292,404-411`).
            if !in_bar {
                return None;
            }
        } else if outside {
            // No man's land with a box down. EmuTOS does **not** pull the box
            // up: `OUTSIDE_STATE` (`gemmnlib.c:415-443`) keeps the title
            // hilited and the box on screen — the title is unchanged, so the
            // `menu_select`/`menu_sr(FALSE)` pair at `:463-464` is skipped —
            // and only the item under the pointer was un-hilited (the
            // `item_changed` guard at `:459`, done by the box block above).
            // The next round waits for `MU_BUTTON | M1` (re-enter `THEACTIVE`)
            // `| M2` (re-enter `cur_menu`) `| M3` (re-enter the submenu)
            // (`:293-305`): re-entry resumes the tracking above, a button
            // transition is the cancel handled in the edge block above.
            //
            // The one exception is a DISABLED title (`:429-434`): `cur_title
            // = NIL; done = TRUE;` — `mn_do` ends with no selection and, since
            // `menu_down` never dropped a box for it (`do_chg(…, chkdisabled)`
            // at `:158-160,469`; [`open_box`] answers `NIL` for it, A89),
            // there is nothing to restore — `close_box` with `cur_box == NIL`
            // only brackets the cursor. (With `cur_menu == NIL` EmuTOS ends
            // even earlier, `:406-411`; same outcome.)
            if (mt.state(cur_title) & DISABLED) != 0 {
                if sub_tree != 0 {
                    close_submenu(sub_t(sub_tree), sub_box, sub_item, backend);
                }
                close_box(mt, cur_title, cur_box, backend);
                return None;
            }
        }

        was_down = down;
        was_in_sub = in_sub_box;
        was_outside = outside;
        h.pump_input();
    }
}

/// Move the SELECTED highlight within a drop-down / submenu box from `old` to
/// `new` (either may be `NIL`). The mouse cursor is bracketed hidden/shown so
/// it does not smear across the redraw. Shared by the parent-box and the
/// nested-submenu hover tracking.
///
/// Each highlight change is EmuTOS's `menu_select` → `do_chg` → `ob_change`
/// (`gemmnlib.c:170`): one XOR of the item rect to select, one to deselect
/// (`gemoblib.c:1295-1304`, A99). No white pre-fill — an XOR over a white
/// pre-fill would turn the deselected item black — and nothing after the
/// toggles: `mn_do` repaints no frame (`gemmnlib.c:451-475`). A drop-down box
/// with an *outside* border (`th = -1`, the desktop's `0xFF1100`,
/// `desk/desk_rsc.c:294`; TeraDesk's `desktop.rsc` likewise) keeps its border
/// clear of the item rectangles, so the XOR never reaches it; a box with an
/// inside border gets its frame inverted in the item's rows in EmuTOS too —
/// the frame repaint that compensated for the then kernel desktop's inside
/// border is gone (A100).
fn rehilite(t: ObjTree, old: i16, new: i16, backend: &dyn RasterBackend) {
    crate::vdi::cursor_hide(backend);
    // EmuTOS highlights and un-highlights a drop-down item only through
    // `menu_select` → `do_chg(..., chkdisabled=TRUE)` (`gemmnlib.c:196` calling
    // the guard at `:159-160`; invoked for the un-hilite at `:459` and the
    // hilite at `:471`): a `DISABLED` item is never given the `SELECTED`
    // (inverted) state. So a disabled item under the cursor stays in its normal,
    // un-inverted state, and cannot leave an inverted residue when the
    // drop-down closes.
    if old != NIL && (t.state(old) & DISABLED) == 0 {
        object::set_state_bit_t(t, old, SELECTED, false);
    }
    if new != NIL && (t.state(new) & DISABLED) == 0 {
        object::set_state_bit_t(t, new, SELECTED, true);
    }
    crate::vdi::cursor_show(backend);
}

/// Programmatically open `title`'s drop-down and optionally highlight
/// `item` (`NIL` for none). Returns the box index. Renders the same state
/// the interactive `track` shows mid-drag; also the primitive a future
/// `menu_popup` builds on.
pub fn open(tree: *mut Object, title: i16, item: i16, backend: &dyn RasterBackend) -> i16 {
    let t = ObjTree::new(tree);
    let imenu = open_box(t, title, backend);
    if item != NIL {
        object::set_state_bit_t(t, item, SELECTED, true);
    }
    imenu
}

/// Programmatically close the drop-down [`open`] dropped for `title` (box
/// `imenu`): put the screen under the box back from the AES screen buffer,
/// draw the title normal again. The headless counterpart of the tracker's
/// cancel path, so the save/restore round trip can be verified without a
/// mouse (A71 host test).
pub fn close(tree: *mut Object, title: i16, imenu: i16, backend: &dyn RasterBackend) {
    close_box(ObjTree::new(tree), title, imenu, backend);
}

/// Render `title`'s drop-down with `item` highlighted and, if `item` has an
/// attached submenu, that submenu opened beside it — the static picture the
/// interactive [`track`] shows mid-hover. Returns the submenu box index, or
/// `NIL` when `item` has no submenu. Lets the nested-submenu layout be
/// verified headlessly (QMP screendump); the *live* hover (moving the mouse
/// into the submenu) is real-Pi4 only per project memory.
pub fn open_submenu_static(tree: *mut Object, title: i16, item: i16, backend: &dyn RasterBackend) -> i16 {
    let t = ObjTree::new(tree);
    open_box(t, title, backend);
    // As the track: a DISABLED item is neither hilited nor given its submenu
    // (`menu_select` → `do_chg(…, chkdisabled)`, `gemmnlib.c:158-160,470-475`).
    if (t.state(item) & DISABLED) != 0 {
        return NIL;
    }
    object::set_state_bit_t(t, item, SELECTED, true);
    match open_submenu(t, item, backend) {
        Some((_, box_obj)) => box_obj,
        None => NIL,
    }
}

/// If parent `(tree, item)` has an attached submenu, open it as a nested
/// drop-down: position its box to the **right** of the item overlapping the
/// submenu arrow, or to the **left** if it would run off the right of the
/// screen; align the box so its start item sits level with the parent item;
/// save the pixels underneath (into [`SUBMENU_SAVE`]) and draw it. Mirrors EmuTOS
/// `display_submenu` (`gemmnext.c:625-678`). Returns `(sub_tree_ptr, sub_box)`
/// when a submenu was opened, `None` otherwise.
///
/// The submenu tree is read through the parent's accessor origin
/// ([`ObjTree::with_base`]): for the tracker that is the bar owner's
/// ([`mntree_accessor`], ARCH03 3a F3).
fn open_submenu(t: ObjTree, item: i16, backend: &dyn RasterBackend) -> Option<(u64, i16)> {
    let slot = smib_find_parent(t.tree_ptr(), item)?;
    // SAFETY: single-CPU AES; exclusive owner of the submenu table.
    let s = unsafe { (*SUBMENU_TAB.0.get())[slot] };
    if s.sub_tree == 0 || s.sub_menu < 0 {
        return None;
    }
    let sub_t = t.with_base(s.sub_tree as *const Object);
    let (ix, iy, iw, _ih) = object::abs_rect_t(t, item);
    let (bx0, by0, bw, bh) = object::abs_rect_t(sub_t, s.sub_menu);
    let cw = crate::vdi::font::CELL_W;
    let sw = backend.width() as i16;
    let sh = backend.height() as i16;
    // x: to the right of the item, overlapping the arrow; if that runs off the
    // right edge, put the box to the left of the item instead, clamped on-screen.
    let mut x = ix + iw - cw;
    if x + bw > sw {
        x = (ix - bw).max(0);
    }
    // y: align the submenu's start item with the parent item, then clamp below
    // the menu bar and above the bottom of the screen.
    let start_off = if s.sub_start >= 0 {
        sub_t.y(s.sub_start)
    } else {
        0
    };
    let bar_h = gl_hbox();
    let ymax = (sh - bh).max(bar_h);
    let y = (iy - start_off).clamp(bar_h, ymax);
    // Re-home the box to the target absolute position: adjust its relative
    // ob_x/ob_y by the delta from its current absolute origin, so this works
    // wherever the box sits in the submenu tree (its parent offset cancels).
    sub_t.set_x(s.sub_menu, sub_t.x(s.sub_menu) + (x - bx0));
    sub_t.set_y(s.sub_menu, sub_t.y(s.sub_menu) + (y - by0));
    crate::vdi::cursor_hide(backend);
    let (fx, fy, fw, fh) = sr_rect(sub_t, s.sub_menu, backend);
    // `blitsave = popup_blit(smobj, NULL)` (`gemmnext.c:668-671`): a fresh
    // pool block per opening, never the drop-down's screen buffer — the
    // drop-down is still down and holds it. No block, no submenu
    // (`display_submenu` returns NULL).
    let Some(saved) = gsxif::popup_blit_save(backend, fx, fy, fw, fh) else {
        crate::vdi::cursor_show(backend);
        return None;
    };
    SUBMENU_SAVE.put(saved);
    object::draw_obj_clip_t(sub_t, s.sub_menu, MAX_DEPTH, sr_clip(fx, fy, fw, fh), backend);
    crate::vdi::cursor_show(backend);
    Some((s.sub_tree, s.sub_menu))
}

/// Close an open nested submenu: clear any highlighted submenu item, restore
/// the pixels saved under its box ([`SUBMENU_SAVE`], the block
/// `popup_blit` took) and release that block. Mirrors EmuTOS
/// `undisplay_submenu` (`gemmnext.c:686-703`, `popup_blit(smobj, blitsave)`).
fn close_submenu(sub: ObjTree, sub_box: i16, sub_item: i16, backend: &dyn RasterBackend) {
    if sub.tree_ptr() == 0 || sub_box < 0 {
        return;
    }
    crate::vdi::cursor_hide(backend);
    if sub_item != NIL {
        object::set_state_bit_t(sub, sub_item, SELECTED, false);
    }
    // Restore the pixels saved under the submenu box exactly (no content-
    // destroying white-erase: the saved grown rect is a superset of the draw).
    if let Some(saved) = SUBMENU_SAVE.take() {
        saved.restore(backend);
    }
    crate::vdi::cursor_show(backend);
}

/// Title index → its drop-down box index (EmuTOS `menu_sub`): walk the box
/// list under `THESCREEN.ob_tail` by the title's position.
fn menu_sub(t: ObjTree, title: i16) -> i16 {
    let themenus = t.tail(THESCREEN);
    let mut imenu = t.head(themenus);
    let mut i = title - THEACTIVE;
    while i > 1 {
        imenu = t.next(imenu);
        i -= 1;
    }
    imenu
}

/// Is device point `(mx, my)` inside object `obj`'s absolute rectangle?
fn point_in(t: ObjTree, obj: i16, mx: i16, my: i16) -> bool {
    let (x, y, w, h) = object::abs_rect_t(t, obj);
    mx >= x && mx < x + w && my >= y && my < y + h
}

// ── drop-down save/restore (GEM's "save under" — EmuTOS `menu_sr`) ────────
//
// Opening a menu box covers part of the screen; closing it must put the
// pixels back. The drop-down goes through **the** AES screen buffer
// (`gsxif::bb_save`/`Saved::restore` — EmuTOS `menu_sr`, `gemmnlib.c:208-223`:
// `bb_save(&t)` on the way down, `bb_restore(&t)` on the way up, `gl_tmp`),
// device-exact: 16-bit RGB565 words straight off the screen, so truecolor
// content under the box (an image, a gradient) comes back bit for bit
// (A71). A nested submenu and a pop-up do **not** share that buffer — the
// submenu opens while the drop-down is down and holds it — but take a fresh
// pool block per opening (`gsxif::popup_blit_save` — EmuTOS `popup_blit`,
// `gemmnext.c:136-215`, `dos_alloc_stram`), freed with the restore.
//
// The saved rectangle is the box **grown to cover its decoration**
// ([`sr_rect`]), and the draw is clipped to exactly that rectangle, so
// *saved-area ⊇ drawn-area* always holds. The copies run with the cursor
// hidden (`bb_set`, `gemgsxif.c:367,378`), nested inside the callers' own
// bracket.

/// One parked [`gsxif::Saved`]: the drop-down's (EmuTOS keeps its rectangle
/// implicit in the tree and the pixels in `gl_tmp`) or the submenu's
/// (`blitsave`, `gemmnext.c`).
struct SaveSlot(UnsafeCell<Option<gsxif::Saved>>);
// SAFETY: touched only from the single-CPU AES trap / desktop event path.
unsafe impl Sync for SaveSlot {}
impl SaveSlot {
    const fn new() -> SaveSlot {
        SaveSlot(UnsafeCell::new(None))
    }
    /// Park `saved`; a still-parked earlier save is dropped (its pool block,
    /// if any, released).
    fn put(&self, saved: gsxif::Saved) {
        // SAFETY: single-CPU AES path; exclusive access to this slot.
        unsafe { *self.0.get() = Some(saved) };
    }
    /// Take the parked save, if any.
    fn take(&self) -> Option<gsxif::Saved> {
        // SAFETY: as `put`.
        unsafe { (*self.0.get()).take() }
    }
}
/// The open drop-down's save (`menu_sr`).
static MENU_SAVE: SaveSlot = SaveSlot::new();
/// The open nested submenu's save — EmuTOS `blitsave` (`gemmnext.c`).
static SUBMENU_SAVE: SaveSlot = SaveSlot::new();

/// The EmuTOS `menu_sr` save/restore rectangle for a drop-down `box_obj`: its
/// absolute rectangle grown to cover the decoration `draw_obj` can paint, then
/// clamped to the screen. EmuTOS `menu_sr` grows the box by `MENU_THICKNESS` per
/// side (`gemmnlib.c:215-217`: `g_x -= MT; g_w += 2*MT; g_h += 2*MT`), and the
/// decoration-aware `popup_blit` additionally grows the top and adds `ADJ3DSTD`
/// on the right/bottom for a 3D drop shadow (`gemmnext.c:146-149`:
/// `g_x -= MT; g_y -= MT; g_w += 2*MT+ADJ3DSTD; g_h += 2*MT+ADJ3DSTD`). We
/// reproduce that superset so the saved area covers a `th<0` outward border on
/// every side (QED's `qed.rsc` boxes) plus a shadow, and clip the draw to it.
fn sr_rect(t: ObjTree, box_obj: i16, backend: &dyn RasterBackend) -> (i16, i16, i16, i16) {
    let (x, y, w, h) = object::abs_rect_t(t, box_obj);
    let sw = backend.width() as i16;
    let sh = backend.height() as i16;
    // Grown edges: MENU_THICKNESS on left/top, MENU_THICKNESS+ADJ3DSTD on
    // right/bottom (the `popup_blit` shape), then clamp to the screen.
    let x0 = (x - MENU_THICKNESS).max(0);
    let y0 = (y - MENU_THICKNESS).max(0);
    let x1 = (x + w + MENU_THICKNESS + ADJ3DSTD).min(sw);
    let y1 = (y + h + MENU_THICKNESS + ADJ3DSTD).min(sh);
    (x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
}

/// The VDI clip rectangle covering exactly the grown save rectangle `(x,y,w,h)`.
fn sr_clip(x: i16, y: i16, w: i16, h: i16) -> ClipRect {
    ClipRect { xmn: x, ymn: y, xmx: x + w - 1, ymx: y + h - 1 }
}

/// Highlight `title` and draw its drop-down box; returns the box index. The
/// cursor is hidden across the draw so it stays on top and the save-under
/// snapshot is clean.
///
/// A **DISABLED** title gets neither: `menu_down` (`gemmnlib.c:229-244`)
/// saves and draws the box only if `do_chg(tree, ititle, SELECTED, TRUE,
/// TRUE, TRUE)` succeeds, and `do_chg` refuses a DISABLED object
/// (`:158-160`) — `mn_do` does not even call it for one (`menu_select` at
/// `:466-467` is the same `do_chg`). Answers `NIL`: no box on screen,
/// nothing to restore (A89).
fn open_box(t: ObjTree, title: i16, backend: &dyn RasterBackend) -> i16 {
    if (t.state(title) & DISABLED) != 0 {
        return NIL;
    }
    crate::vdi::cursor_hide(backend);
    object::set_state_bit_t(t, title, SELECTED, true); // redraws the title highlighted
    let imenu = menu_sub(t, title);
    // Save the box grown to its decoration (EmuTOS `menu_sr`), and clip the box
    // draw to exactly that rectangle so the save area is a superset of every
    // pixel drawn — an outward `th<0` border or an overflowing child cannot
    // paint outside the saved region, so the on-close restore is exact.
    let (x, y, w, h) = sr_rect(t, imenu, backend);
    // `menu_sr(TRUE, tree, imenu)` → `bb_save(&t)` (`gemmnlib.c:219,239`):
    // into the one AES screen buffer. An empty rectangle parks nothing — and
    // drops whatever an earlier box left, so the close restores nothing stale.
    match gsxif::bb_save(backend, x, y, w, h) {
        Some(saved) => MENU_SAVE.put(saved),
        None => drop(MENU_SAVE.take()),
    }
    object::draw_obj_clip_t(t, imenu, MAX_DEPTH, sr_clip(x, y, w, h), backend);
    crate::vdi::cursor_show(backend);
    imenu
}

/// Un-highlight `title` and restore the screen under its box. The title's
/// un-highlight is `ob_change`'s XOR toggle of its rect inset by one
/// (`gemoblib.c:1295-1304`, `gemobjop.c:44-46`), which never touches the
/// bar's separator row — nothing to repaint there (A99).
fn close_box(t: ObjTree, title: i16, imenu: i16, backend: &dyn RasterBackend) {
    crate::vdi::cursor_hide(backend);
    erase_box_inner(imenu, backend);
    object::set_state_bit_t(t, title, SELECTED, false);
    crate::vdi::cursor_show(backend);
}

/// Put the pixels saved under the box back exactly (EmuTOS `menu_sr(FALSE, …)`
/// → `bb_restore`, `gemmnlib.c:221`). The saved grown rectangle is a superset
/// of the drawn area, so this fully undoes the box — including any outward
/// border — with no content-destroying white-erase fallback. Caller owns the
/// cursor hide/show.
fn erase_box_inner(imenu: i16, backend: &dyn RasterBackend) {
    if imenu == NIL {
        return;
    }
    if let Some(saved) = MENU_SAVE.take() {
        saved.restore(backend);
    }
}

/// `menu_icheck` (31) — add (`int_in[1]`=1) / remove the check mark on
/// item `int_in[0]`. `do_chg(..., CHECKED, CHECK_IT, FALSE, FALSE)`
/// (`gemsuper.c:172`): **no immediate redraw** (`dodraw=false`) — the mark
/// appears the next time the drop-down is drawn — and **no disabled guard**.
pub fn menu_icheck(pb: &Aespb) {
    do_chg(pb, pb.intin(0), CHECKED, pb.intin(1) != 0, false, false);
}

/// `menu_ienable` (32) — enable (`int_in[1]`=1) / disable item
/// `int_in[0]`. `do_chg(..., ITEM_NUM&0x7fff, DISABLED, !ENABLE_IT,
/// (ITEM_NUM&0x8000)!=0, FALSE)` (`gemsuper.c:175-176`): bit 15 of the item
/// word is the **redraw-now** flag (masked off the index, reused as `dodraw`);
/// no disabled guard.
pub fn menu_ienable(pb: &Aespb) {
    let raw = pb.intin(0);
    let item = raw & 0x7fff;
    let dodraw = (raw as u16) & 0x8000 != 0; // item bit 15 = redraw-now flag
    do_chg(pb, item, DISABLED, pb.intin(1) == 0, dodraw, false); // disable when enable==0
}

/// `menu_tnormal` (33) — draw title `int_in[0]` normal (`int_in[1]`=1) or
/// highlighted/inverted (0). `do_chg(..., TITLE_NUM, SELECTED, !NORMAL_IT,
/// TRUE, TRUE)` (`gemsuper.c:179`): **always redraws** and, with
/// `chkdisabled=true`, refuses a `DISABLED` title (returns `0`, state
/// unchanged).
pub fn menu_tnormal(pb: &Aespb) {
    do_chg(pb, pb.intin(0), SELECTED, pb.intin(1) == 0, true, true); // select when normal==0
}

/// `menu_text` (34) — replace item `int_in[0]`'s text with the string at
/// `addr_in[1]`, in place. EmuTOS is an inline `strcpy` into `ob_spec` with
/// **no redraw** (`gemsuper.c:183`): the caller repaints the item itself
/// (e.g. via `objc_draw`, or a `menu_ienable` with item bit 15 set). Bounded
/// to 128 bytes as a NewTOS safety cap over EmuTOS's unbounded copy — see the
/// spec "NewTOS implementation notes".
pub fn menu_text(pb: &Aespb) {
    let tree = pb.addrin(0) as *const Object;
    let item = pb.intin(0);
    let src = pb.addrin(1);
    if tree.is_null() || item < 0 || src == 0 {
        pb.set_ret(0);
        return;
    }
    // Read `ob_spec` (the item's text buffer pointer) at the caller's ABI
    // width — this is exactly the QED fault site: an ILP32 tree's `ob_spec`
    // lives at byte 12 as a u32, not at byte 16 as a u64, so the previous
    // 32-byte-stride read handed back a wild pointer.
    let dst = ObjTree::new(tree).spec(item);
    // The item's ob_spec points at a writable text buffer sized by the
    // caller to hold the replacement string plus its NUL — the GEM contract,
    // exactly as EmuTOS's unbounded `strcpy((char *)ob_spec, (char *)MM_PTEXT)`
    // (gemsuper.c:183) assumes. Copy the full source string INCLUDING the
    // terminating NUL, with no length cap, so the result is byte-identical to
    // EmuTOS. A prior 128-byte cap truncated >=128-byte strings AND left the
    // destination un-terminated (read_cstr then ran past the copy into random
    // trailing bytes) — a real, if pathological, divergence; and it protected
    // nothing, since a caller whose buffer is shorter than the new string
    // overflows identically in both EmuTOS and NewTOS.
    //
    // Both strings are the caller's — its source string and the text buffer
    // of its own menu item — and are read and written in its space (ARCH03
    // 3a, F2): byte by byte with the program's rights, so the copy stops at
    // the first byte the program could not read or write itself, the fault
    // is recorded, and the syscall tail ends the program (EmuTOS: bus error
    // inside the `strcpy`). No byte goes through a raw pointer.
    if dst != 0 {
        let space = pb.space();
        let mut i = 0u64;
        loop {
            let Ok(c) = space.get_u8(src + i) else {
                return;
            };
            if space.put_u8(dst + i, c).is_err() {
                return;
            }
            if c == 0 {
                break;
            }
            i += 1;
        }
    }
    pb.set_ret(1);
}

// ── Desk-accessory registry (EmuTOS `gemmnlib.c:64-65`) ──────────────────
//
// `D.g_acctitle[NUM_ACCS]` (the title *pointers* — stored, never copied,
// `:806` "save pointer, like Atari TOS"), `desk_ppd[NUM_ACCS]` (the owning
// process descriptors) and `acc_display[NUM_ACCS]` (display slot → menu id).
// `D.g_accreg` is the count; here it is derived from the title array, which
// carries exactly the same information.

/// `NUM_ACCS` — the one shared budget for loaded accessory files *and*
/// registration slots (`include/sysconf.h:37-40`), re-exported from
/// [`super::acc`] so both halves of the lifecycle use one constant.
use super::acc::NUM_ACCS;

struct Accs {
    /// `D.g_acctitle[]` — the caller's title-string pointer per slot
    /// (`0` = free). Stored as given, at the caller's own address; EmuTOS
    /// does the same and the spec records the consequence (the string must
    /// outlive the registration, `docs/api/aes/menu.md` § note 6).
    title: UnsafeCell<[u64; NUM_ACCS]>,
    /// `desk_ppd[]` — the **AES identity** of the registering process
    /// (`appl::current_identity`), `usize::MAX` = free.
    ///
    /// EmuTOS stores `rlr`, the descriptor of the process *making the call*,
    /// not the `pid` argument (`gemmnlib.c:805`) — the argument is only
    /// validity-checked (`pid >= 0`, `:798`). NewTOS does the same: the
    /// argument is a *reported* pid (`appl::reported_pid` — 0 for the chain,
    /// 2, 3, … for an accessory, A56/A113), not an identity, and EmuTOS never
    /// resolves it.
    owner: UnsafeCell<[usize; NUM_ACCS]>,
    /// `acc_display[]` — display slot → menu id, rebuilt by
    /// [`build_menuid_lookup`] after every registration.
    display: UnsafeCell<[i16; NUM_ACCS]>,
}

// **A kernel-side copy of each title string** — the one place NewTOS cannot
// do what `gemmnlib.c:806` does.
//
// EmuTOS stores the accessory's pointer and the menu renderer dereferences it
// later: one address space, always valid. Here the pointer is an **EL0 VA of
// the accessory's own TPA**, and the renderer runs at EL1 under whatever
// `TTBR0` the *current* client has — the desktop's, during whose drop-down
// the entry is drawn. Writing that VA into the desktop's tree faulted the AES
// outright (measured: data abort, `FAR = 0x1610`, on the first hover over
// *Desk*), which is the A97 class of defect exactly.
//
// So the string is copied, at registration, while the accessory *is* the
// current process and the read is safe, and [`menu_fixup`] writes the
// **copy's** address into the placeholder's `ob_spec`. The observable
// difference is narrow and in the permissive direction: EmuTOS requires the
// caller to keep the string alive for the life of the registration
// (`docs/api/aes/menu.md` note 6) and NewTOS does not, and a caller that
// *edits* its title in place after registering sees the change only at its
// next `menu_register`/`menu_bar` (where the copy is refreshed) rather than
// at the next repaint. No accessory does either.
//
// **The copies are readable by every program** (TASK/ACCTITLE). On EmuTOS the
// string `menu_fixup` links into an application's Desk drop-down is ordinary
// memory the application reads like its own — and applications do: WinDom's
// `menu_exec_cmd` `strstr`s every Desk entry on each unhandled key
// (`sdk/windom/windom-1.21.3/src/menu_execcmd.c`), as does any program that
// walks its menu for shortcuts. A kernel high-half address there is a
// permission fault at EL0 (zView on the real Pi 4, 2026-10-07). The copies
// therefore live in a page of their own that is published read-only to every
// process at VA = PA (`usermem::expose`, `PROT_PR`, ADR-023 — the pattern of
// `bdos::process::standin`), and an application's tree gets the **user
// address** of its copy; a kernel-installed tree (the built-in desktop's
// until DESK43 U10, today only the selftests'), read by kernel code only,
// keeps the kernel address (the per-reader choice `Getmpb` and
// `wind_get(WF_SCREEN)` make).

/// Bytes reserved per copied accessory title, NUL included. The *Desk*
/// drop-down is [`MIN_DESKMENU_WIDTH`]-ish characters wide and classic
/// accessory titles are far shorter (`" NewTOS System"` is 14); EmuTOS has no
/// limit because it copies nothing. A longer title is cut here — the cap is
/// the price of the copy.
///
/// [`MIN_DESKMENU_WIDTH`]: crate::aes::menu
const ACC_TITLE_LEN: usize = 32;

/// The page the copied titles live in — **nothing else**, because the whole
/// page becomes readable to every process when it is published.
#[repr(C, align(4096))]
struct TitlePage([[u8; ACC_TITLE_LEN]; NUM_ACCS]);

const _: () = assert!(core::mem::size_of::<TitlePage>() == 4096);

struct TitleCell(UnsafeCell<TitlePage>);
// SAFETY: single boot CPU only touches the AES.
unsafe impl Sync for TitleCell {}

/// The copied titles, one [`ACC_TITLE_LEN`] row per registry slot.
static TITLES: TitleCell = TitleCell(UnsafeCell::new(TitlePage([[0; ACC_TITLE_LEN]; NUM_ACCS])));

/// The user address [`TITLES`] is published at, `0` until the first
/// publication ([`titles_user_va`]). A publication is permanent.
static TITLES_USER_VA: AtomicU64 = AtomicU64::new(0);

// SAFETY: single boot CPU only touches the AES.
unsafe impl Sync for Accs {}
static ACCS: Accs = Accs {
    title: UnsafeCell::new([0; NUM_ACCS]),
    owner: UnsafeCell::new([usize::MAX; NUM_ACCS]),
    display: UnsafeCell::new([-1; NUM_ACCS]),
};

/// Copy the NUL-terminated title string at the caller VA `pstr` into slot
/// `idx`'s row of [`TITLES`] — see there for why a copy exists at all.
/// Called from [`menu_register`]; the string is read in the caller's `space`
/// (ARCH03 3a), and `Err` — fault recorded, slot `idx` left empty — when the
/// program cannot read it.
fn copy_title(space: crate::uaccess::Space, idx: usize, pstr: u64) -> Result<(), crate::uaccess::UserFault> {
    // SAFETY: single-CPU AES; exclusive owner of the registry.
    let dst = unsafe { &mut (*TITLES.0.get()).0[idx] };
    dst.fill(0);
    if pstr != 0 {
        // The last byte stays 0: the copy is always NUL-terminated, however
        // long the caller's string is. The GEM contract — `addr_in[0]` is a
        // NUL-terminated string in the caller's address space.
        if let Err(e) = space.read_cstr(pstr, &mut dst[..ACC_TITLE_LEN - 1]) {
            dst.fill(0);
            return Err(e);
        }
    }
    Ok(())
}

/// The kernel address of slot `idx`'s copied title.
fn title_copy_kva(idx: usize) -> u64 {
    TITLES.0.get() as u64 + (idx * ACC_TITLE_LEN) as u64
}

/// The user address of [`TITLES`], publishing the page read-only to every
/// process at VA = PA on first use (`usermem::expose`, `PROT_PR`, ADR-023).
/// `None` when the publication fails (the user tables are full) — no program
/// could then read a title.
fn titles_user_va() -> Option<u64> {
    let va = TITLES_USER_VA.load(Ordering::Relaxed);
    if va != 0 {
        return Some(va);
    }
    let kva = TITLES.0.get() as *const u8;
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: AES trap or boot context (the BDOS lock, single CPU); the page
    // is a `'static`, page-aligned kernel object that holds nothing but the
    // title copies and is never freed. `expose` answers the same address for
    // every process; the slot only matters for an uncached publication.
    let published = unsafe {
        crate::bdos::process::usermem::expose(0, kva, core::mem::size_of::<TitlePage>(), tos_types::ProtMode::Pr)
    };
    // On the host user memory is host memory (`usermem::resolve_kva`): there
    // are no user tables to enter the page into.
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let published = Some(crate::bdos::process::kva_to_user_va(kva as u64));
    if let Some(va) = published {
        TITLES_USER_VA.store(va, Ordering::Relaxed);
    }
    published
}

/// The `ob_spec` [`menu_fixup`] links for slot `idx` into a tree read through
/// `t`: the kernel address for a kernel-built tree (a kernel-installed bar,
/// read by kernel code only), the published user address for an
/// application's tree, which the application reads itself. `0` (the empty
/// string to the drawer) if the page could not be published.
fn title_spec(t: ObjTree, idx: usize) -> u64 {
    if t.kernel_built() {
        return title_copy_kva(idx);
    }
    titles_user_va().map_or(0, |base| base + (idx * ACC_TITLE_LEN) as u64)
}

/// `gl_dafirst` (`aes/gemmnlib.c:68`) — the object index of the **first**
/// desk-accessory entry in the installed menu tree's Desk drop-down, set by
/// [`menu_fixup`]. `NIL` while no tree is installed or `menu_fixup` has not
/// run, which makes the `item >= gl_dafirst` test in the click router
/// impossible to pass.
static DAFIRST: AtomicI16 = AtomicI16::new(NIL);

/// `THEDESK` (`aes/gemctrl.c:42`) — "MUST be the same value as `DESKMENU` in
/// `desk/desk_rsc.h`", i.e. object **3**: the first `G_TITLE` under
/// `THEACTIVE` in every GEM menu tree (`desk_rsc.c:258-264`,
/// `gem_rsc.c`'s own bar). A pick under this title at or past
/// [`DAFIRST`] is an accessory, not an application menu item.
const THEDESK: i16 = 3;

/// `gl_dafirst` — the first accessory object of the installed tree, or [`NIL`].
pub(super) fn gl_dafirst() -> i16 {
    DAFIRST.load(Ordering::Relaxed)
}

/// An accessor over the **installed** menu tree, read at the ABI and through
/// the address space of the client that installed it ([`MNPPD`]) rather than
/// of whoever is calling.
///
/// EmuTOS needs no such care: one address space, one pointer width. NewTOS
/// reaches `gl_mntree` from two contexts that are *not* its owner —
/// [`menu_fixup`] called out of a desk accessory's `menu_register`, and the
/// Control Manager's drop-down tracker — and an ILP32 app's 24-byte records
/// read at LP64 width, or an EL0 VA walked under a different `TTBR0`, is the
/// A97 class of defect. A kernel-installed tree (`MNPPD == pd::KERNEL_CONTEXT`)
/// is native LP64 and needs no owner mapping.
fn mntree_accessor(tree: *mut Object) -> ObjTree {
    let owner = MNPPD.load(Ordering::Relaxed);
    match crate::bdos::process::slot(owner) {
        Some(p) => ObjTree::app_owned(tree, p.read_abi(), Some(owner)),
        None => ObjTree::lp64(tree),
    }
}

/// `menu_fixup()` (`aes/gemmnlib.c:89-125`) — splice the registered desk
/// accessories into the installed menu tree's Desk drop-down.
///
/// ```c
/// themenus = tree[THESCREEN].ob_tail;
/// dabox = tree[themenus].ob_head;
/// tree[dabox].ob_head = tree[dabox].ob_tail = NIL;
/// gl_dafirst = dabox + 3;
/// cnt = (D.g_accreg) ? (2 + D.g_accreg) : 1;
/// for (i = 1, st = 0, height = 0; i <= cnt; i++) {
///     ob = dabox + i;  ob_add(tree, dabox, ob);
///     if (i > 2) { find next used g_acctitle[st]; tree[ob].ob_spec = it; }
///     height += gl_hchar;
/// }
/// tree[dabox].ob_height = height;
/// ```
///
/// The Desk box is **relinked from scratch** every time: its children are
/// dropped and `cnt` of the `dabox + 1 …` placeholder objects are re-added.
/// With no accessory registered `cnt == 1`, so only the About item is linked —
/// the separator at `dabox + 2` disappears too, which is why EmuTOS's Desk
/// menu shows a single entry on a machine with no `.ACC` files.
///
/// Called from `mn_bar`'s show path (`:714`) — so *every* application's own
/// menu bar gets the accessories, exactly as the classic GEM convention of
/// reserving six blank items under *Desk* expects — and from
/// [`menu_register`] (`:808`), so a registration that arrives while a bar is
/// up takes effect at once.
///
/// A tree whose Desk box has fewer than `2 + NUM_ACCS` placeholder objects
/// after it is a caller that did not follow that convention; EmuTOS would
/// relink objects that belong to the *next* drop-down. NewTOS stops at the
/// box's own object budget instead — the **one deviation here**, taken because
/// the alternative corrupts an application's tree (see the `avail` bound).
pub(super) fn menu_fixup() {
    let tree = active();
    if tree.is_null() {
        return; // `if ((tree = gl_mntree) == NULL) return;` (:95-96)
    }
    let t = mntree_accessor(tree);
    let themenus = t.tail(THESCREEN);
    if themenus == NIL {
        return;
    }
    let dabox = t.head(themenus);
    if dabox == NIL {
        return;
    }
    // How many objects the Desk box may relink — the **one deviation** here.
    // EmuTOS writes `dabox + 1 … dabox + cnt` unconditionally, trusting the
    // RSC to have reserved `2 + NUM_ACCS` objects for the box; a tree that did
    // not reserve them has its neighbour's objects (or memory past the array)
    // written. On EmuTOS that is a malformed-resource bug with no MMU to catch
    // it; here it is a fault in the AES, caused by an application's tree. So
    // the count is bounded:
    //
    //  * by the **next drop-down box**, when there is one — every real menu
    //    bar has at least two menus, and the objects between the Desk box and
    //    the following box are exactly its own (`desk_rsc.c:345-407`: box 8,
    //    items 9…16, next box 17);
    //  * otherwise by the box's **current child count**, since a conforming
    //    single-menu tree arrives with its placeholders linked. (A second
    //    fixup of such a tree sees only what the first one left linked, so it
    //    can splice fewer entries than the tree could hold — a cosmetic limit
    //    on a shape no GEM menu bar actually has, never a fault.)
    let cap = if t.next(dabox) > dabox {
        t.next(dabox) - dabox - 1
    } else {
        child_count(t, dabox)
    };
    if cap < 1 {
        return; // nothing reserved: leave the tree exactly as it is
    }

    t.set_head(dabox, NIL); // `tree[dabox].ob_head = … = NIL` (:104)
    t.set_tail(dabox, NIL);
    DAFIRST.store(dabox + 3, Ordering::Relaxed); // `gl_dafirst = dabox + 3` (:105)

    let accreg = registered_count();
    let cnt = if accreg > 0 { 2 + accreg as i16 } else { 1 }.min(cap);

    let hchar = super::graf::gl_hchar();
    let mut st = 0usize;
    let mut height = 0i16;
    for i in 1..=cnt {
        let ob = dabox + i;
        object::ob_add(t, dabox, ob); // `ob_add(tree, dabox, ob)` (:110)
        if i > 2 {
            // "the DAs come after the accessory separator line" (:111)
            // SAFETY: single-CPU AES; exclusive owner of the registry.
            let titles = unsafe { &*ACCS.title.get() };
            while st < NUM_ACCS && titles[st] == 0 {
                st += 1;
            }
            if st >= NUM_ACCS {
                break; // "should not happen" (:117)
            }
            // `tree[ob].ob_spec = (LONG)D.g_acctitle[st++];` (`:118`) — with
            // the **copy's** address, not the accessory's own pointer, and the
            // one the tree's reader can follow (see [`TITLES`]).
            t.set_raw_spec(ob, title_spec(t, st));
            st += 1;
        }
        height += hchar;
    }
    t.set_height(dabox, height); // `tree[dabox].ob_height = height` (:123)
}

/// How many children `parent` currently has, walking the `ob_next` sibling
/// chain from its `ob_head` to its `ob_tail` (EmuTOS's own traversal shape,
/// `ob_add`/`ob_delete`, `aes/gemoblib.c`). Bounded by the sibling count a
/// menu drop-down can have, so a corrupt chain terminates the walk instead of
/// looping.
fn child_count(t: ObjTree, parent: i16) -> i16 {
    let mut n = 0i16;
    let mut child = t.head(parent);
    while child != NIL && child != parent && n < 64 {
        n += 1;
        if child == t.tail(parent) {
            break;
        }
        child = t.next(child);
    }
    n
}

/// `build_menuid_lookup()` (`aes/gemmnlib.c:766-782`) — `acc_display[n]` is
/// the menu id of the `n`-th accessory *as displayed*, i.e. the `n`-th used
/// `g_acctitle[]` slot; the remainder is `-1`.
///
/// **EmuTOS's padding loop skips every other entry** (`for ( ; slot <
/// NUM_ACCS; slot++) acc_display[slot++] = -1;` — the body increments `slot`
/// a second time, `:779-780`). It is unreachable through the documented API
/// (`mn_getownid` is only ever called with a display slot below the
/// registered count, `docs/api/aes/menu.md` § DA-slot model), so NewTOS fills
/// the tail properly rather than reproduce a bug no caller can observe.
fn build_menuid_lookup() {
    // SAFETY: single-CPU AES; exclusive owner of the registry.
    unsafe {
        let titles = &*ACCS.title.get();
        let display = &mut *ACCS.display.get();
        let mut slot = 0usize;
        for (i, &tp) in titles.iter().enumerate() {
            if tp != 0 {
                display[slot] = i as i16;
                slot += 1;
            }
        }
        for d in &mut display[slot..] {
            *d = -1;
        }
    }
}

/// `D.g_accreg` — how many accessory slots are registered.
fn registered_count() -> usize {
    // SAFETY: single-CPU AES; exclusive owner of the registry.
    unsafe { (*ACCS.title.get()).iter().filter(|t| **t != 0).count() }
}

/// `mn_getownid(&owner, &id, item)` (`aes/gemmnlib.c:838-848`) — map a Desk
/// drop-down **display slot** to the owning accessory's `(identity, menu id)`.
/// `None` for a slot that names no registration (EmuTOS's "paranoia" guard,
/// `:844`, which leaves the caller's out-parameters untouched).
pub(super) fn mn_getownid(item: i16) -> Option<(usize, i16)> {
    if item < 0 || item as usize >= NUM_ACCS {
        return None;
    }
    // SAFETY: single-CPU AES; exclusive owner of the registry.
    unsafe {
        let n = (*ACCS.display.get())[item as usize];
        if n < 0 || n as usize >= NUM_ACCS {
            return None;
        }
        let owner = (*ACCS.owner.get())[n as usize];
        (owner != usize::MAX).then_some((owner, n))
    }
}

/// `hctl_rect`'s desk-accessory test (`aes/gemctrl.c:337-340`):
///
/// ```c
/// if ((title == THEDESK) && (item >= gl_dafirst)) {
///     item -= gl_dafirst;
///     mn_getownid(&owner, &item, item);   /* display slot -> (owner, menu id) */
/// ```
///
/// Returns the accessory's `(identity, menu id)` for a pick that belongs to a
/// registered desk accessory, `None` for an ordinary application menu item
/// (which becomes `MN_SELECTED` instead). `gl_dafirst` is [`NIL`] until
/// [`menu_fixup`] has run on an installed tree, and `item >= NIL` would then
/// match everything, so the sentinel is rejected explicitly.
pub(super) fn desk_accessory_pick(title: i16, item: i16) -> Option<(usize, i16)> {
    let first = gl_dafirst();
    if title != THEDESK || first == NIL || item < first {
        return None;
    }
    mn_getownid(item - first)
}

/// `do_chg(gl_mntree, title, SELECTED, FALSE, TRUE, TRUE)` (`gemctrl.c:341`) —
/// un-highlight a menu-bar title and repaint it, skipping a DISABLED one
/// (`chkdisabled = TRUE`, `gemmnlib.c:158-159`). The drop-down tracker leaves
/// the title selected for the application to clear with `menu_tnormal`; for a
/// desk-accessory pick there is no application to do it, so the Control
/// Manager does it here.
pub(super) fn normalize_title(title: i16) {
    let tree = active();
    if tree.is_null() || title < 0 {
        return;
    }
    let t = mntree_accessor(tree);
    let state = t.state(title);
    if state & DISABLED != 0 {
        return;
    }
    object::change_t(t, title, state & !SELECTED, true);
}

/// `hctl_rect`'s `CONF_WITH_MENU_EXTENSION` payload of an application menu
/// pick (`aes/gemctrl.c:353-358`):
///
/// ```c
/// treehi = HIWORD(tree);
/// treelo = LOWORD(tree);
/// parent = get_par(tree, item);
/// ```
///
/// returned as `[treehi, treelo, parent]` for `MN_SELECTED`'s `msg[5..7]`
/// (`ct_msgup(mesag, owner, title, item, treehi, treelo, parent)`,
/// `:365`). `tree` is the tree [`track`] picked `item` in — the installed bar
/// (`gl_mntree`), which is also the tree of every submenu [`track`] delivers
/// (`docs/api/aes/menu.md`, "`MN_SELECTED` message", NewTOS binding). The
/// pointer is the client's own address as it passed it to [`menu_bar`]; the
/// parent is read through the bar owner's accessor, like the track itself.
pub(super) fn mn_selected_ext(tree: *mut Object, item: i16) -> [i16; 3] {
    let addr = tree as u64;
    let treehi = (addr >> 16) as u16 as i16; // HIWORD
    let treelo = addr as u16 as i16; // LOWORD
    let parent = object::parent_t(mntree_accessor(tree), item);
    [treehi, treelo, parent]
}

/// `mn_cleanup()` (`aes/gemmnlib.c:740-758`) — "tell all DAs that the
/// currently running application is about to terminate":
///
/// ```c
/// for (i = 0; i < NUM_ACCS; i++)
///     if (desk_ppd[i])
///         ap_sendmsg(appl_msg, AC_CLOSE, desk_ppd[i], i, 0, 0, 0, 0);
/// ```
///
/// `ap_sendmsg`'s fixed argument order (`gemwmlib.c:851-862`) puts the
/// accessory's **menu id in `msg[3]`** and `0` in `msg[4]` — the mirror image
/// of `AC_OPEN`, which arrives through the generic menu-click path and
/// therefore carries the id in `msg[4]`. The asymmetry is EmuTOS's and is
/// contract (`docs/api/aes/accessories.md` §3).
///
/// `msg[1]` is `rlr->p_pid`, the *sender's* pid — the caller of `appl_exit`
/// (`appl::reported_pid` of the caller): 0 for a member of the application
/// chain, the accessory's own pid (2, 3, …) if an accessory calls
/// `appl_exit` (`gemmnlib.c:747`, spec `application.md` `appl_exit` step 2).
///
/// EmuTOS's submenu half (`dos_free(gl_submenu)`, `:751-756`) has no NewTOS
/// counterpart: the submenu table is a static array, not an allocation
/// ([`SUBMENU_TAB`]); it is reset by [`reinit`] at the AES restart, which is
/// where its lifetime actually ends.
///
/// Returns the number of accessories the message went to.
pub(super) fn mn_cleanup() -> usize {
    let sender = super::appl::reported_pid(super::appl::current_identity());
    let mut sent = 0;
    for i in 0..NUM_ACCS {
        // SAFETY: single-CPU AES; exclusive owner of the registry.
        let owner = unsafe { (*ACCS.owner.get())[i] };
        if owner == usize::MAX {
            continue;
        }
        let mut msg = [0i16; super::pd::MSG_WORDS];
        msg[0] = AC_CLOSE;
        msg[1] = sender;
        msg[3] = i as i16; // `w3 = i` — the accessory's menu id
        // SAFETY: single trap path; `post_msg` hands the message to the
        // accessory's FIFO and wakes it (`aqueue`, `gemqueue.c:85-125`).
        unsafe {
            super::pd::post_msg(owner, msg);
        }
        sent += 1;
    }
    sent
}

/// `menu_register` (35) — register a desk accessory. `int_in[0]` = the
/// application id, `addr_in[0]` = its menu-title string. Returns the
/// accessory's menu id (`0..NUM_ACCS-1`) or `-1` when the id is negative or
/// all six slots are taken (EmuTOS `mn_register`, `gemmnlib.c:789-816`).
///
/// The registering process is identified by `appl::current_identity()` — the
/// `rlr` EmuTOS records (`:805`) — and not by `int_in[0]`, which is only
/// validity-checked; see [`Accs::owner`].
pub fn menu_register(pb: &Aespb) {
    let pid = pb.intin(0);
    let pstr = pb.addrin(0);
    // `if ((pid >= 0) && (D.g_accreg < NUM_ACCS))` (:798)
    if pid < 0 || registered_count() >= NUM_ACCS {
        pb.set_ret(-1); // `else return -1;` (:815)
        return;
    }
    // SAFETY: single-CPU AES; exclusive owner of the registry.
    let slot = unsafe {
        let title = &mut *ACCS.title.get();
        let owner = &mut *ACCS.owner.get();
        // `for (openda = 0; …) if (!D.g_acctitle[openda]) break;` (:801-805)
        let Some(openda) = title.iter().position(|t| *t == 0) else {
            pb.set_ret(-1);
            return;
        };
        // The title copy first (ARCH03 3a): a string the program cannot read
        // ends the call (and the program) with the registry unchanged.
        if copy_title(pb.space(), openda, pstr).is_err() {
            return;
        }
        title[openda] = pstr; // `D.g_acctitle[openda] = pstr` (:806)
        owner[openda] = super::appl::current_identity(); // `desk_ppd[openda] = rlr` (:805)
        openda
    };
    menu_fixup(); // (:808)
    build_menuid_lookup(); // (:809)
    pb.set_ret(slot as i16); // `return openda;` (:810)
}

/// Shared helper: set/clear one `ob_state` bit on `obj`, optionally redrawing
/// it (EmuTOS `do_chg`, `gemmnlib.c:145-172`).
///
/// - `set`: set (`true`) or clear (`false`) `stateflag` in `ob_state` (EmuTOS
///   `dochg`).
/// - `dodraw`: repaint the object after the change (EmuTOS `dodraw`). When
///   `false`, `ob_state` is still updated but nothing is drawn — the change
///   shows on the next draw of the enclosing box.
/// - `chkdisabled`: if `true` and the object is already `DISABLED`, refuse the
///   change — leave `ob_state` untouched, draw nothing, return `0`.
///
/// Returns `1` normally, `0` only when the disabled guard blocks the change.
fn do_chg(pb: &Aespb, obj: i16, stateflag: u16, set: bool, dodraw: bool, chkdisabled: bool) {
    let tree = pb.addrin(0) as *mut Object;
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    let state = ObjTree::new(tree).state(obj);
    if chkdisabled && (state & DISABLED) != 0 {
        pb.set_ret(0); // disabled item: no mutation, no draw
        return;
    }
    let new = if set { state | stateflag } else { state & !stateflag };
    object::change(tree, obj, new, dodraw);
    pb.set_ret(1);
}

// ── menu extensions (later-AES: pop-up / sub-menu / settings) ─────────────

/// Decoded `MENU` descriptor (later-AES pop-up / sub-menu). This is a kernel
/// value materialised from the caller's parameter block by [`Menu::read_abi`],
/// **not** a memory overlay: the on-wire `MENU` is packed at the caller's ABI
/// width (gemlib `mt_gem.h:2177-2191`,
/// `{ OBJECT *mn_tree; short mn_menu, mn_item, mn_scroll, mn_keystate; }`),
/// whose leading `mn_tree` pointer is 8 bytes at LP64 but 4 bytes at ILP32,
/// shifting the four trailing WORDs. So a plain `&*(base as *const Menu)` would
/// misread an ILP32 app's 12-byte MENU at the native 16-byte layout.
struct Menu {
    mn_tree: u64,   // OBJECT*
    mn_menu: i16,   // the drop-down box object
    mn_item: i16,   // start / selected item
    mn_scroll: i16,
    mn_keystate: i16,
}

/// Pointer-field width for `abi`: 8 bytes LP64, 4 bytes ILP32. In a `MENU` the
/// leading `mn_tree` pointer strides by this, shifting the four trailing WORDs.
#[inline]
fn menu_ptr_size(abi: Abi) -> u64 {
    match abi {
        Abi::Lp64 => 8,
        Abi::Ilp32 => 4,
    }
}

impl Menu {
    /// Materialise the `MENU` at `base` at the caller's ABI width. Layout
    /// (gemlib `mt_gem.h:2177-2191`): one pointer (`mn_tree`) @0, then four WORDs
    /// (`mn_menu`/`mn_item`/`mn_scroll`/`mn_keystate`) at [`menu_ptr_size`]
    /// (@8 LP64 / @4 ILP32). `mn_tree` is read 8 bytes LP64 / 4 bytes
    /// zero-extended ILP32 — a submenu OBJECT-tree user VA reachable via the
    /// active TTBR0, passed to the menu machinery unchanged. For an LP64 base the
    /// offsets equal the native struct, so the value is byte-identical to the old
    /// direct dereference and LP64 callers are unaffected.
    ///
    /// # Safety
    /// `base` addresses a valid `MENU` packed at `abi` width (GEM contract).
    #[inline]
    unsafe fn read_abi(base: u64, abi: Abi) -> Menu {
        let w = menu_ptr_size(abi); // first WORD offset
        // SAFETY: `base` is a valid MENU packed at `abi` width (caller's
        // contract); each field slot is only 2-byte aligned, so read unaligned.
        unsafe {
            let mn_tree = match abi {
                Abi::Lp64 => (base as *const u64).read_unaligned(),
                Abi::Ilp32 => (base as *const u32).read_unaligned() as u64,
            };
            Menu {
                mn_tree,
                mn_menu: ((base + w) as *const i16).read_unaligned(),
                mn_item: ((base + w + 2) as *const i16).read_unaligned(),
                mn_scroll: ((base + w + 4) as *const i16).read_unaligned(),
                mn_keystate: ((base + w + 6) as *const i16).read_unaligned(),
            }
        }
    }

    /// Write this `MENU` back to `base` at the caller's ABI width (the inverse of
    /// [`Self::read_abi`]): `mn_tree` as an 8-byte LP64 / low-4-byte ILP32
    /// pointer, then the four WORDs at the ABI-shifted offsets. Byte-identical to
    /// the old direct write for an LP64 base.
    ///
    /// # Safety
    /// `base` addresses a valid, writable `MENU` packed at `abi` width.
    #[inline]
    unsafe fn write_abi(&self, base: u64, abi: Abi) {
        let w = menu_ptr_size(abi);
        // SAFETY: `base` is a valid writable MENU packed at `abi` width (caller's
        // contract); each field slot is only 2-byte aligned, so write unaligned.
        unsafe {
            match abi {
                Abi::Lp64 => (base as *mut u64).write_unaligned(self.mn_tree),
                Abi::Ilp32 => (base as *mut u32).write_unaligned(self.mn_tree as u32),
            }
            ((base + w) as *mut i16).write_unaligned(self.mn_menu);
            ((base + w + 2) as *mut i16).write_unaligned(self.mn_item);
            ((base + w + 4) as *mut i16).write_unaligned(self.mn_scroll);
            ((base + w + 6) as *mut i16).write_unaligned(self.mn_keystate);
        }
    }

    /// Write only the `mn_keystate` WORD of the `MENU` at `base` — the last of
    /// the four WORDs behind the `mn_tree` pointer, at the caller's ABI offset
    /// (`@14` LP64 / `@10` ILP32). The other four fields are not touched.
    ///
    /// # Safety
    /// `base` addresses a valid, writable `MENU` packed at `abi` width.
    #[inline]
    unsafe fn write_keystate_abi(base: u64, abi: Abi, keystate: i16) {
        let w = menu_ptr_size(abi);
        // SAFETY: `base` is a valid writable MENU packed at `abi` width (caller's
        // contract); the slot is only 2-byte aligned, so write unaligned.
        unsafe { ((base + w + 6) as *mut i16).write_unaligned(keystate) };
    }
}

/// `menu_popup` (36) — display the pop-up menu described by `addr_in[0]`
/// (`MENU`) so its start item sits at (`int_in[0]`, `int_in[1]`), track it,
/// and write the chosen item into the `MENU` at `addr_in[1]`. Returns 1 on a
/// selection, 0 on cancel. EmuTOS `mn_popup` (`gemmnext.c:304-356`, single
/// level, no sub-menus).
///
/// The whole call runs inside `wm_update(BEG_MCTRL)` … `wm_update(END_MCTRL)`
/// (`:312,353`) = `fm_own(TRUE/FALSE)` (`gemwmlib.c:1778-1797`,
/// `gemfmlib.c:82-107`): the update semaphore is held, `gl_mntree` is parked
/// (no bar track can start while the pop-up is up — `mchange`'s hand-over
/// and `ctlmgr`'s `MU_M1` both gate on it), the control rectangle is the
/// whole screen (`ct_chgown(rlr, &gl_rscreen)`, so a press anywhere is the
/// caller's, not the bar row's `ctl_pd`), and `END_MCTRL` restores all of
/// it. That bracket is [`window::fm_own`](super::window::fm_own), the same
/// one `wind_update(BEG_MCTRL)` and `form_do` drive (A88).
///
/// The output `MENU` gets `mn_keystate` = the shift state at the selecting
/// press on a pick (`handle_popup`'s `keystate = rets[3]`, `:272`, copied at
/// `:342`), and `0` on no pick (`:347`).
pub fn menu_popup(pb: &Aespb) {
    // `MENU` bases, read at the caller's pointer width (0 = null); the struct
    // fields are decoded at the caller ABI's offsets, not the native layout.
    let menu = pb.addrin(0);
    let (xpos, ypos) = (pb.intin(0), pb.intin(1));
    let mdata = pb.addrin(1);
    if menu == 0 {
        pb.set_ret(0);
        return;
    }
    let Some(h) = host() else {
        pb.set_ret(0);
        return;
    };
    let Some(backend) = h.raster() else {
        pb.set_ret(0);
        return;
    };
    // The caller's `MENU` (pointer + four WORDs at its width), checked
    // readable with its rights and read through its kernel alias (ARCH03 3a);
    // one the program cannot read ends the call (and the program).
    let Some(menu) = pb.ubuf(menu, menu_ptr_size(pb.abi) + 8, false) else {
        return;
    };
    // SAFETY: valid MENU (GEM contract) at a checked kernel address, packed
    // at the caller ABI width.
    let m = unsafe { Menu::read_abi(menu, pb.abi) };
    let (tree, box_obj, start_item) = (m.mn_tree as *mut Object, m.mn_menu, m.mn_item);
    if tree.is_null() || box_obj < 0 {
        pb.set_ret(0);
        return;
    }
    let (sel, keystate) = mn_popup(tree, box_obj, start_item, xpos, ypos, backend, h);
    // The output `MENU`, checked writable with the caller's rights and
    // written through its kernel alias (ARCH03 3a); one the program may not
    // write ends the call (and the program) after the pop-up is gone.
    let Some(mdata) = pb.ubuf(mdata, menu_ptr_size(pb.abi) + 8, true) else {
        return;
    };
    if mdata != 0 {
        if sel >= 0 {
            // A pick (`:338-343`): `mn_tree`/`mn_menu` as input (no submenus
            // from a pop-up), `mn_item` the choice, `mn_scroll` copied from
            // the input, `mn_keystate` the shift state at the press.
            let out = Menu {
                mn_tree: tree as u64,
                mn_menu: box_obj,
                mn_item: sel,
                mn_scroll: m.mn_scroll,
                mn_keystate: keystate,
            };
            // SAFETY: valid MENU out (GEM contract), packed at the caller ABI width.
            unsafe { out.write_abi(mdata, pb.abi) };
        } else {
            // No pick: `mdata->mn_keystate = 0; /* TOS always sets mn_keystate */`
            // (`:347`) — the one field written on that branch; `mn_tree`,
            // `mn_menu`, `mn_item` and `mn_scroll` keep whatever the caller
            // had there (A102).
            // SAFETY: valid MENU out (GEM contract), packed at the caller ABI width.
            unsafe { Menu::write_keystate_abi(mdata, pb.abi, 0) };
        }
    }
    pb.set_ret(if sel >= 0 { 1 } else { 0 });
}

/// `clamp_ypos` (`gemmnext.c:114-127`): move the pop-up box, in `gl_hchar`
/// steps, until it neither runs off the bottom of the screen
/// (`y_limit = gl_rscreen.g_h - ob_height`) nor reaches into the menu-bar
/// row (`y_limit = gl_rfull.g_y` = `gl_hbox`, `gemgraf.c:424`) — the second
/// rule wins when both cannot hold. `x` is not touched (`ob_x = xpos`,
/// `:322`); the save rectangle clamps itself to the screen ([`sr_rect`]).
fn clamp_ypos(t: ObjTree, obj: i16, backend: &dyn RasterBackend) {
    let hchar = super::graf::gl_hchar();
    let mut y = t.y(obj);
    // "make sure that object doesn't extend past the bottom of the screen"
    let y_limit = backend.height() as i16 - t.height(obj);
    while y > y_limit {
        y -= hchar;
    }
    // "even more importantly, it mustn't extend into the menu area"
    let y_limit = gl_hbox();
    while y < y_limit {
        y += hchar;
    }
    t.set_y(obj, y);
}

/// EmuTOS `mn_popup` (`gemmnext.c:304-356`) with the parameter block already
/// decoded: take the mouse (`wm_update(BEG_MCTRL)`, `:312`), wait for the
/// button that opened the pop-up to come up (`ev_button(1, 0x00ff, 0x0000,
/// rets)`, `:314`), position the parent box so `start_item` sits at `(xpos,
/// ypos)` and clamp it ([`clamp_ypos`], `:320-324`), save + show + track +
/// restore ([`popup_shown`], `:326-334`), wait for the button that picked or
/// cancelled to come up as well (`:351`), give the mouse back
/// (`wm_update(END_MCTRL)`, `:353`). The call returns with the button
/// **up**, so the selecting press never reaches the caller's next `evnt_*`
/// wait (`docs/api/aes/menu.md` "Contract addition — … it consumes the
/// selecting press", A73 point 3). Returns `(item, shift state at the
/// pick)`, `(NIL, 0)` on no pick.
///
/// The screen under the box goes to a fresh pool block for this one opening
/// (`popup_blit`, `gemmnext.c:329-333`), not the drop-down's screen buffer;
/// no block, no pop-up — `NIL` without a track (D-086,
/// `docs/api/aes/menu.md`).
///
/// `h`/`backend` are passed in rather than looked up so the unit tests can
/// drive this body with a scripted host and no process-wide installation.
fn mn_popup(
    tree: *mut Object,
    box_obj: i16,
    start_item: i16,
    xpos: i16,
    ypos: i16,
    backend: &dyn RasterBackend,
    h: &dyn crate::host::Host,
) -> (i16, i16) {
    // `wm_update(BEG_MCTRL); /* take over the mouse */` (`gemmnext.c:312`)
    // = `fm_own(TRUE)` (`gemwmlib.c:1778-1797`, `gemfmlib.c:82-96`):
    // update semaphore, `gl_mntree` parked, control rectangle = the whole
    // screen — [`window::fm_own`](super::window::fm_own), the bracket
    // `wind_update(BEG_MCTRL)` and `form_do` use. `END_MCTRL` below undoes
    // it on every way out.
    super::window::fm_own(true);
    let r = mn_popup_owned(tree, box_obj, start_item, xpos, ypos, backend, h);
    // `wm_update(END_MCTRL); /* give back the mouse */` (`gemmnext.c:353`).
    super::window::fm_own(false);
    r
}

/// The body of [`mn_popup`] between its two `wm_update` calls
/// (`gemmnext.c:314-351`).
fn mn_popup_owned(
    tree: *mut Object,
    box_obj: i16,
    start_item: i16,
    xpos: i16,
    ypos: i16,
    backend: &dyn RasterBackend,
    h: &dyn crate::host::Host,
) -> (i16, i16) {
    // `ev_button(1, 0x00ff, 0x0000, rets); /* wait for button up */`
    // (`gemmnext.c:314`): the button that opened the pop-up.
    wait_button_up(h);
    // "xpos/ypos are the location of the *start* item, not the parent box,
    // so to get the y position of the parent, we first need to subtract the
    // y offset of the start item" (`:316-324`). EmuTOS indexes
    // `tree[menu->mn_item]` unguarded; a negative start item is a caller
    // bug NewTOS reads as offset 0 instead of faulting.
    let t = ObjTree::new(tree);
    let iy = if start_item >= 0 { t.y(start_item) } else { 0 };
    t.set_x(box_obj, xpos);
    t.set_y(box_obj, ypos - iy);
    clamp_ypos(t, box_obj, backend);
    crate::vdi::cursor_hide(backend);
    let (bx, by, bw, bh) = sr_rect(t, box_obj, backend);
    let Some(saved) = gsxif::popup_blit_save(backend, bx, by, bw, bh) else {
        // No block, no pop-up (`buf == NULL`, `:330`). The second wait
        // (`:351`) has nothing left to wait for here: the first one just
        // saw the button up and nothing was pumped since.
        crate::vdi::cursor_show(backend);
        return (NIL, 0);
    };
    popup_shown(tree, box_obj, backend, h, saved)
}

/// `ev_button(1, 0x00ff, 0x0000, rets)` — block until the left button is
/// up, pumping input meanwhile. `mn_popup` runs it twice (`gemmnext.c:314,
/// 351`); [`mn_popup_owned`] and [`popup_shown`] do the same.
fn wait_button_up(h: &dyn crate::host::Host) {
    while crate::vdi::mouse_buttons() & 0x01 != 0 {
        h.pump_input();
    }
    // `ev_button` ends in `ev_rets` (`aes/gemevlib.c:91`), which consumes the
    // release.
    super::input::ev_consume();
}

/// The pop-up on screen: draw the box over `saved` (the pixels under it,
/// already taken with the cursor hidden), track until a button press
/// (`handle_popup`, `gemmnext.c:229-298`), put the pixels back
/// (`popup_blit(obj, buf)`, `:333`), then wait for that press to be released
/// (`ev_button(1, 0x00ff, 0x0000, rets)`, `:351`). Returns `(item, keystate)`:
/// the picked item or `NIL`, and the keyboard shift state at the press —
/// `handle_popup`'s `keystate = rets[3]` after the `ev_multi` that returned
/// the button (`:268-272`, `ev_rets`'s `rets[3] = kstate`, `gemevlib.c:56`),
/// read here from `h` through [`gsxif::kstate_of`], as `graf_mkstate`/
/// `ev_rets` read it (bits 0-3 only, A126).
fn popup_shown(
    tree: *mut Object,
    box_obj: i16,
    backend: &dyn RasterBackend,
    h: &dyn crate::host::Host,
    saved: gsxif::Saved,
) -> (i16, i16) {
    let tp = tree as *const Object;
    let (bx, by, bw, bh) = saved.rect();
    object::draw_obj_clip(tp, box_obj, MAX_DEPTH, sr_clip(bx, by, bw, bh), backend);
    crate::vdi::cursor_show(backend);

    let mut cur_item = NIL;
    let result;
    let keystate;
    loop {
        let (mx, my) = crate::vdi::mouse_position();
        let it = object::child_at(tp, box_obj, mx, my);
        if it != cur_item {
            crate::vdi::cursor_hide(backend);
            // `ob_change`'s XOR toggle each way (`gemoblib.c:1295-1304`, A99):
            // no white pre-fill before the un-hilite, and no frame repaint
            // after it — `handle_popup` only toggles through `do_chg`
            // (`gemmnext.c:278-287`); see [`rehilite`] (A100).
            if cur_item != NIL {
                object::set_state_bit(tree, cur_item, SELECTED, false);
            }
            if it != NIL {
                object::set_state_bit(tree, it, SELECTED, true);
            }
            crate::vdi::cursor_show(backend);
            cur_item = it;
        }
        if crate::vdi::mouse_buttons() & 0x01 != 0 {
            // `handle_popup`'s `ev_multi(MU_M1|MU_BUTTON, …)` returned the
            // press (`aes/gemmnext.c:268`); its `ev_rets` consumed it.
            super::input::ev_consume();
            result = cur_item; // NIL when pressed outside any item → cancel
            // `keystate = rets[3]` of the `ev_multi` that returned `MU_BUTTON`
            // (`gemmnext.c:272`): the shift state at the press, not at the
            // release the wait below consumes.
            keystate = gsxif::kstate_of(h);
            break;
        }
        h.pump_input();
    }
    crate::vdi::cursor_hide(backend);
    if result != NIL {
        object::set_state_bit(tree, result, SELECTED, false);
    }
    // `popup_blit(obj, buf)`: restore and release the block (`:333`).
    saved.restore(backend);
    crate::vdi::cursor_show(backend);
    // `ev_button(1, 0x00ff, 0x0000, rets); /* wait for button up */`
    // (`gemmnext.c:351`), after the pick/no-pick branch and before
    // `wm_update(END_MCTRL)` (`:353`): the press that picked or cancelled —
    // `handle_popup` returns on the press itself (`:290-291`) — and its
    // release are both consumed inside the call.
    wait_button_up(h);
    (result, keystate)
}

/// Native `MN_SET` behaviour block (later-AES).
#[repr(C)]
#[derive(Clone, Copy)]
struct MnSet {
    display: i32,
    drag: i32,
    delay: i32,
    speed: i32,
    height: i16,
}

/// EmuTOS `MIN_HEIGHT_VALUE` (`gemmnext.c:76`) — the smallest legal scroll
/// height, in items.
const MIN_HEIGHT_VALUE: i16 = 5;

/// Clamp `menu_settings`'s `height` field to EmuTOS's
/// `[MIN_HEIGHT_VALUE, gl_rfull.g_h / gl_hchar]` range (`clamp_height`,
/// `gemmnext.c:83-95`): at least 5 items, at most as many system-font rows as
/// fit in the desktop area **below the menu bar** — `gl_rfull = (0, gl_hbox,
/// gl_width, gl_height - gl_hbox)` (`gemgraf.c:424`), so `gl_rfull.g_h` is the
/// screen height less the bar row (A102). `gl_hchar` is the system-font cell
/// height ([`super::graf::gl_hchar`]). When no raster is up (workstation not
/// yet initialised) only the lower bound is enforceable — the upper clamp is
/// skipped rather than guessed.
fn clamp_height(height: i16) -> i16 {
    if height < MIN_HEIGHT_VALUE {
        return MIN_HEIGHT_VALUE;
    }
    if let Some(backend) = host().and_then(|h| h.raster()) {
        let rfull_h = backend.height() as i16 - gl_hbox();
        let max_height = rfull_h / super::graf::gl_hchar();
        if height > max_height {
            return max_height;
        }
    }
    height
}

struct Settings(UnsafeCell<MnSet>);
// SAFETY: single boot CPU only touches the AES.
unsafe impl Sync for Settings {}

/// The power-up `settings` block — EmuTOS `mnext_init` (`gemmnext.c:103-110`)
/// with the `INIT_*_VALUE` constants (`:71-75`): submenu display delay 200 ms,
/// submenu drag delay 10000 ms, single-click scroll delay 250 ms, continuous
/// scroll delay 0 ms, scroll height 16 items. `mnext_init` runs `height`
/// through `clamp_height` because it is called after the workstation is up;
/// a static initialiser cannot, but the upper bound (`gl_rfull.g_h /
/// gl_hchar`) only bites when fewer than 16 system-font rows fit under the
/// menu bar — a screen of under ~275 rows at the 16-pixel font — which no
/// NewTOS profile has, so 16 is what `clamp_height(16)` returns on every one
/// of them.
const MNSET_INIT: MnSet = MnSet {
    display: 200,
    drag: 10000,
    delay: 250,
    speed: 0,
    height: 16,
};

static SETTINGS: Settings = Settings(UnsafeCell::new(MNSET_INIT));

/// `mnext_init` (`gemmnext.c:103-110`) — put the `settings` block back to the
/// `INIT_*_VALUE` constants. Called once per AES session
/// (`run_accs_and_desktop`, `geminit.c:671-672`), so a restart (A106) undoes
/// whatever the previous session's `menu_settings(1, …)` left behind. Unlike
/// the static initialiser this can run `height` through [`clamp_height`], as
/// EmuTOS does.
fn mnext_init() {
    // SAFETY: single-CPU restart path, no AES call in flight.
    unsafe {
        let s = &mut *SETTINGS.0.get();
        *s = MNSET_INIT;
        s.height = clamp_height(MNSET_INIT.height);
    }
}

/// `menu_settings` (39) — get (`int_in[0]`=0) or set (1) the menu-behaviour
/// block at `addr_in[0]` (`MN_SET`). Negative fields are left unchanged on
/// set (like TOS). EmuTOS `mn_settings` (`gemmnext.c:596-616`) is a
/// `switch(flag)` with the arms 0 and 1 and **no `default`**: any other
/// selector is a no-op that neither reads nor writes either block (A43).
pub fn menu_settings(pb: &Aespb) {
    let set = pb.addrin(0);
    let flag = pb.intin(0);
    if set == 0 {
        pb.set_ret(0);
        return;
    }
    // The caller's `MN_SET`, checked with its rights — writable for a get
    // (0), readable for a set (1) — and reached through its kernel alias
    // (ARCH03 3a); one the program may not use ends the call (and the
    // program). Any other selector touches neither block (A43), so nothing
    // is checked for it.
    let set = if matches!(flag, 0 | 1) {
        match pb.ubuf(set, core::mem::size_of::<MnSet>() as u64, flag == 0) {
            Some(k) => k as *mut MnSet,
            None => return,
        }
    } else {
        set as *mut MnSet
    };
    // SAFETY: single-CPU AES; valid MN_SET (GEM contract).
    unsafe {
        let cur = &mut *SETTINGS.0.get();
        match flag {
            0 => *set = *cur,
            1 => {
                let s = *set;
                if s.display >= 0 {
                    cur.display = s.display;
                }
                if s.drag >= 0 {
                    cur.drag = s.drag;
                }
                if s.delay >= 0 {
                    cur.delay = s.delay;
                }
                if s.speed >= 0 {
                    cur.speed = s.speed;
                }
                if s.height >= 0 {
                    cur.height = clamp_height(s.height);
                }
            }
            _ => {} // no such arm in mn_settings: nothing is read or written
        }
    }
    pb.set_ret(1);
}

// ── submenu (SMIB) registry — menu_attach / menu_istart storage ──────────
//
// EmuTOS keeps a lazily-allocated `SMIB gl_submenu[NUM_SMIBS]` (`struct.h:53,
// 180-186`) holding `(s_usage, s_tree, s_menu, s_start)`, and recovers the
// parent item from the item's `ob_type` upper byte (D-087). NewTOS uses a
// **fixed static table** (no runtime growth, like the DA / settings tables)
// and stores the parent `(tree, item)` explicitly, so the `ME_INQUIRE` /
// `ME_REMOVE` lookup and the nested-dropdown lookup are direct table scans —
// while **still** writing the EmuTOS `ob_type` index + `SUBMENU` flag + arrow
// marker (D-087, contract), so a program that reads `ob_type` after
// `menu_attach` sees the same value plain TOS wrote.

/// SMIB capacity — matches EmuTOS `NUM_SMIBS` (`struct.h:53`), so the
/// "table full → FALSE" path (`gemmnext.c:509-512`) triggers at the same count.
const NUM_SMIBS: usize = 128;

// `menu_attach` operation flags (`aesdefs.h:297-299`).
const ME_INQUIRE: i16 = 0;
const ME_ATTACH: i16 = 1;
const ME_REMOVE: i16 = 2;

/// The right-arrow marker char written into an attached item's text
/// (`gemmnext.c:62`).
const SUBMENU_MARKER: u8 = 0x03;
/// Offset of the marker char from the end of the item string
/// (`gemmnext.c:63`).
const SUBMENU_MARKER_OFFSET: usize = 2;

/// One submenu attachment — EmuTOS `SMIB` (`struct.h:180-186`) extended with
/// the parent `(tree, item)` linkage NewTOS keys on (EmuTOS instead encodes
/// the array index into the parent item's `ob_type`).
#[derive(Clone, Copy)]
struct Smib {
    /// EmuTOS `s_usage != 0` — this slot is in use.
    used: bool,
    /// `OBJECT*` of the menu-bar tree the parent item lives in.
    parent_tree: u64,
    /// the `G_STRING` bar item the submenu hangs off.
    parent_item: i16,
    /// `MENU.mn_tree` — the submenu's own OBJECT tree (EmuTOS `s_tree`).
    sub_tree: u64,
    /// `MENU.mn_menu` — the submenu's enclosing box object (EmuTOS `s_menu`).
    sub_menu: i16,
    /// `MENU.mn_item` — the (scroll) start item (EmuTOS `s_start`).
    sub_start: i16,
    /// `MENU.mn_scroll` — retained; the scroll flag is ignored (restriction #2).
    sub_scroll: i16,
}
impl Smib {
    const EMPTY: Smib = Smib {
        used: false,
        parent_tree: 0,
        parent_item: NIL,
        sub_tree: 0,
        sub_menu: NIL,
        sub_start: NIL,
        sub_scroll: 0,
    };
}

/// The submenu registry (EmuTOS `gl_submenu[]`). Fixed-size static, touched
/// only on the single-CPU AES path.
struct SubmenuTable(UnsafeCell<[Smib; NUM_SMIBS]>);
// SAFETY: single boot CPU only touches the AES.
unsafe impl Sync for SubmenuTable {}
static SUBMENU_TAB: SubmenuTable = SubmenuTable(UnsafeCell::new([Smib::EMPTY; NUM_SMIBS]));

/// Slot of the used entry attached to parent `(tree, item)`, or `None`.
fn smib_find_parent(tree: u64, item: i16) -> Option<usize> {
    // SAFETY: single-CPU AES; exclusive shared read of the table.
    let tab = unsafe { &*SUBMENU_TAB.0.get() };
    tab.iter()
        .position(|s| s.used && s.parent_tree == tree && s.parent_item == item)
}

/// Slot of the used entry whose submenu is `(sub_tree, sub_menu)`, or `None`
/// (EmuTOS `find_submenu_from_tree`, used by `menu_istart`).
fn smib_find_sub(sub_tree: u64, sub_menu: i16) -> Option<usize> {
    // SAFETY: single-CPU AES; exclusive shared read of the table.
    let tab = unsafe { &*SUBMENU_TAB.0.get() };
    tab.iter()
        .position(|s| s.used && s.sub_tree == sub_tree && s.sub_menu == sub_menu)
}

/// First free slot, or `None` when the table is full (EmuTOS "SMIB table full").
fn smib_free() -> Option<usize> {
    // SAFETY: single-CPU AES; exclusive shared read of the table.
    let tab = unsafe { &*SUBMENU_TAB.0.get() };
    tab.iter().position(|s| !s.used)
}

/// Encode the submenu link into the parent item (EmuTOS `attach_submenu` tail,
/// `gemmnext.c:522-527`; D-087): `ob_type` upper byte = `slot + 128`, set the
/// `SUBMENU` `ob_flags` bit, and write the right-arrow marker two chars from
/// the end of the item text.
///
/// # Safety
/// `tree`/`item` are a valid menu-bar tree + `G_STRING` item index.
unsafe fn submenu_mark(tree: *mut Object, item: i16, slot: usize) {
    let t = ObjTree::new(tree);
    t.set_type(item, (t.ob_type(item) & 0x00ff) | (((slot as u16) + 128) << 8));
    t.set_flags(item, t.flags(item) | SUBMENU);
    // SAFETY: forwarded from the caller's contract — the item's ob_spec points
    // at a writable NUL-terminated string.
    unsafe { write_marker(t.spec(item) as *mut u8, SUBMENU_MARKER) };
}

/// Reverse [`submenu_mark`] (EmuTOS `detach_submenu`, `gemmnext.c:449-452`):
/// clear the `ob_type` upper byte, clear `SUBMENU`, blank the marker to a space.
///
/// # Safety
/// `tree`/`item` are a valid menu-bar tree + item index.
unsafe fn submenu_unmark(tree: *mut Object, item: i16) {
    let t = ObjTree::new(tree);
    t.set_type(item, t.ob_type(item) & 0x00ff);
    t.set_flags(item, t.flags(item) & !SUBMENU);
    // SAFETY: forwarded from the caller's contract.
    unsafe { write_marker(t.spec(item) as *mut u8, b' ') };
}

/// Write `c` at `strlen - SUBMENU_MARKER_OFFSET` of the NUL-terminated string
/// at `p` (EmuTOS's marker slot). No-op if the pointer is null or the string is
/// shorter than the offset (NewTOS bound over EmuTOS's unguarded write).
///
/// # Safety
/// `p` is null or points at a writable NUL-terminated buffer.
unsafe fn write_marker(p: *mut u8, c: u8) {
    if p.is_null() {
        return;
    }
    // SAFETY: caller guarantees a valid NUL-terminated buffer.
    unsafe {
        let mut len = 0usize;
        while len < 256 && *p.add(len) != 0 {
            len += 1;
        }
        if len >= SUBMENU_MARKER_OFFSET {
            *p.add(len - SUBMENU_MARKER_OFFSET) = c;
        }
    }
}

/// EmuTOS `query_submenu` (`gemmnext.c:401-433`): copy the submenu descriptor
/// attached to parent `(tree, item)` into `*mdata` (tolerating a null `mdata`,
/// the EmuTOS convenience), returning whether one is attached.
fn query_submenu(tree: *mut Object, item: i16, mdata: u64, abi: Abi) -> bool {
    let Some(slot) = smib_find_parent(tree as u64, item) else {
        return false;
    };
    if mdata != 0 {
        // SAFETY: single-CPU AES read; valid MENU out (GEM contract).
        let s = unsafe { (*SUBMENU_TAB.0.get())[slot] };
        let out = Menu {
            mn_tree: s.sub_tree,
            mn_menu: s.sub_menu,
            mn_item: s.sub_start,
            // EmuTOS zeroes mn_scroll here (the scroll flag is ignored,
            // restriction #2); NewTOS round-trips the value stored at attach so
            // ME_INQUIRE reports the descriptor as given. The tracker still
            // ignores the flag, so this changes no rendering/selection — only
            // what INQUIRE reflects back.
            mn_scroll: s.sub_scroll,
            mn_keystate: 0,
        };
        // SAFETY: valid MENU out (GEM contract), packed at the caller ABI width.
        unsafe { out.write_abi(mdata, abi) };
    }
    true
}

/// EmuTOS `attach_submenu` (`gemmnext.c:465-530`): attach the submenu `*mdata`
/// to parent `(tree, item)`, replacing any existing attachment first; a null
/// `mdata` is the `ME_REMOVE` path. Returns `true` on success, `false` when
/// there is nothing to remove, no descriptor to attach, or the table is full.
fn attach_submenu(tree: *mut Object, item: i16, mdata: u64, abi: Abi) -> bool {
    // If the item already has a submenu, detach it first (replace semantics,
    // gemmnext.c:471-477).
    if let Some(old) = smib_find_parent(tree as u64, item) {
        // SAFETY: single-CPU AES; exclusive mutable access to one slot.
        unsafe { (*SUBMENU_TAB.0.get())[old].used = false };
        // SAFETY: valid tree (GEM contract).
        unsafe { submenu_unmark(tree, item) };
        if mdata == 0 {
            return true; // ME_REMOVE on an attached item: done.
        }
    }
    // ME_REMOVE on an item with no submenu (gemmnext.c:480-481).
    if mdata == 0 {
        return false;
    }
    // Fixed-size table: no free slot → EmuTOS "table full" FALSE.
    let Some(slot) = smib_free() else {
        return false;
    };
    // SAFETY: valid MENU in (GEM contract), packed at the caller ABI width.
    let m = unsafe { Menu::read_abi(mdata, abi) };
    // SAFETY: single-CPU AES; exclusive mutable access to one slot.
    unsafe {
        let s = &mut (*SUBMENU_TAB.0.get())[slot];
        s.used = true;
        s.parent_tree = tree as u64;
        s.parent_item = item;
        s.sub_tree = m.mn_tree;
        s.sub_menu = m.mn_menu;
        s.sub_start = m.mn_item;
        s.sub_scroll = m.mn_scroll;
    }
    // Encode the D-087 ob_type index + SUBMENU flag + marker into the item.
    // SAFETY: valid tree (GEM contract).
    unsafe { submenu_mark(tree, item, slot) };
    true
}

/// `menu_attach` (37) — attach (`ME_ATTACH`=1) / remove (`ME_REMOVE`=2) /
/// query (`ME_INQUIRE`=0) a submenu on a menu-bar item (EmuTOS `mn_attach`,
/// `gemmnext.c:535-557`). Dispatch: `mn_attach(MPOP_FLAG, MPOP_IN, MPOP_ITEM,
/// MPOP_OUT)` = `(int_in[0], addr_in[0], int_in[1], addr_in[1])`.
///
/// Guard: the target item must be a `G_STRING` — applied before the operation
/// switch, for all three flags (`gemmnext.c:539-541`) → else `0`. Then, per
/// EmuTOS `mn_attach`:
///  - `ME_INQUIRE`: `1` and `*MPOP_OUT` filled iff a submenu is attached;
///  - `ME_ATTACH`: record the `MENU` at `MPOP_OUT` for `(tree, item)`
///    (replacing any existing one) and return `1`, or `0` if the SMIB table is
///    full;
///  - `ME_REMOVE`: clear the attachment and return `1`, or `0` if the item had
///    no submenu (D-088);
///  - any other flag: `0` (EmuTOS switch default leaves `rc = FALSE`).
///
/// A successful `ME_ATTACH` also sets the item's `SUBMENU` `ob_flags` bit and
/// writes the SMIB index into its `ob_type` upper byte; `ME_REMOVE` reverses
/// both (D-087, contract).
pub fn menu_attach(pb: &Aespb) {
    let flag = pb.intin(0);
    let tree = pb.addrin(0) as *mut Object;
    let item = pb.intin(1);
    // `MENU` base, read at the caller's pointer width (0 = null); decoded at the
    // caller ABI's field offsets by [`query_submenu`]/[`attach_submenu`].
    let mdata = pb.addrin(1);
    if tree.is_null() || item < 0 {
        pb.set_ret(0);
        return;
    }
    // A submenu can only be attached to a G_STRING item — the guard EmuTOS
    // applies before the ME_* switch, for all three operations.
    let ob_type = ObjTree::new(tree).ob_type(item);
    if (ob_type & 0x00ff) != G_STRING {
        pb.set_ret(0);
        return;
    }
    // The caller's `MENU` (pointer + four WORDs at its width): written by
    // `ME_INQUIRE`, read by `ME_ATTACH` — checked with the caller's rights for
    // that access and handed on as its kernel alias (ARCH03 3a; null stays
    // null). One the program may not use ends the call (and the program).
    let msize = menu_ptr_size(pb.abi) + 8;
    let rc = match flag {
        ME_INQUIRE => {
            let Some(mdata) = pb.ubuf(mdata, msize, true) else { return };
            query_submenu(tree, item, mdata, pb.abi)
        }
        ME_ATTACH => {
            let Some(mdata) = pb.ubuf(mdata, msize, false) else { return };
            attach_submenu(tree, item, mdata, pb.abi)
        }
        ME_REMOVE => attach_submenu(tree, item, 0, pb.abi),
        _ => false, // undefined flag → FALSE (EmuTOS switch default)
    };
    pb.set_ret(if rc { 1 } else { 0 });
}

/// `menu_istart` (38) — get/set the starting item of a scrollable submenu
/// (EmuTOS `mn_istart`, `gemmnext.c:562-587`). Dispatch: `mn_istart(MPOP_FLAG,
/// MPOP_IN, MPOP_ITEM, MPOP_ITEM2)` = `(int_in[0], addr_in[0], int_in[1],
/// int_in[2])`.
///
/// `MPOP_IN` (`addr_in[0]`) is the tree **containing the submenu**, `MPOP_ITEM`
/// (`int_in[1]`) its box object. Looks the submenu up by `(sub_tree, sub_menu)`
/// (EmuTOS `find_submenu_from_tree`) and returns `FALSE` (0) when none is
/// attached — before the flag switch. `flag=0` inquires (returns the current
/// `s_start`); `flag=1` clamps `int_in[2]` to the box's `[ob_head, ob_tail]`
/// child range, stores it as `s_start`, and returns it; any other flag → `0`.
pub fn menu_istart(pb: &Aespb) {
    let flag = pb.intin(0);
    let tree = pb.addrin(0) as *const Object;
    let menu = pb.intin(1); // the submenu box object
    let start = pb.intin(2); // desired start item (update only)
    if tree.is_null() || menu < 0 {
        pb.set_ret(0);
        return;
    }
    // Look the submenu up by (submenu tree, submenu box); no such submenu →
    // FALSE, before the flag switch (gemmnext.c:567-569).
    let Some(slot) = smib_find_sub(tree as u64, menu) else {
        pb.set_ret(0);
        return;
    };
    // SAFETY: single-CPU AES; exclusive mutable access to one slot.
    let s = unsafe { &mut (*SUBMENU_TAB.0.get())[slot] };
    match flag {
        0 => {} // inquire: return the current start item
        1 => {
            // Clamp 'start' to the submenu box's child index range (the EmuTOS
            // two-comparison form, which tolerates head > tail without the
            // ordering panic `clamp` would raise on an empty box).
            let bt = ObjTree::new(tree);
            let (head, tail) = (bt.head(menu), bt.tail(menu));
            s.sub_start = if start < head {
                head
            } else if start > tail {
                tail
            } else {
                start
            };
        }
        _ => {
            pb.set_ret(0); // bad flag → FALSE (gemmnext.c:584-585)
            return;
        }
    }
    pb.set_ret(s.sub_start);
}

// ── headless save/restore regression scaffold (Group-1 P1+P2) ─────────────

/// Test-only reproduction of the QED menu save/restore defect (P1 + P2),
/// runnable headlessly for a QMP screendump. It builds the geometry parsed from
/// QED's `qed.rsc`: drop-down boxes with an **outward** 1-px border (`th = -1`),
/// overlapping in X, of **unequal height** — plus one box **wider than the old
/// 256-px save cap**. It then:
///
///   * opens the wide box and closes it — exercising the P2 buffer cap: the old
///     code marked the snapshot invalid for a box wider than 256 / taller than
///     400 and **white-erased** the interior on close instead of restoring it;
///   * drops box A, then switches to box B exactly as the interactive tracker
///     does on a title slide (`close_box(A)` → `open_box(B)`) — under the old
///     code this left box A's outward-border / outside-`abs_rect` pixels on
///     screen, because `save_under`/`restore_under` covered only `abs_rect(box)`
///     with no margin (the P1 stale residue).
///
/// Returns `(p1, p2)`: the count of stale pixels left outside box B for the
/// slide (P1), and inside the wide box for the cap case (P2). Both are nonzero
/// before the fix and `0` after. The final screen — box B open, everything else
/// the plain background — is what a screendump captures. **Not wired to boot**;
/// a temporary kernel hook drives it during verification.
pub fn selftest_menu_sr_residue(backend: &dyn RasterBackend) -> (u32, u32) {
    use tos_types::aes::{G_BOX, G_IBOX};
    /// A distinctive, non-white/non-black background index, so a black
    /// outward-border residue (1) and a white-erased interior (0) both differ
    /// from it.
    const BG: Pel = 8;
    /// `ob_spec` for a box with an outward 1-px border (`th = -1`, byte 0xFF in
    /// bits 16..23) over a solid white interior with a black border/text colour
    /// word (`0x1170`: border BLACK, text BLACK, IP_SOLID, inside WHITE).
    const SPEC_TH_NEG1: u64 = 0x00FF_1170;

    // Object indices in the tree below.
    const TITLE_A: i16 = 3;
    const TITLE_B: i16 = 4;
    const TITLE_C: i16 = 5;

    let sw = backend.width() as i16;
    let sh = backend.height() as i16;
    let clip = ClipRect::full(backend.width(), backend.height());
    // Lay down the distinctive background across the whole screen.
    backend.fill_rect(0, 0, sw, sh, BG, clip);

    static NA: &[u8] = b"A\0";
    static NB: &[u8] = b"B\0";
    static NC: &[u8] = b"C\0";
    static A0: &[u8] = b"A item 0\0";
    static A1: &[u8] = b"A item 1\0";
    static B0: &[u8] = b"B item 0\0";
    static B1: &[u8] = b"B item 1\0";
    static C0: &[u8] = b"Wide item 0\0";
    static C1: &[u8] = b"Wide item 1\0";

    let mk = |ty, spec: u64, x, y, w, h, next, head, tail| Object {
        ob_next: next, ob_head: head, ob_tail: tail, ob_type: ty, ob_flags: 0,
        ob_state: 0, ob_spec: spec, ob_x: x, ob_y: y, ob_width: w, ob_height: h,
    };
    let bar_h = gl_hbox();
    let title = |p: &[u8], x, next| mk(G_TITLE, p.as_ptr() as u64, x, 0, 64, bar_h, next, -1, -1);
    let item = |p: &[u8], y, w, next| mk(G_STRING, p.as_ptr() as u64, 0, y, w, bar_h, next, -1, -1);

    // Box A: 200×340 (under the old cap) — the box we slide *away* from.
    // Box B: 170×130, overlapping A in X, much shorter — the box we slide *to*.
    // Box C: 280 wide (> the old 256 cap), isolated — the cap/white-erase case.
    let tree = [
        /* 0 THESCREEN */ mk(G_IBOX, 0, 0, 0, sw, sh, -1, 1, 6),
        /* 1 THEBAR    */ mk(G_BOX, 0, 0, 0, sw, bar_h, 6, 2, 2),
        /* 2 THEACTIVE */ mk(G_IBOX, 0, 0, 0, sw, bar_h, 1, 3, 5),
        /* 3 titleA    */ title(NA, 100, 4),
        /* 4 titleB    */ title(NB, 40, 5),
        /* 5 titleC    */ title(NC, 320, 2), // last title → next = THEACTIVE
        /* 6 themenus  */ mk(G_IBOX, 0, 0, 0, sw, sh, 0, 7, 13),
        /* 7 boxA      */ mk(G_BOX, SPEC_TH_NEG1, 100, 40, 200, 340, 10, 8, 9),
        /* 8  A0       */ item(A0, 0, 200, 9),
        /* 9  A1       */ item(A1, bar_h, 200, 7), // last child → next = boxA
        /*10 boxB      */ mk(G_BOX, SPEC_TH_NEG1, 40, 40, 170, 130, 13, 11, 12),
        /*11  B0       */ item(B0, 0, 170, 12),
        /*12  B1       */ item(B1, bar_h, 170, 10), // last child → next = boxB
        /*13 boxC      */ mk(G_BOX, SPEC_TH_NEG1, 320, 250, 280, 180, 6, 14, 15),
        /*14  C0       */ item(C0, 0, 280, 15),
        /*15  C1       */ item(C1, bar_h, 280, 13), // last child → next = boxC
    ];
    let tp = tree.as_ptr() as *mut Object;

    // Count on-screen pixels in [x0,x1)×[y0,y1) that differ from the background,
    // excluding the rectangle `ex` (box B's territory, legitimately redrawn).
    let count = |x0: i16, y0: i16, x1: i16, y1: i16, ex: (i16, i16, i16, i16)| -> u32 {
        let (ex0, ey0, ex1, ey1) = ex;
        let mut n = 0u32;
        let yy0 = y0.max(0);
        let yy1 = y1.min(sh);
        let xx0 = x0.max(0);
        let xx1 = x1.min(sw);
        let mut y = yy0;
        while y < yy1 {
            let mut x = xx0;
            while x < xx1 {
                let excluded = x >= ex0 && x < ex1 && y >= ey0 && y < ey1;
                if !excluded && backend.read_pixel(x, y) != BG {
                    n += 1;
                }
                x += 1;
            }
            y += 1;
        }
        n
    };

    // ── P2: open + close the wide box, then check its footprint is clean.
    let ic = open_box(ObjTree::new(tp), TITLE_C, backend);
    close_box(ObjTree::new(tp), TITLE_C, ic, backend);
    // boxC drawn extent = abs_rect grown 1 px for the th=-1 border: (319,249)..(601,431).
    let p2 = count(319, 249, 601, 431, (0, 0, 0, 0));

    // ── P1: drop box A, then slide to box B (close A → open B) and check that
    // no box-A pixel survives outside box B's territory.
    let ia = open_box(ObjTree::new(tp), TITLE_A, backend);
    close_box(ObjTree::new(tp), TITLE_A, ia, backend);
    open_box(ObjTree::new(tp), TITLE_B, backend); // leave box B open for the screendump
    // boxA drawn extent = abs_rect grown 1 px: (99,39)..(301,381).
    // Exclude box B's territory generously (abs_rect grown 6 px): (34,34)..(216,176).
    let p1 = count(99, 39, 301, 381, (34, 34, 216, 176));

    (p1, p2)
}

#[cfg(test)]
mod popup_wait_tests {
    //! `mn_popup` on the host, three neighbours of one call:
    //!
    //! * `popup_shown` returns only once the button that picked the item is
    //!   up again — `mn_popup`'s second `ev_button(1, 0x00ff, 0x0000, rets)`
    //!   (`aes/gemmnext.c:351`; A73 point 3, `docs/api/aes/menu.md` "Contract
    //!   addition — … it consumes the selecting press");
    //! * the `wm_update(BEG_MCTRL)` … `END_MCTRL` bracket (`:312,353` →
    //!   `fm_own`, `aes/gemfmlib.c:82-107`): `gl_mntree` parked and the mouse
    //!   kept by the caller for the whole call (A88 point 1);
    //! * `mn_keystate` = the shift state at the selecting press (`:272,342`,
    //!   A88 point 2) and `clamp_ypos` (`:114-127`, A88 point 3).
    //!
    //! The pop-up is driven on a raster stand-in that keeps no pixels and a
    //! host whose `pump_input` plays a scripted packet sequence (motion,
    //! button, shift state); the pixels under the box come from
    //! `gsxif::bb_save`'s heap fallback (a host process has neither the
    //! screen buffer nor the BDOS pool `popup_blit_save` needs — the full
    //! `menu_popup` call is the `#[ignore]`d case in
    //! `tests/aes_a73_event_neighbours.rs`), which is enough to hand
    //! `popup_shown` a `Saved` to put back; the paths that need the pool
    //! (`mn_popup`'s tracked half) run their pool-less D-086 branch here. Both
    //! the host and the raster are passed in — nothing is installed
    //! process-wide, so the hostless assumptions of the other unit tests in
    //! this binary stay intact. The mouse, menu and mouse-control statics are
    //! process-wide, so the cases run one at a time ([`SERIAL`]).
    use super::*;
    use crate::host::Host;
    use crate::vdi::backend::{MonoSrc, PatternFill, Rect, WriteMode};
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    /// The AES statics these cases drive are process-wide.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A 640×480 raster that keeps nothing: every draw is a no-op, every
    /// read answers 0. The test observes the button, not the pixels.
    struct NullRaster;

    impl RasterBackend for NullRaster {
        fn width(&self) -> u16 {
            640
        }
        fn height(&self) -> u16 {
            480
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
        fn set_active_palette(&self, _palette: &[u16; 256]) {}
        fn pel_to_device16(&self, _pel: Pel) -> u16 {
            0
        }
        fn device_to_rgb565(&self, _argb: u32) -> u16 {
            0
        }
        fn rgb565_to_device(&self, _px: u16) -> u32 {
            0
        }
        fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
            0
        }
        fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
        #[allow(clippy::too_many_arguments)]
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
        fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _fill: PatternFill, _clip: ClipRect) {}
        #[allow(clippy::too_many_arguments)]
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
        fn blit_mono(&self, _dst: Rect, _src: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _clip: ClipRect) {}
        fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
        fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
            0
        }
        fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
        fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
    }

    static RASTER: NullRaster = NullRaster;

    /// One scripted packet: motion, the VDI button byte, and the keyboard
    /// shift state the host reports from this packet on.
    #[derive(Copy, Clone)]
    struct Pkt {
        dx: i16,
        dy: i16,
        buttons: u8,
        shift: u8,
    }

    /// A button-only packet (no motion, no modifier).
    fn btn(b: u8) -> Pkt {
        Pkt { dx: 0, dy: 0, buttons: b, shift: 0 }
    }

    /// What the host saw at each pump: `gl_mntree == NULL`?
    /// (`super::active().is_null()`), and the pointer position after the packet.
    #[derive(Copy, Clone, Debug)]
    struct Seen {
        mntree_null: bool,
        at: (i16, i16),
    }

    /// Packets that `pump_input` feeds one per call, recording the AES state
    /// it finds on the way.
    struct ScriptHost {
        script: Mutex<VecDeque<Pkt>>,
        idle: AtomicUsize,
        shift: std::sync::atomic::AtomicU8,
        seen: Mutex<Vec<Seen>>,
    }

    /// Pumps on an exhausted script before the test fails instead of
    /// spinning forever inside the wait it is supposed to prove.
    const IDLE_CAP: usize = 1_000;

    impl ScriptHost {
        fn new() -> ScriptHost {
            ScriptHost {
                script: Mutex::new(VecDeque::new()),
                idle: AtomicUsize::new(0),
                shift: std::sync::atomic::AtomicU8::new(0),
                seen: Mutex::new(Vec::new()),
            }
        }
        fn arm(&self, packets: &[u8]) {
            self.arm_pkts(&packets.iter().map(|&b| btn(b)).collect::<Vec<_>>());
        }
        fn arm_pkts(&self, packets: &[Pkt]) {
            let mut s = self.script.lock().unwrap();
            s.clear();
            s.extend(packets.iter().copied());
            self.idle.store(0, Ordering::Relaxed);
            self.seen.lock().unwrap().clear();
        }
        fn left(&self) -> usize {
            self.script.lock().unwrap().len()
        }
        fn seen(&self) -> Vec<Seen> {
            self.seen.lock().unwrap().clone()
        }
    }

    impl Host for ScriptHost {
        fn console_write_byte(&self, _byte: u8) {}
        fn raster(&self) -> Option<&dyn RasterBackend> {
            Some(&RASTER)
        }
        fn keyboard_shift_state(&self) -> u8 {
            self.shift.load(Ordering::Relaxed)
        }
        fn pump_input(&self) {
            let next = self.script.lock().unwrap().pop_front();
            match next {
                Some(p) => {
                    self.shift.store(p.shift, Ordering::Relaxed);
                    crate::vdi::feed_relative(p.dx, p.dy, p.buttons, &RASTER);
                    self.seen.lock().unwrap().push(Seen {
                        mntree_null: super::active().is_null(),
                        at: crate::vdi::mouse_position(),
                    });
                }
                None => {
                    let n = self.idle.fetch_add(1, Ordering::Relaxed) + 1;
                    assert!(
                        n < IDLE_CAP,
                        "still waiting after {n} pumps on an exhausted script (buttons {:#04x})",
                        crate::vdi::mouse_buttons()
                    );
                }
            }
        }
    }

    /// Walk the pointer to `(tx, ty)` with the buttons up, outside any wait
    /// (each packet pumped directly).
    fn park_at(host: &ScriptHost, tx: i16, ty: i16) {
        host.arm(&[0]);
        host.pump_input(); // materialise the pointer
        for _ in 0..200 {
            let (cx, cy) = crate::vdi::mouse_position();
            if (cx, cy) == (tx, ty) {
                return;
            }
            host.arm_pkts(&[Pkt { dx: (tx - cx).clamp(-32, 32), dy: (ty - cy).clamp(-32, 32), buttons: 0, shift: 0 }]);
            host.pump_input();
        }
        assert_eq!(crate::vdi::mouse_position(), (tx, ty), "park_at did not reach its target");
    }

    use tos_types::aes::{G_BOX, G_STRING, LASTOB};
    const ITEM_H: i16 = 19;

    /// A three-item pop-up (`G_BOX` 112×57, `G_STRING` rows at y 0/19/38)
    /// whose second item sits under the pointer at `(px, py)`.
    fn popup_tree(px: i16, py: i16) -> *mut Object {
        static A: &[u8] = b"Alpha\0";
        static B: &[u8] = b"Beta\0";
        static C: &[u8] = b"Gamma\0";
        let s = |p: &[u8], y, next, last| Object {
            ob_next: next,
            ob_head: NIL,
            ob_tail: NIL,
            ob_type: G_STRING,
            ob_flags: if last { LASTOB } else { 0 },
            ob_state: 0,
            ob_spec: p.as_ptr() as u64,
            ob_x: 0,
            ob_y: y,
            ob_width: 112,
            ob_height: ITEM_H,
        };
        let tree: Box<[Object; 4]> = Box::new([
            Object {
                ob_next: NIL,
                ob_head: 1,
                ob_tail: 3,
                ob_type: G_BOX,
                ob_flags: 0,
                ob_state: 0,
                ob_spec: 1 << 16,
                ob_x: px - 20,
                ob_y: py - ITEM_H - 9,
                ob_width: 112,
                ob_height: 3 * ITEM_H,
            },
            s(A, 0, 2, false),
            s(B, ITEM_H, 3, false),
            s(C, 2 * ITEM_H, 0, true),
        ]);
        Box::leak(tree).as_mut_ptr()
    }

    /// The pointer rests on the second item, button up; the script presses
    /// (the pick), holds for three more packets, releases, idles. Before the
    /// second wait `popup_shown` returned on the press itself — with the
    /// button still down and the hold packets unconsumed.
    #[test]
    fn popup_shown_returns_with_the_picking_button_up() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let host = ScriptHost::new();
        park_at(&host, 320, 240);
        let (px, py) = crate::vdi::mouse_position();
        assert_eq!(crate::vdi::mouse_buttons() & 1, 0, "the button starts up");

        let tree = popup_tree(px, py);
        let tp = tree as *const Object;
        assert_eq!(object::child_at(tp, 0, px, py), 2, "the pointer must rest on the second item");

        host.arm(&[0, 0, 1, 1, 1, 1, 0, 0, 0]);
        crate::vdi::cursor_hide(&RASTER);
        let (bx, by, bw, bh) = sr_rect(ObjTree::new(tp), 0, &RASTER);
        let saved = gsxif::bb_save(&RASTER, bx, by, bw, bh).expect("the heap fallback saves the rectangle");
        let (picked, _keystate) = popup_shown(tree, 0, &RASTER, &host, saved);

        assert_eq!(picked, 2, "the item under the pointer at the press");
        assert_eq!(
            crate::vdi::mouse_buttons() & 1,
            0,
            "popup_shown returned with the picking press still held: mn_popup waits it out \
             (ev_button(1, 0x00ff, 0x0000, rets), aes/gemmnext.c:351) before returning"
        );
        assert!(
            host.left() <= 3,
            "the hold and the release must have been consumed inside the call; {} packets left",
            host.left()
        );
    }

    // ── A88 point 2: mn_keystate = the shift state at the selecting press ──

    /// The pointer rests on the second item; the script presses with the
    /// left shift held (`0x02`), keeps the button down while the shift is
    /// released, then releases the button. `handle_popup`'s `keystate =
    /// rets[3]` is taken from the `ev_multi` that returned the press
    /// (`aes/gemmnext.c:268-272`), so `mn_keystate` is `0x02` — not the `0`
    /// the state shows by the time the release has been waited out (`:351`),
    /// nor the `0` at entry.
    #[test]
    fn popup_shown_reports_the_shift_state_at_the_press() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let host = ScriptHost::new();
        park_at(&host, 320, 240);
        let (px, py) = crate::vdi::mouse_position();
        let tree = popup_tree(px, py);
        let tp = tree as *const Object;
        assert_eq!(object::child_at(tp, 0, px, py), 2, "the pointer must rest on the second item");

        host.arm_pkts(&[
            btn(0),
            Pkt { dx: 0, dy: 0, buttons: 1, shift: 0x02 }, // the press, left shift held
            Pkt { dx: 0, dy: 0, buttons: 1, shift: 0x02 },
            Pkt { dx: 0, dy: 0, buttons: 1, shift: 0 }, // shift let go, button still down
            btn(0), // the release
            btn(0),
            btn(0),
        ]);
        crate::vdi::cursor_hide(&RASTER);
        let (bx, by, bw, bh) = sr_rect(ObjTree::new(tp), 0, &RASTER);
        let saved = gsxif::bb_save(&RASTER, bx, by, bw, bh).expect("the heap fallback saves the rectangle");
        let (picked, keystate) = popup_shown(tree, 0, &RASTER, &host, saved);

        assert_eq!(picked, 2, "the item under the pointer at the press");
        assert_eq!(
            keystate, 0x02,
            "mn_keystate is the shift state at the selecting press (handle_popup, \
             aes/gemmnext.c:272 — keystate = rets[3] of the ev_multi that returned MU_BUTTON; \
             copied to mdata at :342); the host reported 0 by the time the release was waited out"
        );
        assert_eq!(host.keyboard_shift_state(), 0, "the state at return is 0: the value was sampled at the press");
        assert_eq!(crate::vdi::mouse_buttons() & 1, 0, "returned with the button up");
    }

    /// A126: the press comes with CapsLock (0x10), Insert-with-Alt (0x40),
    /// a program-set bit 7 (0x80) and Alt (0x08) in the BIOS shift byte —
    /// 0xd8. `mn_keystate` is `rets[3]` = `kstate`, which EmuTOS takes from
    /// `gsx_kstate` → `vq_key_s` → `gshift_s` = `Kbshift(-1) & 0x000f`
    /// (`aes/gemgsxif.c:432-436`, `vdi/vdi_input.c:110-113,183-187`): 0x08.
    #[test]
    fn popup_keystate_carries_only_the_four_modifier_bits() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let host = ScriptHost::new();
        park_at(&host, 320, 240);
        let (px, py) = crate::vdi::mouse_position();
        let tree = popup_tree(px, py);
        let tp = tree as *const Object;

        host.arm_pkts(&[
            btn(0),
            Pkt { dx: 0, dy: 0, buttons: 1, shift: 0xd8 }, // the press
            Pkt { dx: 0, dy: 0, buttons: 1, shift: 0xd8 },
            btn(0), // the release
            btn(0),
            btn(0),
        ]);
        crate::vdi::cursor_hide(&RASTER);
        let (bx, by, bw, bh) = sr_rect(ObjTree::new(tp), 0, &RASTER);
        let saved = gsxif::bb_save(&RASTER, bx, by, bw, bh).expect("the heap fallback saves the rectangle");
        let (picked, keystate) = popup_shown(tree, 0, &RASTER, &host, saved);

        assert_eq!(picked, 2, "the item under the pointer at the press");
        assert_eq!(
            keystate, 0x08,
            "mn_keystate = kstate = Kbshift(-1) & 0x0f (gshift_s, vdi/vdi_input.c:183-187): \
             Alt only, not CapsLock/MODE_INSERT/bit 7"
        );
    }

    // ── A88 point 3: clamp_ypos ────────────────────────────────────────────

    /// `clamp_ypos` (`aes/gemmnext.c:114-127`) on a 640×480 screen with the
    /// 8×16 system font (`gl_hchar` 16, `gl_hbox` = `gl_rfull.g_y` 19): a
    /// 57 px box requested at y = −19 (start item 1 at y = 0) is pushed down
    /// in 16 px steps until it clears the bar row — −19 → −3 → 13 → 29; one
    /// requested at y = 460 (start item at the screen bottom) is pulled up
    /// until it fits above `gl_rscreen.g_h - ob_height` = 423 — 460 → 444 →
    /// 428 → 412; a box that fits stays where it is; a box taller than the
    /// screen ends up below the bar row (the second rule wins).
    #[test]
    fn clamp_ypos_moves_the_box_in_hchar_steps_off_the_bar_and_onto_the_screen() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let hchar = super::super::graf::gl_hchar();
        assert_eq!((hchar, gl_hbox()), (16, 19), "the 8×16 system font");
        let tree = popup_tree(100, 100);
        let t = ObjTree::new(tree);
        assert_eq!(t.height(0), 57);

        t.set_y(0, -19); // ypos 0 for start item 1 (ob_y 0): 0 − 0 … a box at −19 models start item 2
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 29, "−19 → −3 → 13 → 29: the first y ≥ gl_rfull.g_y reached in gl_hchar steps (gemmnext.c:124-126)");

        t.set_y(0, 460);
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 412, "460 → 444 → 428 → 412: the first y ≤ gl_rscreen.g_h − ob_height (423) (gemmnext.c:119-121)");

        t.set_y(0, 200);
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 200, "a box that fits is not moved");

        t.set_y(0, 19);
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 19, "y = gl_rfull.g_y is not \"into the menu area\"");

        t.set_y(0, 18);
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 34, "y = 18 reaches into the bar row: one step down");

        t.set_height(0, 500); // taller than the screen
        t.set_y(0, 0);
        clamp_ypos(t, 0, &RASTER);
        assert_eq!(t.y(0), 32, "0 → −16 → … until ≤ 480 − 500 = −20 (−32), then up to ≥ 19: 32 — the bar-row rule wins");
    }

    // ── A88 point 1: the BEG_MCTRL / END_MCTRL bracket ─────────────────────

    /// A bar tree whose `THEACTIVE` is one 56 px title wide: `ROOT` (`G_IBOX`
    /// 640×480) → `THEBAR` (`G_BOX` 640×19) → `THEACTIVE` (`G_IBOX` 56×19) →
    /// one `G_TITLE`.
    fn bar_tree() -> *mut Object {
        static T: &[u8] = b"File\0";
        let o = |ty: u16, fl: u16, spec: u64, x, y, w, h, next, head, tail| Object {
            ob_next: next,
            ob_head: head,
            ob_tail: tail,
            ob_type: ty,
            ob_flags: fl,
            ob_state: 0,
            ob_spec: spec,
            ob_x: x,
            ob_y: y,
            ob_width: w,
            ob_height: h,
        };
        let tree: Box<[Object; 4]> = Box::new([
            o(25, 0, 0, 0, 0, 640, 480, NIL, 1, 1),
            o(G_BOX, 0, 0, 0, 0, 640, 19, 0, 2, 2),
            o(25, 0, 0, 0, 0, 56, 19, 1, 3, 3),
            o(G_TITLE, LASTOB, T.as_ptr() as u64, 0, 0, 56, 19, 2, NIL, NIL),
        ]);
        Box::leak(tree).as_mut_ptr()
    }

    /// **The bracket.** A bar is installed (`gl_mntree` set, `gl_ctwait` =
    /// `THEACTIVE` = (0,0,56,19)); the pointer rests on the desktop with the
    /// opening press held when `mn_popup` is entered. While `mn_popup` waits
    /// for that button to come up (`ev_button`, `:314`), the script drags the
    /// pointer into `THEACTIVE`, then releases it there with a one-pixel
    /// move — the exact `mchange` condition for handing the mouse to
    /// `ctl_pd` (`button == 0 && gl_mntree && in_mrect(&gl_ctwait)`,
    /// `aes/geminput.c:388-389`). Under `wm_update(BEG_MCTRL)` = `fm_own(TRUE)`
    /// (`:312`; `aes/gemfmlib.c:89-90`) `gl_mntree` is NULL for the whole
    /// call, so no hand-over happens and no bar track can start: the pop-up
    /// keeps the mouse. `END_MCTRL` (`:353`) puts the tree back. Without the
    /// bracket the release in the title hands the mouse to the Control
    /// Manager (`own_mouse()` false after the call).
    ///
    /// The box is requested at x = 700, off the right edge of the 640 px
    /// screen: its save rectangle is empty, so `popup_blit_save` answers
    /// `None` before it asks the BDOS pool (`gsxif.rs`, the `w <= 0` guard)
    /// and the no-pick branch (D-086) follows. That keeps the case
    /// independent of whether another test in this binary has installed the
    /// bookkeeping pool (`resource::mfree_tests`): with one present, a
    /// non-empty save would take a block whose kernel high-half alias no host
    /// process maps. The bracket under test wraps the first wait, which comes
    /// before the placement — the box position is not part of the claim.
    #[test]
    fn mn_popup_parks_the_menu_tree_and_keeps_the_mouse_for_its_whole_call() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let host = ScriptHost::new();
        let bar = bar_tree();
        install_bar_state(bar);
        install_bar_ctwait(bar);
        assert_eq!(ctwait_bar_rect(), Some((0, 0, 56, 19)), "gl_ctwait = THEACTIVE's rectangle (gemmnlib.c:717)");

        park_at(&host, 30, 200);
        // The opening press, on the desktop: the application owns the mouse.
        host.arm(&[1]);
        host.pump_input();
        assert_eq!(crate::vdi::mouse_buttons() & 1, 1, "the opening press is held at the call");
        assert!(super::super::input::own_mouse(), "the application owns the mouse at the call");

        let popup = popup_tree(700, 200);
        // Consumed inside mn_popup's first wait: drag up into THEACTIVE with
        // the button held (no hand-over while a button is down), then release
        // with a 1 px move inside the title — mchange with button == 0 inside
        // gl_ctwait.
        let mut script = Vec::new();
        for _ in 0..9 {
            script.push(Pkt { dx: 0, dy: -20, buttons: 1, shift: 0 }); // (30,200) → (30,20)
        }
        script.push(Pkt { dx: 0, dy: -12, buttons: 1, shift: 0 }); // (30,8): in THEACTIVE, held
        script.push(Pkt { dx: 1, dy: 0, buttons: 0, shift: 0 }); // release + move: (31,8)
        script.push(btn(0));
        script.push(btn(0));
        host.arm_pkts(&script);

        let (item, keystate) = mn_popup(popup, 0, 1, 700, 200, &RASTER, &host);

        let seen = host.seen();
        eprintln!("a88 bracket: item={item} keystate={keystate} seen={seen:?}");
        assert_eq!((item, keystate), (NIL, 0), "empty save rectangle: the D-086 no-pick branch, mn_keystate 0 (gemmnext.c:347)");
        assert_eq!(ObjTree::new(popup).x(0), 700, "ob_x = xpos, unclamped (gemmnext.c:322)");
        assert!(seen.len() >= 11, "the first wait consumed the drag and the release; seen {}", seen.len());
        assert_eq!(seen[10].at, (31, 8), "the release landed inside THEACTIVE; seen = {seen:?}");
        assert!(
            super::super::input::own_mouse(),
            "the pop-up kept the mouse: with gl_mntree parked, mchange's hand-over to ctl_pd \
             (aes/geminput.c:388-389) cannot fire while the pointer crosses the bar during \
             menu_popup — no bar track can start (A88 point 1); seen = {seen:?}"
        );
        assert!(
            seen.iter().all(|s| s.mntree_null),
            "gl_mntree must be NULL for the whole of mn_popup (fm_own(TRUE), aes/gemfmlib.c:89-90, \
             via wm_update(BEG_MCTRL), aes/gemmnext.c:312); seen = {seen:?}"
        );
        assert_eq!(super::active(), bar, "END_MCTRL restores gl_mntree (fm_own(FALSE), aes/gemfmlib.c:103)");
        assert_eq!(crate::vdi::mouse_buttons() & 1, 0, "returned with the button up");

        remove_bar();
        assert!(super::active().is_null());
    }

    // ── A102 point 2: fm_own gives the caller the mouse, and gives it back ──

    /// **`set_mown` on both edges of the bracket.** `fm_own(TRUE)`'s
    /// `ml_ocnt == 0` body ends in `get_mown(&ml_pmown); ct_chgown(rlr,
    /// &gl_rscreen);` (`aes/gemfmlib.c:92-93`) — `set_mown(rlr)`: with the
    /// buttons up the caller owns the mouse from here (`aes/geminput.c:222,
    /// 225`); `fm_own(FALSE)`'s 1 → 0 body is `ct_chgown(ml_pmown, &ml_ctrl)`
    /// (`:102`) — the owner found on entry gets it back. Observable only when
    /// that owner is `ctl_pd`: a bar is installed and the pointer has entered
    /// `THEACTIVE` with the buttons up, `mchange`'s hand-over
    /// (`aes/geminput.c:388-389`). Then `BEG_MCTRL` → the application owns
    /// the mouse; `END_MCTRL` → `ctl_pd` again. With a button held on entry
    /// `set_mown` changes nothing either way (`if (!button)`, `:222`). NewTOS's
    /// bracket parked the menu tree and counted, and left `gl_mowner` alone.
    #[test]
    fn fm_own_gives_the_caller_the_mouse_and_hands_it_back_to_the_previous_owner() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let host = ScriptHost::new();
        let bar = bar_tree();
        install_bar_state(bar);
        install_bar_ctwait(bar);
        let own = super::super::input::own_mouse;
        let fm_own = super::super::window::fm_own;

        // Buttons up on the desktop: the application owns the mouse (the
        // Control Manager's w_setactive, aes/gemctrl.c:442, in the state a
        // previous case may have left).
        park_at(&host, 30, 200);
        super::super::input::w_setactive();
        assert!(own(), "the application owns the mouse on the desktop");

        // Into THEACTIVE with the buttons up: mchange hands the mouse to ctl_pd.
        park_at(&host, 30, 8);
        assert!(!own(), "in THEACTIVE with the buttons up: gl_mowner = ctl_pd (mchange, aes/geminput.c:388-389)");

        fm_own(true);
        assert!(own(), "BEG_MCTRL: ct_chgown(rlr, &gl_rscreen) → set_mown(rlr), the caller owns the mouse (aes/gemfmlib.c:93)");
        fm_own(false);
        assert!(!own(), "END_MCTRL: ct_chgown(ml_pmown, &ml_ctrl) → the previous owner, ctl_pd, again (aes/gemfmlib.c:102)");

        // Nested brackets: only the outermost edges move the mouse.
        fm_own(true);
        assert!(own());
        fm_own(true);
        fm_own(false);
        assert!(own(), "an inner END_MCTRL (ml_ocnt 2 → 1) hands nothing back");
        fm_own(false);
        assert!(!own(), "the outermost END_MCTRL hands the mouse back to ctl_pd");

        // A button held on entry: set_mown does nothing (aes/geminput.c:222).
        host.arm(&[1]);
        host.pump_input();
        assert!(!own(), "the press in the bar row is ctl_pd's (chk_ctrl, aes/geminput.c:96-97)");
        fm_own(true);
        assert!(!own(), "BEG_MCTRL with a button held: set_mown's if (!button) — no change");
        host.arm(&[0]);
        host.pump_input();
        fm_own(false);
        assert!(!own(), "END_MCTRL: ml_pmown was ctl_pd");

        // The application owning on entry keeps the mouse throughout.
        park_at(&host, 30, 200);
        super::super::input::w_setactive();
        assert!(own());
        fm_own(true);
        assert!(own());
        fm_own(false);
        assert!(own(), "END_MCTRL: ml_pmown was the application");

        remove_bar();
        super::super::input::w_setactive();
        assert!(own(), "left as found: the application owns the mouse, no bar");
    }
}

#[cfg(test)]
mod acc_title_tests {
    //! TASK/ACCTITLE — the desk-accessory titles `menu_fixup` links into a
    //! menu tree must be readable by whoever reads that tree. On EmuTOS the
    //! string is ordinary memory an application reads like its own (WinDom's
    //! `menu_exec_cmd` `strstr`s every Desk entry); here the copies live in a
    //! page of their own that is published read-only to every process, an
    //! application's tree gets that page's user address and the kernel
    //! desktop's tree the kernel address.
    use super::*;
    use crate::bdos::process::{kva_to_user_va, usermem};

    /// The NUL-terminated string at `p` (bounded by one title row).
    fn cstr_at(p: u64) -> &'static [u8] {
        // SAFETY: `p` names a row of `TITLES`, which is always NUL-terminated
        // within `ACC_TITLE_LEN` (the copy reserves the last byte).
        unsafe {
            let s = core::slice::from_raw_parts(p as *const u8, ACC_TITLE_LEN);
            &s[..s.iter().position(|&c| c == 0).unwrap()]
        }
    }

    #[test]
    fn titles_own_their_page_and_each_reader_gets_an_address_it_can_follow() {
        // The page invariant `usermem::expose` requires: page-aligned, one
        // page, nothing but the copies in it.
        assert_eq!(core::mem::align_of::<TitlePage>(), 4096);
        assert_eq!(core::mem::size_of::<TitlePage>(), 4096);
        assert_eq!(TITLES.0.get() as u64 % 4096, 0);

        static TITLE: &[u8] = b"  Probe Accessory\0";
        assert!(copy_title(crate::uaccess::Space::Kernel, 1, TITLE.as_ptr() as u64).is_ok());

        let objs = [Object {
            ob_next: NIL,
            ob_head: NIL,
            ob_tail: NIL,
            ob_type: 0,
            ob_flags: 0,
            ob_state: 0,
            ob_spec: 0,
            ob_x: 0,
            ob_y: 0,
            ob_width: 0,
            ob_height: 0,
        }];

        // A kernel-installed tree (`mntree_accessor` with no owner slot):
        // the kernel address of the copy.
        let k = title_spec(ObjTree::lp64(objs.as_ptr()), 1);
        assert_eq!(k, TITLES.0.get() as u64 + ACC_TITLE_LEN as u64);
        assert_eq!(cstr_at(k), b"  Probe Accessory");

        // An application's tree: the user address of the published page —
        // the one the publication recorded, the copy's own VA = PA name —
        // and that address resolves to the copy for the AES reading the tree
        // through its owner (`usermem::resolve_kva`, A97).
        let a = title_spec(ObjTree::app_at(objs.as_ptr(), Abi::Lp64), 1);
        let base = TITLES_USER_VA.load(Ordering::Relaxed);
        assert_ne!(base, 0, "the page is published on first use for an application tree");
        assert_eq!(a, base + ACC_TITLE_LEN as u64);
        assert_eq!(a, kva_to_user_va(title_copy_kva(1)));
        let kv = usermem::resolve_kva(0, a).expect("the published title resolves");
        assert_eq!(cstr_at(kv), b"  Probe Accessory");

        // A second application tree reuses the publication.
        assert_eq!(title_spec(ObjTree::app_at(objs.as_ptr(), Abi::Ilp32), 0), base);

        assert!(copy_title(crate::uaccess::Space::Kernel, 1, 0).is_ok());
        assert_eq!(cstr_at(k), b"");
    }
}
