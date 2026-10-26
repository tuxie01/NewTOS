# Current desktop — defect catalog vs. the EmuTOS `desk/` reference

> Evidence base for **ADR-016** (`docs/MEMORY.md`) — the ground-up desktop
> rebuild decision. Produced by the Researcher role per the team-lead's
> Task #2, Part B. Method: read the **entire** current implementation —
> kernel/src/desktop.rs (2330 lines) and kernel/src/desktop_icons.rs
> (442 lines; both removed in ee84dd42) — line by line, then compared each mechanism against the
> matching EmuTOS `desk/` source (primarily `deskmain.c`, `desksupp.c`,
> `deskwin.c`, `deskapp.c`, `deskfpd.c`) and, where relevant, the AES layer
> the desktop sits on (`tos-api/src/aes/window.rs`, which **already**
> implements the occlusion-aware rectangle list EmuTOS expects — see D1
> below). Every finding below cites **both** sides: the current NewTOS line
> and the EmuTOS (or tos-api) line it diverges from.
>
> **Contract vs. internal** (`docs/COMPATIBILITY.md`): none of this is
> TOS-API trap-ABI contract — a conforming *program* cannot observe how the
> desktop *application* redraws itself. The measuring stick here is
> **ADR-016's own charter**: "EmuTOS-faithful reimplementation" of desktop
> *behaviour* (window semantics, menu semantics, keyboard semantics) as
> observed by the **user**, not byte-identical internals. Every defect below
> is scored by how far the *observable* desktop behaviour (what a user
> sees/can do) diverges from EmuTOS, not by how different the Rust code
> looks from the C.
>
> **STOP-flag on "residue":** for the specific symptom "view switch
> (text↔icon) leaves a residuum," I could not identify a single-window,
> non-overlapping-window code path that leaves stale pixels — the
> erase-then-redraw sequence (`draw_dir_listing`,
> cite stale 2026-10-26: kernel/src/desktop.rs:2060-2105) unconditionally whites out the **entire current work rect**
> before redrawing, for both view modes. My best-evidenced explanation is
> that the symptom is an **instance of D2** (no occlusion-aware clipping —
> see below): the residue appears whenever the switched window overlaps
> another window or a just-closed menu/dialog at the moment of the switch.
> D5 (scroll-unit mismatch) is a second, independently-provable defect in
> the same area that could also present as "wrong content after a view
> switch," though it does not leave stale pixels by itself. Flagging this
> per CLAUDE.md Rule 3 rather than asserting a mechanism I could not fully
> confirm by reading code alone; a screen-recorded repro would settle which
> of D2/D5 is the dominant cause.

---

## Ranked defect list

### D1 — CRITICAL — Icon-view label box: shrink-to-fit + wrong font metric (the 8.3 label-overlap bug)

**What NewTOS does:**
- `label_box(chars, cell_w)` (cite stale 2026-10-26: kernel/src/desktop.rs:505-509) computes the
  label's pixel width as `tw = (chars*cw + 4).clamp(cw, cell_w)` — **it
  shrinks the label box to the actual name length** and centres it, rather
  than always using the full cell width. The function's own doc comment
  states this is a **deliberate deviation**: *"the `gr_gicon` label
  background fills `ib_wtext`, so sizing it to the text (rather than the
  whole cell) keeps the white box tight under short names... EmuTOS uses a
  fixed `MAX_ICONTEXT_WIDTH` box, which reads as an over-long white bar on
  NewTOS' coloured background"* (cite stale 2026-10-26: kernel/src/desktop.rs:499-504).
