# `deskins.c` — icon/application install dialogs, `ANODE` lifecycle

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/deskins.c` (1412 lines). Per
> [`docs/api/desktop/_overview.md`](_overview.md)/[`_data-model.md`](_data-model.md):
> `ANODE` is defined there (rendering-relevant fields only); this entry
> documents the *installation* behaviour that creates/edits/removes
> `ANODE`s, which `_data-model.md` explicitly deferred ("full semantics
> … are in `docs/api/desktop.md`" — that file does **not** cover install
> dialogs either; this is the first extraction of `deskins.c`).
>
> **Contract vs. internal** (`docs/COMPATIBILITY.md`): entirely
> desktop-application-internal. No GEMDOS/AES/VDI trap takes an `ANODE*`.
> What a program *can* eventually observe is the resulting
> `EMUDESK.INF` file format (see [`deskinf.md`](deskinf.md)'s appendix)
> if NewTOS chooses to support reading real EMUDESK.INF files — out of
> scope unless that goal is adopted (flagged in `_data-model.md` already).

---

## EmuTOS source

**Primary locations (public, `deskins.h`):**
- `deskins.c:71-80` `is_installed(pa)`
- `deskins.c:87-111` `is_viewer(pa)` (`CONF_WITH_VIEWER_SUPPORT`)
- `deskins.c:290-339` `ins_devices(void)`
- `deskins.c:721-756` `ins_app(void)`
- `deskins.c:1234-1311` `ins_icon(sobj)`
- `deskins.c:1341-1411` `ins_shortcut(wh, mx, my)` (`CONF_WITH_DESKTOP_SHORTCUTS`)
- `deskins.c:1317-1335` `rmv_icon(sobj)`
- `deskins.c:146-184` `snap_icon(x, y, px, py, sxoff, syoff)`
- `deskins.c:989-1004` `is_executable(filename)` (`CONF_WITH_DESKTOP_SHORTCUTS
  || CONF_WITH_WINDOW_ICONS`)

**Installed handler / vector:** `(none)` — called from `deskmain.c`'s
`do_optnmenu()` (`IDSKITEM`/`IAPPITEM`/`IICNITEM`/`RICNITEM`, see
`deskmain.md`), from [`deskfun.md`](deskfun.md)'s `fun_drag`/
`fun_win2desk`/etc. (`ins_shortcut`), and from `deskmain.c`'s startup
sequence (`ins_devices`, once at boot — cross-check against `deskmain.md`
if it doesn't already cite this).

**Helpers called (this file, `static`):**
- `deskins.c:118-134` `grid_free(x,y)` — is a desktop grid cell occupied
  by another `AF_ISDESK` `ANODE`?
- `deskins.c:197-222` `ins_posdisk(dx, dy, pdx, pdy)` — scan rightward/
  downward from a starting grid cell for the first free one
- `deskins.c:230-255` `install_drive(drive)` / `:263-281`
  `remove_drive(drive)` — the two `ins_devices()` helpers
- `deskins.c:345-351` `clear_all_autorun(void)` — only one `ANODE` may
  carry `AF_AUTORUN`
- `deskins.c:359-372` `remove_all_viewers(void)` (`CONF_WITH_VIEWER_SUPPORT`)
- `deskins.c:381-393` `atow(s)` — ASCII-decimal-to-`WORD`, stops at first
  non-digit (used for the function-key text field)
- `deskins.c:404-444` `get_funkey(tree, pa, installed)` — validates a
  function-key dialog field, checking range and duplicates
- `deskins.c:455-600` `ins_app_dialog(pa, pathname, pfname, installed)` —
  the "Install Application" dialog's full field read/write logic
- `deskins.c:611-710` `ins_one_app(curr, pf)` — resolves which `ANODE` an
  "Install Application" invocation targets (desktop shortcut, window
  file, or default-viewer match) before calling `ins_app_dialog`
- `deskins.c:762-770` `insert_icon(tree, obj, nicon)` — swaps a dialog's
  preview-icon object to a different `G.g_iblist[]` entry (uses a
  **single file-`static` `ICONBLK ib`** scratch instance, see
  [Edge cases](#edge-cases-discovered-in-source))
- `deskins.c:781-978` `install_desktop_icon(pa)` — the "Install desktop
  icon" dialog (disk/trash/printer *and* file/folder-shortcut icon
  editing)
- `deskins.c:1012-1022` `set_icon(pa, icon)` (`CONF_WITH_WINDOW_ICONS`) —
  sets `a_dicon`, and `a_aicon` too iff the file is executable by
  extension
- `deskins.c:1028-1050` `allocate_window_anode(type)`
  (`CONF_WITH_WINDOW_ICONS`)
- `deskins.c:1061-1222` `install_window_icon(pf)` (`CONF_WITH_WINDOW_ICONS`)
  — the "Install window icon" dialog

**Helpers called (other files, cited — not re-derived here):**
- `deskapp.c` `app_alloc`/`app_free`/`app_afind_by_name`/`app_afind_by_id`/
  `app_afind_viewer`/`app_blddesk` (`ANODE` pool + rebuild — see
  `deskapp.md`)
- `deskwin.c` `win_isel`/`win_find`
- `desksupp.c` `deselect_all`/`i_find`
- [`deskinf.md`](deskinf.md) `inf_sset`/`inf_sget`/`set_tedinfo_name`/
  `draw_dial`/`draw_fld`/`start_dialog`/`end_dialog`
- [`deskfun.md`](deskfun.md) `fun_alert`/`illegal_op_msg`
- `util/optimize.c` `scan_str` (declared `deskapp.h`, defined `deskapp.c:258-289`
  — copies a string into the shared `G.g_atext[]` ANODE-text arena)

**Headers / type definitions used:**
- `deskins.h:16-18` — `FIRST_FUNKEY`=1, `LAST_FUNKEY`=20, `NUM_FUNKEYS`=20
- `deskapp.h:27-38` — `ANODE` bit flags: `AF_ISCRYS`/`AF_ISDESK`/
  `AF_ISPARM`/`AF_APPDIR`/`AF_ISFULL`/`AF_VIEWER`/`AF_AUTORUN`/
  `AF_ISEXEC`/`AF_WINDOW`
- `deskapp.h:43-49` — `AT_ISFILE`=0/`AT_ISFOLD`=1/`AT_ISDISK`=2/
  `AT_ISTRSH`=3/`AT_ISPRNT`=4 (`CONF_WITH_PRINTER_ICON`)
- `deskapp.h:54-62` — `IG_HARD`=0/`IG_FLOPPY`=1/`IG_FOLDER`=2/`IG_TRASH`=3/
  `IG_PRINT`=4/`IG_REMOV`=5/`IG_APPL`=6/`IG_DOCU`=7 (builtin icon indices
  into `G.g_iblist[]`)

**System variables / globals read:**
- `G.g_ahead` — the live `ANODE` list, walked by `ins_devices`/
  `grid_free`/`clear_all_autorun`/`remove_all_viewers`/`get_funkey`
- `G.g_icw`/`G.g_ich`/`G.g_desk` — desktop grid pitch + work area (see
  `_data-model.md`), read by `snap_icon`/`ins_posdisk`/`grid_free`
- `G.g_numiblks`/`G.g_iblist[]` — icon count/table, read by
  `install_desktop_icon`/`install_window_icon`'s up/down icon-picker and
  by `insert_icon`
- `G.g_cwin` (`CONF_WITH_DESKTOP_SHORTCUTS`/`CONF_WITH_WINDOW_ICONS`) —
  distinguishes "installing from a desktop shortcut" vs. "installing from
  a window selection" in `ins_one_app`/`ins_app`/`ins_icon`

**System variables / globals written:**
- New/edited `ANODE` fields throughout (`a_flags`/`a_type`/`a_letter`/
  `a_aicon`/`a_dicon`/`a_xspot`/`a_yspot`/`a_funkey`/`a_pappl`/`a_pdata`/
  `a_pargs`) — see per-function detail below

**Build configuration affecting this function:**
- `CONF_WITH_VIEWER_SUPPORT` — `is_viewer`, the "default viewer" concept
  in `ins_one_app`/`ins_app_dialog` (`AF_VIEWER`, `remove_all_viewers`)
- `CONF_WITH_DESKTOP_SHORTCUTS` — desktop file/folder shortcut installs
  (`ins_shortcut`, the `AT_ISFILE`/`AT_ISFOLD` branches in
  `install_desktop_icon`, and `is_executable`'s first gate)
- `CONF_WITH_WINDOW_ICONS` — the entire "install window icon" feature
  (`install_window_icon`/`set_icon`/`allocate_window_anode`, and
  `is_executable`'s second gate)
- `CONF_WITH_PRINTER_ICON` — the printer icon type throughout
  `install_desktop_icon`/`ins_devices` (indirectly, via `AT_ISPRNT`)
- `CONF_WITH_3D_OBJECTS` — purely cosmetic dialog Y-offset adjustments
  around the up/down icon-picker buttons (`install_desktop_icon`,
  `install_window_icon`) — **not** re-derived further, layout-only
- **Correction (this pass) — `CONF_WITH_3D_OBJECTS` default was mis-stated**:
  the earlier draft claimed default `0` in the general/Atari build; this is
  **wrong**. Resolving the `needs deeper analysis` flag by reading
  `include/config.h` directly: the flag is forced to `0` only inside three
  specific guarded blocks — `TARGET_192` (`include/config.h:310-312`, space-
  constrained ROM), `TARGET_256` (`:436-438`, likewise), and
  `#if !CONF_ATARI_HARDWARE` (`:800-802`, comment "Like ST, not Falcon" —
  this is EmuTOS's **non**-Atari-hardware ports, e.g. Apple Lisa, per
  `CLAUDE.md`'s own note "EmuTOS targets non-Atari machines"). The
  **unconditional fallback** actually used by the standard Atari-hardware
  build (ST/TT/Falcon/Firebee/ARAnyM — the profile NewTOS's extraction
  treats as "general/Atari") is `include/config.h:1258-1259`: `#ifndef
  CONF_WITH_3D_OBJECTS` → `#define CONF_WITH_3D_OBJECTS 1` — **on by
  default**, like every other gate in this file. This matches
  `docs/MEMORY.md`'s existing ADR ("AES dialogs: use the 3D look" — the
  user wants EmuTOS's 3D optic) — confirms, rather than contradicts, that
  decision. All gates in this file are therefore **on** by default in the
  general/Atari build, with no exception.

