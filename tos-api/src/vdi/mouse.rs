//! VDI mouse cursor, button/position query, and exchange vectors. Spec:
//! `docs/api/vdi/input/_mouse.md`.
//!
//! The cursor **sprite** machinery (form, hide-count, save/restore draw)
//! is complete here. The cursor **position** and **button** state come
//! from the input subsystem (the IKBD mouse packets), which is not yet
//! wired; per the spec this means a *parked* cursor and zero buttons —
//! a documented limitation, not fabricated motion. When the mouse driver
//! lands it calls [`set_state`] and the same sprite code animates.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI16, AtomicU16, AtomicU64, AtomicU8, AtomicUsize, Ordering};

use super::backend::{ClipRect, RasterBackend};
use crate::bdos::process::hooks::{self, Cell, Hook, Via};
use super::vwk::validate_color_index;
use super::Vdipb;

/// ms per timer-C tick (matches BIOS `Tickcal`) — what `vex_timv` reports and
/// the AES takes as `gl_ticktime` (`aes/geminit.c:704`).
pub(crate) const TICKCAL_MS: i16 = 20;

// ── live mouse state (fed by the input subsystem when present) ──
static GCURX: AtomicI16 = AtomicI16::new(0);
static GCURY: AtomicI16 = AtomicI16::new(0);
static MOUSE_BT: AtomicU8 = AtomicU8::new(0);
/// EmuTOS `cur_ms_stat` (`vdi_defs.h:294`): the mouse status bitmap since the
/// last event, maintained by [`feed_relative`] exactly as `mouse_int`
/// (`vdi_asm.S:240-328`) and consumed by `gloc_key` (the locator input,
/// `vdi_mouse.c:195-231`). Bit 0/1 = current left/right button state, bit 5 =
/// moved, bit 6/7 = left/right button-change flag. Only `v_locator` reads it;
/// the desktop/AES mouse path (`evnt_multi` → `mouse_buttons`) never does, so
/// maintaining it does not perturb the normal cursor/desktop behaviour.
static CUR_MS_STAT: AtomicU8 = AtomicU8::new(0);
/// Cursor hide depth: 0 = shown, ≥1 = hidden. GEM powers up hidden.
static HIDE_COUNT: AtomicI16 = AtomicI16::new(1);
static INIT_DONE: AtomicU8 = AtomicU8::new(0);

// ── cursor form (MFORM): hotspot, colours, 16 mask + 16 data rows ──
static HOT_X: AtomicI16 = AtomicI16::new(0);
static HOT_Y: AtomicI16 = AtomicI16::new(0);
static BG_COL: AtomicU8 = AtomicU8::new(0); // mask colour (white)
static FG_COL: AtomicU8 = AtomicU8::new(1); // data colour (black)
static MASK: [AtomicU16; 16] = [const { AtomicU16::new(0) }; 16];
static DATA: [AtomicU16; 16] = [const { AtomicU16::new(0) }; 16];

// ── AES-tracked cursor forms (EmuTOS gl_prevmouse / rlr->p_mouse) ──
//
// `graf_mouse(M_SAVE/M_RESTORE/M_PREVIOUS)` (EmuTOS `gemgrlib.c:439-451`) must
// snapshot and re-install cursor forms. The *current* form is the live sprite
// state (`HOT_*`/`*_COL`/`MASK`/`DATA` above, = EmuTOS `gl_mouse`).
// [`PREV_FORM`] mirrors `gl_prevmouse` — the form installed immediately before
// the current one, updated by every [`install_form`] (the `gsx_mfset`
// equivalent, `gemgsxif.c:407-420`). [`SAVED_FORM`] mirrors `rlr->p_mouse`;
// single-tasking makes one slot the 1:1 equivalent of the per-process field
// (the per-process `p_mouse` belongs to the cooperative kernel, Phase 2b).

/// A cursor form snapshot (`MFORM`): hotspot, mask/data colours, and the 16
/// mask + 16 data rows.
#[derive(Clone, Copy)]
struct MForm {
    hot_x: i16,
    hot_y: i16,
    bg_col: u8,
    fg_col: u8,
    mask: [u16; 16],
    data: [u16; 16],
}

/// The default arrow form (mask=white / data=black): the power-up current form,
/// and the initial value of both tracked slots so a stray `M_RESTORE` /
/// `M_PREVIOUS` before any save/set installs the arrow, never a blank sprite.
const ARROW_FORM: MForm = MForm {
    hot_x: PRESET_FORMS[0].0,
    hot_y: PRESET_FORMS[0].1,
    bg_col: 0,
    fg_col: 1,
    mask: PRESET_FORMS[0].2,
    data: PRESET_FORMS[0].3,
};

/// One `MFORM` slot, touched only from the single-CPU trap path (like [`Backing`]).
struct FormSlot {
    form: UnsafeCell<MForm>,
}
// SAFETY: touched only from the single-CPU, IRQ-off VDI trap path.
unsafe impl Sync for FormSlot {}

/// EmuTOS `gl_prevmouse` — the form installed before the current one.
static PREV_FORM: FormSlot = FormSlot { form: UnsafeCell::new(ARROW_FORM) };
/// EmuTOS `rlr->p_mouse` — the `M_SAVE` snapshot (one slot; single-tasking).
static SAVED_FORM: FormSlot = FormSlot { form: UnsafeCell::new(ARROW_FORM) };

// ── exchange-vector slots (function pointers) ──
static USER_BUT: AtomicU64 = AtomicU64::new(0);
static USER_MOT: AtomicU64 = AtomicU64::new(0);
static USER_CUR: AtomicU64 = AtomicU64::new(0);
static USER_WHEEL: AtomicU64 = AtomicU64::new(0);
static TIM_ADDR: AtomicU64 = AtomicU64::new(0);

// ── 16×16 backing store for the pixels under a shown cursor ──
//
// Holds the raw device pixel words (XRGB8888) saved by `draw`, restored
// verbatim by `erase` — the bit-exact save-under EmuTOS' `cur_display`/
// `cur_replace` perform on the screen words (AUD-V6-001). A PEL index would
// lose any colour that is not a palette entry (Falcon truecolor content).
struct Backing {
    px: UnsafeCell<[u32; 256]>,
    ox: AtomicI16,
    oy: AtomicI16,
    valid: AtomicU8,
}
// SAFETY: touched only from the single-CPU, IRQ-off VDI trap path.
unsafe impl Sync for Backing {}
static BACK: Backing = Backing {
    px: UnsafeCell::new([0; 256]),
    ox: AtomicI16::new(0),
    oy: AtomicI16::new(0),
    valid: AtomicU8::new(0),
};

