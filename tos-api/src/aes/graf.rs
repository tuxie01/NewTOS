//! AES Graphics Manager (`graf_*`, opcodes 70–79).
//!
//! Spec: [`docs/api/aes/window-graphics.md`]. Source: EmuTOS @ `1a7484ab`
//! `aes/gemgrlib.c` (`gr_*`), dispatch `aes/gemsuper.c`. Interactive
//! feedback helpers (rubber-band / drag / animate an outline box, track a
//! gadget) plus three queries (`graf_handle`, `graf_mouse`,
//! `graf_mkstate`). The box-feedback calls own the mouse and block until
//! the button is released.
//!
//! The interactive outline (rubber-band select, drag-box, move-box) is a
//! 1-pixel rectangle drawn in **XOR** mode and **50% dithered**: XOR is
//! self-inverse, so redrawing the same rectangle erases it and restores the
//! exact pixels underneath — over the (non-white) desktop pattern, window
//! content and desktop icons alike. Matches EmuTOS `gsx_xbox` → `gsx_xline`
//! (`aes/gemgraf.c:140-163,271-278`), whose `0x5555`/`0xaaaa` parity masks put
//! the outline on the screen-aligned checkerboard so it "looks right on a
//! dithered surface". (The earlier solid-white erase assumed a white background
//! and left a white "staircase" trail on the patterned desktop / windows.)

use super::object::{self, Object, ObjTree};
use super::Aespb;
use crate::host::host;
use crate::vdi::backend::{ClipRect, Pel, RasterBackend, WriteMode};
use tos_types::aes::{ARROW, M_OFF, M_ON, M_PREVIOUS, M_RESTORE, M_SAVE, OUTLN_CROSS, USER_DEF};

const WHITE: Pel = 0;
const BLACK: Pel = 1;

/// EmuTOS `gsx_bxpts` (`aes/gemgraf.c:246-258`) — the five polyline points of
/// the box `(x, y, w, h)`, clockwise from the top-left and back to it, so the
/// four segments are top, right, bottom, left.
fn gsx_bxpts(x: i16, y: i16, w: i16, h: i16) -> [(i16, i16); 5] {
    [
        (x, y),
        (x + w - 1, y),
        (x + w - 1, y + h - 1),
        (x, y + h - 1),
        (x, y),
    ]
}

/// EmuTOS `gsx_xline` (`aes/gemgraf.c:140-163`) — draw a polyline as **50%
/// dithered** segments, "a box that will look right on a dithered surface"
/// (`:271-273`). Each segment gets a user line style chosen by the parity of
/// its start point:
///
/// * horizontal (`x0 != x1`): `hztltbl[y & 1]` taken at the *left* endpoint,
///   `hztltbl = { 0x5555, 0xaaaa }`;
/// * vertical (`x0 == x1`): `verttbl[(x & 1) | ((y & 1) << 1)]`,
///   `verttbl = { 0x5555, 0xaaaa, 0xaaaa, 0x5555 }`.
///
/// Both tables resolve to the same screen-aligned checkerboard — a pixel is on
/// exactly when `x + y` is odd — because the horizontal dash is applied at
/// absolute `x` (`abline`'s `draw_rect_common` path, `vdi/vdi_line.c:2327-2344`)
/// while the vertical one rotates one bit per pixel from the start point
/// (`swblit_vertical_line16`, `:2084-2145`). That is what makes two boxes drawn
/// next to each other line up.
///
/// Each segment is its own two-point `v_pline`, so `polyline` sets
/// `LSTLIN = TRUE` for all of them (`vdi/vdi_line.c:1106-1108`) and the XOR
/// end-point shortening never applies: the four corners are complemented by two
/// segments each and therefore stay unchanged. `draw_line` passes `last = true`
/// for the same reason.
fn gsx_xline(pts: &[(i16, i16)], backend: &dyn RasterBackend, clip: ClipRect) {
    const HZTLTBL: [u16; 2] = [0x5555, 0xaaaa];
    const VERTTBL: [u16; 4] = [0x5555, 0xaaaa, 0xaaaa, 0x5555];
    for seg in pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (seg[0], seg[1]);
        let st = if x0 == x1 {
            VERTTBL[((x0 & 1) | ((y0 & 1) << 1)) as usize]
        } else {
            // `linexy` is the left-hand endpoint of the pair (`gemgraf.c:155`).
            let ly = if x0 < x1 { y0 } else { y1 };
            HZTLTBL[(ly & 1) as usize]
        };
        backend.draw_line(x0, y0, x1, y1, BLACK, WriteMode::Xor, clip, st);
    }
}

