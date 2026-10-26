# `deskact.c` — selection, highlight, and drag state machine

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/deskact.c` (691 lines). Per
> [`docs/api/desktop/_overview.md`](_overview.md): `GLOBES`/`WNODE`/
> `FNODE`/`OBJECT`/`ICONBLK` are defined in
> [`_data-model.md`](_data-model.md) and `docs/api/structures/`; this
> entry documents the mouse-driven selection/drag *behaviour* built on
> top of them. Builds on [`desksupp-redraw.md`](desksupp-redraw.md)
> (`do_wredraw`, called throughout) and is a dependency of `deskmain.md`
> (`hndl_button` calls `act_bsclick`/`act_bdown`).
>
> **Contract vs. internal** (`docs/COMPATIBILITY.md`): entirely
> desktop-application-internal — no GEMDOS/AES/VDI trap takes any
> function in this file as a parameter. The **user-observable behaviour**
> (single-click/shift-click/double-click/rubber-band selection rules,
> drag-and-drop visual feedback) is the closest thing to a "contract"
> here, in the loose sense of "what a NewTOS desktop should feel like to
> use" — not an ABI. The actual state-machine implementation, the
polygon-drag-outline mechanism, and the point-array allocation are free.

---

## EmuTOS source

**Primary locations (public, `deskact.h`):**
- `deskact.c:419-466` `act_chg(wh, root, obj, pc, set, dodraw)` — toggle
  one object's `SELECTED` bit
- `deskact.c:480-552` `act_allchg(wh, root, ex_obj, pt, pc, set)` — bulk
  select/deselect within a rectangle
- `deskact.c:566-603` `act_bsclick(wh, root, mx, my, keystate, pc, dclick)`
  — single/double-click selection rules
- `deskact.c:609-690` `act_bdown(wh, root, in_mx, in_my, keystate, pc,
  pdobj)` — button-held handler: rubber-band select **or** drag

**Installed handler / vector:** `(none)` — called from `deskmain.c`'s
`hndl_button()` (mouse-button AES message handler, out of scope for this
task; see `deskmain.md`).

**Helpers called (this file, `static`):**
- `deskact.c:50-59` `gr_obfind(tree, root, mx, my)` — `objc_find` + a
  `act_chkobj` refinement (icon graphic/text hit-testing, not just the
  object's bounding box)
- `deskact.c:66-78` `gr_isdown(out, x, y, w, h)` — blocking wait via
  `evnt_multi(MU_BUTTON|MU_M1, …)` for the mouse to leave/enter a
  rectangle (used as the drag loop's "still held?" poll)
- `deskact.c:87-123` `gr_accobs(tree, root, pnum)` — collect the
  screen-absolute `(x,y)` of every currently-`SELECTED` object into a
  freshly `dos_alloc_anyram`'d `Point[]`
- `deskact.c:126-163` `move_drvicon(tree, root, x, y, pts, sxoff, syoff)`
  — desktop-icon-drag completion: snaps each selected icon to the grid
  via `snap_icon()` (see [`deskins.md`](deskins.md)), updates its `ANODE`
  `a_xspot`/`a_yspot`, and redraws old+new positions. **No collision
  check**: `snap_icon()` only rounds/clamps to the nearest grid cell and
  never consults `G.g_ahead`/`grid_free`, so dragging one or more desktop
  icons onto a cell already occupied by another icon silently overlaps
  them — confirmed by rereading `deskins.c:146-184` (see `deskins.md`'s
  `snap_icon` section for the full analysis).
- `deskact.c:169-188` `gr_extent(numpts, xylnpts, pt)` — bounding
  rectangle of a point list
- `deskact.c:195-207` `gr_plns(x, y, numpts, xylnpts, numobs, xyobpts)` —
  draws (XORs) the drag-outline polygon once per dragged object
- `deskact.c:210-233` `gr_bwait(po, mx, my, numpts, xylnpts, numobs,
  xyobpts)` — draw-old / wait-for-change / erase-old cycle (one frame of
  the drag animation)
- `deskact.c:236-245` `gr_obalign(numobs, x, y, xyobpts)` — recenters a
  point list relative to `(x,y)`
- `deskact.c:251-362` `gr_drgplns(in_mx, in_my, pc, numpts, xylnpts,
  numobs, xyobpts, pdulx, pduly, poffx, poffy, pdwh, pdobj)` — the full
  drag loop (detailed [below](#gr_drgplns--the-drag-loop))
- `deskact.c:374-406` `act_chkobj(tree, root, obj, mx, my, w, h)` — icon/
  text hit-test refinement (detailed [below](#act_chkobj--icon-vs-text-hit-testing))

**Helpers called (other files, cited — not re-derived here):**
- `desksupp-redraw.md`'s `do_wredraw()` — called at `deskact.c:156,159,
  461,550`
- `deskins.c:146-184` `snap_icon()` — see [`deskins.md`](deskins.md)
- `deskfpd.c:304` `pn_clear()` — clears every `FNODE.f_selected` in a
  window (see `_data-model.md`)
- `desksupp.c` `i_find(wh, obj, ppf, pisapp)` — resolves an object index
  to its `ANODE`/`FNODE`
- AES: `objc_find`, `evnt_multi`, `evnt_button`, `graf_mouse`,
  `graf_mkstate`, `graf_rubbox`, `wind_find` — already contract-documented
  under `docs/api/aes/`, not re-derived here

**Headers / type definitions used:**
- `include/rectfunc.h` — `GRECT`, `r_set`, `rc_copy`, `rc_intersect`,
  `rc_union`, `rc_constrain` (AES rectangle helpers, contract-documented
  under `docs/api/aes/window-graphics.md`)
- `deskbind.h:123-124` — `NUM_ICON_POINTS`=9, `NUM_TEXT_POINTS`=5 (see
  `_data-model.md`'s `g_xyicon[9]`/`g_xytext[5]`)
- `deskwin.h:44` — `DROOT`=1
- `deskact.c:42` — `SHRT_MIN`=-32767 (local substitute for a missing
  `limits.h`)
- AES key-state bits (`aesdefs.h`, not re-derived): `K_LSHIFT`/`K_RSHIFT`
  (shift-click detection), `MODE_CTRL` (referenced by callers, not this
  file directly)

**System variables / globals read:**
- `G.g_screen`/`G.g_screeninfo` — the object tree + per-object `FNODE`
  back-pointer (`fnptr`) and `ICONBLK` copy (`u.icon.block`) — see
  `_data-model.md`
- `G.g_iview` — `V_ICON`/`V_TEXT`, read by `act_chkobj`/`act_bdown` to
  choose hit-test geometry / drag-outline point set
- `G.g_iblist[]` — `act_chkobj` reads the icon's own `ICONBLK` (via
  `G.g_screeninfo[obj].u.icon.index`) for its `ib_xicon`/`ib_xtext`
  sub-rectangles
- `G.g_xyicon[9]`/`G.g_xytext[5]` — drag-outline polygon templates (set
  up once by `app_start()`, see `deskapp.md`)
- `gl_rfull` — full-screen clip rectangle (AES global, contract)

**System variables / globals written:**
- `G.g_screen[obj].ob_state` — the `SELECTED` bit, by `act_chg`/
  `act_allchg`
- `FNODE.f_selected` — kept in lockstep with the object's `SELECTED` bit
  via `G.g_screeninfo[obj].fnptr` (`act_chg:447-449`,
  `act_allchg:519-521`)
- `ANODE.a_xspot`/`a_yspot` — updated by `move_drvicon` after a desktop
  icon drag

**Build configuration affecting this function:** none — no
`#if CONF_WITH_*` gating anywhere in `deskact.c`.