/// The eight AES preset cursor forms, `(xhot, yhot, mask[16], data[16])`,
/// extracted verbatim from EmuTOS `aes/mform.rsc`. The index is the
/// `graf_mouse` / VDI shape number: ARROW 0, TEXT_CRSR 1, HOURGLASS 2,
/// POINT_HAND 3, FLAT_HAND 4, THIN_CROSS 5, THICK_CROSS 6, OUTLN_CROSS 7.
/// 16 rows, bit 0x8000 = leftmost pixel; `data` is the black shape, `mask`
/// the white outline. ARROW (0) is also the default installed on first use.
pub const PRESET_FORMS: [(i16, i16, [u16; 16], [u16; 16]); 8] = [
    // ARROW
    (0, 0,
     [0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00, 0xFF80, 0xFFC0, 0xFFE0, 0xFE00, 0xEF00, 0xCF00, 0x8780, 0x0780, 0x0380],
     [0x0000, 0x4000, 0x6000, 0x7000, 0x7800, 0x7C00, 0x7E00, 0x7F00, 0x7F80, 0x7C00, 0x6C00, 0x4600, 0x0600, 0x0300, 0x0300, 0x0000]),
    // TEXT_CRSR (I-beam)
    (7, 7,
     [0x1FF8, 0x1FF8, 0x1FF8, 0x07E0, 0x03C0, 0x03C0, 0x03C0, 0x03C0, 0x03C0, 0x03C0, 0x03C0, 0x03C0, 0x07E0, 0x1FF8, 0x1FF8, 0x1FF8],
     [0x0000, 0x0E70, 0x03C0, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x0180, 0x03C0, 0x0E70, 0x0000]),
    // HOURGLASS (busy)
    (8, 8,
     [0xFFFF, 0xFFFF, 0xFFFF, 0x7FFE, 0x7FFE, 0x3FFC, 0x1FF8, 0x0FF0, 0x0FF0, 0x1FF8, 0x3FFC, 0x7FFE, 0x7FFE, 0xFFFF, 0xFFFF, 0xFFFF],
     [0x0000, 0x7FFE, 0x2004, 0x1008, 0x1448, 0x0AB0, 0x0560, 0x02C0, 0x0340, 0x04A0, 0x0910, 0x1088, 0x12A8, 0x3554, 0x7FFE, 0x0000]),
    // POINT_HAND (pointing finger)
    (0, 0,
     [0x3000, 0x7800, 0x7C00, 0x3E07, 0x1F0F, 0x0F9E, 0x07DE, 0x07FE, 0x1FFE, 0x3FFF, 0x7FFF, 0x7FFE, 0x3FFE, 0x1FFE, 0x0FFF, 0x01FF],
     [0x3000, 0x4800, 0x4400, 0x2207, 0x1109, 0x0892, 0x0452, 0x0632, 0x1912, 0x2481, 0x5241, 0x4982, 0x2602, 0x1802, 0x0E01, 0x0180]),
    // FLAT_HAND (open hand)
    (8, 8,
     [0x0000, 0x0180, 0x0FF0, 0x1FFE, 0x1FFF, 0x1FFF, 0x1FFF, 0xFFFF, 0xFFFF, 0xFFFF, 0x7FFF, 0x3FFF, 0x1FFE, 0x0FFC, 0x0FFC, 0x1FFE],
     [0x0000, 0x0D80, 0x1270, 0x124E, 0x1249, 0x1249, 0x1249, 0xF249, 0x9001, 0x9801, 0x4401, 0x4001, 0x3002, 0x0804, 0x0804, 0x1FFE]),
    // THIN_CROSS
    (7, 7,
     [0x0000, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380, 0x7FFC, 0x7FFC, 0x7FFC, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380],
     [0x0000, 0x0000, 0x0100, 0x0100, 0x0100, 0x0100, 0x0100, 0x0100, 0x3FF8, 0x0100, 0x0100, 0x0100, 0x0100, 0x0100, 0x0100, 0x0000]),
    // THICK_CROSS
    (8, 8,
     [0x0000, 0x07C0, 0x07C0, 0x07C0, 0x07C0, 0x07C0, 0xFFFE, 0xFFFE, 0xFFFE, 0xFFFE, 0xFFFE, 0x07C0, 0x07C0, 0x07C0, 0x07C0, 0x07C0],
     [0x0000, 0x0000, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380, 0x7FFC, 0x7FFC, 0x7FFC, 0x0380, 0x0380, 0x0380, 0x0380, 0x0380, 0x0000]),
    // OUTLN_CROSS
    (8, 8,
     [0x07E0, 0x07E0, 0x0660, 0x0660, 0x0660, 0xFE7F, 0xFE7F, 0xC003, 0xC003, 0xFE7F, 0xFE7F, 0x0660, 0x0660, 0x0660, 0x07E0, 0x07E0],
     [0x0000, 0x03C0, 0x0240, 0x0240, 0x0240, 0x0240, 0x7E7E, 0x4002, 0x4002, 0x7E7E, 0x0240, 0x0240, 0x0240, 0x0240, 0x03C0, 0x0000]),
];

/// Update the mouse position + button state from the input subsystem.
/// Redraws the cursor at the new position if it is currently shown.
///
/// The integration point for the IKBD mouse driver (relative-mouse
/// packets); unused until that driver is wired — see the module note.
#[allow(dead_code)]
pub fn set_state(x: i16, y: i16, buttons: u8, backend: &dyn RasterBackend) {
    if shown() {
        erase(backend);
    }
    GCURX.store(x, Ordering::Relaxed);
    GCURY.store(y, Ordering::Relaxed);
    MOUSE_BT.store(buttons, Ordering::Relaxed);
    if shown() {
        draw(backend);
    }
}

/// Feed a relative-mouse event from the input subsystem (the kernel's
/// IKBD mouse-packet decoder). `dx`/`dy` are signed pixel deltas;
/// `buttons` is already in VDI convention (bit 0 = left, bit 1 = right).
/// Accumulates onto the current cursor position, clamps it to the
/// screen, and updates the cursor + the `vq_mouse`/AES-event state.
///
/// This is the API-level half of the mouse driver: it owns the cursor
/// semantics. The hardware packet decode lives in the kernel — see
/// `docs/api/vdi/input/_mouse-packet.md`.
pub fn feed_relative(dx: i16, dy: i16, buttons: u8, backend: &dyn RasterBackend) {
    ensure_init(backend);
    // Maintain `cur_ms_stat` for the locator input device (`v_locator` /
    // `gloc_key`), byte-for-byte as EmuTOS `mouse_int` (`vdi_asm.S:240-328`):
    // the raw deltas decide the motion bit, the button transition sets the
    // change bits. This touches only `cur_ms_stat`; the cursor draw, `MOUSE_BT`
    // and `GCURX`/`GCURY` (via `set_state` below) are unchanged, so the normal
    // cursor/desktop path stays byte-identical.
    update_ms_stat(dx, dy, buttons);
    // "keep additional mouse button states" (`vdi/vdi_asm.S:270-277`): the
    // packet carries only buttons 1/2, so `mouse_int` merges its two bits into
    // `MOUSE_BT` and leaves bits 2..4 — the extended buttons 3/4/5 that
    // [`mousex_handler`] maintains — standing (`andi.w #0xfffc,d2 / or.w
    // d2,d0`). The merged value is what it stores *and* what it hands to
    // `user_but` (`:278-281`).
    let merged = (MOUSE_BT.load(Ordering::Relaxed) & !0x03) | (buttons & 0x03);
    // `user_but` — the AES's button-change vector, installed with
    // `gsx_setmb(far_bcha, …)` (`aes/gemgsxif.c:271`) and called by the VDI
    // mouse interrupt on a change of the low two button bits, before the
    // position is touched (`vdi/vdi_asm.S:261-284`). `b_click` itself filters
    // out non-changes, as `mouse_int` does.
    crate::aes::user_but(merged);
    let max_x = backend.width() as i16 - 1;
    let max_y = backend.height() as i16 - 1;
    let (cx, cy) = position();
    let nx = (cx + dx).clamp(0, max_x);
    let ny = (cy + dy).clamp(0, max_y);
    // `user_mot` — the AES's motion vector (`far_mcha` → `mchange`), called
    // only for a packet that carried movement and with the **clipped** new
    // position (`vdi/vdi_asm.S:305-362`).
    if dx != 0 || dy != 0 {
        crate::aes::user_mot(nx, ny);
    }
    set_state(nx, ny, merged, backend);
}