- For the **longest** 8.3 names (12 characters — exactly the test data the
  team added in commit `4c47566`, "populate with representative 12-char
  8.3 filenames"), `chars=12, cw=8` ⇒ `12*8+4 = 100`, which the `.clamp(cw,
  cell_w)` **clamps down to `cell_w`** (96, `ICON_LABEL_W = 12 *
  tos_api::vdi::font::CELL_W`, (cite stale 2026-10-26: kernel/src/desktop.rs:2198)). The "+4 px
  margin" the comment promises is **silently discarded exactly for the
  longest names** — the one case that most needs it — leaving `ib_xtext =
  (96-96)/2 = 0`: **zero margin** between the label box and its own
  96-px-wide cell edge. The only remaining breathing room is
  `ICON_GAP_W = 8` (cite stale 2026-10-26: kernel/src/desktop.rs:2199) between *cells*, not
  within one.
- The width unit itself (`tos_api::vdi::font::CELL_W = 8`,
  `tos-api/src/vdi/font.rs:11`) is a single, hardcoded, resolution- and
  font-independent constant — there is no distinct "small font" metric in
  the NewTOS VDI font module at all.

**What EmuTOS does:**
- `deskapp.c:519` — `ib_wtext = MAX_ICONTEXT_WIDTH * gl_wschar` — **always**
  the full `MAX_ICONTEXT_WIDTH=12`-character width (`deskapp.c:74`), for
  every icon, regardless of the actual displayed name's length. There is no
  shrink-to-fit; short names simply leave unused space in a
  fixed-size box.
- `deskapp.c:810` — `G.g_wicon = (MAX_ICONTEXT_WIDTH * gl_wschar) + (2 *
  G.g_iblist[0].ib_xtext)` — the margin (`ib_xtext`) comes from the
  **resource-baked** `ICONBLK` (`icon.rsc`), not a runtime centring
  computation, and is **added on top of**, not carved out of, the
  12-character text width — so the margin never disappears for long names.
- Critically, `gl_wschar`/`gl_hschar` (`aes/gemgraf.c:43-44`, "*width of
  character cell (small font)*") is a **separate font metric from the
  standard system font**, computed once via `vst_height(gl_ws.ws_chminh,
  ...)` (`aes/gemgraf.c:407`) — GEM icon labels are drawn in a distinct,
  dynamically-probed small font, not whatever the system font happens to
  be.

**Why this explains the observed bug:** the prior fix (commit
`af426a0`, "faithful EmuTOS icon-grid metrics") corrected the **grid
pitch** (`ICON_STEP_W`/`ICON_CH`) to match EmuTOS's `g_iwspc`/`g_ihspc`
formulas, but the **label-box width formula itself** remains a NewTOS-only
invention that degenerates to a zero-margin box for exactly the class of
name the team's own test image now exercises (12-char 8.3 names, commit
`4c47566`) — explaining why label overlap reappeared on real hardware even
after the grid-metric fix landed the same day (ADR-016 was written the
same day as `af426a0`). The zero-margin-at-maximum-length case is the
textbook "off-by-one at the boundary" the task explicitly asked to look
for.

**Severity rationale:** directly reproduces the exact bug ADR-016 lists
first; root cause is precisely identified and independently double-caused
(shrink-to-fit clamp *and* a hardcoded-vs-probed font metric), so a patch
would need to fix two independent things to match EmuTOS, not one.

---

### D2 — CRITICAL — Redraw has no occlusion-aware damage-rect list; "handle order, full-screen clip" only works for exactly 2 windows

**What NewTOS does:**
- `redraw_win(handle)` (cite stale 2026-10-26: kernel/src/desktop.rs:1255-1269) →
  `draw_dir_listing(...)` (`:2060-2105`) erases and repaints using **the
  window's own tracked rect**, clipped only against
  `ClipRect::full(backend.width(), backend.height())` (`:2064`) — i.e. **no
  clip restriction from any other window or the menu bar**.
- `redraw_all_windows()` (`:1887-1897`) is the *only* place multiple
  windows are reconciled: it redraws every **non-top** window handle in
  **ascending numeric order** (`for h in 1..8`), then the tracked
  `DESK_TOP` handle **last**. The function's own comment admits the limit:
  *"Full z-ordered, rectangle-list clipping per window is a follow-up;
  handle order suffices until windows deeply overlap"*
  (cite stale 2026-10-26: kernel/src/desktop.rs:1884-1886).
- There is exactly **one** tracked "top" handle (`DESK_TOP`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:1356)) — no ordered list of the other N-1 windows'
  relative stacking. Two non-top windows that overlap each other are
  therefore drawn in **handle-number order**, which has no necessary
  relationship to which one a user last interacted with.
- `WM_REDRAW`'s own message payload — the AES-supplied damage rectangle at
  `msg[4..8]` — is **read only for the handle** (`msg[3]`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:839), `WM_REDRAW => redraw_win(msg[3])`); the
  rectangle itself is **never consulted**.

**What EmuTOS does:**
- `win_top()` (`deskwin.c:218-240`) maintains a genuine **ordered window
  list** (`G.g_wfirst`/`w_next`, the topped window always moved to the
  front) — the true stacking order for **every** window, not just "top vs.
  rest".
- `do_wredraw(w_handle, gptr)` (`desksupp.c:202-232`) intersects **two**
  things per repaint: the caller-supplied damage rect (`gptr`, e.g. the
  exact `WM_REDRAW` rectangle from `deskmain.c:1279`) **and**, per
  iteration, the AES's own **occlusion-aware visible-rectangle list**
  for that window handle (`wind_get_grect(w_handle, WF_FIRSTXYWH)` /
  `WF_NEXTXYWH`, `desksupp.c:223-229`) — the set of on-screen rectangles
  belonging to that window **after every window on top of it has already
  been subtracted out**. Only the intersection is painted. **Draw order is
  irrelevant** to correctness under this model: a window that is 95%
  covered repaints at most the visible 5%, regardless of when it's
  redrawn relative to its neighbours.
- `win_shwall()` (`deskwin.c:805-819`) sends one `fun_msg(WM_REDRAW, wh,
  x, y, w, h)` **per open window**, each with **that window's own current
  rect** — the desktop never repaints "all windows" as an undifferentiated
  batch; each redraw is independently, exactly scoped.

**Confirmed available but unused:** `tos-api` (the AES layer beneath the
desktop) **already implements** the real mechanism —
`tos-api/src/aes/window.rs:54-55` (`WF_FIRSTXYWH`/`WF_NEXTXYWH` constants),
`:264-310` (the `RLIST` rectangle-subtraction machinery, a correct
occlusion-list implementation matching the AES contract) — so this is not
a missing capability, only an unused one. This directly corroborates
ADR-016's assessment: *"the root causes sit in the ad-hoc directory-window
subsystem... not in the AES/VDI foundation, which is EmuTOS-aligned."*

**Why this matters beyond the 3-named-defect list:** this single gap is
the shared root cause the task names explicitly (defect 3) **and** is the
most likely mechanism behind defect 2 ("view-switch residue" — see the
STOP-flag above): any redraw issued while windows or a menu overlap paints
blindly over pixels it doesn't occlusion-check, and — because there is no
per-window rectangle list — nothing guarantees the rightful owner of that
region redraws afterward unless it happens to be `DESK_TOP` (drawn last)
or the code separately calls `redraw_all_windows()` again.

**Severity rationale:** structural, not a local bug; explains 2 of the 3
originally-named defects and is explicitly self-documented as a known
limitation in the current code's own comments.

---

### D3 — CRITICAL — No per-click window resolution; all interaction hardwired to the single tracked "top" window

**What NewTOS does:**
- The button-event path in `step()` (cite stale 2026-10-26: kernel/src/desktop.rs:849-877) and
  every selection/action helper (`select_at`, `:1943-1962`;
  `resolve_selection`, `:1437-1453`; `delete_selected`, `open_selected`,
  `info_rename`, `size_to_fit`, `new_folder`, `close_top_window`) resolve
  the target window exclusively through the tracked `DESK_TOP` global
  (`:1356`) — **never** through the click's screen coordinates.
- `grep -n "wind_find\|desk_verify\|g_cwin\|g_croot\|desk_clear"
  kernel/src/desktop.rs` returns **zero matches** — none of EmuTOS's
  current-window state machine exists in the current code at all.
- `select_at(mx, my)` (`:1943-1962`) reads `DESK_TOP`'s rect and returns
  `false` (click not consumed) if `(mx,my)` falls outside it — so a click
  inside a **different, visible-but-not-topped** window's work area is
  simply **ignored** (falls through to `wait_release()`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:872-874)): no selection, no top change, no
  action.

**What EmuTOS does:**
- `hndl_button(clicks, mx, my, button, keystate)` (`deskmain.c:947-1009`)
  calls `wh = wind_find(mx, my)` (`:955`) **first, unconditionally** — the
  window under the click, determined from screen coordinates, independent
  of which window currently has focus.
- `desk_verify(wh, FALSE)` (`:960`) then updates `G.g_cwin`/`G.g_croot`/
  `G.g_wlastsel` to **that** window before any click/drag handling runs
  (`desksupp.c:175-198`) — every subsequent desktop operation (menu
  actions, `men_update`'s selection counting) operates against whichever
  window was actually clicked, not a separately-tracked "most recently
  topped" handle.
- If `wh != G.g_cwin` (clicking a **different** window than the one
  currently current), `desk_clear_all()` runs first (`:957-958`) — EmuTOS
  explicitly handles the "user clicked into a different window" case as
  its own transition.

**Why this matters:** in the current desktop, clicking to select an item
in a background (non-focused) window does nothing at all — a real,
easily reproduced interaction gap, not just a redraw cosmetic. It also
plausibly contributes to the "residue" symptom family: a window that never
becomes `G.g_cwin`-equivalent is never the trigger for any of the
click-driven redraw paths either.

**Severity rationale:** a basic GEM interaction (click any visible window
to make it current) is entirely absent; independent of D2's redraw
concerns.

---

### D4 — CRITICAL — Keyboard input is dead: `MU_KEYBD` requested, never dispatched

**What NewTOS does:**
- `step()` (cite stale 2026-10-26: kernel/src/desktop.rs:796-877) requests `MU_KEYBD |
  MU_BUTTON | MU_MESAG` from `evnt_multi` (`:801`), but the function body
  only branches on `fired & MU_MESAG` (`:809`) and `fired & MU_BUTTON`
  (`:849`) — **there is no `if fired & MU_KEYBD` branch anywhere in the
  file** (confirmed by grep: `MU_KEYBD`, `kret`, `hndl_kbd`,
  `check_arrow_key`, `check_function_key` do not otherwise appear in
  kernel/src/desktop.rs). The AES-returned keyboard character/scancode
  (what EmuTOS calls `kret`) is computed by the AES call but never read
  out of `a.intout[...]`.

**What EmuTOS does:**
- `hndl_kbd(thechar)` (`deskmain.c:1201-1247`) is a first-class event
  handler, dispatched every time `ev_which & MU_KEYBD` (`deskmain.c:2097`
  `-2099`): CR deselects all (`desk_clear_all`), ESC refreshes the topped
  window, arrow keys scroll the topped window
  (`check_arrow_key`/`kbd_arrow`, `:1043-1081`), F1-F20 launch the
  matching installed application (`check_function_key`/`process_funkey`,
  `:1087-1128`), Alt+A..Z opens that drive letter
  (`check_alt_letter_key`, `:1136-1160`), and any other control character
  is looked up as a menu Ctrl-shortcut (`lookup_ascii_shortcut`,
  `:1169-1198`) and dispatched through the **same** `hndl_menu` a mouse
  click would use.

**Why this matters:** this is a **complete feature-class loss**, not a
partial gap — none of EmuTOS's keyboard-driven desktop interaction exists
in the current build. A user (or an automated test driving the keyboard)
gets no response at all from any key press while the desktop is running.

**Severity rationale:** entire documented subsystem (`deskmain.c:1043-
1247`, ~200 lines of dispatch logic) has zero equivalent; trivially
verified by grep, not a subtle divergence.

---

### D5 — HIGH — View switch (icon↔text) silently reinterprets the scroll offset in different units

**What NewTOS does:**
- The per-window scroll offset (`WinInfo.scroll`,
  cite stale 2026-10-26: kernel/src/desktop.rs:1757-1759) is a single `i16`, shared verbatim between both view modes.
- In **text** mode, one scroll unit = one **file**:
  `let start = scroll as usize;` (`build_dir_text`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:2134)).
- In **icon** mode, one scroll unit = one **row of `cols` files**:
  `let start = scroll as usize * cols as usize;` (`build_dir_icons`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:2273); `find_entry_at` uses the identical `* cols`
  reinterpretation at `:2047-2048`).
- `set_view(mode)` (cite stale 2026-10-26: kernel/src/desktop.rs:1970-1974) flips
  `VIEW_MODE` and calls `redraw_all_windows()` — **it does not touch or
  convert any window's stored `scroll` value.** A window scrolled to, say,
  `scroll=8` in text mode (skip 8 files) is silently reinterpreted as
  `scroll=8` **rows** (skip `8*cols` files, e.g. 32 files at `cols=4`) the
  instant the user switches to icon view — an entirely different, and for
  most directories out-of-range, position (silently clamped by
  `draw_dir_listing`'s `scroll.max(0).min((total_rows-vis_rows).max(0))`,
  (cite stale 2026-10-26: kernel/src/desktop.rs:2075,2032), to *some* valid value, but not the
  visually-corresponding one).

**What EmuTOS does:**
- `win_bldview` (`deskwin.c:392-520`) builds both view types from the
  **same virtual grid** (`w_cvcol`/`w_cvrow`, the "current virtual
  column/row" — `win_ocalc`, cited at `deskwin.c:411`), recomputed
  consistently by `win_view()` (`deskwin.c:63-99`) for whichever view is
  active; there is one coherent scroll-position concept per window, not
  two incompatible unit systems.

**Severity rationale:** concretely reproducible (any window scrolled past
row 1, then a View-menu toggle), silent (no error, just wrong content),
falls precisely under the task's invited "off-by-one/metrics" category;
plausibly a contributing (if not the sole) cause of the reported
view-switch "residue" symptom (see the file-level STOP-flag above).

---

### D6 — HIGH — No `men_update` equivalent: menu items are never enabled/disabled/checked by desktop state

**What NewTOS does:**
- `DISABLED` (cite stale 2026-10-26: kernel/src/desktop.rs:33) is applied **exactly once**, at
  static menu-tree build time, to separator strings only
  (cite stale 2026-10-26: kernel/src/desktop.rs:730, inside `build_menu()`). Grepping the whole
  file for `DISABLED`/`ob_state` confirms **no other location ever
  toggles it** — there is no runtime graying logic at all. "Close top
  window" is clickable with zero windows open; "Delete" is clickable with
  nothing selected; "Install application" is clickable with no
  application-type icon selected; the View/Options checkmarks
  (`menu_icheck`-equivalent) exist for view/sort/size-to-fit
  (`set_view`/`set_sort`/`size_to_fit` do flip their own tree's checkmark
  implicitly via rebuilding `MENUS`'s static text — actually: **checked**
  by inspection, there is **no checkmark rendering at all** in the current
  `MENUS`/`build_menu` model; the item text itself is static).

**What EmuTOS does:**
- `men_update()` (`deskmain.c:464-609`) runs after **every** dispatched
  event except a bare `WM_REDRAW` (`deskmain.c:1357-1358`,
  `if (!menu) men_update();`) and recomputes, from current selection
  count/type and open-window count, which of `ILL_NOSEL`/`ILL_MULTSEL`/
  `ILL_NOWIN` items to disable, plus feature-detected items
  (`QUITITEM`/`RESITEM`/`BLITITEM`/`CACHITEM`) and their checkmarks
  (`menu_icheck` for `BLITITEM`/`CACHITEM`, and — in `do_viewmenu`/
  startup — for the view/sort/size-to-fit items).

**Severity rationale:** an entire, well-specified (~145-line) EmuTOS
mechanism has zero counterpart; while not a rendering bug per se, it is a
correctness/fidelity gap a user notices immediately (menu items that
should be grayed out never are) and squarely within "EmuTOS-faithful
reimplementation" per ADR-016's own charter.

---

### D7 — MEDIUM — Desktop-surface hit-testing duplicates rendering geometry in three independent places ("hit-test drift" risk)

**What NewTOS does:** the drive-icon/trash grid geometry (`icw=80,
ich=64, xcnt = (sw-20)/icw`, cell centring math) is written out
**independently, three times**:
1. `draw_desktop_icons` (cite stale 2026-10-26: kernel/src/desktop.rs:557-618, the renderer),
2. `drive_at` (cite stale 2026-10-26: kernel/src/desktop.rs:1662-1682, the drive-icon hit-test),
3. `trash_at` (cite stale 2026-10-26: kernel/src/desktop.rs:1708-1715, the trash hit-test).

Nothing enforces these three formulas stay in sync; the exact bug class
the recent `af426a0` grid-metric fix had to touch (rendering metrics) is
only *half* of the geometry — the hit-test copies were a separate edit
surface. (By contrast, the **directory-window** content hit-test,
`find_entry_at`, (cite stale 2026-10-26: kernel/src/desktop.rs:2021-2054), correctly rebuilds the
**same** object tree the renderer draws and hit-tests it via `objc_find`
— no drift risk there; this defect is scoped to the desktop-surface
fixtures only.)

**What EmuTOS does:** `app_blddesk()` builds each desktop icon **once**
into `G.g_screen[]` (with its real `ob_x`/`ob_y`/`ob_width`/`ob_height`);
both drawing (`objc_draw`) and hit-testing (`act_bdown`/`act_bsclick`,
`deskact.c:566-609`, via `objc_find`) read the **same** object array — a
single source of truth by construction, with no possibility of drift.

**Severity rationale:** not currently manifesting as a visible bug (the
three formulas do currently agree), but a structural landmine: any future
metric change (exactly what `af426a0` was) must be replicated correctly in
three unrelated functions or the drive icons silently stop being
clickable where they visually appear.

---

### D8 — MEDIUM — `WM_REDRAW`'s AES-supplied rectangle is parsed but discarded

**What NewTOS does:** `WM_REDRAW => redraw_win(msg[3])`
(cite stale 2026-10-26: kernel/src/desktop.rs:839) — only `msg[3]` (the handle) is used;
`msg[4..8]` (x, y, w, h — the exact damage rectangle the AES computed) is
never read.

**What EmuTOS does:** `case WM_REDRAW: ... do_wredraw(handle, (GRECT
*)&G.g_rmsg[4]);` (`deskmain.c:1277-1281`) passes that rectangle straight
through, and `do_wredraw` (`desksupp.c:202-232`) uses it as one half of
its double-clip (see D2).

**Severity rationale:** a specific instance/citation of D2, called out
separately because it is independently, trivially fixable evidence (one
ignored message field) distinct from the larger "no rectangle list"
architecture question.

---

### D9 — MEDIUM — Fixed 64-entry directory-listing buffers silently truncate large directories

**What NewTOS does:** every directory scan (`list_sorted`,
(cite stale 2026-10-26: kernel/src/desktop.rs:1979-1985); `resolve_selection`, `set_win_info`,
`find_entry_at`, `draw_dir_listing`) uses a **fixed-size stack array**
(`[tos_api::bdos::fs::find::DirInfo::default(); 64]`, `DIR_MAX: usize =
64`, (cite stale 2026-10-26: kernel/src/desktop.rs:2207)) — a directory with more than 64 entries
has its remainder **silently dropped** from every listing, sort, count,
and selection; there is no error, warning, or scroll-beyond-the-cap
behaviour.

**What EmuTOS does:** `FNODE`s are dynamically allocated
(`dos_alloc_anyram`) into a per-window linked list
(`w_pnode.p_flist`/`pn_sort`) sized to the actual directory content, up to
available memory — no fixed per-window entry cap.

**Severity rationale:** a real scalability/correctness gap (silently
incomplete listings), lower severity than D1-D4 because it only manifests
on directories exceeding 64 entries — plausible on a populated hard-disk
image but not on the small `SD-image` test fixtures currently in use.

---

### D10 — LOW — Menu drop-down geometry is an ad hoc guess, not EmuTOS's measure-and-shrink-to-fit algorithm

**What NewTOS does:** drop-down box width is `iw = maxlen * 8 + 2 * 8 + 6`
(cite stale 2026-10-26: kernel/src/desktop.rs:715-719) — a fixed padding constant with no
overflow handling if the computed width would exceed the screen.

**What EmuTOS does:** `adjust_menu()` (`deskmain.c:1511-1641`) measures
every title/item string against the actual system-font `CHAR_WIDTH`,
detects when the **title bar** itself would overflow screen width and
progressively shrinks it (half-character trim, then last-title
truncation, `:1550-1581`), force-byte-aligns drop-down `ob_x` "for faster
display" (`:1617`), and repositions any drop-down that would extend past
the right edge (`:1627-1631`). NewTOS's formula has no equivalent
overflow/reposition path.

**Severity rationale:** currently cosmetic only (NewTOS's own menu text is
short enough not to overflow at the target resolution); listed because the
task invited "further defects," and because it's the same *class* of gap
as D1 (a geometry formula invented instead of derived, with no boundary
handling) at lower present impact.

---

### D11 — LOW — No busy-cursor (`desk_busy_on`/`desk_busy_off`) equivalent

**What NewTOS does:** grepping kernel/src/desktop.rs for `busy`/
`HGLASS`/`graf_mouse` returns no matches — no operation (delete, save,
directory rebuild) shows a busy/hourglass cursor.

**What EmuTOS does:** `desk_busy_on()`/`desk_busy_off()`
(`desk/desksupp.c:1840-1852`, `graf_mouse(HGLASS/ARROW, NULL)`) bracket every
slow operation (`deskmain.c:440`/`446` in `desk_all`; `file_exists`,
`desksupp.c:360-370`; the startup sequence, `deskmain.c:1963-1964`,
`2074`).

**Severity rationale:** purely cosmetic; included for completeness of the
catalog, lowest priority to fix.

---

## Cross-reference: the three defects named in the task

| Named symptom | Primary defect(s) | Confidence |
|---|---|---|
| Icon-view 8.3 label overlap on real HW | **D1** | High — precise mechanism identified (shrink-to-fit clamp degenerating to zero margin at 12 chars) with a secondary contributing factor (hardcoded vs. probed font metric) |
| View-switch (text↔icon) leaves a residuum | **D2** (primary hypothesis), **D5** (independently provable, may compound) | Medium — mechanism plausible and well-evidenced from code, but not confirmed by execution; see the file-level STOP-flag above |
| Redraw repaints in handle order with full-screen clip instead of the window damage-rect list | **D2**, **D8** | High — directly observed in code; the correct mechanism (`WF_FIRSTXYWH`/`WF_NEXTXYWH`) already exists, unused, in `tos-api` |

## Additional defects found (beyond the three named)

D3 (no per-click window resolution), D4 (keyboard dead), D5 (scroll-unit
mismatch), D6 (no `men_update`), D7 (hit-test geometry duplication), D9
(fixed 64-entry truncation), D10 (menu geometry ad hoc), D11 (no busy
cursor).

## Open questions / STOP-flags for the team lead

1. The "residue" mechanism (see the file-level note above) — recommend a
   screen-recorded repro (single window, two windows overlapping, and a
   3-window-overlap case) to determine whether D2 alone explains it or
   whether a fourth mechanism exists that code-reading didn't surface.
2. Several `CONF_WITH_*` gates `deskmain.md` catalogued have **no recorded
   NewTOS decision** (`CONF_WITH_SEARCH`, `CONF_WITH_BOTTOMTOTOP`,
   `CONF_WITH_SELECTALL`, `CONF_WITH_FILEMASK`, `CONF_WITH_FORMAT`,
   `CONF_WITH_READ_INF`, `CONF_WITH_SIZE_TO_FIT`'s horizontal half,
   `CONF_WITH_ALT_DESKTOP_GRAPHICS`, `CONF_WITH_PCGEM`,
   `CONF_WITH_DESKTOP_SHORTCUTS`, `CONF_WITH_PRINTER_ICON`,
   `CONF_WITH_EASTER_EGG`) — these affect which menu items/behaviours the
   rebuilt `programs/desktop/src/` should even include; not decided here per
   CLAUDE.md Rule 3.
3. D1's secondary factor (hardcoded `CELL_W` vs. EmuTOS's dynamically
   probed `gl_wschar`/small-font metric) touches the **actual glyph
   renderer**, which lives outside the two files in this task's scope
   (`tos_api::aes::object`/VDI text drawing) — worth a follow-up read
   before the Developer starts D1's fix, to confirm the renderer's real
   per-glyph pixel width matches the `CELL_W=8` assumption on the target
   resolution/font.

---

## Conclusion (for ADR-016)

Every one of the three originally-named defects, plus eight further ones
found during the full read-through, traces to the **same architectural
class** ADR-016 already names: an ad hoc directory-window/desktop-surface
subsystem that re-derives geometry, redraw scope, and interaction state by
hand instead of routing through the already-EmuTOS-faithful AES primitives
(`wind_find`, `WF_FIRSTXYWH`/`WF_NEXTXYWH`, `objc_find`, `men_update`-style
state recomputation) that `tos-api` already implements correctly. This
supports the ground-up rebuild decision: the fixes are not independent
one-line patches but require re-deriving the window/file data model
(`FNODE`/`WNODE`-equivalent), the grid-metric globals (`win_view`), and the
damage-rect redraw (`do_wredraw`) faithfully, exactly as ADR-016's
rationale states.
