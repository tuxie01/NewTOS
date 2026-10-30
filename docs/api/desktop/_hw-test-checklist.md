# Desktop — Real-Hardware Interaction Test Checklist (Task #7/#8)

> **Why this file exists.** Interactive input is **not** drivable in QEMU for the
> desktop build: `qemu raspi4b` has the SD/EMMC storage (so `C:` and directory
> windows exist) but no wired-up mouse/keyboard input path, and `qemu-virt` has
> input but no `C:` drive. Per the project decision of 2026-07-04, all
> **interactive** verification of the desktop happens on **real Pi4 hardware**.
>
> The static/source audit (Round 2, Audit #10) verified the desk-side *logic*
> against the EmuTOS extraction and drove every non-interactive check it could
> (compile both boards, clippy, a QMP boot screendump on `qemu raspi4b` with a
> representative SD image). This checklist is the **primary verification
> instrument** for everything that needs a live pointer/keyboard.
>
> **Status (2026-07-15).** The bottom-up TOS-API re-audit (BIOS→XBIOS→GEMDOS→
> VDI→AES) landed on `desk-rebuild` through commit `9ae7edf` — see
> `docs/STATUS.md` Recent Decisions 2026-07-15 for the full commit chain
> (`fb8c443`…`9ae7edf`) and `docs/api/_discrepancies.md` D-084..D-102. A real-HW
> pass on 2026-07-12 (user, `kernel8.img` @ `916168e`) already confirmed clean
> boot, working menus/dropdowns, working dialog boxes, opening drives, and
> copying a file — see **"Real-HW Validation Log"** below for the exact rows
> this covers. The three later commits (`5d33238`/`74b79c8`/`9ae7edf`) are
> committed and QEMU/screendump-verified but **not yet flashed** to the user's
> card; a fresh `kernel8.img` @ `9ae7edf` is built and waiting. **New section I**
> at the end of this file adds explicit checks for the AES rendering/behaviour
> fixes in that delta, with honest reachability notes (some fixed paths are not
> yet wired into any desktop menu action and need a direct AES-trap probe, not
> normal desktop use, to observe).
>
> **How to run:** boot the current `kernel8.img` on the Pi4 with the 4×500 MB
> FAT16 SD card (C–F) attached (see [[sd-card-real-hw-layout]]). Use a directory
> with **≥ 20 entries incl. sub-folders and 12-char 8.3 names** for the
> adversarial rows. Tick each row; note the actual result next to it. A row that
> fails is a defect — file it against the desk module.
>
> Legend: **[E]** = expected result. **[A #n]** = watch here for audit finding
> #n (see the Round-2 audit report). Rows marked **(residue)** are the
> historically fragile paths. **[PASS yyyy-mm-dd @ hash]** = confirmed on real
> Pi4 hardware at that commit.

---

## Real-HW Validation Log

**2026-07-12, `kernel8.img` @ commit `916168e` (user, real Pi 4).** Coarse-
grained pass, not a row-by-row run of this checklist — recorded here as the
broad categories the user explicitly confirmed working; the granular rows
below (individual click/drag/menu-state variants) are **not** individually
re-verified by this pass and stay open until ticked on HW:

- **Boot** — clean boot to the desktop, no panic. Supersedes/reconfirms the
  QEMU-screendump-only boot check under "What the Round-2 audit already
  confirmed" below, now on real hardware.
- **Menus** — the menu bar's drop-downs open and items dispatch on a real
  pointer. Covers the coarse behaviour underlying section **C** (Menu manager);
  the per-item graying/checkmark rows (C2–C9) were not individually confirmed
  and remain open.
- **Dialog boxes** — `form_alert`-based dialogs (the Desk→Info about box, **C1**;
  delete-confirm alerts, **G7/G13/G17**) render and respond. **Known
  exception, still present:** the **#66 menu-bar-border bug** — dismissing the
  Desk→Info alert leaves a residue at the menu-bar edge, because `form_alert`'s
  screen save/restore depends on a memory-MFDB `vro_cpyfm` copy (**#51
  `bb_save`**, deferred — VDI has no off-screen MFDB backend yet, see
  `docs/api/aes/object-form.md:824-850` / `docs/api/_discrepancies.md` D-090,
  D-092). Expect this artifact until #51 lands; it is **not** part of the AES
  re-audit's scope and is not newly introduced by it.
- **Opening drives** — double-click a drive icon opens a window at that
  drive's root. Matches row **A7**.
- **Copying a file** — a file copy operation completed successfully. The exact
  interactive mechanism (drag-onto-folder same-window vs. cross-window vs.
  onto-a-drive-icon) was not specified in the report; treated here as
  confirming the **G5**-class path (drag a file icon onto a folder, same
  window) at the coarse level. The Ctrl-held-move variant and the other G.2
  drag targets (G6/G8/G9/G10/G11/G12) are **not** individually confirmed and
  stay open.

Of the checklist rows below: **A7** is ticked `[x]` (a clean 1:1 match — "opening
drives works"). **C1** and **G5** carry a `[PASS 2026-07-12 @ 916168e]` note but
stay **unticked**, because each row bundles more than the coarse report
confirmed (C1's residue-free-dismiss clause is known-still-broken; G5's
Ctrl-move variant is unconfirmed) — see the per-row notes. "Boot" and "Menus"
have no single 1:1 checklist row and are recorded only here. Every other row
is unticked and still needs a live run.

---

## A. Button / click manager (`hndl_button`, D3)

- [ ] **A1 — Single-click an icon in the top window** → the icon inverts
  (selected); every other icon in that window de-selects. **[E]** exactly one
  item highlighted.
- [ ] **A2 — Single-click empty work-area** in the top window → all icons in
  that window de-select, nothing else changes.
- [ ] **A3 — Open a second window** (double-click a folder), then **click an
  icon in the now-*background* window** (the one behind, still visible) → that
  background window becomes the current window **and** the clicked icon is
  selected (the click is *consumed*, not ignored). **[E]** background window is
  now current; item selected. *This is the core D3 fix.*
- [ ] **A4 — (was audit #1, now FIXED in #9/F1) Two windows, select an icon in
  window 1, then click an icon (or empty area) in window 2** → **window 1's
  selection clears**; only window 2 shows a highlighted item. **[E per EmuTOS]**
  only ONE window ever shows a selection. The `desk_clear_all` on `wh != g_cwin`
  (deskmain.c:957-958) was added in #9 — confirm the correct single-window
  behaviour on HW.
- [ ] **A5 — Single-click a desktop drive icon** (outside any window) → the
  drive icon selects; open windows de-select as appropriate; menu updates.
- [ ] **A6 — Double-click a folder icon** → a new directory window opens showing
  that folder's contents, staggered from the parent, titled `X:\SUB\`.
- [x] **A7 — Double-click a drive icon** → a window opens at that drive's root
  (`X:\`), listing its contents. **[PASS 2026-07-12 @ 916168e]** ("opening
  drives works" — see Real-HW Validation Log above).
- [ ] **A8 — Double-click a plain file** → **nothing happens** (launch is Task
  #9; must be an honest no-op, not a crash).
- [ ] **A9 — Click in the menu bar area / on the closer/fuller/mover/sizer** →
  routed as the corresponding `WM_*` (see sections D/E), not as a work-area
  click.

## B. Keyboard manager (`hndl_kbd`, D4)

- [ ] **B1 — Down-arrow / Up-arrow** with a scrollable top window → content
  scrolls one line; selection in that window clears first. **[E]** no
  `men_update` re-gray on arrow (EmuTOS quirk — Open/Delete stay as they were).
- [ ] **B2 — Up-arrow at the very top / Down-arrow at the very bottom**
  (boundary) → no scroll; **note:** any icons that were selected get
  *internally* de-selected but the screen is **not** repainted until the next
  redraw (faithful EmuTOS quirk — verify it matches, not a NewTOS-only bug).
- [ ] **B3 — Left/Right-arrow** with `Size to fit` **off** and a horizontally
  scrollable window → horizontal scroll by one column. (With `Size to fit`
  *on* — the default — there is no horizontal scroll; arrows are inert
  horizontally.)
- [ ] **B4 — Enter/Return (CR)** → de-selects **all** icons in all windows +
  desktop. **[E]** no menu re-gray (CR bypasses `men_update`, EmuTOS quirk):
  Open/Delete may remain enabled though nothing is selected.
- [ ] **B5 — ESC** → the top window re-scans its directory and repaints (use it
  after changing the SD contents to confirm a fresh scan).
- [ ] **B6 — Ctrl-O with exactly one item selected** → opens it (same as File →
  Open). With **zero** or **≥2** selected → Ctrl-O is **inert** (item disabled).
- [ ] **B7 — Ctrl-H (Close folder)** on a sub-folder window → navigates up one
  level; on a drive-root window → closes the window.
- [ ] **B8 — Ctrl-U (Close top window)** → closes the top window entirely.
- [ ] **B9 — Ctrl-W (Cycle)** with **≥2** windows → the bottom-most window comes
  to the top. With **1** window → inert (disabled).
- [ ] **B10 — Disabled-item shortcuts are inert:** Ctrl-D (Delete), Ctrl-A
  (Select-all), Ctrl-Z (EmuCON), Ctrl-R (Change-rez), Ctrl-S (Save-desktop),
  Ctrl-I (Info), Ctrl-N (New-folder) → **nothing happens** (all are Task-#9
  items kept disabled; the shortcut must not dispatch a disabled handler).
- [ ] **B11 — F1…F20** → no-op (no installed applications yet — Task #9).
- [ ] **B12 — Alt-A…Alt-Z** (fixed in #13: the keymap folds Alt+letter → ASCII 0
  and the Alt bit is now in `shift_state`) → opens that drive letter's root
  window (e.g. Alt-C → `C:\`). Confirm the correct drive opens (and no crash).
  **Driven in QEMU raspi4b 2026-09-23** (DESK13,
  `measurements/measurement-2026-09-23-desk13/screendumps/b-altc.png`): Alt-C opens `C:\`.
  The claim that this path was inert on the current keyboard layer — carried in
  `check_alt_letter_key`'s own comment and copied into `TASK/DESK11-STATUS-DONE.MD`
  — was **wrong**, and both copies are corrected. What is left for the Pi is a
  real keyboard (the QEMU run uses the IKBD injector) and a drive other than `C:`.

## C. Menu manager + `men_update` (D6)

*(Open each drop-down with the pointer and read the item states.)*

- [ ] **C1 — Desk menu** → `Desktop info...` enabled; selecting it shows the
  About alert; dismissing it repaints the screen underneath cleanly **(residue)**.
  **Partial [PASS 2026-07-12 @ 916168e]:** the menu opens and the alert
  renders/dismisses (part of "menus"/"dialog boxes work" in the Real-HW
  Validation Log). **Known-open defect, expected still present on the fresh
  kernel (`9ae7edf`) too:** the **#66 menu-bar-border bug** — the repaint after
  dismissing this specific alert is *not* clean at the menu-bar edge, because
  `form_alert` has no screen save/restore (blocked on **#51 `bb_save`**, a VDI
  memory-MFDB `vro_cpyfm` gap — see `docs/api/aes/object-form.md:824-850`,
  D-090/D-092). This is a pre-existing gap, not something the AES re-audit
  introduced or was scoped to fix — do not re-file it as a new regression.
- [ ] **C2 — View menu checkmarks** → exactly one of `Show as icons` /
  `Show as text` is checked (Icon at boot); exactly one sort key checked
  (`Sort by name` at boot); `Size to fit` checked at boot. Switch each and
  re-open the menu — the checkmark must follow.
- [ ] **C3 — File menu, no window open, nothing selected** → `Open`, `Close
  folder`, `Close top window`, `Cycle windows` are **grey**; `Info/Rename`,
  `New folder`, `Select all`, `Set file mask`, `Delete`, `Execute EmuCON`,
  `Shutdown`, `Search` are **grey** (Task #9).
- [ ] **C4 — File menu, one window open, nothing selected** → `Close folder`
  and `Close top window` become **enabled**; `Open` still grey; `Cycle` still
  grey (needs ≥2).
- [ ] **C5 — File menu, exactly one icon selected** → `Open` **enabled**.
- [ ] **C6 — File menu, two+ icons selected** (Task #9 multi-select may be
  unavailable; if reachable) → `Open` **grey** again (can't open two at once).
- [ ] **C7 — File menu, two+ windows open** → `Cycle windows` **enabled**.
- [ ] **C8 — Options menu** → **every** item grey (all Task #9): `Set
  background`, `Install icon/application/devices`, `Remove desktop icon`, `Set
  preferences`, `Desktop configuration`, `Change resolution`, `Read .INF`,
  `Save desktop`. (Present-but-disabled, **not** clickable no-ops.)
- [ ] **C9 — Separators** in every drop-down render as grey dash lines and are
  never selectable.
- [ ] **C10 — Menu title flash:** selecting any item (mouse or shortcut) flashes
  the title highlighted then returns it to normal; the drop-down closes cleanly
  with **no residue** on the desktop underneath **(residue)**.

## D. View / sort (`do_viewmenu` → `desk_all`)

- [ ] **D1 — View → Show as text** → the top window switches to the text list
  (indicator / NAME.EXT / size / date / time columns); **no stale icons left
  behind (residue)**. Switch back to icons — same, clean.
- [x] **D2 — (residue, the headline symptom) Two overlapping windows, switch
  View→text then View→icons** → **neither** window shows leftover pixels from
  the previous view in its own work area, and the covered part of the lower
  window is **not** repainted over the upper window. *This is the D2/R2 residue
  case that must be structurally gone.*
  **PASSED on real Pi 4, 2026-09-11** (1920×1080, kernel with FIX-01…06):
  overlapping windows, focus switched between them, moved over each other —
  no residue, no overpainting, no error. Logged in `docs/HW-TEST-LOG.md`.
- [ ] **D3 — Sort by size** → files listed **largest first**; folders always
  precede files. (On the test SD: LYNXBOOT.IMG / KERNEL8.IMG should lead the
  files.)
- [ ] **D4 — Sort by type** → grouped by extension; **extension-less files sort
  first** (before any dotted name), right after the folders.
- [ ] **D5 — Sort by date** → **newest first**; folders first.
- [ ] **D6 — No sort** → raw directory (scan) order; **folders and files
  interleave** (no folder-first grouping).
- [ ] **D7 — Sort by name** (default) → folders first alphabetically, then files
  alphabetically. *(Statically + boot-screendump confirmed already; re-confirm
  interactively.)*
- [ ] **D8 — Size to fit toggle off** → a horizontal scrollbar becomes
  meaningful; the H-slider thumb now **renders** (fixed in #12, a mirror of the
  vertical slider). Confirm the thumb reflects the horizontal position and that
  Left/Right arrows scroll by a column. **Re-check on the fresh kernel
  (`9ae7edf`):** the AES Window/Graphics audit (`0493a85`, findings L-2/L-4)
  changed the underlying slider-size default (`-1` "full window" marker,
  previously clamped to `1000`) and the drag-rounding (`mul_div_round` instead
  of a truncating `mul_div`, D-098) — thumb size/position should be at least as
  correct as before, ideally with less "creep" during a drag.

## E. Window management (`hndl_msg`: TOPPED/MOVED/SIZED/FULLED/CLOSED/ARROWED/SLID)

- [ ] **E1 — Click a background window's title/body → it comes to the top**
  (frame drawn on top, content repainted, occlusion of the now-lower window
  correct).
- [ ] **E2 — Drag the mover (title bar)** → window moves; on release the content
  re-settles with no residue at the old position **(residue)**; items stay
  correctly placed relative to the new work-area origin.
- [ ] **E3 — Drag the sizer** → window resizes; the icon/text grid **re-flows**
  into the new column count; **shrinking** to fewer columns re-lays the grid
  (no clipped half-column left over) **(residue)**.
- [ ] **E4 — Click the fuller** → toggles full-screen extent ↔ previous size;
  grid rebuilds for each; the window is topped by the full action.
- [ ] **E5 — Click the closer / File→Close folder / WM_CLOSED** on a sub-folder
  → goes up one directory level; on a drive-root window → closes it entirely.
- [ ] **E6 — Scroll arrows (up/dn/lf/rt gadgets)** → scroll one line/column;
  content repaints cleanly; the vertical slider thumb tracks the position.
- [ ] **E7 — Drag the vertical slider** → jumps to the proportional scroll
  position; content correct; **at the extreme top/bottom** the last row is fully
  backed (no blank strip below the last item). **Re-check on the fresh kernel
  (`9ae7edf`):** same `graf_slidebox` rounding fix as **D8** (`0493a85`
  L-4/D-098) — the grip should now stay under the cursor at the offset it was
  grabbed at, instead of snapping the elevator's edge to the cursor.
- [ ] **E8 — (was audit #3, fixed in #17/F3) Any full repaint
  (open/close/move/size/top/cycle, or dismissing the About box)** → the pointer
  should stay hidden for the whole repaint and reappear **once**, cleanly (the
  nested `graf_mouse` M_OFF/M_ON was collapsed to a single outer bracket in
  `full_redraw`/`win_shwall`). Confirm **no** mid-repaint cursor flicker and
  **no** persistent cursor droppings (holes/ghosts).
- [ ] **E9 — Open the maximum number of windows (7), each on a full directory,
  then close and reopen several times** → every window keeps showing its full
  content (no items silently missing). *This exercises the lazy
  `obj_wfree`-on-reuse arena path (audit finding #4, proven benign at 640×480 —
  confirm it holds on HW).*

## F. Adversarial / boundary rows (drive these deliberately)

- [ ] **F1 — Directory of 12-char 8.3 names** (`KEYBOARD.KBD`, `LYNXBOOT.IMG`)
  in icon view → labels do **not** overlap horizontally between columns, and do
  not bleed into the neighbour cell. *(Boot-screendump confirmed clean; re-check
  a denser directory.)*
- [ ] **F2 — Full grid** (more items than fit) → the fringe row/column shows a
  partial sliver (deliberate `+1`), the vertical slider thumb shrinks
  proportionally, scrolling reveals the rest with no gaps/dupes.
- [ ] **F3 — Empty directory** → window opens, shows `0 items   0 bytes used`,
  no phantom icons, scrollbars full-range/inert.
- [ ] **F4 — Directory with > 64 entries** → **all** entries listed and scrollable
  (no silent 64-cap; the rebuild dropped the old fixed buffer — confirm on a
  large real folder, D9).
- [ ] **F5 — Scroll to the exact bottom, then resize larger** → the scroll
  position re-clamps so no blank area appears below the last row (win_ocalc
  decrement-only re-clamp).
- [ ] **F6 — View-switch while scrolled past row 1** → the scroll position stays
  coherent between icon and text views (single virtual-grid scroll model, D5) —
  the same files remain in view, not a units-mismatch jump.

## G. Task-#9 selection, drag & file operations (`act.rs` / `dir.rs`)

> **Why here.** The **backend** (`d_doop`/`dir_op`) is proven programmatically
> (`desk::dir::selftest`, run once against a fresh `/tmp/sd.img` on `qemu
> raspi4b`: Dcreate / Fcreate+Fwrite / Copy / Rename / recursive Delete / Count,
> each confirmed by a dir rescan — 12/12 pass, image intact + test folder
> cleaned up afterwards). Everything below needs a **live pointer/keyboard** and
> so is real-HW only. **Test exclusively against a fresh
> `tools/make-sd-image.sh /tmp/sd.img` — these rows mutate the image.**

### G.1 Selection (`act_bsclick` / `act_chkobj` — D7 hit-test)

- [ ] **G1 — Single-click an icon's glyph or its label** → selects it; the dead
  space *between* glyph and label (icon view) is a **miss** (`act_chkobj`
  sub-rectangle hit-test, not the bounding cell). Every other item de-selects.
- [ ] **G2 — Shift-click a second icon** → **adds** it to the selection (both
  highlighted); shift-click a selected icon → **removes** just it; shift-click
  empty space → no-op.
- [ ] **G3 — Single-click an already-selected icon (no shift)** → **no change**
  (does *not* collapse a multi-selection) — this is what lets a held button then
  drag the whole group (`deskact.c:590`).
- [ ] **G4 — Rubber-band: press on empty work area and drag a box** → every item
  whose glyph/label the box touches is selected; an item the box only grazes in
  its margin (not the glyph/label) is **not** selected.

### G.2 Drag & drop (`act_bdown` → `fun_drag` dispatch tree)

- [ ] **G5 — Drag a file icon onto a folder icon (same window)** → the file is
  **copied** into that folder; the folder window (if open) rebuilds. Hold **Ctrl**
  while dragging → **moved** instead (source disappears). **Partial
  [PASS 2026-07-12 @ 916168e]:** "copying a file works" was confirmed at the
  coarse level (Real-HW Validation Log above) and is treated as covering this
  row's plain-copy path; the **Ctrl-held-move** variant of this row was **not**
  separately confirmed — leave it open until explicitly re-tested.
- [ ] **G6 — Drag a selection from one window into another window** → copied to
  the target window's directory (Ctrl = move); the source selection clears; both
  windows rebuild. A **folder** Ctrl-moved this way keeps a correct `..` entry
  (fixed in #15 — spot-check the moved folder still opens and its parent chain is
  intact; `fsck` clean).
- [ ] **G7 — Drag a window selection onto the desktop Trash icon** → a confirm
  alert appears; OK **deletes** the files; rebuild the source window.
- [ ] **G8 — Drag a window selection onto a desktop Disk icon** → copied to that
  drive's root (Ctrl = move).
- [ ] **G9 — Drag desktop Disk C onto Disk D** → C's whole contents copied to
  D:\ (heavy — use the small test image). Drop onto Trash → confirm + delete.
- [ ] **G10 — (move_drvicon, bypasses fun_drag) Drag a desktop drive icon to an
  empty desktop cell** → the icon snaps to the nearest grid cell and stays there;
  **no** file operation happens, **no** confirm. Position now **persists** via
  Options→Save desktop (Task #14, see H4/H5).
- [ ] **G11 — Drag an icon and release it back on itself (same window)** →
  treated as a **cancelled** drop; nothing copies/moves.
- [ ] **G12 — Drag a window item onto empty desktop space** → "Can't drop items
  here" alert (desktop shortcuts are Task #14).

### G.3 File-menu actions (menu wired via `men_update`)

- [ ] **G13 — Select one/more items, File→Delete (or Ctrl-D)** → confirm alert;
  OK deletes them; window rebuilds; dir rescan shows them gone. Delete is
  **grey** with nothing selected. This alert is `form_alert`-based — see **I1**
  for the fresh 3D-bevel edge-click check on its OK/Cancel buttons.
- [ ] **G14 — File→New folder (a window open)** → name-entry dialog; type a name,
  OK → `Dcreate`; the folder appears in the window. Empty name / Cancel → no-op,
  screen restored. Grey with **no** window open.
- [ ] **G15 — Select exactly one item, File→Info/Rename (Ctrl-I)** → the
  **Show-Info** dialog (`inf_file_folder`, Task #14): shows the item's
  size/date/time/attributes with an editable name; a changed name on OK →
  `Frename`, window rebuilds; Cancel / no change → screen restored. Grey unless
  exactly one item is selected. **[E]** size = byte count for a file, `(folder)`
  for a directory; date = `MM/DD/YYYY  HH:MM`; attributes list `DIR`/`R`/`H`/
  `S`/`A` as set.
- [ ] **G16 — File→Select all (a window open)** → every item in the current
  window highlights; menu state updates (Open greys at >1 selected). Grey with
  no window open.
- [ ] **G17 — Delete a desktop drive icon (desktop is current window)** → the
  three-choice alert **Cancel / Remove / Delete**; "Remove" must route to
  Options→Remove-desktop-icon (icon removal, files untouched — the handler is
  Task #14, so it is currently a no-op redirect); "Delete" deletes the drive's
  contents (dangerous — only on the throw-away test image). Also
  `form_alert`-based — see **I1/I2**.

## H. NEWDESK.INF read/save + Show-Info / Set-background (Task #14, deskinf)

> **Non-interactive parts already verified on `qemu raspi4b`** (serial log,
> `/tmp/sd.img`): boot **reads** a hand-crafted `C:\NEWDESK.INF` and applies
> `#E` view/sort/size-to-fit, `#Q` background (`g_patcol`), `#M`/`#T`
> desktop-icon grid positions, and the `#W` window rectangle; the
> `inf::selftest()` save→read round-trip passed 11/11 (in-memory **and** through
> the SD image). The rows below are the **interactive** dialogs, which need a
> live pointer/keyboard. Run only against a **fresh** `/tmp/sd.img` (writes
> mutate the image).

- [ ] **H1 — Boot with a `C:\NEWDESK.INF` present** → the desktop comes up with
  the file's saved view/sort (check the View menu check-marks), background
  pattern/colour, and desktop drive/trash icon positions. **[E]** menu checks +
  icon grid spots match the file.
- [ ] **H2 — Boot with NO `NEWDESK.INF`** → the desktop uses its built-in
  defaults (Icon view, name sort, drives down column 0, trash/printer at the
  right). **[E]** no error, clean boot.
- [ ] **H3 — Options→Set background** → the pattern/colour stepper dialog opens;
  `-`/`+` change the Pattern (0-7) and Colour (0-15) numbers; **OK** repaints the
  desktop with the new background **and** writes `NEWDESK.INF`; **Cancel** leaves
  it unchanged. **[E]** desktop background visibly changes on OK; re-boot keeps
  it (persisted).
- [ ] **H4 — Options→Save desktop** → writes `C:\NEWDESK.INF` from the current
  state (view/sort/background/desktop-icon positions/open windows). **[E]** no
  alert on success; an "`Could not save…`" alert only on a write failure.
- [ ] **H5 — Move a drive icon (G10), Options→Save desktop, then Options→Read
  .INF (or reboot)** → the icon returns to the saved grid cell. **[E]** the
  moved position round-trips through the file.
- [ ] **H6 — Change view (icon↔text) / sort, Options→Save desktop, reboot** →
  the desktop boots into the saved view/sort. **[E]** View-menu checks match
  what was saved.
- [ ] **H7 — Options→Read .INF file** (after an external edit or a Save) →
  re-applies the on-disk `NEWDESK.INF`: view/sort re-checked, background
  repainted, desktop icons re-placed, open windows re-laid. A missing file is a
  silent no-op.

---

## I. Fresh-kernel regression checks (commit `9ae7edf` — AES re-audit delta since the `916168e` HW pass)

> The 2026-07-12 HW pass validated `916168e`. Three more AES re-audit commits
> landed since (`5d33238` Resource/Fileselector, `74b79c8` Scrap/Shell,
> `9ae7edf` Object/Form rendering — see `docs/STATUS.md` 2026-07-15 for the full
> summary). This section adds targeted checks for that delta, split into what
> is genuinely reachable through the current desktop UI (**I.1**) and what
> landed in the code but has **no** desktop-UI path to reach yet (**I.2**) — so
> a tester does not spend time hunting for a menu action that does not exist.

### I.1 Reachable through normal desktop use (`form_alert`-based dialogs: About box **C1**, delete-confirm **G7/G13/G17**)

- [ ] **I1 — 3D button bevel + edge clicks on an alert's OK/Cancel button**
  (`9ae7edf`, `find_hit` hit-test fix, EmuTOS `ob_find` gemoblib.c:1063-1072) →
  click at the very edge of the visibly-drawn raised bevel of an alert's
  button (not just its nominal flat rect, on all four sides) → the click still
  registers. Previously a click on the bevel pixels just outside the nominal
  object rect could miss.
- [ ] **I2 — Alert box double-frame (`OUTLINED`) renders unchanged** (`9ae7edf`
  `draw_signed_border`, D-089) → the alert's outer box still shows the classic
  GEM double frame exactly as before. This is a **regression check**, not a new
  behaviour: every current desktop/alert box uses a positive border thickness,
  and the signed-`th` fix is defined to leave that path bit-for-bit unchanged
  (confirmed by screendump before/after in `9ae7edf`). A visible difference
  here would itself be a *new* bug, not the expected fix.
- [ ] **I3 — `form_alert` `|`/`]` escaping** (`f0b30de`, `gemfmalt.c
  isduplicate`) — not currently reachable: no desk-built alert string uses
  `||`/`]]`. Regression-safe by construction; no interactive row possible until
  a desk alert string needs the escape.
- [ ] **I4 — Disabled dialog objects don't fire** (`f0b30de` F1, `run_form`
  now tests `ob_state & DISABLED` instead of the colliding `ob_flags` bit) —
  no desk-built dialog `OBJECT` is currently marked DISABLED (the Options-menu
  greying in **C8** is a *menu*-level disable, a different code path). Not
  reachable through the desktop UI today; regression-only.

### I.2 Landed in code, NOT yet reachable through the desktop UI

> Do **not** file a defect for these unless you are testing them through a
> direct AES-trap probe (e.g. a selftest binary calling the opcode) — normal
> desktop interaction cannot currently reach them.

- **`form_center` placement** (`9ae7edf`, `gl_hbox`/menu-bar offset fix) — no
  desk-internal dialog calls the AES `form_center` trap (opcode 54).
  `form_alert` (`tos-api/src/aes/form.rs:195-196`), the file selector's
  `center()` (`tos-api/src/aes/fsel.rs:534-541`), and `programs/desktop/src/inf.rs`'s
  hand-built Show-Info/Set-background dialogs (`:1040-1041,1098-1099`) each
  compute their own full-screen centring independently and do not call the
  fixed function. Confirming this fix needs a probe that invokes opcode 54
  directly.
- **File selector empty-path → current-drive root** (`5d33238` L-5,
  `gemfslib.c:634-638`) — `fsel_input`/`fsel_exinput` (opcodes 90/91) are not
  called from anywhere in `programs/desktop/src/` yet (no File→Open/Install picker
  is wired — a Task #9/#14 item). Needs a direct AES-trap probe.
- **Resource loader `ob_type` mask, `rsrc_free` return contract, shared-TEDINFO
  relocation dedup** (`5d33238` M-1/M-2/L-4) — internal `.RSC`-loader
  correctness fixes with no distinguishing visual symptom on the current
  `GEM.RSC`; own coverage was the commit's QEMU selftest (all asserts passing,
  round-trip unregressed).
- **Shell manager fixes** (`74b79c8`: `shel_get`/`put` 4192-byte buffer,
  `shel_envrn` NULL-on-miss, `shel_rdef`/`wdef` own storage, `shel_write`
  `doex`/`isgem` dispatch, `shel_find` current-drive root) — `programs/desktop/src/`
  calls no `shel_*`/`scrp_*` AES entry point yet; this is forward
  infrastructure for the not-yet-wired shell-loop↔`Pexec` launch path (see
  `docs/STATUS.md` Blocked, the F5/F7 note). Nothing to test on the desktop
  yet.

---

## What the Round-2 audit already confirmed (do NOT re-derive on HW)

Compile clean on **both** boards, zero clippy/compiler warnings under
`programs/desktop/src/`, no stubs/`todo!`/`unwrap`/panics. Boot on `qemu raspi4b`
(QMP screendump, `/tmp/sd.img`): clean boot (no panic), menu bar (4 titles),
`C:\` window content, correctly occluded desktop drive icon, **default view =
Icon**, **folder-first name sort**, info-line format, **no label overlap at
12-char names**, frame intact, no boot residue. Shortcut table (11 entries)
matches `deskmain.c:269-307`; `men_update` live-item gating (Open/Close/Cycle)
matches `ILL_NOSEL`/`ILL_MULTSEL`/`ILL_NOWIN`/`BTOPITEM`; Task-#9 items disabled
(not stubbed); the self-clearing white item-root box erases the old view within
the `do_wredraw` clip. These are the static/screendump-verified items — the HW
rows above cover only what needs a live pointer/keyboard.

## J. Cooperative kernel — EL0 upcall/signal path (coop-kernel step 1, commit `ee82ead`)

QEMU-TCG is cache-coherent and blind to the QUIRK-001 class (TTBR0 wedge /
I-cache-PoU coherence on real Cortex-A72). The upcall path exercises exactly
that surface, so it needs a **real Pi4** pass before the mechanism counts as
hardware-solid:

- **J1 — Trampoline I-cache coherence:** the kernel writes the 5-instruction
  stub at EL0-VA `0x700` at load time (`loader.rs` `write_upcall_trampoline`,
  in the cache-maintenance window). On real HW the EL0 instruction fetch of
  `0x700` must see the freshly-written bytes (PoU coherence). QEMU: no-op.
- **J2 — TTBR0 switch on the upcall/`Psigreturn` path:** `deliver_upcall` +
  `handle_psigreturn` redirect the frame and resume under the process's TTBR0
  (log shows "TTBR0 restored"). Validate no QUIRK-001-style wedge on a real
  handler round-trip — trigger once a real syscall-boundary consumer (etv_term /
  G_USERDEF, step 2) lands: an app installs a handler, it fires, resumes cleanly.
- The step-1 selftest (`selftest_upcall`, all 6 checks PASS in QEMU) is
  synthetic; J1/J2 are the residual HW-only confirmations.
- **J3 — Two-process context switch (coop-kernel step 3b, `59b60d4`):** the S2
  restartable switch (`apply_switch`/`save_yield_frame`) does a mid-syscall yield
  deep in `evnt_multi` + a per-process `TTBR0` swap + an ELR-rewound `svc`
  re-execution + deliver-on-resume. `selftest_sleepwake_switch` passes all 7 checks
  in QEMU, but that path has only run synthetically. QUIRK-001 is a QEMU-TCG
  artifact (real PIPT caches don't alias), so QEMU-green is not itself a silicon
  risk — but as a precaution, do a real-Pi4 smoke of **two interacting GEM
  processes** (a real `evnt_mesag` block + a peer's reply that wakes it) once step 4
  makes multiple threads live.

---

## Section K — full-screen application launch and return

K1/K2 used to be driven by the bouncing-ball demo, a full-screen LP64 `.NAP`
that took the whole screen and quit on a key. The demo left this tree on
2026-09-09 (`/Volumes/Daten/Development/ball-demo/`), and with it the only
application that exercised this path end to end. **The two desktop checks it
carried are still open and still belong here:**

- **K1 — Launch by double-click.** Double-click a full-screen `.NAP` on the
  desktop. Expect: the desktop disappears, the application owns the screen.
  (The AES shell's `#Z` autorun line is the other real mechanism, not a test
  hook.)
- **K2 — Quit, screen left clean.** Expect: the mouse pointer is back, the
  desktop is repainted whole, and the serial log shows `exited with rc=0`.
  The `plat_close`-shaped sequence a GEM program runs on the way out
  (`Vsync` → `Setscreen(page0, page0)` → `graf_mouse(M_ON)` → `v_clsvwk` →
  `appl_exit`) is what has to leave the screen usable.

Neither can run in QEMU `raspi4b`: it has no keyboard or mouse path — a
serial byte never reaches the key queue on this board (since BIOS09 it goes to
AUX, `Iorec(0)`; `kernel/src/console.rs:170@serial_rx_byte`), CON is the
USB-HID keyboard. Pick a replacement full-screen application (QED and
TeraDesk are in the tree) or rebuild the demo from its new home.