/// Update [`CUR_MS_STAT`] from one relative-mouse event, reproducing EmuTOS
/// `mouse_int` (`vdi_asm.S:240-328`) bit-for-bit. On a button-state change the
/// byte is rewritten to the new button bits (0/1) with the changed buttons
/// flagged in bits 6 (left) / 7 (right) — the `eor`/`ror.b #2`/`or` sequence —
/// which also clears the motion bit; with no button change the byte keeps its
/// button/change bits. The motion bit (5) is then set if the packet carried
/// movement (raw `dx`/`dy`, before clipping), else cleared. `buttons` is in VDI
/// convention (bit 0 = left, bit 1 = right); only bits 0/1 are used, so EmuTOS'
/// "keep the extended-button bits in MOUSE_BT" reduces to a no-op here.
fn update_ms_stat(dx: i16, dy: i16, buttons: u8) {
    let new_btn = buttons & 0x03;
    let moved = dx != 0 || dy != 0;
    // Atomic RMW: the feed can race a `gloc_key` consume if it runs from an
    // IRQ during a `pump_input`.
    let _ = CUR_MS_STAT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |prev| {
        let old_btn = prev & 0x03;
        let mut stat = if new_btn != old_btn {
            let delta = (old_btn ^ new_btn) & 0x03;
            new_btn | delta.rotate_right(2)
        } else {
            prev
        };
        if moved {
            stat |= 0x20;
        } else {
            stat &= !0x20;
        }
        Some(stat)
    });
}

/// The current cursor position (for the locator input device).
pub fn position() -> (i16, i16) {
    (GCURX.load(Ordering::Relaxed), GCURY.load(Ordering::Relaxed))
}

/// The current cursor position, materialising the parked start position
/// (screen centre) on first use. `graf_mouse(M_ON)` shows the cursor "where
/// it is"; before any mouse motion that must be the initialised park
/// position, never the power-up `(0, 0)`. Reading the raw [`position`]
/// before [`ensure_init`] yields `(0, 0)`, so the very first `M_ON` would
/// pin the cursor over the menu bar (opening an AES hover-menu at boot).
/// Idempotent — `ensure_init` runs at most once.
pub fn position_init(backend: &dyn RasterBackend) -> (i16, i16) {
    ensure_init(backend);
    position()
}

/// The current button bitmask (for the locator/choice input devices).
#[allow(dead_code)]
pub fn buttons() -> u8 {
    MOUSE_BT.load(Ordering::Relaxed)
}

/// EmuTOS `gloc_key` button branch (`vdi_mouse.c:203-209`): if a mouse-button
/// change is pending (bit 6 or 7 of `cur_ms_stat`), consume it and return the
/// terminator code — 0x20 for the left button (bit 6), 0x21 for the right
/// (bit 7), left taking precedence. Clears the change bits, keeping the button
/// and motion bits (`&= 0x23`). `None` when no button change is pending. Used
/// only by `v_locator`.
pub fn take_button_terminator() -> Option<i16> {
    // Test-and-clear bits 6/7 atomically (see `update_ms_stat` on the race);
    // masking to 0x23 keeps bits 0,1 (buttons) and 5 (motion). When no change
    // bit is set this matches EmuTOS leaving `cur_ms_stat` untouched, because
    // the reserved bits 2/3/4 are always zero here.
    let prev = CUR_MS_STAT.fetch_and(0x23, Ordering::Relaxed);
    if prev & 0xc0 != 0 {
        Some(if prev & 0x40 != 0 { 0x20 } else { 0x21 })
    } else {
        None
    }
}

/// EmuTOS `gloc_key` motion branch (`vdi_mouse.c:221-227`): if the motion bit
/// (5) of `cur_ms_stat` is set, consume it and report movement — the caller
/// reads the live [`position`] for the point. Used only by `v_locator`.
pub fn take_motion() -> bool {
    (CUR_MS_STAT.fetch_and(!0x20, Ordering::Relaxed) & 0x20) != 0
}

/// EmuTOS `vdi_v_locator` entry (`vdi_mouse.c:286-287`): set the locator / VDI
/// cursor position `GCURX`/`GCURY` from the input point, unconditionally in
/// both request and sample modes. This is the logical position `dis_cur` draws
/// at and `gloc_key` / `PTSOUT` report; the request-mode `dis_cur`/`hide_cur`
/// bracket performs the drawing, so this does not itself erase/redraw a shown
/// sprite (the next `erase` still restores from the last drawn origin).
pub fn set_locator_pos(x: i16, y: i16) {
    GCURX.store(x, Ordering::Relaxed);
    GCURY.store(y, Ordering::Relaxed);
}

fn shown() -> bool {
    HIDE_COUNT.load(Ordering::Relaxed) <= 0
}

/// The cursor's park position for a `w` x `h` screen — EmuTOS `vdimouse_init`
/// (`vdi/vdi_mouse.c:582-583`):
///
/// ```c
/// GCURX = xres / 2;           /* initialize the mouse to center */
/// GCURY = yres / 2;
/// ```
///
/// `xres`/`yres` are *not* the screen dimensions. `vdi/vdi_defs.h:94-95`
/// aliases them onto `DEV_TAB[0]`/`DEV_TAB[1]`, and `update_rez_dependent`
/// fills those with `V_REZ_HZ - 1` / `V_REZ_VT - 1`
/// (`vdi/vdi_control.c:203-204`) — the last addressable pixel coordinate, one
/// less than the pixel count. So the centre is the midpoint of the coordinate
/// *range*, and at every even resolution that is one pixel up and left of the
/// midpoint of the pixel *count*: at 640x480, 319/239 and not 320/240 (V17).
pub fn park_position(w: u16, h: u16) -> (i16, i16) {
    // `xres` / `yres` in EmuTOS' sense: the last coordinate, DEV_TAB[0]/[1].
    let xres = w as i16 - 1;
    let yres = h as i16 - 1;
    (xres / 2, yres / 2)
}