---

## Source extraction summary

`deskins.c` covers everything that creates, edits, or removes an
`ANODE` interactively (as opposed to `deskapp.c`'s `app_parse`/`app_save`,
which (de)serialize the `ANODE` list to/from `EMUDESK.INF` — see
[`deskinf.md`](deskinf.md)'s appendix). Three logically separate features
share the file:

1. **`ins_devices()`** — non-interactive, called at startup (and
   presumably after a media-change/rescan, cross-check `deskmain.md`):
   reconciles the desktop's disk-drive icons against the actual set of
   mounted drives (`dos_sdrv(dos_gdrv())`), removing icons for
   now-absent drives and adding icons for newly-present ones.
2. **`ins_app()`** — the "Install Application" dialog: associates a
   document type (wildcard) and launch behaviour with a `.PRG`/`.APP`/
   `.TOS`/`.TTP`/`.GTP` file, so that double-clicking a matching data file
   later launches this application with it as an argument.
3. **`ins_icon()`/`rmv_icon()`/`ins_shortcut()`** — desktop icon and
   (optionally) window-icon and desktop-shortcut management: create/edit/
   remove the disk/trash/printer/shortcut icons that live directly on the
   desktop surface, and (separately, `CONF_WITH_WINDOW_ICONS`) custom
   icons for specific files/folders shown *inside* windows.

All three ultimately mutate `G.g_ahead`'s `ANODE` chain via
`deskapp.c`'s `app_alloc()`/`app_free()` (a fixed-size pool of
`NUM_ANODES`=64 slots, see `deskapp.h:75`) and, for anything
desktop-visible, trigger `app_blddesk()` to rebuild the desktop's
`OBJECT` tree.

---

## `snap_icon`/`ins_posdisk`/`grid_free` — desktop grid placement

`deskins.c:146-184` (`snap_icon`), `:118-134` (`grid_free`), `:197-222`
(`ins_posdisk`). Already referenced from `_data-model.md`'s
"`a_xspot`/`a_yspot` unit gotcha" section (flagged there as **needs
deeper analysis** — this section resolves that flag).

**Build configuration:** all three functions are **unconditional** — none
of `snap_icon`/`grid_free`/`ins_posdisk` is wrapped in any `#if
CONF_WITH_*` (verified by rereading `deskins.c` in full). Only some of
their *callers* are conditional: `ins_shortcut` (`CONF_WITH_DESKTOP_
SHORTCUTS`) and, transitively, `move_drvicon`'s desktop-only invocation
context; `install_drive`/`install_desktop_icon`/`app_start`'s call sites
are unconditional in the general/Atari build NewTOS targets (see
`_overview.md`'s config-variant convention).

- **`snap_icon(x, y, px, py, sxoff, syoff)`** (`WORD` params, no return
  value; result via the `px`/`py` out-params) — converts a raw pixel
  position (typically the current mouse position, minus the cursor's
  offset within the dragged icon, `sxoff`/`syoff`) into a grid-aligned
  pixel position. Computes how many whole columns/rows fit in
  `G.g_desk.g_w`/`g_h` at pitch `G.g_icw`/`g_ich`, and how many "spare"
  pixels are left over after fitting that many whole columns/rows
  (`spare_pixels = g_w - columns*icw`) — these spare pixels are
  distributed **evenly as a single leading margin** (`spare_pixels /
  columns`, integer division — any remainder beyond that is simply lost,
  not distributed further), i.e. the icon grid is not flush to the
  desktop's top-left corner but instead centred by whole-cell excess.
  The target column/row are clamped to `[0, columns-1]`/`[0, rows-1]`
  (`:164,181` — `min(...)`) as a safety measure against
  out-of-range results from a mouse position outside the desktop.
  **`snap_icon` itself performs no collision check whatsoever** — reading
  the full body (`:146-184`) top to bottom, it only rounds `(x,y)` to the
  nearest grid cell and clamps to the desktop bounds; it never inspects
  `G.g_ahead`/any `ANODE`, never calls `grid_free`, and has **no
  "exclusion icon" parameter** (there is nothing to exclude — it doesn't
  look at other icons at all). Collision avoidance, where it exists at
  all, is entirely the job of `grid_free`/`ins_posdisk` below, and only
  the three *new-icon-install* call sites use them; `app_start()`'s
  INF-position-apply loop and `move_drvicon()`'s drag-drop path (see
  below) call `snap_icon` **alone**, so both can silently place/leave an
  icon exactly on top of another one.
- **`grid_free(x, y)`** (grid coordinates, not pixels): `TRUE` unless
  some other `AF_ISDESK` `ANODE`'s **current** `a_xspot`/`a_yspot`
  (pixels, per the dual-unit lifecycle already documented in
  `_data-model.md`) divides back to the same grid cell. No exclusion
  parameter either; the callers below rely instead on calling it
  **before** the new `ANODE` is allocated (`install_drive`/
  `install_desktop_icon`, `:236`/`:793`, both call `ins_posdisk` before
  `app_alloc()`), so the not-yet-existing icon can never collide with
  itself. `ins_shortcut` breaks this ordering for its *first* icon — see
  below.
- **`ins_posdisk(dx, dy, pdx, pdy)`**: starting from the grid cell
  containing `(dx,dy)`, scans rightward, then wraps to the next row
  (resetting the column to 0, not to the original starting column) via
  a **row-major** search, returning the first free cell found via
  `grid_free`. **If the entire remaining grid is occupied** (loop falls
  through without finding one), returns the **input position
  unchanged** (`:220-221`) — i.e. a completely full desktop grid places
  the new icon exactly on top of an existing one, with no further
  fallback (no scrolling, no error, no alert).