/// Draw/erase a 1-pixel outline rectangle in **XOR** mode — EmuTOS `gsx_xbox`
/// = [`gsx_bxpts`] + [`gsx_xline`] (`aes/gemgraf.c:271-278`). XOR is
/// self-inverse, so drawing the same rectangle a second time restores the exact
/// pixels underneath — the rubber-band / drag / move outline leaves no trail
/// over the desktop pattern, window content or icons.
///
/// The `_col` pen is ignored — `WriteMode::Xor` complements the destination
/// regardless of pen — but the argument is kept so the draw/erase call sites
/// read symmetrically (`BLACK` to show, `WHITE` to hide).
fn outline(x: i16, y: i16, w: i16, h: i16, _col: Pel, backend: &dyn RasterBackend, clip: ClipRect) {
    if w <= 0 || h <= 0 {
        return;
    }
    gsx_xline(&gsx_bxpts(x, y, w, h), backend, clip);
}

/// EmuTOS `gsx_xbox` (`aes/gemgraf.c:274-278`): complement the 1-pixel outline
/// of the rectangle `(x, y, w, h)` in XOR write mode. `gsx_bxpts` builds the
/// five corner points and `gsx_xline` draws the four segments; the write mode
/// and the screen-wide clip come from `gr_setup`
/// (`gsx_sclip(&gl_rscreen); gsx_attr(FALSE, MD_XOR, BLACK)`, `gemgrlib.c:76-80`).
///
/// Shared with the Control Manager's sizer and mover drags
/// (`window::drag_size`/`drag_move`, our `gr_rubwind`/`gr_dragbox`), which draw
/// the same box — one primitive, one behaviour.
pub(super) fn gsx_xbox(x: i16, y: i16, w: i16, h: i16, backend: &dyn RasterBackend, clip: ClipRect) {
    outline(x, y, w, h, BLACK, backend, clip);
}

/// The poll behind every drag/watch loop here — EmuTOS `gr_stilldn`'s
/// `ev_multi(MU_KEYBD | MU_BUTTON | MU_M1, …, 0x0001ff00L, …)`
/// (`aes/gemgrlib.c:47-73`), which returns `MU_BUTTON` once the button is up.
/// That return runs `ev_rets` (`aes/gemevlib.c:268`), so the release the loop
/// ends on is consumed there and not reported again by the application's next
/// event ([`super::input::ev_rets`]).
fn mouse_down() -> bool {
    let down = crate::vdi::mouse_buttons() & 0x01 != 0;
    if !down {
        super::input::ev_consume();
    }
    down
}

/// EmuTOS `mul_div_round` (util/miscasm.S): `mult1 * mult2 / divisor`, rounded
/// **away from zero** — the D-098 "slider creep" fix (gemgrlib.c:410). Computed
/// as `((m1·m2·2)/divisor ± 1) >> 1`, matching the m68k `divs.w`/`asr.w`
/// sequence; done in `i32` so `rel·1000` cannot overflow.
pub(crate) fn mul_div_round(mult1: i32, mult2: i32, divisor: i32) -> i32 {
    if divisor == 0 {
        return 0;
    }
    let n = mult1 * mult2 * 2 / divisor;
    if n < 0 {
        (n - 1) >> 1
    } else {
        (n + 1) >> 1
    }
}

/// `gr_clamp` (EmuTOS `gemgrlib.c:83`): the signed rubber-band extent from a
/// pinned `origin` to the live `cursor`, plus one, clamped by a **signed** `max`
/// to `min`. The `+1` makes the extent span both endpoints inclusively; the
/// clamp is a signed `max`, *not* a magnitude clamp. A positive `min` therefore
/// forces a down-right minimum-size box (a leftward drag is pulled back up to
/// `+min`); `SHRT_MIN` disables the clamp for free four-quadrant growth
/// (`w`/`h` go negative when dragging up/left).
fn gr_clamp(cursor: i16, origin: i16, min: i16) -> i16 {
    (cursor - origin + 1).max(min)
}

/// Normalise a pinned `origin` plus a signed `gr_clamp` extent
/// (`ext = cursor - origin + 1`) into a drawable top-left / positive-length
/// pair for [`outline`]: the far corner sits at `origin + ext - 1`, so the box
/// spans that corner and `origin` and grows toward the cursor in any quadrant.
/// Saturating arithmetic keeps a `SHRT_MIN`-derived extreme extent from
/// overflowing (`outline` needs a non-negative length).
fn draw_span(origin: i16, ext: i16) -> (i16, i16) {
    let far = origin.saturating_add(ext.saturating_sub(1));
    let lo = origin.min(far);
    let hi = origin.max(far);
    (lo, (hi - lo).saturating_add(1))
}