fn ensure_init(backend: &dyn RasterBackend) {
    if INIT_DONE.swap(1, Ordering::Relaxed) != 0 {
        return;
    }
    let (_, _, mask, data) = &PRESET_FORMS[0]; // ARROW
    for i in 0..16 {
        MASK[i].store(mask[i], Ordering::Relaxed);
        DATA[i].store(data[i], Ordering::Relaxed);
    }
    // Park the cursor at the screen centre.
    let (cx, cy) = park_position(backend.width(), backend.height());
    GCURX.store(cx, Ordering::Relaxed);
    GCURY.store(cy, Ordering::Relaxed);
    // Install the default exchange-vector slots (EmuTOS `vdimouse_init`,
    // `vdi_mouse.c:585-589`; the `vex_timv` slot `tim_addr`, `vdi_misc.c:128`):
    // the button / motion / wheel / timer vectors default to the `just_rts`
    // no-op, the cursor-draw vector to `mov_cur`. NewTOS never *invokes* these
    // from the cooperative feed path — the default behaviour is already inlined
    // (`feed_relative` redraws the cursor; buttons/motion are no-ops) — but the
    // slots must hold a real address so an app that installs its own vector via
    // `vex_*` gets the genuine previous address back to save/restore, instead
    // of a spurious NULL (AUD-V6-002).
    // Cast the fn items through a raw pointer (not a direct fn→int cast) to
    // take their address as a `u64` sentinel.
    USER_BUT.store(just_rts as *const () as u64, Ordering::Relaxed);
    USER_MOT.store(just_rts as *const () as u64, Ordering::Relaxed);
    USER_CUR.store(mov_cur as *const () as u64, Ordering::Relaxed);
    TIM_ADDR.store(just_rts as *const () as u64, Ordering::Relaxed);
    // `user_wheel = just_rts` (`vdi_mouse.c:588`) and `mousexvec =
    // vdi_mousex_handler` (`:612`) — the wheel half of `vdimouse_init`, split
    // out into [`wheel_init`] because EmuTOS runs `vdimouse_init` (workstation
    // open) strictly before `gsx_init`'s `vex_wheelv(aes_wheel, …)`
    // (`aes/gemgsxif.c:291`), while NewTOS' `ensure_init` is lazy — the first
    // cursor operation — and can therefore run *after* the AES has installed
    // its handler. [`wheel_init`] is one-shot, so whichever runs first seeds
    // the slot and the other leaves it alone.
    wheel_init();
}

/// EmuTOS `just_rts` (`vdi_asm.S`): the default no-op mouse vector shared by
/// the button / motion / wheel / timer slots. For the button / motion /
/// cursor / timer slots only its **address** matters — it is the sentinel the
/// `vex_*` swap returns so an app can save and restore the default; those
/// four are never called from NewTOS' cooperative feed path (AUD-V6-002).
/// The **wheel** slot is different since GUI04: [`call_user_wheel`] really
/// dispatches it, and `just_rts` is the address it recognises as "no handler,
/// do nothing" — the behavioural equivalent of jumping to an `rts`.
fn just_rts() {}

/// EmuTOS `mov_cur` (`vdi_asm.S:496`): the default cursor-draw vector. EmuTOS'
/// `mouse_int` calls it to redraw the sprite after a move; NewTOS redraws the
/// cursor inline in [`feed_relative`], so this is a pure address sentinel for
/// the `vex_curv` save/restore contract — never invoked.
fn mov_cur() {}

// ── extended mouse: buttons 3-5 and the wheel ──────────────────────────────
//
// `CONF_WITH_EXTENDED_MOUSE` is `1` in the reference build
// (`include/config.h:1491-1498`). The chain, end to end:
//
//   IKBD scancode 0x37/0x59/0x5a/0x5c/0x5d/0x5e/0x5f
//     → `kbd_int` → `mousexvec(scancode)`          (`bios/ikbd.c:746-760`)
//     → `vdi_mousex_handler`                       (`vdi/vdi_mouse.c:525-553`)
//         · buttons 3/4/5 → `MOUSE_BT` bits 2..4 + `call_user_but(MOUSE_BT)`
//         · wheel         → `call_user_wheel(number, amount)`
//     → `user_wheel`, the vector `vex_wheelv` installs (`:438-441`)
//     → the AES's `aes_wheel` → `wheel_change`     (`aes/geminput.c:395-420`)
//
// NewTOS' source is a **USB** mouse, not an ACIA IKBD, but it enters at the
// same point: `kernel/src/drivers/usb/hid.rs` already translates every boot
// report into IKBD bytes, and the wheel byte becomes the very same scancodes.

/// [`USER_WHEEL`] holds `just_rts` — no handler, do nothing. EmuTOS
/// `vdimouse_init`'s `user_wheel = just_rts` (`vdi/vdi_mouse.c:588`).
const WHEEL_KIND_NONE: u8 = 0;
/// [`USER_WHEEL`] holds the AES's own handler, EmuTOS `aes_wheel`
/// (`aes/gemdosif.S:353`), installed by `gsx_init`'s `vex_wheelv(aes_wheel,
/// &old_wheelv)` (`aes/gemgsxif.c:291`). The AES is EL1 code here, so the
/// call is a plain same-privilege call.
const WHEEL_KIND_AES: u8 = 1;
/// [`USER_WHEEL`] holds a routine **inside an EL0 application**, installed by
/// that application's own `vex_wheelv`. [`USER_WHEEL_SLOT`] names the process
/// whose address space the VA belongs to; the call is an EL1→EL0 upcall.
const WHEEL_KIND_EL0: u8 = 2;

/// Which of the three kinds [`USER_WHEEL`] currently holds.
static USER_WHEEL_KIND: AtomicU8 = AtomicU8::new(WHEEL_KIND_NONE);
/// The process slot owning a [`WHEEL_KIND_EL0`] vector; `usize::MAX` = none.
static USER_WHEEL_SLOT: AtomicUsize = AtomicUsize::new(usize::MAX);
/// One-shot guard for [`wheel_init`] (see the call in [`ensure_init`]).
static WHEEL_INIT_DONE: AtomicU8 = AtomicU8::new(0);

/// EmuTOS `aes_wheel` (`aes/gemdosif.S:353-374`) as an **address token**: the
/// value `vex_wheelv` hands back as `old` while the AES owns the vector, and
/// the value an application writes back to restore it. It is never jumped to
/// — [`call_user_wheel`] dispatches on [`USER_WHEEL_KIND`] and calls
/// [`crate::aes::aes_wheel`] directly — because an EL0 application could not
/// call an EL1 address anyway. EmuTOS' documented "call the old routine too"
/// chaining (gemlib `vex_wheelv.c`) therefore stops at the privilege boundary:
/// an application that saves and later restores `old` gets the AES's wheel
/// handling back, but it cannot chain into it while its own is installed.
fn aes_wheel_token() {}

/// Address of the `just_rts` no-op, as stored in the vector slots.
fn just_rts_va() -> u64 {
    just_rts as *const () as u64
}

/// Address of the [`aes_wheel_token`] sentinel.
fn aes_wheel_va() -> u64 {
    aes_wheel_token as *const () as u64
}

/// The wheel half of `vdimouse_init` (`vdi/vdi_mouse.c:588,612`): seed
/// `user_wheel` with `just_rts` and install the VDI's extended-mouse handler
/// into the BIOS' `mousexvec`. One-shot — see the comment at its
/// [`ensure_init`] call site for why the ordering against the AES's
/// `gsx_init` must not matter.
fn wheel_init() {
    if WHEEL_INIT_DONE.swap(1, Ordering::Relaxed) != 0 {
        return;
    }
    USER_WHEEL.store(just_rts_va(), Ordering::Relaxed);
    USER_WHEEL_KIND.store(WHEEL_KIND_NONE, Ordering::Relaxed);
    crate::bios::vectors::set_mousexvec(mousex_handler);
}

