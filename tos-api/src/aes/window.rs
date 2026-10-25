//! AES Window Manager (`wind_*`, opcodes 100–109) — Batch 1.
//!
//! Spec: [`docs/api/aes/window-graphics.md`]. Source: EmuTOS @ `1a7484ab`
//! `aes/gemwmlib.c` (`wm_*`, `wm_calc`, `w_getsize`). A window has a
//! **border** rectangle (the full frame) and a **work** rectangle (the
//! client area inside the gadgets); `wm_calc` converts between them from
//! the `kind` widget bitmask. The frame is drawn **procedurally** at our
//! altitude (each gadget rect computed from the border rect + `kind`),
//! rather than via EmuTOS's 19-object `W_ACTIVE` tree — same visible
//! frame and hit-testing without porting the tree machinery.
//!
//! **Batch-1 scope:** the window table + lifecycle (`create`/`open`/
//! `close`/`delete`), geometry (`calc`/`get`/`set`/`find`), and frame
//! drawing (border, title + name, closer/fuller/sizer, scroll arrows +
//! slider tracks). The `WM_*` message protocol (redraw/topped/moved/…),
//! the rectangle list (visible pieces), interactive drag/resize, and
//! `graf_*` are Batch 2+. No save/restore under windows yet (see the
//! Menu-Manager note); closing erases to the desktop background.

use super::graf::{gl_hbox, gl_hchar, gl_wchar};
use super::object::{
    add_3d_effect, gr_crack, gr_rect, render_tree_app, render_tree_lp64, xor_color, Object,
    ACT_BUTCOL, ACT_COLCHANGE, ACT_TXTMOVE, BACKGR_COL, IP_PATTERNS,
};
use super::Aespb;
use crate::bdos::process::{self, usermem, Abi};
use crate::host::{host, Host};
use crate::vdi::backend::{ClipRect, PatternFill, Pel, RasterBackend, WriteMode};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI16, AtomicI32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use tos_types::aes::{
    Grect, ADJ3DSTD, BEG_MCTRL, BEG_UPDATE, CLOSER, DESKWH, DNARROW, END_MCTRL, END_UPDATE, FULLER,
    HOTCLOSE, HSLIDE, INFO, IP_4PATT, IP_HOLLOW, IP_SOLID, LFARROW, MOVER, NAME, NIL, RTARROW,
    SIZER, TE_CNTR, TE_LEFT, UPARROW, VSLIDE, WA_DNLINE, WA_DNPAGE, WA_LFLINE, WA_LFPAGE,
    WA_RTLINE, WA_RTPAGE, WA_UPLINE, WA_UPPAGE, WC_BORDER, WC_WORK, WF_COLOR, WF_CXYWH, WF_DCOLOR,
    WF_FIRSTXYWH, WF_FXYWH, WF_HSLIDE, WF_HSLSIZ, WF_INFO, WF_NAME, WF_NEWDESK, WF_NEXTXYWH,
    WF_OWNER, WF_PXYWH, WF_SCREEN, WF_TOP, WF_VSLIDE, WF_VSLSIZ, WF_WXYWH, WM_ARROWED, WM_CLOSED,
    WM_FULLED, WM_HSLID, WM_MOVED, WM_REDRAW, WM_SIZED, WM_TOPPED, WM_UNTOPPED, WM_VSLID,
};

// ── kind (border widget) bits, aesdefs.h:519-531 (`tos_types::aes`) ──

const VGADGETS: u16 = UPARROW | DNARROW | VSLIDE;
const HGADGETS: u16 = LFARROW | RTARROW | HSLIDE;
/// Top-row gadgets — EmuTOS `TGADGETS` (gemwmlib.h:16). **Includes `MOVER`**:
/// in the 3D branch a window with only a `MOVER` still gets a full title band
/// (the non-3D branch tests `NAME|CLOSER|FULLER` and gives it none).
const TGADGETS: u16 = NAME | CLOSER | FULLER | MOVER;

// ── WF_* fields, aesdefs.h:534-552 (`tos_types::aes`) ──
// `WF_KIND` (1) is deliberately absent: `wm_get`/`wm_set` do not handle it in
// this EmuTOS build (both fall through to `return FALSE`, gemwmlib.c:1591,1745),
// so there is nothing for the dispatcher to match on.

/// Number of window gadget-colour slots — one per `W_*` gadget object (EmuTOS
/// `NUM_ELEM`, gemlib.h:58). `WF_COLOR`/`WF_DCOLOR` index the per-window /
/// global default colour-word arrays by gadget number.
const NUM_ELEM: usize = 19;

// The `W_*` gadget object indices (gemlib.h:38-56): the slot of each gadget's
// colour word in `w_tcolor`/`w_bcolor` (`W_DATA` 6 / `W_WORK` 7 have no drawn
// pixel — `th = 0`, `G_IBOX` — and are never consulted).
const W_BOX_IDX: usize = 0;
const W_TITLE_IDX: usize = 1;
const W_CLOSER_IDX: usize = 2;
const W_NAME_IDX: usize = 3;
const W_FULLER_IDX: usize = 4;
const W_INFO_IDX: usize = 5;
const W_SIZER_IDX: usize = 8;
const W_VBAR_IDX: usize = 9;
const W_UPARROW_IDX: usize = 10;
const W_DNARROW_IDX: usize = 11;
const W_VSLIDE_IDX: usize = 12;
const W_VELEV_IDX: usize = 13;
const W_HBAR_IDX: usize = 14;
const W_LFARROW_IDX: usize = 15;
const W_RTARROW_IDX: usize = 16;
const W_HSLIDE_IDX: usize = 17;
const W_HELEV_IDX: usize = 18;

/// Global default per-gadget colour words for a **topped** window (EmuTOS
/// `gl_wtcolor`, `wm_start` gemwmlib.c:1347-1354): every slot `0x1101`
/// (border+text black, transparent fill, pattern 0), with `W_NAME |= 0xa0` and
/// the two sliders `|= 0x10`. These are the AES colour-word values a
/// `wind_get(WF_DCOLOR)` must return 1:1 — and, since A79, what
/// [`draw_frame`] paints a gadget from ([`gadget_ink`]): every gadget is
/// rendered literally from its word, so these defaults *are* the look.
const GL_WTCOLOR_DEFAULT: [i16; NUM_ELEM] = {
    let mut a = [0x1101i16; NUM_ELEM];
    a[W_NAME_IDX] = 0x1101 | 0xa0; // 0x11a1
    a[W_VSLIDE_IDX] = 0x1101 | 0x10; // 0x1111
    a[W_HSLIDE_IDX] = 0x1101 | 0x10; // 0x1111
    a
};

/// Global default per-gadget colour words for an **untopped** window (EmuTOS
/// `gl_wbcolor`, `wm_start` gemwmlib.c:1348-1357): every slot `0x1101`, the two
/// sliders `|= 0x10` (3D object support on — our Falcon030/TOS4 target profile).
const GL_WBCOLOR_DEFAULT: [i16; NUM_ELEM] = {
    let mut a = [0x1101i16; NUM_ELEM];
    a[W_VSLIDE_IDX] = 0x1101 | 0x10; // 0x1111
    a[W_HSLIDE_IDX] = 0x1101 | 0x10; // 0x1111
    a
};

/// `objc_draw`/`ob_draw` recursion depth for the desktop-background composite
/// (EmuTOS `MAX_DEPTH`, `obdefs.h:30`): deep enough to draw the registered
/// desktop tree's `DROOT` box **and** every icon child under it.
const MAX_DEPTH: i16 = 8;

// ── gadget glyphs (ST system font 0x01-0x07) ──
const G_UP: u16 = 0x01;
const G_DN: u16 = 0x02;
const G_RT: u16 = 0x03;
const G_LF: u16 = 0x04;
const G_CLOSE: u16 = 0x05;
const G_SIZE: u16 = 0x06;
const G_FULL: u16 = 0x07;

// The system-font cell (`gl_wchar`/`gl_hchar`) and the unadjusted gadget box
// height (`gl_hbox = gl_hchar + 3`) come from one place, `graf::gl_wchar` /
// `graf::gl_hchar` / `graf::gl_hbox` (EmuTOS `gsx_start`, `gemgraf.c:405-412`).
// `gl_wchar`/`gl_hchar` are the sizer drag's minimum width/height for a window
// without the respective gadgets (`hctl_window`, `gemctrl.c:252-253`);
// `gl_hbox` is the height of every gadget's glyph cell and the menu-bar row.

/// Unadjusted gadget box width — EmuTOS `gl_wbox`
/// (`gl_wbox = max(gl_wchar+4, gl_hbox·aspect)`, `gemgraf.c:412-415`): 19 for
/// the 8×16 system font. This is what `graf_handle` reports (`gemsuper.c:287-291`)
/// and what the *glyph cell* of every window gadget measures — the 3D window
/// layout inflates the surrounding box ([`adj_wbox`]), never this.
pub(super) const GL_WBOX: i16 = 19;

/// EmuTOS `MAX_COORDINATE` (`include/sysconf.h:63`) — the "no limit" extent the
/// Control Manager gives `gr_dragbox` for the vertical direction, so a window
/// can be dragged off the bottom of the screen.
const MAX_COORDINATE: i16 = 10000;

/// 3D-adjusted gadget box width — EmuTOS `adj_wbox`, set in `wm_init`
/// (`gemwmlib.c:1311-1324`): `gl_wbox + 2*ADJ3DSTD`, shrunk by 1 on a screen
/// narrower than 640 (and by 1 more when such a screen is at least 400 tall) to
/// mimic TOS4's ST-Low work area. Computed from the live screen instead of
/// cached at init; for a fixed resolution the value is the same either way.
fn adj_wbox() -> i16 {
    let mut a = GL_WBOX + 2 * ADJ3DSTD;
    let s = screen_rect();
    if s.w < 640 {
        a -= 1;
        if s.h >= 400 {
            a -= 1;
        }
    }
    a
}

/// 3D-adjusted gadget box height — EmuTOS `adj_hbox` (`gemwmlib.c:1312`):
/// `gl_hbox + 2*ADJ3DSTD`, with no resolution-dependent correction.
fn adj_hbox() -> i16 {
    gl_hbox() + 2 * ADJ3DSTD
}

const WHITE: Pel = 0;
/// VDI pen 1 (`obdefs.h`, EmuTOS `BLACK`).
const BLACK: Pel = 1;
const GRAY_LT: Pel = 8; // 3D face (EmuTOS LWHITE)

/// Colour of the window drop shadow.
///
/// EmuTOS paints it in `W_BOX`'s border colour — `gr_crack` takes bits 15..12
/// of `ob_spec`'s low word `0x1101` (`aes/gemgraf.c:612-627`), i.e. BLACK.
///
/// NewTOS deviates **deliberately** here (user decision 2026-09-10): a hard
/// black band reads far heavier on a 1920x1080 truecolor screen than it did on
/// the TT's four planes, so the shadow is drawn in light grey. This is a look
/// decision, not a conformance one — the dump records rectangles, not pixels,
/// so it changes nothing in the comparison against the reference. Like
/// [`DROP_SHADOW_SIZE`] it is the **one** place the value lives, and tests take
/// it from here instead of naming a colour of their own.
pub const DROP_SHADOW_COLOR: Pel = GRAY_LT;

/// Number of application window handles (1..=`MAX_WINDOWS`). EmuTOS ties the
/// table size to the AES version it advertises: `NUM_WIN` is **16** for
/// `AES_VERSION > 0x0320` and 8 below it (`include/sysconf.h:26-35`), and the
/// desktop window at handle 0 is one of those slots. NewTOS reports
/// `ap_version 0x0340` (`super::appl`), so the contract is the 16-slot table —
/// fifteen application handles, 1..=15.
///
/// Measured: `tosdump/cases/aes-r2-wind.tsv` case 154 (the ninth
/// `wind_create` after the `wind_new` reset) recorded **9** on the reference
/// TT and **−1** here while this was 8.
const MAX_WINDOWS: usize = 15;
/// Window-table size — the application handles **plus the desktop window at
/// handle 0** (EmuTOS `NUM_WIN`, `gemlib.h:136` `WINDOW w_win[NUM_WIN]`).
const NUM_WIN: usize = MAX_WINDOWS + 1;

/// Ceiling on the bytes one frame build reads from a `WF_NAME`/`WF_INFO`
/// string — the size of the scratch buffer [`LiveStr::read`] fills and the
/// upper bound of its walk. **A walk bound, not a contract limit** (A83):
/// the AES stores the caller's pointer and never truncates the string
/// (`docs/api/aes/window-graphics.md`, "`WF_NAME`/`WF_INFO` store a pointer,
/// not a copy; no length limit"); what a frame build actually needs is the
/// glyphs that fit the gadget, so [`draw_frame`] bounds each read by the
/// gadget's width in characters + 1 (see [`label_walk`]) and this ceiling only
/// caps a gadget wider than 512 glyphs (> 4096 px), where the walk stops and
/// the label simply ends. Big enough for the 1920-px profile's widest name
/// area (234 glyphs) and info line (240 glyphs).
const LIVE_STR_MAX: usize = 512;

/// The desktop window's `kind` — EmuTOS `NONE` (`gemwmlib.c:1290`, no gadgets).
const NONE: u16 = 0;

// ── the drop shadow ─────────────────────────────────────────────────────

/// `DROP_SHADOW_SIZE` (EmuTOS `aes/gemwmlib.c:49`) — how far, in pixels, every
/// window's solid drop shadow reaches past its border rectangle to the right
/// and downwards.
///
/// **The single source of truth for the shadow.** Everything that has to agree
/// on the size takes it from here, because a mismatch between any two of them
/// is a visible defect rather than a cosmetic one:
///
/// | Reader | via | EmuTOS |
/// |---|---|---|
/// | owner rectangle list — own start rect **and** the rect every higher window breaks it with | [`ws_true`] in [`border_pieces`] | `w_getsize(WS_TRUE, …)`, `gemwmlib.c:254-261`, used twice in `newrect`, `aes/gemwrect.c:190,203` |
/// | the draw walk (frames, sliders, desktop background) | [`walk_rects`] over [`border_pieces`] | `do_walk`, `gemwmlib.c:286-312` |
/// | `WF_FIRSTXYWH`/`WF_NEXTXYWH` | [`compute_rlist`] over [`border_pieces`], clipped per piece by [`rlist_owns`] | `w_owns`, `gemwmlib.c:1230-1240,1556-1561` |
/// | the `WM_REDRAW` union | [`w_union`] over [`border_pieces`] | `w_union`, `gemwmlib.c:869-884` |
/// | "is this window overlapped" | [`ws_true`] in [`is_broken`] | `mkrect` sets `VF_BROKEN` after breaking with the `WS_TRUE` rect, `aes/gemwrect.c:136-162,190` |
/// | the pixels themselves | [`draw_frame`] | `W_ACTIVE[ROOT].ob_state = SHADOWED` (`gemwmlib.c:1284`) drawn by `ob_draw` (`aes/gemoblib.c:889-901`) |
/// | the frame-draw clip of a damage-scoped repaint | [`draw_frame_pieces`] | `w_cpwalk`'s `usetrue == FALSE` branch, `gemwmlib.c:369-374` |
/// | the damage rectangle of open / close / delete / move / size | [`plus_shadow`] | `draw_change`, `gemwmlib.c:1152-1154,1213-1215` |
///
/// Reporting the area as covered without painting it leaves a `DROP_SHADOW_SIZE`
/// wide strip of stale pixels at every window edge; painting it without
/// reporting it lets the desktop and the windows underneath draw back into it.
///
/// EmuTOS uses `2` on a 640×480 ST screen. The value here is deliberately kept
/// at `2` for now so the drop-shadow-dependent cases of the frozen TT reference
/// table (`tosdump/cases/aes-r2b-rlist.tsv`, cases 166/174/183/213) stay a valid
/// oracle for a real-hardware run; the user's decision to raise it to `4` for
/// the Pi profile is applied only after that run (`TASK/A23.md`,
/// "Anwenderentscheidung — Schattengröße" and its addendum).
pub const DROP_SHADOW_SIZE: i16 = 2;

/// The shadow bands are `2 * th` wide, `th` being the border thickness of the
/// `W_BOX` window element (`aes/gemoblib.c:889-901`); in EmuTOS `th = 1` from
/// `W_BOX`'s `ob_spec 0x00011101` (`gemwmlib.c:63,84`, byte 1 taken out by
/// `ob_sst`, `aes/gemobjop.c:54-57`), which is what makes
/// `DROP_SHADOW_SIZE == 2 * th` hold. We keep the identity by deriving `th`
/// from the constant instead of storing it twice — which is only exact for an
/// even size, so the build enforces that rather than trusting a comment.
const SHADOW_TH: i16 = DROP_SHADOW_SIZE / 2;
const _: () = assert!(
    2 * SHADOW_TH == DROP_SHADOW_SIZE,
    "DROP_SHADOW_SIZE must stay even: the drawn bands are 2*th wide, so an odd \
     size would report more area as covered than the frame paints"
);

/// `w_getsize(WS_TRUE, …)` — the window's border rectangle grown by the drop
/// shadow (`aes/gemwmlib.c:253-261`):
///
/// ```c
/// rc_copy(w_getxptr(which, w_handle), pt);
/// if ((which == WS_TRUE) && pt->g_w && pt->g_h)
/// {
///     pt->g_w += DROP_SHADOW_SIZE;
///     pt->g_h += DROP_SHADOW_SIZE;
/// }
/// ```
///
/// `x`/`y` are untouched — the shadow only ever grows right and down — and an
/// empty rectangle stays empty, which is what lets `newrect` bail out on a
/// zero-size window (`aes/gemwrect.c:189-192`).
///
/// This is **not** `WS_CURR`: `w_getxptr` returns the same storage for both and
/// only `WS_TRUE` gets the addition (`:235-249`), so `WF_CXYWH`, `WF_WXYWH`,
/// `WF_PXYWH`, `WF_FXYWH`, `wind_calc` and every hit test keep the bare
/// rectangle. Only the owner rectangle list, the draw clip and `VF_BROKEN` use
/// this one.
fn ws_true(curr: Grect) -> Grect {
    if curr.w != 0 && curr.h != 0 {
        Grect { w: curr.w + DROP_SHADOW_SIZE, h: curr.h + DROP_SHADOW_SIZE, ..curr }
    } else {
        curr
    }
}

/// A *damage* rectangle grown by the drop shadow — EmuTOS's unconditional
///
/// ```c
/// /* account for drop shadow (BUGFIX in 2.1) */
/// c.g_w += DROP_SHADOW_SIZE;
/// c.g_h += DROP_SHADOW_SIZE;
/// ```
///
/// at the tail of `draw_change` (`aes/gemwmlib.c:1213-1215`), just before
/// `w_drawdesk(&c)` and `w_update(start, &c, stop, moved)`. Unlike [`ws_true`]
/// this does **not** test for an empty rectangle: `draw_change` adds the shadow
/// to whatever damage it ended up with, including the zero rectangle a close
/// passes in.
///
/// Without it the area a vanishing or shrinking window's shadow used to cover
/// is never repainted and its black band stays on the screen.
fn plus_shadow(r: Grect) -> Grect {
    Grect { w: r.w + DROP_SHADOW_SIZE, h: r.h + DROP_SHADOW_SIZE, ..r }
}

/// A `WF_NAME` / `WF_INFO` string as the AES keeps it: **the caller's
/// pointer, not a copy**, plus the address space the pointer belongs to.
///
/// EmuTOS's `wm_set` stores exactly the pointer the application passed —
/// `pwin->w_pname = *(char **)pinwds` (`WF_NAME`, `aes/gemwmlib.c:1661`),
/// `pwin->w_pinfo = *(char **)pinwds` (`WF_INFO`, `:1669`) — and `w_bldactive`
/// hands that pointer to the `W_NAME`/`W_INFO` `TEDINFO` on **every** frame
/// build (`gl_aname.te_ptext = … pw->w_pname; gl_ainfo.te_ptext =
/// pw->w_pinfo;`, `:501-502`). So an application that rewrites its title
/// buffer in place sees the new text on the next frame repaint, and no length
/// limit exists in the AES — the gadget clips (A83 contract,
/// `docs/api/aes/window-graphics.md`). `w_setup` starts a window at
/// `w_pname = w_pinfo = ""` (`:215-216`): [`LiveStr::NONE`].
///
/// **Address space.** Since ARCH03 stage 1 a user address means the same in
/// every process (VA = PA, `bdos/process/mmu.rs`), but a frame is rebuilt
/// from *other* processes' AES calls too — a `Pexec` child compositing its
/// parent's windows, the desktop repainting after a child's `Pterm` — and
/// the owner may have freed the buffer meanwhile. A raw dereference of the
/// stored pointer would go through whichever `TTBR0` happens to be
/// installed and fault on memory nobody owns any more. Instead the string is
/// read through **the owner's own mapping**: [`Self::slot`] records the
/// process slot that made the `wind_set` call, and [`Self::read`] translates
/// each page of the string with [`usermem::resolve_kva`] into the kernel
/// high-half alias of the block the process was handed — the same authority
/// XBIOS `Setscreen` uses for a screen base an application passes back
/// (`xbios/screen.rs::caller_base_to_kva`). An EL1 caller (the built-in
/// desktop, the boot-time selftests) passes a kernel VA, which is read
/// directly. A NULL pointer, or a VA the owner has no mapping for (the
/// process ended or freed the block — A96's zombie windows), reads as `""`:
/// EmuTOS leaves NULL undefined and NewTOS resolves it to the empty string
/// (Lead decision, A83, 2026-09-20).
#[derive(Clone, Copy)]
struct LiveStr {
    /// The pointer exactly as the caller passed it; `0` = none → `""`.
    ptr: u64,
    /// The process slot whose mapping `ptr` is valid in — `Some` for an EL0
    /// caller (`process::slot_for(process::current())` at `wind_set` time),
    /// `None` for an EL1 caller, whose pointer is a kernel VA.
    slot: Option<usize>,
}

impl LiveStr {
    /// `w_pname = ""` (`w_setup`, `gemwmlib.c:215-216`): no string.
    const NONE: LiveStr = LiveStr { ptr: 0, slot: None };

    /// The string `ptr` names in the address space of the **current caller**
    /// — taken inside the `wind_set` trap, where the caller is the process
    /// that owns the pointer.
    fn of_caller(ptr: u64) -> LiveStr {
        LiveStr { ptr, slot: process::slot_for(process::current()) }
    }

    /// Read the string's current contents — at most `limit` bytes and never
    /// more than `buf` holds — into `buf`, returning the bytes read (without
    /// the NUL). The walk ends at the NUL, at `limit`, or at the first byte
    /// the owner has no mapping for; a NULL pointer reads as empty.
    ///
    /// For an EL0 owner every **page** of the string is resolved once
    /// (`usermem::resolve_kva`: live user memory → its kernel alias, which is
    /// contiguous since VA = PA). Crossing into the next page re-resolves,
    /// which is where a string that runs into memory nobody owns stops.
    fn read<'a>(&self, buf: &'a mut [u8], limit: usize) -> &'a [u8] {
        let n = limit.min(buf.len());
        if self.ptr == 0 {
            return &buf[..0];
        }
        let page = usermem::PAGE_SIZE;
        let mut len = 0usize;
        let mut kva = 0u64;
        while len < n {
            let va = self.ptr.wrapping_add(len as u64);
            if len == 0 || va & (page - 1) == 0 {
                kva = match self.slot {
                    None => va,
                    Some(slot) => match owner_kva(slot, va) {
                        Some(k) => k,
                        None => break,
                    },
                };
            }
            // SAFETY: `kva` is either the EL1 caller's own kernel VA of a
            // NUL-terminated string (the contract `wind_set` documents for
            // an in-kernel caller) or the kernel high-half alias of a page
            // the owner process holds in its block table, and `kva` advances
            // only within that page (re-resolved at each page boundary). The
            // read is volatile: the owner may rewrite the buffer at any time.
            let c = unsafe { core::ptr::read_volatile(kva as *const u8) };
            if c == 0 {
                break;
            }
            buf[len] = c;
            len += 1;
            kva = kva.wrapping_add(1);
        }
        &buf[..len]
    }
}

/// The kernel high-half alias of user VA `va` in process `slot`'s mapping —
/// [`usermem::resolve_kva`], the seam [`LiveStr::read`] reads an EL0 owner's
/// string through and the object drawer reads an EL0 owner's `WF_NEWDESK`
/// tree through (`object.rs`, A97). The host unit tests substitute a
/// resolver over host buffers here ([`live_title_tests`],
/// [`newdesk_owner_tests`]): a host process has no process table and cannot
/// dereference a kernel high-half address, so the owner-slot and
/// dangling-pointer cases are modelled at this seam (A83).
pub(crate) fn owner_kva(slot: usize, va: u64) -> Option<u64> {
    #[cfg(test)]
    if let Some(resolve) = live_title_tests::resolver() {
        return resolve(slot, va);
    }
    usermem::resolve_kva(slot, va)
}

/// The walk bound [`draw_frame`] hands [`LiveStr::read`] for a label drawn in
/// an area `max_w` pixels wide: the glyphs that fit, plus one so that a
/// string longer than the area is seen as such (its measured length then
/// exceeds the area, which is all the centring needs to know).
fn label_walk(max_w: i16) -> usize {
    (max_w.max(0) as usize / 8) + 1
}

/// One window slot.
#[derive(Clone, Copy)]
struct Window {
    in_use: bool,
    open: bool,
    kind: u16,
    z: i16,     // stacking order (higher = nearer the top)
    /// Owning AES identity (`w_owner`, the descriptor index a `WM_*` message
    /// for this window is posted to), or [`Window::NO_OWNER`] for a free slot
    /// and for the desktop window until `WF_NEWDESK` re-owns it. `WF_OWNER`
    /// reports its value through `appl::reported_pid`.
    owner: i16,
    full: Grect, // maximum extent (WF_FXYWH)
    curr: Grect, // current border rect (WF_CXYWH)
    prev: Grect, // previous border rect (WF_PXYWH)
    /// Work-area rectangle (`WF_WXYWH`) — **stored**, not derived, exactly like
    /// EmuTOS' `w_work`: `w_move` recomputes it with `wm_calc(WC_WORK, …)` from
    /// the new `WS_CURR` (`gemwmlib.c:1073-1077`), and `wm_init` sets the
    /// desktop window's to `gl_rfull` directly (`gemwmlib.c:1294`), which is
    /// **not** what `wm_calc` would produce for `kind = NONE`. Maintained by
    /// [`set_curr`].
    work: Grect,
    /// `w_pname` / `w_pinfo` — the **caller's pointer** to the title / info
    /// string, exactly as `wind_set(WF_NAME)` / `wind_set(WF_INFO)` received
    /// it (`gemwmlib.c:1661,1669`), read afresh on every frame build
    /// (`w_bldactive`, `:501-502`). See [`LiveStr`].
    name: LiveStr,
    info: LiveStr,
    /// Vertical slider position (`WF_VSLIDE`, 1..1000 per-mille of travel)
    /// and thumb size (`WF_VSLSIZ`, 1..1000 per-mille of the track, or **−1**
    /// = "use default size", one gadget cell). `wind_create` inits the size to
    /// −1 (EmuTOS `w_vslsiz`, gemwmlib.c:232); the getter round-trips −1.
    vslide: i16,
    vslsize: i16,
    /// Horizontal slider position (`WF_HSLIDE`) and thumb size (`WF_HSLSIZ`),
    /// same per-mille encoding as the vertical pair (mirror axis); size −1 =
    /// default.
    hslide: i16,
    hslsize: i16,
    /// Per-gadget AES colour words for the topped / untopped states (EmuTOS
    /// `w_tcolor`/`w_bcolor`, gemlib.h:81-82). Set from the live global defaults
    /// at [`wind_create`], read/written by `WF_COLOR`, and **consumed by
    /// [`draw_frame`]** (A79): the frame draw cracks the word of each gadget it
    /// paints (`istop ? w_tcolor : w_bcolor`, exactly what `w_setcolor` writes
    /// into the `W_ACTIVE` tree, `gemwmlib.c:382-397`) into border / text /
    /// pattern / interior colour and text mode ([`gadget_ink`]).
    tcolor: [i16; NUM_ELEM],
    bcolor: [i16; NUM_ELEM],
    /// The owner rectangle list and the `WF_FIRSTXYWH`/`WF_NEXTXYWH` cursor
    /// into it — both belong to the **window**, not to the window manager.
    /// EmuTOS holds them as fields of the `WINDOW` struct: `ORECT *w_rlist`
    /// (the list) and `ORECT *w_rnext` (the search-first/search-next cursor),
    /// `aes/gemlib.h:84-85`. `wm_get` picks the *requested* window with
    /// `get_pwin` and takes `pwin->w_rlist` for `WF_FIRSTXYWH` or
    /// `pwin->w_rnext` for `WF_NEXTXYWH` (`gemwmlib.c:1513`, `:1556-1561`);
    /// `w_owns` advances that same window's link (`:1230-1240`). A `FIRST` on
    /// one window therefore cannot disturb an iteration another window is in
    /// the middle of.
    ///
    /// Where EmuTOS links `ORECT` cells out of one shared pool
    /// (`or_start`/`get_orect`, `aes/gemwrect.c:36-61`) we keep a fixed array
    /// per window and an index cursor; the observable FIRST/NEXT contract is
    /// the same, the array is as long as EmuTOS's whole pool ([`RCAP`] =
    /// `NUM_ORECT`, A21), and an index into our own array cannot dangle the
    /// way a freed `ORECT *` can. Written by [`compute_rlist`], read and
    /// advanced by [`rlist_owns`].
    ///
    /// The stored pieces are the **raw** `WS_TRUE` remainders `newrect` links up
    /// (`aes/gemwrect.c:165-205`), not the work-area intersections: `w_owns`
    /// clips each piece on the way out and keeps walking when the clip comes up
    /// empty (`gemwmlib.c:1230-1240`), so the border-only pieces have to be in
    /// here for the walk to step over them and for the end of the list to report
    /// the coordinates of the last one it looked at.
    ///
    /// Bound to the slot, so every path that resets a slot — `wind_create`'s
    /// `Window::EMPTY`, [`wm_delete`] (behind [`wind_delete`] and [`wm_new`]),
    /// [`init_desk_window`] — resets list and cursor **together**.
    rlist: [Grect; RCAP],
    rcount: u16,
    rnext: u16,
}

impl Window {
    /// `pwin->w_owner = NULL` (`gemwmlib.c:1482`): no identity owns the
    /// window. Distinct from every descriptor index (GEMDOS slot 0 is a valid
    /// identity), and reported by `WF_OWNER` as 0, the value the unowned
    /// desktop window has always reported.
    const NO_OWNER: i16 = -1;

    const EMPTY: Window = Window {
        in_use: false,
        open: false,
        kind: 0,
        z: 0,
        owner: Self::NO_OWNER,
        full: Grect { x: 0, y: 0, w: 0, h: 0 },
        curr: Grect { x: 0, y: 0, w: 0, h: 0 },
        prev: Grect { x: 0, y: 0, w: 0, h: 0 },
        work: Grect { x: 0, y: 0, w: 0, h: 0 },
        name: LiveStr::NONE,
        info: LiveStr::NONE,
        vslide: 0,
        vslsize: -1,
        hslide: 0,
        hslsize: -1,
        tcolor: GL_WTCOLOR_DEFAULT,
        bcolor: GL_WBCOLOR_DEFAULT,
        rlist: [Grect { x: 0, y: 0, w: 0, h: 0 }; RCAP],
        rcount: 0,
        rnext: 0,
    };
}

/// The window table. Single boot CPU only → `UnsafeCell` with a hand-rolled
/// `Sync`, mirroring the cookie jar and AES process table.
struct WinTable {
    slots: UnsafeCell<[Window; NUM_WIN]>,
    next_z: UnsafeCell<i16>, // monotonically increasing stacking counter
}
// SAFETY: only the single boot CPU touches the AES; no concurrent access.
unsafe impl Sync for WinTable {}

static TABLE: WinTable = WinTable {
    slots: UnsafeCell::new([Window::EMPTY; NUM_WIN]),
    next_z: UnsafeCell::new(1),
};

/// Set once [`wm_init`] has built the desktop window's slot.
static WM_READY: AtomicBool = AtomicBool::new(false);

/// EmuTOS `wm_init` (`gemwmlib.c:1287-1294`), reduced to the part that is state
/// rather than object-tree bookkeeping: make handle 0 a **real** window.
///
/// ```c
/// w_setup(ppd, DESKWH, NONE);                 /* VF_INUSE, sliders 0/0/-1/-1 */
/// w_setsize(WS_CURR, DESKWH, &gl_rscreen);
/// w_setsize(WS_PREV, DESKWH, &gl_rscreen);
/// w_setsize(WS_FULL, DESKWH, &gl_rfull);
/// w_setsize(WS_WORK, DESKWH, &gl_rfull);
/// ```
///
/// `gl_rscreen` is the whole screen and `gl_rfull` the screen below the menu
/// row, both fixed at `gsx_start` (`gemgraf.c:423-424`) — i.e. after the
/// resolution is known. EmuTOS runs `wm_init` at AES start-up for the same
/// reason; we run it on first use of the table, which is the first AES window
/// call and therefore equally after the screen is up. `VF_ISOPEN` is *not* set
/// (the desktop is the tree root, never an open child), so every `in_use &&
/// open` walk skips it exactly as EmuTOS' `W_TREE` walks do.
///
/// This is the desktop-window half of EmuTOS's `wm_init`; the public
/// [`wm_init`] runs it together with the window sweep and the `gl_newdesk`
/// reset, which is the whole of `gemwmlib.c:1247-1310` that is observable here.
fn init_desk_window() {
    // SAFETY: single-CPU AES; exclusive logical owner, no live borrow here.
    let d = unsafe { &mut (*TABLE.slots.get())[DESKWH as usize] };
    *d = Window::EMPTY;
    d.in_use = true; // VF_INUSE
    d.kind = NONE;
    d.curr = screen_rect(); // gl_rscreen
    d.prev = screen_rect();
    d.full = desktop_work(); // gl_rfull
    d.work = desktop_work();
}

#[allow(clippy::mut_from_ref)]
fn slots() -> &'static mut [Window; NUM_WIN] {
    // EmuTOS runs `wm_init` after `gsx_start`, i.e. once the resolution is
    // known; we run it on first use of the table. Latch it only once a raster
    // backend actually exists — before that [`screen_rect`] would answer with
    // its 640×480 fallback and freeze the desktop window at the wrong size.
    if !WM_READY.load(Ordering::Relaxed) {
        init_desk_window();
        if host().and_then(|h| h.raster()).is_some() {
            WM_READY.store(true, Ordering::Relaxed);
        }
    }
    // SAFETY: single-CPU AES; exclusive logical owner.
    unsafe { &mut *TABLE.slots.get() }
}

/// Next stacking value (raises a window to the top of the z-order).
///
/// EmuTOS has no counter: its z-order **is** the sibling chain of the window
/// tree — `wm_open` appends at `W_TREE[ROOT].ob_tail` (`w_obadd`,
/// `gemwmlib.c:186-201`, from `:1425`), `wm_mktop` moves the topped window
/// there with `ob_order(gl_wtree, w_handle, NIL)` (`:1609`), and the top
/// window is `W_TREE[ROOT].ob_tail` (`:1022,1093`). Our monotonically
/// increasing `i16` is an implementation detail with one obligation: always
/// answer a value above every open window's, without ever overflowing (A52:
/// in the dev profile `z + 1` at `i16::MAX` was a panic). When the counter
/// reaches `i16::MAX` the open windows are renumbered into `1..=n` in their
/// current order ([`renumber_z`]) and the counter carries on from `n + 1`.
/// Saturating would tie every later window at `i16::MAX` (and the tie-break
/// is the slot index — a wrong order); wrapping to `i16::MIN` would put the
/// next window under every existing one. Renumbering keeps the order exactly.
fn bump_z() -> i16 {
    // SAFETY: single-CPU AES.
    unsafe {
        let p = TABLE.next_z.get();
        if *p == i16::MAX {
            *p = renumber_z();
        }
        let z = *p;
        *p = z + 1;
        z
    }
}

/// Compress the open windows' `z` values into `1..=n`, lowest first, keeping
/// their order (ties, which a monotonic counter never produces, break by slot
/// index exactly as the draw order and [`get_top`] break them). Returns
/// `n + 1`, the next free value. Closed and free slots are not compared by
/// any z-order walk and are left alone; a re-open takes a fresh value.
fn renumber_z() -> i16 {
    // SAFETY: single-CPU AES. Accessed through the raw table pointer, not a
    // second `&mut` over the array — a caller of [`bump_z`] may be holding
    // one slot's `&mut Window` (`w.z = bump_z()`), and its own write to `z`
    // lands after this renumbering, which is the value that must win.
    let slots = TABLE.slots.get();
    let mut done = [false; NUM_WIN];
    let mut next = 1i16;
    loop {
        let mut pick = usize::MAX;
        let mut pick_z = i16::MAX;
        for (i, &was_done) in done.iter().enumerate() {
            // SAFETY: `i < NUM_WIN`; single-CPU AES, see above.
            let w = unsafe { &(*slots)[i] };
            if w.in_use && w.open && !was_done && (pick == usize::MAX || w.z < pick_z) {
                pick = i;
                pick_z = w.z;
            }
        }
        if pick == usize::MAX {
            return next;
        }
        // SAFETY: `pick < NUM_WIN`; single-CPU AES, see above.
        unsafe { (*slots)[pick].z = next };
        done[pick] = true;
        next += 1;
    }
}

// ── wind_update state (P5-005) ───────────────────────────────────────────
//
// EmuTOS `wm_update` (gemwmlib.c:1778-1797) maintains two counters: the
// counting window-tree lock `wind_spb.sy_tas` (BEG/END_UPDATE) and the
// mouse-control-ownership counter `ml_ocnt` (BEG/END_MCTRL, via `fm_own`,
// gemfmlib.c:52). Single boot CPU, so plain `Relaxed` atomics suffice.

/// Counting window-tree lock — EmuTOS `wind_spb.sy_tas`. `BEG_UPDATE` takes it
/// (`tak_flag`, `aes/gemflag.c:68-86`), `END_UPDATE` releases it (`unsync`,
/// `:99-130`, clamped at 0). A second process asking for a held lock is
/// refused and parked on the lock's wait line ([`UPDATE_WAIT`]) until the
/// holder's last `END_UPDATE` hands the lock to it (GH1). Who holds it is
/// [`UPDATE_OWNER`].
static UPDATE_LOCK: AtomicI16 = AtomicI16::new(0);

/// `ctl_pd` as a holder of the window-tree lock — the Screen Manager, which
/// `ctlmgr` runs as when it takes the screen after its own `ev_multi` fired
/// (`wm_update(BEG_UPDATE)` … `wm_update(END_UPDATE)`, `aes/gemctrl.c:463,484`).
/// Not an AES identity (those are GEMDOS slots and
/// [`super::pd::KERNEL_CONTEXT`]) and not [`NO_OWNER`].
const CTL_PD: usize = usize::MAX - 1;

/// `rlr == ctl_pd` — the inline Control Manager is running ([`ctl_begin`] …
/// [`ctl_finish`]). Our Control Manager executes inside an application's
/// `evnt_multi` trap; what it does there is `ctlmgr`'s, in `ctl_pd`'s name, so
/// the lock calls it makes on the way (`gr_dragbox`/`gr_rubwind`'s
/// `wm_update` pair, `aes/gemgrlib.c:205,219,250,267`) nest under the
/// Control Manager's own hold instead of being refused as a second process's.
static CTL_RUNNING: AtomicBool = AtomicBool::new(false);

/// Each AES identity's share of [`UPDATE_LOCK`] — how many of the counted
/// `BEG_UPDATE`s are its own. EmuTOS has one `AESPD` for the whole application
/// chain (`aes/geminit.c:851-877`), so a launched program nests on the
/// desktop's hold as the same process; NewTOS gives each GEMDOS slot an
/// identity of its own and folds them back for the ownership test
/// ([`same_holder`]). The per-identity split is what lets the dead-slot sweep
/// ([`release_locks_of`]) take exactly the dead client's holds off — `sh_ldapp`'s
/// `if (wind_spb.sy_owner == rlr) unsync(&wind_spb)` (`aes/gemshlib.c:668-669`)
/// for a client that is not the only one alive.
static UPDATE_HOLDS: [AtomicI16; super::pd::NUM_PDS] = [const { AtomicI16::new(0) }; super::pd::NUM_PDS];

/// The lock's wait line — EmuTOS `wind_spb.sy_wait`, the `EVB` list
/// `amutex` appends a refused taker to (`evinsert(e, &sy->sy_wait)`,
/// `aes/gemflag.c:89-101`) and `unsync` takes the head off
/// (`sy->sy_wait = p->e_link`, `:120`). AES identities in arrival order,
/// [`UPDATE_WAIT_LEN`] of them; each identity waits at most once.
static UPDATE_WAIT: [AtomicUsize; super::pd::NUM_PDS] = [const { AtomicUsize::new(NO_OWNER) }; super::pd::NUM_PDS];
static UPDATE_WAIT_LEN: AtomicUsize = AtomicUsize::new(0);

/// The lock was handed to this identity while it slept on the wait line
/// (`sy_owner = p->e_pd; sy_tas = 1; azombie(p, 0);`, `aes/gemflag.c:121-123`).
/// The hold is already counted; the waiter's re-executed call (the S2
/// restartable block) collects it in [`tak_flag`] instead of counting it a
/// second time.
static UPDATE_GRANTED: [AtomicBool; super::pd::NUM_PDS] = [const { AtomicBool::new(false) }; super::pd::NUM_PDS];

/// Owner of the window-tree lock — EmuTOS `wind_spb.sy_owner`
/// (`aes/geminit.c:138,837-839`). `tak_flag` writes the taking process into it
/// (`aes/gemflag.c:77-82`) and `unsync` clears it on the 1 → 0 edge (`:125-129`),
/// so the field names *one* process: in EmuTOS a second one cannot get the lock
/// while the first holds it, it blocks on `wind_spb.sy_wait`
/// (`wm_update`'s `ev_block(MU_MUTEX, …)`, `gemwmlib.c:1785-1786`), and the
/// count is that one owner's nesting depth.
///
/// The field exists for exactly one reader: `sh_ldapp` releases the screen lock
/// of a program that has ended **only when that program held it** —
/// `if (wind_spb.sy_owner == rlr) unsync(&wind_spb)` (`aes/gemshlib.c:668-669`).
/// Our per-slot sweep ([`wm_new_for_slot`]) needs the same test, because unlike
/// EmuTOS's shell loop it runs while another client is still alive (A110).
///
/// A GEMDOS slot index or [`super::pd::KERNEL_CONTEXT`], [`CTL_PD`] for the
/// Control Manager; [`NO_OWNER`] = free (`sy_owner = NULL`).
static UPDATE_OWNER: AtomicUsize = AtomicUsize::new(NO_OWNER);

/// Mouse-control-ownership counter — EmuTOS `ml_ocnt`. `BEG_MCTRL` increments,
/// `END_MCTRL` decrements. While `> 0` the app has taken over the mouse via
/// `wind_update(BEG_MCTRL)`, so the AES frame interception ([`intercept`])
/// switches off and the control rectangle is the whole screen ([`chk_ctrl`]).
/// The edge bodies (menu-tree hold, mouse-owner hand-over) are [`mctrl_own`]'s.
/// Who holds it is [`MCTRL_OWNER`].
static MCTRL_CNT: AtomicI16 = AtomicI16::new(0);

/// Owner of the mouse-control counter — **a NewTOS addition, and a necessary
/// one.** `ml_ocnt` is a bare `WORD` in EmuTOS (`aes/gemfmlib.c:51`); the only
/// process `fm_own` remembers is [`ML_PMOWN`], the *mouse* owner it hands the
/// mouse back to, not the holder of the count. EmuTOS gets away with that
/// because the only place the count is force-drained is `wm_new`
/// (`while (ml_ocnt > 0) wm_update(END_MCTRL)`, `gemwmlib.c:1879-1881`), and
/// `wm_new` runs from the shell loop after the program it swept was the **only**
/// client (`gemshlib.c:676-677`) or at a session boundary.
///
/// [`wm_new_for_slot`] runs while the parent client is alive, so an
/// owner-blind drain would hand the parent's mouse back and un-hold its menu
/// tree in the middle of the parent's own modal dialog. The counter therefore
/// carries the slot of the client that took it to `1`, on the same 0 → 1 /
/// 1 → 0 edges as `fm_own`'s own state (`gemfmlib.c:87-93,99-104`), and the
/// per-slot sweep only drains its own (A110).
static MCTRL_OWNER: AtomicUsize = AtomicUsize::new(NO_OWNER);

/// "No process holds this counter" — EmuTOS's `sy_owner = NULL`
/// (`aes/gemflag.c:128`, `geminit.c:838`). Not a valid identity: identities are
/// GEMDOS slots `< MAX_PROCESSES` plus [`super::pd::KERNEL_CONTEXT`].
const NO_OWNER: usize = usize::MAX;

/// EmuTOS `ml_pmown` (`gemfmlib.c:58`) — the mouse owner `fm_own` found on
/// the outermost `BEG_MCTRL` (`get_mown(&ml_pmown)`, `:92`), handed the mouse
/// back on the matching `END_MCTRL` (`ct_chgown(ml_pmown, &ml_ctrl)`, `:102`).
/// An [`input::Mown`] as [`input::get_mown`] reports it, in
/// [`input::Mown::to_raw`]'s encoding; power-up value: the application chain.
static ML_PMOWN: AtomicUsize = AtomicUsize::new(super::pd::KERNEL_CONTEXT);

/// Global default per-gadget colour words (`WF_DCOLOR`): the topped/untopped
/// defaults new windows inherit (EmuTOS `gl_wtcolor`/`gl_wbcolor`,
/// gemwmlib.c:138-139). Mutable so `wind_set(WF_DCOLOR)` can change them; single
/// boot CPU, so an `UnsafeCell` with a hand-rolled `Sync`, like [`TABLE`].
struct WinColors {
    tcolor: UnsafeCell<[i16; NUM_ELEM]>,
    bcolor: UnsafeCell<[i16; NUM_ELEM]>,
}
// SAFETY: only the single boot CPU touches the AES; no concurrent access.
unsafe impl Sync for WinColors {}

static GL_WCOLORS: WinColors = WinColors {
    tcolor: UnsafeCell::new(GL_WTCOLOR_DEFAULT),
    bcolor: UnsafeCell::new(GL_WBCOLOR_DEFAULT),
};

#[allow(clippy::mut_from_ref)]
fn gl_wtcolor() -> &'static mut [i16; NUM_ELEM] {
    // SAFETY: single-CPU AES; exclusive logical owner.
    unsafe { &mut *GL_WCOLORS.tcolor.get() }
}

#[allow(clippy::mut_from_ref)]
fn gl_wbcolor() -> &'static mut [i16; NUM_ELEM] {
    // SAFETY: single-CPU AES; exclusive logical owner.
    unsafe { &mut *GL_WCOLORS.bcolor.get() }
}

/// Clamp a `WF_COLOR`/`WF_DCOLOR` gadget number to a valid array index. EmuTOS
/// indexes `w_tcolor`/`w_bcolor` with the raw `pinwds[0]` unchecked
/// (gemwmlib.c:1568); we bound it so a bogus gadget can't index out of range and
/// panic (fatal in `no_std`).
fn gadget_index(g: i16) -> usize {
    g.clamp(0, NUM_ELEM as i16 - 1) as usize
}

/// EmuTOS `gl_wtop` itself — the handle of the top-most open window, or `NIL`
/// (−1) when none is open. `wm_init` sets it to `NIL` (`gemwmlib.c:1297`) and
/// `draw_change` re-derives it as `W_TREE[ROOT].ob_tail` (`:1093`), which is
/// `NIL` again once the last window is closed. `WF_OWNER` reports this raw
/// value (`:1587`), unlike `WF_TOP`, which reports `w_top()` — the same value
/// with `NIL` folded to `DESKWH` (`:315-318`).
fn gl_wtop() -> i16 {
    let t = get_top();
    if t == DESKWH {
        NIL
    } else {
        t
    }
}

/// Handle of the top-most open window (highest `z`), or 0 (desktop).
fn get_top() -> i16 {
    let mut best = 0i16;
    let mut best_z = i16::MIN;
    for (i, w) in slots().iter().enumerate() {
        if w.in_use && w.open && w.z >= best_z {
            best_z = w.z;
            best = i as i16;
        }
    }
    best
}

/// Handle of the bottom-most open window (lowest `z`), or −1 (`NIL`) if none is
/// open — EmuTOS `W_TREE[ROOT].ob_head`, the first/oldest child of the window
/// tree root (bottom of the z-order), reported by `WF_OWNER` (gemwmlib.c:1588).
fn get_bottom() -> i16 {
    let mut best = -1i16;
    let mut best_z = i16::MAX;
    for (i, w) in slots().iter().enumerate() {
        if w.in_use && w.open && w.z <= best_z {
            best_z = w.z;
            best = i as i16;
        }
    }
    best
}

/// EmuTOS `wm_mktop` (`gemwmlib.c:1605-1613`) — `ob_order(gl_wtree, h, NIL)`,
/// then `draw_change(h, WS_CURR)` with `WS_PREV` saved around it and restored
/// afterwards. What happens depends on whether `h` is an **open** window:
///
/// * **Open:** the reorder and `draw_change`'s top-request branch — that is
///   [`top_window`], frames repainted and a `WM_REDRAW` when something was
///   uncovered.
/// * **The desktop handle (0):** `ob_order` refuses `ROOT`
///   (`aes/gemoblib.c:1209-1210`) — no reorder; `draw_change` stores the
///   geometry — `WS_PREV := WS_CURR`, `WS_CURR := pt` (which *is* `WS_CURR`),
///   `WS_WORK := wm_calc(WC_WORK, w_kind, pt)` (`:1069-1077`) — and returns
///   before the tree walk and before any drawing, because the desktop window
///   is `VF_INUSE` but never `VF_ISOPEN` (`w_setup`, `:205-213,1290`; the test
///   `:1085-1086`); `wm_mktop` then puts `WS_PREV` back (`:1613`). Net effect:
///   **no pixel, no message, no z-order change, and the desktop window's work
///   rect becomes `wm_calc(WC_WORK, NONE, WS_CURR(0))`** — `(1, 1, W−2, H−2)`
///   for the screen-sized desktop window, every border 1 px for `kind = NONE`
///   (`:1815-1867`) — instead of the `gl_rfull` `wm_init` stored (`:1294`).
///   That is what `wind_get(0, WF_WXYWH)` answers from then on, and it is the
///   clip of the desktop's own rectangle-list walk ([`rlist_owns`], EmuTOS
///   `w_owns`, `:1558`) and, with nothing open, the Control Manager's ctrl
///   rect ([`chk_ctrl`], `w_setactive`, `:326-328`). Rebuilt 1:1 (spec
///   `window-graphics.md` § `wind_set`, "`WF_TOP` on the desktop handle", A80).
/// * **A created-but-unopened handle ≥ 1:** EmuTOS never gets past
///   `ob_order`: `get_par` walks `W_TREE[h].ob_next`, which `w_nilit` left at
///   `NIL`, and reads `tree[-1]` (`aes/gemobjop.c:145-160`) — undefined, no
///   behaviour to rebuild. NewTOS contract: nothing happens (no repaint, no
///   message, no `bump_z`, rectangles untouched); `wm_set` still answers TRUE.
///
/// Before A80 the two non-open cases went through [`top_window`], which drew
/// the *still* topmost window's frame deactivated (and, for handle 0, a 1-px
/// frame around the desktop work area) and burnt a stacking value on a closed
/// slot — 2775 / 1215 pixels where EmuTOS paints none (A59 T5/T6).
fn wm_mktop(handle: i16) {
    let Some(w) = win(handle) else {
        return; // `wind_set` has already refused an invalid handle
    };
    if !w.open {
        if handle == DESKWH {
            // `draw_change`'s geometry store (`:1069-1077`) for a window that is
            // not open: only `WS_WORK` changes observably — `WS_CURR` is stored
            // onto itself and `WS_PREV` is restored by `wm_mktop` (`:1613`).
            let curr = w.curr;
            set_curr(w, curr);
        }
        return;
    }
    top_window(handle);
}

/// Raise `handle` to the top of the stack — EmuTOS `wm_mktop`→`draw_change`,
/// **top-request branch** (`gemwmlib.c:1600-1611,1112-1136`).
///
/// A top request repaints exactly two things and nothing else: the old top's
/// frame with *deactivated* decorations and the new top's frame with activated
/// ones, each clipped to its own owner-rectangle list ([`draw_frame_pieces`] =
/// `w_cpwalk(…, MAX_DEPTH, TRUE)`→`do_walk`, `:363-376,295-311`). In particular
/// it does **not** repaint the desktop background: `start` stays `w_handle`, so
/// `draw_change`'s `if (start == DESKWH) w_drawdesk(&c)` never fires
/// (`:1215-1219`) — and it does not touch any window's client region, because
/// `W_DATA`/`W_WORK` are borderless `G_IBOX`es that `ob_draw` skips
/// (`:67-68,87-88`, `aes/gemoblib.c:663-670`). Both windows keep their content.
///
/// Whether the newly-topped window is asked to redraw depends on the `VF_BROKEN`
/// bookkeeping: if the window was unobstructed before the reorder (`wasclr`)
/// *and* the old top is unobstructed after it (`clrold`), nothing was uncovered
/// and `draw_change` returns after the two border draws (`:1128-1135`).
/// Otherwise it falls through to `w_update(start = w_handle, …, stop = DESKWH)`,
/// whose single loop turn re-draws that one window's border and calls
/// `w_redraw`, posting one `WM_REDRAW` for `pt ∩ union(w_rlist)`
/// ([`send_redraw_in`], `:1004-1048,887-907`) — with the window now on top and
/// `pt` its own shadowed rectangle, that is its whole `WS_TRUE` rectangle
/// clipped to the screen: work area, frame and drop shadow.
fn top_window(handle: i16) {
    let old_top = get_top(); // current top before the bump (may equal handle)
    // Not a top request at all: `draw_change` returns immediately when the
    // window is already `W_TREE[ROOT].ob_tail` (`gemwmlib.c:1117-1119`).
    if old_top == handle {
        return;
    }
    // `wasclr` is read *before* the reorder, from the `VF_BROKEN` flag the
    // previous `newrect` pass left (`gemwmlib.c:1066`).
    let wasclr = !is_broken(handle);
    if let Some(w) = win(handle) {
        w.z = bump_z();
    }
    let Some(backend) = host().and_then(|h| h.raster()) else {
        return;
    };
    // Counting cursor bracket around the frame repaints — our cursor is a
    // save-under sprite, so painting under a shown sprite would leave its
    // backing store stale (see [`compose`]; EmuTOS has the same `gsx_moff`
    // bracket inside `w_update`, `gemwmlib.c:1011/1047`).
    crate::vdi::cursor_hide(backend);
    // "draw oldtop covered with deactivated borders" (`gemwmlib.c:1120-1126`).
    // `get_top` answers `DESKWH` when no window was topped (EmuTOS `w_top`,
    // `:315-318`); the desktop window has no frame to deactivate, and EmuTOS
    // then takes `clrold = TRUE` (`:1125`).
    let clrold = if old_top != DESKWH {
        draw_frame_pieces(old_top, false, backend, None);
        // `clrold` is read *after* the rectangle lists were rebuilt with the
        // new order (`:1122`), i.e. with `handle` already above `old_top`.
        !is_broken(old_top)
    } else {
        true
    };
    // The new top's frame, activated — `w_cpwalk(gl_wtop, 0, MAX_DEPTH, TRUE)`
    // in the clear case (`:1133`), `w_update`'s `w_cpwalk` in the other
    // (`:1029`); both draw the same frame clipped to the same rectangle list.
    draw_frame_pieces(handle, true, backend, None);
    crate::vdi::cursor_show(backend);
    if !(clrold && wasclr) {
        if let Some(r) = win(handle).map(|w| w.curr) {
            // The damage rectangle `draw_change` reaches its tail with, on the
            // top path: `c` is the window's `WS_CURR` (`:1069-1070`), the
            // `:1198-1201` union with `gl_wtop`'s current rectangle is a no-op
            // because `gl_wtop` *is* this window, and `:1213-1215` then adds
            // the drop shadow unconditionally before handing `c` to
            // `w_update`. Passing the bare `curr` instead left the frame and
            // the shadow — the part of the window that is genuinely new on
            // screen — out of the `WM_REDRAW` the window is about to get.
            send_redraw_in(handle, plus_shadow(r));
        }
    }
}

/// `VF_BROKEN` (`aes/gemlib.h:62`) — "the window is overlapped, can't be
/// blitted": `newrect`/`mkrect`/`brkrct` set the flag on a window whenever a
/// window **above** it breaks one of its owner rectangles
/// (`aes/gemwrect.c:100-160,163-200`), and `draw_change` clears + rebuilds the
/// whole set with `everyobj(gl_wtree, ROOT, NIL, newrect, …)` (`gemwmlib.c:1088`)
/// before reading it. Since the flag is a pure function of the current z-order
/// and geometry, it is computed on demand here instead of being cached: a
/// window is broken exactly when some open window above it overlaps its border
/// rectangle.
///
/// Both rectangles are `WS_TRUE` ([`ws_true`]), not `WS_CURR`: the list `mkrect`
/// breaks consists of `WS_TRUE` pieces (`newrect`, `aes/gemwrect.c:203`) and the
/// rectangle it breaks them with is `WS_TRUE` too (`:190`). A window that
/// overlaps another one **with its drop shadow alone** therefore already breaks
/// it, and `draw_change` reads that flag to decide whether the topped window
/// needs a `WM_REDRAW` at all (`gemwmlib.c:1066,1121-1135`).
fn is_broken(handle: i16) -> bool {
    let Some((rect, myz)) = win(handle).filter(|w| w.open).map(|w| (ws_true(w.curr), w.z)) else {
        return false;
    };
    slots().iter().any(|ow| {
        ow.in_use && ow.open && ow.z > myz && intersect_grect(ws_true(ow.curr), rect).is_some()
    })
}

/// `handle`'s **owner rectangle list** — EmuTOS `D.w_win[wh].w_rlist`: the
/// window's own `WS_TRUE` rectangle with the rectangle of every window above it
/// broken out of it, rebuilt by `newrect`/`mkrect`/`brkrct`
/// (`aes/gemwrect.c:64-205`) at the top of every `draw_change`
/// (`aes/gemwmlib.c:1129`), i.e. before anything is drawn. Returns the piece
/// count; `out[..n]` holds them.
///
/// This is *the* list the whole window manager clips against: `do_walk`
/// ([`walk_rects`]) walks it for both `w_drawdesk` and `w_cpwalk`, and
/// `WF_FIRSTXYWH` reports it intersected with the work rectangle
/// ([`compute_rlist`]).
///
/// **The subtraction order is contract: ascending z, one covering window at a
/// time.** `draw_change` rebuilds every list with a single
/// `everyobj(gl_wtree, ROOT, NIL, newrect, …)` sweep (`gemwmlib.c:1089`), and
/// `everyobj` walks the `W_TREE` sibling chain `ob_head` → `ob_tail`
/// (`aes/gemobjop.c:76-131`). That chain **is** the z-order by construction:
/// `w_obadd` appends a newly opened window at `ob_tail` (`:186-201`, from
/// `wm_open` `:1425`) and `wm_mktop` moves a topped one there with
/// `ob_order(gl_wtree, w_handle, NIL)` (`:1605-1610`, `aes/gemoblib.c:1213-1240`,
/// the `new_pos == NIL` branch). Inside the sweep, `newrect(tree, wh)` resets
/// `wh`'s own list to its unbroken `WS_TRUE` rect and then lets
/// `everyobj(tree, ROOT, wh, mkrect, …)` break the list of every window *below*
/// `wh` with `wh`'s rect (`gemwrect.c:165-205`) — so read across the sweep, a
/// window's list is cut once per window above it, in ascending z-order of those
/// covering windows.
///
/// Not a detail: [`subtract`] replaces a surviving piece with its sub-pieces on
/// the spot and never revisits a piece an earlier cut removed, so cutting with
/// `A` then `B` does not in general give the same fragments as `B` then `A`,
/// even though both remove the same union. Measured: the same three windows
/// opened 1,2,3 leave window 1 with **four** pieces and opened 1,3,2 with
/// **five** (`tosdump/cases/aes-r3-wind-state.tsv` §D against
/// `tosdump/reference/EMUTOS14-AES-r3.BIN`, cases 439–574; regression test
/// `tos-api/tests/aes_window_rectlist_zorder.rs`). Iterating the slot array in
/// slot order answered the four-piece list for both.
///
/// [`DESKWH`] (the desktop window) is the **bottom** of the stack: its own
/// rectangle is the whole screen (`w_setsize(WS_CURR, DESKWH, &gl_rscreen)`,
/// `aes/gemwmlib.c:1290`) and every open window is above it (`i16::MIN` z), so
/// the loop subtracts them all. The menu row is kept out by `do_walk`'s
/// `gl_rfull` clip, not by this list — exactly as in EmuTOS.
///
/// A closed or invalid handle owns nothing (`newrect` returns early for a
/// zero-size window, `aes/gemwrect.c:189-192`).
///
/// **Both** rectangles are `WS_TRUE` ([`ws_true`]) — the window's own start
/// rectangle *and* the rectangle each higher window breaks it with. `newrect`
/// uses `w_getsize(WS_TRUE, …)` for exactly these two roles and no other:
///
/// ```c
/// w_getsize(WS_TRUE, wh, &gl_mkrect.o_gr);            /* :190 — the breaker */
/// ...
/// everyobj(tree, ROOT, wh, mkrect, 0, 0, MAX_DEPTH);
/// new = get_orect();
/// w_getsize(WS_TRUE, wh, &new->o_gr);                 /* :203 — our own    */
/// ```
///
/// Growing only one of the two would be worse than growing neither: taking the
/// shadow off the breaker leaves the strip under a neighbour's shadow claimed
/// by two windows at once, and taking it off our own start rectangle drops the
/// strip our own frame paints out of the list that clips that very paint.
fn border_pieces(handle: i16, out: &mut [Grect; RCAP]) -> usize {
    let (rect, myz) = match win(handle) {
        Some(w) if handle == DESKWH => (ws_true(w.curr), i16::MIN),
        Some(w) if w.open => (ws_true(w.curr), w.z),
        _ => return 0,
    };
    *out = [Grect::default(); RCAP];
    out[0] = rect;
    let mut n = 1;
    // Collect the covering windows first, then cut with them in **ascending
    // z-order** — never in slot/handle order. Which order it is decides how
    // many pieces come out and where the cuts fall, because [`subtract`]
    // re-splits every piece that survived an earlier cut and never revisits one
    // an earlier cut removed.
    let mut cover = [(0i16, Grect::default()); NUM_WIN];
    let mut c = 0usize;
    for ow in slots().iter() {
        if ow.in_use && ow.open && ow.z > myz {
            cover[c] = (ow.z, ws_true(ow.curr));
            c += 1;
        }
    }
    // Insertion sort — `c` is at most fifteen, and it is stable, so windows
    // that somehow share a `z` keep slot order rather than an arbitrary one.
    for i in 1..c {
        let mut j = i;
        while j > 0 && cover[j - 1].0 > cover[j].0 {
            cover.swap(j - 1, j);
            j -= 1;
        }
    }
    for (_, r) in cover.iter().take(c) {
        n = subtract(out, n, *r);
    }
    n
}

/// EmuTOS `do_walk` (`aes/gemwmlib.c:286-312`), reduced to the rectangles it
/// would `gsx_sclip` and draw in:
///
/// ```c
/// if (pc) rc_intersect(&gl_rfull, pc); else pc = &gl_rfull;
/// for (po = D.w_win[wh].w_rlist; po; po = po->o_link) {
///     rc_copy(&po->o_gr, &t);
///     if (rc_intersect(pc, &t)) { gsx_sclip(&t); ob_draw(tree, obj, depth); }
/// }
/// ```
///
/// `pc` = `None` is the C `pc == NULL` case (clip to `gl_rfull` alone). Returns
/// the count; `out[..n]` are the clip rectangles, each of which the caller draws
/// its tree in once. An empty result means the caller draws nothing at all —
/// which is the whole point: a window that owns no visible piece, or a desktop
/// wholly covered by windows, paints nothing.
///
/// `do_walk` itself adds **no** drop shadow to `pc`: `w_drawdesk` hands its
/// caller's rectangle straight down (`gemwmlib.c:340-359`), and only `w_cpwalk`
/// grows the clip — which is why the growth lives in [`draw_frame_pieces`] and
/// not here. The shadow still reaches the walk, through the `WS_TRUE` pieces
/// [`border_pieces`] produces.
fn walk_rects(wh: i16, pc: Option<Grect>, out: &mut [Grect; RCAP]) -> usize {
    // `rc_intersect(&gl_rfull, pc)` — `gl_rfull` is the screen minus the menu
    // row (`aes/gemgraf.c:423-424`, set at `gsx_start` whether or not a menu bar
    // is installed), so no `do_walk` ever touches the menu strip.
    let full = desktop_work();
    let pc = match pc {
        Some(p) => match intersect_grect(p, full) {
            Some(t) => t,
            None => return 0,
        },
        None => full,
    };
    let mut list = [Grect::default(); RCAP];
    let n = border_pieces(wh, &mut list);
    let mut m = 0usize;
    for r in list.iter().take(n) {
        if let Some(t) = intersect_grect(*r, pc) {
            out[m] = t;
            m += 1;
        }
    }
    m
}

/// Draw `handle`'s frame once per visible piece of its owner rectangle list —
/// EmuTOS `w_cpwalk(wh, 0, MAX_DEPTH, …)`→`do_walk` (`gemwmlib.c:363-379`,
/// `:286-312`), which walks `D.w_win[wh].w_rlist` and `ob_draw`s the window
/// element tree inside each rectangle. Clipping the draw to the rectangle list
/// is what stops a window that is *below* another one from painting its frame
/// over it. `istop` selects the activated / deactivated decorations
/// (`w_bldactive`'s `istop`, `:495-560`).
///
/// `pc` is `w_cpwalk`'s clip, and the two branches differ by the drop shadow
/// (`gemwmlib.c:365-375`):
///
/// ```c
/// if (usetrue)
///     w_getsize(WS_TRUE, wh, &c);     /* the window plus its shadow */
/// else
/// {
///     gsx_gclip(&c);                  /* the damage rect w_update set */
///     c.g_w += DROP_SHADOW_SIZE;      /* add in drop shadow   */
///     c.g_h += DROP_SHADOW_SIZE;
/// }
/// ```
///
/// - `Some(damage)` is the `usetrue == FALSE` call inside `w_update`, which runs
///   under `gsx_sclip(pt)` and picks that clip back up with `gsx_gclip`
///   (`:1029-1031`) — so the damage rectangle is grown by the shadow here
///   ([`plus_shadow`]). `draw_change` already added one shadow to the damage on
///   the way in (`:1213-1215`); this second one is `w_cpwalk`'s own and is not
///   a substitute for it. Without it a window whose damage rectangle ends at
///   its border keeps its shadow unpainted.
/// - `None` is the `usetrue == TRUE` call. Its clip is `WS_TRUE(wh)`, which is
///   already implied: every piece [`border_pieces`] produces starts from
///   `ws_true(curr)` and is only ever cut down, so intersecting with it again
///   cannot change anything.
fn draw_frame_pieces(handle: i16, istop: bool, backend: &dyn RasterBackend, pc: Option<Grect>) {
    let mut pieces = [Grect::default(); RCAP];
    let n = walk_rects(handle, pc.map(plus_shadow), &mut pieces);
    if n == 0 {
        return;
    }
    let Some(w) = win(handle) else {
        return;
    };
    for r in pieces.iter().take(n) {
        draw_frame(w, istop, backend, rect_clip(r.x, r.y, r.w, r.h));
    }
}

/// The registered desktop-background object tree (EmuTOS `gl_newdesk`,
/// `gemwmlib.c:54`): the base VA of the desktop application's own `OBJECT`
/// array, set via `wind_set(DESKWH, WF_NEWDESK, tree, 1, root)`. `0` = none →
/// [`paint_bg_clip`] draws the AES-internal fallback root instead.
///
/// The registering caller records its origin + ABI in [`NEWDESK_APP`] /
/// [`NEWDESK_ABI`] so the later background composite reads the tree at the right
/// record width and treats a `G_USERDEF` callback at the right privilege. The
/// NewTOS desktop registers its own **native (LP64) kernel-built** `g_screen`
/// (drawn via `render_tree_lp64`); an EL0 GEM app (TeraDesk) registers an
/// **app-provided** tree in EL0 memory (drawn via `render_tree_app`). It stays
/// stable for its owner's lifetime (the arena is never reallocated), so holding
/// the raw pointer across draws is sound.
static NEWDESK_TREE: AtomicU64 = AtomicU64::new(0);

/// The object index within [`NEWDESK_TREE`] that the background composite draws
/// from (EmuTOS `gl_newroot`, `gemwmlib.c:55`; the desktop passes its `DROOT`).
static NEWDESK_ROOT: AtomicI16 = AtomicI16::new(0);

/// `true` iff the tree in [`NEWDESK_TREE`] was supplied by an **EL0 GEM app**
/// (e.g. TeraDesk via `wind_set(WF_NEWDESK)`), `false` for the kernel/desktop's
/// own native `g_screen`. Recorded in [`set_newdesk`] from whether a user
/// process is running at registration time (`process::current()` non-null).
/// Drives [`paint_bg_clip`]: an app tree is drawn as **not** kernel-built so a
/// `G_USERDEF`'s EL0 `ub_code` is an EL0 callout from `ob_user` instead of being
/// called at EL1 (the WF_NEWDESK crash: TeraDesk `ub_bckbox` → PXN instruction
/// abort).
static NEWDESK_APP: AtomicBool = AtomicBool::new(false);

/// The ABI ([`Abi`] as `u8`) of the caller that registered [`NEWDESK_TREE`],
/// captured in [`set_newdesk`] via `current_abi()`. The background composite
/// runs later — possibly when a different (or no) process is current — so the
/// tree's record width must be replayed from this captured value rather than
/// re-read from `current_abi()` at draw time. A kernel/desktop registration
/// records `Lp64`. Only consulted when [`NEWDESK_APP`] is `true`.
static NEWDESK_ABI: AtomicU8 = AtomicU8::new(Abi::Lp64 as u8);

/// The process slot that registered [`NEWDESK_TREE`] — `-1` for a kernel
/// (EL1) registrant, whose tree is a kernel VA, else the GEMDOS slot of the
/// EL0 caller (`process::slot_for(process::current())` inside the
/// `wind_set` trap), captured in [`set_newdesk`] exactly as [`LiveStr::slot`]
/// is for a title pointer (A83). The composite reads the tree **through this
/// slot's mapping** ([`draw_desk_tree`] → `render_tree_app(…, owner, …)`,
/// A97) — the tree's records, sub-structs, strings and rasters are all user
/// VAs of that process, and the composite runs from other processes' AES
/// calls and from kernel code, under other `TTBR0`s. A registrant that has
/// ended is swept with its windows ([`wm_new_for_slot`], [`wm_init`]).
static NEWDESK_OWNER: AtomicI32 = AtomicI32::new(-1);

/// [`NEWDESK_OWNER`] as an `Option`: `Some(slot)` for an EL0 registrant.
fn newdesk_owner() -> Option<usize> {
    usize::try_from(NEWDESK_OWNER.load(Ordering::Acquire)).ok()
}

/// Forget the registered desktop tree — `gl_newdesk = NULL`
/// (`gemwmlib.c:1300`): tree, root, origin, ABI and owner.
fn clear_newdesk() {
    NEWDESK_TREE.store(0, Ordering::Release);
    NEWDESK_ROOT.store(0, Ordering::Release);
    NEWDESK_APP.store(false, Ordering::Release);
    NEWDESK_ABI.store(Abi::Lp64 as u8, Ordering::Release);
    NEWDESK_OWNER.store(-1, Ordering::Release);
}

/// `wind_set(DESKWH, WF_NEWDESK, …)` handler (EmuTOS `wm_set`,
/// `gemwmlib.c:1685-1689`): register (non-null tree) or un-register (null tree)
/// the desktop's background object tree + root object. Never fails.
///
/// The tree pointer arrives by the same dual convention as `WF_NAME`
/// ([`wf_str_ptr`], D-094), and the **root companion word moves with it**
/// (`docs/api/aes/window-graphics.md`, "Pointers in `addr_in[]`" / edge case
/// 16, A81):
///
/// * **native** (`AIN_LEN >= 1`): tree in `addr_in[0]`, root in `int_in[2]` —
///   what the native desktop sends (`programs/desktop/src/lib.rs::wind_set_newdesk`)
///   and gemlib's overlay bindings after A81;
/// * **packed** (`AIN_LEN == 0`): tree packed at the caller's own pointer width
///   from `int_in[2]`, root in the word right after it, `int_in[2 + ptr_words]`
///   ([`Aespb::ptr_words`]) — `int_in[4]` for a 2-word ILP32 pointer, bit-exact
///   with EmuTOS's `gl_newroot = pinwds[2]` after `gl_newdesk = *(OBJECT **)
///   pinwds` (`aes/gemwmlib.c:1685-1689`, `pinwds = &int_in[2]`,
///   `gemsuper.c:345-346`); `int_in[6]` for a 4-word LP64 pointer, which the
///   caller must declare (`IN_LEN >= 7`) or the dispatch-local copy reads 0.
///
/// Before A81 the packed arm read a fixed `int_in[4]` for every caller width;
/// for an LP64 caller that is the packed pointer's own bits 32..47, so a
/// non-zero root there was the pointer's upper half, never the root
/// (`TASK/A81.md`: tree `0x0000_0001_0177_E4C0`, root 2).
fn set_newdesk(pb: &Aespb) {
    let tree = wf_str_ptr(pb);
    let root = if pb.ain_len() >= 1 { pb.intin(2) } else { pb.intin(2 + pb.ptr_words()) };
    if tree == 0 {
        NEWDESK_TREE.store(0, Ordering::Release);
        NEWDESK_OWNER.store(-1, Ordering::Release);
    } else {
        // Capture the tree's origin + ABI + owner at registration time; the
        // background composite runs later, when a different (or no) process
        // may be current. A running user process (`current()` non-null)
        // means an EL0 GEM app is registering its own background tree — its
        // `G_USERDEF` `ub_code` lives in EL0 and must not be up-called from
        // EL1, and its tree is read through its own mapping (the owner slot,
        // A97); a null `current()` is the kernel/desktop registering its
        // native `g_screen`. `current_abi()` records the record width now
        // (the app is current at this instant).
        let is_app = !crate::bdos::process::current().is_null();
        NEWDESK_APP.store(is_app, Ordering::Release);
        NEWDESK_ABI.store(crate::bdos::process::current_abi() as u8, Ordering::Release);
        let owner = process::slot_for(process::current()).and_then(|s| i32::try_from(s).ok());
        NEWDESK_OWNER.store(owner.unwrap_or(-1), Ordering::Release);
        // NEWDESK_ROOT is taken verbatim from caller input (EmuTOS trusts the
        // desktop's `gl_newroot`/`DROOT` the same way, gemwmlib.c:1685). A
        // negative index is defused downstream (`render_tree_*` bail on
        // `start < 0`); an out-of-range positive
        // index cannot be validated here without the tree's object count, which
        // WF_NEWDESK does not carry — left as the caller's contract.
        NEWDESK_ROOT.store(root, Ordering::Release);
        NEWDESK_TREE.store(tree, Ordering::Release);
    }
}

/// Build the AES-internal background **fallback root** — EmuTOS
/// `gl_wtree[ROOT]` (= `W_TREE[ROOT]`), the object `w_drawdesk` draws when no
/// desktop tree is registered ([`NEWDESK_TREE`] == 0, `gemwmlib.c:340-359`).
///
/// `wm_init` builds that root as a `G_BOX` with `NONE` flags and `NORMAL` state
/// and copies its `ob_spec` **from the AES's own built-in DESKTOP tree**
/// (`W_TREE[ROOT].ob_spec = rs_trees[DESKTOP]->ob_spec`,
/// `gemwmlib.c:1268-1271`), i.e. the resource constant `4419L` = `0x1143` —
/// pattern 4 in colour 3, the classic green dither (`gem_rsc.c:551-556`) — with
/// its low byte replaced by the `.INF` `#Q` background if there was one
/// (`set_aes_background`, `geminit.c:635-641`, which runs *before* `wm_start`,
/// `geminit.c:719-730`). [`super::init::desktop_root_obspec`] holds exactly that
/// word, so read it here rather than caching a copy.
///
/// It is **not** white: the white screen belongs to `sh_draw(cmd, TRUE)`, which
/// swaps the DESKTOP root's `ob_spec` for `CLEAR_SCREEN`, draws once and puts
/// the old spec straight back (`gemshlib.c:269-286`) — see
/// [`super::shell::sh_draw_clear`].
///
/// Its geometry is the desktop window's `WS_CURR` = `gl_rscreen`, the whole
/// screen (`gemwmlib.c:1290`); built fresh per call so it always spans the
/// current framebuffer.
fn fallback_root(w: i16, h: i16) -> Object {
    Object {
        ob_next: -1,
        ob_head: -1,
        ob_tail: -1,
        ob_type: 20, // G_BOX
        ob_flags: 0,
        ob_state: 0,
        ob_spec: super::init::desktop_root_obspec(),
        ob_x: 0,
        ob_y: 0,
        ob_width: w,
        ob_height: h,
    }
}

/// Paint the desktop background within `clip` — NewTOS's `w_drawdesk`
/// (EmuTOS `gemwmlib.c:340-359`), which is a single
/// `do_walk(DESKWH, tree, root, depth, pc)` (`gemwmlib.c:286-312`):
///
/// ```c
/// if (pc) rc_intersect(&gl_rfull, pc); else pc = &gl_rfull;
/// for (po = D.w_win[wh].w_rlist; po; po = po->o_link) {
///     rc_copy(&po->o_gr, &t);
///     if (rc_intersect(pc, &t)) { gsx_sclip(&t); ob_draw(tree, obj, depth); }
/// }
/// ```
///
/// So the background is **not** painted over the damage rectangle. It is painted
/// over the damage rectangle intersected with `gl_rfull` (the screen minus the
/// menu row) and then with each piece of the **desktop window's owner rectangle
/// list** — its own rectangle minus every window above it, rebuilt by `newrect`
/// (`gemwrect.c:165-205`) at the top of `draw_change` (`gemwmlib.c:1129`), i.e.
/// before anything is drawn. An open window owns its rectangle; the desktop does
/// not, so `w_drawdesk` can never paint inside one. What a window shows is
/// restored by its owner in answer to the `WM_REDRAW` that `w_update`/`w_redraw`
/// post (`gemwmlib.c:1004-1048,887-907`) — the AES does not paint it.
///
/// Painting the whole clip instead put the desktop background — box *and* icons —
/// inside every window a damage rectangle reached across, on `wind_open`,
/// `wind_close`, `wind_set(WF_CURRXYWH)`, the mover drag and
/// `form_dial(FMD_FINISH)`, until the owner answered its `WM_REDRAW`.
///
/// Runs **inside** `compose`/`redraw_all` (the AES SVC handler), so it must not
/// re-enter the AES trap: [`draw_desk_tree`] draws straight through the backend.
fn paint_bg_clip(backend: &dyn RasterBackend, clip: ClipRect) {
    let mut pieces = [Grect::default(); RCAP];
    let n = walk_rects(DESKWH, Some(clip_grect(clip)), &mut pieces);
    for r in pieces.iter().take(n) {
        draw_desk_tree(backend, rect_clip(r.x, r.y, r.w, r.h));
    }
}

/// `ob_draw(tree, obj, depth)` for the desktop background tree, clipped to
/// `clip` — the body of `w_drawdesk`'s `do_walk` iteration
/// (`gemwmlib.c:340-359,306-308`).
///
/// If a desktop tree is registered ([`NEWDESK_TREE`] != 0, via
/// `wind_set(DESKWH, WF_NEWDESK, …)`), draw that tree from its root
/// ([`NEWDESK_ROOT`], the desktop's `DROOT`) at **`MAX_DEPTH`** — the `DROOT`
/// background box **and every icon child** — exactly as EmuTOS draws
/// `gl_newdesk` at `MAX_DEPTH`. Otherwise draw the AES-internal [`fallback_root`]
/// — the built-in DESKTOP root's own box — at depth 0, exactly as EmuTOS draws
/// `gl_wtree` at depth 0. Both go through the ordinary object
/// manager ([`render_tree_lp64`] = the non-trapping `ob_draw` engine), clipped
/// to `clip` — never a bespoke fill.
fn draw_desk_tree(backend: &dyn RasterBackend, clip: ClipRect) {
    let cw = (clip.xmx - clip.xmn + 1).max(0);
    let ch = (clip.ymx - clip.ymn + 1).max(0);
    if cw == 0 || ch == 0 {
        return;
    }
    let tree = NEWDESK_TREE.load(Ordering::Acquire);
    if tree != 0 {
        let root = NEWDESK_ROOT.load(Ordering::Acquire);
        if NEWDESK_APP.load(Ordering::Acquire) {
            // App-supplied background tree (EL0 GEM app, e.g. TeraDesk): read it
            // at the ABI captured at registration and treat it as NOT
            // kernel-built, so a `G_USERDEF`'s EL0 `ub_code` runs as an EL0
            // callout in the current process (`ob_user`; ARCH03 3c E10: also
            // when the registrant is not the current process, as in EmuTOS)
            // rather than at EL1 (→ PXN instruction abort at the app
            // callback's VA, the WF_NEWDESK crash).
            //
            // The tree is a user VA of the registrant, and this runs under
            // whatever `TTBR0` the current caller has: read it **through the
            // owner's mapping** (A97, the A83 model). The root record must
            // resolve up front — if the owner has no mapping there any more
            // (the block freed, the slot re-seeded), the tree counts as not
            // registered and the AES fallback root is drawn instead; inside
            // the draw every further address resolves the same way.
            let abi = Abi::from_u8(NEWDESK_ABI.load(Ordering::Acquire));
            let owner = newdesk_owner();
            let root_va = tree.wrapping_add(root.max(0) as u64 * super::object::record_stride(abi));
            let readable = match owner {
                Some(slot) => owner_kva(slot, root_va).is_some(),
                None => true,
            };
            if readable {
                // SAFETY: `tree` is the app's registered background-tree base —
                // a valid OBJECT array (at `abi` width) in the owner's address
                // space, read through that owner's mapping; `root` in range
                // (caller contract, see `set_newdesk`).
                unsafe {
                    render_tree_app(
                        tree as *const Object,
                        root,
                        MAX_DEPTH,
                        clip.xmn,
                        clip.ymn,
                        cw,
                        ch,
                        abi,
                        owner,
                        backend,
                    );
                }
                return;
            }
            draw_fallback_root(backend, clip, cw, ch);
        } else {
            // Kernel/desktop-built native `g_screen` (LP64 records); a
            // `G_USERDEF` here is a real EL1 callback (`kernel_built = true`).
            // SAFETY: `tree` is the desktop's `g_screen` base, registered via
            // WF_NEWDESK; a valid native OBJECT array kept stable for the
            // desktop's lifetime, `root` in range.
            unsafe {
                render_tree_lp64(tree as *const Object, root, MAX_DEPTH, clip.xmn, clip.ymn, cw, ch, backend);
            }
        }
    } else {
        draw_fallback_root(backend, clip, cw, ch);
    }
}

/// Draw the AES-internal [`fallback_root`] at depth 0 within `clip` — the
/// `gl_newdesk == NULL` arm of `w_drawdesk` (`gemwmlib.c:340-359`).
fn draw_fallback_root(backend: &dyn RasterBackend, clip: ClipRect, cw: i16, ch: i16) {
    let fb = fallback_root(backend.width() as i16, backend.height() as i16);
    // SAFETY: `fb` is a single valid native OBJECT (root index 0, no children).
    unsafe {
        render_tree_lp64(&fb as *const Object, 0, 0, clip.xmn, clip.ymn, cw, ch, backend);
    }
}

/// A `ClipRect` for the rectangle `[x, x+w) × [y, y+h)`.
fn rect_clip(x: i16, y: i16, w: i16, h: i16) -> ClipRect {
    ClipRect { xmn: x, ymn: y, xmx: x + w - 1, ymx: y + h - 1 }
}

/// The inverse of [`rect_clip`] — a `ClipRect` (inclusive bounds, the VDI
/// convention) as a `GRECT` (origin + size, the AES convention), so the
/// rectangle algebra EmuTOS does on `GRECT`s can be done on a clip.
fn clip_grect(c: ClipRect) -> Grect {
    Grect {
        x: c.xmn,
        y: c.ymn,
        w: (c.xmx - c.xmn + 1).max(0),
        h: (c.ymx - c.ymn + 1).max(0),
    }
}

/// Repaint the desktop and every open window back-to-front (painter's
/// algorithm — front windows overwrite the ones they cover). The simple
/// stand-in for per-window rectangle-list clipping when redrawing frames;
/// the rectangle list (`WF_FIRSTXYWH`) still drives what the *app* redraws.
fn redraw_all(backend: &dyn RasterBackend) {
    compose(backend, ClipRect::full(backend.width(), backend.height()));
}

/// The desktop-background + window-frame composite, confined to `clip`. Paints
/// the desktop background (painter hook) then every open window's frame
/// back-to-front (painter's algorithm — front windows overwrite the ones they
/// cover), each clipped to `clip`. [`redraw_all`] passes the full-device clip;
/// [`redraw_clip`] passes a damage rectangle so a move/size settle repaints only
/// the affected area instead of the whole screen. The window **content** (item
/// objects) is re-laid by the owner afterwards, exactly as in the full path.
fn compose(backend: &dyn RasterBackend, clip: ClipRect) {
    compose_except(backend, clip, None);
}

/// [`compose`] with one window's frame left **out** of the walk — the
/// `moved` case of `draw_change`'s tail: after `w_move` has blitted the top
/// window to its new place, `w_update(…, moved = TRUE)` steps over `gl_wtop`
/// (`gemwmlib.c:1025`) and nothing repaints its frame, so the blitted pixels —
/// title dither included, at the phase the blit carried them to — stay as
/// they are. Painting that frame again inside the damage rectangle would
/// re-align the dither there and leave a seam against the rest of the bar.
/// Every other window's frame and the desktop background are painted as in
/// [`compose`].
fn compose_except(backend: &dyn RasterBackend, clip: ClipRect, skip: Option<i16>) {
    // Bracket the whole window redraw with the **counting** cursor semaphore,
    // exactly as EmuTOS `w_update` wraps the frame walk in `gsx_moff()` /
    // `gsx_mon()` (gemwmlib.c:1011/1047). The cursor is a save-under sprite:
    // painting the desktop background + frames over the shown sprite would
    // leave its backing store stale, so the next cursor move restores old
    // pixels — the reported rectangular cursor "dropping". Hide it once here
    // (its backing is restored first), repaint, then show it on top of the
    // fresh pixels. The count makes any nested composite (an outer
    // `graf_mouse(M_OFF)` desktop bracket, or a re-entrant redraw) safe.
    crate::vdi::cursor_hide(backend);
    paint_bg_clip(backend, clip);
    let top = get_top();
    // Selection sort by ascending z, drawing each open window once.
    let mut drawn = [false; NUM_WIN];
    loop {
        let mut pick = usize::MAX;
        let mut pick_z = i16::MAX;
        for (i, w) in slots().iter().enumerate() {
            if w.in_use && w.open && !drawn[i] && w.z <= pick_z {
                // pick the lowest z not yet drawn; tie-break stable
                if pick == usize::MAX || w.z < pick_z {
                    pick = i;
                    pick_z = w.z;
                }
            }
        }
        if pick == usize::MAX {
            break;
        }
        drawn[pick] = true;
        if skip == Some(pick as i16) {
            continue;
        }
        let istop = pick as i16 == top;
        // `w_update`'s per-window step: `gsx_sclip(pt); w_cpwalk(top, 0,
        // MAX_DEPTH, FALSE)` (`gemwmlib.c:1029-1030`) — the frame is drawn once
        // per piece of *that window's* owner rectangle list inside the damage
        // rectangle, never over the whole damage rectangle. Without the list a
        // covered window repainted its frame on top of the window covering it.
        draw_frame_pieces(pick as i16, istop, backend, Some(clip_grect(clip)));
    }
    crate::vdi::cursor_show(backend);
}

/// Damage-scoped repaint of the desktop background + every window frame, clipped
/// to `clip` (a border-rect union). The public entry the desktop drives after a
/// solid window move/resize (WM_MOVED/WM_SIZED) and the programmatic WF_CXYWH
/// path use, in place of [`redraw`] (whole-screen `redraw_all`) — only the union
/// of the window's old and new border rects is repainted, which removes the
/// full-screen flicker. The window content is re-laid by the desktop
/// (`do_wredraw`) / by the `send_redraw` WM_REDRAW that follows.
pub fn redraw_clip(x: i16, y: i16, w: i16, h: i16) {
    if w <= 0 || h <= 0 {
        return;
    }
    if let Some(backend) = host().and_then(|h| h.raster()) {
        let cr = ClipRect { xmn: x, ymn: y, xmx: x + w - 1, ymx: y + h - 1 };
        compose(backend, cr);
    }
}

/// The bounding rectangle enclosing both `a` and `b` — the damage region of a
/// window move/resize (old border rect ∪ new border rect).
fn union_grect(a: Grect, b: Grect) -> Grect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Grect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// The intersection of `a` and `b`, or `None` when they don't overlap — the
/// per-window clip EmuTOS's `w_redraw` computes with `rc_intersect`
/// (`gemwmlib.c:897,903`).
fn intersect_grect(a: Grect, b: Grect) -> Option<Grect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.w).min(b.x + b.w);
    let y1 = (a.y + a.h).min(b.y + b.h);
    (x1 > x0 && y1 > y0).then_some(Grect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// EmuTOS `rc_intersect` (`util/rectfunc.c:61-78`) with **both** halves of its
/// result: `p2` is overwritten with the clipped rectangle *unconditionally*,
/// and the return value says whether that rectangle still has size.
///
/// ```c
/// WORD rc_intersect(const GRECT *p1, GRECT *p2)
/// {
///     tw = min(p2->g_x + p2->g_w, p1->g_x + p1->g_w);
///     th = min(p2->g_y + p2->g_h, p1->g_y + p1->g_h);
///     tx = max(p2->g_x, p1->g_x);
///     ty = max(p2->g_y, p1->g_y);
///     p2->g_x = tx;  p2->g_y = ty;
///     p2->g_w = tw - tx;  p2->g_h = th - ty;
///     return ((tw > tx) && (th > ty));
/// }
/// ```
///
/// The four stores happen **before** the test, so a failed intersection still
/// leaves `g_x = max(p2.x, p1.x)` and `g_y = max(p2.y, p1.y)` behind — and a
/// `g_w`/`g_h` that may be zero or negative. That leftover is not debris: it is
/// what `w_owns` hands back at the end of a rectangle list it did walk
/// ([`Owns::Exhausted`]), which is the only reason this form exists next to
/// [`intersect_grect`]. Callers that never look at a failed rectangle keep
/// using the `Option` form.
fn rc_intersect(p1: Grect, p2: &mut Grect) -> bool {
    let tw = (p2.x + p2.w).min(p1.x + p1.w);
    let th = (p2.y + p2.h).min(p1.y + p1.h);
    let tx = p2.x.max(p1.x);
    let ty = p2.y.max(p1.y);
    p2.x = tx;
    p2.y = ty;
    p2.w = tw - tx;
    p2.h = th - ty;
    tw > tx && th > ty
}

// ── rectangle list (visible pieces) ──────────────────────────────────────

/// Capacity of one owner rectangle list — the size of EmuTOS's **whole**
/// `ORECT` pool, `NUM_ORECT = NUM_WIN * 10` (`aes/gemlib.h:88`, with the
/// source's own "is this enough???"): 160 cells at `NUM_WIN = 16`
/// (`include/sysconf.h:32`, the AES 0x0340 table [`NUM_WIN`] mirrors).
/// `or_start` threads all of them onto the free list `rul` once, in `wm_init`
/// (`aes/gemwrect.c:40-50`, `gemwmlib.c:1258`); `newrect` and `brkrct` hand
/// cells back as they rebuild (`gemwrect.c:174-181`, `:124-127`), so the
/// pool's occupancy is the sum of every window's list, the desktop's
/// included, plus the pieces in flight while one rectangle is being broken.
///
/// **Exhaustion has no path in EmuTOS.** `get_orect` returns the free-list
/// head — `NULL` once there is none (`gemwrect.c:53-61`) — and none of its
/// three callers looks: `mkpiece` writes `rl->o_link = old` and the four
/// rectangle words straight through it (`:68-75`, i.e. into the first twelve
/// bytes of the m68k exception-vector table), `newrect` does the same for the
/// window's own cell (`:201-203`), `wm_init` for the desktop's
/// (`gemwmlib.c:1287-1289`). The list under construction is cut short at the
/// piece that got no cell and its remaining tail is orphaned (`:121` links
/// the `NULL`, `:125` relinks through it). That is undefined behaviour, not a
/// contract — and not even a function of the window arrangement: which
/// `draw_change` runs dry depends on the old lists still held by the windows
/// its sweep has not reached yet (`gemwmlib.c:1089`, `everyobj` in tree
/// order, each `newrect` freeing only its own window's old list first).
///
/// NewTOS keeps one list per window instead of one pool, so the capacity is
/// per list: every arrangement whose lists fit EmuTOS's pool (their sum at
/// most 160) fits here piece for piece, and a single list that does not fit
/// here (more than 160) is beyond EmuTOS's pool with certainty. On that
/// overflow [`put`] **stops with a panic** that names the pool and the lines
/// it stands in for — the one honest translation of "write through the
/// `NULL` that `get_orect` returned": silent corruption cannot be
/// reimplemented, and dropping the piece would be the silent loss of redraw
/// area this constant's old value of 16 caused (A21).
const RCAP: usize = NUM_WIN * 10;

/// Append `r` to the list under construction — `mkpiece`'s `get_orect()` plus
/// the link (`aes/gemwrect.c:68-69`). A piece without area is not a piece
/// (`brkrct` never makes one: `have_piece[]` uses strict inequalities,
/// `:113-116`). Past [`RCAP`] see there: EmuTOS writes through `NULL`, we stop.
fn put(out: &mut [Grect; RCAP], m: &mut usize, r: Grect) {
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    assert!(
        *m < RCAP,
        "AES owner rectangle list: more than NUM_ORECT ({RCAP}) pieces in one window's list — \
         EmuTOS's ORECT pool is exhausted here and it writes through the NULL get_orect() \
         returns (aes/gemwrect.c:53-61, :68-69); NewTOS stops instead"
    );
    out[*m] = r;
    *m += 1;
}

/// Subtract covering rect `c` from each rect in `list[0..n]`, producing the
/// non-overlapping remainder — EmuTOS `brkrct` (`aes/gemwrect.c:100-135`)
/// applied to one covering rectangle. Returns the new count.
///
/// The pieces of a broken rectangle replace it **in place**, so the relative
/// order of the untouched rectangles is kept (`brkrct` re-links `p->o_link`,
/// `mkrect` `:136-162`).
fn subtract(list: &mut [Grect; RCAP], n: usize, c: Grect) -> usize {
    let mut out = [Grect::default(); RCAP];
    let mut m = 0usize;
    for r in list.iter().take(n).copied() {
        // No intersection → keep whole.
        if c.x >= r.x + r.w || c.x + c.w <= r.x || c.y >= r.y + r.h || c.y + c.h <= r.y {
            put(&mut out, &mut m, r);
            continue;
        }
        // TOP, LEFT, RIGHT, BOTTOM — the one order `brkrct` emits the pieces
        // in: it indexes `have_piece[]` with the constants `TOP 0`, `LEFT 1`,
        // `RIGHT 2`, `BOTTOM 3` (`aes/gemwrect.c:30-33`) and appends the ones it
        // has in an ascending `for (i = 0; i < 4; i++)` loop (`:113-125`).
        //
        // This is algorithm semantics, not a device parameter, and it is
        // observable: it is the order in which every reader of the owner
        // rectangle list sees the pieces — the draw walk ([`walk_rects`], which
        // thereby also fixes the call order of a `G_USERDEF` object), the
        // public `WF_FIRSTXYWH`/`WF_NEXTXYWH` iteration ([`compute_rlist`]) and
        // the redraw union ([`w_union`]).
        //
        // The edge formulas are `mkpiece` (`aes/gemwrect.c:64-98`): TOP and
        // BOTTOM keep the broken rectangle's full width, LEFT and RIGHT get the
        // "common calcs" overlap band `oy`/`oh`.
        if r.y < c.y {
            put(&mut out, &mut m, Grect { x: r.x, y: r.y, w: r.w, h: c.y - r.y });
        }
        let oy = r.y.max(c.y);
        let oh = (r.y + r.h).min(c.y + c.h) - oy;
        if r.x < c.x {
            put(&mut out, &mut m, Grect { x: r.x, y: oy, w: c.x - r.x, h: oh });
        }
        if r.x + r.w > c.x + c.w {
            put(&mut out, &mut m, Grect { x: c.x + c.w, y: oy, w: (r.x + r.w) - (c.x + c.w), h: oh });
        }
        if r.y + r.h > c.y + c.h {
            put(&mut out, &mut m, Grect { x: r.x, y: c.y + c.h, w: r.w, h: (r.y + r.h) - (c.y + c.h) });
        }
    }
    *list = out;
    m
}

/// Load `handle`'s **raw** owner rectangle list ([`border_pieces`]) into *that
/// window's* slot and rewind its cursor — what `WF_FIRSTXYWH` starts from.
///
/// EmuTOS stores the list in `pwin->w_rlist` and reads it from there
/// (`gemwmlib.c:1556-1561`); it is (re)built for every window by `newrect`
/// whenever the window arrangement changes (`gemwrect.c:165-205`, driven from
/// `draw_change`, `gemwmlib.c:1089`). We build it on demand when the
/// application asks for the first piece, which for a stable arrangement — the
/// only state in which FIRST/NEXT has a defined geometric answer at all — is
/// the same list. Storing it in the window slot is what makes it *that
/// window's* list: an interleaved `FIRST(B)` no longer overwrites A's.
///
/// **Raw, not work-clipped.** What `newrect` links into `w_rlist` are the
/// `WS_TRUE` remainder pieces themselves; the work-area clip happens on the way
/// *out*, once per piece, inside `w_owns` (`w_getsize(WS_WORK, …)` then
/// `rc_intersect`, `gemwmlib.c:1556-1561,1230-1240`). Storing only the pieces
/// that survive that clip would throw away two things the caller can observe:
/// the pure-border pieces the walk still has to step over, and the coordinates
/// the last failed `rc_intersect` leaves behind ([`Owns::Exhausted`]).
/// [`w_union`], the other reader of the same list, does not clip either — the
/// clip belongs to `w_owns` and to nothing else.
fn compute_rlist(handle: i16) {
    let mut list = [Grect::default(); RCAP];
    let n = border_pieces(handle, &mut list);
    let Some(w) = win(handle) else {
        return; // no window, no list to own it (callers check the handle first)
    };
    w.rlist[..n].copy_from_slice(&list[..n]);
    w.rcount = n as u16;
    w.rnext = 0;
}

/// What one `w_owns` walk ended with (`aes/gemwmlib.c:1230-1241`). The three
/// cases differ in **how many of the four words are defined**, which is the
/// whole content of the end-of-list contract:
///
/// ```c
/// while (po) {
///     rc_copy(&po->o_gr, poutwds);
///     pwin->w_rnext = po = po->o_link;
///     if (rc_intersect(pt, poutwds))
///         return;
/// }
/// poutwds->g_w = poutwds->g_h = 0;
/// ```
enum Owns {
    /// A piece still had size after the work-area clip: all four words are the
    /// clipped rectangle (the `return` inside the loop).
    Visible(Grect),
    /// Pieces were walked, none survived. All four words are written, because
    /// [`rc_intersect`] stores its result even when it fails
    /// (`util/rectfunc.c:63-78`) — x/y are those of the **last piece examined**,
    /// clamped to the work area — and `:1240` then zeroes w/h.
    Exhausted { x: i16, y: i16 },
    /// The loop was never entered (`po == NULL` on the way in). `:1240` writes
    /// `g_w`/`g_h` and **nothing else**: x/y keep whatever the dispatcher's
    /// `WORD int_out[O_SIZE]` held (`aes/gemsuper.c:432`, copied out at
    /// `:444-445`). EmuTOS leaves that array uninitialised, so its x/y have no
    /// value to reproduce — A15 traced the values a TT delivers here to the
    /// *caller's* call history rather than to any window state, over all fifteen
    /// measured records. We leave the two words alone as well; ours then read as
    /// the dispatcher's defined zero (`super::dispatch`) instead of stack debris.
    Empty,
}

/// Walk `handle`'s owner rectangle list from its own cursor to the first piece
/// that still has size inside the window's work area — EmuTOS `w_owns`
/// (`aes/gemwmlib.c:1230-1241`), reached from `wm_get` with
/// `pt = w_getsize(WS_WORK, …)` and `po = w_rlist` (FIRST) or `w_rnext` (NEXT)
/// (`:1556-1561`).
///
/// The cursor advances **before** the clip is tested and therefore also on a
/// failed piece (`pwin->w_rnext = po = po->o_link;` sits above the `if`), so a
/// pure-border piece is stepped over rather than re-offered on the next call.
fn rlist_owns(handle: i16) -> Owns {
    let Some(w) = win(handle) else {
        return Owns::Empty;
    };
    let work = w.work;
    let mut last: Option<(i16, i16)> = None;
    while (w.rnext as usize) < (w.rcount as usize) {
        let mut t = w.rlist[w.rnext as usize];
        w.rnext += 1;
        if rc_intersect(work, &mut t) {
            return Owns::Visible(t);
        }
        // Failed — but `rc_intersect` has written all four words anyway, and
        // these are the ones that stand at the end of the walk.
        last = Some((t.x, t.y));
    }
    match last {
        Some((x, y)) => Owns::Exhausted { x, y },
        None => Owns::Empty,
    }
}

/// The identity that owns `handle`, or `None` for no window / no owner.
fn owner_of(handle: i16) -> Option<usize> {
    win(handle).and_then(|w| usize::try_from(w.owner).ok())
}

/// Post a `WM_REDRAW` for `rect` to `handle`'s owner process.
///
/// `msg[1]` is the **caller's** pid (0 for a member of the application chain,
/// the accessory's 2, 3, … for an accessory): `w_redraw`'s `ap_sendmsg`
/// (`gemwmlib.c:904`) runs from `draw_change`, i.e. inside the caller's own
/// `wind_open`/`wind_close`/`wind_set` trap (`:1426,1453,1612`) or its
/// `form_dial(FMD_FINISH)`, so `rlr` is the caller, not the Screen Manager
/// (spec § "AES process identity and pid", contract (b), A113). Every path
/// into this function is such a trap; the
/// control-manager hook ([`intercept`]) posts no redraws.
fn send_redraw(handle: i16, rect: Grect) {
    let Some(owner) = owner_of(handle) else {
        return;
    };
    let mut msg = [0i16; 8];
    msg[0] = WM_REDRAW;
    msg[1] = super::appl::current_pid();
    msg[3] = handle;
    msg[4] = rect.x;
    msg[5] = rect.y;
    msg[6] = rect.w;
    msg[7] = rect.h;
    // Enqueue into the owner's FIFO — EmuTOS `w_redraw`→`ap_sendmsg`→`ap_rdwr`
    // →`doq` (`gemwmlib.c:887-907,851-863`, `gemqueue.c`). Same-window redraws
    // coalesce (rect-union) instead of overwriting, so no content redraw is
    // lost when several are posted before the event loop drains them.
    // SAFETY: post_msg hands the message to the live process slot on the
    // single trap path.
    unsafe {
        super::pd::post_msg(owner, msg);
    }
}

/// EmuTOS `w_union` (`aes/gemwmlib.c:869-884`) — the bounding rectangle over
/// `handle`'s owner rectangle list, or `None` for an empty list (the C
/// function's `FALSE`, which is what keeps [`send_redraw_in`] from sending
/// anything at all):
///
/// ```c
/// static WORD w_union(ORECT *po, GRECT *pt)
/// {
///     if (!po)
///         return FALSE;
///     rc_copy(&po->o_gr, pt);
///     po = po->o_link;
///     while (po) { rc_union(&po->o_gr, pt); po = po->o_link; }
///     return TRUE;
/// }
/// ```
///
/// **The list is the raw one.** `w_union` walks `D.w_win[wh].w_rlist`, the
/// `WS_TRUE` remainder pieces `newrect` links in (`aes/gemwrect.c:190,203`);
/// nothing on this path clips them to the window's work area. The hull
/// therefore covers the window's frame and drop shadow as well, and it is *not*
/// interchangeable with the hull of the work-clipped pieces: intersecting each
/// piece first and taking the hull afterwards is in general a strictly smaller
/// rectangle than taking the hull first and intersecting once.
///
/// EmuTOS (re)builds `w_rlist` with `newrect` at the top of every `draw_change`
/// (`gemwmlib.c:1089`), i.e. before anything is drawn or sent; we derive it on
/// demand from the current arrangement with [`border_pieces`] — the same
/// producer [`compute_rlist`] stores for `WF_FIRSTXYWH`. Reading *that* stored
/// copy instead would be wrong: it only exists once an application has asked
/// for a piece, and stepping it would disturb the caller's iterator.
fn w_union(handle: i16) -> Option<Grect> {
    let mut list = [Grect::default(); RCAP];
    let n = border_pieces(handle, &mut list);
    list.iter().take(n).copied().reduce(union_grect)
}

/// EmuTOS `w_redraw` (`aes/gemwmlib.c:887-907`) — post `handle`'s owner one
/// `WM_REDRAW` for the part of the damage rectangle `damage` that the window
/// owns:
///
/// ```c
/// static void w_redraw(WORD w_handle, GRECT *pt)
/// {
///     GRECT t, d;
///     /* make sure work rect and word rect intersect */
///     rc_copy(pt, &t);
///     w_getsize(WS_WORK, w_handle, &d);
///     if (rc_intersect(&t, &d))                          /* :896 */
///         /* make sure window owns a rectangle */
///         if (w_union(D.w_win[w_handle].w_rlist, &d))    /* :899 */
///             /* intersect redraw rect with union of owner rects */
///             if (rc_intersect(&d, &t))                  /* :902 */
///                 ap_sendmsg(wind_msg, WM_REDRAW, ppd, w_handle,
///                            t.g_x, t.g_y, t.g_w, t.g_h);
/// }
/// ```
///
/// **The work rectangle is a gate, not a clip.** `rc_intersect(&t, &d)` writes
/// its result into `d` and leaves `t` at the full damage rectangle
/// (`util/rectfunc.c:61-78`, [`rc_intersect`]); its return value is only asked
/// whether the window's work area is touched at all. `d` is then *overwritten*
/// by [`w_union`], and what goes out is `damage ∩ hull(raw owner list)` — which
/// reaches over the work area's edge, across the window's frame and drop
/// shadow. That is not an oversight in EmuTOS: the owner clips its own painting
/// with `WF_FIRSTXYWH`/`WF_NEXTXYWH`, and *those* are work-clipped, per piece,
/// by `w_owns` ([`rlist_owns`]) — e.g. `do_wredraw`, `desk/desksupp.c:223-229`.
///
/// Sending `damage ∩ hull(pieces ∩ work)` instead — which is what this function
/// did before — is a different, strictly smaller rectangle for two independent
/// reasons: the work clip, and the fact that a hull of clipped pieces is not
/// the clipped hull.
///
/// The leading clip against [`desktop_work`] is `w_update`'s "limit to screen",
/// `rc_intersect(&gl_rfull, pt)` (`gemwmlib.c:1010`), which every path into
/// `w_redraw` passes through. It sits here rather than in [`w_update`]
/// because both of our entries into that fan-out ([`w_update`] and
/// [`top_window`]'s single-window turn) funnel through this function, and
/// clipping the damage once before the loop or once per turn is the same
/// rectangle.
fn send_redraw_in(handle: i16, damage: Grect) {
    let Some(work) = win(handle).map(|w| w.work) else {
        return; // no window, no owner to send to
    };
    let mut t = damage;
    rc_intersect(desktop_work(), &mut t); // w_update `:1010`, result unused there too
    let mut d = work;
    if !rc_intersect(t, &mut d) {
        return; // the damage does not reach the work area
    }
    let Some(d) = w_union(handle) else {
        return; // the window owns no rectangle
    };
    if rc_intersect(d, &mut t) {
        send_redraw(handle, t);
    }
}

/// The handles of every open window, **top of the z-order first** — EmuTOS's
/// `W_TREE[ROOT]` sibling chain read from `ob_tail` back to `ob_head`, which
/// is the order `w_update` visits them in (`gemwmlib.c:1021-1043`). Returns
/// the count; `out[..n]` holds the handles. Ties (which the monotonic counter
/// never produces) break towards the higher slot, the mirror image of
/// [`compose`]'s ascending walk.
fn open_by_z_desc(out: &mut [i16; NUM_WIN]) -> usize {
    let mut n = 0usize;
    let mut done = [false; NUM_WIN];
    loop {
        let mut pick = usize::MAX;
        let mut pick_z = i16::MIN;
        for (i, w) in slots().iter().enumerate() {
            if w.in_use && w.open && !done[i] && (pick == usize::MAX || w.z >= pick_z) {
                pick = i;
                pick_z = w.z;
            }
        }
        if pick == usize::MAX {
            return n;
        }
        done[pick] = true;
        out[n] = pick as i16;
        n += 1;
    }
}

/// EmuTOS `w_update(bottom, pt, top, moved)` (`gemwmlib.c:1004-1048`), the
/// message half — the `WM_REDRAW` fan-out over the exposed `damage`
/// rectangle:
///
/// ```c
/// if (bottom == DESKWH) bottom = W_TREE[ROOT].ob_head;     /* :1014-1015 */
/// if (bottom != NIL) {
///     if (top == DESKWH) top = W_TREE[ROOT].ob_tail;       /* :1021-1022 */
///     do {
///         if (!(moved && (top == gl_wtop))) {              /* :1025      */
///             gsx_sclip(pt);
///             w_cpwalk(top, 0, MAX_DEPTH, FALSE);          /* the frame  */
///             w_redraw(top, pt);                           /* the message*/
///         }
///         /* scan to find prev */                          /* :1032-1041 */
///         ...
///     } while (!done);
/// }
/// ```
///
/// The windows are visited **from `top` down to `bottom`, inclusive, in
/// z-order** — `DESKWH` standing for the top-most (`ob_tail`) and the
/// bottom-most (`ob_head`) window respectively — and each visited one is
/// posted its part of `damage` by [`send_redraw_in`] (`w_redraw`,
/// `:887-907`). Two things are contract in that walk and were not kept
/// before (A78): the **order** — with `h1` under `h2` and `h1` moved, `h2`'s
/// message is queued before `h1`'s, not the other way round — and the
/// **`moved` skip**: after `w_move` blitted the top window to its new place,
/// or after a pure-subset shrink, `draw_change` passes `moved = TRUE` and the
/// top window is stepped over without a message, because nothing of its
/// content is missing. Windows above `top` and below `bottom` are not visited
/// at all — a superset grow (`start = w_handle`) stops at the grown window
/// and the ones underneath it get nothing, even where the hull of their
/// rectangle list still reaches into the damage.
///
/// The **frame half** of the loop — `w_cpwalk` per visited window, clipped to
/// its owner rectangle list inside `damage` — is done by [`compose`] /
/// [`redraw_clip`], which paints every open window's frame in ascending
/// z-order through the same per-window clip; since the clips are the disjoint
/// pieces of the rectangle lists, the pixels do not depend on the order, and a
/// frame outside the `[top, bottom]` range repaints its own unchanged pixels.
///
/// Every posted `WM_REDRAW` is appended to the owner's per-app FIFO queue
/// (`Aespd::queue`), so when several **same-owner** windows are uncovered at
/// once each one's redraw is delivered (in FIFO order) — none is overwritten.
/// Redraws for the *same* window coalesce via the queue's rectangle-union
/// (EmuTOS `doq`, `gemqueue.c:45-56`).
fn w_update(bottom: i16, damage: Grect, top: i16, moved: bool) {
    let mut order = [0i16; NUM_WIN];
    let n = open_by_z_desc(&mut order);
    let gl_wtop = get_top();
    let mut started = top == DESKWH;
    for &h in order.iter().take(n) {
        if !started {
            if h != top {
                continue;
            }
            started = true;
        }
        if !(moved && h == gl_wtop) {
            send_redraw_in(h, damage);
        }
        if h == bottom {
            break;
        }
    }
}

/// AES `bb_screen` (`aes/gemgraf.c:357-363` → `gsx_blt`, `:328-351`): move
/// the screen rectangle `(sx, sy, w, h)` to `(dx, dy)` — a screen-to-screen
/// `vro_cpyfm(S_ONLY)` with the clip `w_move` set just before (`gsx_sclip(&gl_
/// rfull)`, `gemwmlib.c:969`), i.e. the backend's overlap-safe block copy
/// with the replace op (`3`) clipped to [`desktop_work`]. `gsx_blt` brackets
/// the copy in `gsx_moff()`/`gsx_mon()` (`:337,350`); the cursor is a
/// save-under sprite here too, so it is hidden for the copy exactly as it is
/// for `bb_save`/`bb_restore` — otherwise the sprite would be copied along and
/// its backing store would restore stale pixels at the old place.
fn bb_screen(backend: &dyn RasterBackend, sx: i16, sy: i16, dx: i16, dy: i16, w: i16, h: i16) {
    const S_ONLY: u8 = 3;
    let full = desktop_work();
    let clip = rect_clip(full.x, full.y, full.w, full.h);
    crate::vdi::cursor_hide(backend);
    backend.copy_rect(crate::vdi::backend::Rect { x: sx, y: sy, w, h }, dx, dy, S_ONLY, clip);
    crate::vdi::cursor_show(backend);
}

/// EmuTOS `w_mvfix` (`aes/gemwmlib.c:915-930`) — clip one of the blit's
/// rectangles to `gl_rfull` and, when it stood at `x = -1` (the classic
/// "hide the left border" placement), take one column off the *other*
/// rectangle's left edge so the two stay aligned:
///
/// ```c
/// tmpsx = ps->g_x;
/// rc_intersect(&gl_rfull, ps);
/// if (tmpsx == -1) { pd->g_x++; pd->g_w--; return TRUE; }
/// return FALSE;
/// ```
///
/// Only `-1` is realigned; a rectangle further off the left edge is clipped
/// without its partner being shifted — that is the reference's behaviour and
/// is kept.
fn w_mvfix(ps: &mut Grect, pd: &mut Grect) -> bool {
    let tmpsx = ps.x;
    rc_intersect(desktop_work(), ps);
    if tmpsx == -1 {
        pd.x += 1;
        pd.w -= 1;
        return true;
    }
    false
}

/// EmuTOS `w_move` (`aes/gemwmlib.c:938-996`) — carry the **top** window's
/// pixels to its new place with a screen blit, if that is possible, and hand
/// back what `draw_change` needs: `(moved, stop, c)`.
///
/// ```c
/// w_getsize(WS_PREV, w_handle, &s);  s.g_w += DROP_SHADOW_SIZE;  s.g_h += …;
/// w_getsize(WS_TRUE, w_handle, &d);                              /* :945-948 */
/// if ( ((s.g_x+s.g_w > gl_width)  && (d.g_x < s.g_x)) ||
///      ((s.g_y+s.g_h > gl_height) && (d.g_y < s.g_y)) )          /* :951-952 */
///     { rc_union(&s, &d); *pstop = DESKWH; }                     /* :954-955 */
/// else
///     *pstop = w_handle;                                         /* :959     */
/// sminus1 = w_mvfix(&s, &d);  dminus1 = w_mvfix(&d, &s);         /* :963-964 */
/// if (*pstop == w_handle) {
///     gsx_sclip(&gl_rfull);
///     bb_screen(s.g_x, s.g_y, d.g_x, d.g_y, s.g_w, s.g_h);       /* :969-970 */
///     if (sminus1 != dminus1) {                                  /* :972-982 */
///         if (dminus1) s.g_x--;
///         if (sminus1) { d.g_x--; d.g_w = 1; gsx_sclip(&d); w_cpwalk(gl_wtop, 0, 0, FALSE); }
///     }
///     pc = &s;
/// } else
///     pc = &d;
/// rc_copy(pc, prc);                                              /* :992     */
/// return (*pstop == w_handle);                                   /* :994     */
/// ```
///
/// **When the blit is refused.** A window that reaches past the right or the
/// bottom edge of the screen has columns/rows that were never drawn; if the
/// move brings them *into* view (leftwards / upwards) the blit cannot supply
/// them, so the whole union of old and new rectangle is redrawn instead —
/// `stop = DESKWH`, `moved = FALSE`, and the window will be sent a
/// `WM_REDRAW` like any other. Moving such a window further towards that
/// edge blits the on-screen part (the source clipped to `gl_rfull`), which is
/// everything that stays visible.
///
/// **What comes back as `c`.** In the blit case it is the *vacated* source
/// rectangle (`pc = &s`): the destination already has its pixels, only the
/// place the window left needs the desktop and the windows underneath
/// repainted. In the refused case it is the clipped union.
///
/// The `-1` fringe repair — `w_cpwalk(gl_wtop, 0, 0, FALSE)` under a
/// one-column clip — is [`draw_frame_pieces`] with that column as the damage
/// rectangle: the left border column the blit could not bring along (its
/// source was off-screen) is drawn afresh at the destination.
fn w_move(handle: i16, backend: Option<&dyn RasterBackend>) -> (bool, i16, Grect) {
    let Some((prev, curr)) = win(handle).map(|w| (w.prev, w.curr)) else {
        return (false, DESKWH, Grect::default());
    };
    let screen = screen_rect();
    let mut s = Grect { w: prev.w + DROP_SHADOW_SIZE, h: prev.h + DROP_SHADOW_SIZE, ..prev };
    let mut d = ws_true(curr);
    let stop = if (s.x + s.w > screen.w && d.x < s.x) || (s.y + s.h > screen.h && d.y < s.y) {
        d = union_grect(s, d); // rc_union(&s, &d)
        DESKWH
    } else {
        handle
    };
    let sminus1 = w_mvfix(&mut s, &mut d);
    let dminus1 = w_mvfix(&mut d, &mut s);
    if stop == handle {
        if let Some(backend) = backend {
            bb_screen(backend, s.x, s.y, d.x, d.y, s.w, s.h);
        }
        if sminus1 != dminus1 {
            if dminus1 {
                s.x -= 1;
            }
            if sminus1 {
                d.x -= 1;
                d.w = 1;
                if let Some(backend) = backend {
                    crate::vdi::cursor_hide(backend);
                    draw_frame_pieces(handle, true, backend, Some(d));
                    crate::vdi::cursor_show(backend);
                }
            }
        }
        (true, stop, s)
    } else {
        (false, stop, d)
    }
}

/// EmuTOS `draw_change` (`aes/gemwmlib.c:1058-1223`) for a geometry change of
/// an **open** window that keeps its place in the z-order — `wm_set`'s
/// `WF_CXYWH` (`:1676-1678`). The geometry itself (`WS_PREV := WS_CURR`,
/// `WS_CURR := pt`, `WS_WORK := wm_calc`, `:1069-1077`) is stored by the
/// caller before this runs, exactly as `draw_change` stores it before the
/// `VF_ISOPEN` test (`:1085-1086`); `old` is the rectangle the window had.
/// The rectangle lists EmuTOS rebuilds here (`:1089`) are derived on demand
/// ([`border_pieces`]), and `oldtop == W_TREE[ROOT].ob_tail` throughout, so
/// the open/close block (`:1194-1211`) has nothing to do.
///
/// What is decided is `start` (the bottom of the redraw walk), `stop` (its
/// top), `moved` (skip `gl_wtop`) and the damage rectangle `c`, branch by
/// branch as in the reference — see the spec table in
/// `docs/api/aes/window-graphics.md` § `wind_set`, "`WF_CXYWH` on an open
/// window". Then:
///
/// ```c
/// c.g_w += DROP_SHADOW_SIZE;  c.g_h += DROP_SHADOW_SIZE;   /* :1213-1215 */
/// if (start == DESKWH) w_drawdesk(&c);                     /* :1217-1219 */
/// w_update(start, &c, stop, moved);                        /* :1221-1222 */
/// ```
///
/// `w_drawdesk` plus the frame half of `w_update` is one [`compose_except`]
/// over `c` (the desktop's own pieces inside `c` are empty whenever `start !=
/// DESKWH` — a superset grow's damage is the grown window's `WS_TRUE`, which
/// no desktop piece reaches into — so the conditional is implied), with
/// `gl_wtop` left out when `moved`; the message half is [`w_update`].
fn draw_change_open(handle: i16, pt: Grect, old: Grect) {
    let backend = host().and_then(|h| h.raster());
    let gl_wtop = get_top();
    let mut c = old;
    let mut start = handle; // init. starting window            (:1097)
    let mut stop = DESKWH; // stop at the top                   (:1098)
    let mut moved = false; // we haven't moved the top window   (:1101)
    if pt != Grect::default() && pt.x == c.x && pt.y == c.y {
        // "size or top request" (:1108-1155)
        if pt.w == c.w && pt.h == c.h {
            // A top request — which a `wm_set` never is, the window is
            // either already `ob_tail` (== oldtop) or not the tail at all:
            // "return if this isn't a top request" (:1113-1116). Nothing is
            // drawn and nothing is sent for an unchanged rectangle.
            return;
        }
        // "handle size request" (:1138-1155)
        if pt.w <= c.w && pt.h <= c.h {
            // "stop before current window if shrink was a pure subset"
            stop = handle;
            if let Some(backend) = backend {
                // w_cpwalk(gl_wtop, 0, MAX_DEPTH, TRUE) (:1144) — the top
                // window's frame, clipped to its own rectangle list.
                crate::vdi::cursor_hide(backend);
                draw_frame_pieces(gl_wtop, true, backend, None);
                crate::vdi::cursor_show(backend);
            }
            moved = true;
        }
        // "start at bottom if a shrink occurred" (:1148-1150)
        if pt.w < c.w || pt.h < c.h {
            start = DESKWH;
        }
        // "update rect. is the union of two sizes + the drop shadow"
        // (:1152-1154) — this shadow is in addition to the tail's, so a size
        // change repaints two shadow widths past the larger rectangle.
        c.w = pt.w.max(c.w) + DROP_SHADOW_SIZE;
        c.h = pt.h.max(c.h) + DROP_SHADOW_SIZE;
    } else if (c.w == 0 || c.h == 0)
        || (pt.x <= c.x && pt.y <= c.y && pt.x + pt.w >= c.x + c.w && pt.y + pt.h >= c.y + c.h)
    {
        // "a grow that is a superset or an open" (:1160-1167): the new
        // rectangle covers the old one, so it alone is the damage, and the
        // walk starts at this window (`start` stays `w_handle`).
        c = pt;
    } else {
        // "move, close or shrink" (:1169-1191)
        if pt.w == c.w && pt.h == c.h && gl_wtop == handle {
            // "do a move of top guy" (:1173-1178)
            let (m, st, rc) = w_move(handle, backend);
            moved = m;
            stop = st;
            c = rc;
            start = DESKWH;
        }
        // "check for a close" (:1180-1182)
        if pt.w == 0 || pt.h == 0 {
            start = DESKWH;
        }
        // "handle other moves and shrinks" (:1184-1190)
        if start != DESKWH {
            c = union_grect(pt, c);
            if pt != c {
                start = DESKWH;
            }
        }
    }
    // "account for drop shadow (BUGFIX in 2.1)" (:1213-1215)
    c = plus_shadow(c);
    // w_drawdesk(&c) (:1217-1219) + w_update's frame walk (:1028-1029)
    if c.w > 0 && c.h > 0 {
        if let Some(backend) = backend {
            compose_except(backend, rect_clip(c.x, c.y, c.w, c.h), moved.then_some(gl_wtop));
        }
    }
    // w_update(start, &c, stop, moved) (:1221-1222) — the messages
    w_update(start, c, stop, moved);
}

/// Map a window handle to its table slot, if valid and in use — EmuTOS
/// `get_pwin` (`gemwmlib.c:1391-1404`): the handle indexes the table directly
/// and **0 is a legal handle** (the desktop window, `VF_INUSE` since
/// [`wm_init`]); only a negative handle, one past the table, or a slot without
/// `VF_INUSE` fails.
fn win(handle: i16) -> Option<&'static mut Window> {
    if handle < 0 || handle as usize >= NUM_WIN {
        return None;
    }
    let w = &mut slots()[handle as usize];
    w.in_use.then_some(w)
}

/// Move a window: store the new border rect and recompute the work rect from
/// it — EmuTOS `w_move` (`gemwmlib.c:1069-1077`), the *only* place besides
/// `wm_init`/`wm_delete` where `WS_WORK` is written.
fn set_curr(w: &mut Window, r: Grect) {
    w.curr = r;
    w.work = calc(WC_WORK, w.kind, r);
}

// ── geometry ────────────────────────────────────────────────────────────

/// Which scroll bars a `kind` gets after EmuTOS' `SIZER` conflict resolution —
/// `(havevbar, havehbar)`, `wm_calc`'s 3D branch (`gemwmlib.c:1823-1834`).
///
/// A `SIZER` on its own asks for *both* bars; when the window already has real
/// gadgets on an axis the `SIZER` is stripped from both requests and only the
/// axes with gadgets keep a bar. If stripping leaves nothing, the vertical bar
/// is forced back on — TOS4 draws the sizer in a vertical bar when the window
/// has no other elements. This is the rule the non-3D branch does not have, and
/// it is why `SIZER|HGADGETS` grows only the bottom border while `SIZER` alone
/// grows only the right one.
fn bars(kind: u16) -> (bool, bool) {
    let mut havevbar = kind & (VGADGETS | SIZER);
    let mut havehbar = kind & (HGADGETS | SIZER);
    if havevbar != 0 && havehbar != 0 {
        havevbar &= !SIZER;
        havehbar &= !SIZER;
        if havevbar == 0 && havehbar == 0 {
            havevbar |= SIZER;
        }
    }
    (havevbar != 0, havehbar != 0)
}

/// Border thickness contributed by `kind` on each side: `(left, top,
/// right, bottom)` — EmuTOS `wm_calc`, **3D branch** (`gemwmlib.c:1817-1840`),
/// the branch our Falcon030/TOS4 profile compiles (`CONF_WITH_3D_OBJECTS`,
/// `config.h:1259`).
///
/// Four things differ from the `#else` branch: the title band is
/// `adj_hbox - 1` (not `gl_hbox - 1`) thick, `MOVER` alone earns that band
/// (`TGADGETS`), an `INFO` line costs a full `gl_hbox` (not `gl_hbox - 1`), and
/// the scroll bars are decided by [`bars`]' strip-and-force rule instead of a
/// plain `SIZER` test on both axes.
fn borders(kind: u16) -> (i16, i16, i16, i16) {
    let lb = 1;
    let mut tb = 1;
    let mut rb = 1;
    let mut bb = 1;
    if kind & TGADGETS != 0 {
        tb += adj_hbox() - 1;
    }
    if kind & INFO != 0 {
        tb += gl_hbox();
    }
    let (havevbar, havehbar) = bars(kind);
    if havevbar {
        rb += adj_wbox() - 1;
    }
    if havehbar {
        bb += adj_hbox() - 1;
    }
    (lb, tb, rb, bb)
}

/// Convert a rectangle between border and work coordinates (`wm_calc`).
fn calc(wtype: i16, kind: u16, r: Grect) -> Grect {
    let (mut lb, mut tb, mut rb, mut bb) = borders(kind);
    if wtype == WC_BORDER {
        lb = -lb;
        tb = -tb;
        rb = -rb;
        bb = -bb;
    }
    Grect {
        x: r.x + lb,
        y: r.y + tb,
        w: r.w - lb - rb,
        h: r.h - tb - bb,
    }
}

// ── frame drawing ───────────────────────────────────────────────────────
//
// Every gadget is painted from its colour word exactly as `just_draw` paints
// the `W_ACTIVE` tree (`aes/gemoblib.c:610-733`): the 1px `gr_box` border in
// `bc`, the interior `ip`/`ic` via `gr_rect` (hollow = white), text in `tc`
// with `md`, and the 3D bevel on the `FL3DACT` gadgets. With the compiled
// defaults (`gl_wtcolor`/`gl_wbcolor`, `gemwmlib.c:1347-1357`) that is white
// faces with black borders, the `IP_2PATT` stipple on the topped name and
// `IP_1PATT` slider tracks — the picture EmuTOS itself draws on a Falcon
// (A79, user decision 2026-09-20; reference
// `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/`). The only fixed
// colour left in the frame is [`DROP_SHADOW_COLOR`].

/// A gadget's colour word cracked for the frame draw — EmuTOS `gr_crack`
/// (`aes/gemgraf.c:612-628`) after `just_draw`'s 3D substitution
/// (`aes/gemoblib.c:582-600,619-636`).
#[derive(Clone, Copy)]
struct Ink {
    /// Border colour (bits 15-12): the 1px `gr_box` stroke.
    bc: Pel,
    /// Text colour (bits 11-8): name / info / gadget glyph.
    tc: Pel,
    /// Interior pattern (bits 6-4), `IP_HOLLOW`..`IP_SOLID`.
    ip: u16,
    /// Interior colour (bits 3-0).
    ic: Pel,
    /// Text writing mode (bit 7): `true` = `MD_REPLACE` (the VDI paints each
    /// glyph cell's background white), `false` = `MD_TRANS`.
    replace: bool,
}

impl Ink {
    fn text_mode(self) -> WriteMode {
        if self.replace {
            WriteMode::Replace
        } else {
            WriteMode::Transparent
        }
    }
}

/// Whether gadget `g` carries the `FL3DACT` role — `gl_3dflags`
/// (`aes/gemwmlib.c:108-127`); `W_NAME` gets it at build time only when the
/// window has a `NAME` (`w_bldactive`, `:553`). Every other frame gadget is
/// `FL3DNONE`.
fn is_3d_act(g: usize, kind: u16) -> bool {
    matches!(
        g,
        W_CLOSER_IDX
            | W_FULLER_IDX
            | W_SIZER_IDX
            | W_UPARROW_IDX
            | W_DNARROW_IDX
            | W_VELEV_IDX
            | W_LFARROW_IDX
            | W_RTARROW_IDX
    ) || (g == W_NAME_IDX && kind & NAME != 0)
}

/// The ink gadget `g` of window `w` is painted with.
///
/// The word is `istop ? w_tcolor[g] : w_bcolor[g]` — what `w_setcolor` writes
/// into the gadget's `ob_spec` / `te_color` on every `w_bldactive`
/// (`aes/gemwmlib.c:382-397`). `just_draw` then substitutes a hollow-WHITE
/// interior on a 3D object with the solid role colour — `actbutcol` for the
/// `FL3DACT` box types (`aes/gemoblib.c:619-636`), `backgrcol` for a text type
/// with any 3D role, which also drops `MD_REPLACE` to `MD_TRANS` so the VDI's
/// white glyph background does not punch holes in it (`:582-600`). No compiled
/// default has `ic == WHITE`, so this only ever fires on an application's word.
fn gadget_ink(w: &Window, g: usize, istop: bool) -> Ink {
    let word = if istop { w.tcolor[g] } else { w.bcolor[g] };
    ink_of(word as u16, is_3d_act(g, w.kind), g == W_NAME_IDX || g == W_INFO_IDX)
}

/// [`Ink`] for a raw colour word: `gr_crack` plus the 3D substitution above.
/// `boxtext` = the gadget is a `G_BOXTEXT` (`W_NAME`/`W_INFO`, `gl_watype`,
/// `aes/gemwmlib.c:61-81`), whose substitution rule differs from the box types'.
fn ink_of(word: u16, d3_act: bool, boxtext: bool) -> Ink {
    let (bc, tc, mut ip, mut ic, mut replace) = gr_crack(word);
    if d3_act && ip == IP_HOLLOW && ic == WHITE {
        ip = IP_SOLID;
        if boxtext {
            ic = BACKGR_COL.load(Ordering::Relaxed) as Pel;
            replace = false;
        } else {
            ic = ACT_BUTCOL.load(Ordering::Relaxed) as Pel;
        }
    }
    Ink { bc, tc, ip, ic, replace }
}

/// The rect `just_draw` paints an `FL3DACT` object on: its own rect grown by
/// `ADJ3DSTD` on every side (`aes/gemoblib.c:503-508`) — so a `gl_wbox` ×
/// `gl_hbox` gadget cell is drawn over its whole `adj_wbox` × `adj_hbox` slot,
/// border on the slot's edge. Draw-only: the stored rects, the hit tests and
/// the rectangle lists keep the un-grown cell.
fn grow_3d(r: Grect) -> Grect {
    Grect { x: r.x - ADJ3DSTD, y: r.y - ADJ3DSTD, w: r.w + 2 * ADJ3DSTD, h: r.h + 2 * ADJ3DSTD }
}

/// `gr_inside(&t, th)` for `th = 1` — the rect inside a 1px border.
fn inside_1(r: Grect) -> Grect {
    Grect { x: r.x + 1, y: r.y + 1, w: r.w - 2, h: r.h - 2 }
}

fn rect_outline(r: Grect, pel: Pel, backend: &dyn RasterBackend, clip: ClipRect) {
    backend.fill_rect(r.x, r.y, r.w, 1, pel, clip); // top
    backend.fill_rect(r.x, r.y + r.h - 1, r.w, 1, pel, clip); // bottom
    backend.fill_rect(r.x, r.y, 1, r.h, pel, clip); // left
    backend.fill_rect(r.x + r.w - 1, r.y, 1, r.h, pel, clip); // right
}

/// Border and interior of a box-type gadget as `just_draw`'s shared arm
/// draws them (`aes/gemoblib.c:660-694`): `gr_box` in `bc` — `th = 1` for
/// every frame gadget that paints at all (`gl_waspec` byte 2, `gemwmlib.c:84-
/// 106`; `te_thickness = 1` for the two `TEDINFO`s, `:154-157`) — then
/// `gr_rect(ic, ip)` on the rect `gr_inside(&t, th)` leaves, in `MD_REPLACE`:
/// `IP_HOLLOW` is an opaque **white** fill, not a no-op.
fn draw_box_ink(r: Grect, ink: Ink, backend: &dyn RasterBackend, clip: ClipRect) {
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    rect_outline(r, ink.bc, backend, clip);
    let i = inside_1(r);
    gr_rect(backend, i.x, i.y, i.w, i.h, ink.ic, ink.ip, clip);
}

/// Draw one gadget cell with a centred glyph (closer/fuller/arrows/sizer).
/// `sel` = the object is `SELECTED` (pressed feedback): the bevel sinks and
/// the glyph nudges.
///
/// `just_draw` on the 3D-grown slot ([`grow_3d`]): border + interior from
/// the word ([`draw_box_ink`]), the glyph in `tc`/`md` (`gsx_attr(TRUE,
/// tmode, tcol)` then `gr_gtext`, `aes/gemoblib.c:709,722-757`) at
/// [`boxchar_origin`], and last the bevel inside the border
/// (`add_3d_effect`, `:949-950`, inset by `th = 1`, `:230-237`) — the two
/// 1-px polylines of [`add_3d_effect`], not a 2-px band (A95).
///
/// **`changecol` (A105 (4)).** Every frame gadget drawn here is `FL3DACT`
/// (`gl_3dflags`, `gemwmlib.c:110-129`), so `just_draw` takes `changecol =
/// actcolchange` (`aes/gemoblib.c:509-513`; default 0, `init_3d`, `:1332`,
/// settable by `objc_sysvar(LK3DACT)`). With it set, a `SELECTED` gadget
/// changes colour: a hollow interior becomes solid black, any other takes
/// its complement (`:681-690`, `xor_color`), the glyph colour takes its
/// complement and the text mode drops to `MD_TRANS` (`:699-706`), and the
/// bevel is judged against the *changed* interior colour (`:950`). With it
/// clear (the default) the press shows only as the sunk bevel and the moved
/// glyph — the interior keeps its colour (the hollow white of the default
/// `0x1101` word, `wm_start`, `gemwmlib.c:1346-1349`; `ic = BLACK` there,
/// so the 3D face substitution of [`ink_of`] does not apply either).
fn gadget_sel(r: Grect, ch: u16, sel: bool, ink: Ink, backend: &dyn RasterBackend, clip: ClipRect) {
    let slot = grow_3d(r);
    let mut ink = ink;
    if sel && ACT_COLCHANGE.load(Ordering::Relaxed) != 0 {
        if ink.ip == IP_HOLLOW {
            ink.ip = IP_SOLID;
            ink.ic = BLACK;
        } else {
            ink.ic = xor_color(backend, ink.ic);
        }
        ink.tc = xor_color(backend, ink.tc);
        ink.replace = false;
    }
    draw_box_ink(slot, ink, backend, clip);
    if let Some((gx, gy)) = boxchar_origin(slot, sel) {
        backend.draw_glyph(gx, gy, crate::vdi::font::glyph(ch), ink.tc, ink.text_mode(), clip);
    }
    add_3d_effect(backend, slot.x, slot.y, slot.w, slot.h, sel, 1, ink.ic, clip);
}

/// Where `just_draw` puts the one glyph of a `G_BOXCHAR` drawn on the grown
/// slot `slot` with a 1-px border — the glyph cell's top-left, or `None`
/// when the cell has no room for a full glyph.
///
/// `gr_gtext(TE_CNTR, IBM, ch, &c)` gets `c = gr_inside(&t, tmpth)`, the
/// rect inside the border (`aes/gemoblib.c:735-738`), nudged for an
/// activator when `acttxtmove` is set (`:742-755`, default 1, `:1332`):
/// `c.g_y--` while not `SELECTED`, `c.g_x++` while `SELECTED`. `gr_just`
/// (`aes/gemgraf.c:567-593`) then measures the text with `gsx_tcalc`
/// (`:480-497`: width `min(c.w, 8)`, height `min(c.h, 16)`, and **no**
/// character at all when the cell is shorter than a glyph) and centres it
/// with `(h + 1) / 2` and `(w + 1) / 2` — rounding *up* on an odd remainder,
/// which is what puts the closer's glyph at `(8, 3)` of its `23 x 23` slot
/// and not at `(7, 3)` (reference `falcon-tc-window-c.png`, A95 (3)).
fn boxchar_origin(slot: Grect, sel: bool) -> Option<(i16, i16)> {
    let mut c = inside_1(slot);
    if ACT_TXTMOVE.load(Ordering::Relaxed) != 0 {
        if sel {
            c.x += 1;
        } else {
            c.y -= 1;
        }
    }
    let tw = c.w.min(gl_wchar());
    let th = c.h.min(gl_hchar());
    if th / gl_hchar() == 0 || tw / gl_wchar() == 0 {
        return None;
    }
    let (mut x, mut y) = (c.x, c.y);
    let dh = c.h - th;
    if dh > 0 {
        y += (dh + 1) / 2;
    }
    let dw = c.w - tw;
    if dw > 0 {
        x += (dw + 1) / 2;
    }
    Some((x, y))
}

fn gadget(r: Grect, ch: u16, ink: Ink, backend: &dyn RasterBackend, clip: ClipRect) {
    gadget_sel(r, ch, false, ink, backend, clip);
}

/// The window's solid drop shadow: two black bands to the lower right of the
/// border rectangle `b`.
///
/// EmuTOS never draws this as frame code — it sets `SHADOWED` on the root of
/// the window element tree once, in `wm_init`
/// (`W_ACTIVE[ROOT].ob_state = SHADOWED`, `aes/gemwmlib.c:1284`), and every
/// `w_cpwalk`→`do_walk`→`ob_draw` of that tree then paints it as an ordinary
/// object decoration (`aes/gemoblib.c:889-901`):
///
/// ```c
/// if (th > 0) gr_inside(&t, th); else th = -th;
/// if ((state & SHADOWED) && th)
/// {
///     vsf_color(bcol);
///     bb_fill(MD_REPLACE, FIS_SOLID, 0, t.g_x, t.g_y+t.g_h+th, t.g_w + th, 2*th);
///     bb_fill(MD_REPLACE, FIS_SOLID, 0, t.g_x+t.g_w+th, t.g_y, 2*th, t.g_h+(3*th));
/// }
/// ```
///
/// Our frame is a raster routine rather than an object tree, so the same two
/// fills are issued here. Substituting the inset rectangle
/// `t = (x+th, y+th, w-2th, h-2th)` that `gr_inside` leaves, and
/// `2*th = DROP_SHADOW_SIZE`, gives them in terms of the border rect:
///
/// - bottom band `(x + th, y + h, w - th, DROP_SHADOW_SIZE)`
/// - right band  `(x + w, y + th, DROP_SHADOW_SIZE, h + th)`
///
/// which together fill exactly `ws_true(b) \ b` minus two `th`-wide notches —
/// bottom-left `(x, y+h)…(x+th-1, …)` and top-right `(x+w, y)…(…, y+th-1)`.
/// Those notches are unpainted in EmuTOS too; they are not a defect and no test
/// may demand a colour there.
///
/// `bcol` is the border colour cracked out of `W_BOX`'s `ob_spec 0x00011101`
/// (`gemwmlib.c:63,84`): `gr_crack` takes bits 15..12 of the low word `0x1101`
/// (`aes/gemgraf.c:612-627`), i.e. `1` = BLACK.
fn draw_shadow(b: Grect, backend: &dyn RasterBackend, clip: ClipRect) {
    // `if ((state & SHADOWED) && th)` — a zero thickness casts no shadow, and
    // `ws_true` then adds nothing either, so both halves stay in step.
    if SHADOW_TH == 0 || b.w <= 0 || b.h <= 0 {
        return;
    }
    backend.fill_rect(
        b.x + SHADOW_TH, b.y + b.h, b.w - SHADOW_TH, DROP_SHADOW_SIZE,
        DROP_SHADOW_COLOR, clip,
    );
    backend.fill_rect(
        b.x + b.w, b.y + SHADOW_TH, DROP_SHADOW_SIZE, b.h + SHADOW_TH,
        DROP_SHADOW_COLOR, clip,
    );
}

/// Where `gr_gtext(just, IBM, text, &c)` puts an `n`-character system-font
/// string in the cell `c`, and how many of its characters it draws —
/// `(x, y, numchs)`, or `None` when nothing is drawn.
///
/// `gsx_tcalc` (`aes/gemgraf.c:480-497`): the text's extent is clamped to
/// the cell (`textw = min(c.w, n·gl_wchar)`, `texth = min(c.h, gl_hchar)`)
/// and then **whole characters that do not fit are dropped** (`numchs =
/// min(n, textw / gl_wchar)`); a cell shorter than one glyph draws nothing
/// (`texth / gl_hchar == 0` → `numchs = 0`). `gr_just` (`:567-589`) then
/// centres vertically and justifies horizontally, each axis only when there
/// is slack and rounding the half-margin **up** (`(slack + 1) / 2`); text
/// wider than the cell has no slack and stays at the cell's origin —
/// **left-aligned** whatever `te_just` says. The same rule
/// [`boxchar_origin`] applies to the one glyph of a `G_BOXCHAR`
/// (`object::gr_gtext` to the object drawer's TEDINFO family).
fn gr_just_text(just: i16, c: Grect, n: usize) -> Option<(i16, i16, usize)> {
    let (cw, ch) = (gl_wchar(), gl_hchar());
    let n = n.min(i16::MAX as usize) as i16;
    let textw = c.w.min(n.saturating_mul(cw));
    let texth = c.h.min(ch);
    let numchs = if texth / ch != 0 { n.min(textw / cw) } else { 0 };
    if numchs <= 0 {
        return None;
    }
    let mut y = c.y;
    let slack_h = c.h - texth;
    if slack_h > 0 {
        y += (slack_h + 1) / 2;
    }
    let mut x = c.x;
    let slack_w = c.w - textw;
    if slack_w > 0 && just == TE_CNTR {
        x += (slack_w + 1) / 2;
    }
    Some((x, y, numchs as usize))
}

/// `gsx_tblt(IBM, x, y, numchs)` (`aes/gemgraf.c:500-527`) for the frame:
/// `s` one 8×16 glyph per byte with the cell top at `(x, y)`, in the
/// gadget's text colour and writing mode (`gsx_attr(TRUE, tmode, tcol)`,
/// `aes/gemoblib.c:709`): `Replace` paints every glyph cell's background
/// white (the VDI's replace-mode text), `Transparent` inks only the set bits.
fn draw_label(x: i16, y: i16, s: &[u8], fg: Pel, mode: WriteMode, backend: &dyn RasterBackend, clip: ClipRect) {
    let mut cx = x;
    for &b in s {
        backend.draw_glyph(cx, y, crate::vdi::font::glyph(b as u16), fg, mode, clip);
        cx += gl_wchar();
    }
}

/// Draw the full window frame for `w` (border + gadgets + work area), confined
/// to `clip`. Callers repainting the whole screen pass the full-device clip;
/// the damage-scoped composite ([`redraw_clip`]) passes the union of the moved
/// window's old and new border rects, so a settle-overlap repaint never clears
/// a work area (or overdraws a frame) outside the damage region.
fn draw_frame(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    let b = w.curr;
    let kind = w.kind;
    // The gadget's colour word for this window's topped/untopped state —
    // `w_setcolor(pw, gadget, istop)` on each `w_bldactive` (A79).
    let ink = |g: usize| gadget_ink(w, g, istop);

    // Outer frame — `W_BOX`'s border colour (`gr_crack` bits 15..12 of its
    // word, `0x1101` → BLACK). Only the border: the drop shadow below stays
    // [`DROP_SHADOW_COLOR`] (a documented look decision) and must not follow
    // this word.
    rect_outline(b, ink(W_BOX_IDX).bc, backend, clip);

    // …and the drop shadow that goes with it. `ob_draw` draws the `W_BOX`
    // border and its `SHADOWED` decoration in the same visit, before it
    // descends into the title bar and the gadgets (`aes/gemoblib.c:640-901`),
    // so it is drawn here and in that order. It falls outside `b`, into the
    // strip [`ws_true`] claimed for this window in the owner rectangle list —
    // which is what keeps the desktop and the windows below from painting into
    // it, and what the caller's clip lets through ([`draw_frame_pieces`]).
    draw_shadow(b, backend, clip);

    let ah = adj_hbox();
    let aw = adj_wbox();
    let mut top = b.y + 1; // running top of the next band

    // Title row (name area + closer + fuller). EmuTOS `w_bldactive`'s 3D branch
    // (gemwmlib.c:515-560): the title box is `adj_hbox` tall, each gadget is a
    // `gl_wbox` × `gl_hbox` cell inset by `ADJ3DSTD`, and the closer/fuller
    // advance the name area by a full `adj_wbox`. `MOVER` alone earns the band
    // (`TGADGETS`) — the same test [`borders`] uses, so band and work area agree.
    // Colours come from the gadget words: `W_TITLE` (`G_BOX`, `0x1101`) is a
    // black 1px box over the whole band with a hollow — white — interior;
    // `W_NAME` (`G_BOXTEXT`) is topped `0x11a1` = `IP_2PATT` in BLACK under
    // black text in `MD_REPLACE`, untopped `0x1101` = hollow under transparent
    // text.
    //
    // The title box's `ob_spec 0x00011101` has border thickness 1
    // (`gemwmlib.c:63,84`) and `w_bldactive` gives it `y = 0, h = adj_hbox`
    // inside `W_BOX` (`:514-517`), so its bottom border row is
    // `b.y + adj_hbox - 1`: exactly one row **above** the first row `wm_calc`
    // hands to the work area (`tb = 1 + (adj_hbox - 1)`, `:1808-1810`) — the
    // box is drawn whole, before the gadgets, and never reaches the client
    // region.
    if kind & TGADGETS != 0 {
        let title = Grect { x: b.x, y: b.y, w: b.w, h: ah };
        draw_box_ink(title, ink(W_TITLE_IDX), backend, clip);

        let gy = b.y + ADJ3DSTD;
        let mut name_x = b.x;
        let mut name_w = b.w;
        if kind & CLOSER != 0 {
            let g = Grect { x: b.x + ADJ3DSTD, y: gy, w: GL_WBOX, h: gl_hbox() };
            gadget(g, G_CLOSE, ink(W_CLOSER_IDX), backend, clip);
            name_x += aw;
            name_w -= aw;
        }
        if kind & FULLER != 0 {
            name_w -= aw;
            let g = Grect { x: name_x + name_w + ADJ3DSTD, y: gy, w: GL_WBOX, h: gl_hbox() };
            gadget(g, G_FULL, ink(W_FULLER_IDX), backend, clip);
        }
        // W_NAME sits `ADJ3DSTD` inside what is left (gemwmlib.c:534-537).
        name_x += ADJ3DSTD;
        name_w -= 2 * ADJ3DSTD;
        let nr = Grect { x: name_x, y: gy, w: name_w, h: gl_hbox() };
        // A `MOVER` without a `NAME` still gets the name box, but
        // `w_bldactive` never calls `w_setcolor` for it and sets
        // `gl_aname.te_color = 0` instead (`gemwmlib.c:540-550`): the stored
        // word is not consulted — hollow, white border, no text, `FL3DNONE`.
        // With a `NAME` the gadget is `FL3DACT` (`:553`) → drawn on the
        // 3D-grown rect, whose border meets the title box's edges.
        let (name_ink, name_box) =
            if kind & NAME != 0 { (ink(W_NAME_IDX), grow_3d(nr)) } else { (ink_of(0, false, true), nr) };
        draw_box_ink(name_box, name_ink, backend, clip);
        // The name text as `just_draw` places a `G_BOXTEXT`'s (A105 (2)):
        // `gr_gtext(TE_CNTR, IBM, te_ptext, &c)` with `c = gr_inside(&t,
        // tmpth)` — the rect inside the border of the *drawn* (3D-grown)
        // box, `te_thickness = 1` (`aes/gemoblib.c:735-738`,
        // `gemwmlib.c:154-157`) — nudged one row up for an unselected
        // activator when `acttxtmove` is set (`:742-755`; the name is
        // `FL3DACT` only with a `NAME`, `gemwmlib.c:553`), then justified by
        // [`gr_just_text`]: centred with `(slack + 1) / 2`, left-aligned and
        // cut to whole glyphs once wider than the cell. `gl_aname.te_ptext`
        // is `pw->w_pname` only with a `NAME`, else `""` (`:501`): the
        // caller's buffer as it reads *now*, walked as far as the cell can
        // show (the extra glyph tells a longer string from a fitting one).
        let mut c = inside_1(name_box);
        if kind & NAME != 0 && ACT_TXTMOVE.load(Ordering::Relaxed) != 0 {
            c.y -= 1;
        }
        let mut nbuf = [0u8; LIVE_STR_MAX];
        let name = if kind & NAME != 0 { w.name.read(&mut nbuf, label_walk(c.w)) } else { &nbuf[..0] };
        if let Some((tx, ty, n)) = gr_just_text(TE_CNTR, c, name.len()) {
            draw_label(tx, ty, &name[..n], name_ink.tc, name_ink.text_mode(), backend, clip);
        }
        // Untopped, `W_NAME.ob_state = DISABLED` (`w_bldactive`, both arms,
        // `gemwmlib.c:537,546`): `just_draw`'s state block lays `IP_4PATT` in
        // WHITE, `MD_TRANS`, over the rect inside the border (`gr_inside(&t,
        // th)` `:889`, `bb_fill` `:917-930`) — every glyph pixel on a
        // pattern-set position turns white, the rest is untouched. It comes
        // **before** the 3D effect (`:949-950`), so the bevel below stays
        // solid (reference `falcon-tc-two-windows.png`, A95 (6)).
        if !istop {
            let i = inside_1(name_box);
            let pat = PatternFill {
                fg: WHITE,
                bg: WHITE,
                pattern: &IP_PATTERNS[IP_4PATT as usize],
                mode: WriteMode::Transparent,
            };
            backend.fill_pattern(i.x, i.y, i.w, i.h, pat, clip);
        }
        // The `FL3DACT` name box ends with `add_3d_effect` like every other
        // activator (`:949-950`; reference `falcon-tc-window-c.png` row 97
        // white / row 117 LBLACK across the name, A95 (2)). `FL3DNONE`
        // (`MOVER` alone) gets none.
        if kind & NAME != 0 {
            add_3d_effect(backend, name_box.x, name_box.y, name_box.w, name_box.h, false, 1, name_ink.ic, clip);
        }
        top += ah - 1;
    }

    // Info line — `gl_hbox` tall, *not* adjusted (gemwmlib.c:562-567), which is
    // why [`borders`] charges a full `gl_hbox` for it.
    if kind & INFO != 0 {
        // `W_INFO` is the full-width `G_BOXTEXT` box `(0, adj_hbox, w,
        // gl_hbox)` (`w_bldactive`, `gemwmlib.c:561-567`) whose `gl_ainfo`
        // TEDINFO has `te_thickness = 1` (`:153-157`): border on all four
        // sides — its bottom row is the band's last row, `wm_calc`'s work area
        // starts on the row after it (`tb += gl_hbox`, `:1811-1812`) — hollow
        // inside, then the text in `tc`/`md`. `FL3DNONE` — no growth, no bevel.
        let i = ink(W_INFO_IDX);
        let info_box = Grect { x: b.x, y: top, w: b.w, h: gl_hbox() };
        draw_box_ink(info_box, i, backend, clip);
        // `gl_ainfo.te_ptext = pw->w_pinfo` (`gemwmlib.c:502`) — live, as
        // above — drawn as `just_draw` places a `G_BOXTEXT`'s text (A105 (1)):
        // `gr_gtext(TE_LEFT, IBM, …, gr_inside(&t, 1))` (`gl_asamp`
        // `TE_LEFT`, `te_thickness = 1`, `gemwmlib.c:154-157`;
        // `aes/gemoblib.c:735-738`), no 3D growth and no `movetext` for the
        // `FL3DNONE` gadget (`:521-524`), so the cell is `(1, 1, w - 2,
        // gl_hbox - 2)` of the box and `gr_just` adds `(17 - 16 + 1) / 2 = 1`
        // row (`gemgraf.c:574-576`): glyph cells at `(1, 2)` of the box —
        // reference `falcon-tc-window-c.png`, first ink row of the `5` on
        // frame row 27 for a window whose info band starts on row 23.
        let c = inside_1(info_box);
        let mut ibuf = [0u8; LIVE_STR_MAX];
        let info = w.info.read(&mut ibuf, label_walk(c.w));
        if let Some((tx, ty, n)) = gr_just_text(TE_LEFT, c, info.len()) {
            draw_label(tx, ty, &info[..n], i.tc, i.text_mode(), backend, clip);
        }
        top += gl_hbox();
    }

    // The work area is deliberately **not** painted here. In EmuTOS the window
    // frame is an OBJECT tree (`gl_awind`) in which `W_BOX`, `W_DATA` and
    // `W_WORK` all have the type `G_IBOX` (`aes/gemwmlib.c:62-69`), and
    // `W_DATA`/`W_WORK` carry `ob_spec == 0x00001101` — border thickness 0
    // (`:87-88`). `ob_draw` skips the border of a zero-thickness object
    // (`if (th != 0) gr_box(...)`) and the interior fill of a `G_IBOX`
    // (`if (obtype != G_IBOX) gr_inside(...)`), `aes/gemoblib.c:663-670`. So a
    // frame redraw — a topping, a `WF_NAME`/`WF_INFO` change, a composite —
    // never touches one pixel of the client region: it belongs to the owner,
    // which repaints it on `WM_REDRAW`. Filling it white here erased the
    // content of every window whose info line was rewritten (TeraDesk's
    // `dir_info` does that on every selection change) or that was topped.

    // Right scroll bar (arrows + track + thumb). Draw the bar column's box
    // first — `W_VBAR` is a `G_BOX` (`0x1101`): border on all four sides of
    // the column as `w_bldactive` lays it out ([`vbar_box`]), hollow inside —
    // its gadgets are `gl_wbox` × `gl_hbox` cells inside that `adj_wbox`-wide
    // column, so without the box the `ADJ3DSTD` margins around them would
    // show the desktop through. Like the bottom band below, the box lives
    // here and not in [`draw_vslider`], so the slider-only redraw path never
    // erases the sizer corner it does not repaint.
    if vbar_rect(b, kind).is_some() {
        draw_box_ink(vbar_box(b, kind), ink(W_VBAR_IDX), backend, clip);
    }
    draw_vslider(w, istop, backend, clip);

    // Bottom scroll bar (arrows + track + thumb). Draw the whole bottom band's
    // box first — `W_HBAR` is a `G_BOX` (`0x1101`): border on all four sides
    // of the band as `w_bldactive` lays it out ([`hbar_box`]), hollow inside —
    // so the corner cell next to the right bar is painted even when the band
    // carries no gadgets. The box lives here (not in `draw_hslider`) so the
    // slider-only redraw path never erases the sizer corner it does not
    // repaint. The band exists exactly when [`borders`] charged for it —
    // otherwise the work area reaches the bottom edge and a band box here
    // would paint over the client's rectangle.
    let (havevbar, havehbar) = bars(kind);
    if havehbar {
        draw_box_ink(hbar_box(b, kind), ink(W_HBAR_IDX), backend, clip);
    }
    draw_hslider(w, istop, backend, clip);

    // Corner cell (EmuTOS `W_SIZER`, gemwmlib.c:612-627): drawn whenever the
    // window has both bars or a `SIZER`, `gl_wbox` × `gl_hbox` inset by
    // `ADJ3DSTD` from the bottom-right corner of the data area. The sizer
    // *glyph* is added only for a real `SIZER` (`ob_spec |= 0x06000000`).
    if (havevbar && havehbar) || kind & SIZER != 0 {
        let g = Grect {
            x: b.x + b.w - GL_WBOX - ADJ3DSTD,
            y: b.y + b.h - gl_hbox() - ADJ3DSTD,
            w: GL_WBOX,
            h: gl_hbox(),
        };
        let sizer_ink = ink(W_SIZER_IDX);
        if kind & SIZER != 0 {
            gadget(g, G_SIZE, sizer_ink, backend, clip);
        } else {
            // The glyph-less `W_SIZER` (`ob_spec &= 0x00ffffff`) is still an
            // `FL3DACT` `G_BOXCHAR`: grown slot, border, interior, bevel.
            let slot = grow_3d(g);
            draw_box_ink(slot, sizer_ink, backend, clip);
            add_3d_effect(backend, slot.x, slot.y, slot.w, slot.h, false, 1, sizer_ink.ic, clip);
        }
    }
}

/// EmuTOS's `W_VBAR` object rectangle — the rectangle `w_bldactive` hands
/// `w_bldvbar` (`aes/gemwmlib.c:585-610`): the work area is one pixel inside
/// the data area, narrowed by `adj_wbox - 1` when there is a vertical bar or
/// a `SIZER` and shortened by `adj_hbox - 1` when there is a horizontal bar
/// or a `SIZER`; the bar column then starts at `work_x + work_w`, `adj_wbox`
/// wide, `work_h + 2` tall — so it **shares its bottom row with the
/// horizontal band's top row**. The bar's children — the arrow cells and the
/// `W_VSLIDE` track ([`varrow_cells`], [`vtrack`]) — are laid out inside
/// this rect exactly as `w_bldvbar` does (A95 (4)). [`vbar_rect`] (the
/// hit-test column) stops one row short of the seam; the hit tests keep it.
fn vbar_box(b: Grect, kind: u16) -> Grect {
    let (_, havehbar) = bars(kind);
    let sizer = kind & SIZER != 0;
    let top = band_top(b, kind);
    let data_h = b.y + b.h - top;
    let work_h = data_h - 2 - if havehbar || sizer { adj_hbox() - 1 } else { 0 };
    Grect { x: b.x + b.w - adj_wbox(), y: top, w: adj_wbox(), h: work_h + 2 }
}

/// EmuTOS's `W_HBAR` object rectangle — what `w_bldactive` hands `w_bldhbar`
/// (`aes/gemwmlib.c:585-616`): `x = 0` in the data area (on the window's left
/// border column), `adj_hbox` tall against the bottom edge, `work_w + 2` wide
/// — so it **shares its right column with the vertical bar's left column**.
/// The band's children — the arrow cells and the `W_HSLIDE` track
/// ([`harrow_cells`], [`htrack`]) — are laid out inside this rect exactly as
/// `w_bldhbar` does (A95 (5)). [`hbar_rect`] (the hit-test band) sits one
/// column inside on both sides; the hit tests keep it.
fn hbar_box(b: Grect, kind: u16) -> Grect {
    let (havevbar, _) = bars(kind);
    let sizer = kind & SIZER != 0;
    let work_w = b.w - 2 - if havevbar || sizer { adj_wbox() - 1 } else { 0 };
    Grect { x: b.x, y: b.y + b.h - adj_hbox(), w: work_w + 2, h: adj_hbox() }
}

/// Elevator (thumb) geometry within a scroll track `track_len` px long:
/// returns `(offset, len)` — the thumb starts `offset` px into the track and
/// spans `len` px. `slsize` = `WF_?SLSIZ` (per-mille of the track, or **−1** =
/// "use default size" = one gadget cell), `slpos` = `WF_?SLIDE` (per-mille of
/// the remaining travel), `min_size` = that one gadget cell along the
/// track's axis: **`gl_hbox` for the vertical bar, `gl_wbox` for the
/// horizontal one** — `w_bldvbar` `size = gl_hbox … max(gl_hbox, mul_div(h,
/// w_vslsiz, 1000))` (`gemwmlib.c:431-434`), `w_bldhbar` `size = gl_wbox …
/// max(gl_wbox, mul_div(w, w_hslsiz, 1000))` (`:469-471`); the two differ
/// on a non-square-pixel screen (`gl_wbox = gl_hbox · aspect`,
/// `gemgraf.c:413-415`). The single source of truth for the thumb, shared
/// by the slider draw ([`draw_vslider`]/[`draw_hslider`]) and the frame
/// hit-test / drag ([`gadget_at`]/[`drag_elevator`]) so the clicked thumb is
/// exactly the drawn one.
fn thumb_geom(track_len: i16, slsize: i16, slpos: i16, min_size: i16) -> (i16, i16) {
    let min_thumb = min_size.min(track_len);
    let len = if slsize == -1 {
        min_thumb
    } else {
        ((track_len as i32 * slsize.clamp(1, 1000) as i32 / 1000) as i16).clamp(min_thumb, track_len)
    };
    let travel = track_len - len;
    let pos = slpos.clamp(0, 1000);
    let offset = (travel as i32 * pos as i32 / 1000) as i16;
    (offset, len)
}

/// First row of the window's work area — the running band top after the title
/// and info bands. Identical to `calc(WC_WORK, …).y` by construction (both add
/// [`borders`]' `tb`), which is what keeps the painted bands and the reported
/// work rectangle edge-to-edge.
fn band_top(b: Grect, kind: u16) -> i16 {
    let mut top = b.y + 1;
    if kind & TGADGETS != 0 {
        top += adj_hbox() - 1;
    }
    if kind & INFO != 0 {
        top += gl_hbox();
    }
    top
}

/// Screen rectangle of the vertical scroll-bar column — EmuTOS `W_VBAR`
/// (`gemwmlib.c:408`, placed at `gemwmlib.c:606-610`): `adj_wbox` wide against
/// the window's right edge, from the first work row down to the top of the
/// horizontal band. `None` when [`bars`] gave the window no vertical bar.
fn vbar_rect(b: Grect, kind: u16) -> Option<Grect> {
    let (havevbar, havehbar) = bars(kind);
    if !havevbar {
        return None;
    }
    let aw = adj_wbox();
    let top = band_top(b, kind);
    let bot = b.y + b.h - 1 - if havehbar { adj_hbox() } else { 0 };
    (bot >= top).then_some(Grect { x: b.x + b.w - aw, y: top, w: aw, h: bot - top + 1 })
}

/// Screen rectangle of the horizontal scroll-bar band — EmuTOS `W_HBAR`
/// (`gemwmlib.c:446`, placed at `gemwmlib.c:612-616`): `adj_hbox` tall against
/// the window's bottom edge, from the left border to the vertical bar column.
/// `None` when [`bars`] gave the window no horizontal bar.
fn hbar_rect(b: Grect, kind: u16) -> Option<Grect> {
    let (havevbar, havehbar) = bars(kind);
    if !havehbar {
        return None;
    }
    let ah = adj_hbox();
    let left = b.x + 1;
    let right = b.x + b.w - 1 - if havevbar { adj_wbox() } else { 0 };
    (right >= left).then_some(Grect { x: left, y: b.y + b.h - ah, w: right - left + 1, h: ah })
}

/// The vertical scroll track span `(top, length)` — EmuTOS's `W_VSLIDE`
/// object, the region left for the thumb + paging after the up/down arrows —
/// or `None` if there is no VSLIDE track. `w_bldvbar` (3D, `gemwmlib.c:401-
/// 436`) works on the `W_VBAR` rect ([`vbar_box`], `work_h + 2` tall, down to
/// the seam row): each arrow consumes a full `adj_hbox` of it (`:415-416`,
/// `:421`) and what remains is the slide box (`:429`) — so the track's bottom
/// border row is the down arrow's grown top border (`:424`), one row below
/// where [`vbar_rect`] ends (A95 (4)). Shared with the hit-test / drag
/// ([`gadget_at`]/[`drag_elevator`]) so they agree pixel-for-pixel with the
/// draw, and the elevator's travel is EmuTOS's `h - size` (`:434`).
fn vtrack(w: &Window) -> Option<(i16, i16)> {
    let kind = w.kind;
    if kind & VSLIDE == 0 {
        return None;
    }
    vbar_rect(w.curr, kind)?;
    let bar = vbar_box(w.curr, kind);
    let ah = adj_hbox();
    let top = bar.y + if kind & UPARROW != 0 { ah } else { 0 };
    let len = bar.h
        - if kind & UPARROW != 0 { ah } else { 0 }
        - if kind & DNARROW != 0 { ah } else { 0 };
    (len > 0).then_some((top, len))
}

/// The up/down arrow cells `(up, down)` — EmuTOS `W_UPARROW`/`W_DNARROW`
/// as `w_bldvbar` places them in the `W_VBAR` rect ([`vbar_box`]): the up
/// arrow at `(ADJ3DSTD, ADJ3DSTD)` (`gemwmlib.c:414`), the down arrow at
/// `(ADJ3DSTD, y+h+ADJ3DSTD-1)` with `y`/`h` the space left after the arrows
/// (`:415-416`, `:421-424`) — i.e. `gl_hbox + 1` rows above the bar's last
/// row, so its grown slot ends on the row above the seam (A95 (4)). Each is
/// the un-grown `gl_wbox` × `gl_hbox` cell; `None` for an absent gadget.
fn varrow_cells(b: Grect, kind: u16) -> (Option<Grect>, Option<Grect>) {
    if vbar_rect(b, kind).is_none() {
        return (None, None);
    }
    let bar = vbar_box(b, kind);
    let ah = adj_hbox();
    let up = (kind & UPARROW != 0)
        .then_some(Grect { x: bar.x + ADJ3DSTD, y: bar.y + ADJ3DSTD, w: GL_WBOX, h: gl_hbox() });
    let dn = (kind & DNARROW != 0)
        .then_some(Grect { x: bar.x + ADJ3DSTD, y: bar.y + bar.h - ah + ADJ3DSTD - 1, w: GL_WBOX, h: gl_hbox() });
    (up, dn)
}

/// Screen rectangle of a scroll-bar elevator (thumb) — EmuTOS `W_VELEV` /
/// `W_HELEV` (`gemwmlib.c:436`, `:474`): inset `ADJ3DSTD` inside the slide box
/// on the cross axis at `gl_?box`, and `2*ADJ3DSTD + 1` shorter than the slide
/// length along the drag axis. `track` is the `(start, length)` from
/// [`vtrack`]/[`htrack`], `bar` the enclosing bar column/band.
fn elev_rect(bar: Grect, track: (i16, i16), off: i16, len: i16, vert: bool) -> Grect {
    let (start, _) = track;
    let thick = (len - 2 * ADJ3DSTD - 1).max(1);
    if vert {
        Grect { x: bar.x + ADJ3DSTD, y: start + off + ADJ3DSTD, w: GL_WBOX, h: thick }
    } else {
        Grect { x: start + off + ADJ3DSTD, y: bar.y + ADJ3DSTD, w: thick, h: gl_hbox() }
    }
}

/// Draw the right (vertical) scroll bar's gadgets — up/down arrows, the
/// track, and a thumb positioned + sized from `w.vslide` / `w.vslsize` (both
/// per-mille) — in `W_VBAR`'s child order (`w_bldvbar`, `gemwmlib.c:403-438`).
/// For `draw_frame` (the whole-tree walk) and the drag feedback; drawing only
/// the slider column leaves the work area (content) untouched. The
/// `wind_set(WF_VSLIDE/WF_VSLSIZ)` path draws [`draw_vslide`] alone (A98).
fn draw_vslider(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    draw_varrows(w, istop, backend, clip);
    draw_vslide(w, istop, backend, clip);
}

/// The up/down arrow cells — `W_UPARROW`/`W_DNARROW`, siblings of `W_VSLIDE`
/// under `W_VBAR` (`w_adjust(W_VBAR, W_UPARROW, …)`/`w_adjust(W_VBAR,
/// W_DNARROW, …)`, `gemwmlib.c:412-424`), so **not** part of the slider's
/// subtree redraw.
fn draw_varrows(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    let ink = |g: usize| gadget_ink(w, g, istop);
    // Arrows: the `gl_wbox` × `gl_hbox` cells `w_bldvbar` lays into the
    // `W_VBAR` rect ([`varrow_cells`]).
    let (up, dn) = varrow_cells(w.curr, w.kind);
    if let Some(g) = up {
        gadget(g, G_UP, ink(W_UPARROW_IDX), backend, clip);
    }
    if let Some(g) = dn {
        gadget(g, G_DN, ink(W_DNARROW_IDX), backend, clip);
    }
}

/// The `W_VSLIDE` subtree — the track plus its one child, the `W_VELEV`
/// elevator (`w_adjust(W_VBAR, W_VSLIDE, …)`, `w_adjust(W_VSLIDE, W_VELEV, …)`,
/// `gemwmlib.c:427-436`). Exactly what `w_cpwalk(w_handle, W_VSLIDE,
/// MAX_DEPTH, TRUE)` paints: `ob_draw` walks `obj`'s own subtree and stops at
/// `tree[obj].ob_next` (`gemoblib.c:959-977`), so the arrow siblings stay
/// untouched (A98).
fn draw_vslide(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    let Some(bar) = vbar_rect(w.curr, w.kind) else {
        return;
    };
    let ink = |g: usize| gadget_ink(w, g, istop);
    if let Some((track_y, track_h)) = vtrack(w) {
        let track = Grect { x: bar.x, y: track_y, w: bar.w, h: track_h };
        // `W_VSLIDE` is a `G_BOX` whose default word `0x1111` is `IP_1PATT` in
        // BLACK under a black border.
        draw_box_ink(track, ink(W_VSLIDE_IDX), backend, clip);
        // Thumb geometry shared with `gadget_at`/`drag_elevator` (see
        // [`thumb_geom`]): a stored −1 size renders as one gadget cell, never a
        // zero/negative-height elevator; the stored/returned value stays −1.
        let (off, thumb_h) = thumb_geom(track_h, w.vslsize, w.vslide, gl_hbox());
        let e = elev_rect(bar, (track_y, track_h), off, thumb_h, true);
        draw_elevator(e, ink(W_VELEV_IDX), backend, clip);
    }
}

/// The scroll-bar elevator (`W_VELEV`/`W_HELEV`, `G_BOX` `0x1101`, `FL3DACT`):
/// `just_draw` on the 3D-grown rect — border + interior from the word, the
/// 1-px bevel inside the border (`add_3d_effect`, `gemoblib.c:238-276`) —
/// the same arm as a glyph cell without the glyph.
fn draw_elevator(e: Grect, ink: Ink, backend: &dyn RasterBackend, clip: ClipRect) {
    let slot = grow_3d(e);
    draw_box_ink(slot, ink, backend, clip);
    add_3d_effect(backend, slot.x, slot.y, slot.w, slot.h, false, 1, ink.ic, clip);
}

/// Redraw just window `handle`'s vertical slider **subtree** — track +
/// elevator, not the arrows ([`draw_vslide`]) — for
/// `wind_set(WF_VSLIDE/WF_VSLSIZ)`: EmuTOS `wm_set`'s
/// `w_cpwalk(w_handle, W_VSLIDE, MAX_DEPTH, TRUE)`
/// (`gemwmlib.c:1697-1717,1739-1755`), so `do_walk` clips it to the window's
/// own owner rectangle list rather than letting it draw over a window on top,
/// and `ob_draw` confines it to the `W_VSLIDE` object (`gemoblib.c:959-977`;
/// A98).
fn redraw_vslider(handle: i16) {
    if let (Some(backend), Some(w)) = (host().and_then(|h| h.raster()), win(handle)) {
        if w.open {
            let mut pieces = [Grect::default(); RCAP];
            let n = walk_rects(handle, None, &mut pieces);
            // Bracket the slider repaint with the counting cursor semaphore
            // (EmuTOS `w_update` moff/mon) so a cursor over the scroll bar
            // leaves no dropping. See the note in [`compose`].
            crate::vdi::cursor_hide(backend);
            let istop = handle == get_top();
            for r in pieces.iter().take(n) {
                draw_vslide(w, istop, backend, rect_clip(r.x, r.y, r.w, r.h));
            }
            crate::vdi::cursor_show(backend);
        }
    }
}

/// The horizontal scroll track span `(left, length)` — EmuTOS's `W_HSLIDE`
/// object, the region left for the thumb + paging after the left/right
/// arrows — or `None` if there is no HSLIDE track. Mirror of [`vtrack`] with
/// the axes swapped (x↔y, w↔h): `w_bldhbar` (`gemwmlib.c:441-474`) works on
/// the `W_HBAR` rect ([`hbar_box`], from the window's left border column to
/// the seam column), each arrow consuming a full `adj_wbox` (`:453-454`,
/// `:459`), the slide box in between (`:467`) — its right border column is
/// the right arrow's grown left border (A95 (5)).
fn htrack(w: &Window) -> Option<(i16, i16)> {
    let kind = w.kind;
    if kind & HSLIDE == 0 {
        return None;
    }
    hbar_rect(w.curr, kind)?;
    let bar = hbar_box(w.curr, kind);
    let aw = adj_wbox();
    let left = bar.x + if kind & LFARROW != 0 { aw } else { 0 };
    let len = bar.w
        - if kind & LFARROW != 0 { aw } else { 0 }
        - if kind & RTARROW != 0 { aw } else { 0 };
    (len > 0).then_some((left, len))
}

/// The left/right arrow cells `(left, right)` — EmuTOS `W_LFARROW`/
/// `W_RTARROW` as `w_bldhbar` places them in the `W_HBAR` rect
/// ([`hbar_box`]): the left arrow at `(x+ADJ3DSTD, ADJ3DSTD)` with `x = 0`
/// (`gemwmlib.c:452`) — its grown slot starts on the window's left border
/// column — the right arrow at `(x+w-1+ADJ3DSTD, ADJ3DSTD)` (`:459`), i.e.
/// `gl_wbox + 1` columns before the band's last column, so its grown slot
/// ends on the column before the seam (A95 (5)). Mirror of
/// [`varrow_cells`].
fn harrow_cells(b: Grect, kind: u16) -> (Option<Grect>, Option<Grect>) {
    if hbar_rect(b, kind).is_none() {
        return (None, None);
    }
    let bar = hbar_box(b, kind);
    let aw = adj_wbox();
    let lf = (kind & LFARROW != 0)
        .then_some(Grect { x: bar.x + ADJ3DSTD, y: bar.y + ADJ3DSTD, w: GL_WBOX, h: gl_hbox() });
    let rt = (kind & RTARROW != 0)
        .then_some(Grect { x: bar.x + bar.w - aw + ADJ3DSTD - 1, y: bar.y + ADJ3DSTD, w: GL_WBOX, h: gl_hbox() });
    (lf, rt)
}

/// Draw the bottom (horizontal) scroll bar's gadgets — left/right arrows, the
/// track, and a thumb positioned + sized from `w.hslide` / `w.hslsize` (both
/// per-mille) — in `W_HBAR`'s child order (`w_bldhbar`, `gemwmlib.c:441-476`).
/// The mirror image of [`draw_vslider`] with the axes swapped (x↔y, w↔h): the
/// track runs along the bottom row instead of the right column, offset only by
/// the left border and the vertical scroll bar / sizer column on the right (no
/// title/info bands consume the horizontal axis). For `draw_frame` and the
/// drag feedback; drawing only the slider row leaves the work area (content)
/// untouched. The bottom-band fill is done by the caller. The
/// `wind_set(WF_HSLIDE/WF_HSLSIZ)` path draws [`draw_hslide`] alone (A98).
fn draw_hslider(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    draw_harrows(w, istop, backend, clip);
    draw_hslide(w, istop, backend, clip);
}

/// The left/right arrow cells — `W_LFARROW`/`W_RTARROW`, siblings of
/// `W_HSLIDE` under `W_HBAR` (`gemwmlib.c:450-462`), outside the slider's
/// subtree redraw. Mirror of [`draw_varrows`].
fn draw_harrows(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    let ink = |g: usize| gadget_ink(w, g, istop);
    // Arrows: mirror of [`draw_varrows`] — the cells `w_bldhbar` lays into
    // the `W_HBAR` rect ([`harrow_cells`], gemwmlib.c:448-462).
    let (lf, rt) = harrow_cells(w.curr, w.kind);
    if let Some(g) = lf {
        gadget(g, G_LF, ink(W_LFARROW_IDX), backend, clip);
    }
    if let Some(g) = rt {
        gadget(g, G_RT, ink(W_RTARROW_IDX), backend, clip);
    }
}

/// The `W_HSLIDE` subtree — the track plus its `W_HELEV` child
/// (`gemwmlib.c:465-474`); what `w_cpwalk(w_handle, W_HSLIDE, MAX_DEPTH,
/// TRUE)` paints (`ob_draw`, `gemoblib.c:959-977`). Mirror of
/// [`draw_vslide`] (A98).
fn draw_hslide(w: &Window, istop: bool, backend: &dyn RasterBackend, clip: ClipRect) {
    let Some(bar) = hbar_rect(w.curr, w.kind) else {
        return;
    };
    let ink = |g: usize| gadget_ink(w, g, istop);
    if let Some((track_x, track_w)) = htrack(w) {
        let track = Grect { x: track_x, y: bar.y, w: track_w, h: bar.h };
        // `W_HSLIDE` — the same `G_BOX` `0x1111` word as the vertical track.
        draw_box_ink(track, ink(W_HSLIDE_IDX), backend, clip);
        // Thumb geometry shared with `gadget_at`/`drag_elevator` (see
        // [`thumb_geom`]): a stored −1 size renders as one gadget cell.
        let (off, thumb_w) = thumb_geom(track_w, w.hslsize, w.hslide, GL_WBOX);
        let e = elev_rect(bar, (track_x, track_w), off, thumb_w, false);
        draw_elevator(e, ink(W_HELEV_IDX), backend, clip);
    }
}

/// Redraw just window `handle`'s horizontal slider **subtree** — track +
/// elevator, not the arrows ([`draw_hslide`]) — for
/// `wind_set(WF_HSLIDE/WF_HSLSIZ)`: EmuTOS `wm_set`'s
/// `w_cpwalk(w_handle, W_HSLIDE, MAX_DEPTH, TRUE)`
/// (`gemwmlib.c:1690-1695,1704-1709,1739-1755`); see [`redraw_vslider`] (A98).
fn redraw_hslider(handle: i16) {
    if let (Some(backend), Some(w)) = (host().and_then(|h| h.raster()), win(handle)) {
        if w.open {
            let mut pieces = [Grect::default(); RCAP];
            let n = walk_rects(handle, None, &mut pieces);
            // Counting cursor bracket around the slider repaint — see
            // [`redraw_vslider`] / [`compose`].
            crate::vdi::cursor_hide(backend);
            let istop = handle == get_top();
            for r in pieces.iter().take(n) {
                draw_hslide(w, istop, backend, rect_clip(r.x, r.y, r.w, r.h));
            }
            crate::vdi::cursor_show(backend);
        }
    }
}

/// Resolve a `wind_set` pointer argument — the `WF_NAME`/`WF_INFO` string and
/// the `WF_NEWDESK` tree — for both calling conventions used at our altitude
/// (F7 ABI-width audit):
///
///  * **Native** (the desktop's `desk::wind_set_ptr`, `selftest_objc`): the
///    pointer travels in `addr_in[0]` with `AIN_LEN == 1`.
///  * **Classic AES** (gemlib `wind_set_str`, both ILP32 and LP64): the
///    pointer is packed at `&int_in[2]` with `AIN_LEN == 0`, at the caller's
///    pointer width — 4 bytes ILP32, 8 bytes LP64.
///
/// `AIN_LEN` is the declared discriminator, so an LP64 desktop and an ILP32
/// gemlib app (QED) both resolve to the right slot.
///
/// `wind_get` has no use for this: `wm_get` implements neither title field and
/// refuses them from its `default:` arm without touching a caller buffer
/// (`gemwmlib.c:1591-1592`, TASK/A42).
fn wf_str_ptr(pb: &Aespb) -> u64 {
    if pb.ain_len() >= 1 {
        pb.addrin(0)
    } else {
        pb.intin_ptr(2)
    }
}

// ── dispatch entry points ───────────────────────────────────────────────

/// `wind_create` (100) — allocate a window of border `kind`, max extent
/// `int_in[1..4]`. Returns a handle (≥1) or a negative error.
pub fn wind_create(pb: &Aespb) {
    let kind = pb.intin(0) as u16;
    let full = Grect { x: pb.intin(1), y: pb.intin(2), w: pb.intin(3), h: pb.intin(4) };
    // EmuTOS `wm_create` scans from slot 0 (`gemwmlib.c:1373-1374`); slot 0 is
    // the desktop window and permanently `VF_INUSE`, so the first free handle
    // is 1. The handle *is* the slot index.
    for (i, w) in slots().iter_mut().enumerate() {
        if !w.in_use {
            // **`WS_WORK` survives the slot reset.** `wm_create` runs `w_setup`
            // plus three `w_setsize` calls (`gemwmlib.c:1376-1379`) and neither
            // touches `w_work` (`:205-228`), so what a created-but-unopened
            // window reports is the rectangle the slot's *previous* life left —
            // `gl_rfull`, written by `wm_delete` (`:1480`, [`wm_delete`]). The
            // wholesale reset below would wipe exactly that, so the field is
            // carried across it. A33; measured cases 340 and 344.
            let work = w.work;
            *w = Window::EMPTY;
            w.work = work;
            w.in_use = true;
            w.kind = kind;
            // `w_setup(rlr, i, kind)` (`gemwmlib.c:1376`, `:205-215`) records
            // the *creating* process as `w_owner`; `wm_open` never touches it.
            // Reported by `WF_OWNER` (`:1580`).
            w.owner = super::appl::current_identity() as i16;
            // EmuTOS `wm_create` stores the caller's rectangle in WS_FULL only
            // and zeroes the current/previous ones — `w_setsize(WS_CURR, i,
            // &gl_rzero)` / `w_setsize(WS_PREV, i, &gl_rzero)`, then
            // `w_setsize(WS_FULL, i, pt)` (`gemwmlib.c:1377-1380`). A created
            // but unopened window therefore answers `WF_CXYWH`/`WF_PXYWH` with
            // an all-zero rect (measured: `aes-r2-wind.tsv` cases 230, 232).
            w.full = full;
            w.curr = Grect::default();
            w.prev = Grect::default();
            // **`WS_WORK` is deliberately not written here.** `wm_create` writes
            // three slots and stops (`:1377-1379`), and `w_setup` does not
            // assign `w_work` either (`:205-228`); in the whole AES the work
            // rectangle is written only by `wm_init` for the desktop window
            // (`:1294`), by `wm_delete` (`:1480`) and by `draw_change`/`w_move`
            // (`:1073-1077`) — [`set_curr`]. `wm_get(WF_WXYWH)` is a plain
            // `w_getsize(WS_WORK, …)` (`:1529`, `:236-262`), so a created but
            // unopened window reports the rectangle its *slot* was left with,
            // which no argument of `wind_create` can influence. Computing
            // `wm_calc(WC_WORK, kind, full)` here answered `11,43,276,154` where
            // EmuTOS answered `gl_rfull` — measured, `aes-r3-wind-state.tsv`
            // cases 340 and 344 against `EMUTOS14-AES-r3.BIN` (A34).
            //
            // The positive half of the same two cases is the `gl_rfull` the
            // slot arrives with: [`wm_delete`] writes `WS_CURR`/`WS_PREV` =
            // `gl_rscreen` and `WS_FULL`/`WS_WORK` = `gl_rfull` into every slot
            // it frees (`:1477-1480`), and the `let work` above carries the
            // last of those four past this reset. `WS_CURR`, `WS_PREV` and
            // `WS_FULL` are all overwritten below, so `WS_WORK` is the only one
            // of the four a create leaves visible (A33).
            // Per-window gadget colours inherit the current global defaults
            // (EmuTOS `w_setup`, gemwmlib.c:222-226) so a prior WF_DCOLOR change
            // is reflected in windows created afterwards.
            w.tcolor = *gl_wtcolor();
            w.bcolor = *gl_wbcolor();
            pb.set_ret(i as i16);
            return;
        }
    }
    pb.set_ret(-1); // no free handle
}

/// `wind_open` (101) — display window `int_in[0]` at rect `int_in[1..4]`,
/// on top of the stack. The owner is the process that *created* the window
/// (`w_setup`, `gemwmlib.c:210`); `wm_open` does not re-own it.
pub fn wind_open(pb: &Aespb) {
    let handle = pb.intin(0);
    let r = Grect { x: pb.intin(1), y: pb.intin(2), w: pb.intin(3), h: pb.intin(4) };
    // `oldtop = gl_wtop` (`gemwmlib.c:1090`) — read before this window joins
    // the tree; `DESKWH` stands for `NIL` (nothing open).
    let old_top = get_top();
    if let Some(w) = win(handle) {
        // Re-opening an already-open window is a no-op returning FALSE — no
        // re-top, no re-draw (EmuTOS wm_open, gemwmlib.c:1418-1419:
        // `if (pwin->w_flags & VF_ISOPEN) return FALSE`). The guard sits after
        // the handle-validity check so an invalid handle keeps its own path.
        if w.open {
            pb.set_ret(0);
            return;
        }
        // `wm_update(BEG_UPDATE)` (`:1422`), after both refusals: the opcode
        // waited for the lock before it came here ([`opcode_takes_lock`]).
        update_beg();
        // `draw_change` first shifts the old current rect down into PREV and
        // installs `pt` as the new current one (`gemwmlib.c:1069-1077`) — then
        // `wm_open` overwrites PREV with `pt` as well (`:1427`), so an opened
        // window answers `WF_PXYWH` with the rect the *open* was given, not the
        // one `wind_create` stored (measured: `aes-r2-wind.tsv` case 253).
        let old_curr = w.curr;
        w.prev = old_curr;
        set_curr(w, r);
        w.open = true;
        w.prev = r;
        w.z = bump_z();
        // "if it was an open, then draw the old top guy covered" —
        // `draw_change`'s open/close block (`gemwmlib.c:1194-1209`), reached
        // because the previous top is no longer `W_TREE[ROOT].ob_tail`
        // (`:1194`) and this window now is (`:1091`):
        //
        // ```c
        // if ((oldtop != NIL) && (oldtop != w_handle)) {
        //     w_getsize(WS_PREV, gl_wtop, &pprev);      /* the old WS_CURR  */
        //     if (rc_equal(&pprev, &gl_rzero))          /* :1204-1206       */
        //         w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE);  /* :1207            */
        // }
        // ```
        //
        // `w_cpwalk(…, TRUE)` rebuilds the old top's element tree with `istop
        // = (gl_wtop == w_handle)` now false (`w_bldactive`, `:495`: name
        // `DISABLED`, `:537`, untopped colour words, `:382-397`) and draws it
        // through the old top's **own** rectangle list clipped to its
        // `WS_TRUE` (`:363-379` → `do_walk`, `:286-312`) — every visible piece
        // of that frame, not only the part inside the new window's damage
        // rectangle, and nothing where the new window covers it. That is
        // [`draw_frame_pieces`] with `istop = false` and no damage clip. The
        // `WS_PREV` test is "old `WS_CURR` was the zero rectangle": what a
        // created (`wm_create`, `:1377`) or closed (`wm_close` →
        // `draw_change(…, &gl_rzero)`, `:1449,1073`) window carries into its
        // open. No overlap test is involved — a window opened beside the old
        // top repaints it untopped all the same.
        //
        // Before A94 only `redraw_clip` below ran, so the old top kept its
        // topped stipple and bevels outside the new window's rectangle
        // (`measurements/measurement-2026-09-20-a79/screendumps/final-desk-B-two.png`
        // against the Hatari reference `falcon-tc-two-windows.png`).
        //
        // **The second gate to the same call (A104).** A window that got its
        // rectangle from `wind_set(WF_CXYWH)` while it was created but not
        // open carries a non-zero old `WS_CURR`, so the test above fails for
        // it. Opening such a window *at that very rectangle* is
        // `draw_change`'s **top request** instead — `pt` non-zero, same
        // origin, same size (`:1108-1112`) — and that branch is not the early
        // `return` of `:1113-1116`, because `w_obadd` has just made this
        // window `W_TREE[ROOT].ob_tail` (`wm_open`, `:1425`) and it cannot be
        // `oldtop`, not having been open. What it does first is the identical
        // call, gated on `oldtop != NIL` alone:
        //
        // ```c
        // if (oldtop != NIL) {                          /* :1118 */
        //     w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE);      /* :1120 */
        //     clrold = !(D.w_win[oldtop].w_flags & VF_BROKEN);
        // }
        // ```
        //
        // The remaining branches of that tree (size request, `w_move`'s blit,
        // the shrink union) change the *damage rectangle*, not this call; what
        // of them is modelled, and what is not, is in the spec
        // (`docs/api/aes/window-graphics.md` § `wind_open`, "A preset
        // `WS_CURR`").
        let top_request = r != Grect::default() && r == old_curr;
        if old_top != DESKWH && old_top != handle && (old_curr == Grect::default() || top_request) {
            if let Some(backend) = host().and_then(|h| h.raster()) {
                // Cursor bracket as in [`top_window`]'s `w_cpwalk(oldtop, …)`:
                // the cursor is a save-under sprite, painting under it would
                // leave its backing store stale.
                crate::vdi::cursor_hide(backend);
                draw_frame_pieces(old_top, false, backend, None);
                crate::vdi::cursor_show(backend);
            }
        }
        // The new window is topmost and covers exactly its own rect `r` **plus
        // its drop shadow**; nothing outside that changes. Repaint only that
        // (background + frames, this window on top) rather than the whole
        // screen — the open/close full-screen flicker fix, mirroring the
        // move/size damage-scope. This window's content follows via
        // `send_redraw` (WM_REDRAW).
        //
        // `draw_change`'s open branch takes `c = pt` (`gemwmlib.c:1163-1167`,
        // the "grow that is a superset or an open" case, reached because the
        // created-but-unopened window's current rect is still zero) and its
        // tail then adds the shadow (`:1213-1215`) — [`plus_shadow`].
        let d = plus_shadow(r);
        redraw_clip(d.x, d.y, d.w, d.h);
        send_redraw(handle, w.work);
        update_end(); // `wm_update(END_UPDATE)` (`:1429`)
        pb.set_ret(1);
    } else {
        pb.set_ret(0);
    }
}

/// `wind_close` (102) — hide window `int_in[0]` (the handle stays valid), then
/// broadcast `WM_REDRAW` to every window newly uncovered by the removal.
///
/// EmuTOS `wm_close`→`draw_change`→`w_update` (`gemwmlib.c:1438-1459,1058-1223,
/// 1004-1048`): once the window leaves the tree, the AES repaints the desktop
/// background **directly** (`w_drawdesk`, `:340-360,1217-1219`) plus the
/// remaining window frames over the exposed region `c`, and `w_update` walks
/// every still-open window top-to-bottom posting a `WM_REDRAW` for the part of
/// its now-visible area inside `c` (`w_redraw`, `:887-907`). `c` is the closed
/// window's rect, unioned with the new top window's rect **when the closed
/// window was on top** (so the newly-activated top frame + content repaint —
/// `:1194-1211`). The exposed background gets no message; the AES paints it, as
/// here via [`compose`]/[`redraw_clip`] — matching EmuTOS.
pub fn wind_close(pb: &Aespb) {
    let handle = pb.intin(0);
    // `wm_close` returns FALSE for an invalid handle *and* for a valid but
    // not-open window (EmuTOS gemwmlib.c:1444-1447), TRUE only after an actual
    // uncover. [`close_window`] returns exactly that boolean.
    let ok = close_window(handle);
    pb.set_ret(i16::from(ok));
}

/// Close window `handle` — move its rectangles into the closed state, clear its
/// open flag and repaint the area it vacated. The shared body of [`wind_close`]
/// and the implicit close [`wind_delete`] performs on a still-open window
/// (EmuTOS `wm_close`, gemwmlib.c:1438-1459, invoked from `wm_delete`
/// :1473-1474). Returns `false` for an invalid handle or a window that was not
/// open (no rectangle written, no damage, no message); `true` after the uncover.
///
/// **A closed window's rectangles are not the ones it had.** `wm_close` closes
/// *through* `draw_change(w_handle, &gl_rzero)` (`:1449,1453`), and
/// `draw_change` rewrites all three of them before it so much as looks at
/// `VF_ISOPEN` (`:1069-1077` against the shortcut at `:1085-1086`):
///
/// ```c
/// w_getsize(WS_CURR, w_handle, &c);       /* c = the rect it had        */
/// w_setsize(WS_PREV, w_handle, &c);       /* WS_PREV = the old WS_CURR  */
/// w_setsize(WS_CURR, w_handle, pt);       /* WS_CURR = gl_rzero         */
/// pw = w_getxptr(WS_WORK, w_handle);
/// wm_calc(WC_WORK, D.w_win[w_handle].w_kind,
///             pt->g_x, pt->g_y, pt->g_w, pt->g_h,
///             &pw->g_x, &pw->g_y, &pw->g_w, &pw->g_h);   /* WS_WORK from it */
/// ```
///
/// Only after `draw_change` has returned does `wm_close` clear the flag
/// (`:1454`). So a closed window answers `WF_CXYWH` with `gl_rzero` — 0,0,0,0,
/// `aes/gemgraf.c:425` — `WF_PXYWH` with the rectangle it was closed from, and
/// `WF_WXYWH` with `wm_calc(WC_WORK, kind, 0,0,0,0)`, which is a negative-size
/// rectangle for every window that has a border. `WS_FULL` is not touched;
/// resetting the slot is `wm_delete`'s job (`:1476-1481`).
fn close_window(handle: i16) -> bool {
    let old_top = get_top();
    // `wm_close` refuses **before** it changes anything: an unknown handle
    // (`get_pwin`, `:1443-1445`) and a window that is merely not open
    // (`:1446-1447`) both return FALSE without ever reaching `draw_change`, so
    // neither may have its rectangles rewritten. The second case is not
    // hypothetical — the desktop window is `VF_INUSE` but never `VF_ISOPEN`
    // (measured: `aes-r2-wind.tsv` case 194), and zeroing *its* `WS_CURR` would
    // take the whole screen out of [`find_handle`] and every other hit test.
    let Some(w) = win(handle).filter(|w| w.open) else {
        return false; // no such window, or it was not open
    };
    // `wm_update(BEG_UPDATE)` (`:1450`) — after the two refusals, before
    // anything changes. The opcodes that lead here waited for the lock first
    // ([`opcode_takes_lock`], the dispatcher's gate); the AES's own callers
    // ([`wm_new`], [`wm_new_for_slot`]) nest on a drained or their own hold.
    update_beg();
    let r = w.curr;
    w.prev = r; // WS_PREV = old WS_CURR      (`:1069-1070`)
    set_curr(w, Grect::default()); // WS_CURR = gl_rzero, WS_WORK recalculated (`:1073-1077`)
    // EmuTOS takes the window out of `gl_wtree` with `ob_delete` *before*
    // `draw_change` (`:1452`) and clears `VF_ISOPEN` after it (`:1454`).
    // Our single `open` flag carries both roles — every tree walk
    // ([`get_top`], [`border_pieces`], [`w_update`],
    // [`find_handle`]) keys off it — so it goes down here: after the three
    // rectangle writes, as in `wm_close`, and before the damage below is
    // worked out, as after `ob_delete`.
    w.open = false;
    // Damage region `c`: the vacated rect, plus the new top window's rect
    // when the closed window was the top one (activates the new top frame).
    let new_top = get_top();
    let damage = if handle == old_top {
        // `get_top` answers `DESKWH` when nothing is left to top (EmuTOS
        // `w_top`, gemwmlib.c:315-318); there is then no new top frame to
        // activate, so the damage stays the vacated rect.
        match (new_top != DESKWH).then(|| win(new_top).map(|w| w.curr)).flatten() {
            Some(nt) => union_grect(r, nt),
            None => r,
        }
    } else {
        r
    };
    // `draw_change`'s tail adds the drop shadow to whatever damage it arrived
    // with, unconditionally (`gemwmlib.c:1213-1215`), before both `w_drawdesk`
    // and `w_update`. For a close that is the whole point: the vanishing
    // window's shadow lies outside its border rect, so without this its black
    // band would stay on the screen. Note EmuTOS unions the *current* rect of
    // the new top here (`:1198-1201`), not its `WS_TRUE` one — the single
    // addition below covers both.
    let damage = plus_shadow(damage);
    // AES repaints background + remaining frames directly over the damage,
    redraw_clip(damage.x, damage.y, damage.w, damage.h);
    // then fans WM_REDRAW out to every window whose visible region grew —
    // `w_update(DESKWH, &c, DESKWH, FALSE)`: all of them, top to bottom.
    w_update(DESKWH, damage, DESKWH, false);
    update_end(); // `wm_update(END_UPDATE)` (`:1456`)
    true
}

/// EmuTOS `wm_delete` (`aes/gemwmlib.c:1466-1485`) — free `handle`'s slot for
/// the next `wm_create`, in full:
///
/// ```c
/// pwin = get_pwin(w_handle);
/// if (!pwin) return FALSE;
/// if (pwin->w_flags & VF_ISOPEN) wm_close(w_handle);
/// newrect(gl_wtree, w_handle);                    /* give back recs. */
/// w_setsize(WS_CURR, w_handle, &gl_rscreen);
/// w_setsize(WS_PREV, w_handle, &gl_rscreen);
/// w_setsize(WS_FULL, w_handle, &gl_rfull);
/// w_setsize(WS_WORK, w_handle, &gl_rfull);
/// pwin->w_flags = 0;
/// pwin->w_owner = NULL;
/// ```
///
/// A still-open window is closed first so its frame + content are uncovered —
/// background and lower windows repaint and the newly-exposed windows get
/// `WM_REDRAW` — *before* the slot is freed; otherwise the deleted window would
/// linger on screen as a dead pixel block.
///
/// **The four rectangles are not housekeeping.** `wm_create` writes `WS_CURR`,
/// `WS_PREV` and `WS_FULL` (`:1377-1379`) but never `WS_WORK`, and `w_setup`
/// does not assign `w_work` either (`:205-228`), so the `gl_rfull` written here
/// is what the *next* occupant of the handle reports from
/// `wind_get(WF_WXYWH)` until it is opened — measured, `aes-r3-wind-state.tsv`
/// cases 340 and 344 against `EMUTOS14-AES-r3.BIN`, both `0,19,640,461` (A33).
/// The other three are invisible from outside by construction: a freed slot
/// fails `get_pwin` ([`win`]), so nothing can read it while it is free, and a
/// create overwrites all three before returning the handle. They are written
/// anyway because that is what the function does; only `WS_WORK` is load-bearing.
///
/// EmuTOS leaves `w_kind`, `w_pname`/`w_pinfo`, the slider fields and the gadget
/// colours standing — `w_setup` reassigns every one of them on the next create
/// (`:210-226`) — and so do we.
///
/// Returns `get_pwin`'s verdict: FALSE for an out-of-range or already-free
/// handle, TRUE otherwise.
fn wm_delete(handle: i16) -> bool {
    // `get_pwin(w_handle)` (`:1470-1472`) — refuse before anything is written.
    let Some(was_open) = win(handle).map(|w| w.open) else {
        return false;
    };
    if was_open {
        // `wm_close(w_handle)` (`:1474`) — the established close-uncover path
        // (redraw_clip + w_update), which is `draw_change`'s tail.
        close_window(handle);
    }
    let Some(w) = win(handle) else {
        return false; // unreachable: `close_window` does not free the slot
    };
    // `newrect(gl_wtree, w_handle)` — "give back recs." (`:1476`): the window's
    // owner rectangle list is dumped before the slot is freed, so a later
    // occupant of the handle can never be handed a piece from the previous
    // window's life. Its cursor goes with it; EmuTOS leaves `w_rnext` pointing
    // at the freed list, which is not a contract we reproduce (an index into an
    // emptied array simply reports the end).
    w.rlist = [Grect::default(); RCAP];
    w.rcount = 0;
    w.rnext = 0;
    // `:1477-1480`. `gl_rscreen` is the whole screen, `gl_rfull` the screen
    // below the menu row (`aes/gemgraf.c:423-424`) — [`screen_rect`] and
    // [`desktop_work`], the same pair [`init_desk_window`] writes.
    w.curr = screen_rect();
    w.prev = screen_rect();
    w.full = desktop_work();
    w.work = desktop_work();
    // `pwin->w_flags = 0;` (`:1481`) — `VF_INUSE` and `VF_ISOPEN` both go down.
    w.in_use = false;
    w.open = false;
    // `pwin->w_owner = NULL;` (`:1482`) — no process owns a free slot, so no
    // stale identity can be handed a `WM_REDRAW` for a window that no longer
    // exists.
    w.owner = Window::NO_OWNER;
    true
}

/// `wind_delete` (103) — free the handle `int_in[0]`: the [`wm_delete`] opcode,
/// whose TRUE/FALSE is `int_out[0]` (`aes/gemsuper.c`, `WM_DELETE`). An invalid
/// handle keeps the existing FALSE return.
pub fn wind_delete(pb: &Aespb) {
    let ok = wm_delete(pb.intin(0));
    pb.set_ret(i16::from(ok));
}

/// Write a single scalar to `int_out[1]` with `ret = 1` — the common
/// `wind_get` scalar-field return (WF_TOP / the slider fields).
fn scalar_out(pb: &Aespb, v: i16) {
    pb.set_intout(1, v);
    pb.set_ret(1);
}

/// `wind_get` (104) — read a `WF_*` field of window `int_in[0]`.
pub fn wind_get(pb: &Aespb) {
    let handle = pb.intin(0);
    let field = pb.intin(1);

    // Rectangle-list iteration is computed separately from the slot fields.
    if field == WF_FIRSTXYWH || field == WF_NEXTXYWH {
        // The iterator belongs to a *window*, so the handle is checked first —
        // EmuTOS `wm_get` looks the `WINDOW` up with `get_pwin` and returns
        // FALSE for every field but `WF_TOP`/`WF_SCREEN`/`WF_DCOLOR` before the
        // switch runs (`gemwmlib.c:1513-1524`, `get_pwin` `:1391-1404`); only
        // then does it reach `w_owns`, the one place that reads `w_rlist` and
        // writes `w_rnext` (`:1230-1240,1556-1561`). Touching the list on an
        // invalid handle would not just answer wrongly: it would rebuild or
        // advance the iteration *another* window is in the middle of. Handle 0
        // is legal (the desktop window), and so is a created-but-closed one —
        // both pass `win`.
        if win(handle).is_none() {
            pb.set_ret(0);
            return;
        }
        if field == WF_FIRSTXYWH {
            compute_rlist(handle);
        }
        // How many of the four words `w_owns` writes depends on how the walk
        // ended (`gemwmlib.c:1230-1241`, see [`Owns`]). `poutwds` is
        // `&int_out[1]` (`aes/gemsuper.c:343`, `WM_OX`, `include/aesdefs.h:509`),
        // so word *i* of the rectangle is `int_out[1 + i]`.
        match rlist_owns(handle) {
            Owns::Visible(r) => {
                pb.set_intout(1, r.x);
                pb.set_intout(2, r.y);
                pb.set_intout(3, r.w);
                pb.set_intout(4, r.h);
            }
            Owns::Exhausted { x, y } => {
                pb.set_intout(1, x);
                pb.set_intout(2, y);
                pb.set_intout(3, 0);
                pb.set_intout(4, 0);
            }
            // Loop never entered: `:1240` writes w/h only. Leaving x/y untouched
            // is the point — writing zeroes here would claim a contract EmuTOS
            // does not have (A15). They read back as the dispatcher's zeroed
            // `int_out`, which is our defined stand-in for its `auto` array.
            Owns::Empty => {
                pb.set_intout(3, 0);
                pb.set_intout(4, 0);
            }
        }
        pb.set_ret(1);
        return;
    }

    // WF_TOP is answered from the global stack, independent of `handle` —
    // in particular wind_get(0, WF_TOP) (a desktop-owner query) must return
    // the top window handle, not fall through to the handle-0 screen rect.
    if field == WF_TOP {
        pb.set_intout(1, get_top());
        pb.set_ret(1);
        return;
    }

    // WF_SCREEN (17): the *persistent* menu/alert screen-save buffer —
    // address as a LONG over `int_out[1..2]`, length as a LONG over
    // `int_out[3..4]` (`gsx_mret((LONG *)poutwds, (LONG *)(poutwds + 2))`,
    // `gemwmlib.c:1563-1565`, `gemgsxif.c:125-129`, `poutwds = &int_out[1]`,
    // `gemsuper.c:343`), TRUE. Like WF_TOP it needs no valid handle
    // (`gemwmlib.c:1517-1520`). The buffer itself is `aes::gsxif` (one block
    // of the BDOS pool, taken once by `gsx_malloc` from `sh_tographic`,
    // `gemshlib.c:236`).
    //
    // Which address the caller gets depends on who asks (ADR-023):
    //   * an EL0 process gets the block's own address (`usermem::expose`):
    //     it is a BDOS-pool block, which since ARCH03 stage 1 is in every
    //     process's address space at VA = PA, read-write (`PROT_G` in
    //     FreeMiNT's words). Should the publication fail, the answer is
    //     FALSE: a pointer the caller would fault on is not an answer;
    //   * an EL1 caller (kernel code, a selftest) gets the block's
    //     physical address — the pool address, ADR-021's "raw PA for kernel
    //     callers" — which fits the two WORDs where the high-half alias does
    //     not.
    // The two LONGs are stored as EmuTOS stores them: one native 32-bit store
    // at `&int_out[1]` and one at `&int_out[3]` (`Aespb::set_intout_long`).
    //
    // Without a buffer (the pool could not supply it at boot; a host test
    // without the fixture) the answer stays FALSE with nothing written — the
    // A54 fallback, `wm_get`'s idiom for a field it cannot serve
    // (`default: return FALSE`, `gemwmlib.c:1590-1591`; sanctioned deviation,
    // `docs/COMPATIBILITY.md`). The buffer's size is NewTOS' own — one
    // screen, not `MENU_BUFFER_SIZE` cells — see `gsxif` and the
    // COMPATIBILITY entry (A57).
    if field == WF_SCREEN {
        let Some(buf) = super::gsxif::gsx_mret() else {
            pb.set_ret(0);
            return;
        };
        let addr = match crate::bdos::process::slot_for(crate::bdos::process::current()) {
            None => buf.pa,
            // SAFETY: BDOS lock held (single-CPU trap context). The buffer is
            // a kernel-owned pool block reached through its high-half alias,
            // page-aligned and page-rounded, never freed — it outlives every
            // process that holds the pointer and owns its pages.
            Some(slot) => match unsafe {
                crate::bdos::process::usermem::expose(
                    slot,
                    buf.kva as *const u8,
                    buf.len,
                    tos_types::ProtMode::G,
                )
            } {
                Some(user_va) => user_va,
                None => {
                    pb.set_ret(0);
                    return;
                }
            },
        };
        pb.set_intout_long(1, addr as u32);
        pb.set_intout_long(3, buf.len as u32);
        pb.set_ret(1);
        return;
    }

    // WF_DCOLOR (19): the *global* default gadget colour words. Like WF_TOP /
    // WF_SCREEN it needs no valid handle (EmuTOS gemwmlib.c:1519,1572-1576).
    // gadget number = int_in[2] (pinwds[0]); colour words → int_out[2..3]
    // (poutwds[1..2], D-095), TRUE.
    if field == WF_DCOLOR {
        let g = gadget_index(pb.intin(2));
        pb.set_intout(2, gl_wtcolor()[g]);
        pb.set_intout(3, gl_wbcolor()[g]);
        pb.set_ret(1);
        return;
    }

    let r = match win(handle) {
        Some(w) => match field {
            // The four rectangle fields are plain reads of the stored
            // `WS_WORK`/`WS_CURR`/`WS_PREV`/`WS_FULL` (EmuTOS `wm_get` →
            // `w_getsize`, gemwmlib.c:1528-1541,1596-1597). `WF_WXYWH` is
            // *stored*, not recomputed: for the desktop window `wm_init` puts
            // `gl_rfull` there, which is not `wm_calc(WC_WORK, NONE, screen)`
            // — until a `wind_set(0, WF_TOP)` stores exactly that
            // ([`wm_mktop`], A80).
            WF_WXYWH => w.work,
            WF_CXYWH => w.curr,
            WF_PXYWH => w.prev,
            WF_FXYWH => w.full,
            // Slider position/size scalars → int_out[1] (EmuTOS wm_get,
            // gemwmlib.c:1542-1551). WF_HSLSIZ/WF_VSLSIZ round-trip the −1
            // "use default size" marker unchanged.
            WF_HSLIDE => return scalar_out(pb, w.hslide),
            WF_VSLIDE => return scalar_out(pb, w.vslide),
            WF_HSLSIZ => return scalar_out(pb, w.hslsize),
            WF_VSLSIZ => return scalar_out(pb, w.vslsize),
            // WF_COLOR (18): the per-window topped/untopped colour words for
            // gadget int_in[2] → int_out[2..3] (poutwds[1..2], D-095), TRUE
            // (EmuTOS wm_get, gemwmlib.c:1567-1571).
            WF_COLOR => {
                let g = gadget_index(pb.intin(2));
                pb.set_intout(2, w.tcolor[g]);
                pb.set_intout(3, w.bcolor[g]);
                pb.set_ret(1);
                return;
            }
            // WF_OWNER (20): owner pid, open flag, and the top/bottom window
            // handles (EmuTOS wm_get, AES_VERSION>=0x330 gemwmlib.c:1579-1588;
            // our profile is 0x340). poutwds[0..3] → int_out[1..4].
            WF_OWNER => {
                let (owner, open) = (w.owner, w.open);
                // `pwin->w_owner->p_pid` (`:1580`): the owner's **reported**
                // pid, computed from the stored identity — 0 for a window a
                // chain member created, whichever slot owns it, the
                // accessory's pid (2, 3, …) for one an accessory created
                // (`wm_create` → `w_setup(rlr, …)`, `:1377`, storing
                // `w_owner`, `:212`; spec § "AES process identity and pid",
                // contract (b), A113); 0 as well for an unowned window (the
                // desktop window before `WF_NEWDESK`, EmuTOS `wm_init`'s
                // `rlr`, `:1255,1290`), as it has always reported.
                let owner = usize::try_from(owner).map_or(0, super::appl::reported_pid);
                pb.set_intout(1, owner);
                pb.set_intout(2, i16::from(open));
                // `poutwds[2] = gl_wtop` — the raw variable, so `NIL` when
                // nothing is open, *not* `w_top()`'s `DESKWH` fold.
                pb.set_intout(3, gl_wtop());
                pb.set_intout(4, get_bottom());
                pb.set_ret(1);
                return;
            }
            // Any other field — `WF_KIND` (1), `WF_NAME` (2) and `WF_INFO` (3)
            // among them, none of which `wm_get` handles: they fall to
            // `default: return FALSE` and write nothing, neither a `poutwds`
            // word nor a byte of any buffer the caller passed
            // (`gemwmlib.c:1591-1592`). The two title fields exist in `wm_set`
            // only (`:1660`, `:1668`) — the AES stores a title, it does not
            // hand one back.
            //
            // We used to answer all three. `WF_KIND` was deliberate deviation
            // D-053 / AUD-P5-014; `WF_NAME`/`WF_INFO` copied the stored string
            // out and returned TRUE because **gemlib declares the read**
            // (`a_wind_get.c:344` packs a `char*` into `int_in[2..]`). That was
            // the wrong authority: gemlib is the client-side binding, and a
            // binding can declare a call the AES on the other side of the trap
            // never implements. What a `wind_get` *does* is decided here, and
            // on real TOS it is refused — measured on EmuTOS 1.4, case 102 of
            // `tosdump/cases/aes-m05-addrin.tsv`: `int_out[0] = 0` on a window
            // whose title the prelude had just set (TASK/A42). Both deviations
            // are dropped; outside the hardware layer NewTOS does not deviate
            // from EmuTOS on purpose.
            _ => {
                pb.set_ret(0);
                return;
            }
        },
        None => {
            pb.set_ret(0);
            return;
        }
    };
    pb.set_intout(1, r.x);
    pb.set_intout(2, r.y);
    pb.set_intout(3, r.w);
    pb.set_intout(4, r.h);
    pb.set_ret(1);
}

/// `wind_set` (105) — set a `WF_*` field of window `int_in[0]`.
///
/// EmuTOS `wm_set` grabs the window sync around its whole body, after the one
/// refusal — an unknown handle for any field but `WF_DCOLOR` — and gives it
/// up on every path out, the unknown-field `FALSE` included: `wm_update(
/// BEG_UPDATE); /* grab the window sync */` … `wm_update(END_UPDATE); /* give
/// up the sync */` (`aes/gemwmlib.c:1632-1638,1757`). The opcode waited for
/// the lock before it came here ([`opcode_takes_lock`]).
pub fn wind_set(pb: &Aespb) {
    let takes = pb.intin(1) == WF_DCOLOR || win(pb.intin(0)).is_some();
    if takes {
        update_beg();
    }
    wm_set(pb);
    if takes {
        update_end();
    }
}

/// The body of [`wind_set`] — EmuTOS `wm_set` (`aes/gemwmlib.c:1624-1760`)
/// between its `wm_update` pair.
fn wm_set(pb: &Aespb) {
    let handle = pb.intin(0);
    let field = pb.intin(1);
    // WF_DCOLOR (19) writes the *global* default gadget colour words and needs
    // no valid handle (EmuTOS wm_set, gemwmlib.c:1632-1636,1727-1734): gadget =
    // int_in[2], tcolor = int_in[3], bcolor = int_in[4] (−1 = leave unchanged).
    // No window redraw (EmuTOS sets `gadget = -1` → no w_cpwalk): the new
    // defaults reach only the windows `wind_create` sets up afterwards
    // (`w_setup`, `:220-227`), whose [`draw_frame`] then paints from them.
    if field == WF_DCOLOR {
        let g = gadget_index(pb.intin(2));
        let (tc, bc) = (pb.intin(3), pb.intin(4));
        if tc != -1 {
            gl_wtcolor()[g] = tc;
        }
        if bc != -1 {
            gl_wbcolor()[g] = bc;
        }
        pb.set_ret(1);
        return;
    }
    if win(handle).is_none() {
        pb.set_ret(0);
        return;
    }
    match field {
        // WF_NEWDESK registers the desktop-background tree on the window named
        // by the handle — in practice the desktop window, `DESKWH` = 0, which
        // is a real slot since [`wm_init`]. EmuTOS also re-owns that window to
        // the caller (`pwin->w_owner = rlr`, `gemwmlib.c:1684-1687`).
        WF_NEWDESK => {
            if let Some(w) = win(handle) {
                w.owner = super::appl::current_identity() as i16;
            }
            set_newdesk(pb);
        }
        WF_NAME => {
            // The title pointer arrives two ways (see `wf_str_ptr`): the
            // desktop's native `addr_in[0]` (AIN_LEN=1), or the classic-AES
            // `&int_in[2]` packing at the caller's pointer width (gemlib
            // `wind_set_str`, AIN_LEN=0) — the latter is how ILP32 apps
            // (QED) pass it. Reading `addr_in[0]` unconditionally handed an
            // ILP32 caller an empty slot → garbage title (F7 audit).
            //
            // `pwin->w_pname = *(char **)pinwds` (`gemwmlib.c:1661`): the
            // **pointer** is stored, no copy — the frame build reads the
            // caller's buffer live ([`LiveStr`], A83). The slot of the
            // calling process goes with it, so that a later frame build from
            // another process's AES call reads it through the owner's
            // mapping.
            let title = wf_str_ptr(pb);
            if let Some(w) = win(handle) {
                w.name = LiveStr::of_caller(title);
            }
            redraw_open(handle);
        }
        WF_INFO => {
            // `pwin->w_pinfo = *(char **)pinwds` (`gemwmlib.c:1669`) — as
            // `WF_NAME`: the pointer and its owner, read live.
            let info = wf_str_ptr(pb);
            if let Some(w) = win(handle) {
                w.info = LiveStr::of_caller(info);
            }
            redraw_open(handle);
        }
        WF_CXYWH => {
            let r = Grect { x: pb.intin(2), y: pb.intin(3), w: pb.intin(4), h: pb.intin(5) };
            // `draw_change` stores the geometry first — `WS_PREV := WS_CURR`,
            // `WS_CURR := pt`, `WS_WORK := wm_calc(WC_WORK, …)`
            // (`gemwmlib.c:1069-1077`) — and then bails out before touching
            // the window tree when the window is created but not open
            // (`:1085-1086`): no repaint, no `WM_REDRAW`. Since `wm_create`
            // leaves the current rect at `gl_rzero`, repainting here would
            // smear the union with an all-zero rectangle across the top-left
            // of the screen.
            let (prev, open) = if let Some(w) = win(handle) {
                w.prev = w.curr;
                set_curr(w, r);
                (w.prev, w.open)
            } else {
                (r, false)
            };
            if open {
                // The rest of `draw_change` (`:1089-1222`): the top window's
                // pure move is **blitted** (`w_move`) and the window is then
                // left out of the `WM_REDRAW` fan-out (`moved`); everything
                // else is repainted over the damage rectangle and the windows
                // it reaches are asked for their content, top to bottom.
                draw_change_open(handle, r, prev);
            }
        }
        WF_TOP => {
            // `if (w_handle != gl_wtop) wm_mktop(w_handle)` — EmuTOS `wm_set`
            // (`gemwmlib.c:1679-1683`). A `WF_TOP` on the window that is
            // *already* on top does nothing whatsoever: no repaint, no message.
            // (Topping it anyway used to run the whole composite over the
            // window's rectangle, which erased its content; TeraDesk answers
            // every `WM_TOPPED` with a `wind_set(WF_TOP)`, so this ran on every
            // click on a window.)
            //
            // The comparison is against the **raw** `gl_wtop` — `NIL` while
            // nothing is open — not against `w_top()`'s `DESKWH` fold: the
            // desktop handle is never `gl_wtop`, so `wind_set(0, WF_TOP)`
            // always reaches `wm_mktop(0)`, with or without an open window
            // (A80). No `WM_UNTOPPED` here: EmuTOS sends it solely from the
            // Control Manager's `perform_untop` on the click
            // (`aes/gemctrl.c:146-153,174,344`), which is [`intercept`]'s job.
            if handle != gl_wtop() {
                wm_mktop(handle);
            }
        }
        // Slider positions clamp to 1..1000; sizes clamp to 1..1000 but let −1
        // ("use default size") pass through (EmuTOS wm_set, gemwmlib.c:1643-1656).
        // Each arm stores and redraws **only when the value changed** — `if
        // (pwin->w_vslide != pinwds[0]) { pwin->w_vslide = pinwds[0]; gadget =
        // W_VSLIDE; }` (`gemwmlib.c:1690-1716`): an unchanged value leaves
        // `gadget == -1`, so `w_cpwalk` is never reached (`:1739-1755`) and
        // not one draw operation is issued (A84).
        WF_VSLIDE => {
            let v = pb.intin(2).clamp(1, 1000);
            if win(handle).is_some_and(|w| store_if_changed(&mut w.vslide, v)) {
                redraw_vslider(handle);
            }
        }
        WF_VSLSIZ => {
            let v = slider_size(pb.intin(2));
            if win(handle).is_some_and(|w| store_if_changed(&mut w.vslsize, v)) {
                redraw_vslider(handle);
            }
        }
        WF_HSLIDE => {
            let v = pb.intin(2).clamp(1, 1000);
            if win(handle).is_some_and(|w| store_if_changed(&mut w.hslide, v)) {
                redraw_hslider(handle);
            }
        }
        WF_HSLSIZ => {
            let v = slider_size(pb.intin(2));
            if win(handle).is_some_and(|w| store_if_changed(&mut w.hslsize, v)) {
                redraw_hslider(handle);
            }
        }
        // WF_COLOR (18): store the per-window colour words for gadget int_in[2]
        // — tcolor = int_in[3], bcolor = int_in[4] (−1 = leave unchanged) —
        // EmuTOS wm_set gemwmlib.c:1719-1726, then redraw the window's frame
        // with them (`do_cpwalk` → `w_cpwalk(w_handle, gadget, MAX_DEPTH,
        // TRUE)`, `:1754-1755`, clipped to the window's own rectangle list):
        // [`draw_frame`] cracks the stored word of every gadget it paints
        // ([`gadget_ink`]), so the re-emit shows the new colours (A79).
        WF_COLOR => {
            let g = gadget_index(pb.intin(2));
            let (tc, bc) = (pb.intin(3), pb.intin(4));
            if let Some(w) = win(handle) {
                if tc != -1 {
                    w.tcolor[g] = tc;
                }
                if bc != -1 {
                    w.bcolor[g] = bc;
                }
            }
            redraw_open(handle);
        }
        // Unknown or unimplemented field → FALSE (EmuTOS wm_set default:
        // gemwmlib.c:1736). WF_NEWDESK/WF_DCOLOR are handled above; WF_OWNER is
        // get-only (EmuTOS has no wm_set WF_OWNER case) → FALSE here.
        _ => {
            pb.set_ret(0);
            return;
        }
    }
    pb.set_ret(1);
}

/// Validate a slider *size* (`WF_HSLSIZ`/`WF_VSLSIZ`): clamp to 1..1000, but
/// pass −1 ("use default size") through unclamped (EmuTOS gemwmlib.c:1643-1648).
fn slider_size(v: i16) -> i16 {
    if v == -1 {
        -1
    } else {
        v.clamp(1, 1000)
    }
}

/// The slider arms' change guard — `if (pwin->w_hslsiz != pinwds[0]) {
/// pwin->w_hslsiz = pinwds[0]; … }` (`gemwmlib.c:1691-1694`, likewise
/// `:1698`, `:1705`, `:1712`): store `v` into `field` and report whether it
/// differed, which is what decides the slider redraw. `v` is the already
/// validated (clamped, or −1) value, as `pinwds[0]` is by then (`:1643-1656`).
fn store_if_changed(field: &mut i16, v: i16) -> bool {
    if *field == v {
        false
    } else {
        *field = v;
        true
    }
}

/// Redraw just `handle`'s frame if it is open (for name/info changes) — EmuTOS
/// `wm_set`'s `w_cpwalk(w_handle, W_NAME/W_INFO, MAX_DEPTH, TRUE)`
/// (`gemwmlib.c:1660-1675,1753-1754`), i.e. `do_walk` over the window's own
/// owner rectangle list: a window that another one covers repaints its title
/// only where it is actually visible.
fn redraw_open(handle: i16) {
    if let (Some(backend), Some(w)) = (host().and_then(|h| h.raster()), win(handle)) {
        if w.open {
            // Counting cursor bracket around the single-frame repaint (EmuTOS
            // `w_update` moff/mon) — see [`compose`].
            crate::vdi::cursor_hide(backend);
            draw_frame_pieces(handle, handle == get_top(), backend, None);
            crate::vdi::cursor_show(backend);
        }
    }
}

/// `wind_find` (106) — handle of the window under `(int_in[0], int_in[1])`;
/// 0 = the desktop window, −1 when the point is on no window at all (off the
/// screen) — see [`find_handle`].
pub fn wind_find(pb: &Aespb) {
    pb.set_ret(find_handle(pb.intin(0), pb.intin(1)));
}

/// Does window opcode `opcode` reach a `wm_update(BEG_UPDATE)` — and so wait
/// while another process holds the lock (`tak_flag` → `ev_block(MU_MUTEX, …)`,
/// `aes/gemwmlib.c:1785-1786`)? Each takes it after its own refusals and not
/// for a call it refuses:
///
/// * `wind_open` (101) — `wm_open`, for a window that exists and is not open
///   (`:1415-1422`);
/// * `wind_close` (102) — `wm_close`, for a window that is open
///   (`:1443-1450`);
/// * `wind_delete` (103) — `wm_delete` of an open window closes it through
///   `wm_close` (`:1473-1474`); a closed one is freed without the lock;
/// * `wind_set` (105) — `wm_set`, for a window that exists, or for
///   `WF_DCOLOR`, which needs none (`:1632-1638`).
///
/// The dispatcher waits ([`update_block`]) before the opcode commits anything
/// (A118), as it does for the `fm_own(TRUE)` opcodes (GH1).
pub(super) fn opcode_takes_lock(opcode: i16, pb: &Aespb) -> bool {
    let handle = pb.intin(0);
    match opcode {
        101 => win(handle).is_some_and(|w| !w.open),
        102 | 103 => win(handle).is_some_and(|w| w.open),
        105 => pb.intin(1) == WF_DCOLOR || win(handle).is_some(),
        _ => false,
    }
}

/// `wind_update` (107) — window-tree screen lock (BEG/END_UPDATE) and mouse-
/// control-ownership transfer (BEG/END_MCTRL).
///
/// EmuTOS `wm_update` (gemwmlib.c:1778-1797): mode `< 2` takes/releases the
/// counting window-tree lock `wind_spb`; mode `>= 2` calls `fm_own(mode - 2)`,
/// the mouse-control-ownership transfer. `fm_own` (gemfmlib.c:82-107) brackets
/// with BEG/END_UPDATE and keeps the `ml_ocnt` counter. While a `BEG_MCTRL` is
/// in effect (`ml_ocnt > 0`) the app has taken over the mouse, so the AES frame
/// interception ([`intercept`]) switches off; the `ml_ocnt == 0` edge bodies
/// (menu-tree hold, control rectangle, `ct_chgown`) are [`mctrl_own`]'s.
///
/// **A taking mode blocks while another process holds the lock** —
/// `if (!tak_flag(&wind_spb)) ev_block(MU_MUTEX, (LONG)&wind_spb);`
/// (`gemwmlib.c:1785-1786`), reached directly for `BEG_UPDATE` and through
/// `fm_own(TRUE)`'s own `wm_update(BEG_UPDATE)` (`gemfmlib.c:86`) for
/// `BEG_MCTRL`. The wait ([`update_block`]) comes first and commits nothing,
/// so a call that has to give the CPU away is re-executed from the top when
/// the lock has been handed to it (GH1).
pub fn wind_update(pb: &Aespb) -> Option<crate::bdos::process::sched::Switch> {
    // EmuTOS `wm_update` (gemwmlib.c:1778-1797): mode `< 2` is the window-tree
    // lock (0 = END_UPDATE, any non-zero = BEG_UPDATE); mode `>= 2` is
    // `fm_own(mode - 2)` (non-zero → begin the mouse-control transfer). The two
    // guard arms reproduce that for out-of-range modes.
    let mode = pb.intin(0);
    let takes = if mode < 2 { mode != 0 } else { mode - 2 != 0 };
    if takes {
        if let Some(sw) = update_block() {
            return Some(sw);
        }
    }
    match mode {
        END_UPDATE => update_end(),
        BEG_UPDATE => update_beg(),
        END_MCTRL => mctrl_own(false),
        BEG_MCTRL => mctrl_own(true),
        m if m < 2 => update_beg(), // negative: non-zero, `< 2` → BEG_UPDATE
        m => mctrl_own(m - 2 != 0), // `> 3`: fm_own(m-2) begins
    }
    pb.set_ret(1);
    None
}

/// EmuTOS `rlr` for the window-tree lock: [`CTL_PD`] while the inline Control
/// Manager runs ([`CTL_RUNNING`]), else the calling AES identity.
fn rlr() -> usize {
    if CTL_RUNNING.load(Ordering::Relaxed) {
        CTL_PD
    } else {
        super::appl::current_identity()
    }
}

/// `sy_owner == rlr` for two holders of the lock. EmuTOS compares `AESPD`s;
/// NewTOS identities fold back to them as the mouse owner's do
/// ([`input::same_pd`](super::input::same_pd): one descriptor for the whole
/// application chain, one per desk accessory). `ctl_pd` is only itself.
fn same_holder(a: usize, b: usize) -> bool {
    if a == CTL_PD || b == CTL_PD {
        return a == b;
    }
    a != NO_OWNER && b != NO_OWNER && super::input::same_pd(a, b)
}

/// `tak_flag(&wind_spb)` (EmuTOS `aes/gemflag.c:68-86`) — take the counting
/// window-tree lock for `me` if it is free or `me`'s already:
///
/// ```c
/// sy->sy_tas++;
/// if ((sy->sy_owner != rlr) && (sy->sy_tas != 1))
///     sy->sy_tas--;           /* refused: wm_update blocks on sy_wait */
/// else
///     sy->sy_owner = rlr;     /* free, or we already had it */
/// return (sy->sy_owner == rlr);
/// ```
///
/// A lock [`unsync`] handed to `me` while it slept on the wait line is already
/// counted for it (`sy_tas = 1`, `:122`); the re-executed call collects the
/// hand-over here ([`UPDATE_GRANTED`]) and counts nothing. Ownership moves on
/// the 0 → 1 edge only: a nested take by another identity of the same
/// application chain ([`same_holder`]) keeps the first taker as the recorded
/// owner and counts its own share in [`UPDATE_HOLDS`].
fn tak_flag(me: usize) -> bool {
    if UPDATE_GRANTED.get(me).is_some_and(|g| g.swap(false, Ordering::Relaxed)) {
        return true;
    }
    let tas = UPDATE_LOCK.load(Ordering::Relaxed);
    if tas <= 0 {
        UPDATE_LOCK.store(1, Ordering::Relaxed);
        UPDATE_OWNER.store(me, Ordering::Relaxed);
        hold_add(me);
        return true;
    }
    if same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me) {
        UPDATE_LOCK.store(tas + 1, Ordering::Relaxed);
        hold_add(me);
        return true;
    }
    false
}

/// May `me` take the lock without waiting — `tak_flag` would succeed.
fn may_take(me: usize) -> bool {
    UPDATE_GRANTED.get(me).is_some_and(|g| g.load(Ordering::Relaxed))
        || UPDATE_LOCK.load(Ordering::Relaxed) <= 0
        || same_holder(UPDATE_OWNER.load(Ordering::Relaxed), me)
}

/// Count one more hold for identity `me` in [`UPDATE_HOLDS`] (`ctl_pd` and the
/// unowned are counted in [`UPDATE_LOCK`] only).
fn hold_add(me: usize) {
    if let Some(h) = UPDATE_HOLDS.get(me) {
        h.fetch_add(1, Ordering::Relaxed);
    }
}

/// Take one hold off [`UPDATE_HOLDS`] for an `unsync` by `me`. EmuTOS's
/// `unsync` does not look at the caller at all — a stray `END_UPDATE` simply
/// counts the owner's lock down — so when `me` has no hold of its own the one
/// taken off is the recorded owner's, else any holder's.
fn hold_take(me: usize) {
    let dec = |i: usize| -> bool {
        match UPDATE_HOLDS.get(i) {
            Some(h) if h.load(Ordering::Relaxed) > 0 => {
                h.fetch_sub(1, Ordering::Relaxed);
                true
            }
            _ => false,
        }
    };
    if dec(me) || dec(UPDATE_OWNER.load(Ordering::Relaxed)) {
        return;
    }
    for i in 0..UPDATE_HOLDS.len() {
        if dec(i) {
            return;
        }
    }
}

/// Append `me` to the lock's wait line unless it is on it already — `amutex`'s
/// `evinsert(e, &sy->sy_wait)` (`aes/gemflag.c:97-101`), once per refused
/// `wm_update` (a re-executed call is still the same `EVB`).
fn wait_append(me: usize) {
    let n = UPDATE_WAIT_LEN.load(Ordering::Relaxed);
    if UPDATE_WAIT[..n].iter().any(|w| w.load(Ordering::Relaxed) == me) || n >= UPDATE_WAIT.len() {
        return;
    }
    UPDATE_WAIT[n].store(me, Ordering::Relaxed);
    UPDATE_WAIT_LEN.store(n + 1, Ordering::Relaxed);
}

/// Take `me` off the wait line, keeping the others' order.
fn wait_remove(me: usize) {
    let n = UPDATE_WAIT_LEN.load(Ordering::Relaxed);
    let Some(pos) = UPDATE_WAIT[..n].iter().position(|w| w.load(Ordering::Relaxed) == me) else {
        return;
    };
    for i in pos..n - 1 {
        UPDATE_WAIT[i].store(UPDATE_WAIT[i + 1].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    UPDATE_WAIT[n - 1].store(NO_OWNER, Ordering::Relaxed);
    UPDATE_WAIT_LEN.store(n - 1, Ordering::Relaxed);
}

/// The head of the wait line, taken off — `p = sy->sy_wait; sy->sy_wait =
/// p->e_link;` (`aes/gemflag.c:118-120`).
fn wait_pop() -> Option<usize> {
    let n = UPDATE_WAIT_LEN.load(Ordering::Relaxed);
    if n == 0 {
        return None;
    }
    let head = UPDATE_WAIT[0].load(Ordering::Relaxed);
    wait_remove(head);
    Some(head)
}

/// The `ev_block(MU_MUTEX, (LONG)&wind_spb)` half of `wm_update(BEG_UPDATE)`
/// (`aes/gemwmlib.c:1785-1786`; `amutex`, `aes/gemflag.c:89-101`): wait until
/// the calling identity may take the lock — it is free, or held by the
/// caller's own process, or [`unsync`] has handed it over. Takes nothing; the
/// caller's [`update_beg`] (directly, or inside `fm_own(TRUE)`) does that
/// right after, with nothing in between that could give the CPU away.
///
/// Every AES opcode that reaches a `wm_update(BEG_UPDATE)` waits here **first**,
/// before it has committed anything — `wind_update` itself, and the opcodes
/// whose body opens with `fm_own(TRUE)` (`form_do`, `form_alert`,
/// `form_error`, `fsel_input`/`fsel_exinput`, `menu_popup`). A refused caller
/// is appended to the wait line and sleeps until the lock is handed to it
/// (`sched::WAKE_MUTEX`); then the call — the S2 restartable block —
/// re-executes from its `svc` and finds the hand-over. Returns the
/// [`Switch`](crate::bdos::process::sched::Switch) to give the CPU away with,
/// or `None` when the caller may go on.
///
/// While the Control Manager holds the lock across `ct_msgup`'s wait for the
/// selecting press to come up (`while (button & 0x0001) dsptch();`,
/// `aes/gemctrl.c:100-101`, inside `ctlmgr`'s `BEG_UPDATE`, `:463,484`), the
/// release of that press is what frees the lock; the waiter therefore also
/// wakes on input and lets the Control Manager finish
/// ([`input::ct_msgup_busy`](super::input::ct_msgup_busy)) — our Control
/// Manager has no process of its own to do it.
///
/// No host means nothing can run to release the lock: the caller is not
/// parked, and [`update_beg`] nests the count (host-side unit tests only).
pub(super) fn update_block() -> Option<crate::bdos::process::sched::Switch> {
    use crate::bdos::process::sched;
    let me = rlr();
    if me == CTL_PD || host().is_none() {
        return None;
    }
    loop {
        // The Control Manager's progress: `ct_msgup` ends with the release.
        let _ = super::input::ct_msgup_busy();
        if may_take(me) {
            wait_remove(me);
            return None;
        }
        wait_append(me);
        let mut events = sched::WAKE_MUTEX;
        if UPDATE_OWNER.load(Ordering::Relaxed) == CTL_PD {
            events |= sched::WAKE_INPUT;
        }
        let key = sched::wait_multi(events, if me < super::pd::NUM_PDS { me } else { 0 });
        if let sched::SleepOutcome::Yield(sw) = sched::sleep(key) {
            return Some(sw); // parked on `sy_wait`; re-executed on the hand-over
        }
    }
}

/// `wm_update(BEG_UPDATE)` — take the lock for the caller ([`tak_flag`]).
///
/// Reached after [`update_block`] (an opcode — `wind_update`, the
/// `fm_own(TRUE)` opcodes, and the AES's own takers behind opcodes 19, 70,
/// 71, 90, 91 and [`opcode_takes_lock`]'s four, A118) or from inside the AES
/// with the lock already the caller's (an `fm_own(TRUE)` nested in a dialog,
/// the Control Manager's own drags), so the take succeeds. The exceptions are
/// takers that cannot give the CPU away — a host-less unit test, and the
/// `Pexec`-child sweep's `wm_close`s ([`wm_new_for_slot`], run at the next AES
/// dispatch in the living caller's name) — finding the lock held by another
/// process: they cannot park, and their hold is counted on top of the owner's
/// instead (the owner keeps the lock; the matching `END_UPDATE` takes the
/// count back off). The shell loop's `fm_error` alert parks like any client
/// since A118: it is issued through the GEM trap (`kernel/src/shell_loop.rs`).
pub(super) fn update_beg() {
    let me = rlr();
    if !tak_flag(me) {
        UPDATE_LOCK.fetch_add(1, Ordering::Relaxed);
        hold_add(me);
    }
}

/// `all_run`'s "then get in the wait line" (`aes/geminit.c:564-565`):
/// `wm_update(BEG_UPDATE); wm_update(END_UPDATE);` — wait behind whoever holds
/// the window-tree lock ([`update_block`]), then take it and give it back at
/// once. `Some` = parked; the caller's call re-executes on the hand-over.
pub(super) fn update_wait_line() -> Option<crate::bdos::process::sched::Switch> {
    if let Some(sw) = update_block() {
        return Some(sw);
    }
    update_beg();
    update_end();
    None
}

/// `wm_update(END_UPDATE)` — [`unsync`] in the caller's name.
pub(super) fn update_end() {
    unsync(rlr());
}

/// `unsync(&wind_spb)` (EmuTOS `aes/gemflag.c:105-130`) — release one count of
/// the window-tree lock: never below zero (a stray `END_UPDATE` with the count
/// already 0 is ignored, `:109-113`), and on the 1 → 0 edge give the lock to
/// the head of the wait line —
///
/// ```c
/// if ((p = sy->sy_wait) != 0) {
///     sy->sy_wait = p->e_link;
///     sy->sy_owner = p->e_pd;
///     sy->sy_tas = 1;           /* restart counting semaphore */
///     azombie(p, 0);
///     dsptch();
/// } else
///     sy->sy_owner = NULL;
/// ```
///
/// `azombie` is the wake ([`sched::wait_mutex`](crate::bdos::process::sched::wait_mutex));
/// the `dsptch()` is the dispatch every completed AES call ends with
/// (`sched::dispatch_after_call`), which hands the CPU to the woken waiter
/// before the caller goes on. `me` is the releasing identity, used only for the
/// per-identity share ([`hold_take`]).
fn unsync(me: usize) {
    let tas = UPDATE_LOCK.load(Ordering::Relaxed);
    if tas <= 0 {
        UPDATE_LOCK.store(0, Ordering::Relaxed);
        return;
    }
    hold_take(me);
    let tas = tas - 1;
    UPDATE_LOCK.store(tas, Ordering::Relaxed);
    if tas > 0 {
        // The recorded owner gave its last share back while another identity
        // of the same process still holds one: the lock stays with that
        // process, recorded under the remaining holder.
        let owner = UPDATE_OWNER.load(Ordering::Relaxed);
        if UPDATE_HOLDS.get(owner).is_some_and(|h| h.load(Ordering::Relaxed) == 0) {
            if let Some(i) = UPDATE_HOLDS.iter().position(|h| h.load(Ordering::Relaxed) > 0) {
                UPDATE_OWNER.store(i, Ordering::Relaxed);
            }
        }
        return;
    }
    for h in UPDATE_HOLDS.iter() {
        h.store(0, Ordering::Relaxed);
    }
    match wait_pop() {
        Some(w) => {
            UPDATE_OWNER.store(w, Ordering::Relaxed);
            UPDATE_LOCK.store(1, Ordering::Relaxed);
            hold_add(w);
            if let Some(g) = UPDATE_GRANTED.get(w) {
                g.store(true, Ordering::Relaxed);
            }
            crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_mutex());
        }
        None => UPDATE_OWNER.store(NO_OWNER, Ordering::Relaxed),
    }
}

/// `ctlmgr`'s `wm_update(BEG_UPDATE); /* take the screen */`
/// (`aes/gemctrl.c:463`) for the inline Control Manager, taken once its event
/// is known (a press on a frame, the pointer over the bar): the lock is taken
/// in `ctl_pd`'s name and [`CTL_RUNNING`] makes everything the Control Manager
/// does until [`ctl_finish`] its own. `false` — nothing taken, nothing run —
/// while another process holds the lock: `ctlmgr` would be blocked in that
/// `wm_update` until the holder lets go, so the inline Control Manager does
/// not act this pass and the event stays pending ([`ctl_may_run`]).
pub(super) fn ctl_begin() -> bool {
    if !ctl_may_run() {
        return false;
    }
    let took = tak_flag(CTL_PD);
    debug_assert!(took, "ctl_may_run: the lock is free or the Control Manager's");
    CTL_RUNNING.store(true, Ordering::Relaxed);
    true
}

/// The end of the inline Control Manager's body. `hold` = `ct_msgup` is now
/// waiting for the selecting press to come up (`aes/gemctrl.c:100-101`): the
/// lock stays the Control Manager's until [`ctl_release`]. Otherwise this is
/// the loop's `wm_update(END_UPDATE); /* give up the screen */` (`:484`).
pub(super) fn ctl_finish(hold: bool) {
    CTL_RUNNING.store(false, Ordering::Relaxed);
    if !hold {
        ctl_release();
    }
}

/// `ctlmgr`'s `wm_update(END_UPDATE)` (`aes/gemctrl.c:484`) after
/// `ct_msgup`'s wait ended ([`input::ct_msgup_busy`](super::input::ct_msgup_busy)).
/// Only a hold that is still the Control Manager's is released: an AES
/// restart or `wind_new` may have taken it away in between.
pub(super) fn ctl_release() {
    if UPDATE_OWNER.load(Ordering::Relaxed) == CTL_PD {
        unsync(CTL_PD);
    }
}

/// Could the Control Manager take the screen now? `true` while the lock is
/// free or already the Control Manager's. While another process holds it,
/// EmuTOS's `ctlmgr` is either still asleep in its `ev_multi` or blocked in
/// the `wm_update(BEG_UPDATE)` after it (`aes/gemctrl.c:439-463`) — in
/// neither place does it intercept a click, track the bar or reach its loop
/// head's `w_setactive()` (`:442`). The inline Control Manager is gated on
/// this ([`super::event`]).
pub(super) fn ctl_may_run() -> bool {
    UPDATE_LOCK.load(Ordering::Relaxed) <= 0 || UPDATE_OWNER.load(Ordering::Relaxed) == CTL_PD
}

/// Public `fm_own` entry for the Form Manager (EmuTOS gemfmlib.c:82-107): a
/// modal dialog brackets its wait loop with `fm_own(TRUE)` / `fm_own(FALSE)`
/// (`fm_do`, gemfmlib.c:287,349; `fm_alert` reaches the loop via `fm_do`).
/// Delegates to [`mctrl_own`], the shared mouse-control-ownership + menu-hold
/// transition that `wind_update(BEG/END_MCTRL)` also drives. The opcode that
/// leads here waits for the lock first ([`update_block`]).
pub fn fm_own(begin: bool) {
    mctrl_own(begin);
}

/// `fm_own(begin)` — mouse-control-ownership transfer (EmuTOS gemfmlib.c:82-107).
/// `begin`: `BEG_UPDATE`, then on the outermost 0 → 1 edge park the menu tree,
/// remember the mouse owner and give the mouse to the caller, then `ml_ocnt++`.
/// `!begin`: `ml_ocnt--`, then on the 1 → 0 edge hand the mouse back to the
/// remembered owner and restore the menu tree, then `END_UPDATE`.
///
/// The `ml_ocnt == 0` edge body (`:89-93`) is `ml_mnhold = gl_mntree;
/// gl_mntree = NULL; get_ctrl(&ml_ctrl); get_mown(&ml_pmown); ct_chgown(rlr,
/// &gl_rscreen);` — the menu-hold (a menu-bar click is inert while the dialog
/// is up), the whole screen as the control rectangle (read live by
/// [`chk_ctrl`] from `ml_ocnt > 0`, so `ml_ctrl` needs no store), and
/// `set_mown(rlr)`: the caller owns the mouse — and the keyboard — for the
/// dialog, unless a button is held (`set_mown`'s `if (!button)`,
/// `aes/geminput.c:222`) or a menu track holds it (`ct_chgown`'s `if
/// (!gl_ctmown)`, `aes/gemctrl.c:395`). The 1 → 0 edge (`:101-103`) is
/// `ct_chgown(ml_pmown, &ml_ctrl); gl_mntree = ml_mnhold;` — the previous
/// owner gets the mouse back under the same two guards (A102). `rlr` is the
/// caller's AES identity, the remembered owner is [`input::get_mown`]'s.
fn mctrl_own(begin: bool) {
    if begin {
        update_beg(); // fm_own: wm_update(BEG_UPDATE) (gemfmlib.c:86)
        // 0 → 1 edge, evaluated BEFORE the increment (EmuTOS tests `ml_ocnt`
        // while still 0, gemfmlib.c:87): park the active menu tree so the
        // Control Manager stops tracking the bar for the duration of the
        // dialog, remember the owner, give the mouse to the caller (`:89-93`).
        if MCTRL_CNT.load(Ordering::Relaxed) == 0 {
            super::menu::hold_menu();
            ML_PMOWN.store(super::input::get_mown().to_raw(), Ordering::Relaxed);
            // `ct_chgown(rlr, &gl_rscreen)` (gemfmlib.c:93): the caller.
            super::input::ct_chgown(super::input::Mown::Pd(super::appl::current_identity()));
            // The holder of the count, on the same edge as the rest of the
            // 0 → 1 state — see [`MCTRL_OWNER`] for why the counter needs one
            // although `ml_ocnt` has none.
            MCTRL_OWNER.store(super::appl::current_identity(), Ordering::Relaxed);
        }
        MCTRL_CNT.fetch_add(1, Ordering::Relaxed);
    } else {
        mctrl_end(rlr());
    }
}

/// The `fm_own(FALSE)` half of [`mctrl_own`] with the releasing identity
/// explicit — the per-slot sweep ([`release_locks_of`]) releases a dead
/// client's holds in that client's name, not the sweeping caller's.
fn mctrl_end(me: usize) {
    // ml_ocnt-- then, on the 1 → 0 edge, give the mouse back to the
    // remembered owner and restore the parked menu tree (EmuTOS
    // `ml_ocnt--; if (ml_ocnt == 0) { ct_chgown(ml_pmown, &ml_ctrl);
    // gl_mntree = ml_mnhold; }`, gemfmlib.c:99-104), then
    // wm_update(END_UPDATE). `fetch_sub` returns the PRE-decrement value,
    // so the outermost edge is `prev == 1`. EmuTOS decrements
    // unconditionally; the `intercept` gate (`> 0`) treats any
    // non-positive count as "app does not own the mouse", so an unbalanced
    // END_MCTRL cannot wedge interception off.
    if MCTRL_CNT.fetch_sub(1, Ordering::Relaxed) == 1 {
        super::input::ct_chgown(super::input::Mown::from_raw(ML_PMOWN.load(Ordering::Relaxed)));
        super::menu::release_menu();
        MCTRL_OWNER.store(NO_OWNER, Ordering::Relaxed);
    }
    unsync(me);
}

/// `wind_calc` (108) — convert work↔border rect for `kind`.
pub fn wind_calc(pb: &Aespb) {
    let wtype = pb.intin(0);
    let kind = pb.intin(1) as u16;
    let r = Grect { x: pb.intin(2), y: pb.intin(3), w: pb.intin(4), h: pb.intin(5) };
    let o = calc(wtype, kind, r);
    pb.set_intout(1, o.x);
    pb.set_intout(2, o.y);
    pb.set_intout(3, o.w);
    pb.set_intout(4, o.h);
    pb.set_ret(1);
}

/// `wind_new` (109) — close + delete every window and clear every lock taken
/// with `wind_update` (EmuTOS `wm_new`, gemwmlib.c:1875-1893: it drains
/// `ml_ocnt` via `END_MCTRL` and `wind_spb.sy_tas` via `END_UPDATE` before
/// deleting the windows). Releasing the locks here is the observable equivalent
/// of those drain loops (both counters end at 0).
pub fn wind_new(pb: &Aespb) {
    wm_new();
    if let Some(backend) = host().and_then(|h| h.raster()) {
        redraw_all(backend);
    }
    pb.set_ret(1);
}

/// `wm_new` (`gemwmlib.c:1875-1893`) — the internal window sweep behind
/// [`wind_new`]: drain the outstanding `wind_update` mouse-control and
/// screen-sync locks, then close and delete every window still in use.
///
/// ```c
/// while(ml_ocnt > 0) wm_update(END_MCTRL);
/// while(wind_spb.sy_tas > 0) wm_update(END_UPDATE);
/// for (wh = 1; wh < NUM_WIN; wh++) {
///     if (D.w_win[wh].w_flags & VF_ISOPEN) wm_close(wh);
///     if (D.w_win[wh].w_flags & VF_INUSE) wm_delete(wh);
/// }
/// ```
///
/// The shell loop calls it after a launched program terminates
/// (`gemshlib.c:677`) and after an abnormal desktop exit (`:609`), so the
/// windows the previous occupant left open are swept before the next program
/// starts.
///
/// **Every slot goes through [`wm_delete`], and only the ones that are in
/// use.** That is what distinguishes the sweep from blanking the table: a slot
/// `wm_delete` has freed holds `gl_rscreen`/`gl_rfull` in its four rectangles
/// (`:1477-1480`), not zeros, and the next `wm_create` inherits the `WS_WORK`
/// of them (A33 — `aes-r3-wind-state.tsv` cases 340/344). A slot that was never
/// in use is skipped by the `VF_INUSE` test and keeps its BSS zeros, which is
/// why a virgin handle and a recycled one legitimately answer differently.
///
/// The locks are drained **first**, as in EmuTOS: the `wm_close` below brackets
/// its repaint in `wm_update(BEG_UPDATE)`/`(END_UPDATE)` (`:1451-1457`), so a
/// counter left standing from the previous occupant has to be gone before the
/// first window is closed, not after the last one.
///
/// It draws whatever closing those windows draws — `wm_close` runs
/// `draw_change` (`:1453`), whose tail is `w_drawdesk` + `w_update`
/// (`:1216-1222`), i.e. the background over each vacated rectangle and a
/// `WM_REDRAW` to every window still open under it. The whole-screen repaint is
/// a separate thing and belongs to the `wind_new` *opcode*, which an
/// application calls expecting its screen back.
pub fn wm_new() {
    // Release all update / mouse-control locks (`:1879-1883`) — count and
    // owner (`unsync`'s `sy_owner = NULL`, `gemflag.c:128`).
    MCTRL_CNT.store(0, Ordering::Relaxed);
    MCTRL_OWNER.store(NO_OWNER, Ordering::Relaxed);
    drain_update_lock();
    // `for (wh = 1; wh < NUM_WIN; wh++)` (`:1886`) — handle 0, the desktop
    // window, is not one of the windows `wm_new` deletes.
    for wh in 1..NUM_WIN as i16 {
        // `if (D.w_win[wh].w_flags & VF_ISOPEN) wm_close(wh);` (`:1887-1888`).
        if win(wh).map(|w| w.open).unwrap_or(false) {
            close_window(wh);
        }
        // `if (D.w_win[wh].w_flags & VF_INUSE) wm_delete(wh);` (`:1889-1890`) —
        // [`wm_delete`]'s own `get_pwin` is that test, so a free slot is
        // refused without being touched.
        wm_delete(wh);
    }
    // Our own stacking counter: every window that carried a `z` is gone, so the
    // next `wind_open` starts the order again (EmuTOS reorders `gl_wtree`
    // instead and has no counter to reset).
    // SAFETY: single-CPU AES.
    unsafe { *TABLE.next_z.get() = 1 };
}

/// `while (wind_spb.sy_tas > 0) wm_update(END_UPDATE);` (`gemwmlib.c:1882-1883`)
/// — drain the window-tree lock completely. Each `unsync` that reaches 0
/// hands the lock to the head of the wait line with `sy_tas = 1` and
/// `azombie`s it (`aes/gemflag.c:117-124`), and the loop's next pass counts
/// that hand-over down again; so the drain ends with the count at 0, no owner,
/// an empty wait line, and **every** waiter woken believing it holds the lock.
/// Reproduced as that end state: each waiter's call is handed the lock without
/// a count ([`UPDATE_GRANTED`]), so its later `END_UPDATE` finds the count at 0
/// and is ignored (`:109-113`), as in EmuTOS.
fn drain_update_lock() {
    UPDATE_LOCK.store(0, Ordering::Relaxed);
    UPDATE_OWNER.store(NO_OWNER, Ordering::Relaxed);
    for h in UPDATE_HOLDS.iter() {
        h.store(0, Ordering::Relaxed);
    }
    let mut woke = false;
    while let Some(w) = wait_pop() {
        if let Some(g) = UPDATE_GRANTED.get(w) {
            g.store(true, Ordering::Relaxed);
        }
        woke = true;
    }
    if woke {
        crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_mutex());
    }
}

/// The `sh_ldapp` window sweep narrowed to **one dead client**: the GEMDOS
/// slot `slot` has ended, take what it left in the Window Manager and leave
/// every other client's state alone (A107).
///
/// This is the nested-`Pexec` case A96 could not cover. EmuTOS never has it:
/// there *every* program is started by the shell loop, so the `wm_new()` at
/// `gemshlib.c:676-677` runs after each of them, and a GEM program that
/// starts another one does it through `shel_write` — the parent ends, the
/// shell loop runs the child, sweeps, and starts the parent again
/// (`gemshlib.c:600-612,708-731`). NewTOS runs an EL0 `Pexec` child **nested**
/// inside its living parent (TeraDesk → QED): the parent never left, so no
/// shell-loop sweep happens for the child. What this does for that child is
/// what `wm_new` does for a program the shell loop ran, restricted to the
/// dead slot's own property:
///
/// * every window whose `w_owner` is `slot`: closed if open (`wm_close`, with
///   its `draw_change` repaint and the `WM_REDRAW` fan-out to whatever the
///   vanished window uncovered — the parent's windows) and deleted
///   (`gemwmlib.c:1886-1890`);
/// * the `WF_NEWDESK` background tree, if **this** slot registered it: the
///   pointer is a VA of an address space that is gone (A97), and EmuTOS drops
///   `gl_newdesk` before the next program runs (`wm_init`, `gemwmlib.c:1300`);
/// * the menu bar, if **this** slot installed it ([`super::menu::release_bar_of`]).
///
/// * the two `wind_update` counters, **but only the dead slot's own holds** —
///   [`release_locks_of`]. `wm_new` drains both unconditionally
///   (`gemwmlib.c:1879-1883`), because at that point no other client is
///   alive to hold one. Here the parent *is*
///   alive and may legitimately hold its own, which is precisely the case
///   EmuTOS's owner test covers: `if (wind_spb.sy_owner == rlr)
///   unsync(&wind_spb)` (`gemshlib.c:668-669`) (A110).
pub fn wm_new_for_slot(slot: usize) {
    let Ok(owner) = i16::try_from(slot) else { return };
    for wh in 1..NUM_WIN as i16 {
        let Some((w_owner, open)) = win(wh).map(|w| (w.owner, w.open)) else { continue };
        if w_owner != owner {
            continue;
        }
        if open {
            close_window(wh);
        }
        wm_delete(wh);
    }
    if newdesk_owner() == Some(slot) {
        clear_newdesk();
    }
    super::menu::release_bar_of(slot);
    release_locks_of(slot);
}

/// The `wind_update` half of [`wm_new_for_slot`]: hand back the screen lock
/// and the mouse-control count that the dead client `slot` still holds, and
/// **nothing else's**.
///
/// EmuTOS does this in two steps, one of them owner-aware:
///
/// ```c
/// if (wind_spb.sy_owner == rlr)      /* sh_ldapp, gemshlib.c:668-669 */
///     unsync(&wind_spb);
/// if (gl_shgem)
///     wm_new();                      /* :676-677 */
/// /* wm_new, gemwmlib.c:1879-1883 */
/// while (ml_ocnt > 0) wm_update(END_MCTRL);
/// while (wind_spb.sy_tas > 0) wm_update(END_UPDATE);
/// ```
///
/// The owner test is `sh_ldapp`'s; `wm_new`'s drain loops need none, because
/// the program they run for was the only client. Our sweep has to carry the
/// test *into* the drain: the parent is alive. So each loop is `wm_new`'s,
/// gated on the counter's owner being the dead slot.
///
/// **Order and route matter.** `ml_ocnt` first, `sy_tas` second, as in
/// `wm_new`; and the mouse-control count is drained through [`mctrl_own`], not
/// stored to zero, so every turn runs `fm_own`'s 1 → 0 edge body — the mouse
/// goes back to [`ML_PMOWN`] and the parked menu tree is restored
/// (`gemfmlib.c:99-104`) — and its own `END_UPDATE` (`:105`), which is what
/// makes the `sy_tas` the child's `BEG_MCTRL` took come off again.
///
/// **The screen lock goes by the dead client's own share** ([`UPDATE_HOLDS`]),
/// released in its name ([`unsync`]) — so the lock passes to the next waiter
/// on its wait line exactly as if the client had called `END_UPDATE` itself.
/// A hand-over it never collected is part of that share; and a dead client
/// still on the wait line leaves it (GH1).
fn release_locks_of(slot: usize) {
    while MCTRL_OWNER.load(Ordering::Relaxed) == slot && MCTRL_CNT.load(Ordering::Relaxed) > 0 {
        mctrl_end(slot);
    }
    if let Some(g) = UPDATE_GRANTED.get(slot) {
        g.store(false, Ordering::Relaxed);
    }
    wait_remove(slot);
    while UPDATE_HOLDS.get(slot).is_some_and(|h| h.load(Ordering::Relaxed) > 0)
        && UPDATE_LOCK.load(Ordering::Relaxed) > 0
    {
        unsync(slot);
    }
}

/// `wm_init` (`gemwmlib.c:1247-1310`) — re-initialise the Window Manager
/// before each program the shell loop starts (`gemshlib.c:717`). EmuTOS
/// rebuilds the whole window-extent/element object arena; the observable
/// part for us is that **every window is gone**, the desktop window (handle
/// 0) spans the screen again, and `gl_newdesk` is reset to NULL
/// (`gemwmlib.c:1300`) so a tree the previous program registered with
/// `wind_set(WF_NEWDESK)` cannot leak into the next one.
///
/// Resetting `gl_newdesk` is what makes it safe for the shell loop to run
/// after an EL0 application: the tree pointer that application registered
/// lived in its own (now unmapped) address space.
///
/// Like `wm_new` it draws nothing; `sh_draw(clear)` does the painting
/// immediately afterwards (`gemshlib.c:719`).
pub fn wm_init() {
    wm_new();
    // `w_setup(ppd, DESKWH, NONE)` + the four `w_setsize` calls
    // (`gemwmlib.c:1290-1294`).
    init_desk_window();
    // `gl_newdesk = NULL;` (gemwmlib.c:1300)
    clear_newdesk();
}

/// The Window Manager's share of an AES restart (A106, [`super::reinit`]) —
/// the `theglo` re-zero of `D.w_win[]` (`gemstart.S:101-110`,
/// `aes/gemlib.h:136`) plus `wm_start()` (`geminit.c:723` →
/// `gemwmlib.c:1320-1360`).
///
/// **Deliberately not [`wm_init`], and that is the faithful choice.** `wm_init`
/// is the *per-program* reset the shell loop runs inside a session
/// (`gemshlib.c:717`): its `wm_new` walks the table closing each window, which
/// **draws** (`draw_change` → `w_drawdesk` + a `WM_REDRAW` fan-out,
/// `gemwmlib.c:1216-1222,1453`) and **posts into other processes' queues**. At
/// a session boundary EmuTOS does neither: the assembly wrapper blanks the
/// whole global area before `gem_main` runs again, so every `WINDOW` is simply
/// gone — no close, no repaint, no message — and `wm_start` then builds the
/// table up from nothing. Reproducing the sweep here would repaint a screen
/// that is about to be redrawn anyway and, worse, refill the message queues
/// [`super::pd::reinit`] has just emptied.
///
/// So: blank the window table, rebuild the desktop window
/// (`w_setup`/`w_setsize`, `gemwmlib.c:1290-1294`), and put back the state
/// that lives outside them —
///
/// * `gl_newdesk`/`gl_newroot` (`gemwmlib.c:55,1300`): the tree a program
///   registered with `wind_set(WF_NEWDESK)` is a VA of an address space that
///   is gone (A97).
/// * `ml_ocnt` (`geminit.c:805`) and `wind_spb.sy_tas` (`:837-839`) — the
///   `wind_update` counters, which `gem_main` resets explicitly.
/// * `gl_wtcolor`/`gl_wbcolor` — the per-gadget default colour words new
///   windows inherit (`wm_start`, `gemwmlib.c:1346-1357`), which
///   `wind_set(WF_DCOLOR)` may have changed for the session that ended.
/// * `ml_pmown` (`gemfmlib.c:58`) — the mouse owner parked by the outermost
///   `BEG_MCTRL`; the application chain is its power-up value.
pub(super) fn reinit() {
    // `bzero(&D, size_theglo())` over `D.w_win[]` (`gemstart.S:101-110`).
    // SAFETY: single-CPU restart path, no AES call in flight.
    unsafe {
        *TABLE.slots.get() = [Window::EMPTY; NUM_WIN];
        *TABLE.next_z.get() = 1;
    }
    // `wm_start()`: the desktop window and the global defaults.
    init_desk_window();
    clear_newdesk();
    MCTRL_CNT.store(0, Ordering::Relaxed);
    MCTRL_OWNER.store(NO_OWNER, Ordering::Relaxed);
    UPDATE_LOCK.store(0, Ordering::Relaxed);
    UPDATE_OWNER.store(NO_OWNER, Ordering::Relaxed);
    // `wind_spb` is re-zeroed with the rest of `theglo` (`geminit.c:837-839`):
    // no holder, no wait line — no process of the ended session is left to
    // be handed anything.
    for h in UPDATE_HOLDS.iter() {
        h.store(0, Ordering::Relaxed);
    }
    for g in UPDATE_GRANTED.iter() {
        g.store(false, Ordering::Relaxed);
    }
    while wait_pop().is_some() {}
    CTL_RUNNING.store(false, Ordering::Relaxed);
    *gl_wtcolor() = GL_WTCOLOR_DEFAULT;
    *gl_wbcolor() = GL_WBCOLOR_DEFAULT;
    ML_PMOWN.store(super::pd::KERNEL_CONTEXT, Ordering::Relaxed);
}

fn inside(r: Grect, x: i16, y: i16) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

/// Repaint the desktop background + every open window (whole screen). The
/// desktop composite (`desk::redraw_desktop`) drives this after a full-screen
/// program returns; the Form Manager's scoped `form_dial(FMD_FINISH)` uses
/// [`redraw_dialog_area`] instead so only the dialog's rectangle is repainted.
pub fn redraw() {
    if let Some(backend) = host().and_then(|h| h.raster()) {
        redraw_all(backend);
    }
}

/// Repaint the desktop background + remaining window frames within the dialog
/// target rectangle `pt`, then fan a `WM_REDRAW` out to every window overlapping
/// it — the Form Manager's `form_dial(FMD_FINISH)` restore, confined to `pt`
/// (not the whole desktop).
///
/// EmuTOS `fm_dial` FMD_FINISH (gemfmlib.c:375-379): `w_drawdesk(pt)` +
/// `w_update(DESKWH, pt, DESKWH, FALSE)` — the same background-repaint +
/// window-redraw fan-out pair used on window close ([`wind_close`]),
/// scoped to `pt` (`int_in[5..8]`, the big rect) rather than the full screen.
pub fn redraw_dialog_area(x: i16, y: i16, w: i16, h: i16) {
    if w <= 0 || h <= 0 {
        return;
    }
    // w_drawdesk(pt) + remaining window frames over pt.
    redraw_clip(x, y, w, h);
    // w_update(DESKWH, pt, DESKWH, FALSE): WM_REDRAW for each overlapping window.
    w_update(DESKWH, Grect { x, y, w, h }, DESKWH, false);
}

fn screen_rect() -> Grect {
    if let Some(backend) = host().and_then(|h| h.raster()) {
        Grect { x: 0, y: 0, w: backend.width() as i16, h: backend.height() as i16 }
    } else {
        Grect { x: 0, y: 0, w: 640, h: 480 }
    }
}

/// EmuTOS `gl_rfull` — the full screen minus the menu row at the top
/// (`gsx_start`, `gemgraf.c:423-424`, set **unconditionally**, whether or not a
/// menu bar is installed). [`wm_init`] stores it as the desktop window's `FULL`
/// and `WORK` rectangles, which is where every later reader takes it from.
fn desktop_work() -> Grect {
    let full = screen_rect();
    let top = super::menu::desk_top();
    Grect { x: full.x, y: full.y + top, w: full.w, h: full.h - top }
}

// ── Batch 2: interactive frame + WM_* messages ──────────────────────────

// WM_* message ids (aesdefs.h:229-239) and the WM_ARROWED actions
// (aesdefs.h:245-253, gemctrl.c:63-74; line = arrow, page = track click
// either side of the elevator) are `tos_types::aes`'.

/// Which frame component a point falls in. Beyond the title/sizer/arrow
/// gadgets, the scroll bars distinguish the elevator **thumb** (drag →
/// `WM_HSLID`/`WM_VSLID`) from a **page** click in the track either side of it
/// (→ `WM_ARROWED` with `WA_*PAGE`), mirroring EmuTOS `hctl_window`'s split of
/// `W_?ELEV` vs `W_?SLIDE` (gemctrl.c:262-306).
#[derive(PartialEq, Clone, Copy)]
enum Gadget {
    None,
    Closer,
    Fuller,
    Mover,
    Sizer,
    Up,
    Dn,
    Lf,
    Rt,
    VElev,  // vertical elevator thumb (drag)
    HElev,  // horizontal elevator thumb (drag)
    VUpPage, // v-track above the thumb (WA_UPPAGE)
    VDnPage, // v-track below the thumb (WA_DNPAGE)
    HLfPage, // h-track left of the thumb (WA_LFPAGE)
    HRtPage, // h-track right of the thumb (WA_RTPAGE)
}

/// Hit-test a point against `w`'s frame gadgets (mirrors `draw_frame`'s
/// layout).
///
/// The arrow cells follow EmuTOS's `ob_find` on the `W_ACTIVE` tree
/// (`hctl_window`, `gemctrl.c:188`; `ob_find`, `gemoblib.c:1034-1113`): an
/// `FL3DACT` arrow answers for its **grown** slot (`ob_actxywh`,
/// `:313-338`), and where that slot shares a border line with the slide box
/// — the down arrow's top row (`w_bldvbar`, `gemwmlib.c:424`), the right
/// arrow's left column (`:459`) — the line belongs to the slide: `ob_find`
/// tests a parent's children from `ob_tail` backwards, and `W_VSLIDE`/
/// `W_HSLIDE` are added after the arrows (`:427`, `:465`). So the down
/// arrow starts one row below its slot's top border, the right arrow one
/// column right of its slot's left border, and the left arrow ends on its
/// slot's last column, the column before the track's border (A95 (4), (5)).
fn gadget_at(w: &Window, mx: i16, my: i16) -> Gadget {
    let b = w.curr;
    let kind = w.kind;
    if !inside(b, mx, my) {
        return Gadget::None;
    }
    let ah = adj_hbox();
    let aw = adj_wbox();

    // Title row — one `adj_hbox` band, `adj_wbox` per gadget cell (the layout
    // [`draw_frame`] paints).
    if kind & TGADGETS != 0 && my < b.y + ah {
        if kind & CLOSER != 0 && mx < b.x + aw {
            return Gadget::Closer;
        }
        if kind & FULLER != 0 && mx >= b.x + b.w - aw {
            return Gadget::Fuller;
        }
        if kind & MOVER != 0 {
            return Gadget::Mover;
        }
        return Gadget::None;
    }
    // Sizer corner.
    if kind & SIZER != 0 && mx >= b.x + b.w - aw && my >= b.y + b.h - ah {
        return Gadget::Sizer;
    }
    // Right scroll-bar arrows.
    if let Some(bar) = vbar_rect(b, kind).filter(|_| mx >= b.x + b.w - aw) {
        let (up, dn) = varrow_cells(b, kind);
        if up.map(grow_3d).is_some_and(|s| my < s.y + s.h) {
            return Gadget::Up;
        }
        if dn.map(grow_3d).is_some_and(|s| my > s.y) {
            return Gadget::Dn;
        }
        // Elevator thumb (drag) vs. a page click in the track either side of
        // it (EmuTOS `hctl_window` W_VELEV vs W_VSLIDE split, gemctrl.c:262-306:
        // `my < elev_y` → up-page, else down-page).
        if let Some((track_y, track_h)) = vtrack(w) {
            if my >= track_y && my < track_y + track_h {
                let (off, len) = thumb_geom(track_h, w.vslsize, w.vslide, gl_hbox());
                let e = elev_rect(bar, (track_y, track_h), off, len, true);
                if my >= e.y && my < e.y + e.h {
                    return Gadget::VElev;
                }
                return if my < e.y { Gadget::VUpPage } else { Gadget::VDnPage };
            }
        }
    }
    // Bottom scroll-bar arrows.
    if let Some(bar) = hbar_rect(b, kind).filter(|_| my >= b.y + b.h - ah) {
        let (lf, rt) = harrow_cells(b, kind);
        if lf.map(grow_3d).is_some_and(|s| mx < s.x + s.w) {
            return Gadget::Lf;
        }
        if rt.map(grow_3d).is_some_and(|s| mx > s.x) {
            return Gadget::Rt;
        }
        // Elevator thumb (drag) vs. a page click in the track either side of it
        // (EmuTOS W_HELEV vs W_HSLIDE split, `mx < elev_x` → left-page).
        if let Some((track_x, track_w)) = htrack(w) {
            if mx >= track_x && mx < track_x + track_w {
                let (off, len) = thumb_geom(track_w, w.hslsize, w.hslide, GL_WBOX);
                let e = elev_rect(bar, (track_x, track_w), off, len, false);
                if mx >= e.x && mx < e.x + e.w {
                    return Gadget::HElev;
                }
                return if mx < e.x { Gadget::HLfPage } else { Gadget::HRtPage };
            }
        }
    }
    Gadget::None
}

/// The screen rectangle of a title-row gadget (closer/fuller), for the
/// `watchbox` feedback — the `gl_wbox` × `gl_hbox` cell [`draw_frame`] paints,
/// inset `ADJ3DSTD` inside the `adj_wbox`-wide title cell.
fn gad_rect(w: &Window, g: Gadget) -> Grect {
    let b = w.curr;
    let y = b.y + ADJ3DSTD;
    match g {
        Gadget::Closer => Grect { x: b.x + ADJ3DSTD, y, w: GL_WBOX, h: gl_hbox() },
        Gadget::Fuller => {
            Grect { x: b.x + b.w - adj_wbox() + ADJ3DSTD, y, w: GL_WBOX, h: gl_hbox() }
        }
        _ => Grect::default(),
    }
}

/// The scroll-arrow cell of `g` with the glyph and the `gl_wa*color` index
/// that go with it — `(rect, G_*, W_*ARROW_IDX)`, `None` for anything that is
/// not one of the four arrows (a page click in the track, or an absent
/// gadget). The rectangles are [`varrow_cells`]/[`harrow_cells`], i.e. the
/// un-grown `gl_wbox` × `gl_hbox` cells `w_bldvbar`/`w_bldhbar` lay out
/// (`aes/gemwmlib.c:412-424,450-462`) — the same ones [`draw_varrows`] /
/// [`draw_harrows`] paint, so the pressed cell is exactly the drawn one.
fn arrow_cell(w: &Window, g: Gadget) -> Option<(Grect, u16, usize)> {
    match g {
        Gadget::Up => varrow_cells(w.curr, w.kind).0.map(|r| (r, G_UP, W_UPARROW_IDX)),
        Gadget::Dn => varrow_cells(w.curr, w.kind).1.map(|r| (r, G_DN, W_DNARROW_IDX)),
        Gadget::Lf => harrow_cells(w.curr, w.kind).0.map(|r| (r, G_LF, W_LFARROW_IDX)),
        Gadget::Rt => harrow_cells(w.curr, w.kind).1.map(|r| (r, G_RT, W_RTARROW_IDX)),
        _ => None,
    }
}

/// `ob_change(gl_awind, gadget, selected|normal, TRUE)` for a scroll arrow
/// (`aes/gemctrl.c:272-284`): redraw the cell with its `SELECTED` bit set or
/// clear. Only the four arrows have a cell ([`arrow_cell`]); everything else
/// is a no-op, which is the `W_HSLIDE`/`W_VSLIDE` guard of those lines.
///
/// `ob_change` brackets its draw with `gsx_moff()`/`gsx_mon()`
/// (`aes/gemoblib.c:1275,1305`) — the cursor's save-under would otherwise
/// restore the pre-draw pixels over the cell (A65) — and so does this; the
/// *wait* in between runs with the cursor up, as EmuTOS's `handle_arrow_msg`
/// does. The window is the top one (checked by [`intercept`] before the
/// gadget switch), so the topped colour word applies.
fn arrow_change(w: &Window, g: Gadget, sel: bool, backend: &dyn RasterBackend) {
    let Some((r, ch, idx)) = arrow_cell(w, g) else {
        return;
    };
    let clip = ClipRect::full(backend.width(), backend.height());
    crate::vdi::cursor_hide(backend);
    gadget_sel(r, ch, sel, gadget_ink(w, idx, true), backend, clip);
    crate::vdi::cursor_show(backend);
}

/// Track the button over a closer/fuller gadget: invert it while the mouse
/// is inside, restore on leave; return whether it was released inside
/// (EmuTOS `gr_watchbox`).
fn watchbox(r: Grect, ch: u16, ink: Ink, backend: &dyn RasterBackend) -> bool {
    let Some(h) = host() else { return false };
    let clip = ClipRect::full(backend.width(), backend.height());
    let mut was_in = true;
    gadget_sel(r, ch, true, ink, backend, clip);
    loop {
        let (mx, my) = crate::vdi::mouse_position();
        let now_in = inside(r, mx, my);
        if now_in != was_in {
            gadget_sel(r, ch, now_in, ink, backend, clip);
            was_in = now_in;
        }
        if crate::vdi::mouse_buttons() & 0x01 == 0 {
            // `gr_watchbox` ends on `gr_stilldn`'s `ev_multi` returning the
            // release (`aes/gemgrlib.c:62-70,352`), whose `ev_rets` consumes it.
            super::input::ev_consume();
            break;
        }
        h.pump_input();
    }
    gadget_sel(r, ch, false, ink, backend, clip); // restore normal
    was_in
}

/// EmuTOS `gr_stilldn` (`aes/gemgrlib.c:47-73`) as the sizer and mover drags use
/// it: block until the pointer leaves the 1×1 rectangle at `(x, y)` — i.e. until
/// it moves at all — or the button comes up. Returns `false` for the button-up
/// exit (`which & MU_BUTTON`, `:69-70`) and `true` for the `MU_M1` exit.
///
/// `gr_wait` passes `gr_stilldn(TRUE, xrat, yrat, 1, 1)` (`:182`), so the
/// `MOBLK` fires as soon as the pointer is *outside* that one-pixel rectangle;
/// the button condition is `0x0001ff00` = "all buttons up, one click"
/// (`:62-66`), which here is simply the release. A key press (`MU_KEYBD`) is
/// also armed but does not end the drag — it returns without `MU_BUTTON`.
fn gr_stilldn(x: i16, y: i16, h: &'static dyn Host) -> bool {
    loop {
        if crate::vdi::mouse_buttons() & 0x01 == 0 {
            // The `ev_multi` returning `MU_BUTTON` ran `ev_rets`
            // (`aes/gemevlib.c:268`): the release is consumed here.
            super::input::ev_consume();
            return false;
        }
        let (mx, my) = crate::vdi::mouse_position();
        if mx != x || my != y {
            return true;
        }
        h.pump_input();
    }
}

/// EmuTOS `gr_draw` (`aes/gemgrlib.c:155-166`): the rubber box `o`, plus — when
/// `poff` is not the zero rectangle (`have2box`) — a second box offset from it
/// per component. XOR, so the same call draws and erases.
fn gr_draw(o: Grect, poff: Grect, backend: &dyn RasterBackend, clip: ClipRect) {
    super::graf::gsx_xbox(o.x, o.y, o.w, o.h, backend, clip);
    if poff != Grect::default() {
        super::graf::gsx_xbox(
            o.x + poff.x,
            o.y + poff.y,
            o.w + poff.w,
            o.h + poff.h,
            backend,
            clip,
        );
    }
}

/// EmuTOS `gr_rubwind` (`aes/gemgrlib.c:199-220`) — stretch the free corner of
/// an XOR box pinned at `(xorigin, yorigin)` until the button comes up, drawing
/// a second box offset by `poff` alongside it; returns the final `(w, h)`.
///
/// The window is **not** resized here and nothing on the screen is repainted:
/// the only pixels touched are the two XOR outlines, which each `gr_wait`
/// draws, waits on, and complements away again (`:169-191`). The Control
/// Manager then posts `WM_SIZED` and the *owner* confirms the new geometry with
/// `wind_set(WF_CURRXYWH)`, which is what repaints the window and the region it
/// uncovered (`hctl_window` → `ct_msgup`, `gemctrl.c:258-259,314`).
///
/// This replaces the former solid live-resize, which repainted the desktop
/// background (`w_drawdesk` — background box *and every icon*) over the whole
/// window rectangle and redrew the frame on top of it, once per drag frame. The
/// window's area therefore held the background-behind-the-window, icons
/// included, for the duration of each frame's redraw: the square "holes" that
/// travel with the drag on a real Pi 4. The mover drag never showed it because
/// it blits the window to its new place instead of exposing what is under it.
fn gr_rubwind(
    h: &'static dyn Host,
    xorigin: i16,
    yorigin: i16,
    wmin: i16,
    hmin: i16,
    poff: Grect,
    backend: &dyn RasterBackend,
) -> (i16, i16) {
    update_beg(); // wm_update(BEG_UPDATE)
    // `gr_setup(BLACK)`: clip to the whole screen, XOR write mode
    // (`gemgrlib.c:76-80`); the XOR mode lives in `gsx_xbox`.
    let clip = ClipRect::full(backend.width(), backend.height());
    let mut o = Grect { x: xorigin, y: yorigin, w: 0, h: 0 };
    loop {
        // `gr_clamp` (`gemgrlib.c:83-88`): the free corner follows the pointer,
        // never closer than (wmin, hmin) — a *signed* max, no upper clamp.
        let (mx, my) = crate::vdi::mouse_position();
        o.w = (mx - o.x + 1).max(wmin);
        o.h = (my - o.y + 1).max(hmin);
        // `gr_wait` (`gemgrlib.c:169-191`): draw, wait for a change, erase.
        // The `gsx_moff`/`gsx_mon` brackets are our counted cursor hide/show.
        crate::vdi::cursor_hide(backend);
        gr_draw(o, poff, backend, clip);
        crate::vdi::cursor_show(backend);
        let down = gr_stilldn(mx, my, h);
        crate::vdi::cursor_hide(backend);
        gr_draw(o, poff, backend, clip);
        crate::vdi::cursor_show(backend);
        if !down {
            break;
        }
    }
    update_end(); // wm_update(END_UPDATE)
    (o.w, o.h)
}

/// The `W_SIZER` arm of EmuTOS `hctl_window` (`aes/gemctrl.c:241-260`): work out
/// the second box's offset and the minimum size from the window, run
/// [`gr_rubwind`], and report the rectangle that goes into the `WM_SIZED`
/// message (`ct_msgup(message, owner, wh, x, y, w, h)`, `gemctrl.c:314`) — the
/// **unchanged** border origin with the dragged size.
fn drag_size(w: &Window, backend: &dyn RasterBackend) -> Grect {
    let border = w.curr;
    // No host means no input pump, so there is no drag to run: report the
    // geometry unchanged rather than inventing one.
    let Some(h) = host() else { return border };
    // `t = WS_WORK - WS_CURR`, per component (`gemctrl.c:247-251`) — the second
    // XOR box is the work area the new size would have.
    let poff = Grect {
        x: w.work.x - border.x,
        y: w.work.y - border.y,
        w: w.work.w - border.w,
        h: w.work.h - border.h,
    };
    // Minimum size (`gemctrl.c:252-257`): one character cell, widened to four
    // gadget boxes when the window has top/horizontal gadgets and heightened to
    // six when it has vertical ones.
    let wmin = if w.kind & (TGADGETS | HGADGETS) != 0 { GL_WBOX * 4 } else { gl_wchar() };
    let hmin = if w.kind & VGADGETS != 0 { gl_hbox() * 6 } else { gl_hchar() };
    let (nw, nh) = gr_rubwind(h, border.x, border.y, wmin, hmin, poff, backend);
    Grect { x: border.x, y: border.y, w: nw, h: nh }
}

/// EmuTOS `rc_constrain` (`util/rectfunc.c:32-42`) — pull `pt` back inside `pc`
/// without changing its size: each edge that sticks out moves the origin, the
/// left/top checks first so a `pt` larger than `pc` ends up pinned at `pc`'s
/// far edge.
fn rc_constrain(pc: Grect, pt: &mut Grect) {
    if pt.x < pc.x {
        pt.x = pc.x;
    }
    if pt.y < pc.y {
        pt.y = pc.y;
    }
    if pt.x + pt.w > pc.x + pc.w {
        pt.x = (pc.x + pc.w) - pt.w;
    }
    if pt.y + pt.h > pc.y + pc.h {
        pt.y = (pc.y + pc.h) - pt.h;
    }
}

/// EmuTOS `gr_dragbox` (`aes/gemgrlib.c:243-267`) — drag a fixed-size `w`×`h`
/// XOR box that keeps its distance to the pointer, constrained to `pc`, until
/// the button comes up; returns the final origin.
///
/// The grab offset is taken **once**, before the loop, with
/// `gr_clamp(sx+1, sy+1, 0, 0, &offx, &offy)` (`:253`) — which for
/// `gr_clamp(cursor, origin, min) = max(cursor - origin + 1, min)` is
/// `max(mouse - box_origin, 0)`, i.e. where inside the box the pointer took
/// hold; the box then tracks as `origin = mouse - offset` (`:259-260`) so it
/// does not snap a corner to the pointer.
///
/// Like [`gr_rubwind`] this draws **nothing but the outline**: `gr_wait`
/// (`:169-191`) complements the box, blocks in `gr_stilldn` until the pointer
/// moves or the button comes up, and complements it away again. The window is
/// not moved and no pixel underneath it is repainted — the Control Manager
/// posts `WM_MOVED` and the *owner* applies the new position with
/// `wind_set(WF_CURRXYWH)` (`hctl_window` → `ct_msgup`, `gemctrl.c:237-238,314`).
///
/// This replaces the former solid live-move, which blitted the whole window to
/// its new place once per drag frame (`copy_rect`) and repainted only the
/// vacated L-region with the desktop background. Inside a window underneath,
/// that L is not the desktop's to paint (`do_walk`, `gemwmlib.c:286-312`) and
/// no `WM_REDRAW` asked its owner for it either, so every drag step left the
/// moving window's leading edge behind: the comb of one-pixel vertical lines,
/// one per pointer step, reported from a real Pi 4.
fn gr_dragbox(
    h: &'static dyn Host,
    w: i16,
    hgt: i16,
    sx: i16,
    sy: i16,
    pc: Grect,
    backend: &dyn RasterBackend,
) -> (i16, i16) {
    update_beg(); // wm_update(BEG_UPDATE)
    // `gr_setup(BLACK)`: clip to the whole screen, XOR write mode
    // (`gemgrlib.c:76-80`); the XOR mode lives in `gsx_xbox`.
    let clip = ClipRect::full(backend.width(), backend.height());
    let (mx0, my0) = crate::vdi::mouse_position();
    let offx = (mx0 - sx).max(0);
    let offy = (my0 - sy).max(0);
    let mut o = Grect { x: sx, y: sy, w, h: hgt };
    loop {
        let (mx, my) = crate::vdi::mouse_position();
        o.x = mx - offx;
        o.y = my - offy;
        rc_constrain(pc, &mut o);
        // `gr_wait` (`gemgrlib.c:169-191`) with `poff = gl_rzero`, so
        // `have2box` is FALSE and `gr_draw` emits the single box (`:171,155-166`).
        // The `gsx_moff`/`gsx_mon` brackets are our counted cursor hide/show.
        crate::vdi::cursor_hide(backend);
        gr_draw(o, Grect::default(), backend, clip);
        crate::vdi::cursor_show(backend);
        let down = gr_stilldn(mx, my, h);
        crate::vdi::cursor_hide(backend);
        gr_draw(o, Grect::default(), backend, clip);
        crate::vdi::cursor_show(backend);
        if !down {
            break;
        }
    }
    update_end(); // wm_update(END_UPDATE)
    (o.x, o.y)
}

/// The `W_NAME` arm of EmuTOS `hctl_window` (`aes/gemctrl.c:225-239`): build the
/// constraining rectangle, run [`gr_dragbox`] over the window's **border** rect,
/// and report the dragged origin with the *unchanged* size — the rectangle that
/// goes into the `WM_MOVED` message (`ct_msgup(message, owner, wh, x, y, w, h)`,
/// `gemctrl.c:314`).
///
/// The constraining rectangle is the 3D branch's
/// `r_set(&f, 0, gl_hbox, gl_rscreen.g_w + w - gl_wbox - 6 - 2*ADJ3DSTD,
/// MAX_COORDINATE)` (`:230-232`), i.e. "prevent the mover gadget from being
/// moved completely offscreen": the window may hang off the right edge until
/// only `gl_wbox + 6 + 2*ADJ3DSTD` pixels of the title band are left, may not
/// rise above the menu row, and is effectively unbounded downward
/// (`MAX_COORDINATE` = 10000, `include/sysconf.h:63`). So only two of the four
/// sides are held: `f.g_x` keeps the left edge at 0 and `f.g_y` the top at the
/// menu row, while the window may be pushed off the right edge down to
/// `gl_wbox + 6 + 2*ADJ3DSTD` = 29 pixels of title band, and off the bottom
/// entirely. The old solid drag pinned all four sides to the screen rectangle;
/// EmuTOS does not.
fn drag_move(w: &Window, backend: &dyn RasterBackend) -> Grect {
    let border = w.curr;
    // No host means no input pump, so there is no drag to run: report the
    // geometry unchanged rather than inventing one.
    let Some(h) = host() else { return border };
    let screen = screen_rect();
    let f = Grect {
        x: 0,
        y: gl_hbox(),
        w: screen.w + border.w - GL_WBOX - 6 - 2 * ADJ3DSTD,
        h: MAX_COORDINATE,
    };
    let (nx, ny) = gr_dragbox(h, border.w, border.h, border.x, border.y, f, backend);
    Grect { x: nx, y: ny, w: border.w, h: border.h }
}

/// Live-drag a scroll-bar elevator (thumb) following the mouse until release;
/// returns the final position 0..1000, which the caller posts as `WM_VSLID` /
/// `WM_HSLID` `msg[4]`. Models EmuTOS `gr_slidebox` (`gemgrlib.c:358-410`): the
/// thumb is dragged within its track, held at a constant grip offset from the
/// cursor, and the reported position is `mul_div_round(travel_used, 1000,
/// travel)` (the D-098 slider-creep rounding). Feedback is **thumb-follow** (the
/// slider is repainted at the new spot each frame, bracketed with the counting
/// cursor semaphore); EmuTOS instead drags an XOR outline of the elevator inside
/// its slide box (`gr_slidebox` → `gr_dragbox`, `gemgrlib.c:388`). That is a
/// remaining deviation, tracked separately — unlike the mover and the sizer it
/// repaints only the slider gadget, never the window or what is under it.
/// The window's stored slider position is advanced so the thumb tracks the
/// cursor; the owner then normally confirms it via `wind_set(WF_?SLIDE, pos)`.
fn drag_elevator(w: &mut Window, vert: bool, backend: &dyn RasterBackend) -> i16 {
    let (slsize, slpos) = if vert { (w.vslsize, w.vslide) } else { (w.hslsize, w.hslide) };
    let Some(h) = host() else { return slpos };
    let Some((track_start, track_len)) = (if vert { vtrack(w) } else { htrack(w) }) else {
        return slpos;
    };
    let (thumb_off, thumb_len) = thumb_geom(track_len, slsize, slpos, if vert { gl_hbox() } else { GL_WBOX });
    let travel = track_len - thumb_len;
    // Grip: how far the cursor sits from the thumb's leading edge at grab, held
    // constant so the thumb does not snap its edge to the cursor (EmuTOS
    // `gr_dragbox` `gr_clamp(sx+1, sy+1, 0, 0)` → `max(mouse - origin, 0)`).
    let (mx0, my0) = crate::vdi::mouse_position();
    let thumb_start = track_start + thumb_off;
    let grip = ((if vert { my0 } else { mx0 }) - thumb_start).max(0);
    let clip = ClipRect::full(backend.width(), backend.height());
    // The `loop` always runs at least once, assigning `pos` before it can break,
    // so a seed value would be a dead store — declare it deferred instead.
    let mut pos;
    loop {
        let (mx, my) = crate::vdi::mouse_position();
        let m = if vert { my } else { mx };
        // New thumb leading edge, constrained to the track [start, start+travel].
        let new_start = (m - grip).clamp(track_start, track_start + travel);
        let rel = new_start - track_start; // travel used, 0..travel
        pos = if travel > 0 {
            super::graf::mul_div_round(rel as i32, 1000, travel as i32) as i16
        } else {
            0
        };
        if vert {
            w.vslide = pos;
        } else {
            w.hslide = pos;
        }
        crate::vdi::cursor_hide(backend);
        // Only the top window's gadgets are ever dragged (`hctl_window`
        // returns before the gadget switch for any other, `gemctrl.c:170-178`),
        // so its topped colour words apply.
        if vert {
            draw_vslider(w, true, backend, clip);
        } else {
            draw_hslider(w, true, backend, clip);
        }
        crate::vdi::cursor_show(backend);
        if crate::vdi::mouse_buttons() & 0x01 == 0 {
            // `gr_slidebox` → `gr_dragbox` → `gr_wait` → `gr_stilldn`
            // (`aes/gemgrlib.c:182,262,451`): its `ev_multi` returning the
            // release ran `ev_rets`, which consumes it.
            super::input::ev_consume();
            break;
        }
        h.pump_input();
    }
    pos
}

/// Block until the left button is released (used after edge-triggered
/// gadgets like topped/arrowed so one press fires once).
fn wait_release() {
    if let Some(h) = host() {
        while crate::vdi::mouse_buttons() & 0x01 != 0 {
            h.pump_input();
        }
    }
}

/// Handle of the open window under `(mx,my)`, **top-most first** (the highest
/// z among the windows whose border rect contains the point); 0 = the desktop
/// window, **−1 = nobody**.
///
/// Mirrors EmuTOS `wm_find` = `ob_find(gl_wtree, 0, 2, x, y)`
/// (`gemsuper.c:348-350`, `gemwmlib.c:1767-1770`), which walks the window tree
/// strictly top-to-bottom from the ROOT. `ob_find` starts with
/// `lastfound = NIL` (`gemoblib.c:1044`, `NIL = -1`, `obdefs.h:21`) and only
/// assigns a handle once the point is found **inside** an object's rectangle —
/// and the ROOT of the window tree carries the desktop window's `WS_CURR`, i.e.
/// `gl_rscreen`. So a point off the screen belongs to no window at all: the
/// desktop is *not* an unconditional fallback.
/// EmuTOS `chk_ctrl` (`aes/geminput.c:86-104`) — who owns the part of the
/// screen the point falls in: `+1` the active process (inside the control
/// rectangle), `-1` the Control Manager (menu bar, or any window), `0` the
/// desktop. `chkown` turns that into an `AESPD *` on the button-down edge
/// (`:310-322`); at single-app `+1` and `0` are both the running application.
///
/// The control rectangle is EmuTOS's `ctrl`, which has exactly two sources:
/// `fm_own` sets it to the whole screen for as long as `ml_ocnt > 0`
/// (`ct_chgown(rlr, &gl_rscreen)`, `aes/gemfmlib.c:93`), and otherwise the
/// Control Manager's `w_setactive` keeps it at the **work** area of the top
/// window (`aes/gemwmlib.c:326-333`). Both are read live here instead of being
/// mirrored in a `set_ctrl` variable — [`MCTRL_CNT`] is `ml_ocnt` and
/// [`get_top`] is `w_top`, so the value is the same one `set_ctrl` would hold.
///
/// The menu-bar test is `inside(mx, my, &gl_rmenu)`, and `gl_rmenu` is the top
/// character row of the screen whether or not a menu is installed
/// (`vdi`-independent, `aes/gemgraf.c:427`) — [`super::menu::desk_top`] is that
/// row's height, and no window's work area reaches into it because `gl_rfull`
/// excludes it. `wm_find` is [`find_handle`]; it answers `DESKWH` (0) for the
/// desktop, which is EmuTOS's `wh == 0` "not a window" case.
pub(super) fn chk_ctrl(mx: i16, my: i16) -> i16 {
    let ctrl = if MCTRL_CNT.load(Ordering::Relaxed) > 0 {
        screen_rect()
    } else {
        match win(get_top()) {
            Some(w) => w.work,
            None => screen_rect(),
        }
    };
    if inside(ctrl, mx, my) {
        return 1;
    }
    if my < super::menu::desk_top() {
        return -1;
    }
    if find_handle(mx, my) > DESKWH {
        -1
    } else {
        0
    }
}

fn find_handle(mx: i16, my: i16) -> i16 {
    let desk = slots()[DESKWH as usize].curr;
    let mut best = if inside(desk, mx, my) { DESKWH } else { -1 };
    let mut best_z = i16::MIN;
    for (i, w) in slots().iter().enumerate() {
        if w.in_use && w.open && inside(w.curr, mx, my) && w.z >= best_z {
            best_z = w.z;
            best = i as i16;
        }
    }
    best
}

/// EmuTOS `wm_find(x, y)` (`aes/gemwmlib.c:1767-1770`): the handle of the
/// window the point falls in — `ob_find(gl_wtree, 0, 2, x, y)`, i.e. the
/// topmost window whose frame contains it, `0` for the desktop window and
/// `-1` for "outside everything". Body shared with the frame interception's
/// own lookup ([`find_handle`]).
pub(super) fn wm_find(x: i16, y: i16) -> i16 {
    find_handle(x, y)
}

/// The owning identity of window `handle` — EmuTOS `D.w_win[wh].w_owner`
/// (`aes/gemwmlib.c` `W_TREE`/`w_owner`). `None` for a handle that names no
/// live window (including [`find_handle`]'s `-1`). The `pub(super)` face of
/// [`owner_of`], for the wheel's `WM_ARROWED`.
pub(super) fn owner_of_window(handle: i16) -> Option<usize> {
    owner_of(handle)
}

/// `D.w_win[w_top()].w_owner` — the process `w_setactive` hands the mouse and
/// the keyboard to (`aes/gemwmlib.c:322-334`): the owner of the top open
/// window, or of the desktop window when none is open ([`get_top`] answers
/// `DESKWH` then, EmuTOS `w_top`, `:315-318`). See [`desk_owner`] for a
/// desktop window without a recorded owner.
pub(super) fn top_owner() -> usize {
    let top = get_top();
    if top == DESKWH {
        return desk_owner();
    }
    owner_of(top).unwrap_or_else(desk_owner)
}

/// `D.w_win[DESKWH].w_owner` — `chkown`'s answer for a press on the desktop
/// (`aes/geminput.c:324`). EmuTOS records `rlr` there in `wm_init`
/// (`w_setup(ppd, DESKWH, NONE)`, `aes/gemwmlib.c:1254-1290`), and `rlr` is
/// the application chain at every `wm_init` the shell loop runs; NewTOS leaves
/// the slot unowned until `WF_NEWDESK` re-owns it, so an unowned desktop window
/// answers the chain, represented by [`super::pd::KERNEL_CONTEXT`].
pub(super) fn desk_owner() -> usize {
    owner_of(DESKWH).unwrap_or(super::pd::KERNEL_CONTEXT)
}

/// `ml_ocnt > 0` — a `wind_update(BEG_MCTRL)` / `fm_own(TRUE)` bracket is in
/// force (`aes/gemfmlib.c:82-107`).
pub(super) fn mctrl_held() -> bool {
    MCTRL_CNT.load(Ordering::Relaxed) > 0
}

/// AES window-frame interception (the `evnt_multi` hook, mirroring the EmuTOS
/// Control Manager's `hctl_window`, `gemctrl.c:158`). On a button press in a
/// window frame, handle the gadget and post the resulting `WM_*` message into
/// the window **owner's** per-app FIFO — the EmuTOS `ct_msgup`→`ap_sendmsg`→
/// `aqueue`/`doq` path (`gemctrl.c:81-102`, `gemwmlib.c:851-863`,
/// `gemqueue.c:85-125`), exactly the routing `send_redraw` already uses. The
/// caller (`evnt_multi`) then delivers it through the regular `MU_MESAG`
/// dequeue, so frame messages obey FIFO order (and same-window redraw / arrow
/// coalescing) instead of clobbering a message the app has not yet read
/// (AUD-P2-001). A click in the work area (or on the desktop) is a no-op.
///
/// **It runs as `ctlmgr`'s loop body, under the screen lock** — `ev_multi`
/// fired, `wm_update(BEG_UPDATE); /* take the screen */`, `hctl_window(…)`,
/// `wm_update(END_UPDATE); /* give up the screen */` (`aes/gemctrl.c:463-484`).
/// While another process holds the lock `ctlmgr` is blocked in that
/// `wm_update`, so a press on a frame is not handled until the holder lets go:
/// the press stays latched ([`super::input::ctl_press_take`] is not consumed)
/// and is handled on the first pass after the release. The handling itself
/// runs in `ctl_pd`'s name ([`ctl_begin`]/[`ctl_finish`]), so the drags'
/// own `wm_update` pair nests under it.
pub fn intercept() {
    // While an app holds mouse control via `wind_update(BEG_MCTRL)`
    // (`ml_ocnt > 0`), the app has taken over the mouse and the AES frame
    // interception switches off — EmuTOS `fm_own` hands the control region to the
    // app so `ctlmgr` no longer processes frame clicks (gemfmlib.c:82-107). See
    // [`wind_update`] and [`mctrl_own`].
    if MCTRL_CNT.load(Ordering::Relaxed) > 0 {
        return;
    }
    // `ctlmgr` blocked in `wm_update(BEG_UPDATE)` (`aes/gemctrl.c:463`): the
    // event is left for later, latch included.
    if !ctl_may_run() {
        return;
    }
    // The level `ctlmgr`'s `ev_multi(MU_BUTTON, …, 0x0001ff01)` tests is the
    // AES's `button` (`ev_multi`'s quick check and `abutton`,
    // `aes/gemevlib.c:185-200`, `aes/geminput.c:549`) — not the raw byte. The
    // two differ only after `ct_msgup`'s HOTCLOSE exit faked the button up
    // (`aes/gemctrl.c:92`): that press has been acted on and must not be
    // taken for a new one while it is still held.
    //
    // Beside the level, `ctlmgr`'s wait is completed by a published press
    // (`post_button(ctl_pd, …)`, `aes/geminput.c:463-483`) even when its
    // release was published with it — a short click a button-delay window
    // held back (`b_delay`, `:155-171`). That press is latched by the input
    // layer and taken here once ([`super::input::ctl_press_take`]).
    let latched = super::input::ctl_press_take();
    if super::input::button() & 0x01 == 0 && !latched {
        return;
    }
    // `wm_update(BEG_UPDATE); /* take the screen */` … `wm_update(END_UPDATE);
    // /* give up the screen */` (`aes/gemctrl.c:463,484`). `ctl_may_run` above
    // answered, so the take succeeds.
    if !ctl_begin() {
        return;
    }
    // `true` = the press ended in `ct_msgup(0, …)`'s wait for the release,
    // which runs inside the screen lock (`aes/gemctrl.c:100-101,463,484`):
    // the lock stays the Control Manager's until the button is up
    // ([`super::input::ct_msgup_busy`] → [`ctl_release`]). A press already
    // released ends that wait at once — the plain `END_UPDATE`.
    let held = hctl_window() && super::input::button() & 0x01 != 0;
    if held {
        super::input::ct_msgup_hold();
    }
    ctl_finish(held);
}

/// The press half of [`intercept`]: `ctlmgr`'s `MU_BUTTON` arm, `wh =
/// wm_find(…); if (wh > 0) hctl_window(wh, …)` (`aes/gemctrl.c:468-470`),
/// run with the screen lock held in `ctl_pd`'s name. Returns `true` when the
/// arm ended in `ct_msgup`'s wait for the button to come up with the press
/// still to be released — the Control Manager's own press on the top window
/// that produced no message (see the `Gadget::None` arm).
fn hctl_window() -> bool {
    let (mx, my) = crate::vdi::mouse_position();
    let handle = find_handle(mx, my);
    // 0 = the desktop window, −1 = no window at all: neither has a frame to
    // intercept.
    if handle <= DESKWH {
        return false;
    }
    let Some(backend) = host().and_then(|h| h.raster()) else {
        return false;
    };
    // The FIFO target is the window owner (EmuTOS `ct_msgup`'s `owner` =
    // `pwin->w_owner`, `gemctrl.c:314`); at single-app this is the running app.
    let Some(owner) = owner_of(handle) else {
        return false;
    };

    let mut msg = [0i16; 8];
    // `ap_sendmsg` sets `msg[1] = rlr->p_pid`, the sender pid (`gemwmlib.c:855`);
    // `hctl_window` runs inside `ctlmgr()`, so that is the Screen Manager's,
    // 1 — for every message this function posts: `WM_TOPPED`, `WM_CLOSED`
    // (both forms), `WM_FULLED`, `WM_MOVED`, `WM_SIZED`, `WM_HSLID`,
    // `WM_VSLID`, and the queued `WM_ARROWED` (`ct_msgup`/`handle_arrow_msg`,
    // `gemctrl.c:125,178,210-220,238,259,300-314`). A stamp, not a process
    // (spec § "AES process identity and pid", contract (b)). NewTOS has no
    // non-queued `WM_ARROWED` (the `p_msg` fast path of `gemctrl.c:129-133`/
    // `gemevlib.c:100-117`, whose stamp would be the reader's 0): every arrow
    // here is queued, and carries 1.
    msg[1] = super::appl::SCRENMGR_PID;
    msg[3] = handle;

    // Click on a non-top window → tell the old top's owner it lost the top spot
    // and the clicked window's owner to bring itself up, then **return without
    // topping anything**. EmuTOS `hctl_window` (`aes/gemctrl.c:170-178`):
    //
    //     if (w_handle != gl_wtop) {
    //         perform_untop(gl_wtop);
    //         ct_msgup(WM_TOPPED, pwin->w_owner, w_handle, 0, 0, 0, 0);
    //         return;
    //     }
    //
    // The topping is the *owner's* to request with `wind_set(WF_TOP)` (TeraDesk
    // `wd_type_topped`, `sdk/apps/teradesk/window.c:2375-2378`; our desktop's
    // `top_msg`, `programs/desktop/src/main.rs:1396-1406`). Doing it here as well
    // made the owner's `WF_TOP` a no-op on an already-topped window — and the
    // repaint it triggered took the window's content with it.
    if handle != get_top() {
        // `ctlmgr`'s `ev_multi` returned on this press (`aes/gemctrl.c:456-
        // 462`); its `ev_rets` consumed the transition (`aes/gemevlib.c:268`).
        super::input::ev_consume();
        let old_top = get_top();
        msg[0] = WM_TOPPED;
        wait_release();
        post_untopped(old_top, handle);
        post_to_owner(owner, &msg);
        return false;
    }

    let w = match win(handle) {
        Some(w) => w,
        None => return false,
    };
    // `hctl_window` reads the window's current border rect once, before the
    // gadget switch — `w_getsize(WS_CURR, w_handle, &t); r_get(&t, &x, &y,
    // &w, &h)` (`aes/gemctrl.c:196-197`) — and `ct_msgup(message, owner, wh,
    // x, y, w, h)` (`:314`) sends whatever the arm left in those four. The
    // closer and fuller arms do not touch them (`:208-221`), so `WM_CLOSED`
    // (both forms) and `WM_FULLED` carry `WS_CURR` in `msg[4..7]` (A77); the
    // elevator arms overwrite `x` alone (`:305`), so `WM_HSLID`/`WM_VSLID`
    // carry the `WS_CURR` `y, w, h` in `msg[5..7]` (A93).
    let ws_curr = w.curr;
    let gad = gadget_at(w, mx, my);
    // A press on a frame gadget is the Control Manager's: `ctlmgr`'s
    // `ev_multi` returned on it (`aes/gemctrl.c:456-462`) and its `ev_rets`
    // consumed the transition (`aes/gemevlib.c:268`). A press in the work area
    // (`Gadget::None`) belongs to the application and is left for it — unless
    // the Control Manager owns the mouse (see the `Gadget::None` arm).
    if !matches!(gad, Gadget::None) {
        super::input::ev_consume();
    }
    match gad {
        Gadget::Closer => {
            // `HOTCLOSE`: `message = WM_CLOSED; break;` — no `gr_watchbox`, the
            // press itself is the click (`aes/gemctrl.c:208-214`). The 3D
            // build's `ob_change(gadget, normal)` after the switch repaints a
            // gadget that was never selected — nothing to reproduce. Then
            // `ct_msgup`'s HOTCLOSE exit (`:86-96`): post the message and, in
            // place of the wait for the release, fake the button up, so the
            // press still held neither re-triggers the Control Manager nor
            // reaches the application as a press.
            if w.kind & HOTCLOSE != 0 {
                msg[0] = WM_CLOSED;
                put_rect(&mut msg, ws_curr);
                post_to_owner(owner, &msg);
                super::input::ct_msgup_hotclose();
                return false;
            }
            // The clicked window is the top one (checked above), so the
            // gadget's topped colour word applies.
            if !watchbox(gad_rect(w, Gadget::Closer), G_CLOSE, gadget_ink(w, W_CLOSER_IDX, true), backend) {
                return false;
            }
            msg[0] = WM_CLOSED;
            put_rect(&mut msg, ws_curr);
        }
        Gadget::Fuller => {
            if !watchbox(gad_rect(w, Gadget::Fuller), G_FULL, gadget_ink(w, W_FULLER_IDX, true), backend) {
                return false;
            }
            msg[0] = WM_FULLED;
            put_rect(&mut msg, ws_curr);
        }
        Gadget::Mover => {
            let r = drag_move(w, backend);
            msg[0] = WM_MOVED;
            msg[4] = r.x;
            msg[5] = r.y;
            msg[6] = r.w;
            msg[7] = r.h;
        }
        Gadget::Sizer => {
            let r = drag_size(w, backend);
            msg[0] = WM_SIZED;
            msg[4] = r.x;
            msg[5] = r.y;
            msg[6] = r.w;
            msg[7] = r.h;
        }
        // Scroll arrows and track-page clicks both post WM_ARROWED with a
        // WA_* sub-action (EmuTOS `handle_arrow_msg`, gemctrl.c:111-139). Like
        // the existing arrow arms these fire once per press (our single-tasking
        // model does not run the cooperative auto-repeat loop; the page click
        // is edge-triggered, same as the line arrows already are here).
        Gadget::Up
        | Gadget::Dn
        | Gadget::Lf
        | Gadget::Rt
        | Gadget::VUpPage
        | Gadget::VDnPage
        | Gadget::HLfPage
        | Gadget::HRtPage => {
            msg[0] = WM_ARROWED;
            msg[4] = match gad {
                Gadget::Up => WA_UPLINE,
                Gadget::Dn => WA_DNLINE,
                Gadget::Lf => WA_LFLINE,
                Gadget::Rt => WA_RTLINE,
                Gadget::VUpPage => WA_UPPAGE,
                Gadget::VDnPage => WA_DNPAGE,
                Gadget::HLfPage => WA_LFPAGE,
                _ => WA_RTPAGE,
            };
            // `hctl_window` brackets `handle_arrow_msg` with the gadget's
            // pressed state: `ob_change(gl_awind, gadget, selected, TRUE)` …
            // `handle_arrow_msg(w_handle, cpt)` … `ob_change(gl_awind, gadget,
            // normal, TRUE)` (`aes/gemctrl.c:272-284`) — an **arrow** only: the
            // guard `if ((gadget != W_HSLIDE) && (gadget != W_VSLIDE))` leaves
            // a page click in the track without feedback, because there
            // `gadget` is the slide box the point fell in while `cpt` was
            // moved on to the elevator (`:262-270`). `selected = normal |
            // SELECTED` with `normal = ob_state & ~SELECTED` (`:189-194`), so
            // the only bit that moves is `SELECTED` — [`gadget_sel`]'s `sel`.
            // The message cadence is untouched: one `WM_ARROWED`, posted after
            // the release as before (A109).
            arrow_change(w, gad, true, backend);
            wait_release();
            arrow_change(w, gad, false, backend);
        }
        // Elevator-thumb drag → post the final 0..1000 position as WM_VSLID /
        // WM_HSLID (EmuTOS `hctl_window` W_?ELEV, `aes/gemctrl.c:295-306`).
        // The arm overwrites **only `x`** — `x = gr_slidebox(gl_awind, cpt-1,
        // cpt, (cpt == W_VELEV))` (`:305`) — so `ct_msgup(…, x, y, w, h)`
        // (`:314`) still carries the `WS_CURR` `y, w, h` read before the
        // switch (`:196-197`) in `msg[5..7]` (A93; the same shape as
        // `WM_CLOSED`/`WM_FULLED` above, A77).
        Gadget::VElev => {
            msg[0] = WM_VSLID;
            put_rect(&mut msg, ws_curr);
            msg[4] = drag_elevator(w, true, backend);
        }
        Gadget::HElev => {
            msg[0] = WM_HSLID;
            put_rect(&mut msg, ws_curr);
            msg[4] = drag_elevator(w, false, backend);
        }
        // No control point: `message` stays 0 and `hctl_window` ends in
        // `ct_msgup(0, …)` — no message, then `while (button & 0x0001)
        // dsptch();` inside `ctlmgr`'s screen lock (`aes/gemctrl.c:81-102,
        // 182,314`). `ctlmgr` reaches this only for a press **it** owns
        // (`ev_multi`'s quick check `rlr == gl_mowner`,
        // `aes/gemevlib.c:185`; `post_button(gl_mowner, …)`,
        // `aes/geminput.c:463-483`): the press that cancelled a drop-down,
        // which `bchange` left with `ctl_pd` because `gl_ctmown` was up
        // (`aes/geminput.c:334-335`, `aes/gemmnlib.c:277`), or one outside
        // the top window's work area (`chkown` → `ctl_pd`,
        // `aes/geminput.c:310-322`). It is eaten (`ev_rets`,
        // `aes/gemevlib.c:268`) and the screen stays the Control Manager's
        // until the release — an application that asks for it meanwhile
        // (`wind_update(BEG_UPDATE/BEG_MCTRL)`) gets it with the button up,
        // so its `fm_own(TRUE)` → `set_mown` can hand it the mouse
        // (`aes/geminput.c:220-235`). A work-area press the application owns
        // never reaches `ctlmgr` and is left alone.
        Gadget::None => {
            if super::input::own_mouse() {
                return false;
            }
            super::input::ev_consume();
            return true;
        }
    }
    post_to_owner(owner, &msg);
    false
}

/// Post a Control-Manager `WM_*` message into `owner`'s per-app FIFO — the
/// EmuTOS `ap_sendmsg`→`aqueue`/`doq` enqueue path (`gemwmlib.c:851-863`,
/// `gemqueue.c:85-125`), the same routing [`send_redraw`] uses. This replaces
/// the former direct-`pbuff` write so frame messages queue in FIFO order (and
/// `WM_ARROWED` coalesces) instead of overwriting a message still unread by
/// the app (AUD-P2-001).
fn post_to_owner(owner: usize, msg: &[i16; 8]) {
    // SAFETY: single trap path; `post_msg` hands the message to the live slot.
    unsafe {
        super::pd::post_msg(owner, *msg);
    }
}

/// Write `r` into the four rect words of a Control-Manager message — the
/// `x, y, w, h` tail of `ct_msgup(message, owner, wh, x, y, w, h)`
/// (`aes/gemctrl.c:314`), landing in `msg[4..7]` via `ap_sendmsg`
/// (`gemwmlib.c:851-863`).
fn put_rect(msg: &mut [i16; 8], r: Grect) {
    msg[4] = r.x;
    msg[5] = r.y;
    msg[6] = r.w;
    msg[7] = r.h;
}

/// Post `WM_UNTOPPED` (30) to the previously-topped window's owner when the top
/// changes — EmuTOS `perform_untop` (gemctrl.c:146-155): the old top's owner is
/// told its window is no longer active (`msg[3]` = old top handle). No message
/// when `old_top` is the same window being (re-)topped or is not a real window
/// (0 = desktop / none). Routed through the owner FIFO like every other WM_*
/// (no `pbuff` bypass, AUD-P2-001).
fn post_untopped(old_top: i16, new_top: i16) {
    if old_top == new_top || old_top == DESKWH {
        return;
    }
    if let Some(owner) = owner_of(old_top) {
        let mut msg = [0i16; 8];
        msg[0] = WM_UNTOPPED;
        // `ap_sendmsg` sets msg[1] = sender pid (gemwmlib.c:855); `perform_untop`
        // runs inside `ctlmgr()` (gemctrl.c:146-155,174), so the Screen
        // Manager's 1.
        msg[1] = super::appl::SCRENMGR_PID;
        msg[3] = old_top;
        post_to_owner(owner, &msg);
    }
}

/// Test hook (not wired to boot): the message at the **front** of the FIFO
/// queue of the process reporting `pid` (without removing it), or `None` if
/// the queue is empty. Lets the P5 redraw-broadcast selftest observe the
/// `WM_REDRAW` posted on `wind_close` / `wind_set(WF_TOP)`.
pub fn test_pending_msg(pid: i16) -> Option<[i16; 8]> {
    // SAFETY: single-CPU AES; test scaffolding on the boot trap path.
    unsafe { test_pd(pid).and_then(|p| p.queue.front()) }
}

/// Test hook (not wired to boot): number of messages queued for the process
/// reporting `pid` (0 when empty). Lets a selftest assert the WM_REDRAW
/// rectangle-union coalesces two same-window redraws into a single queue entry.
pub fn test_msg_count(pid: i16) -> usize {
    // SAFETY: single-CPU AES; test scaffolding on the boot trap path.
    unsafe { test_pd(pid).map(|p| p.queue.pending()).unwrap_or(0) }
}

/// Test hook (not wired to boot): drop every pending message of the process
/// reporting `pid`, so the P5 selftest can isolate the `WM_REDRAW` a subsequent
/// window op posts.
pub fn test_clear_msg(pid: i16) {
    // SAFETY: single-CPU AES; test scaffolding on the boot trap path.
    unsafe {
        if let Some(p) = test_pd(pid) {
            p.queue.clear();
        }
    }
}

/// The descriptor of the process that reports `pid` — the test hooks take
/// the value `appl_init` handed the caller (`appl::identity_of`).
///
/// # Safety
/// Single trap path.
unsafe fn test_pd(pid: i16) -> Option<&'static mut super::pd::Aespd> {
    // SAFETY: forwarded.
    super::appl::identity_of(pid).and_then(|idx| unsafe { super::pd::by_index(idx) })
}

/// Test hook (not wired to boot): set the stacking counter [`bump_z`] hands
/// out next, so a host test can drive it to the edge of `i16` (A52) without
/// opening 32767 windows. Returns the previous value.
pub fn test_set_next_z(z: i16) -> i16 {
    // SAFETY: single-CPU AES; test scaffolding.
    unsafe {
        let p = TABLE.next_z.get();
        let old = *p;
        *p = z;
        old
    }
}

/// Test hook (not wired to boot): the stacking value the open window
/// `handle` carries, or `None` for a free / closed slot.
pub fn test_window_z(handle: i16) -> Option<i16> {
    win(handle).filter(|w| w.open).map(|w| w.z)
}

/// Test hook (not wired to boot): the identity that owns the in-use window
/// `handle` (`w_owner` — a GEMDOS slot, or `MAX_PROCESSES` for the kernel
/// desktop), or `None` for a free slot / no such handle. Lets a host test
/// state which windows a sweep took and which it kept (A96).
pub fn test_window_owner(handle: i16) -> Option<usize> {
    owner_of(handle)
}

/// Test hook (not wired to boot): the counting window-tree lock as
/// `(sy_tas, sy_owner)` — the count and the identity holding it, `None` for
/// free. Lets a host test state that a dead client's `BEG_UPDATE` is released
/// and a living one's is not (A110).
pub fn test_update_lock() -> (i16, Option<usize>) {
    let owner = UPDATE_OWNER.load(Ordering::Relaxed);
    (UPDATE_LOCK.load(Ordering::Relaxed), (owner != NO_OWNER).then_some(owner))
}

/// Test hook (not wired to boot): the mouse-control-ownership counter as
/// `(ml_ocnt, owner)` — see [`test_update_lock`].
pub fn test_mctrl_count() -> (i16, Option<usize>) {
    let owner = MCTRL_OWNER.load(Ordering::Relaxed);
    (MCTRL_CNT.load(Ordering::Relaxed), (owner != NO_OWNER).then_some(owner))
}

/// Test hook (not wired to boot): the registered desktop-background tree
/// (`gl_newdesk`), 0 when none is registered — so a host test can state
/// that a sweep dropped a dead program's registration (A96/A97).
pub fn test_newdesk_tree() -> u64 {
    NEWDESK_TREE.load(Ordering::Acquire)
}

/// Test hook: name of the gadget under a point (for the headless logic
/// check in the selftest).
pub fn gadget_name_at(handle: i16, mx: i16, my: i16) -> &'static str {
    match win(handle) {
        Some(w) => match gadget_at(w, mx, my) {
            Gadget::None => "none",
            Gadget::Closer => "closer",
            Gadget::Fuller => "fuller",
            Gadget::Mover => "mover",
            Gadget::Sizer => "sizer",
            Gadget::Up => "up",
            Gadget::Dn => "dn",
            Gadget::Lf => "lf",
            Gadget::Rt => "rt",
            Gadget::VElev => "velev",
            Gadget::HElev => "helev",
            Gadget::VUpPage => "vuppage",
            Gadget::VDnPage => "vdnpage",
            Gadget::HLfPage => "hlfpage",
            Gadget::HRtPage => "hrtpage",
        },
        None => "no-window",
    }
}

#[cfg(test)]
mod newdesk_root_word_tests {
    //! `set_newdesk` reads the `WF_NEWDESK` root companion word at the position
    //! the pointer convention in use dictates (A81, `docs/api/aes/
    //! window-graphics.md` "Pointers in `addr_in[]`", edge case 16):
    //!
    //! * native form (`AIN_LEN >= 1`): tree in `addr_in[0]`, root in `int_in[2]`;
    //! * packed form (`AIN_LEN == 0`): tree packed at the caller's pointer width
    //!   from `int_in[2]`, root in the word right after it — `int_in[4]` for a
    //!   2-word ILP32 pointer (EmuTOS `pinwds[2]`, `aes/gemwmlib.c:1688`),
    //!   `int_in[6]` for a 4-word LP64 pointer.
    //!
    //! The handler never dereferences the tree, so the pointers here are the
    //! literal values from `TASK/A81.md`; the registered pair is read back from
    //! [`NEWDESK_TREE`]/[`NEWDESK_ROOT`] directly. The ILP32 case is built as a
    //! pre-marshalled block (what [`Aespb::from_raw`] produces) because a host
    //! process has no 32-bit-addressable arrays for the block itself.
    use super::*;
    use crate::aes::{AI_SIZE, I_SIZE, O_SIZE};
    use std::sync::Mutex;

    /// The statics are process-global; the cases run serially — and
    /// [`super::newdesk_owner_tests`], which sets the same statics, takes
    /// this lock too.
    pub(super) static SERIAL: Mutex<()> = Mutex::new(());

    const OP_WIND_SET: i16 = 105;

    /// The tree of `TASK/A81.md`'s measured case: an LP64 EL0 address whose
    /// bits 32..47 are `0x0001` — exactly the word a fixed `int_in[4]` read
    /// picks up as "the root" from an LP64-packed pointer.
    const LP64_TREE: u64 = 0x0000_0001_0177_E4C0;
    /// The same tree as an ILP32 caller carries it (4 bytes, under 4 GiB).
    const ILP32_TREE: u64 = 0x0177_E4C0;

    struct Block {
        control: [i16; 5],
        intin: [i16; I_SIZE],
        intout: [i16; O_SIZE],
        addrin: [u64; AI_SIZE],
        global: [i16; 16],
    }

    impl Block {
        fn new(in_len: i16, ain_len: i16) -> Block {
            Block {
                control: [OP_WIND_SET, in_len, 1, ain_len, 0],
                intin: [0; I_SIZE],
                intout: [0; O_SIZE],
                addrin: [0; AI_SIZE],
                global: [0; 16],
            }
        }

        /// An [`Aespb`] over this block as [`Aespb::from_raw`] would have
        /// marshalled it: the local `int_in`/`addr_in` already hold the
        /// declared words. `abi` is the caller pointer width the packed-form
        /// read follows.
        fn pb(&mut self, abi: Abi) -> Aespb {
            Aespb {
                control: self.control.as_mut_ptr(),
                global: self.global.as_mut_ptr() as u64,
                intin: self.intin.as_mut_ptr(),
                caller_intin: self.intin.as_ptr(),
                intout: self.intout.as_mut_ptr(),
                caller_intout: self.intout.as_mut_ptr() as u64,
                addrin: self.addrin.as_mut_ptr(),
                addrout: 0,
                abi,
                space: crate::uaccess::Space::Kernel,
            }
        }
    }

    /// Pack `p` little-endian into `words` consecutive `int_in` WORDs at
    /// `at` — the packed form at the caller's own pointer width (gemlib's
    /// `*(void**)(aes_intin + 2) = v` on a little-endian LP64/ILP32 target).
    fn pack(intin: &mut [i16; I_SIZE], at: usize, p: u64, words: usize) {
        for i in 0..words {
            intin[at + i] = ((p >> (16 * i)) & 0xFFFF) as u16 as i16;
        }
    }

    fn registered() -> (u64, i16) {
        (NEWDESK_TREE.load(Ordering::Acquire), NEWDESK_ROOT.load(Ordering::Acquire))
    }

    fn reset() {
        NEWDESK_TREE.store(0, Ordering::Release);
        NEWDESK_ROOT.store(0, Ordering::Release);
    }

    /// LP64 packed form: the 8-byte pointer spans `int_in[2..5]`, the root is
    /// the word after it, `int_in[6]`. Root 2 with this tree is the measured
    /// A81 case; a fixed `int_in[4]` read returns the pointer's bits 32..47
    /// (`0x0001`) as the root instead.
    #[test]
    fn lp64_packed_root_is_the_word_after_the_four_pointer_words() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let mut b = Block::new(7, 0);
        b.intin[0] = 0; // DESKWH
        b.intin[1] = WF_NEWDESK;
        pack(&mut b.intin, 2, LP64_TREE, 4);
        b.intin[6] = 2;
        set_newdesk(&b.pb(Abi::Lp64));
        assert_eq!(
            registered(),
            (LP64_TREE, 2),
            "LP64 packed wind_set(WF_NEWDESK): tree int_in[2..5], root int_in[6] \
             (docs/api/aes/window-graphics.md, edge case 16; TASK/A81.md)"
        );
    }

    /// Native form: tree in `addr_in[0]`, root in `int_in[2]` — what the native
    /// desktop sends (`programs/desktop/src/lib.rs::wind_set_newdesk`).
    #[test]
    fn native_form_root_is_int_in_2() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let mut b = Block::new(3, 1);
        b.intin[0] = 0;
        b.intin[1] = WF_NEWDESK;
        b.intin[2] = 1;
        b.addrin[0] = LP64_TREE;
        set_newdesk(&b.pb(Abi::Lp64));
        assert_eq!(registered(), (LP64_TREE, 1), "native wind_set(WF_NEWDESK): addr_in[0] + int_in[2]");
    }

    /// ILP32 packed form: the 4-byte pointer spans `int_in[2..3]`, the root is
    /// `int_in[4]` — bit-exact with EmuTOS's `pinwds[2]` (`aes/gemwmlib.c:1688`).
    /// Regression guard: the LP64 fix must not move the ILP32 root word.
    #[test]
    fn ilp32_packed_root_is_int_in_4() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let mut b = Block::new(6, 0);
        b.intin[0] = 0;
        b.intin[1] = WF_NEWDESK;
        pack(&mut b.intin, 2, ILP32_TREE, 2);
        b.intin[4] = 1;
        // Poison the LP64 root slot: an ILP32 caller's `int_in[6]` is never
        // part of this call and must not be read.
        b.intin[6] = 0x7FFF;
        set_newdesk(&b.pb(Abi::Ilp32));
        assert_eq!(registered(), (ILP32_TREE, 1), "ILP32 packed wind_set(WF_NEWDESK): tree int_in[2..3], root int_in[4]");
    }
}

#[cfg(test)]
mod live_title_tests {
    //! A83 — `WF_NAME`/`WF_INFO` are **pointers read live through the owner's
    //! mapping** ([`LiveStr`]). The in-place-rewrite (T10) and no-79-cap cases
    //! run end-to-end through the dispatcher in
    //! `tests/aes_window_name_pointer.rs` with kernel (EL1) pointers. The
    //! EL0-owner cases cannot: a host process has no process table, and the
    //! kernel high-half alias `usermem::resolve_kva` computes is not a host
    //! address. They are modelled at the [`owner_kva`] seam instead — a
    //! resolver over host buffers stands in for the owner's block table — and
    //! the frame is built by [`draw_frame`] directly on a raster that records
    //! the glyphs it is asked to paint (no host installed, no shared window
    //! table touched, so the other unit tests in this binary are unaffected).
    //!
    //! * **owner mapping, not the current one** — with two slots mapping the
    //!   *same* VA to different strings, the title comes from the slot the
    //!   `wind_set` caller owned, and the resolver is asked for that slot
    //!   only. (`process::current()` is null on the host; the reader never
    //!   consults it — that is the modelled statement.)
    //! * **dangling pointer** — a VA the owner has no mapping for renders as
    //!   `""`, and nothing is dereferenced.
    //! * **block end** — a string that runs off the end of the owner's block
    //!   ends there; the unmapped next page is never read.
    //! * **page crossing** — a string spanning two pages is read from both,
    //!   each page resolved on its own (the two pages are separate host
    //!   allocations, so a single base-plus-offset read could not produce
    //!   the concatenation).
    use super::*;
    use crate::vdi::backend::{MonoSrc, PatternFill, Rect};
    use std::sync::Mutex;

    // ── the resolver stand-in ──────────────────────────────────────────

    /// One modelled block: `slot` maps `[va, va + len)` to host memory at
    /// `host` (page-aligned VA and page-rounded length, as `usermem::Block`).
    /// Shared with [`super::newdesk_owner_tests`], which models the A97
    /// tree reads at the same seam.
    #[derive(Clone, Copy)]
    pub(super) struct Map {
        pub(super) slot: usize,
        pub(super) va: u64,
        pub(super) len: u64,
        pub(super) host: u64,
    }

    static MAPS: Mutex<Vec<Map>> = Mutex::new(Vec::new());
    pub(super) static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    /// The seam is process-global; the cases run serially.
    pub(super) static SERIAL: Mutex<()> = Mutex::new(());

    /// The resolver [`owner_kva`] consults while a case is running.
    pub(super) fn resolver() -> Option<fn(usize, u64) -> Option<u64>> {
        ACTIVE.load(Ordering::Acquire).then_some(fake_resolve)
    }

    fn fake_resolve(slot: usize, va: u64) -> Option<u64> {
        ASKED.lock().unwrap().push(slot);
        MAPS.lock()
            .unwrap()
            .iter()
            .find(|m| m.slot == slot && va >= m.va && va - m.va < m.len)
            .map(|m| m.host + (va - m.va))
    }

    /// Run `f` with `maps` installed at the seam, then take it down.
    pub(super) fn with_maps<R>(maps: &[Map], f: impl FnOnce() -> R) -> R {
        *MAPS.lock().unwrap() = maps.to_vec();
        ASKED.lock().unwrap().clear();
        ACTIVE.store(true, Ordering::Release);
        let r = f();
        ACTIVE.store(false, Ordering::Release);
        MAPS.lock().unwrap().clear();
        r
    }

    /// A page-sized host block holding `s` at offset `at` (NUL-terminated
    /// unless `s` fills the block to its end).
    pub(super) fn page_with(s: &[u8], at: usize) -> Box<[u8]> {
        let mut p = vec![0u8; usermem::PAGE_SIZE as usize].into_boxed_slice();
        p[at..at + s.len()].copy_from_slice(s);
        p
    }

    // ── a raster that records the glyphs it paints ─────────────────────

    struct GlyphLog {
        cells: Mutex<Vec<(i16, i16, [u8; 16])>>,
    }

    impl GlyphLog {
        fn new() -> GlyphLog {
            GlyphLog { cells: Mutex::new(Vec::new()) }
        }

        /// The glyph rows painted at `y` (the title row), left to right.
        fn row(&self, y: i16) -> Vec<(i16, [u8; 16])> {
            let mut v: Vec<(i16, [u8; 16])> =
                self.cells.lock().unwrap().iter().filter(|c| c.1 == y).map(|c| (c.0, c.2)).collect();
            v.sort_by_key(|c| c.0);
            v
        }
    }

    impl RasterBackend for GlyphLog {
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
            x: i16,
            y: i16,
            glyph: &[u8],
            _cell_w: i16,
            _cell_h: i16,
            _fg: Pel,
            _mode: WriteMode,
            _clip: ClipRect,
        ) {
            let mut g = [0u8; 16];
            g[..glyph.len().min(16)].copy_from_slice(&glyph[..glyph.len().min(16)]);
            self.cells.lock().unwrap().push((x, y, g));
        }
        fn blit_mono(&self, _dst: Rect, _src: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _clip: ClipRect) {}
        fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
        fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
            0
        }
        fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
        fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
    }

    // ── the stage ──────────────────────────────────────────────────────

    /// A titled window's border rect.
    const R: Grect = Grect { x: 30, y: 40, w: 320, h: 240 };
    /// The row [`draw_label`] paints the title glyphs on: `gy + 1`,
    /// `gy = b.y + ADJ3DSTD` ([`draw_frame`]).
    const TITLE_ROW: i16 = R.y + ADJ3DSTD + 1;

    /// An open window with a title bar, its name string `name`.
    fn titled(name: LiveStr) -> Window {
        let mut w = Window::EMPTY;
        w.in_use = true;
        w.open = true;
        w.kind = NAME | CLOSER | FULLER | MOVER;
        w.curr = R;
        w.name = name;
        w
    }

    /// The title-row glyphs the frame of `w` paints.
    fn title_of(w: &Window) -> Vec<(i16, [u8; 16])> {
        let log = GlyphLog::new();
        draw_frame(w, true, &log, ClipRect::full(640, 480));
        log.row(TITLE_ROW)
    }

    /// The title row of a window titled by a kernel (EL1) pointer to `s` —
    /// the reference every EL0 case is held against.
    fn kernel_title(s: &'static [u8]) -> Vec<(i16, [u8; 16])> {
        title_of(&titled(LiveStr { ptr: s.as_ptr() as u64, slot: None }))
    }

    const PAGE: u64 = usermem::PAGE_SIZE;
    /// A low user VA, page-aligned, the way the EL0 heap window hands them out.
    const VA: u64 = 0x0040_0000;

    #[test]
    fn the_string_is_read_through_the_owners_mapping_not_the_current_one() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let owner = page_with(b"OWNER\0", 0);
        let other = page_with(b"OTHER\0", 0);
        let maps = [
            Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 },
            Map { slot: 5, va: VA, len: PAGE, host: other.as_ptr() as u64 },
        ];
        let (title, asked) = with_maps(&maps, || {
            let t = title_of(&titled(LiveStr { ptr: VA, slot: Some(3) }));
            (t, ASKED.lock().unwrap().clone())
        });
        assert_eq!(title, kernel_title(b"OWNER\0"), "the title must be the owner slot's string");
        assert_ne!(title, kernel_title(b"OTHER\0"), "the same VA in another slot must not be read");
        assert!(!asked.is_empty() && asked.iter().all(|&s| s == 3), "resolved through slots {asked:?}, not the owner's 3 only");
    }

    #[test]
    fn a_dangling_el0_pointer_is_an_empty_title_and_touches_nothing() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let owner = page_with(b"OWNER\0", 0);
        let maps = [Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 }];
        let title = with_maps(&maps, || {
            // A VA the owner never had — a freed block, or a slot re-seeded
            // for a new process (A96): the resolver answers `None`.
            title_of(&titled(LiveStr { ptr: VA + 8 * PAGE, slot: Some(3) }))
        });
        assert_eq!(title, kernel_title(b"\0"), "an unresolvable VA must render as the empty title");
        assert_eq!(title, title_of(&titled(LiveStr::NONE)));
    }

    #[test]
    fn a_string_running_off_the_owners_block_ends_at_the_block_end() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // "ABC" in the last three bytes of the block, no NUL; the next page
        // is not the owner's.
        let last = PAGE as usize - 3;
        let owner = page_with(b"ABC", last);
        let maps = [Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 }];
        let title = with_maps(&maps, || title_of(&titled(LiveStr { ptr: VA + last as u64, slot: Some(3) })));
        assert_eq!(title, kernel_title(b"ABC\0"), "the read must stop at the end of the owner's block");
    }

    #[test]
    fn a_string_spanning_two_pages_is_read_from_both() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // Two separately allocated host pages model two pages of one block
        // (or two adjacent blocks): the first ends with "SPAN", the second
        // starts with "NING\0".
        let head = page_with(b"SPAN", PAGE as usize - 4);
        let tail = page_with(b"NING\0", 0);
        let maps = [
            Map { slot: 3, va: VA, len: PAGE, host: head.as_ptr() as u64 },
            Map { slot: 3, va: VA + PAGE, len: PAGE, host: tail.as_ptr() as u64 },
        ];
        let title = with_maps(&maps, || title_of(&titled(LiveStr { ptr: VA + PAGE - 4, slot: Some(3) })));
        assert_eq!(title, kernel_title(b"SPANNING\0"), "each page must be resolved on its own");
    }

    #[test]
    fn label_walk_is_the_areas_glyph_count_plus_one() {
        // 1850 px name area (the 1900-px window of the integration case) →
        // 231 glyphs fit, the walk reads 232 so a longer string measures
        // longer than the area; a degenerate area still walks one byte.
        assert_eq!(label_walk(1850), 232);
        assert_eq!(label_walk(0), 1);
        assert_eq!(label_walk(-5), 1);
    }

    /// A105 (3): the default / minimum elevator is the gadget cell along the
    /// track's axis — `gl_hbox` vertically (`w_bldvbar`, `gemwmlib.c:431-434`),
    /// `gl_wbox` horizontally (`w_bldhbar`, `:469-471`). The two are equal
    /// on this profile (19/19), so the helper is pinned with a pair that
    /// differs, as on a screen with non-square pixels (`gl_wbox = gl_hbox ·
    /// aspect`, `gemgraf.c:413-415`).
    #[test]
    fn thumb_geom_takes_the_axis_minimum() {
        // `-1` = "default size" = exactly the axis minimum, at the track start.
        assert_eq!(thumb_geom(200, -1, 0, 19), (0, 19));
        assert_eq!(thumb_geom(200, -1, 0, 23), (0, 23));
        // A per-mille size under the minimum is raised to it — and the travel
        // (`h - size`, `:435`) shrinks by the same amount.
        assert_eq!(thumb_geom(200, 50, 1000, 19), (181, 19));
        assert_eq!(thumb_geom(200, 50, 1000, 23), (177, 23));
        // A size above the minimum is untouched by it.
        assert_eq!(thumb_geom(200, 500, 500, 19), (50, 100));
        assert_eq!(thumb_geom(200, 500, 500, 23), (50, 100));
        // A track shorter than the minimum caps the elevator at the track.
        assert_eq!(thumb_geom(10, -1, 0, 23), (0, 10));
    }
}

#[cfg(test)]
mod newdesk_owner_tests {
    //! A97 — the `WF_NEWDESK` tree is **read through its owner's mapping** at
    //! composite time ([`NEWDESK_OWNER`], `object.rs` `ObjTree::app_owned`).
    //! The registrant's tree, its sub-structs, strings and rasters are user
    //! VAs of that process; the composite runs from other processes' AES
    //! calls and from kernel code, under other `TTBR0`s. Modelled at
    //! the [`owner_kva`] seam exactly as the A83 title cases
    //! ([`live_title_tests`]): a resolver over host pages stands in for the
    //! owner's block table, the registration state is set directly (the
    //! `wind_set` marshalling is `newdesk_root_word_tests`' business), and
    //! [`draw_desk_tree`] is called on a raster that records what it is
    //! asked to paint. `VA` is a low user address that is **not** mapped in
    //! the host process, so a raw dereference would fault: every case
    //! finishing is the "no fault" half of the ticket.
    //!
    //! * **owner mapping, not the current one** — two slots map the same VA
    //!   to different trees; the composite draws the owner's and asks the
    //!   resolver for the owner's slot only;
    //! * **dangling** — the owner has no mapping for the tree any more: the
    //!   AES fallback root is drawn, the tree is never touched;
    //! * **the tree's icon** — ICONBLK, mask/data rasters and label are
    //!   reached through the same mapping (the sub-struct and span readers);
    //! * **a raster the owner does not map is skipped**, the rest of the
    //!   icon still drawn;
    //! * **a raster spanning two pages** is drawn only if the owner's alias
    //!   of the second page continues the first (one block), not when the
    //!   two pages are separate allocations.
    use super::live_title_tests::{page_with, with_maps, Map, ASKED, SERIAL as SEAM};
    use super::newdesk_root_word_tests::SERIAL as STATE;
    use super::*;
    use crate::vdi::backend::{MonoSrc, Rect};
    use std::sync::Mutex;

    // ── a raster that records the fills, blits and glyphs it paints ────

    #[derive(Debug, PartialEq, Clone, Copy)]
    enum Paint {
        Rect(i16, i16, i16, i16, Pel),
        Pattern(i16, i16, i16, i16, Pel),
        Mono(i16, i16, i16, i16, Pel),
        Glyph(i16, i16),
    }

    struct PaintLog(Mutex<Vec<Paint>>);

    impl PaintLog {
        fn new() -> PaintLog {
            PaintLog(Mutex::new(Vec::new()))
        }
        fn take(&self) -> Vec<Paint> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }

    impl RasterBackend for PaintLog {
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
        fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, _clip: ClipRect) {
            self.0.lock().unwrap().push(Paint::Rect(x, y, w, h, pel));
        }
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
        fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, _clip: ClipRect) {
            self.0.lock().unwrap().push(Paint::Pattern(x, y, w, h, fill.fg));
        }
        #[allow(clippy::too_many_arguments)]
        fn draw_glyph_sized(
            &self,
            x: i16,
            y: i16,
            _glyph: &[u8],
            _cell_w: i16,
            _cell_h: i16,
            _fg: Pel,
            _mode: WriteMode,
            _clip: ClipRect,
        ) {
            self.0.lock().unwrap().push(Paint::Glyph(x, y));
        }
        fn blit_mono(&self, dst: Rect, _src: MonoSrc, fg: Pel, _bg: Pel, _m: WriteMode, _clip: ClipRect) {
            self.0.lock().unwrap().push(Paint::Mono(dst.x, dst.y, dst.w, dst.h, fg));
        }
        fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
        fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
            0
        }
        fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
        fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
    }

    // ── the stage ──────────────────────────────────────────────────────

    const PAGE: u64 = usermem::PAGE_SIZE;
    /// A low user VA, page-aligned, the way the EL0 heap window hands them
    /// out — and unmapped in the host process.
    const VA: u64 = 0x0040_0000;
    use tos_types::aes::{G_BOX, G_ICON};
    /// `ob_spec` of a solid box in colour `c` (`gr_crack`: pattern 7 =
    /// IP_SOLID in bits 4..6, interior colour in bits 0..3, thickness 0).
    const fn solid(c: u16) -> u64 {
        (0x70 | c) as u64
    }

    fn object(ty: u16, next: i16, head: i16, tail: i16, spec: u64, r: (i16, i16, i16, i16)) -> Object {
        Object {
            ob_next: next,
            ob_head: head,
            ob_tail: tail,
            ob_type: ty,
            ob_flags: 0,
            ob_state: 0,
            ob_spec: spec,
            ob_x: r.0,
            ob_y: r.1,
            ob_width: r.2,
            ob_height: r.3,
        }
    }

    /// Write `objs` as native (LP64, 32-byte) records at `at` in `page`.
    fn put_objects(page: &mut [u8], at: usize, objs: &[Object]) {
        for (i, o) in objs.iter().enumerate() {
            let bytes: [u8; 32] = unsafe { core::mem::transmute(*o) };
            page[at + i * 32..at + i * 32 + 32].copy_from_slice(&bytes);
        }
    }

    /// Write an LP64 ICONBLK at `at`: three pointers, then the eleven WORDs
    /// `ib_char, xchar, ychar, xicon, yicon, wicon, hicon, xtext, ytext,
    /// wtext, htext` (`mt_gem.h:2004-2020`).
    fn put_iconblk(page: &mut [u8], at: usize, pmask: u64, pdata: u64, ptext: u64, words: [i16; 11]) {
        page[at..at + 8].copy_from_slice(&pmask.to_le_bytes());
        page[at + 8..at + 16].copy_from_slice(&pdata.to_le_bytes());
        page[at + 16..at + 24].copy_from_slice(&ptext.to_le_bytes());
        for (i, w) in words.iter().enumerate() {
            page[at + 24 + 2 * i..at + 26 + 2 * i].copy_from_slice(&w.to_le_bytes());
        }
    }

    /// A 32×32 mono raster (2 words per row, 32 rows = 128 bytes), all set.
    fn put_raster(page: &mut [u8], at: usize) {
        for b in &mut page[at..at + 128] {
            *b = 0xFF;
        }
    }

    /// Register `tree` (root `root`) as an EL0 app's background tree owned
    /// by `owner`, at LP64 width — what `set_newdesk` records for an LP64
    /// `.NAP` registrant (`NEWDESK_APP`, `NEWDESK_ABI`, `NEWDESK_OWNER`).
    fn register(tree: u64, root: i16, owner: usize) {
        NEWDESK_APP.store(true, Ordering::Release);
        NEWDESK_ABI.store(Abi::Lp64 as u8, Ordering::Release);
        NEWDESK_OWNER.store(owner as i32, Ordering::Release);
        NEWDESK_ROOT.store(root, Ordering::Release);
        NEWDESK_TREE.store(tree, Ordering::Release);
    }

    /// The composite of the whole screen — what `paint_bg_clip` asks
    /// [`draw_desk_tree`] for per piece of the desktop's rectangle list.
    fn composite(log: &PaintLog) -> Vec<Paint> {
        draw_desk_tree(log, ClipRect::full(640, 480));
        log.take()
    }

    /// Both locks: the resolver seam and the `NEWDESK_*` statics.
    fn locks() -> (std::sync::MutexGuard<'static, ()>, std::sync::MutexGuard<'static, ()>) {
        (STATE.lock().unwrap_or_else(|e| e.into_inner()), SEAM.lock().unwrap_or_else(|e| e.into_inner()))
    }

    #[test]
    fn the_tree_is_read_through_the_owners_mapping_not_the_current_one() {
        let _g = locks();
        // Slot 3's page at VA: a root box (10,20,100,50) in colour 1.
        let mut owner = page_with(b"", 0);
        put_objects(&mut owner, 0, &[object(G_BOX, -1, -1, -1, solid(1), (10, 20, 100, 50))]);
        // Slot 5's page at the same VA: a full-screen box in colour 2.
        let mut other = page_with(b"", 0);
        put_objects(&mut other, 0, &[object(G_BOX, -1, -1, -1, solid(2), (0, 0, 640, 480))]);
        let maps = [
            Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 },
            Map { slot: 5, va: VA, len: PAGE, host: other.as_ptr() as u64 },
        ];
        register(VA, 0, 3);
        let log = PaintLog::new();
        let (paint, asked) = with_maps(&maps, || (composite(&log), ASKED.lock().unwrap().clone()));
        clear_newdesk();
        assert_eq!(paint, vec![Paint::Rect(10, 20, 100, 50, 1)], "the composite must draw the owner slot's tree");
        assert!(!asked.is_empty() && asked.iter().all(|&s| s == 3), "resolved through slots {asked:?}, not the owner's 3 only");
    }

    #[test]
    fn a_tree_the_owner_no_longer_maps_is_not_composited() {
        let _g = locks();
        // Slot 3 maps something else entirely; the registered VA is gone
        // (the block freed, the slot re-seeded for another program).
        let owner = page_with(b"", 0);
        let maps = [Map { slot: 3, va: VA + 8 * PAGE, len: PAGE, host: owner.as_ptr() as u64 }];
        register(VA, 0, 3);
        let log = PaintLog::new();
        let (paint, asked) = with_maps(&maps, || (composite(&log), ASKED.lock().unwrap().clone()));
        clear_newdesk();
        // The AES fallback root: one patterned fill of the whole clip in the
        // built-in DESKTOP root's colour (`fallback_root`), and nothing else.
        let fb_col = (super::super::init::desktop_root_obspec() & 0x0F) as Pel;
        assert_eq!(paint, vec![Paint::Pattern(0, 0, 640, 480, fb_col)], "an unresolvable tree counts as not registered: the fallback root is drawn");
        assert_eq!(asked, vec![3], "one failed lookup of the root record, then no further access");
    }

    #[test]
    fn the_trees_icon_is_read_through_the_owners_mapping() {
        let _g = locks();
        let mut owner = page_with(b"", 0);
        // Root box (0,0,640,480) with one G_ICON child at (100,100,64,48)
        // whose ICONBLK sits at +0x100, mask at +0x200, data at +0x300 and
        // label at +0x400 — all VAs of slot 3.
        put_objects(
            &mut owner,
            0,
            &[
                object(G_BOX, -1, 1, 1, solid(0), (0, 0, 640, 480)),
                object(G_ICON, 0, -1, -1, VA + 0x100, (100, 100, 64, 48)),
            ],
        );
        put_iconblk(&mut owner, 0x100, VA + 0x200, VA + 0x300, VA + 0x400, [0x1000, 0, 0, 0, 0, 32, 32, 0, 32, 64, 8]);
        put_raster(&mut owner, 0x200);
        put_raster(&mut owner, 0x300);
        owner[0x400..0x405].copy_from_slice(b"ICON\0");
        let maps = [Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 }];
        register(VA, 0, 3);
        let log = PaintLog::new();
        let (paint, asked) = with_maps(&maps, || (composite(&log), ASKED.lock().unwrap().clone()));
        clear_newdesk();
        // `gr_gicon`: the mask in bg (0), the data in fg (1), the label box,
        // then the four label glyphs.
        let mono: Vec<Paint> = paint.iter().copied().filter(|p| matches!(p, Paint::Mono(..))).collect();
        assert_eq!(
            mono,
            vec![Paint::Mono(100, 100, 32, 32, 0), Paint::Mono(100, 100, 32, 32, 1)],
            "mask and data rasters must be blitted from the owner's pages: {paint:?}"
        );
        let glyphs = paint.iter().filter(|p| matches!(p, Paint::Glyph(..))).count();
        assert_eq!(glyphs, 4, "the label ICON must be read from the owner's page: {paint:?}");
        assert!(asked.iter().all(|&s| s == 3), "resolved through slots {asked:?}");
    }

    #[test]
    fn a_raster_the_owner_does_not_map_is_skipped_not_read() {
        let _g = locks();
        let mut owner = page_with(b"", 0);
        put_objects(
            &mut owner,
            0,
            &[
                object(G_BOX, -1, 1, 1, solid(0), (0, 0, 640, 480)),
                object(G_ICON, 0, -1, -1, VA + 0x100, (100, 100, 64, 48)),
            ],
        );
        // The data raster points into a page the owner never had.
        put_iconblk(&mut owner, 0x100, VA + 0x200, VA + 9 * PAGE, VA + 0x400, [0x1000, 0, 0, 0, 0, 32, 32, 0, 32, 64, 8]);
        put_raster(&mut owner, 0x200);
        owner[0x400..0x405].copy_from_slice(b"ICON\0");
        let maps = [Map { slot: 3, va: VA, len: PAGE, host: owner.as_ptr() as u64 }];
        register(VA, 0, 3);
        let log = PaintLog::new();
        let paint = with_maps(&maps, || composite(&log));
        clear_newdesk();
        let mono: Vec<Paint> = paint.iter().copied().filter(|p| matches!(p, Paint::Mono(..))).collect();
        assert_eq!(mono, vec![Paint::Mono(100, 100, 32, 32, 0)], "only the mapped mask is blitted; the unmapped data raster is skipped: {paint:?}");
        assert_eq!(paint.iter().filter(|p| matches!(p, Paint::Glyph(..))).count(), 4, "the label still draws");
    }

    #[test]
    fn a_raster_spanning_two_pages_needs_one_contiguous_alias() {
        let _g = locks();
        // Tree + ICONBLK in page A; the 128-byte data raster starts 64
        // bytes before the end of page B and runs into page C. Mapped once
        // with C's alias two pages above B's (a hole between: not one block
        // → skipped), once with C's alias directly following B's (one
        // block → drawn). One 3-page host buffer serves both, so the
        // aliases are what the map says and not what the allocator did.
        let mut a = page_with(b"", 0);
        put_objects(
            &mut a,
            0,
            &[
                object(G_BOX, -1, 1, 1, solid(0), (0, 0, 640, 480)),
                object(G_ICON, 0, -1, -1, VA + 0x100, (100, 100, 64, 48)),
            ],
        );
        let raster_va = VA + 2 * PAGE - 64;
        put_iconblk(&mut a, 0x100, 0, raster_va, 0, [0x1000, 0, 0, 0, 0, 32, 32, 0, 32, 64, 8]);
        let buf = vec![0xFFu8; 3 * PAGE as usize].into_boxed_slice();
        let host = buf.as_ptr() as u64;
        let split = [
            Map { slot: 3, va: VA, len: PAGE, host: a.as_ptr() as u64 },
            Map { slot: 3, va: VA + PAGE, len: PAGE, host },
            Map { slot: 3, va: VA + 2 * PAGE, len: PAGE, host: host + 2 * PAGE },
        ];
        let joined = [
            Map { slot: 3, va: VA, len: PAGE, host: a.as_ptr() as u64 },
            Map { slot: 3, va: VA + PAGE, len: 2 * PAGE, host },
        ];
        register(VA, 0, 3);
        let log = PaintLog::new();
        let split_paint = with_maps(&split, || composite(&log));
        let joined_paint = with_maps(&joined, || composite(&log));
        clear_newdesk();
        assert!(
            !split_paint.iter().any(|p| matches!(p, Paint::Mono(..))),
            "an alias with a hole is not one contiguous run: the raster must be skipped: {split_paint:?}"
        );
        assert!(
            joined_paint.iter().any(|p| matches!(p, Paint::Mono(100, 100, 32, 32, 1))),
            "one block spanning both pages is read whole: {joined_paint:?}"
        );
    }
}