/// `graf_rubberbox` (70) — rubber-band a box with one corner pinned at `(x,y)`;
/// the opposite corner tracks the mouse until the button comes up
/// (spec: `window-graphics.md` §`graf_rubberbox (70)`).
///
/// `int_out[1]`/`int_out[2]` are the **signed** `gr_clamp` extents
/// `w = max(cursor_x - x + 1, minw)`, `h = max(cursor_y - y + 1, minh)` — the
/// `+1` and the *signed* `max` are load-bearing. A positive `minw`/`minh`
/// yields a positive, down-right-only minimum-size box; a caller wanting free
/// four-quadrant growth (negative `w`/`h` when dragging up/left) passes
/// `minw = minh = SHRT_MIN`, exactly as the desktop's rubber-band select does
/// (EmuTOS `deskact.c:631`). The live outline follows the same signed extent
/// (top-left/length via [`draw_span`], like EmuTOS `gr_rubwind`). `int_out[0]`=1.
///
/// `gr_rubbox` → `gr_rubwind` brackets the whole track with
/// `wm_update(BEG_UPDATE)`/`(END_UPDATE)` (`aes/gemgrlib.c:205,219`); the
/// dispatcher waited for the lock before it came here (A118).
pub fn graf_rubberbox(pb: &Aespb) {
    super::window::update_beg();
    let ax = pb.intin(0);
    let ay = pb.intin(1);
    let minw = pb.intin(2);
    let minh = pb.intin(3);
    // Seed the extent from the current mouse position so the first drawn box is
    // the live rubber-band rect, not a degenerate `SHRT_MIN`-sized frame.
    let (mut w, mut h) = {
        let (mx, my) = crate::vdi::mouse_position();
        (gr_clamp(mx, ax, minw), gr_clamp(my, ay, minh))
    };
    if let (Some(host_h), Some(backend)) = (host(), host().and_then(|h| h.raster())) {
        let clip = ClipRect::full(backend.width(), backend.height());
        // Hide the cursor for the whole rubber-band: the XOR outline is the drag
        // feedback (its far corner tracks the mouse), and a shown cursor's
        // save-under would otherwise capture XOR outline pixels and strew single-
        // pixel artefacts along the drag path. EmuTOS `gr_rubwind` likewise
        // brackets the band with `gsx_moff`/`gsx_mon`.
        crate::vdi::cursor_hide(backend);
        let (mut bx, mut bw) = draw_span(ax, w);
        let (mut by, mut bh) = draw_span(ay, h);
        outline(bx, by, bw, bh, BLACK, backend, clip);
        loop {
            let (mx, my) = crate::vdi::mouse_position();
            let nw = gr_clamp(mx, ax, minw);
            let nh = gr_clamp(my, ay, minh);
            if nw != w || nh != h {
                outline(bx, by, bw, bh, WHITE, backend, clip);
                w = nw;
                h = nh;
                (bx, bw) = draw_span(ax, w);
                (by, bh) = draw_span(ay, h);
                outline(bx, by, bw, bh, BLACK, backend, clip);
            }
            if !mouse_down() {
                break;
            }
            host_h.pump_input();
        }
        outline(bx, by, bw, bh, WHITE, backend, clip);
        crate::vdi::cursor_show(backend);
    }
    super::window::update_end();
    pb.set_intout(1, w);
    pb.set_intout(2, h);
    pb.set_ret(1);
}