/// Install the AES's wheel handler — EmuTOS `gsx_init`'s
/// `vex_wheelv(aes_wheel, &old_wheelv)` (`aes/gemgsxif.c:288-292`), called
/// from [`crate::aes::init::init`] (and its restart twin). EmuTOS guards that
/// call with `aestrap_intercepted()`, an NVDI-crash workaround for a
/// third-party VDI that does not understand opcode 134 (`:277-290`); NewTOS
/// *is* the VDI, so there is nothing to intercept and no guard to reproduce.
///
/// Idempotent, and it never overrides an application's own vector: it only
/// installs while the slot still holds the [`WHEEL_KIND_NONE`] default.
pub fn install_aes_wheel() {
    wheel_init();
    if USER_WHEEL_KIND.load(Ordering::Relaxed) != WHEEL_KIND_NONE {
        return;
    }
    USER_WHEEL.store(aes_wheel_va(), Ordering::Relaxed);
    USER_WHEEL_KIND.store(WHEEL_KIND_AES, Ordering::Relaxed);
}

/// Drop a [`WHEEL_KIND_EL0`] vector owned by process slot `slot` and put the
/// AES's handler back. Called from `free_slot` when the process goes away.
///
/// **Why this exists and EmuTOS has no counterpart.** EmuTOS runs every
/// program in one flat address space, so a vector an exiting program forgot
/// to restore still points at (dead but mapped) memory. Here it points into a
/// torn-down EL0 address space: honouring it would deliver an upcall to a VA
/// that names something else, and *not* honouring it would leave the wheel
/// permanently dead for the desktop. The nearest EmuTOS anchor for "put the
/// default back when the owner of the vector is gone" is `vdimouse_exit`'s
/// `user_wheel = just_rts` (`vdi/vdi_mouse.c:628`); the default of a *running*
/// AES session is the AES's own handler, which is what we restore.
pub fn release_wheel_vector(slot: usize) {
    if USER_WHEEL_KIND.load(Ordering::Relaxed) != WHEEL_KIND_EL0
        || USER_WHEEL_SLOT.load(Ordering::Relaxed) != slot
    {
        return;
    }
    USER_WHEEL_SLOT.store(usize::MAX, Ordering::Relaxed);
    USER_WHEEL.store(aes_wheel_va(), Ordering::Relaxed);
    USER_WHEEL_KIND.store(WHEEL_KIND_AES, Ordering::Relaxed);
}

/// EmuTOS `vdi_mousex_handler` (`vdi/vdi_mouse.c:525-553`) — the
/// `mousexvec` handler the VDI installs. `scancode` is the **full** IKBD
/// scancode, release bit included, exactly as `kbd_int` passes it
/// (`bios/ikbd.c:758`).
///
/// Buttons 3/4/5 live in `MOUSE_BT` bits 2/3/4 and, on a change, go out
/// through the button vector (`call_user_but(MOUSE_BT)`, `:545`) — the same
/// vector a normal packet uses, so `vq_mouse` and the AES see one consistent
/// button mask. The four wheel codes go out through the wheel vector.
pub fn mousex_handler(scancode: u8) {
    let old_buttons = MOUSE_BT.load(Ordering::Relaxed);
    let buttons = match scancode {
        0x37 => old_buttons | 0x04,  // Mouse button 3 press
        0xB7 => old_buttons & !0x04, // Mouse button 3 release
        0x5E => old_buttons | 0x08,  // Mouse button 4 press
        0xDE => old_buttons & !0x08, // Mouse button 4 release
        0x5F => old_buttons | 0x10,  // Mouse button 5 press
        0xDF => old_buttons & !0x10, // Mouse button 5 release
        _ => old_buttons,
    };
    if buttons != old_buttons {
        MOUSE_BT.store(buttons, Ordering::Relaxed);
        crate::aes::user_but(buttons);
    }

    match scancode {
        0x59 => call_user_wheel(0, -1), // Wheel up
        0x5A => call_user_wheel(0, 1),  // Wheel down
        0x5C => call_user_wheel(1, -1), // Wheel left
        0x5D => call_user_wheel(1, 1),  // Wheel right
        _ => {}
    }
}

/// EmuTOS `call_user_wheel(wheel_number, wheel_amount)` (`vdi/vdi_asm.S:546-
/// 552`): invoke the installed `user_wheel` vector with the wheel number in
/// `d0.w` and the signed detent count in `d1.w` (gemlib `vex_wheelv.c`).
/// Wheel number 0 is the vertical wheel, 1 the horizontal one; the amount is
/// `-1` for up/left and `+1` for down/right.
///
/// The three kinds of vector are dispatched as [`USER_WHEEL_KIND`] says:
///
/// * [`WHEEL_KIND_NONE`] — `just_rts`, nothing happens.
/// * [`WHEEL_KIND_AES`] — a direct EL1 call into [`crate::aes::aes_wheel`].
///   EmuTOS' `aes_wheel` is the m68k glue that pushes the two WORDs and
///   `forkq`s `wheel_change`; here it is the function itself.
/// * [`WHEEL_KIND_EL0`] — an application routine. It is posted as an EL0
///   upcall (`docs/analysis/coop-kernel/10-upcall-signal-design.md`, consumer
///   **vex**) with `x0 = wheel_number`, `x1 = wheel_amount` per AAPCS64, and
///   delivered when that process next returns to EL0.
///
///   Two properties follow from the cooperative model and are deliberate, not
///   oversights: (a) the vector is only called while its **owning** process is
///   the running one (it is delivered as an upcall into that process; unlike
///   a `G_USERDEF`'s `ub_code`, which since ARCH03 3c E10 runs in whichever
///   process draws, `aes/object.rs` `ob_user`); (b) delivery happens at the next syscall return,
///   not inside an interrupt, and a second detent arriving before the first
///   was delivered replaces it ([`crate::bdos::process::post_upcall`]'s
///   last-writer-wins contract). A true ISR-timed call needs the lower-EL IRQ
///   route the design marks as the vex consumer's open precondition (§7.3),
///   which NewTOS does not have.
fn call_user_wheel(wheel_number: i16, wheel_amount: i16) {
    match USER_WHEEL_KIND.load(Ordering::Relaxed) {
        WHEEL_KIND_AES => crate::aes::aes_wheel(wheel_number, wheel_amount),
        WHEEL_KIND_EL0 => {
            let slot = USER_WHEEL_SLOT.load(Ordering::Relaxed);
            let current = crate::bdos::process::slot_for(crate::bdos::process::current());
            if current != Some(slot) {
                return;
            }
            crate::bdos::process::post_upcall(
                slot,
                crate::bdos::process::UpcallDesc {
                    handler_va: USER_WHEEL.load(Ordering::Relaxed),
                    args: [wheel_number as i64 as u64, wheel_amount as i64 as u64, 0, 0],
                    mask_bit: crate::bdos::process::UPCALL_MASK_VEX_WHEEL,
                    disposition: crate::bdos::process::Disposition::Resume,
                },
            );
        }
        _ => {}
    }
}