`install_drive` (`:236`) and `install_desktop_icon` (`:793`) both call
`ins_posdisk(0, 0, &x, &y)` **before** allocating the new `ANODE` — i.e.
**always starting the search from grid cell `(0,0)`**, not from any
"last placed" position — meaning successively-installed icons are placed
by scanning from the top-left every time, not by remembering where the
last one landed. **`ins_shortcut` does *not* follow this pattern — this
corrects an earlier draft of this entry**, which mis-stated that all
three call sites call `ins_posdisk(0,0,…)`: rereading `deskins.c:1341-
1411` shows `ins_shortcut` calls `snap_icon(mx, my, &x, &y, 0, 0)`
directly (`:1353`) for its *first* icon — using the actual mouse-drop
position, with **zero collision checking**, since (as established above)
`snap_icon` never calls `grid_free` — and only calls `ins_posdisk(x, y,
&x, &y)` (`:1396`) **after** placing each icon, to find the position for
the *next* one in the same multi-select drop. Net effect: **the first
icon dropped in a multi-icon desktop-shortcut install can land exactly on
top of a pre-existing icon** (no check at all), while the second and
later icons in that same drop *are* collision-safe (each `ins_posdisk`
call scans starting from the previous icon's just-placed position, and
`grid_free` sees that previous icon's now-committed `ANODE` and skips
past it) — see [Edge cases](#edge-cases-discovered-in-source).

---

## `ins_devices` — reconcile disk icons with mounted drives

`deskins.c:290-339`. Builds a bitmask of `installed` drives (from
existing `AT_ISDISK` `ANODE`s' `a_letter`) and compares against
`current = dos_sdrv(dos_gdrv())` (the live drive bitmap). Removes icons
for `installed & ~current` (drives that vanished), installs icons for
`current & ~installed` (drives that appeared), in that order (removals
before installs). Returns `TRUE` iff any change was made (used by the
caller to decide whether to rebuild the desktop). `install_drive`
(`:230-255`) sets `a_aicon = (drive > 1) ? IG_HARD : IG_FLOPPY` — **A
and B are always floppies, C and up are always hard disks**, matching
`deskapp.md`'s `app_blddesk`/`build_inf` observation for the built-in
EMUDESK.INF generator (same rule, independent code path).

---

## `ins_app`/`ins_app_dialog`/`ins_one_app` — install application

`ins_app()` (`:721-756`) branches on whether the current context is the
desktop (`CONF_WITH_DESKTOP_SHORTCUTS`, `G.g_cwin==DESKWH`) or a window:
on the desktop, loops every selected desktop icon
(`win_isel(G.g_screen, G.g_croot, curr)`); in a window, loops every
selected `FNODE`. Each iteration calls `ins_one_app(curr, pf)`, stopping
early (`break`) if it returns negative (user cancelled further installs)
— crucially, **the loop does not stop on `rc==0`** ("nothing changed for
this one," e.g. user hit Skip), only on cancel. Returns `-1` if
**any** item actually changed (window context only tracks `change++`;
the desktop-shortcut context, per the comment at `:735`, "doesn't track
changes since we don't rebuild the desktop" — installing an application
via a desktop shortcut **never triggers a rebuild**, only via a window
file selection does).

`ins_one_app` (`:611-710`) resolves *which* `ANODE` the dialog will
edit:
- **Desktop-shortcut context** (`CONF_WITH_DESKTOP_SHORTCUTS`): looks up
  the shortcut's own path via `app_afind_by_id`, then searches for an
  **already-installed, non-shortcut** `ANODE` for the same full path
  (`app_afind_by_name`) — if found, edits *that* one instead of the
  shortcut's own `ANODE`, so installing-an-application through a desktop
  shortcut and through the equivalent window file converge on the same
  underlying record.
- **Window context**: if `CONF_WITH_VIEWER_SUPPORT` and a default viewer
  is already installed and matches this exact file, uses the viewer's
  `ANODE`; otherwise searches by name (`app_afind_by_name`, ignoring
  `AF_ISDESK|AF_WINDOW|AF_VIEWER` flags when matching — i.e. matches
  purely on path+name, not on how the existing entry was categorized).

`ins_app_dialog` (`:455-600`) is the actual dialog loop
(`ADINSAPP` resource): pre-fills name/args/doctype/funkey/flags fields
from the existing `ANODE` if any, loops `form_do` until a terminal
button:
- **`APINSTAL`**: allocates a new `ANODE` if not already installed,
  validates the function key (`get_funkey` — invalid or duplicate-and-
  user-declined both **request a retry without exiting the dialog**,
  `:521-529`, by setting `exitobj=-1` and looping the outer `do…while`),
  sets `a_flags` from the GEM-app/needs-parms/app-dir/full-path/autorun
  radio/checkbox groups (`AF_ISPARM`/`AF_ISCRYS`/`AF_APPDIR`/`AF_ISFULL`/
  `AF_AUTORUN` — **choosing autorun clears every other `ANODE`'s
  autorun flag first**, `clear_all_autorun()`, enforcing "at most one
  autorun app"), builds the full pathname only if newly installed
  (`installed` case keeps the existing `a_pappl`), and — if
  `CONF_WITH_VIEWER_SUPPORT` and the resulting `ANODE` now qualifies as a
  viewer (`is_viewer`) — removes any other existing default viewer first.
- **`APREMOVE`**: frees the `ANODE`, but **only if `installed`** — you
  cannot "remove" the generic placeholder `ANODE` for a file type that
  was never actually installed (`:585-586`).
- **`APSKIP`**/**`APCANCEL`**: no state change; cancel additionally sets
  `change=-1` to stop the outer loop in `ins_app`.

---

## `ins_icon`/`rmv_icon` — desktop and window icon install/remove

`ins_icon(sobj)` (`:1234-1311`):
- **`sobj==0`** (menu-invoked with nothing selected): prompts
  `STICNTYP` (a choice: desktop / window / cancel, `CONF_WITH_WINDOW_ICONS`
  only offers the choice — without that config, always goes straight to
  `install_desktop_icon(NULL)`) and installs a **brand-new** icon of the
  chosen kind. Returns `1` (rebuild desktop) or `-1` (rebuild windows)
  or `0` (nothing changed/cancelled).
- **`sobj!=0` and it resolves to a desktop `ANODE`**
  (`app_afind_by_id(sobj)` succeeds): loops every selected desktop icon
  (again via `win_isel(G.g_screen, DROOT, sobj)`, `:1275`), calling
  `install_desktop_icon(pa)` **per existing icon** — i.e. selecting
  multiple desktop icons and choosing "Install icon" edits each one in
  turn via repeated dialogs, summing how many were actually changed into
  the return value; a negative (cancelled) result from any one stops the
  loop early. **Precision note for the 1:1 rebuild** (re-verified against
  source): `ins_icon`'s (and `rmv_icon`'s, `:1322`) desktop-selection walk
  always passes the **literal `DROOT` object index** as `win_isel`'s root
  parameter, not `G.g_croot` — this differs from `ins_app`'s desktop-shortcut
  branch and `ins_shortcut`'s selection walk, both of which pass
  `G.g_croot` (`deskins.c:730`, `:1359`). This is not a bug (both functions
  only ever operate on desktop icons, whose root object *is* always
  `DROOT`, so the literal constant and `G.g_croot` happen to coincide
  whenever the desktop is actually current), but a faithful function-level
  port must reproduce which of the two (constant vs. global) each specific
  call site actually reads, since a Rust rebuild that always uses one form
  could silently diverge from EmuTOS if `G.g_croot`'s desktop-context
  invariant is ever violated (e.g. by a future non-EmuTOS-faithful call
  path).
- **Else, if `CONF_WITH_WINDOW_ICONS`**: assumes the selection is inside
  the current window, loops every selected `FNODE`, calling
  `install_window_icon(pf)` per file/folder — return sign convention is
  **inverted** for this branch (`rebuild--` on success, i.e. **negative**
  = "rebuild windows", matching the desktop-menu handler's sign
  convention documented in `deskmain.md`: "`IICNITEM` rebuilds only if
  `rebld>0`... and instead does a window rebuild if `rebld<0`" — this
  confirms `deskmain.md`'s cross-reference from the caller side).

`install_desktop_icon(pa)` (`:781-978`) handles **both** creating a new
desktop icon (`pa==NULL`, defaults to a disk-type placeholder at the
first free grid slot from `(0,0)`) and editing an existing one
(`icon_exists=TRUE`). Field editability depends on the icon's *current*
type: file/folder shortcuts (`CONF_WITH_DESKTOP_SHORTCUTS`) only allow
editing the label and icon glyph (ID/drive/trash/print fields are
`DISABLED`); disk/trash/printer icons allow changing *everything*,
including re-categorizing the icon's type via the ID/Drive/Trash/Print
radio group (`inf_gindex(tree, ID_DRIVE, 3)` at commit time). The
up/down icon-picker buttons (`ID_UP`/`ID_DOWN`) cycle `curr_icon` through
`[0, G.g_numiblks-1]` clamped (not wrapped) at both ends, live-swapping
the preview via `insert_icon`+`draw_fld`. Disk-type icons additionally
**require** a non-blank drive letter (`retry=TRUE` if the `ID_ID` field
is empty or a space, `:917-918`) — the dialog **redisplays itself**
(`draw_dial`) with an alert (`STDRIVID`) rather than accepting an
incomplete disk icon. On `ID_SKIP`/`ID_CNCL` for a **newly-allocated**
icon (`!icon_exists`), the allocated `ANODE` is explicitly freed
(`:942-943`) — a cancelled "create new" leaves no dangling `ANODE`.

`install_window_icon(pf)` (`CONF_WITH_WINDOW_ICONS`, `:1061-1222`) is the
window-icon analogue: `pf==NULL` means "create a new file/folder-name
association not tied to a specific currently-visible file" (the name
field becomes editable, file/folder type becomes choosable); `pf!=NULL`
edits the icon for that specific already-existing `FNODE`'s match (name
fixed, type fixed to whatever the file actually is). `IW_INST`
(install/reinstall) versus `IW_REMV` (remove) both first resolve the
target `ANODE` by name (`app_afind_by_name(type, AF_ISDESK, "",
new_name,…)` when `pf==NULL`, else `pf->f_pa` directly) and compare
`pa->a_pdata` against the (possibly just-edited) name field
(`identical`) — **removing** a window icon requires the name to be
unchanged from the matched `ANODE`'s (`STNOMTCH` alert otherwise,
`:1183-1185`); **installing** only allocates a genuinely new `ANODE` if
the name is **not** identical to an existing match (`:1166-1177` —
i.e. re-submitting the same name/icon just calls `set_icon` on the
already-found `ANODE`, no duplicate is created).

`rmv_icon(sobj)` (`:1317-1335`): loops every selected desktop object,
frees the `ANODE` **only if it's flagged `AF_ISDESK`** (`:1327` — a
selected object that resolves to something else, e.g. this function
being called on the wrong root, is silently skipped, not erred).

---

## Edge cases discovered in source

- **`snap_icon` is pure grid-rounding, not collision avoidance** — the
  function has no knowledge of other icons at all (no `G.g_ahead` walk,
  no `grid_free` call, no exclusion parameter); only `grid_free`/
  `ins_posdisk` avoid collisions, and only three call sites
  (`install_drive`, `install_desktop_icon`, and `ins_shortcut`'s 2nd+
  icon) use them. Two important consumers call `snap_icon` **alone**,
  with no collision check at all:
  - `deskapp.c:979-987`'s `app_start()` INF-position-apply loop (see
    [`deskapp.md`](deskapp.md)) — every `AF_ISDESK` `ANODE`'s parsed
    grid-unit spot is converted to pixels and passed through
    `snap_icon` independently, in `G.g_ahead` list order. **Two
    EMUDESK.INF entries with the same (or rounding-adjacent) saved grid
    position produce two desktop icons drawn on top of each other at
    boot**, with no detection, no error, no re-spacing. This is the key
    fact for INF-position handling (task #14): the INF file's grid
    coordinates are trusted verbatim; nothing in `app_start()` de-duplicates
    or nudges them apart.
  - `deskact.c:126-163`'s `move_drvicon()` (see [`deskact.md`](deskact.md))
    — dragging one or more desktop icons to a new spot calls `snap_icon`
    per selected icon with no `grid_free` check either; a user can drag
    an icon (or a multi-selection) directly on top of another icon and
    EmuTOS will happily overlap them.
- **`ins_shortcut`'s first icon in a multi-select drop is not
  collision-checked; the rest are** (`deskins.c:1341-1411`, see above) —
  the initial `snap_icon(mx,my,…)` call (`:1353`) has no `grid_free`
  behind it, but every subsequent icon in the same drop goes through
  `ins_posdisk(x,y,&x,&y)` (`:1396`), which *is* collision-safe because
  it re-scans starting from the position just committed. Relevant to
  task #20 (install-icon placement): a NewTOS reimplementation that
  wants to guarantee no overlap for *every* installed icon (including
  the first of a batch) must add a `grid_free`/`ins_posdisk`-style check
  before the first placement too — EmuTOS itself does not do this for
  `ins_shortcut`, only for `install_drive`/`install_desktop_icon`.
- **A completely full desktop icon grid silently overlaps a new icon
  onto an existing one** (`ins_posdisk`, `:220-221`) — no error, no
  scroll, no alert; this is a genuine "can't happen in practice" gap
  (64 `ANODE` slots vs. typically far more grid cells) but is
  reproducible if a NewTOS reimplementation ever has a much smaller
  desktop resolution with many more installed icons than fit.
- **`insert_icon` uses a single file-`static` `ICONBLK ib` scratch
  instance** (`deskins.c:52`, written by `:762-770`) shared by **every**
  dialog's icon-preview object across `install_desktop_icon` and
  `install_window_icon` — safe only because dialogs are strictly modal
  (one at a time) and each dialog fully re-`insert_icon`s before
  displaying; a reimplementation with concurrent/re-entrant dialogs
  would need per-dialog storage instead.
- **`ins_app`'s desktop-shortcut branch never signals "rebuild needed"**
  even when an install actually happened (`:735-736` comment) — this
  appears to rely on the fact that installing-an-application changes
  only `a_pdata`/`a_pargs`/`a_flags`/icon numbers on an **existing**
  `ANODE` that's already correctly placed and drawn; no new icon needs
  adding to any tree. Worth a regression check that this assumption
  holds (i.e. that `ins_one_app`'s desktop-shortcut path never allocates
  a *new* `ANODE` that would then be missing from the desktop until the
  next unrelated rebuild).
- **`ins_icon`'s window-icon-loop return sign is inverted relative to
  its desktop-icon-loop** (`rebuild++` vs. `rebuild--`, `:1282` vs.
  `:1303`) — this is a **deliberate protocol** with the caller
  (`deskmain.c`'s `do_optnmenu`, per its own documented sign convention),
  not an accidental inconsistency; a reimplementation must preserve the
  sign, not just the magnitude.
- **Function-key validation can force a dialog retry mid-install**
  (`get_funkey`, invalid range or user-declined duplicate) — the dialog
  clears the funkey field, deselects the `ID_OK`-equivalent button, and
  redraws in place (`draw_dial`) rather than closing — a stateful retry
  loop inside what looks like a single `form_do` call from the outside.

---

## Error conditions

| Condition | Source location | Result |
|---|---|---|
| `app_alloc()` returns `NULL` (pool of 64 `ANODE`s exhausted) | `deskapp.c:207-222`, checked at e.g. `deskins.c:239-240`,`822-823`,`513-517` | `STAPGONE` alert (inside `app_alloc` itself); callers here treat it as "stop trying" (`return -1` from `install_drive`, `change=-1` from `ins_app_dialog`) |
| Function key out of `[FIRST_FUNKEY,LAST_FUNKEY]` range | `deskins.c:421-425` | `STINVKEY` alert, `get_funkey` returns `-1`, dialog retries |
| Function key already used by another `ANODE`, user declines to steal it | `deskins.c:436-437` | `STDUPKEY` alert; declining returns `-1` (retry); accepting clears the other `ANODE`'s key |
| Disk icon install with blank drive-letter field | `deskins.c:916-918` | `STDRIVID` alert, dialog redisplays, no commit |
| Window-icon remove with a changed (non-identical) name | `deskins.c:1183-1185` | `STNOMTCH` alert, no removal |

---

## Verification criteria

- **`ins_devices` idempotence**: calling `ins_devices()` twice in a row
  with no drive-mount changes between calls must return `FALSE` the
  second time and make no `ANODE` changes.
- **Grid placement determinism**: `ins_posdisk(0,0,…)` on an otherwise-
  empty desktop must return `(0,0)`; on a desktop with cell `(0,0)`
  occupied, must return `(1,0)` (row-major scan).
- **Autorun exclusivity**: installing a new application with the autorun
  checkbox selected must leave **at most one** `ANODE` with
  `AF_AUTORUN` set afterward, regardless of how many previously had it
  (`clear_all_autorun` runs unconditionally before setting the new one).
- **Cancelled new-icon install leaves no orphan `ANODE`**: choosing
  Skip/Cancel on a freshly-allocated (not-yet-committed) desktop or
  window icon must return the `ANODE` to the free pool
  (`G.g_aavail` count unchanged from before the dialog opened).

---

## Dependencies

### Requires (must be initialized / functional)
- `deskapp.c` — `app_alloc`/`app_free`/`app_afind_by_id`/
  `app_afind_by_name`/`app_afind_viewer`/`app_blddesk` (see `deskapp.md`)
- [`deskinf.md`](deskinf.md) — dialog primitives (`inf_sset`/`inf_sget`/
  `set_tedinfo_name`/`start_dialog`/`end_dialog`/`draw_dial`/`draw_fld`)
- [`deskfun.md`](deskfun.md) — `fun_alert`/`illegal_op_msg`
- `docs/api/desktop/_data-model.md` — `G.g_icw`/`g_ich`/`g_desk`/
  `g_iblist[]`/`g_numiblks`

### Used by
- `deskmain.c`'s `do_optnmenu` (`IDSKITEM`→`ins_devices`(?, cross-check),
  `IAPPITEM`→`ins_app`, `IICNITEM`→`ins_icon`, `RICNITEM`→`rmv_icon` — per
  `deskmain.md`'s own citation of this file)
- [`deskfun.md`](deskfun.md)'s `fun_drag` (`ins_shortcut`, drop-onto-
  empty-desktop case, `CONF_WITH_DESKTOP_SHORTCUTS`)
- [`deskact.md`](deskact.md)'s `move_drvicon` (`snap_icon`)
- `deskapp.c`'s `app_start()` (`snap_icon`, converting parsed
  `EMUDESK.INF` grid-unit spots to pixels — see `_data-model.md`'s
  "`a_xspot`/`a_yspot` unit gotcha", now resolved above)

## Related data structures

- `docs/api/desktop/_data-model.md` — `ANODE` (rendering-relevant
  fields), `a_xspot`/`a_yspot` dual-unit lifecycle
- `docs/api/structures/iconblk.md` — `ICONBLK` (the `ib` scratch instance,
  `G.g_iblist[]`)

## Cross-references

Not independently cross-checked against Compendium/Profibuch — icon
installation dialogs are desktop-application UI, not documented TOS API
surface. The resulting `ANODE` semantics (icon type, application
association, autorun) are cross-referenced against the `EMUDESK.INF`
format in [`deskinf.md`](deskinf.md), which **is** partially documented
in period sources (Atari's own EMUDESK.INF format was informally
published) — no such cross-check was performed in this pass; flagged as
a possible follow-up if NewTOS ever targets EMUDESK.INF file
compatibility.

## Configuration / version notes

See [Build configuration](#emutos-source) above. No TOS-version-specific
behaviour identified (EmuDesk-specific file; original Atari TOS desktop
had a broadly similar but not identical install-icon feature set per
general knowledge — not independently verified against a TOS ROM
disassembly, out of scope).

---

## NewTOS implementation notes

### `ins_app` — Install Application (deskins #20)

The dialog half lives in `programs/desktop/src/insapp.rs`, the resolution and
commit half in `desk::main::do_install_app`; the association itself is a
`NEWDESK.INF` `#G`/`#Y`/`#P`/`#F` line held by `desk::inf`. See that
module's header for the field subset it offers and why.

#### The function-key field (DESK20, 2026-09-23)

`ADINSAPP` object 5 is `APFUNKEY` (`desk_rsc.h:119`, `desk_rsc.c:1059-1063`):
an `EDITABLE | FL3DBAK` `G_FBOXTEXT` over `desk_rs_tedinfo[19]`, template
`"Install as: F__"`, `te_pvalid` `"9"`, `te_txtlen` **3**. A `te_txtlen`
counts the NUL, so the field takes **two** digits — which is what
`FIRST_FUNKEY..LAST_FUNKEY` = 1..20 (`deskins.h:16-17`) needs. NewTOS's
dialog had no such field at all, so `a_funkey` was carried, saved and
displayed in *Desktop configuration* but could never be assigned; the panel
there was always empty.

Now built, in EmuTOS's three places:

* **pre-fill** (`deskins.c:474-477`) — `sprintf(name, "%u ", pa->a_funkey)`
  when the node has a key, empty otherwise, and empty for an application that
  is not installed. `inf_sset` truncates to the field width, which for a
  two-digit key drops the trailing blank.
* **validate** (`get_funkey`, `:404-444`) — `atow` (`:380-393`) over the raw
  field; an empty field is 0 ("no key"); `installed && funkey == a_funkey`
  returns unchanged **without** re-validating, which is deliberate in EmuTOS
  (a hand-written `.INF` value outside the range survives an edit of the other
  fields); out of range raises `STINVKEY` (`desk_rsc.c:2525`); a key another
  `ANODE` holds raises `STDUPKEY` (`:2526-2527`) and, on OK, clears it there.
* **store** (`:520-531`) — a negative answer clears the field, deselects
  `APINSTAL`, redraws and asks for a retry; otherwise `pa->a_funkey = funkey`.

**NewTOS shape.** EmuTOS asks its duplicate question *inside* its own walk
over `G.g_ahead`; NewTOS must not hold the `desk::inf` `SESSION` mutex across
a trap, so the walk is split into `funkey_owner` (the test) and
`release_funkey` (the `an->a_funkey = 0`), with the alert between them. The
desktop is single-CPU and the dialog modal, so the answer cannot change in
between. The two alert strings are written verbatim as resource text
(`[  OK  ]`, `[  OK  |Cancel]`) rather than through `dir::fun_alert`, whose
wrapper writes `[ OK ]` — a pre-existing difference in that wrapper that is
not carried into these two.

**Still missing, and now a real gap.** Pressing the key launches nothing:
`process_funkey` (`deskmain.c:1087-1106`) has no counterpart, so
`desk::main::check_function_key` still falls through. Until DESK20 that was
unobservable (no key could exist); it is a genuine hole now.

Measured 2026-09-23 (`measurements/measurement-2026-09-23-desk19-desk20/`): `7` assigned
and shown in *Desktop configuration*; `99` rejected with `STINVKEY` and the
field cleared for a retry; a second application asking for `7` answered with
`STDUPKEY` and, on OK, taking the key over.

---

### `ins_devices` / `ins_icon` / `rmv_icon` (DESK03 + DESK04, 2026-09-24)

`programs/desktop/src/ins.rs` is the `deskins.c` module: `grid_free`,
`ins_posdisk`, `install_drive`, `remove_drive`, `ins_devices`,
`install_desktop_icon` (the `ADINSDSK` dialog), `ins_icon` and `rmv_icon`.
`snap_icon` itself sits with the other grid metrics in `desk::app`
(`DeskState::snap_icon` / `snap_icon_grid`), which is where NewTOS keeps the
`app_start` geometry.

#### The desktop icon list became state — this is the load-bearing change

Before this ticket `app_blddesk` **derived** the desktop icons on every call:
one `G_ICON` per bit of `dos_sdrv(dos_gdrv())`, plus a Trash and a Printer
fixture, with the `NEWDESK.INF` `#M`/`#T`/`#O` lines contributing nothing but
a grid position. Three consequences made DESK03/DESK04 impossible:

* *Install devices* was a no-op by construction — every drive in the map
  always had an icon.
* *Remove icon* could not remove anything — the next `app_blddesk` put it
  straight back.
* *Install icon* had nowhere to write a new icon to.

So `GLOBES` gained EmuTOS's `G.g_ahead`: `DeskState::g_ahead: Vec<Anode>`
(`desk::global::Anode`), and `app_blddesk` now walks it and invents nothing —
`deskapp.c:1356-1400`, line for line, including the `pa->a_obid = obid`
write-back that lets `app_afind_by_id` map a clicked object to its node.

| field | EmuTOS | NewTOS |
|---|---|---|
| `a_type` | `AT_ISDISK`/`AT_ISTRSH`/`AT_ISPRNT` | `model::AType::Disk/Trash/Printer` |
| `a_letter` | `char`, `'\0'` = none | `u8`, `0` = none |
| `a_aicon` | icon number | same numbering (`IG_*`, see below) |
| `a_dicon` | icon number | **not carried** — always `NIL` for a desktop icon (`deskins.c:250`, `:854`), written as `FF` |
| `a_pappl` | label (usage (1), `deskapp.h:88-90`) | `String` |
| `a_xspot`/`a_yspot` | pixels, screen-absolute | pixels, **relative to `DROOT`** |
| `a_obid` | object index | same |
| `a_pdata`/`a_pargs`/`a_funkey`/`a_flags` | shared with the file-type nodes | **not carried** — see below |

Two deliberate narrowings, both named:

1. **The list holds desktop icons only.** EmuTOS keeps one `ANODE` chain for
   the desktop icons *and* the file-type / installed-application
   associations. NewTOS split those when Install Application was built: the
   association half lives in `desk::inf`'s session `NEWDESK.INF` model. Both
   halves serialise into the same file in the same line grammar, so the split
   is internal.
2. **Window icons live in the *other* half; desktop shortcuts are still a
   gap.** Both `CONF_WITH_WINDOW_ICONS` (`install_window_icon`, the
   `ADINSWIN` tree) and `CONF_WITH_DESKTOP_SHORTCUTS` (`AT_ISFILE`/`AT_ISFOLD`
   icons parked on the desktop, `ins_shortcut`) attach an `ANODE` to a *file or
   folder name*. Since **DESK15** the window-icon half is implemented, and it
   is implemented in the association half of the split: an `AF_WINDOW` node is
   an `#I`/`#N`/`#D` line of the `desk::inf` session model, resolved by
   `inf::app_afind_by_name`. The desktop-shortcut half would need an
   `AT_ISFILE`/`AT_ISFOLD` variant of *this* list plus `fun_win2win`'s
   drop-on-a-program branch, and is **still a gap** (DESK30) — reported, not
   faked. Both are 1 in our Falcon030/TOS4 `make 512` profile
   (`include/config.h:1782-1784`, `:1866-1868`; the `0` defaults at `:322-327`
   are inside `#ifdef TARGET_192`, which `make 512` does not define).

`app_revit()` has no counterpart. EmuTOS reverses the chain before
`app_blddesk` and before `app_save` (`deskapp.c:1352`, `:1211`) purely because
`app_alloc` *prepends* (`:215-216`), so its list runs newest-first. A `Vec`
that `push`es is already in creation order; both walks are plain forward
loops, visiting the same nodes in the same order without the list surgery.

`move_drvicon` writes the snapped spot back into the node
(`deskact.c:146-151`), which it did not have to before: with the object tree
rebuilt from the list on every `app_blddesk`, a dragged icon would otherwise
jump home the moment *Install devices* or *Remove icon* ran, and `app_save`
would persist the old spot.

#### Where the list comes from at boot — and the signature check

`app_start`'s order is reproduced in `desk::run`: `app_start()` (grid
metrics) → `inf::load_and_apply()` → `inf::build_inf()` when that found no INF
data → `win_view()` → `app_blddesk()`.

The guard is EmuTOS's and it is **a content test, not a file-existence
test**: `read_inf_file(inf_data); if (inf_data[0] != '#') build_inf(…)`
(`deskapp.c:829-834`). `shel_get` hands EmuTOS a buffer either way, so the
only thing that can distinguish "there is an INF" from "there is not" is the
`#` signature. NewTOS's test image ships a 21-byte `newdesk placeholder\r\n`
as `C:\NEWDESK.INF`, so this is not a theoretical case: the first cut of this
ticket tested file existence and the desktop booted with **no icons at all**.

The asymmetry matters in the other direction too. A file that *is* INF data
and carries no `#M`/`#T`/`#O` line describes a desktop with **no icons**, and
that is honoured — it is exactly what *Remove desktop icon* followed by
*Save desktop* writes, and `build_inf` must not resurrect what the user
removed.

`inf::build_inf()` = `apply(…)` + `SESSION = Some(…)` + `build_desk_icons()`.
EmuTOS assembles three pieces of text and re-parses them; here the first piece
**is** `InfData::default()` (the same `#E 1A E0 00 00 60` /
`#Q 41 40 43 40 43 40` bytes as `desk_inf_data1`), the third is
`InfData::default_type_anodes()` (`desk_inf_data2`), and only the icon half
has to be built. The text detour is not observable.

**The synthesised file becomes the session model (DESK15).** It used to be
applied and dropped, on the grounds that `build_from_state` reaches
`default_type_anodes()` again on a first-ever save. That was enough while
nothing *read* the generic nodes; it stopped being enough when `win_icalc`
started asking them for a window item's icon and `install_window_icon` started
resolving a name against them. EmuTOS **parses** its `infbuf` into the live
`G.g_ahead` (`deskapp.c:745-752`, `:838-841`), so `app_afind_by_name` always
has the eight generic nodes to answer from and `install_window_icon`'s
`if (!pa) … "can't happen"` (`deskins.c:1157-1161`) really cannot happen.
Leaving the model empty made it happen on any machine whose `NEWDESK.INF` is
missing or unreadable — which is every fresh card.

**`build_inf`'s layout is now the desktop's default, and it differs from what
NewTOS drew before.** Drives fill the grid left-to-right
(`icon_index % xcnt` / `icon_index / xcnt`, `deskapp.c:719-721`); the Trash
goes to the bottom-**left**, moved to the bottom-right only when the last
drive icon reached that row (`:738-744`); the Printer takes the bottom-right
only when the Trash did not (`:747-754`). The previous NewTOS rule — drives
down column 0, Trash and Printer in the rightmost column, rows 0 and 1 —
carried no source citation. At 640x480 with one drive the difference is that
Trash and Printer move from the top-right corner to the bottom row.

The grid counts are `app_start`'s, `xcnt = G.g_desk.g_w / (G.g_wicon +
MIN_WINT)` and `ycnt = G.g_desk.g_h / (G.g_hicon + MIN_HINT)`
(`deskapp.c:813-817`) — **not** `g_desk.g_w / g_icw`, which can differ by a
column when the division is not exact.

#### The icon set: EmuTOS's numbering, all eight pictures

The `a_aicon` byte of an `ANODE` line is **file format**, not an internal
index: `app_parse` reads it (`deskapp.c:359-365`), `app_save` writes it
(`:1258`), and `desk_inf_data2` spells `07`/`02`/`06` literally. NewTOS
therefore uses EmuTOS's numbers (`desk::icons`):

| `IG_*` | picture | NewTOS raster |
|---|---|---|
| 0 `IG_HARD` | hard drive | yes |
| 1 `IG_FLOPPY` | floppy drive | yes |
| 2 `IG_FOLDER` | folder | yes |
| 3 `IG_TRASH` | trash can | yes |
| 4 `IG_PRINT` | printer | yes |
| 5 `IG_REMOV` | removable disk | yes (DESK16) |
| 6 `IG_APPL` | generic application | yes |
| 7 `IG_DOCU` | generic document | yes |

**DESK16 closed the `IG_REMOV` hole.** Until then `icons::BUILTIN_ICONS` had
seven entries, the stepper walked seven instead of `0 .. G.g_numiblks-1`, and
an `a_aicon` that named the missing picture fell back to a type-shaped default.
The raster came the same way the other seven did: out of
`references/emutos/desk/icons.c`, the C the EmuTOS build generates from
`desk/icon.rsc` with `ird`. `icon_rs_iconblk[5]` is `rs_icondata5` over
**`rs_iconmask0`** — the removable disk carries *no mask of its own*, it shares
the hard disk's, and `icons::rasters` hands out `HARD_MASK` for it for that
reason. Its `ib_xchar`/`ib_ychar` are the hard disk's too, `(13,16)`, so a
removable drive shows its drive letter in the same place.

`BUILTIN_ICONS` is therefore eight entries = `icons::NUM_GEM_IBLKS`
(`deskapp.h:62`), and the two consequences of the hole are gone:

* the `ID_UP`/`ID_DOWN` stepper cycles all eight, i.e.
  `0 .. G.g_numiblks-1` as `deskins.c:902-906` does (minus the floppy — see
  below, which is a different and still-live NewTOS rule);
* `app_blddesk` resolves `a_aicon` without a fallback, because
  `desk::inf`'s parser now applies `app_parse`'s own out-of-range guard
  (`deskapp.c:359-365`): `a_aicon >= NUM_GEM_IBLKS` becomes `IG_APPL`,
  `a_dicon >= NUM_GEM_IBLKS` becomes `IG_DOCU`. `app::default_icon_kind` is
  gone with the hole it served.

  One named deviation remains, on a value EmuTOS reads out of bounds: an
  `a_aicon` of `-1` (an `#M`/`#T`/`#O` line that spells the icon field `FF`,
  which only a hand-written file can carry — `build_inf` always writes the
  number, `deskapp.c:723`). EmuTOS would index `G.g_iblist[-1]`; NewTOS takes
  `app_parse`'s own answer for an unusable `a_aicon`, `IG_APPL`.

**The floppy is offered only when it can be used.** `IG_FLOPPY` is dropped
from the stepper's list unless the drive map actually reports `A:` or `B:`.
The machine this desktop runs on has no floppy controller — the menu already
drops *Format* for the same reason — so a floppy picture would be a choice no
drive could ever carry. The one exception: a node that *already* carries
`IG_FLOPPY` (a `NEWDESK.INF` written where a floppy existed) keeps it in the
list, so the dialog can always step away from the picture it is showing
rather than silently rewriting it.

#### `install_desktop_icon` — the `ADINSDSK` tree

`desk_rsc.c:922-1020`. Seventeen objects, and the NewTOS tree uses the same
indices `desk_rsc.h:100-112` gives them, so a reader can hold the two side by
side:

| # | name | type / flags | content |
|---|---|---|---|
| 0 | ROOT | `G_BOX`, `FL3DBAK`, state `OUTLINED`, `ob_spec = (1<<16)\|0x1100` | the panel |
| 1 | — | `G_STRING` | `INSTALL DESKTOP ICON` |
| 2 | `ID_ID` | `G_FBOXTEXT`, `FL3DBAK` — **no `EDITABLE`** | `Identifier: _:`, validation `A`, 1 char |
| 3 | `ID_LABEL` | `G_FBOXTEXT`, `EDITABLE\|FL3DBAK` | `Label: ____________`, validation `X`, 12 chars |
| 4 | — | `G_STRING`, `FL3DBAK` | `Item type:` |
| 5 | — | `G_IBOX` | the radio group's parent |
| 6-8 | `ID_DRIVE`/`ID_TRASH`/`ID_PRINT` | `G_BUTTON`, `SELECTABLE\|RBUTTON\|FL3DIND` | `Drive` / `Trash can` / `Printer` |
| 9 | `ID_IBOX` | `G_BOX`, `FL3DBAK`, `ob_spec = 0x1100` | the preview panel |
| 10 | `ID_ICON` | `G_ICON` (`insert_icon` swaps the type in) | the `ICONBLK`, `ib_ptext = ""` |
| 11-12 | `ID_UP`/`ID_DOWN` | `G_BUTTON`, `SELECTABLE\|TOUCHEXIT\|FL3DACT` | font characters `\001` / `\002` = the arrows |
| 13 | — | `G_STRING`, `FL3DBAK` | `Icon:` |
| 14-16 | `ID_OK`/`ID_SKIP`/`ID_CNCL` | `G_BUTTON`, `…EXIT\|FL3DACT`, `LASTOB` on the last | `OK` / `Skip` / `Cancel` |

The 3D roles are `desk_rsc.c`'s, read off the table in `deskinf.md`
(DESK01b): `FL3DBAK` on the ROOT and the two framed text fields, `FL3DIND` on
the radio buttons, `FL3DACT` (= `FL3DBAK|FL3DIND`) on the exit buttons **and**
on the two stepper gadgets — `ADINSDSK`'s `ID_UP`/`ID_DOWN` are precisely the
pair DESK01b cited as "a stepper gadget is an activator"
(`desk_rsc.c:990,996`).

Two box values are worth naming, for the same reason DESK01b had to name
`ADDINFO`'s: the AES substitutes the 3D face colour only for a box that is
`IP_HOLLOW` **and** white (`gemoblib.c:619-636`). The ROOT is `69888L` =
`(1<<16)|0x1100` — one pixel of border over that colour word. `ID_IBOX` is
`4352L` = `0x1100`, the same colour word with the thickness word **zero**: it
takes the face and draws **no** frame of its own. The first cut of this tree
gave `ID_IBOX` the ROOT's value and produced a black rectangle EmuTOS does not
have.

**`ID_ID` is not `EDITABLE`, and that is the source, not an omission.**
`desk_rsc.c:936-940` gives it `FL3DBAK | 0x4000`, the second being the
desktop-private `RIGHT_ALIGNED` alignment hint (`deskmain.c:106`, stripped
again by `align_objects`), and no `EDITABLE` bit — yet
`install_desktop_icon` passes `start_fld = ID_ID` to `form_do` and reads the
field back with `inf_sget`. It works because `fm_inifld` hands `start_fld`
through untouched (`aes/gemfmlib.c:153-160`) and `ob_edit` never looks at the
flag, so the caret *starts* in the identifier field and the user can type
there. What they cannot do is come back: `fm_button` gates the click on
`EDITABLE`, and the arrow-key walk visits only genuinely editable fields, so
once the caret has moved to `ID_LABEL` the identifier is out of reach for that
turn of the dialog. NewTOS reproduces flag and start field verbatim. (Should
this ever be decided to be an EmuTOS bug rather than a quirk, the fix is one
`EDITABLE` bit — but it is a change to EmuTOS's behaviour and needs its own
ticket.)

EmuTOS nudges `ID_DOWN` by `ADJ3DSTD` before the dialog and back afterwards
(`deskins.c:826-831`, `:966-971`) because its resource was laid out for flat
buttons and a 3D indicator grows two pixels per side before its border is
drawn. The NewTOS tree is built fresh on every call with the grown rectangles
already accounted for (the `ADSETPRE` spacing rules DESK01b derived), so there
is nothing to nudge and no restore to forget.

The commit path is `deskins.c:909-943`: the type comes from
`inf_gindex(tree, ID_DRIVE, 3)`; a **Drive** must end up with a non-blank
identifier or the dialog re-displays itself behind `STDRIVID`
(`"[1][No drive identifier.][ Retry ]"`, `desk_rsc.c:2542` — the button reads
*Retry*, so the string goes to `form_alert` verbatim rather than through
`dir::fun_alert`'s one-OK-button wrapper); the label is written back only when
it changed; `ID_SKIP`/`ID_CNCL` on a *newly created* node free it again
(`:941-943`), so a cancelled "create new" leaves nothing behind.

`draw_fld(tree, ID_IBOX)` (`deskins.c:962`) now exists as
`desk::dir::draw_fld`, next to `draw_dial` — which is exactly where EmuTOS
keeps it (`deskdir.c:131-139`): `objc_offset` the object, then `objc_draw`
that subtree into its own rectangle. It needed a new `objc_offset` (44)
binding in the desktop's aesbind block. A stepper click therefore repaints the
preview panel and nothing else, where `inf_backgrounds` still redraws its
whole tree.

#### `ins_devices`

`deskins.c:290-333`, unchanged in shape: build the installed bitmask from the
`AT_ISDISK` nodes' letters, remove the icons of `installed & ~current`, then
install one for each of `current & ~installed`, in that order, and report
whether anything changed. `install_drive` takes the first free grid cell
(`ins_posdisk(0, 0)` *before* the node is appended, `:231`) and
`a_aicon = (drive > 1) ? IG_HARD : IG_FLOPPY` (`:249`).

The labels follow EmuTOS: `desktop_str_addr(STDISK)` plus the drive letter for
a drive icon of either kind, `STTRASH`, `STPRINT` (`desk_rsc.c:2481-2483`),
i.e. `"DISK C"` / `"TRASH"` / `"PRINTER"`. NewTOS printed `"Hard disk"` /
`"Floppy disk"` / `"Trash"` / `"Printer"` from its first icon tree until the
wording was adopted; that changes the boot picture and invalidates the label
text on every earlier desktop screendump. `install_drive`
(`deskins.c:247`) and `build_inf` (`deskapp.c:723`) build the same string, so
an icon added by *Install devices* is labelled exactly like one the desktop
built at boot; the picture (`IG_HARD` vs `IG_FLOPPY`) is what distinguishes a
floppy from a hard disk, not the text.

#### `ins_icon` / `rmv_icon` and the menu state

`ins_icon(sobj)` (`deskins.c:1234-1311`): with `sobj == 0` a brand-new icon is
created; otherwise every selected desktop icon is edited in turn, one dialog
each, stopping early on cancel. The selection walk passes the **literal
`DROOT`**, not `G.g_croot` — the precision note earlier in this document,
reproduced. `rmv_icon(sobj)` (`:1317-1335`) frees the node of every selected
desktop object and returns how many. Its `if (pa->a_flags & AF_ISDESK)` guard
has no counterpart because `G.g_ahead` holds desktop icons only; an object
that resolves to no node is skipped, as there.

`do_optnmenu` gets the three cases `deskmain.c:847-881` has, `curr =
win_isel(G.g_screen, G.g_croot, 0)` included (`:842`), and each rebuild is
`app_blddesk()` plus the `do_wredraw(DESKWH, &G.g_desk)` EmuTOS folds into the
end of `app_blddesk` (`deskapp.c:1402`), plus a `men_update()` — the selection
died with the objects that carried it.

`men_update` gains EmuTOS's `ndesk` count (`deskmain.c:504-521`, the
`G.g_cwin == DESKWH` arm): *Remove desktop icon* is disabled iff no desktop
icon is selected (`:540-542`). *Install icon* and *Install devices* appear in
no `ILL_*` table and are disabled nowhere, so both are always available —
*Install icon* with nothing selected **is** how a new icon is made. A greyed
menu entry is the whole of EmuTOS's answer to "Remove with nothing
selected"; there is no alert.

#### `a_letter` reaches the bitmap unconditionally

`app_blddesk` ends with `pic->ib_char |= pa->a_letter;` (`deskapp.c:1399`),
with no test on the icon picture, and NewTOS now does the same. It has to:
the Install-icon dialog lets a drive node take any picture, and every desktop
-icon consumer reads the drive letter back out of `ib_char` (`desk::dir`'s
drop handlers, `desk::inf`'s `#M` writer). For the two disk rasters
`build_iconblk` already ORs it in at their own `ib_xchar`/`ib_ychar` spot; for
any other picture the letter lands at the bitmap's origin, because NewTOS's
rasters carry a character position only where EmuTOS's resource does.

#### Verification

`measurements/measurement-2026-09-24-desk03-desk04/`. QEMU raspi4b at 640x480 with the
scratch-copy IKBD injector: the Options drop-down in both selection states,
select → *Remove desktop icon* → *Install devices* (pixel-identical to the
start), *Install icon* with the stepper and the type radios, *Save desktop*,
the written `NEWDESK.INF`, a cold boot that restores the installed icon, and a
second cold boot that keeps a removed one removed. Plus
`desk::ins::selftest()` — 14 cases, including the vanished-drive
reconciliation QEMU cannot stage on its own — run once from a scratch copy.

### `install_window_icon` / `ins_icon`'s `STICNTYP` branch (DESK15, 2026-09-23)

`CONF_WITH_WINDOW_ICONS` is **1** in our Falcon030/TOS4 `make 512` profile
(`include/config.h:1866-1868`; the `0` at `:322-324` is inside
`#ifdef TARGET_192`, and `make 512` defines `TARGET_512`, `Makefile:560-562`).
The feature is therefore part of the reference build, and since DESK15 it is
implemented.

#### The reader came first: `win_icalc`

`win_bldview` used to pick a window item's icon from its file-name extension
alone (`desk::ftype::classify`: folder → `IG_FOLDER`, `.NAP`/`.NCP`/`.NTP` →
`IG_APPL`, anything else → `IG_DOCU`). EmuTOS picks it from the `ANODE` list:
`win_icalc` resolves `app_afind_by_name(type, AF_ISDESK|AF_VIEWER,
pw->w_pnode.p_spec, pf->f_name, &pf->f_isap)` and `win_bldview` then takes
`i_index = f_isap ? anode->a_aicon : anode->a_dicon`
(`deskwin.c:377-384`, `:462-474`).

NewTOS now does the same. `DeskState::win_icalc` (`desk::win`) runs the query
in **both** view branches, as `deskwin.c:455`/`:462` do, and the icon branch
maps the resulting number through `icons::icon_kind`. Two named differences:

* **No `f_pa`/`f_isap` cache in the `FNODE`.** EmuTOS stores the resolved
  `ANODE *` back into the node because its second consumer, `do_aopen`'s
  installed-application launch (`desksupp.c:857-875`), reads it from there.
  NewTOS's `do_aopen` resolves the association independently through
  `inf::lookup_association`, so `win_icalc` returns the answer instead of
  caching it, and `install_window_icon` re-runs the identical query.
* **The extension rule survives as the fall-back.** EmuTOS's is `IG_FOLDER` /
  `f_isap ? IG_APPL : IG_DOCU` (`deskwin.c:469-473`) and can only be reached
  when the list is empty, which its `app_start` never leaves it. NewTOS keeps
  `ftype::classify` in that position — it also catches the `-1` an `ANODE`
  line spells `FF` for “no icon of that kind”.

`inf::app_afind_by_name` is the EmuTOS walk, line for line: skip the `ignore`
nodes, require `a_type == atype`, try `wildcmp(a_pdata, pname)` first
(`*pisapp = FALSE`), then `a_pappl` — `wildcmp` when it starts with `*`/`?`,
otherwise `strcmp` against `pspec`'s directory + `pname`. The `ignore` mask is
`AF_ISDESK` alone: `AF_VIEWER` is `CONF_WITH_VIEWER_SUPPORT`, which this
desktop does not implement, so no node can carry it.

#### `desk_inf_data2` is stored in EmuTOS's *live* order, and gains three lines

The generic type `ANODE`s decide which extensions get the application icon, so
the order they are searched in is now observable. `desk_inf_data2` lists the
three `*.*` catch-alls first (`deskapp.c:181-189`), but `app_alloc` links every
parsed node at the **head** (`:204-219`), so `G.g_ahead` runs the other way
round — `*.TOS` … `*.GTP` first, the catch-alls last — and `app_save` restores
the file order with the `app_revit` pair around the write (`:1213`, `:1285`).
NewTOS has no head-linked list to reverse: `InfData::anodes` *is* the live list
and is serialised in that order. `default_type_anodes()` is therefore written
in the **search** order, and the on-disk order of those lines is the reverse of
EmuTOS's. Named deviation; the resolution result is identical, and without it
`FOO.PRG` would be swallowed by `#F FF 07 @ *.*@`.

Three lines are NewTOS's own, on the same footing as ADR-018's file names:

    #G 06 FF *.NAP@ @      native GEM application
    #F 06 FF *.NCP@ @      native console program
    #P 06 FF *.NTP@ @      native program with a command tail

`desk_inf_data2` is exactly where EmuTOS declares which extensions name a
program; the classic five stay (they are still the m68k family's names) and
the native three join them. `is_executable` follows: EmuTOS's
`exec_ext[] = {"TOS","TTP","PRG","APP","GTP"}` (`deskins.c:984`) is NewTOS's
`FileType::M68k`, so the question becomes "not a `FileType::Doc`".

A `NEWDESK.INF` written by a pre-DESK15 NewTOS carries only the eight EmuTOS
lines, so on such a card a `.NAP`/`.NCP`/`.NTP` matches `#F FF 07 @ *.*@` and
gets the document icon until the file is saved again. EmuTOS has the identical
behaviour for an `EMUDESK.INF` older than one of its own `desk_inf_data2`
changes and does nothing about it; neither do we.

#### `install_window_icon` — the `ADINSWIN` tree

Built on the stack like the sibling dialogs, with `ADINSWIN`'s own object
numbers (`desk_rsc.h:225-237`) and its row structure (`desk_rsc.c:2038-2135`):
title, the `Name: ________.___` field (`fmt_str`/`unfmt_str` 8.3 form, the
`desk_rs_tedinfo[33]` template and its `"F"` validation), *Item type:* with the
Folder/File radio pair, the icon preview with its two steppers beside `Icon:`,
then Install / Remove on one row and Skip / Cancel on the next.

* With an `FNODE`: the name is fixed and not `EDITABLE`, `edit_start = 0`, and
  both radios are pre-set **and** `DISABLED` (`deskins.c:1093-1104`).
* Without one: the name is `EDITABLE` and empty, `edit_start = IW_NAME`, *File*
  is pre-selected and both radios are enabled (`:1105-1112`).
* `curr_icon = pa ? pa->a_dicon : 0`, clamped (`:1115-1120`), stepped over
  `offered_icons` — all eight of EmuTOS's `BUILTIN_ICONS` since DESK16, the
  floppy only when a floppy drive exists.
* `IW_INST`/`IW_REMV` both resolve the node first, compare `a_pdata` against
  the (possibly edited) name (`identical`, `:1164`), and then: Install reuses
  the node when the names are identical, else `allocate_window_anode` +
  `scan_str` + `set_icon`; Remove refuses a non-identical name with `STNOMTCH`
  and otherwise `app_free`s (`:1166-1192`).
* The loop breaks **before** clearing `SELECTED` and only deselects for
  `IW_UP`/`IW_DOWN` (`:1196-1201`) — the opposite of `install_desktop_icon`
  (`:943-945`), and reproduced as such.

Two named differences from the resource. (1) The `ADJ3DSTD` nudge of `IW_DOWN`
(`:1117-1123`, `:1210-1216`) has no counterpart: the tree is built fresh with
the grown rectangles already accounted for, exactly as `ADINSDSK` is. (2)
EmuTOS's `app_alloc` can fail when the 64-entry `NUM_ANODES` pool is full and
`install_window_icon` answers `change = -1` (`:1170-1174`); NewTOS's list is a
`Vec` with no fixed pool, so that arm has no counterpart.

`set_icon` (`:1012-1022`) sets `a_dicon` always and `a_aicon` too when the data
name is executable. EmuTOS records that in `AF_ISEXEC` and lets `app_save` turn
it into the `#I`/`#N` tag (`deskapp.c:1230-1231`); `InfAnode` carries no flag
word, so the **tag is the flag** and is set there. A folder node keeps `#D`,
which is what `app_save` writes for any non-shortcut `AT_ISFOLD` node.

#### `ins_icon`'s three branches

`sobj == 0` now asks `STICNTYP` ("Please select icon type." / Desktop | Window
| Cancel, `desk_rsc.c:2534`) and answers `1` / `-1` / `0`. A selection that
resolves to a desktop `ANODE` takes the existing per-icon loop. Anything else
is taken to be inside the current window: every selected `FNODE` of
`win_find(G.g_cwin)` gets one `ADINSWIN` dialog, and the sign is **inverted**
(`rebuild--`, `:1298`), which is what makes `deskmain`'s `IICNITEM` arm run
`win_bdall(); win_shwall();` instead of `app_blddesk()` (`deskmain.c:868-875`).
That arm is now in `main::do_optnmenu` too.

One named difference: EmuTOS walks the live `p_flist` across the dialogs;
NewTOS snapshots the selected entries first, because the dialog issues AES
traps and cannot run while the desktop lock is held. Nothing the dialog does
touches the file list, so the two walks see the same entries.

#### Verification

`measurements/measurement-2026-09-23-desk15/`. QEMU raspi4b at 640x480 with the scratch-copy
IKBD injector: the reader (a `.NCP`/`.NAP` shows the application icon, a `.TXT`
the document one — pixel-compared, `measurements/measurement-2026-09-23-desk15/screendumps/icons-cmp.png`)
, select
`README.TXT` → *Install icon* → `ADINSWIN` → three steps up to the trash
picture → *Install* → that file alone changes in the window, *Save desktop*
writes `#N FF 03 @ README.TXT@`, and a cold boot brings the trash icon back.
Then the `STICNTYP` alert with nothing selected, its *Window* answer opening
the editable-name shape of the dialog, and *Remove* on the installed name
putting the document icon back.

---

### `ins_shortcut` — desktop shortcuts (DESK30, 2026-09-24)

`CONF_WITH_DESKTOP_SHORTCUTS` is 1 in the Falcon030/TOS4 `make 512` profile
(`include/config.h:1782-1784`; the `0` at `:325-327` sits inside
`#ifdef TARGET_192`), so the second half of the DESK15 report — a **file or
folder icon parked on the desktop surface** — was a real gap. It is now
implemented; the entry point is `programs/desktop/src/ins.rs::ins_shortcut`
(`deskins.c:1341-1406`), called from `dir::fun_drag`'s "dropping onto desktop
surface" arm (`deskfun.c:1312-1319`), which until now raised the `#else`
branch's alert.

**Data layer.** `global::Anode` gained `a_dicon`, `a_flags` (the `AF_ISEXEC`
bit; `AF_ISDESK` stays implicit because every node of `G.g_ahead` *is* a
desktop node) and `a_pdata`. For a shortcut `a_pappl` is the **full path** and
`a_pdata` the bare file name — the revision-2 layout — and `app_blddesk`
labels the icon from `a_pdata` (`deskapp.c:1392-1396`) and picks the icon with
`(a_dicon < 0) || (a_type == AT_ISFILE && (a_flags & AF_ISEXEC))`
(`:1374-1378`).

**`.INF`.** The `#X` (file) and `#V` (folder) tags were already accepted by the
parser; they now produce `AT_ISFILE`/`AT_ISFOLD` nodes, `app_save` writes them
back (`deskapp.c:1224-1243`), and the pre-revision-2 `a_pappl`/`a_pdata` swap
(`:404-415`) is applied for a file that carries `#R 00`/`#R 01`. `AF_ISEXEC` is
re-derived from the extension on load (`:422-426`), through
`ins::is_executable`, whose set is EmuTOS's five plus NewTOS's native
`.NAP`/`.NCP`/`.NTP` (ADR-018).

**Consumers.** A double-click routes through `main::open_item`'s desktop arm
(`do_open`, `desksupp.c:1219-1259`): a file is classified by its extension and
launched/associated exactly as its twin in a window would be, a folder always
gets a **new** window because the "window" it was opened from is the desktop
pseudo-window (`do_fopen`'s `WN_DESKTOP` arm, `:1112-1126`). Dragging a
shortcut into a window or onto another icon goes through the new
`dir::desktop_selected_sources`, which is `fun_file2any`'s `an_src` switch
(`deskfun.c:1149-1160`): a disk contributes every top-level entry of its root,
a shortcut the one file or folder it names. Dropping something **onto** a
shortcut is `fun_file2desk`'s two new arms (`:1021-1040`) — a program launches
with the dragged item as its tail, a folder takes a copy or move.

**Orphans.** `remove_locate_shortcut` (`desksupp.c:376-412`) is implemented in
`dir.rs`: the `STRMVLOC` alert (Remove | Locate | Cancel), *Locate* running
`fsel_exinput` from the boot drive's root and rewriting `a_pappl`. It is
raised where EmuTOS raises it — `do_aopen`'s `file_exists` check before a
launch (`:861-868`), `do_fopen`'s `set_default_path == EPTHNF` before a folder
window (`:1119-1123`), and the empty-file-list case of a drag (`:1183-1188`).

**One collapsed step, named.** `ins_shortcut` takes the icon numbers from the
`ANODE` `i_find` returns and then overrides them with a second lookup,
`app_afind_by_name(…, AF_ISDESK, …)` (`:1394-1400`). The two differ only in
the ignore mask — `i_find` also skips `AF_WINDOW` nodes (`desksupp.c:1755`) —
and NewTOS's `app_afind_by_name` skips desktop nodes only, i.e. it already *is*
the second call's variant. The first lookup therefore returns the node the
override would have chosen, and the override is a no-op; one search is made.

**Still open, and not part of this ticket.** *Show info* on a **desktop icon**
is not implemented at all in NewTOS (`main::do_info_rename` returns early for
`G.g_cwin == DESKWH`), so `do_info`'s shortcut arm (`deskinf.c` via
`desksupp.c:1300-1330`) has nothing to attach to. That gap predates DESK30 and
covers drives, trash and printer as well.

#### Verification

`measurements/measurement-2026-09-24-desk37-desk36-desk30/`, QEMU raspi4b at 640×480 with
the scratch-copy IKBD injector:

* `d30-after-D-shortcut.png` — `QED.NAP` dragged out of the `C:\` window onto
  the desktop surface becomes an icon labelled `QED.NAP` with the application
  picture. `d30-before-D-shortcut.png` is the defect the user hit on the Pi:
  the alert "Can't drop items here."
* `d30-after-E-saved.png` + the card it wrote: *Options → Save desktop* puts
  `#X 02 09 06 FF   C:\QED.NAP@ QED.NAP@` into `NEWDESK.INF`.
* `d30b-A-coldstart.png` — that card booted again from scratch: the shortcut is
  back, in the same grid cell, with the same label and picture.
* `d30c-D-qed.png` — double-clicking the restored shortcut starts QED.
* `d30d-D-opened.png` — the folder case: `SUB` dragged to the desktop
  (`d30d-C-shortcut.png`), double-clicked, opens a **new** window titled
  `C:\SUB\` with its three items.