/// `graf_dragbox` (71) — drag a fixed-size `w×h` box (starting at
/// `sx,sy`) with the mouse, clamped to the bound rect `int_in[4..7]`.
/// Returns the final `x,y` in `int_out[1..2]`.
///
/// `gr_dragbox` brackets the whole drag with `wm_update(BEG_UPDATE)`/
/// `(END_UPDATE)` (`aes/gemgrlib.c:250,267`); the dispatcher waited for the
/// lock before it came here (A118).
pub fn graf_dragbox(pb: &Aespb) {
    super::window::update_beg();
    let w = pb.intin(0);
    let h = pb.intin(1);
    let sx = pb.intin(2);
    let sy = pb.intin(3);
    let (bx, by, bw, bh) = (pb.intin(4), pb.intin(5), pb.intin(6), pb.intin(7));
    let (mut lx, mut ly) = (sx, sy);
    if let (Some(host_h), Some(backend)) = (host(), host().and_then(|h| h.raster())) {
        let clip = ClipRect::full(backend.width(), backend.height());
        // Hide the cursor for the whole drag — see the note in `graf_rubberbox`:
        // the XOR outline is the feedback, and a shown cursor's save-under would
        // capture XOR pixels and leave a dotted trail.
        crate::vdi::cursor_hide(backend);
        let (smx, smy) = crate::vdi::mouse_position();
        outline(lx, ly, w, h, BLACK, backend, clip);
        loop {
            let (mx, my) = crate::vdi::mouse_position();
            // Guard the clamp bounds: a bound rect smaller than the box makes
            // `max < min`, which would panic `i16::clamp` (fatal in no_std).
            let nx = (sx + mx - smx).clamp(bx, (bx + bw - w).max(bx));
            let ny = (sy + my - smy).clamp(by, (by + bh - h).max(by));
            if nx != lx || ny != ly {
                outline(lx, ly, w, h, WHITE, backend, clip);
                lx = nx;
                ly = ny;
                outline(lx, ly, w, h, BLACK, backend, clip);
            }
            if !mouse_down() {
                break;
            }
            host_h.pump_input();
        }
        outline(lx, ly, w, h, WHITE, backend, clip);
        crate::vdi::cursor_show(backend);
    }
    super::window::update_end();
    pb.set_intout(1, lx);
    pb.set_intout(2, ly);
    pb.set_ret(1);
}

/// Animate a 1-pixel **XOR** outline between two rects (movebox / grow /
/// shrink), used by `graf_movebox`/`graf_growbox`/`graf_shrinkbox` and the Form
/// Manager's `form_dial` (FMD_GROW/FMD_SHRINK). Each frame erases the previous
/// outline and draws the next through [`outline`] (XOR is self-inverse), so the
/// moving box restores the desktop pattern underneath and leaves no trail — no
/// solid-white overpaint. The number of frames scales with the box's travel
/// (EmuTOS `gr_scale`, see [`stepcalc`]), not a fixed count.
pub(crate) fn animate(from: (i16, i16, i16, i16), to: (i16, i16, i16, i16)) {
    let Some(backend) = host().and_then(|h| h.raster()) else {
        return;
    };
    let clip = ClipRect::full(backend.width(), backend.height());
    // Frame count from the box's travel (move distance or size change), not a
    // fixed 8 — see [`stepcalc`] (EmuTOS `gr_scale` halving cadence). A large
    // dialog animates in more frames than a small one, matching the reference.
    let steps = stepcalc(from, to);
    let lerp = |a: i16, b: i16, i: i16| a + (b - a) * i / steps;
    let mut prev: Option<(i16, i16, i16, i16)> = None;
    for i in 0..=steps {
        let r = (
            lerp(from.0, to.0, i),
            lerp(from.1, to.1, i),
            lerp(from.2, to.2, i),
            lerp(from.3, to.3, i),
        );
        // Erase the previous outline (a second XOR pass restores the exact
        // pixels underneath), then draw the next — both are XOR, so no white
        // erase-fill is ever painted.
        if let Some(p) = prev {
            outline(p.0, p.1, p.2, p.3, WHITE, backend, clip);
        }
        outline(r.0, r.1, r.2, r.3, BLACK, backend, clip);
        // brief dwell so the animation is visible
        if let Some(h) = host() {
            let t = h.monotonic_millis() + 15;
            while h.monotonic_millis() < t {
                core::hint::spin_loop();
            }
        }
        prev = Some(r);
    }
    if let Some(p) = prev {
        outline(p.0, p.1, p.2, p.3, WHITE, backend, clip);
    }
}