/// Initialise the cursor subsystem at workstation-open time (EmuTOS
/// `vdimouse_init`, `vdi_mouse.c:582-583`): load the default arrow form and
/// park the cursor at the screen centre. This materialises `GCURX`/`GCURY`
/// **before** the first cursor op, so `vq_mouse`/`v_locator` report the parked
/// centre rather than the power-up `(0, 0)`. Idempotent (guarded by
/// `INIT_DONE`), so it never clobbers a live cursor position moved by the HID
/// mouse.
pub fn init(backend: &dyn RasterBackend) {
    ensure_init(backend);
}

/// Install an arbitrary cursor form (16-row mask + data, hotspot), drawn
/// with the standard mask=white / data=black colours. Erases and redraws
/// the sprite if it is currently shown. Shared by the AES presets and
/// `graf_mouse`'s USER_DEF path.
pub fn set_form(hot_x: i16, hot_y: i16, mask: &[u16; 16], data: &[u16; 16], backend: &dyn RasterBackend) {
    // Standard mask=white / data=black colours (the AES presets and the
    // `graf_mouse(USER_DEF)` path). The install itself lives in [`install_form`]
    // (the `gsx_mfset` equivalent), which also records the outgoing form as the
    // "previous" one so `graf_mouse(M_PREVIOUS)` can revert to it.
    install_form(
        &MForm { hot_x, hot_y, bg_col: 0, fg_col: 1, mask: *mask, data: *data },
        backend,
    );
}

/// Install one of the eight preset cursor forms by its shape number
/// (0..=7); out-of-range numbers are ignored (EmuTOS `gsx_mfset`).
pub fn set_preset(n: i16, backend: &dyn RasterBackend) {
    if let Some((hx, hy, mask, data)) = PRESET_FORMS.get(n as usize) {
        set_form(*hx, *hy, mask, data, backend);
    }
}

/// Read the live sprite state into an [`MForm`] (EmuTOS `gl_mouse`).
fn current_form() -> MForm {
    let mut mask = [0u16; 16];
    let mut data = [0u16; 16];
    for i in 0..16 {
        mask[i] = MASK[i].load(Ordering::Relaxed);
        data[i] = DATA[i].load(Ordering::Relaxed);
    }
    MForm {
        hot_x: HOT_X.load(Ordering::Relaxed),
        hot_y: HOT_Y.load(Ordering::Relaxed),
        bg_col: BG_COL.load(Ordering::Relaxed),
        fg_col: FG_COL.load(Ordering::Relaxed),
        mask,
        data,
    }
}

/// Install `form` as the current cursor — the `gsx_mfset` equivalent (EmuTOS
/// `gemgsxif.c:407-420`). Record the outgoing form as [`PREV_FORM`]
/// (`gl_prevmouse = gl_mouse`), then set the new one, erasing and redrawing the
/// sprite if it is currently shown. The hotspot is clamped to the 16×16 grid,
/// exactly as [`set_form`] did.
fn install_form(form: &MForm, backend: &dyn RasterBackend) {
    ensure_init(backend);
    // EmuTOS `gsx_mfset`: gl_prevmouse = gl_mouse before installing the new form.
    // SAFETY: single trap path.
    unsafe { *PREV_FORM.form.get() = current_form() };
    let was_shown = shown();
    if was_shown {
        erase(backend);
    }
    HOT_X.store(form.hot_x.clamp(0, 15), Ordering::Relaxed);
    HOT_Y.store(form.hot_y.clamp(0, 15), Ordering::Relaxed);
    BG_COL.store(form.bg_col, Ordering::Relaxed);
    FG_COL.store(form.fg_col, Ordering::Relaxed);
    for i in 0..16 {
        MASK[i].store(form.mask[i], Ordering::Relaxed);
        DATA[i].store(form.data[i], Ordering::Relaxed);
    }
    if was_shown {
        draw(backend);
    }
}

/// EmuTOS `gr_mouse(M_SAVE)` (`gemgrlib.c:441`): snapshot the current cursor
/// form into the saved slot (`rlr->p_mouse`). No draw, no form change.
pub fn save_form() {
    // SAFETY: single trap path.
    unsafe { *SAVED_FORM.form.get() = current_form() };
}

/// EmuTOS `gr_mouse(M_RESTORE)` (`gemgrlib.c:443-445`): reinstall the form
/// stored by [`save_form`] (`rlr->p_mouse`).
pub fn restore_form(backend: &dyn RasterBackend) {
    // SAFETY: single trap path; `MForm` is `Copy`.
    let form = unsafe { *SAVED_FORM.form.get() };
    install_form(&form, backend);
}

/// EmuTOS `gr_mouse(M_PREVIOUS)` (`gemgrlib.c:446-450`): reinstall the form
/// installed immediately before the current one (`gl_prevmouse`). EmuTOS copies
/// it into a temporary first because `gsx_mfset` overwrites `gl_prevmouse`; we
/// read the slot into a local before [`install_form`] rewrites it, so the two
/// forms swap (current ↔ previous) exactly as EmuTOS does.
pub fn previous_form(backend: &dyn RasterBackend) {
    // SAFETY: single trap path; `MForm` is `Copy`.
    let form = unsafe { *PREV_FORM.form.get() };
    install_form(&form, backend);
}

fn full_clip(backend: &dyn RasterBackend) -> ClipRect {
    ClipRect::full(backend.width(), backend.height())
}

/// Save the 16×16 area under the cursor bit-exact, then draw the sprite.
///
/// EmuTOS `cur_display`/`cur_display16` save the raw screen words under the
/// sprite (`*save++ = *dst`, `vdi_mouse.c:983-985`) *before* overwriting
/// them. We do the chunky-per-pixel equivalent: store the exact device
/// pixel word via [`get_device_pixel`](RasterBackend::get_device_pixel),
/// **not** the nearest-PEL round-trip `read_pixel` gives — the latter
/// corrupts every pixel whose colour is not a palette entry (Falcon
/// truecolor content), AUD-V6-001. The sprite paint itself keeps the PEL
/// `fill_rect` path (mask → bg, data → fg), which is exact for the
/// two-colour sprite. Order (save → draw) is unchanged.
fn draw(backend: &dyn RasterBackend) {
    let ox = GCURX.load(Ordering::Relaxed) - HOT_X.load(Ordering::Relaxed);
    let oy = GCURY.load(Ordering::Relaxed) - HOT_Y.load(Ordering::Relaxed);
    let clip = full_clip(backend);
    let fg = FG_COL.load(Ordering::Relaxed);
    let bg = BG_COL.load(Ordering::Relaxed);
    // SAFETY: single trap path.
    let store = unsafe { &mut *BACK.px.get() };
    for dy in 0..16i16 {
        let mrow = MASK[dy as usize].load(Ordering::Relaxed);
        let drow = DATA[dy as usize].load(Ordering::Relaxed);
        for dx in 0..16i16 {
            let (px, py) = (ox + dx, oy + dy);
            store[(dy * 16 + dx) as usize] = backend.get_device_pixel(px, py);
            let bit = 0x8000u16 >> dx;
            if drow & bit != 0 {
                backend.fill_rect(px, py, 1, 1, fg, clip);
            } else if mrow & bit != 0 {
                backend.fill_rect(px, py, 1, 1, bg, clip);
            }
        }
    }
    BACK.ox.store(ox, Ordering::Relaxed);
    BACK.oy.store(oy, Ordering::Relaxed);
    BACK.valid.store(1, Ordering::Relaxed);
}