---

## Source extraction summary

`deskact.c` implements the desktop's entire mouse-driven object-selection
and drag-and-drop UI, independent of whether the objects live on the
desktop (`root==DROOT`) or inside a window (`root==pw->w_root`) — every
function is parameterized by `wh`/`root` rather than hard-coding either
case, which is why the same code drives both desktop-icon dragging and
in-window icon/file dragging. There are two entry points, both invoked
from `deskmain.c`'s button-event handler:

- **`act_bsclick`** handles a completed click (single or double) that
  did **not** result in a drag: it implements the four selection rules
  from the header comment (shift-click-on-nothing does nothing;
  click-on-nothing-or-outside-window deselects everything; plain click
  on an object deselects-all-then-selects-it; shift-click toggles just
  that object).
- **`act_bdown`** handles the case where the button is *held down* after
  the initial press: if the press landed on empty space, this becomes a
  rubber-band selection rectangle (`graf_rubbox`, an AES primitive);
  if it landed on an already-selected object, this becomes a drag of
  every currently-selected object as a group.

Both ultimately call **`act_chg`**/**`act_allchg`** to mutate the
`SELECTED` bit (and the paired `FNODE.f_selected`) and trigger a clipped
redraw via `do_wredraw`.

---

## `act_chkobj` — icon vs. text hit-testing

`deskact.c:374-406`. Given a candidate object (already found by
`objc_find`'s bounding-box test), refines the hit test based on the
current view mode:

- **`V_TEXT`** (or always, when `root==DROOT`? — no: `view =
  (root==DROOT) ? V_ICON : G.g_iview`, `:383` — the desktop is **always**
  treated as icon view for hit-testing, regardless of `G.g_iview`, since
  the desktop has no text-view mode): tests against a rectangle
  `LEN_FNODE * gl_wchar` wide (`LEN_FNODE`=48, `deskwin.h:36`) — the full
  text-line width, not just the name portion.
- **`V_ICON`**: tests against the icon's **own two sub-rectangles**
  (`ib_xicon`/`ib_wicon`/`ib_hicon` for the glyph, `ib_xtext`/`ib_wtext`/
  `ib_htext` for the label). **Corrected 2026-07-21** (re-validation):
  the previous wording here said these are read from "that specific
  object's `ICONBLK` copy (`u.icon.block`/`u.icon.index`)" — that
  overstates it. The code (`:395-396`) only reads `.u.icon.index` to
  look up `ib = &G.g_iblist[icon]` — i.e. it consults the **shared
  master icon-block template array** `G.g_iblist[]`, not the per-object
  `.u.icon.block` copy that `_data-model.md` documents as existing
  alongside it. In practice this makes no observable difference for
  `ib_xicon`/`ib_xtext` (the master and every object's copy share the
  same glyph-shape geometry — only fields like `ib_char`/`ib_ptext`
  legitimately diverge per instance), but a NewTOS reimplementation
  should know the hit-test reads the **shared template**, not each
  object's own copy — **not** the object's overall bounding box either
  way. A click in the gap between the icon glyph and its label (if any)
  misses and returns `root` (i.e. "no object hit here"). The two
  sub-rectangle tests are OR'd (`:397-401`): intersects text-box **or**
  icon-box, checked in that order (text first).

This function is why a desktop with widely-spaced icon cells (icons
smaller than their grid cell) doesn't select an icon when clicking in
the dead space around it — the hit box is the icon's *own* pixel
footprint from its `ICONBLK`, not the grid cell.

---

## `act_chg`/`act_allchg` — selection state mutation

- **`act_chg(wh, root, obj, pc, set, dodraw)`** (`:419-466`): single-object
  version. **Refuses to change a `DISABLED` object** (`:429-430` —
  returns `FALSE` immediately, no state change, no redraw). Computes the
  object's absolute rectangle (root-relative `ob_x/y` plus the root
  object's own screen position), flips `SELECTED` per `set`, updates the
  paired `FNODE.f_selected` if this object has one
  (`G.g_screeninfo[obj].fnptr`), and — only if the state actually
  changed **and** `dodraw` **and** the rectangle intersects the caller's
  clip `pc` — calls `do_wredraw(wh, &t)`.
- **`act_allchg(wh, root, ex_obj, pt, pc, set)`** (`:480-552`): bulk
  version, used both for rubber-band selection (`set=TRUE`, `pt` = the
  rubber-band rectangle) and for "deselect everything" (`set=FALSE`,
  `pt` = `gl_rfull`, the whole screen). When **clearing**
  (`set==FALSE`), first calls `pn_clear(win_find(wh))` to reset **every**
  `FNODE.f_selected` in the target window — not just the visible ones —
  since a window can have more files than currently fit on screen
  (`:490-491`, explicit comment: "do all FNODEs, not just the visible
  ones"). Then walks every child of `root` except `ex_obj` (the object
  to exclude — used so a click-and-hold that becomes a drag doesn't
  re-select the object already under the cursor), testing intersection
  with the (clip-intersected) target rectangle `w` **and** requiring
  `act_chkobj` to actually hit the icon/text sub-rectangle, not just the
  bounding box (`:510-511` — this is why a rubber-band that only grazes
  an icon's outer margin, without touching its glyph or label, does not
  select it). Accumulates the union of all changed objects' rectangles
  into `a`, and issues a **single** `do_wredraw` call at the end covering
  that accumulated union (`:548-550`) — unlike `act_chg`'s per-object
  redraw, a bulk rubber-band selection redraws once for the whole
  affected region, not once per newly-selected icon.

**Edge case in `act_allchg`'s clear path** (`:540-544`): when
`set==FALSE` and an object does *not* intersect `w`/fails the hit test,
its `SELECTED` bit is **still explicitly cleared** (`tree[obj].ob_state
&= ~SELECTED`) even though it wasn't part of the accumulated redraw
rectangle `a` — this is because a "deselect all" pass must guarantee
every object ends up unselected in the tree, regardless of whether it
was previously selected (a no-op state write for already-unselected
objects, but not skipped).

---

## `act_bsclick` — click semantics

`deskact.c:566-603`. Implements exactly the four numbered rules from the
function's own header comment:

1. Shift-click on empty space within the window/desktop → no-op
   (`obj==root && shifted` → return immediately, `:577-578`).
2. Click (shifted or not) on empty space, **or** any click landing
   outside a window entirely (`obj==NIL`) → deselect everything
   (`act_allchg(…, gl_rfull, pc, FALSE)`).
3. Plain click on an object: if it's a **double-click**, or the object
   wasn't already selected, deselect everything then select just this
   object. **If it was a single click on an already-selected object**,
   this branch is skipped entirely (`:590` — `if (dclick ||
   !(state&SELECTED))`) — i.e. **a single click on an already-selected
   object changes nothing** (neither deselecting others nor re-selecting
   this one) — this is deliberate: it lets `act_bdown`'s caller (which
   runs *before* `act_bsclick` in the button-handling sequence, per
   `deskmain.c`, not re-derived here) initiate a drag of a multi-object
   selection without `act_bsclick` collapsing it down to one object
   first.
4. Shift-click on an object: toggle just that object's `SELECTED` bit
   (`state ^= SELECTED`), regardless of double-click.

---

## `act_bdown` — button-held: rubber-band or drag

`deskact.c:609-690`. Called while the button remains down after the
initial press (i.e. before `act_bsclick` would even apply — a "click" is
only a click if the button comes back up quickly without moving past a
threshold, which is `graf_rubbox`'s/`evnt_button`'s job, not this
file's).

1. **Hit-test the press point** (`gr_obfind`).
2. **If nothing was hit** (`sobj==root || sobj==NIL`): rubber-band mode.
   `graf_rubbox(l_mx, l_my, SHRT_MIN, SHRT_MIN, &l_mw, &l_mh)` — an AES
   call that tracks the mouse and returns the final drag rectangle's
   width/height (negative if dragged left/up) — the code then normalizes
   negative width/height into an adjusted origin+positive size
   (`:632-641`), and calls `act_allchg(…, TRUE)` to select everything the
   final rectangle covers.
3. **If an already-selected object was hit** (`tree[sobj].ob_state &
   SELECTED`): drag mode. Collects every selected object's position
   (`gr_accobs`), and if there's at least one, chooses the drag-outline
   point template (`G.g_xyicon`/`NUM_ICON_POINTS`=9 for icon view,
   `G.g_xytext`/`NUM_TEXT_POINTS`=5 for text view — **and, again, the
   desktop root is always treated as icon view here too**, `:652`), then
   runs the full interactive drag (`gr_drgplns`, [below](#gr_drgplns--the-drag-loop)).
   After the drag completes:
   - **If it landed on a destination window/object and that's the same
     object it started on, in the same window** (`wh==dst_wh &&
     *pdobj==sobj`) — treat as **cancelled**, not a self-drop
     (`:668-670`).
   - **Else if it landed nowhere** (`dst_wh` still `0`/falsy) **and** the
     source was the desktop **and** the drop point resolved back to
     `root` (empty desktop space) — this is a **desktop-icon
     rearrangement**, not a copy/move: calls `move_drvicon()` directly
     (snap to grid, update `ANODE`, redraw) and sets `dst_wh=NIL` so the
     caller (`deskmain.c`'s button handler) does **not** additionally
     invoke `fun_drag` (see [Edge cases](#edge-cases-discovered-in-source)).
   - Frees the point array (`dos_free(obpts)`).
4. **If the object hit was *not* already selected**: no rubber-band, no
   drag — falls through doing nothing (the comment-free `else` at
   `:645-682` only has a body for the "was selected" branch; an
   unselected single object under a held-down button is handled by
   `act_bsclick` once the button comes back up, not here).
5. Finally, `evnt_button(0x01,0x01,0x00,&l_mx,&l_my,&button,keystate)`
   drains the button-up event so the caller's next `evnt_multi` doesn't
   immediately re-see this same release.

**Return value**: `dst_wh` — the destination window handle the drag
ended on (`NIL` for "no drag happened" or "handled internally via
`move_drvicon`", a real handle or `DESKWH` if the caller should now
invoke `fun_drag`). `*pdobj` is set to the destination object index;
`*in_mx`/`*in_my` are updated to the drag's final offset-adjusted
position.

---

## `gr_drgplns` — the drag loop

`deskact.c:251-362`. The interactive per-frame drag loop, driving the
polygon-outline "ghost" that follows the mouse:

1. Computes the bounding box of a single polygon instance (`gr_extent`
   over `xylnpts`) and the overall extent of all dragged objects'
   original positions (`gr_extent` over `xyobpts`), then **re-centers**
   the per-object point list around the combined origin
   (`gr_obalign`) — this produces a single reference frame the whole
   drag operates in.
2. **Per-frame loop** (`do…while(down)`, renumbered 2026-07-21 — this
   list previously skipped from "1." to "3." with no "2." at all, a
   numbering slip, not a content error): compute the candidate new
   origin (`l_mx/l_my` minus fixed offsets), constrain it to stay within
   `pc` (the allowed drag bounds, typically the full screen —
   `rc_constrain`), then `gr_bwait()` draws the outline at the *old*
   position, blocks until the mouse moves at least 2px or the button
   changes (`gr_isdown`), then erases the old outline — i.e. **the
   outline is drawn in XOR mode and must be redrawn to erase**, so every
   frame does draw-wait-erase, and the *next* iteration draws again at
   the new position. This is a classic XOR-rubber-band animation, not a
   double-buffered one.
3. **Per-frame destination tracking** (`:294-350`): re-reads the mouse
   position/button/keystate (`graf_mkstate`), resolves which window (if
   any) the mouse is now over (`wind_find`), and finds the object under
   the cursor in that window's tree (`gr_obfind`). If the object under
   the cursor **changed** since the last frame, un-highlights the
   previous candidate (`act_chg(…, FALSE, TRUE)`) before highlighting the
   new one — but only once it is re-resolved as *not currently selected*
   (`:335-336`, `if (!(obj->ob_state & SELECTED))` — an object that is
   itself part of the dragged group is never highlighted as a drop
   target, added 2026-07-21 for completeness). **Only objects that
   resolve to a valid `ANODE`/`FNODE` via `i_find`, and are not a plain
   (non-executable) file, get highlighted** (`:336-349` —
   `if ((pa->a_type==AT_ISFILE) && !isapp) continue;` — you cannot "drop
   onto" a plain data file as a highlighted target, only onto
   folders/disks/trash/printer/applications).
4. Loop continues `while(down)` (i.e. while the button remains held).
5. On exit, un-highlights any still-highlighted candidate, passes back
   the final mouse position, offsets, destination window and object, and
   calls `desk_busy_off()` (the drag itself runs with the busy/hourglass
   cursor implicitly off — `desk_busy_on()` is **not** called by this
   function, only `_off` at the end, meaning the caller is responsible
   for any busy-cursor bracketing before the drag starts, if desired).

---

## Edge cases discovered in source

- **A single click on an already-selected object is a no-op in
  `act_bsclick`** (`deskact.c:590`) — this is *only* correct because
  `act_bdown` runs first (button-down before button-up) and handles the
  "drag the existing multi-selection" case; if a NewTOS reimplementation
  changes the button-event ordering, this rule alone would appear to be
  a bug (clicking a selected icon "does nothing").
- **The desktop root is hard-wired to icon-view hit-testing and
  drag-outline shape**, independent of `G.g_iview`, in **three separate
  places** (`act_chkobj:383`, `act_bdown:652`, and implicitly
  `move_drvicon`'s caller) — `G.g_iview` only ever affects *window*
  content. A reimplementation must not accidentally let a global "text
  view" setting affect desktop-icon hit-testing.
- **Rearranging desktop icons bypasses `fun_drag` entirely**
  (`act_bdown:673-677`) — `move_drvicon` is called directly from
  `deskact.c`, not routed through `deskfun.c`'s drag dispatch tree at
  all. This means desktop-icon-to-empty-desktop-space is handled by a
  completely different code path than desktop-icon-to-another-icon
  (which *does* go through `fun_drag`/`fun_desk2desk`).
  A NewTOS reimplementation must preserve this split, or verify the
  merged path produces identical results (grid snap, `ANODE` position
  update, no filesystem interaction).
- **Cancelling a self-drop**: dropping a dragged object back onto
  itself, in the same window, is explicitly detected and treated as "no
  drop happened" (`:668-670`), distinct from "dropped on empty space in
  the same window" (which *would* trigger `move_drvicon` if on the
  desktop, or nothing if in a regular window — window icons aren't
  freely repositionable the way desktop icons are, per the absence of an
  equivalent "rearrange window icon" call here).
- **`act_allchg`'s clear path always writes `~SELECTED` to every
  non-matching object**, even ones that were never selected
  (`:540-544`) — a harmless but unconditional write, worth noting for a
  reimplementation that might want to skip unchanged objects for
  performance (must still produce the same *final* state, just perhaps
  fewer redundant writes).
- **`gr_drgplns` never calls `desk_busy_on()`**, only `desk_busy_off()`
  at the very end (`:361`) — the hourglass cursor state during a drag is
  the caller's responsibility, if used at all.

---

## Error conditions

`deskact.c` has essentially no "error conditions" in the GEMDOS-error
sense — it's pure input/selection-state logic. The closest equivalents:

| Condition | Source location | Result |
|---|---|---|
| `gr_accobs`'s point-array allocation is capped by available memory | `deskact.c:99-101` | Silently truncates the number of tracked drag points to whatever `dos_avail_anyram()/sizeof(Point)` allows — **not** an error, no alert; a drag with more selected objects than fit in available memory drops the excess objects from the *visual* drag outline (their positions past the cap are simply not included) while `move_drvicon`/`fun_drag` still act on all selected objects via other means (since `gr_accobs`'s array is only used for the outline animation, not the actual selected-object enumeration, which `move_drvicon`/`fun_desk2desk`/etc. redo independently via `tree[obj].ob_state&SELECTED` walks) |
| Attempt to change the `SELECTED` state of a `DISABLED` object | `deskact.c:429-430` | `act_chg` returns `FALSE`, no state change, no redraw |

---

## Verification criteria

- **Shift-click on empty space**: selection state before and after must
  be bit-identical (no `do_wredraw` call either, since `act_bsclick`
  returns before calling anything).
- **Plain click on unselected object**: exactly one object ends up
  `SELECTED`; every previously-selected object (if any) ends up
  deselected; exactly one `do_wredraw` covering the accumulated
  changed-rectangle union.
- **Shift-click on a selected object**: only that object's `SELECTED`
  bit flips (off); all other objects' selection state is unchanged.
- **Rubber-band selecting an icon whose bounding cell overlaps the
  rectangle but whose glyph/label does not**: must **not** select that
  icon (`act_chkobj`'s sub-rectangle test inside `act_allchg`).
- **Drag-cancel-on-self**: dragging a selected icon and releasing it
  over its own original position, in the same window, must leave
  selection and position both unchanged (`act_bdown:668-670`).

---

## Dependencies

### Requires (must be initialized / functional)
- `docs/api/desktop/_data-model.md` — `G.g_screen[]`/`g_screeninfo[]`,
  `OBJECT.ob_state` `SELECTED`/`DISABLED` bits
- [`desksupp-redraw.md`](desksupp-redraw.md) — `do_wredraw`
- [`deskins.md`](deskins.md) — `snap_icon` (via `move_drvicon`)
- AES: `objc_find`, `evnt_multi`/`evnt_button`, `graf_mouse`/
  `graf_mkstate`/`graf_rubbox`, `wind_find` (all contract, `docs/api/aes/`)

### Used by
- `deskmain.c`'s `hndl_button()` — the sole caller of `act_bsclick`/
  `act_bdown` (out of scope for this task; see `deskmain.md`)
- The caller of `act_bdown` is also responsible for invoking
  `deskfun.c`'s `fun_drag()` when `act_bdown`'s return value indicates an
  actual cross-window/cross-icon drop occurred (not handled internally
  via `move_drvicon`) — cited in `deskmain.md`'s helper list
  (`deskact.c:566`/`:609`)

## Related data structures

- `docs/api/desktop/_data-model.md` — `G.g_screen[]`/`g_screeninfo[]`,
  `FNODE.f_selected`, `ANODE.a_xspot`/`a_yspot`
- `docs/api/structures/object.md` — `OBJECT.ob_state` bits
- `docs/api/structures/iconblk.md` — `ib_xicon`/`ib_xtext` sub-rectangles

## Cross-references

Not independently cross-checked against Compendium/Profibuch — mouse
interaction rules are not part of the documented GEMDOS/AES/VDI
trap-level contract; this is desktop-application UX behaviour, per
`docs/COMPATIBILITY.md`'s "EmuTOS is the reference, not the template."

## Configuration / version notes

No `#if CONF_WITH_*` gating anywhere in `deskact.c`. No TOS-version-
specific behaviour identified (EmuDesk-specific file).

---

## NewTOS implementation notes

### `gr_drgplns` — the drag loop, and the target highlight (DESK36, 2026-09-24)

Until DESK36 NewTOS's desktop did not run this loop at all: `act_bdown`'s drag
branch called the **AES** `graf_dragbox` (71), which runs its own loop, draws
one outline box and returns only the final position. The consequence was the
defect DESK36 reports — the object under the pointer never inverted, because
the loop that has to call `act_chg` was inside the AES, and the AES knows
nothing about `ANODE`s. (`graf_dragbox` was never right here on EmuTOS's own
terms either: `gembind.md` records that the desktop's binding does not contain
it, `aesbind.h:103-105`.)

The loop now lives where `deskact.c` has it, in
`programs/desktop/src/main.rs` — `gr_drgplns`, `gr_bwait`, `gr_isdown`, `gr_plns`,
`gr_extent`, `gr_obalign`, `rc_constrain` — next to `gr_obfind` and
`act_bdown`, which were already there because they need the AES trap wrappers.
The pure halves stay in `programs/desktop/src/act.rs`: `gr_accobs` (the origins of
the selected objects) and `DeskState::act_chg`/`obj_extent`; `act_chg_draw` in
`main.rs` is the `dodraw` half, `do_wredraw(wh, rc_intersect(pc, extent))`.

The two outline point lists are `G.g_xyicon`/`G.g_xytext`, built by
`app_start` (`deskapp.c:988-1011`) and now carried in `DeskState`;
`DRAG_BOX_WIDTH` is `2 + LEN_ZNODE + 1 + LEN_ZEXT` = 14.

**The desktop needed a VDI binding for this.** `gr_plns` draws the outline
with `gsx_pline` → `gsx_xline`, i.e. `vsl_udsty` + `v_pline` per segment in
`MD_XOR` (`aes/gemgraf.c:140-182`), on the workstation EmuTOS's desktop
*shares* with the AES — it may, because the ROM desktop is linked into the AES.
NewTOS's desktop reaches the system only through traps, so
`programs/desktop/src/gsx.rs` makes the same VDI calls over the VDI trap, on
the handle `graf_handle` reports (`deskmain.c:1956`, `gl_handle`): the AES's
own physical workstation, opened at session start
(`tos_api::aes::gsxif::gsx_init`, EmuTOS `gsx_wsopen`,
`aes/gemgsxif.c:169-186`), whose `vsl_type(7)` + `vsl_udsty(0xffff)` set-up
is the AES's (`gsx_start`, `aes/gemgraf.c:420-422`). *Correction (K1,
2026-10-07):* until then NewTOS's AES opened no workstation (handle 1 was
never open), and the desktop opened its own virtual one — a named deviation
that, from the first drag on, kept virtual handle 2 taken for the rest of the
session, so a program started afterwards got handle 3 from its first
`v_opnvwk`; both are gone. `gr_bwait` re-establishes the clip and the write mode
on every pass, which is what EmuTOS's own comment at `deskact.c:211-214`
demands anyway. One further collapse: `gsx_attr`'s `gl_mode`/`gl_lcolor` cache
(`:205-240`) is not reproduced — it saves VDI calls and leaves the workstation
in the same state either way.

`act_bdown`'s `evnt_button(0x01, 0x01, 0x00, …)` (`:681`), which drains the
release the drag ended on, was also missing and is now taken.

**Measured** (`measurements/measurement-2026-09-24-desk37-desk36-desk30/`, one gesture,
`measurements/measurement-2026-09-24-desk37-desk36-desk30/method/d36.actions`): a plain
 file dragged over four targets in turn.

| picture | target | highlight |
|---|---|---|
| `d36-after-C-prog.png` | `QED.NAP`, an installed application | **yes** |
| `d36-after-D-folder.png` | the `SUB` folder | **yes** |
| `d36-after-E-file.png` | `README.TXT`, a plain file | **no** |
| `d36-after-F-trash.png` | the desktop trash | **yes** |
| `d36-after-G-back.png` | the dragged item itself (already `SELECTED`) | not touched |
| `d36-after-H-released.png` | after the release | all gone |

Each `d36-before-*` counterpart shows one box outline and no highlight at all.
The XOR erase is exact: `d36-after-H-released.png` differs from
`d36-after-B-win1.png` (taken before the drag started) only inside the info
line and the source item's own cell — bounding box `(58,70)-(383,211)` — so
the drag left no residue anywhere on the desktop background or on any other
item.