/// The outline-animation frame count via EmuTOS `gr_scale` (gemgrlib.c:91-111):
/// count how many times the average of an x-/y-distance halves to zero — i.e.
/// `floor(log2((xdist+ydist)/2)) + 1`. The distance fed to `gr_scale` is chosen
/// exactly as EmuTOS does per operation, so both callers stay faithful:
/// - a pure translation (`graf_movebox`, same size) feeds the **move distance**,
///   like `gr_movebox` (gemgrlib.c:296-299);
/// - a grow/shrink (`graf_growbox`/`graf_shrinkbox`, `form_dial` FMD_GROW/SHRINK)
///   feeds the width/height **half-difference** `pt_w/2 - orgw/2`, like
///   `gr_stepcalc` (gemgrlib.c:114-124).
///
/// Each `/2` is applied per term as in EmuTOS so the integer rounding matches.
/// Clamped to at least 1 so the outline lerp always reaches the far rect (a zero
/// count would divide by zero and draw a single frame).
fn stepcalc(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> i16 {
    let (dw, dh) = ((b.2 - a.2).abs(), (b.3 - a.3).abs());
    let (xdist, ydist) = if dw == 0 && dh == 0 {
        // Pure translation → gr_movebox's move distance.
        ((b.0 - a.0).abs(), (b.1 - a.1).abs())
    } else {
        // Grow/shrink → gr_stepcalc's size half-difference (bigger − smaller).
        (
            (a.2.max(b.2) / 2) - (a.2.min(b.2) / 2),
            (a.3.max(b.3) / 2) - (a.3.min(b.3) / 2),
        )
    };
    let mut dist = (xdist + ydist) / 2;
    let mut cnt = 0i16;
    while dist != 0 {
        cnt += 1;
        dist /= 2;
    }
    cnt.max(1)
}

/// `graf_movebox` (72) — animate an outline box of size `w×h` from
/// `(sx,sy)` to `(dx,dy)`.
pub fn graf_movebox(pb: &Aespb) {
    let w = pb.intin(0);
    let h = pb.intin(1);
    let sx = pb.intin(2);
    let sy = pb.intin(3);
    let dx = pb.intin(4);
    let dy = pb.intin(5);
    animate((sx, sy, w, h), (dx, dy, w, h));
    pb.set_ret(1);
}

/// `graf_growbox` (73) — zoom an outline from the small rect (`int_in
/// [0..3]`) out to the big rect (`int_in[4..7]`).
pub fn graf_growbox(pb: &Aespb) {
    let small = (pb.intin(0), pb.intin(1), pb.intin(2), pb.intin(3));
    let big = (pb.intin(4), pb.intin(5), pb.intin(6), pb.intin(7));
    animate(small, big);
    pb.set_ret(1);
}

/// `graf_shrinkbox` (74) — zoom an outline from the big rect down to the
/// small rect.
pub fn graf_shrinkbox(pb: &Aespb) {
    let small = (pb.intin(0), pb.intin(1), pb.intin(2), pb.intin(3));
    let big = (pb.intin(4), pb.intin(5), pb.intin(6), pb.intin(7));
    animate(big, small);
    pb.set_ret(1);
}

/// `graf_watchbox` (75) — track the button while it is held: object `obj`
/// in `addr_in[0]` shows `instate` while the mouse is inside its rect and
/// `outstate` while outside. Returns 1 if released inside, else 0.
pub fn graf_watchbox(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let obj = pb.intin(1);
    let instate = pb.intin(2) as u16;
    let outstate = pb.intin(3) as u16;
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    pb.set_ret(if watchbox(tree, obj, instate, outstate) { 1 } else { 0 });
}

/// Track the button over object `obj`: show `instate` while the mouse is
/// inside its rectangle, `outstate` while outside, restoring on each edge;
/// return whether it was released inside (EmuTOS `gr_watchbox`). Shared with
/// the Form Manager's `form_button`.
pub(crate) fn watchbox(tree: *mut Object, obj: i16, instate: u16, outstate: u16) -> bool {
    watchbox_t(ObjTree::new(tree as *const Object), obj, instate, outstate)
}

/// Track the button over `obj` through the caller-supplied [`ObjTree`] accessor
/// (`lp64` for a kernel-built tree such as a `form_alert` button). Body of
/// [`watchbox`]; used by `do_alert`'s modal loop with an LP64 accessor so the
/// kernel alert array is read at its native 32-byte width even while an ILP32
/// app is current.
pub(crate) fn watchbox_t(t: ObjTree, obj: i16, instate: u16, outstate: u16) -> bool {
    let mut was_in = true;
    if let Some(h) = host() {
        object::change_t(t, obj, instate, true);
        loop {
            let (mx, my) = crate::vdi::mouse_position();
            let (rx, ry, rw, rh) = object::abs_rect_t(t, obj);
            let now_in = mx >= rx && mx < rx + rw && my >= ry && my < ry + rh;
            if now_in != was_in {
                object::change_t(t, obj, if now_in { instate } else { outstate }, true);
                was_in = now_in;
            }
            if !mouse_down() {
                break;
            }
            h.pump_input();
        }
    }
    was_in
}

/// `graf_slidebox` (76) — slide child `obj` within `parent` along one axis
/// (`isvert`=`int_in[2]`); returns the position 0–1000.
pub fn graf_slidebox(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let parent = pb.intin(0);
    let obj = pb.intin(1);
    let isvert = pb.intin(2) != 0;
    if tree.is_null() || parent < 0 || obj < 0 {
        pb.set_ret(0);
        return;
    }
    let mut pos = 0i16;
    // App-provided tree at `addr_in[0]`: read/write `parent`/`obj` through the
    // caller-ABI `ObjTree` accessor so an ILP32 app's 24-byte OBJECT records are
    // addressed at their own field offsets, not the native LP64 ones. For an
    // LP64/kernel caller `current_abi()` is `Lp64`, so the offsets are
    // byte-identical to the previous `*mut Object` access. `abs_rect`/`draw_obj`
    // build their own ABI-aware accessor internally.
    let t = ObjTree::new(tree as *const Object);
    // `travel` = parent − child extent (EmuTOS `divis = c.g_h - t.g_h`);
    // `c0` = the elevator's start position relative to the parent.
    let (travel, c0) = if isvert {
        (t.height(parent) - t.height(obj), t.y(obj))
    } else {
        (t.width(parent) - t.width(obj), t.x(obj))
    };
    if let Some(h) = host() {
        let (prx, pry, _, _) = object::abs_rect(tree as *const Object, parent);
        // Grip offset: how far the mouse sat from the elevator's top-left
        // at grab time, held constant for the whole drag so the elevator
        // does not snap its leading edge to the cursor. EmuTOS `gr_dragbox`
        // takes it via `gr_clamp(sx+1, sy+1, 0, 0, &off)` → `off =
        // max(mouse − elev, 0)` (gemgrlib.c:253-260). `travel` can be
        // negative for a malformed slider; `.max(0)` keeps `clamp` sound.
        let (m0x, m0y) = crate::vdi::mouse_position();
        let grip = if isvert {
            (m0y - (pry + c0)).max(0)
        } else {
            (m0x - (prx + c0)).max(0)
        };
        let hi = travel.max(0);
        loop {
            let (mx, my) = crate::vdi::mouse_position();
            // New elevator position (relative to the parent) = mouse − grip
            // − parent origin, constrained to the track [0, travel].
            let rel = if isvert {
                (my - grip - pry).clamp(0, hi)
            } else {
                (mx - grip - prx).clamp(0, hi)
            };
            if isvert {
                t.set_y(obj, rel);
            } else {
                t.set_x(obj, rel);
            }
            if let Some(backend) = h.raster() {
                object::draw_obj(tree as *const Object, parent, 8, backend);
            }
            // Position 0..1000, rounded away from zero — EmuTOS D-098
            // "slider creep" fix: `mul_div_round(divnd, 1000, divis)`
            // (gemgrlib.c:410), not a truncating divide.
            pos = if travel > 0 {
                mul_div_round(rel as i32, 1000, travel as i32) as i16
            } else {
                0
            };
            if !mouse_down() {
                break;
            }
            h.pump_input();
        }
    }
    // EmuTOS `gr_slidebox` returns the position 0..1000 as its function return
    // value, which the dispatcher (`gemsuper.c:284-285`) stores in `int_out[0]`
    // (= RET_CODE). The standard AES binding reads `int_out[0]`, so writing the
    // position anywhere else makes every caller read a constant. Match EmuTOS:
    // position → `int_out[0]`, nothing in `int_out[1]`.
    pb.set_ret(pos);
}

// ── the system-font metrics and the gadget box height ────────────────────
//
// EmuTOS keeps these as globals (`aes/gemgraf.c:40-52`) filled once by
// `gsx_start` (`:405-415`), right after the AES workstation is open, from
// that workstation's current font:
//
//     gsx_textsize(&gl_wptschar, &gl_hptschar, &gl_wchar, &gl_hchar);  /* :405 */
//     gl_hbox = gl_hchar + 3;                                           /* :412 */
//
// `gsx_textsize` is `vqt_attributes` (`gemgsxif.c:505-512`): `gl_wchar` =
// `ptsout[2]` (the cell width), `gl_hchar` = `ptsout[3]` (the cell height,
// `top + bottom + 1` of the font header, `vdi_text.c:1018`). Which font that
// is, the VDI decides at `v_opnwk`: `def_font`, the `F_DEFAULT` face whose
// `form_height` fits the screen — 16 lines from 400 rows up, 8 below
// (`vdi_text.c:612`).
//
// NewTOS's AES opens no workstation of its own; it draws through the raster
// backend with the VDI's system font, and the face a fresh workstation gets
// is [`crate::vdi::font::DEFAULT`] (a constant — the `V_REZ_VT` choice is not
// implemented). The three functions below are that face's `vqt_attributes`
// answer and the `gl_hbox` rule on top of it: the **one** place the
// menu-bar row / gadget box height comes from. `graf_handle` reports it
// (`gemsuper.c:288-291`), the window manager builds `gl_rfull` and the
// gadget cells from it (`window::desktop_work`, `window::adj_hbox`), the
// Menu Manager reserves the row with it (`menu::desk_top`) and the mouse
// ownership test for a press reads the row from it (`window::chk_ctrl`,
// EmuTOS `gl_rmenu`, `gemgraf.c:427`; the bar *track* itself waits on
// `THEACTIVE`'s extent, `menu::in_ctwait`).

/// EmuTOS `gl_width` (`gemgraf.c:37`, set `:393`: `gl_ws.ws_xres + 1`) — the
/// AES workstation's pixel width, read from the raster backend the AES draws
/// through. It is the `gl_width` the resource loader's `fix_chpos` puts in
/// for an 80-character `ob_width` (`gemrslib.c:71-74`).
///
/// Without a raster (a host test that installs none) the classic 80-column
/// width `80 × gl_wchar` stands in — the one value that special case can
/// then mean.
pub(super) fn gl_width() -> i16 {
    crate::host::host()
        .and_then(|h| h.raster())
        .map(|r| r.width() as i16)
        .unwrap_or(80 * gl_wchar())
}

/// EmuTOS `gl_wchar` (`gemgraf.c:40`, set `:405`) — the system-font cell
/// width, `vqt_attributes`' `ptsout[2]` for the VDI's default face.
pub(super) fn gl_wchar() -> i16 {
    crate::vdi::font::font(crate::vdi::font::DEFAULT).max_cell_width
}

/// EmuTOS `gl_hchar` (`gemgraf.c:41`, set `:405`) — the system-font cell
/// height, `vqt_attributes`' `ptsout[3]` (`top + bottom + 1`) for the VDI's
/// default face: 16 for the 8×16 font.
pub(super) fn gl_hchar() -> i16 {
    let f = crate::vdi::font::font(crate::vdi::font::DEFAULT);
    f.top + f.bottom + 1
}

/// The `gl_hbox` rule on its own — `gl_hbox = gl_hchar + 3`
/// (`gemgraf.c:412`) for a system-font cell height `hchar`. [`gl_hbox`]
/// applies it to the live [`gl_hchar`]; it is exposed so the rule can be
/// checked for a cell height the VDI's fixed default face cannot supply
/// (8 → 11, the value EmuTOS computes on a screen under 400 lines).
pub const fn hbox_of(hchar: i16) -> i16 {
    hchar + 3
}

/// EmuTOS `gl_hbox` (`gemgraf.c:47`, set `:412`) — the unadjusted gadget box
/// height and the menu-bar row (`gl_rmenu = (0, 0, gl_width, gl_hbox)`,
/// `:427`; `gl_rfull = (0, gl_hbox, gl_width, gl_height - gl_hbox)`, `:424`):
/// `gl_hchar + 3`, i.e. 19 for the 8×16 font.
pub(super) fn gl_hbox() -> i16 {
    hbox_of(gl_hchar())
}

/// `graf_handle` (77) — the AES's VDI workstation handle (`int_out[0]`)
/// plus the system-font cell metrics in `int_out[1..4]` (EmuTOS
/// gemsuper.c:287-293).
pub fn graf_handle(pb: &Aespb) {
    // gl_handle: the physical workstation the AES's own `v_opnwk` opened at
    // session start (`gsx_wsopen`, gemgsxif.c:169-186) — a real, open one.
    pb.set_ret(super::gsxif::gl_handle());
    pb.set_intout(1, gl_wchar()); // gl_wchar: system-font cell width
    pb.set_intout(2, gl_hchar()); // gl_hchar: system-font cell height
    // gl_wbox / gl_hbox: the *unadjusted* gadget box unit (gemsuper.c:290-291)
    // — never the 3D-adjusted adj_wbox/adj_hbox the window frame is built
    // from. gl_hbox is derived above (gl_hchar + 3, gemgraf.c:412); gl_wbox
    // (max(gl_wchar+4, gl_hbox·aspect), gemgraf.c:413-415) is still the
    // window manager's constant, 19 for the 8×16 font on square pixels.
    pb.set_intout(3, super::window::GL_WBOX); // gl_wbox
    pb.set_intout(4, gl_hbox()); // gl_hbox
}

/// `graf_mouse` (78) — set the mouse cursor: `int_in[0]` = a preset shape
/// (ARROW 0 … OUTLN_CROSS 7), `USER_DEF` (255) with an `MFORM` in `addr_in[0]`,
/// `M_OFF`/`M_ON` to hide/show it (the **counting** cursor semaphore), or
/// `M_SAVE`/`M_RESTORE`/`M_PREVIOUS` to snapshot / re-install a cursor form.
///
/// EmuTOS `gr_mouse` (`gemgrlib.c:419-455`): `M_OFF`→`gsx_moff` (`gl_moff++`),
/// `M_ON`→`gsx_mon` (`gl_moff--`, show only at 0) — a counting semaphore, so
/// nested `M_OFF;M_OFF;M_ON` stays hidden. `M_SAVE` stores the current form in
/// `rlr->p_mouse`; `M_RESTORE`/`M_PREVIOUS` reinstall the saved / previous form.
///
/// Every other number is the `default:` arm (`gemgrlib.c:426-429`): a mode
/// outside `ARROW..=OUTLN_CROSS` is clamped to `ARROW` ("fail safe") and the
/// preset is then **installed** by the `gsx_mfset` the arm falls through to
/// (`:454`). There is no "unknown number, leave the cursor alone" path — 8,
/// 254, 261 or -1 all put the arrow up (A44).
pub fn graf_mouse(pb: &Aespb) {
    let number = pb.intin(0);
    if let Some(backend) = host().and_then(|h| h.raster()) {
        match number {
            // Counting hide/show (EmuTOS gsx_moff/gsx_mon), the same primitives
            // the window/menu/form redraw brackets use — not the force
            // remove/display_at, so nested hide blocks balance correctly.
            M_OFF => crate::vdi::mouse::hide(backend),
            M_ON => crate::vdi::mouse::show(backend),
            USER_DEF => set_user_form(pb, backend),
            // Saved-form slot (EmuTOS rlr->p_mouse) + previous-form slot
            // (gl_prevmouse), both maintained in the VDI cursor subsystem.
            M_SAVE => crate::vdi::mouse::save_form(),
            M_RESTORE => crate::vdi::mouse::restore_form(backend),
            M_PREVIOUS => crate::vdi::mouse::previous_form(backend),
            // `default:` — a preset, or an unknown number clamped to ARROW.
            _ => {
                let preset = if (ARROW..=OUTLN_CROSS).contains(&number) {
                    number
                } else {
                    ARROW
                };
                crate::vdi::mouse::set_preset(preset, backend);
            }
        }
    }
    pb.set_ret(1);
}

/// Install a caller-supplied `MFORM` from `addr_in[0]` (EmuTOS `gsx_mfset`):
/// 37 native WORDs — hotspot x/y, planes, fg/bg colour, 16 mask rows, 16
/// data rows.
fn set_user_form(pb: &Aespb, backend: &dyn RasterBackend) {
    let p = pb.addrin(0);
    if p == 0 {
        return;
    }
    // GEM contract — addr_in[0] points at a 37-WORD MFORM — read in the
    // caller's space (ARCH03 3a) before the form changes; one the program
    // cannot read ends the call (and the program) with the cursor untouched.
    let mut w = [0u8; 2 * 37];
    if pb.space().copy_from(&mut w, p).is_err() {
        return;
    }
    let word = |i: usize| i16::from_ne_bytes([w[2 * i], w[2 * i + 1]]);
    let mut mask = [0u16; 16];
    let mut data = [0u16; 16];
    for i in 0..16 {
        mask[i] = word(5 + i) as u16;
        data[i] = word(21 + i) as u16;
    }
    crate::vdi::mouse::set_form(word(0), word(1), &mask, &data, backend);
}

/// `graf_mkstate` (79) — poll the instantaneous mouse position
/// (`int_out[1..2]`), button mask (`int_out[3]`) and keyboard state
/// (`int_out[4]`). Does not wait. The keyboard state is `gr_mkstate`'s
/// `*pkstat = kstate` (`aes/gemgrlib.c:463`): Shift/Control/Alternate only,
/// bits 0-3, through [`super::gsxif::gsx_kstate`] (A126).
pub fn graf_mkstate(pb: &Aespb) {
    let (mx, my) = crate::vdi::mouse_position();
    pb.set_intout(1, mx);
    pb.set_intout(2, my);
    pb.set_intout(3, crate::vdi::mouse_buttons() as i16);
    pb.set_intout(4, super::gsxif::gsx_kstate());
    pb.set_ret(1);
}