/// Restore the pixels saved under the cursor bit-exact.
///
/// EmuTOS `cur_replace`/`cur_replace16` write the saved screen words
/// straight back (`*dst = *src++`, `vdi_mouse.c:1099`/`:1046`). We do the
/// chunky-per-pixel equivalent: put each saved device pixel word back via
/// [`put_device_pixel`](RasterBackend::put_device_pixel) — restoring the
/// exact original colour, not the palette approximation `fill_rect` would
/// (AUD-V6-001). `put_device_pixel` does not flush on its own, so the whole
/// 16×16 box is flushed to the point of coherency once at the end. Cells
/// off the screen edge are ignored by the backend, matching the clipping in
/// `cur_replace`. Order (erase → restore) and the `BACK.valid` swap
/// (guarding against a double restore) are unchanged.
fn erase(backend: &dyn RasterBackend) {
    if BACK.valid.swap(0, Ordering::Relaxed) == 0 {
        return;
    }
    let ox = BACK.ox.load(Ordering::Relaxed);
    let oy = BACK.oy.load(Ordering::Relaxed);
    // SAFETY: single trap path.
    let store = unsafe { &*BACK.px.get() };
    for dy in 0..16i16 {
        for dx in 0..16i16 {
            backend.put_device_pixel(ox + dx, oy + dy, store[(dy * 16 + dx) as usize]);
        }
    }
    backend.flush_rect(ox, oy, ox + 15, oy + 15);
}

/// Opcode 111 — `vsc_form`: install the 37-WORD cursor definition from
/// `INTIN` (hotspot x/y, reserved, mask colour, data colour, then 16
/// mask rows and 16 data rows).
pub fn vsc_form(pb: &Vdipb, backend: &dyn RasterBackend) {
    ensure_init(backend);
    let was_shown = shown();
    if was_shown {
        erase(backend);
    }
    // Hotspot constrained to 0..15 by masking (EmuTOS `set_mouse_form`
    // `& 0x0f`): values above 15 wrap, they never overflow the 16×16 sprite.
    HOT_X.store(pb.intin(0) & 0x0f, Ordering::Relaxed);
    HOT_Y.store(pb.intin(1) & 0x0f, Ordering::Relaxed);
    // intin(2) = reserved (planes, must be 1) — ignored, like `set_mouse_form`.
    // Mask/data colour indices run through `validate_color_index`: an
    // out-of-range pen (< 0 or ≥ numcolors) silently becomes pen 1.
    let ncol = backend.num_colors();
    BG_COL.store(validate_color_index(pb.intin(3), ncol), Ordering::Relaxed);
    FG_COL.store(validate_color_index(pb.intin(4), ncol), Ordering::Relaxed);
    for i in 0..16 {
        MASK[i].store(pb.intin(5 + i) as u16, Ordering::Relaxed);
        DATA[i].store(pb.intin(21 + i) as u16, Ordering::Relaxed);
    }
    if was_shown {
        draw(backend);
    }
}

/// EmuTOS `dis_cur`: the counted-show core. Decrement the hide depth,
/// underflow-clamped to 0, and physically draw the cursor on the 1 → 0
/// transition (`vdi_mouse.c:127-141`).
fn dis_cur(backend: &dyn RasterBackend) {
    let prev = HIDE_COUNT.load(Ordering::Relaxed);
    let new = (prev - 1).max(0);
    HIDE_COUNT.store(new, Ordering::Relaxed);
    if prev > 0 && new == 0 {
        draw(backend);
    }
}

/// Opcode 122 — `v_show_c`: counted show. `INTIN[0] == 0` is a **force-show**
/// (EmuTOS `linea_show_mouse`, `vdi_mouse.c:1125-1131`): if the cursor is
/// currently hidden (`HIDE_COUNT != 0`), collapse the hide depth to a single
/// level so the following `dis_cur` uncovers it immediately. A **non-zero**
/// `INTIN[0]` performs an ordinary counted show — this is the reverse of the
/// naive reading of the flag.
pub fn v_show_c(pb: &Vdipb, backend: &dyn RasterBackend) {
    ensure_init(backend);
    if pb.intin(0) == 0 && HIDE_COUNT.load(Ordering::Relaxed) != 0 {
        HIDE_COUNT.store(1, Ordering::Relaxed);
    }
    dis_cur(backend);
}

/// Force-show the cursor at its **current** position (VDI `v_dspcur` escape,
/// EmuTOS escfn18 → `v_show_c` with `INTIN[0]` forced to 0): collapse any hide
/// depth to a single level, then display. The Atari-documented `x,y`
/// coordinates are ignored, matching EmuTOS/Atari TOS.
pub fn force_show(backend: &dyn RasterBackend) {
    ensure_init(backend);
    if HIDE_COUNT.load(Ordering::Relaxed) != 0 {
        HIDE_COUNT.store(1, Ordering::Relaxed);
    }
    dis_cur(backend);
}

/// Opcode 123 — `v_hide_c`: erase the cursor (if shown) and increment the
/// hide depth.
pub fn v_hide_c(backend: &dyn RasterBackend) {
    if shown() {
        erase(backend);
    }
    let c = HIDE_COUNT.load(Ordering::Relaxed);
    HIDE_COUNT.store(c + 1, Ordering::Relaxed);
}

// ── Direct (non-trap) cursor control for kernel code ──
//
// Callers without a VDI parameter block — the VDI's own raster and input
// paths, `graf_mouse(M_OFF/M_ON)`, the shell loop's `sh_toalpha`
// (`kernel/src/shell_loop.rs`); until DESK43 U10 also the built-in desktop,
// which bracketed each repaint with `hide` + `show` — use the same machinery
// as `v_hide_c`/`v_show_c`, without a `Vdipb`.

/// Erase the cursor and increment the hide depth (kernel `v_hide_c`).
pub fn hide(backend: &dyn RasterBackend) {
    if shown() {
        erase(backend);
    }
    HIDE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Decrement the hide depth; draw the cursor when it becomes shown (kernel
/// `v_show_c`). Balances a [`hide`]. Same counted-show core as the trap path.
pub fn show(backend: &dyn RasterBackend) {
    ensure_init(backend);
    dis_cur(backend);
}

/// Force the cursor visible (hide depth 0) and draw it — the initial unhide.
pub fn show_reset(backend: &dyn RasterBackend) {
    ensure_init(backend);
    let prev = HIDE_COUNT.swap(0, Ordering::Relaxed);
    if prev > 0 {
        draw(backend);
    }
}

/// Show the graphics cursor at `(x, y)` (VDI `v_dspcur` escape).
pub fn display_at(x: i16, y: i16, backend: &dyn RasterBackend) {
    ensure_init(backend);
    if shown() {
        erase(backend);
    }
    GCURX.store(x, Ordering::Relaxed);
    GCURY.store(y, Ordering::Relaxed);
    HIDE_COUNT.store(0, Ordering::Relaxed);
    draw(backend);
}

/// Remove the graphics cursor (VDI `v_rmcur` escape).
pub fn remove(backend: &dyn RasterBackend) {
    if shown() {
        erase(backend);
    }
    HIDE_COUNT.store(1, Ordering::Relaxed);
}

/// Opcode 124 — `vq_mouse`: report the button bitmask + cursor position.
pub fn vq_mouse(pb: &Vdipb) {
    pb.set_intout(0, MOUSE_BT.load(Ordering::Relaxed) as i16);
    pb.set_ptsout(0, GCURX.load(Ordering::Relaxed));
    pb.set_ptsout(1, GCURY.load(Ordering::Relaxed));
    pb.set_contrl(4, 1);
    pb.set_contrl(2, 1);
}

/// Swap one exchange vector: install VDI pointer slot 0, return the previous
/// one in VDI pointer slot 1 — EmuTOS `vdi_vex_wheelv` and friends,
/// `CONTRL[7..8]` in, `CONTRL[9..10]` out (`vdi/vdi_mouse.c:438-441`).
///
/// Both slots are addressed at the **caller's** pointer width, the way gemlib
/// packs them: pointer `n` starts at CONTRL word `7 + n * N_PTRINTS` with
/// `N_PTRINTS = sizeof(void *) / sizeof(short)`
/// (`sdk/gemlib/gemlib-0.44.0/gem_vdiP.h:177,248`) — 2 WORDs on m68k (whence
/// EmuTOS' fixed `CONTRL[9]`), **4 on LP64**. Reading the new pointer at word
/// 7 was already right; writing the old one at word 9 was the m68k offset and
/// landed two WORDs short of where an LP64 `vex_*(new, &old)` reads it, so
/// `old` came back as garbage. The kernel's own VDI client already uses the
/// LP64 slot-1 offset (`kernel/src/vdi_client.rs:226`).
fn swap_vector(pb: &Vdipb, slot: &AtomicU64) -> (u64, u64) {
    let new = pb.contrl_ptr(0);
    let prev = slot.swap(new, Ordering::Relaxed);
    pb.set_contrl_ptr(1, prev);
    (prev, new)
}

/// The OS-hook half of `vex_butv`/`vex_motv`/`vex_curv`/`vex_timv` (ADR-031):
/// the calling process owns the vector it installed, so its `Pterm` can give
/// the previous one back (rule 3), and a vector other than the default warns
/// once per process — the VDI stores it but never calls it (rules 1, 5).
fn exchange_hook(cell: Cell, hook: Hook, prev: u64, new: u64, default: u64) {
    let slot = crate::bdos::process::slot_for(crate::bdos::process::current());
    hooks::record_install(slot, cell, prev, new);
    if let Some(slot) = slot {
        if new != default {
            hooks::note(slot, hook, new, Via::Vex);
        }
    }
}

/// The vector behind a VDI hook cell, for its owner's `Pterm`
/// ([`hooks::on_terminate`]); `None` for a cell that is no VDI vector.
pub(crate) fn vex_cell(cell: Cell) -> Option<&'static AtomicU64> {
    match cell {
        Cell::VexButv => Some(&USER_BUT),
        Cell::VexMotv => Some(&USER_MOT),
        Cell::VexCurv => Some(&USER_CUR),
        Cell::VexTimv => Some(&TIM_ADDR),
        Cell::Vector(_) => None,
    }
}

/// Opcode 125 — `vex_butv`: install the button-change vector.
///
/// The installed address is stored and returned, never executed — and the
/// same holds for `vex_motv`, `vex_curv` and `vex_timv` — although EmuTOS
/// calls all four from `mouse_int` / `tick_int` (`vdi/vdi_asm.S:283,359,367`,
/// `vdi/vdi_misc.c:83`). ARCH03 stage 3a (F4) re-checked this against the
/// source and keeps it a documented no-op: `user_mot` returns a modified
/// position to its caller, which the fire-and-forget `vex_wheelv` upcall
/// cannot carry, and the delivery contract for that is not designed. Since
/// nothing is ever run or dereferenced through these slots, per-process
/// rights change nothing for them. `docs/api/vdi/input/_mouse.md`, NewTOS
/// implementation notes. ADR-031: not silent — a vector other than the
/// default warns once per process, and the installing process owns it
/// ([`exchange_hook`]).
pub fn vex_butv(pb: &Vdipb) {
    let (prev, new) = swap_vector(pb, &USER_BUT);
    exchange_hook(Cell::VexButv, Hook::VexButv, prev, new, just_rts_va());
}
/// Opcode 126 — `vex_motv`: install the mouse-move vector.
pub fn vex_motv(pb: &Vdipb) {
    let (prev, new) = swap_vector(pb, &USER_MOT);
    exchange_hook(Cell::VexMotv, Hook::VexMotv, prev, new, just_rts_va());
}
/// Opcode 127 — `vex_curv`: install the cursor-draw vector.
pub fn vex_curv(pb: &Vdipb) {
    let (prev, new) = swap_vector(pb, &USER_CUR);
    exchange_hook(Cell::VexCurv, Hook::VexCurv, prev, new, mov_cur as *const () as u64);
}

/// Opcode 134 — `vex_wheelv`: install the mouse-wheel vector (EmuTOS
/// `vdi_vex_wheelv`, a Milan VDI extension, `vdi/vdi_mouse.c:424-442`).
///
/// EmuTOS only stores the address, because on m68k every caller is at the
/// same privilege as the VDI. Here the stored address may belong to the EL1
/// AES or to an EL0 application, and [`call_user_wheel`] has to know which —
/// so the swap is followed by a classification of the **new** value:
///
/// * `just_rts` → [`WHEEL_KIND_NONE`] (an application clearing the vector);
/// * the [`aes_wheel_token`] → [`WHEEL_KIND_AES`] (an application putting the
///   `old` it saved back, the normal tear-down of a well-behaved program);
/// * anything else, issued by a live EL0 process → [`WHEEL_KIND_EL0`] owned by
///   that process's slot;
/// * anything else with no live process behind the trap → left as
///   [`WHEEL_KIND_NONE`]: the address is stored (so the `vex_*` save/restore
///   contract still round-trips) but not called, because nothing identifies
///   whose code it is. No in-kernel caller installs a wheel vector through the
///   trap; the AES uses [`install_aes_wheel`].
pub fn vex_wheelv(pb: &Vdipb) {
    wheel_init();
    swap_vector(pb, &USER_WHEEL);
    let new = USER_WHEEL.load(Ordering::Relaxed);
    let kind = if new == just_rts_va() || new == 0 {
        WHEEL_KIND_NONE
    } else if new == aes_wheel_va() {
        WHEEL_KIND_AES
    } else {
        match crate::bdos::process::slot_for(crate::bdos::process::current()) {
            Some(slot) => {
                USER_WHEEL_SLOT.store(slot, Ordering::Relaxed);
                WHEEL_KIND_EL0
            }
            None => WHEEL_KIND_NONE,
        }
    };
    if kind != WHEEL_KIND_EL0 {
        USER_WHEEL_SLOT.store(usize::MAX, Ordering::Relaxed);
    }
    USER_WHEEL_KIND.store(kind, Ordering::Relaxed);
}

/// Opcode 118 — `vex_timv`: install the timer-tick vector and return the
/// tick period in ms (`Tickcal`).
pub fn vex_timv(pb: &Vdipb) {
    let (prev, new) = swap_vector(pb, &TIM_ADDR);
    exchange_hook(Cell::VexTimv, Hook::VexTimv, prev, new, just_rts_va());
    pb.set_intout(0, TICKCAL_MS);
    pb.set_contrl(4, 1);
}
