# `deskinf.c` — info/preference dialogs, text-view formatting — **plus** the `EMUDESK.INF` file format

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/deskinf.c` (1272 lines). Per
> [`docs/api/desktop/_overview.md`](_overview.md): this file was
> previously flagged "out of scope … `dr_code`/`format_fnode` are the
> `V_TEXT` rendering callback referenced (not traced) from
> `win_bldview` … flagged **needs deeper analysis**" and
> [`desksupp-redraw.md`](desksupp-redraw.md) similarly deferred it. This
> entry resolves both flags.

## Important naming note: the `EMUDESK.INF` line format is **not** in this file

The task brief asks for the "`NEWDESK.INF` Parse/Save inkl. `#W`
(Fensterpositionen), `#M`/`#T`/`#O` (Icon-Platzierung), `#E`
(Environment/Desktop-Flags), `#Q`/`#D`/`#G`/`#F`/`#P`" to be extracted
from `deskinf.c`. **This is a naming mismatch in the task brief, not in
this extraction**: `deskinf.c` contains **no** `EMUDESK.INF` parsing or
saving code at all — it is entirely dialog/display logic (Show Info,
Preferences, Set Background, Desktop Configuration). The actual
`EMUDESK.INF` parser and writer (`app_parse()`, `app_start()`,
`build_inf()`, `app_save()`) live in **`desk/deskapp.c`**, which is a
sibling extraction task's file
([`deskapp.md`](deskapp.md), produced in parallel) — that file's own
scope note says "EMUDESK.INF parsing / ANODE installation already
narrated in `docs/api/desktop.md`", but neither `deskapp.md` nor
`docs/api/desktop.md` actually give the **per-prefix line format**
(verified by reading both — see the report to the lead for the grep
evidence). Since the exact line format is the one thing the task brief
explicitly calls "the most valuable extract," and it is genuinely
missing from every existing spec file, it is documented **here**, in a
clearly-separated [appendix](#appendix-the-emudeskinf-file-format-source-deskappc-not-deskinfc)
below, honestly cited to `desk/deskapp.c` (not `deskinf.c`). The rest of
this file documents `deskinf.c`'s actual, real content.

---

## EmuTOS source (`deskinf.c` itself)

**Primary locations (public, `deskinf.h`):**
- `deskinf.c:331-343` `dr_code(pparms)` — the `V_TEXT` `USERBLK` draw
  callback
- `deskinf.c:396-403` `inf_show(tree, start)`
- `deskinf.c:493-639` `inf_file_folder(ppath, pf)`
- `deskinf.c:650-693` `inf_disk(dr_id)`
- `deskinf.c:410-420` `inf_numset(tree, obj, value)`
- `deskinf.c:721-818` `inf_pref(void)`
- `deskinf.c:427-433` `set_tedinfo_name(tree, obj, str)`
- `deskinf.c:1253-1271` `opn_appl(papname, ptail)`
- `deskinf.c:349-357` `start_dialog(tree)` / `:363-390` `end_dialog(tree)`
- `deskinf.c:825-921` `inf_backgrounds(void)` (`CONF_WITH_BACKGROUNDS`)
- `deskinf.c:1096-1246` `inf_conf(void)` (`CONF_WITH_DESKTOP_CONFIG`)

**Installed handler / vector:** `(none)` — called from `deskmain.c`'s
`do_filemenu`/`do_optnmenu` (`SHOWITEM`→`inf_file_folder`/`inf_disk`,
`PREFITEM`→`inf_pref`, `BACKGRND`→`inf_backgrounds`, `CONFITEM`→
`inf_conf`), and from other `desk/` modules for dialog primitives
(`start_dialog`/`end_dialog`/`inf_show`/`inf_numset`/`set_tedinfo_name`
are used throughout `deskdir.c`, `deskins.c`, `deskfun.c` — see those
files' "Helpers called (other files)" sections).

**Helpers called (this file, `static`):**
- `deskinf.c:74-105` `fmt_time(time, fmt_string, ptime)` — DOS packed
  time → display string
- `deskinf.c:124-180` `fmt_date(date, fourdigit, pdate)` — DOS packed
  date → display string, per `G.g_cdateform`/`_IDT` cookie
- `deskinf.c:190-217` `fmt_size(size, wide, psize)` — file size → display
  string, kbyte-scaled for huge files on narrow screens
- `deskinf.c:220-307` `format_fnode(pf, pfmt)` — **the `V_TEXT` line
  formatter** (detailed [below](#format_fnode--dr_code--the-v_text-line-formatter))
- `deskinf.c:310-328` `dr_fnode(last_state, curr_state, x, y, w, h, fnode)`
  — draws one formatted line, or XOR-inverts it for a selection-state
  change
- `deskinf.c:440-457` `inf_fifosz(tree, dl_fi, dl_fo, dl_sz)` — files/
  folders/size triple into a dialog (used by both `inf_file_folder` and
  `inf_disk`)
- `deskinf.c:460-469` `inf_dttm(tree, pf, dl_dt, dl_tm)` — date+time pair
  into a dialog
- `deskinf.c:476-481` `count_ffs(path)` — thin wrapper over
  `d_doop(0, OP_COUNT, path, path, 0, NULL)` (see
  [`deskdir.md`](deskdir.md))
- `deskinf.c:705-715` `inf_which(tree, baseobj, numobj)` — reverse of
  `inf_gindex` (find the selected radio button, counting from the end)
- Desktop-configuration-dialog helpers (`CONF_WITH_DESKTOP_CONFIG`):
  `:929-944` `get_all_funkeys`, `:957-973` `scroll_conf_funkeys`,
  `:979-996` `init_conf_funkeys`, `:1002-1018` `init_conf_shortcuts`,
  `:1028-1057` `save_shortcut`, `:1066-1090` `recover_shortcuts`

**Helpers called (other files, cited — not re-derived here):**
- `deskdir.c:528` `d_doop()` (via `count_ffs`) — see
  [`deskdir.md`](deskdir.md)
- `desksupp.c` `desk_busy_on`/`desk_busy_off`, `valid_drive`
- `deskrsrc.c` `desktop_str_addr`

**`deskrsrc.c` — checked directly, confirmed trivial (task-brief follow-up)**:
`desk/deskrsrc.c` is 36 lines total, containing exactly **one** function,
`char *desktop_str_addr(WORD stnum)` (`:33-36`) — a one-line wrapper over
`gettext(desk_rs_fstr[stnum])` (NLS-aware resource-string lookup). It is
**not** "built-in resource load" despite the name suggesting otherwise: the
actual resource-loading entry point, `desk_rs_init()` (called once,
`deskmain.c:1980`, "copies OBJECTs and TEDINFOs in ROM to RAM"), and the
`desk_rs_trees[]`/`desk_rs_fstr[]` arrays it populates, have **no visible
definition anywhere in `references/emutos/`** (verified by grep across the
whole tree) — they are generated at build time by EmuTOS's resource
compiler from `desktop.def`/`desktop.rsc` (confirmed no `desk_rsc.c` exists
in the checkout; only the generated header `desk_rsc.h` is `#include`d by
name, never present as source). **Conclusion: no new `deskrsrc.md` is
needed** — `deskrsrc.c`'s only real content is already correctly cited here
and in [`deskdir.md`](deskdir.md)/[`_overview.md`](_overview.md) ("resource
string lookup... out of scope (utility)"). For NewTOS, "resource loading"
has no EmuTOS-parity code to port at all: NewTOS builds its `OBJECT` trees
directly in Rust rather than loading a compiled `.rsc` blob, so this whole
mechanism is internal-and-free (`docs/COMPATIBILITY.md`), not contract.
**Out-of-scope observation for the lead**: `deskmain.md:888` cites
"`desk_rs_init()`/resource loading (`deskrsrc.md`/`deskobj.md`)" — but
`deskobj.c` (checked directly, `obj_init`/`obj_walloc`/`obj_wfree`/
`obj_ialloc`/`obj_get_obid`) is actually the **screen-object-arena
allocator** (`G.g_screen[]`/`G.g_screeninfo[]` memory management), not
resource loading either; this citation in `deskmain.md` appears imprecise
and is outside this task's scope to fix (not one of the three files
assigned here).
- AES: `form_center`, `form_dial`, `objc_draw`, `evnt_multi`, `form_do`
  (contract, `docs/api/aes/object-form.md`)
- GEMDOS: `dos_chmod`, `dos_rename`, `dos_space`, `dos_label`
- `util/optimize.c` `unfmt_str`/`fmt_str` (dialog text field ↔ plain
  string conversion, not re-derived)

**Headers / type definitions used:**
- `include/aesdefs.h` — `PARMBLK` (the `objc_draw`/`USERBLK` callback
  parameter block: `pb_prevstate`/`pb_currstate`/`pb_x`/`pb_y`/`pb_w`/
  `pb_h`/`pb_parm`/`pb_xc`) — this is the AES contract `dr_code`
  implements against; see `docs/api/structures/` for `USERBLK`/`PARMBLK`
  if separately documented (cross-check — not verified present in this
  pass)
- `deskfpd.h` — `FNODE` (`f_attr`/`f_name`/`f_size`/`f_date`/`f_time`)
- `deskwin.h:36` — `LEN_FNODE`=48 (the fixed formatted-line buffer size;
  see [`deskact.md`](deskact.md)'s `act_chkobj`, which hit-tests against
  this same width in `V_TEXT` mode)

**System variables / globals read:**
- `G.g_ctimeform`/`G.g_cdateform` (`deskbind.h:176-177`) — display format
  preference (`TIMEFORM_12H`/`_24H`/`_IDT`, `DATEFORM_MDY`/`_DMY`/`_IDT`)
- `G.g_idt` (`deskbind.h:183`) — the `_IDT` cookie value, consulted when
  the format preference is `*FORM_IDT`
- `G.g_ndirs`/`g_nfiles`/`g_size` — populated by `count_ffs`, displayed
  by `inf_fifosz`
- `G.g_cdelepref`/`g_ccopypref`/`g_covwrpref`/`g_cdclkpref` — Preferences
  dialog state (`inf_pref`)
- `G.g_appdir`/`g_fullpath` (`CONF_WITH_DESKTOP_CONFIG`) — Desktop
  Configuration dialog state (`inf_conf`)
- `G.g_screen[DROOT].ob_spec`/`G.g_screen[DROOT+1].ob_spec` — current
  desktop/window background pattern+colour (`inf_backgrounds`)
- `G.g_patcol[]` (`CONF_WITH_BACKGROUNDS`) — background defaults array
- `G.g_rmsg[]` — AES message buffer, drained by `end_dialog`'s
  redraw-catch-up loop

**System variables / globals written:**
- `G.g_cdelepref`/`g_ccopypref`/`g_covwrpref`/`g_cdclkpref`/
  `g_ctimeform`/`g_cdateform` — by `inf_pref` on OK
- `G.g_appdir`/`g_fullpath` (`CONF_WITH_DESKTOP_CONFIG`) — by `inf_conf`
  on OK
- `G.g_screen[DROOT].ob_spec`/`G.g_screen[DROOT+1].ob_spec`, `G.g_patcol[index]` — by
  `inf_backgrounds` on OK (`CONF_WITH_BACKGROUNDS`)
- `menu_shortcuts[]` (`deskmain.h`, not `G`) — by `save_shortcut`/
  `inf_conf` (`CONF_WITH_DESKTOP_CONFIG`)

**Build configuration affecting this function:**
- `CONF_WITH_BACKGROUNDS` — gates `inf_backgrounds` entirely
- `CONF_WITH_DESKTOP_CONFIG` — gates `inf_conf` and its helpers; the menu
  item itself (`CONFITEM`) is also config-gated (see `docs/api/desktop.md`)
- Both default **on** in the general/Atari build, off under `TARGET_192`
  (same convention as `_overview.md`).

---

## Source extraction summary

`deskinf.c` is a grab-bag of dialog-driven "information display" and
"preference editing" features, unified only by all going through the
same `start_dialog`/`form_do`/`end_dialog` pattern:

1. **Formatting helpers** (`fmt_time`/`fmt_date`/`fmt_size`) — convert
   DOS-packed date/time/size values into display strings, honoring the
   user's chosen format (12/24-hour, MDY/DMY, or `_IDT`-cookie-driven) —
   used both by dialogs in this file (`inf_file_folder`/`inf_disk`) *and*
   by the `V_TEXT` per-file line formatter below.
2. **`format_fnode`/`dr_fnode`/`dr_code`** — the actual "show folder
   contents as a text list" renderer, invoked indirectly by
   `deskwin.c`'s `win_bldview()` (not re-derived here, see
   [`deskwin.md`](deskwin.md)) via a `USERBLK` callback installed on each
   `G_USERDEF` object.
3. **`inf_file_folder`/`inf_disk`** — the "Show Info" dialogs for a
   single file/folder and for a whole disk, respectively; both use
   `count_ffs`/`inf_fifosz` to show folder/disk content totals.
4. **`inf_pref`** — the global Preferences dialog (confirm-copy/-delete/
   -overwrite, double-click speed, date/time format).
5. **`inf_backgrounds`** (`CONF_WITH_BACKGROUNDS`) — the "Set
   Background" pattern/colour picker, live-previewed.
6. **`inf_conf`** (`CONF_WITH_DESKTOP_CONFIG`) — the "Desktop
   Configuration" dialog: browse/edit function-key-to-application
   assignments and menu-item keyboard shortcuts.
7. **`opn_appl`** — the "enter command-line arguments" prompt shown when
   launching a `.TTP`/`.GTP` (parameter-requiring) application.
8. **`start_dialog`/`end_dialog`** — shared dialog open/close plumbing,
   used by essentially every dialog in `desk/`, not just this file's own.

---

## `format_fnode`/`dr_code` — the `V_TEXT` line formatter

`deskinf.c:220-307` (`format_fnode`), `:310-328` (`dr_fnode`),
`:331-343` (`dr_code`). This is the callback `win_bldview()` installs
(via a `USERBLK` whose `ub_code = dr_code`, `ub_parm = <FNODE*>`) on
every `G_USERDEF` object when the current view is `V_TEXT` — the AES
calls `dr_code` (through the generic `USERBLK`/`objc_draw` contract)
every time that object needs (re)drawing, passing a `PARMBLK*` with the
object's screen rectangle and previous/current `ob_state`.

**`dr_code(pparms)`**: saves/restores the VDI clip rectangle around the
call (`gsx_gclip`/`gsx_sclip`), delegating to `dr_fnode`.

**`dr_fnode(last_state, curr_state, x, y, w, h, fnode)`**: if only the
`SELECTED` bit changed between the previous and current state (i.e. this
redraw is purely a selection-highlight toggle, not a content change),
does a cheap **XOR fill** over the object's rectangle
(`bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, x,y,w,h)`) instead of
re-formatting and redrawing the text — an optimisation that only works
because XOR-filling an already-drawn text line twice restores it exactly
(text drawn in `MD_TRANS` mode, i.e. it only sets the foreground pixels
class, so XOR-filling the whole cell inverts foreground pixels to
background and background to a color that, XORed again, returns to
original — this reasoning is implicit in the code, not spelled out in
comments; flagged as **needs deeper analysis** only if a pixel-exact
reimplementation of the XOR-select visual is required — the *contract*
observable here is simply "selected text lines are shown inverted," the
XOR mechanism is a free implementation choice). Otherwise, calls
`format_fnode` to build the line text, then draws it with
`gsx_tblt(IBM, x, y, len)` after `expand_string()` (tab/control-char
expansion, not re-derived — `intin[]`-based, VDI-internal) in
`MD_TRANS` mode.

**`format_fnode(pf, pfmt)`** — the actual column layout, returns the
formatted line's length. Fields, in order:

1. **Indicator character** (1 byte): `0x07` (bullet, a special glyph in
   the desktop's font) if the entry is a folder (`FA_SUBDIR`); `0x7F`
   (another special glyph) if read-only (`FA_RO`); else a space.
2. *(wide format only)* one extra space.
3. **Name** (9 chars) + **extension** (3 chars), both space-padded,
   built by copying `f_name` up to the first `.` (max 9 chars — **not**
   validated against overflow beyond that count; a legally-8.3-formed
   name never exceeds this, so no bounds check is needed given TOS's own
   filename length limits) then the extension.
4. **Size**: for folders, blank-filled (8 or 11 chars depending on
   width, no digits shown at all — folder sizes are never displayed in
   the text view, unlike the "Show Info" dialogs which *do* show a
   folder's aggregate content size via `count_ffs`); for files,
   `fmt_size()` (right-justified, kbyte-scaled above 9,999,999 bytes on
   narrow screens only).
5. **Date**, then **Time** (space-separated, extra padding space on wide
   screens), via `fmt_date`/`fmt_time`.
6. **Trailing 2-space trim** (`:300-304`): if the formatted line's last
   two characters are both spaces (which only happens when using a
   24-hour clock, since `fmt_time`'s AM/PM suffix is otherwise
   non-blank), they're trimmed — explicitly to keep the line at a
   maximum of 36 characters so it still qualifies for a VDI "fast text
   output" fast-path in a maximum-width low-resolution window (comment,
   `:296-299`) — a **performance-motivated formatting rule**, not a
   display preference; a NewTOS reimplementation is free to always emit
   the two trailing spaces (or not) as long as the visually-relevant
   content (name/size/date/time) is unaffected — the trim exists purely
   as an EmuTOS-internal rendering optimisation.

**Wide vs. narrow format** (`USE_WIDE_FORMAT()`, not traced — presumably
a screen-width macro in `deskbind.h`/similar, gates the extra padding
spaces and `fmt_size`'s 8-vs-11-char field and the year's 2-vs-4-digit
display): affects total line length (32 vs. some larger count) but not
field order or semantic content.

**Contract note**: the *visual layout* of `V_TEXT` mode (what a user
sees) is behaviour a NewTOS desktop should reproduce for fidelity, but
it is not TOS-API contract in the trap-ABI sense — no program reads this
formatted string. It is documented here purely to resolve the
"needs deeper analysis" flags left by `_overview.md`/`desksupp-redraw.md`.

---

## `inf_file_folder`/`inf_disk` — Show Info dialogs

`inf_file_folder(ppath, pf)` (`:493-639`): for a folder, runs
`count_ffs()` (recursive `OP_COUNT`, [`deskdir.md`](deskdir.md)) and
populates file/folder/size fields via `inf_fifosz`; for a file, blanks
those fields and shows just its own size. Always shows name/date/time
(`inf_dttm`) and the read-only checkbox state. The dialog **loops**
(`while(1)`, `:551-634`) to allow retrying a failed rename: `FFOK`
commits both a possible attribute change (`dos_chmod`, **result not
checked** — "like Atari TOS, we don't check the return code," comment
`:592-593`) and a possible rename (`dos_rename`); on rename failure,
shows `STRENAME` and loops back to redisplay the same dialog with the
attempted (unsuccessful) new name still shown (`:632-633`, `Cancel`
breaks out, anything else retries the same values — no re-prompt for
different input; the field is still editable, the user just re-submits
the OK button after adjusting it). Returns `0` (cancel), `1` (continue,
no change), or `-1` (continue, mark for redraw — a rename or attribute
change happened).

`inf_disk(dr_id)` (`:650-693`): if `!valid_drive(dr_id)` (the drive is no
longer present), returns `1` immediately with **no dialog shown at all**
— this handles the case where the drive was removed between the user
selecting "Show Info" and the dialog actually opening. Otherwise counts
the whole drive's contents (`count_ffs("X:\*.*")`), reads free/total
space (`dos_space`) and the volume label (`dos_label` — if it fails,
label is blank, not an error), and shows the dialog.

---

## `inf_pref`/`inf_backgrounds`/`inf_conf` — preference dialogs

**`inf_pref`** (`:721-818`): pre-selects each radio/checkbox group from
the current `G.g_c*pref` globals, plus `evnt_dclick(0,FALSE)` (query,
don't set, the current double-click speed — an AES call). On OK,
reads every group back via `inf_which` and calls `evnt_dclick(newval,
TRUE)` to actually apply the double-click speed change immediately.
**Returns `TRUE` only if** the current view is `V_TEXT` **and** either
the time or date format actually changed (`:812-814`) — signalling to
the caller (`deskmain.c`'s `do_optnmenu`, per `deskmain.md`) that
open windows need rebuilding (since `V_TEXT` lines embed formatted
dates/times; `V_ICON` mode never shows dates, so a format change there
needs no rebuild).

**`inf_backgrounds`** (`CONF_WITH_BACKGROUNDS`, `:825-921`): a live
preview loop — every button press updates `tree[BGSAMPLE].ob_spec` (the
preview swatch) in place and redraws just that one field
(`draw_fld(tree, BGSAMPLE)`) before calling `form_do` again, rather than
running one `form_do` to completion. Tracks **desktop** and **window**
background candidates (`curdesk`/`curwin`) independently, switching
which one the swatch reflects based on the `BGDESK`/`BGWIN` radio
buttons. On `BGOK`: if the desktop background actually changed, applies
it immediately (`set_aes_background`, updates `G.g_patcol[index]` and
`G.g_screen[DROOT].ob_spec`, and calls `do_wredraw(DESKWH,&G.g_desk)`
directly — the desktop repaints itself before this function even
returns); if the **window** background changed, updates every open
window's border object (`G.g_screen[DROOT+1 .. WOBS_START-1]`, i.e. all
window-frame slots, whether or not currently occupied by an open
window) and returns `TRUE` (the caller must trigger a full rebuild for
window content areas, unlike the desktop case which is handled
in-function).

**`inf_conf`** (`CONF_WITH_DESKTOP_CONFIG`, `:1096-1246`): two
independent "scroll through a list" mini-UIs in one dialog — function
keys (`DCFUNPRV`/`DCFUNNXT`/`DCFUNLT`/`DCFUNRT` to move between assigned
function keys and horizontally scroll a long pathname within a fixed-
width field, `scroll_conf_funkeys`) and menu shortcuts
(`DCMNUPRV`/`DCMNUNXT` to move between the 33 (`NUM_SHORTCUTS`,
`deskmain.h:18`) shortcut-assignable menu items, editing a single
shortcut-key character per item via `save_shortcut`, which itself
handles duplicate-key detection with a user prompt). `DCMNUCLR` clears
**all** shortcuts at once, no per-item confirmation. On exit,
`DCOK` calls `install_shortcuts()` (`deskmain.c`, not re-derived — see
`deskmain.md`'s helper list) to copy the in-memory `menu_shortcuts[]`
array into the actual menu resource strings; any other exit calls
`recover_shortcuts()` to rebuild `menu_shortcuts[]` from the resource as
it currently stands (undoing any not-yet-committed edits made during the
dialog).

---

## Edge cases discovered in source

- **`inf_disk` silently no-ops for a removed drive** (`:662-663`) — no
  alert, just returns as if the user had clicked OK.
- **`dos_chmod`'s return code is deliberately ignored** in
  `inf_file_folder` (`:604-605`, explicit comment citing Atari TOS
  parity) — an attribute-change failure is invisible to the user.
- **`format_fnode` never shows a folder's own size** in the text view,
  even though the exact same information (aggregate folder size) *is*
  computed and shown in the "Show Info" dialog via `count_ffs` — two
  different code paths, two different answers to "what's this folder's
  size."
- **The trailing-2-space trim in `format_fnode`** exists purely for a
  VDI fast-path performance reason, not a display preference — see
  above.
- **`inf_backgrounds` applies the desktop-background change immediately,
  in-function**, but leaves the window-background change for the caller
  to apply via a full rebuild — an asymmetry between the two "background"
  concepts that a reimplementation must preserve (or verify unifying
  them doesn't change observable timing/flicker in ways that matter).
- **`inf_pref`'s rebuild signal is narrowly scoped**: only date/time
  format changes in `V_TEXT` mode trigger a `TRUE` return — a
  double-click-speed or confirm-preference change is applied immediately
  (`evnt_dclick`, the `G.g_c*pref` writes) but never signals "please
  rebuild," since none of those affect anything currently on screen.

---

## Error conditions

`deskinf.c` has few hard "error" paths (it's dialog logic, not I/O) —
the notable ones:

| Condition | Source location | Result |
|---|---|---|
| `count_ffs` fails (deep recursion/OOM, see `deskdir.md`) | `deskinf.c:525-529`,`:673-677` | `inf_file_folder`/`inf_disk` return `1` immediately, dialog never shown |
| `dos_rename` fails in `inf_file_folder`'s commit | `deskinf.c:622-633` | `STRENAME` alert; Cancel exits the loop (change so far, if any attribute change succeeded, still applies), else retries |
| `dos_label` fails (no volume label) | `deskinf.c:682-683` | label field left blank, not an error |
| Drive no longer valid | `deskinf.c:662-663` | `inf_disk` returns `1`, no dialog |

---

## Verification criteria

- **`format_fnode` column layout**: given a file `README.TXT`, 1234
  bytes, dated 2024-01-15, must produce a line with the exact fixed-width
  name/extension/size/date/time columns per the format above (folder
  indicator space, 9-char name, 3-char extension, size field, date,
  time) — a concrete byte-for-byte test is the natural verification here
  once NewTOS implements `V_TEXT` mode.
- **`inf_pref` rebuild signal**: changing only the double-click speed
  must return `FALSE`; changing only the date format while in `V_ICON`
  mode must also return `FALSE`; changing the date format while in
  `V_TEXT` mode must return `TRUE`.
- **`inf_disk` on a removed drive**: must show no dialog and return `1`.

---

## Dependencies

### Requires (must be initialized / functional)
- [`deskdir.md`](deskdir.md) — `d_doop`/`count_ffs`
- `desksupp.c` — `valid_drive`, `desk_busy_on`/`off`
- AES dialog primitives (`docs/api/aes/object-form.md`)

### Used by
- `deskmain.c`'s `do_filemenu`/`do_optnmenu` (see `deskmain.md`)
- [`deskdir.md`](deskdir.md), [`deskins.md`](deskins.md),
  [`deskfun.md`](deskfun.md) — all use `start_dialog`/`end_dialog`/
  `inf_show`/`inf_numset`/`set_tedinfo_name`/`draw_dial`/`draw_fld`
  (the last two are actually `deskdir.c`'s own, see
  [`deskdir.md`](deskdir.md) — not this file; cited here only because
  they're used alongside these dialog primitives throughout)
- `deskwin.c`'s `win_bldview()` — installs `dr_code` as the `V_TEXT`
  `USERBLK` callback (see [`deskwin.md`](deskwin.md))

## Related data structures

- `docs/api/desktop/_data-model.md` — `SCREENINFO.u.udef` (the
  `USERBLK` whose `ub_code`/`ub_parm` this file's `dr_code`/`FNODE*`
  fill in)
- `docs/api/structures/dta.md` — `FNODE`'s DTA-mirrored fields
  (`f_attr`/`f_date`/`f_time`/`f_size`) that `format_fnode` renders

## Cross-references

Not independently cross-checked against Compendium/Profibuch for the
dialog UI flows. The `EMUDESK.INF` format appendix below **was**
loosely cross-checked against general knowledge that Atari's original
EMUDESK.INF format is informally documented in period sources — no
specific page citation is available in this pass; treat the appendix as
EmuTOS-sourced only, per protocol Rule 1.

## Configuration / version notes

See [Build configuration](#emutos-source-deskinfc-itself) above
(`CONF_WITH_BACKGROUNDS`, `CONF_WITH_DESKTOP_CONFIG`). No TOS-version-
specific behaviour identified (EmuDesk-specific file).

---

## NewTOS implementation notes

### The dialog chain — `start_dialog` / `end_dialog` / `inf_show` (DESK08, 2026-09-23)

`programs/desktop/src/inf.rs` carries the three shared dialog primitives, at
the same place in the tree as `deskinf.c` carries them, and every
desktop dialog goes through them:

| EmuTOS | NewTOS | AES opcodes it issues |
|---|---|---|
| `start_dialog(tree)` (`:349-357`) | `inf::start_dialog(tree) -> Grect` | 54 `form_center`, 51 `form_dial(FMD_START)`, 42 `objc_draw` |
| `end_dialog(tree)` (`:363-390`) | `inf::end_dialog(tree)` | 54 `form_center`, 51 `form_dial(FMD_FINISH)`, 25 `evnt_multi` drain |
| `inf_show(tree, start)` (`:396-402`) | `inf::inf_show(tree, start) -> i16` | the two above + 50 `form_do` |
| `draw_dial(tree)` (`deskdir.c:79-85`) | `dir::draw_dial(tree)` | 54 `form_center`, 42 `objc_draw` |

Call sites, one for one with the source:

| dialog | EmuTOS | NewTOS |
|---|---|---|
| New folder / Set file mask / Search name box | `fun_mkdir` `start_dialog`+`form_do`+`end_dialog` (`deskfun.c:697,734`), `fun_mask` `inf_show` (`:655`) | `dlg::edit_name` → `inf_show(tree, ROOT)` |
| Show Info / rename | `inf_file_folder` → `inf_show` (`deskinf.c:574`) | `inf::inf_file_folder` → `inf_show(tree, ROOT)` |
| Set background | `inf_backgrounds` `start_dialog`/`draw_fld`-loop/`end_dialog` (`deskinf.c:862-896`) | `inf::inf_backgrounds` → `start_dialog` / loop with `dir::draw_fld(tree, BGSAMPLE)` / `end_dialog` (DESK64) |
| Install application | `ins_app_dialog` `start_dialog`/`draw_dial`-retry/`end_dialog` (`deskins.c:504,527,597`) | `insapp::ins_app_dialog`, same shape |

Three consequences, all of them the point of the change:

1. **The trees are built at the origin.** A NewTOS desktop dialog is a
   stack-built `OBJECT` array whose ROOT carries `ob_x = ob_y = 0`, the
   way a resource-file dialog does; `form_center` writes the centred
   position into the ROOT (`ob_center`, `aes/gemobed.c:81-82`). The
   four dialog builders used to compute `((sw-dw)/2, (sh-dh)/2)`
   themselves, which is **9 px too high** at `gl_hbox = 19`: EmuTOS
   centres in the screen *below the menu bar*,
   `yd = gl_hbox + (gl_height - gl_hbox - hd) / 2`.
2. **`objc_draw` is clipped to the dialog's own rectangle**, not to the
   whole screen.
3. **The undraw is `form_dial(FMD_FINISH)` over that rectangle**, and
   the desktop no longer repaints itself afterwards. Every
   `redraw::redraw_desktop()` that stood after a dialog in
   `desk::main` is gone: `set_background`, `ins_app_dialog`,
   `do_new_folder`, `do_info_rename`, `do_set_mask`, `do_search`.

**FMD_START does not photograph the screen**, in EmuTOS or here:
`fm_dial`'s `FMD_START` arm is a bare `break` with the comment "grab
screen sync or some other mutual exclusion method"
(`aes/gemfmlib.c:365-369`). The restore is the FMD_FINISH *repaint* —
`w_drawdesk(pt)` plus `w_update(DESKWH, pt, DESKWH, FALSE)`
(`:375-379`) — which is exactly why `end_dialog` has to drain the
message pipe itself: the desktop answers its own messages directly, so
an undrained `WM_REDRAW` would arrive out of sequence and show as a
partial refresh (EmuTOS says so at `deskinf.c:371-378`). NewTOS's drain
is the same loop over `evnt_multi(MU_TIMER|MU_MESAG, …)` +
`main::hndl_msg`.

**Set background since DESK64 (2026-10-17).** `inf::inf_backgrounds` builds
`ADBKGND` object for object (`desk_rsc.c:2230-2452`, `desk_rsc.h:251-260`:
the `BGDESK`/`BGWIN` radios, eight `BGPAT` and sixteen `BGCOL` `TOUCHEXIT`
swatches, the `BGSAMPLE` box, `BGOK`/`BGCANCEL`) and runs the source's loop:
`draw_fld(tree, BGSAMPLE)` before every `form_do`, the sample's
`FILLPAT_MASK`/`FILLCOL_MASK` bits taken from the clicked swatch, colours
`unused..15` `HIDETREE`d per `gl_nplanes` (`deskinf.c:832-852`; the desktop
asks `vq_extnd(handle, 1)` for the plane count, `gsx::gsx_nplanes`). On OK a
changed desktop background is applied and repainted in the function, a changed
window background goes into every window box and returns TRUE →
`BACKGROUND_HAS_CHANGED` → `desk_all` (`deskmain.c:824-827,438-447`). The
window boxes are `G_BOX`es whose `ob_spec` `set_background`
(`deskapp.c:1306-1331`) and the dialog set; `win_bldview` no longer forces a
white spec on them. The radio a user leaves selected is kept for the next call
of the same desktop run (EmuTOS's RAM tree, `desk_rs_init` once per `deskmain`).

Two named differences remain:

1. **Saved at once (deviation, user decision 2026-10-17, DESK64; to be
   generalised in DESK65).** EmuTOS writes the `.INF` only on the next *Save
   desktop* (`inf_backgrounds` ends at `deskinf.c:919`). NewTOS calls
   `cnx_put(); app_save(TRUE)` whenever OK changed either background —
   `cnx_put` first, as before every EmuTOS `app_save` (`deskmain.c:891,2154`).
2. **No `set_aes_background(curdesk & 0xff)` (`deskinf.c:904`).** It is an
   AES-internal function (`aes/geminit.c:635-641`) that the ROM desktop can
   call because it is linked into the AES; it sets the background the AES
   paints when a program is launched. The `NEWDESK` program has no AES call
   for it; the AES reads the byte from the `.INF` `#Q` line when it starts
   (`tos-api/src/aes/init.rs`, `geminit.c:719-728`). Open point, not decided.

Covered by `tos-api/tests/aes_desk_dialog_chain.rs` (placement, the
untouched surroundings, and the scope of the restore, all through the
real trap dispatcher into a pixel buffer).

---
---

# Appendix: the `EMUDESK.INF` file format (source: `desk/deskapp.c`, NOT `deskinf.c`)

> **Re-stating the naming note from the top of this file**: everything
> below is extracted from `desk/deskapp.c`, not `desk/deskinf.c`. It is
> placed in this document because the task brief asked for it under the
> name `deskinf.md`, and it is not duplicated anywhere else in the
> existing extraction (`deskapp.md` narrates the *loading process* —
> icon setup, grid metrics, `ANODE` list init — but does not give the
> line grammar). If the lead prefers this material physically relocated
> into `deskapp.md` (its rightful home by source file), that's a trivial
> move; the content is kept self-contained here so it isn't lost either
> way.

## Primary locations

- `desk/deskapp.c:295-436` `app_parse(pcurr, pa)` — parses **one**
  `ANODE`-describing line (`#T`/`#M`/`#G`/`#Y`/`#F`/`#P`/`#D`/`#I`/`#N`/
  `#X`/`#V`/`#O`) into an already-allocated `ANODE`
- `desk/deskapp.c:781-1014` `app_start(void)` — the full-file parse loop
  (the `switch(*pcurr)` over line-prefix characters, `:851-976`), calling
  `app_parse` for the `ANODE` lines and handling `#R`/`#Z`/`#W`/`#E`/`#Q`/
  `#K` inline
- `desk/deskapp.c:1106-1300` `app_save(todisk)` — writes every line back
  out, mirroring `app_start`'s format exactly
- `desk/deskapp.c:698-753` `build_inf(infbuf, xcnt, ycnt)` — synthesizes
  a **default** in-memory `EMUDESK.INF` (used when no saved file/shell
  buffer is found) from the two built-in string constants below plus a
  generated `#M`-line-per-drive loop
- `desk/deskapp.c:171-189` `desk_inf_data1`/`desk_inf_data2` — the
  literal built-in default-file text (the actual byte-for-byte default
  content, invaluable as ground truth for the format)
- `util/optimize.c:202-219` `scan_2(pcurr, pwd)` — the shared 2-hex-digit
  field parser **every** numeric field in the format uses (see
  [below](#shared-field-encodings))
- `util/optimize.c` `scan_str`(`deskapp.c:258-289`, despite living in
  `deskapp.c` it's declared for reuse, not `util/`) — the shared
  `@`-terminated string field parser

## Line-oriented structure

The file (or in-memory shell buffer) is one giant character buffer, not
literally split into records; both `app_start`'s reader and `app_save`'s
writer treat it as line-oriented only by convention: every logical line
begins with `#` immediately followed by a **one-character prefix**, and
ends with `\r\n` (written explicitly; the reader doesn't require it —
it just scans forward for the next `#`, `deskapp.c:848` `if (*pcurr++ !=
'#') continue;` — so any bytes between the end of one recognized field
and the next `#` are silently skipped, including a missing or malformed
`\r\n`). Fields *within* a line are whitespace-separated only by
convention of the specific parser called for that field; `scan_2` skips
leading spaces itself, `scan_str` also skips leading spaces before
capturing.

## Shared field encodings

- **2-hex-digit numeric field** (`scan_2`, `util/optimize.c:202-219`):
  skips leading spaces; if the next character is `\r` (end of line
  reached early), the value is `0` and no characters are consumed
  beyond that check; otherwise reads exactly 2 hex digits. **Special
  case: the value `0xFF` is converted to `-1`** — this is how
  `a_aicon`/`a_dicon` encode "no icon override" (`IG_APPL`/`IG_DOCU`
  defaults apply instead, per `app_parse:359-364`).
- **`@`-terminated string field** (`scan_str`, `deskapp.c:258-289`):
  skips leading spaces, copies characters into the shared `G.g_atext[]`
  arena (`SIZE_BUFF`=4096 bytes total, shared across **every** `ANODE`'s
  strings for the whole desktop session — `deskapp.h:76`) until `@`,
  `\r`, or NUL; the terminator (`@` or `\r`) is consumed, NUL is not. If
  the arena fills up, further characters are silently dropped (`KDEBUG`
  only, no user-visible error) — a hard **4 KB ceiling on the total
  size of all `ANODE` path/label/argument strings combined**, not
  per-string.
- **Single literal character field** (used only for the desktop drive
  letter): exactly one raw ASCII byte, not hex-encoded; a space means
  "no letter."

## Per-prefix line grammar

### `#R <rr>\r\n` — revision level

`deskapp.c:853-856` (parse), `:1128` (save: `"#R %02X\r\n"`,
`INF_REV_LEVEL`=`0x02`, `deskapp.c:129`). One 2-hex field,
`inf_rev_level`, consulted by `app_parse` to convert two legacy icon
index values (`IG_APPL_REV0`=8→`IG_APPL`, `IG_DOCU_REV0`=40→`IG_DOCU`,
`app_parse:370-375`) and by the desktop-shortcut fixup (swaps
`a_pappl`/`a_pdata` for old-format `X`/`V` entries if `inf_rev_level<2`,
`app_parse:406-423`, `CONF_WITH_DESKTOP_SHORTCUTS`). **Always written as
`INF_REV_LEVEL`=2 on save**, regardless of what was read (the file is
always upgraded to the current revision when re-saved).

### `#Z <cc> <path>@\r\n` — autorun program

`deskapp.c:857-859` (parse: `scan_str(pcurr+5, &pauto)` — the `<cc>`
byte is **skipped, never actually parsed** into anything; `pcurr+5`
jumps straight past `Z`+space+2 chars+space to the path), `:1131-1133`
(save: `"#Z %02X %s@\r\n"`, `pa->a_flags & AF_ISCRYS`). The path is held
in a local `pauto` pointer until a later `#G`/`#Y`/`#F`/`#P` line's
`a_pappl` matches it by `strcmp`, at which point that `ANODE` gets
`AF_AUTORUN` set (`app_start:880-887`) — **the `<cc>` (`AF_ISCRYS`) byte
written on save is redundant / write-only**, since the reader discards
it and instead re-derives everything about the autorun program from the
`ANODE` line it matches.

### `#E <e1> <e2> <vh> <vl> <e5>\r\n` — environment/desktop flags

`deskapp.c:914-951` (parse), `:1135-1174` (save,
`"#E %02X %02X %02X %02X %02X\r\n"`). Five 2-hex fields:

| Field | Bit | Meaning | Constant |
|---|---|---|---|
| `e1` bit 7 | `0x80` | view folder contents as text (else icons) | `INF_E1_VIEWTEXT` |
| `e1` bits 6-5 | `0x60` | sort mode 0-3 (`S_NAME`/`S_TYPE`/`S_SIZE`/`S_DATE`) | `INF_E1_SORTMASK` |
| `e1` bit 4 | `0x10` | confirm deletes | `INF_E1_CONFDEL` |
| `e1` bit 3 | `0x08` | confirm copies | `INF_E1_CONFCPY` |
| `e1` bits 2-0 | `0x07` | double-click speed 0-4 | `INF_E1_DCMASK` |
| `e2` bit 7 | `0x80` | blitter enabled | `INF_E2_BLITTER` |
| `e2` bit 6 | `0x40` | date format from `_IDT` cookie (ignore day/month bit) | `INF_E2_IDTDATE` |
| `e2` bit 5 | `0x20` | time format from `_IDT` cookie (ignore 24h bit) | `INF_E2_IDTTIME` |
| `e2` bit 4 | `0x10` | **allow overwrites** — inverted sense: `1` = allow, i.e. **do not** confirm overwrite (see [Edge cases](#edge-cases-in-the-format)) | `INF_E2_ALLOWOVW` |
| `e2` bit 2 | `0x04` | day-before-month (only if not `_IDT`-driven) | `INF_E2_DAYMONTH` |
| `e2` bit 1 | `0x02` | 24-hour clock (only if not `_IDT`-driven) | `INF_E2_24HCLOCK` |
| `vh`,`vl` | — | video mode, high/low byte of `Getrez()`/Falcon mode (`0xFF00\|Getrez()` if not Videl) — **read but discarded on load** (`scan_2(pcurr,&dummy)` ×2, `app_start:935-936`); only ever written, for informational/diagnostic value, never restored | — |
| `e5` bit 7 | `0x80` | do not sort folder contents (**overrides** the `e1` sort-mask entirely) | `INF_E5_NOSORT` |
| `e5` bit 6 | `0x40` | new-app launch: use app's own dir as CWD (else top window's) — `CONF_WITH_DESKTOP_CONFIG` | `INF_E5_APPDIR` |
| `e5` bit 5 | `0x20` | new-app launch: pass full path in args (else filename only) — `CONF_WITH_DESKTOP_CONFIG` | `INF_E5_ISFULL` |
| `e5` bit 4 | `0x10` | disable "size to fit" — `CONF_WITH_SIZE_TO_FIT` | `INF_E5_NOSIZE` |
| `e5` bit 3 | `0x08` | disable CPU cache — `CONF_WITH_CACHE_CONTROL` | `INF_E5_NOCACHE` |

Default (`INF_E1_DEFAULT`=`INF_E1_CONFDEL|INF_E1_CONFCPY|2`,
`INF_E2_DEFAULT`=`INF_E2_BLITTER|INF_E2_IDTDATE|INF_E2_IDTTIME`,
`INF_E5_DEFAULT`=`INF_E5_APPDIR|INF_E5_ISFULL`) is embedded verbatim in
`desk_inf_data1`: `"#E 1A E0 00 00 60\r\n"` (`deskapp.c:173`) — i.e.
`e1=0x1A`, `e2=0xE0`, video=`0000`, `e5=0x60`.

### `#Q <d1> <w1> <d2> <w2> <d3> <w3>\r\n` — background pattern/colour (`CONF_WITH_BACKGROUNDS`)

`deskapp.c:952-960` (parse), `:1176-1182` (save,
`"#Q %02X %02X %02X %02X %02X %02X\r\n"`). Three `(desktop, window)`
pattern+colour byte pairs, one pair per **plane-count bucket**
(`G.g_patcol[0]`=1-plane/mono, `[1]`=2-plane, `[2]`=more-than-2-plane —
see `_overview.md`'s config-variant note and `deskapp.md`'s
`set_background` narration). Each byte packs a fill-pattern index in the
high nibble and a colour index in the low nibble (per
`INF_Q1_DEFAULT`=`(IP_4PATT<<4)|BLACK` etc., `deskapp.c:118-123`).
Built-in default: `"#Q 41 40 43 40 43 40\r\n"` (`deskapp.c:175`) →
mono desktop=`0x41`, mono window=`0x40`, colour(2pl) desktop=`0x43`,
colour(2pl) window=`0x40`, colour(>2pl) desktop=`0x43`, colour(>2pl)
window=`0x40`.

### `#K <k0> <k1> … <k32>\r\n` — menu-item keyboard shortcuts

`deskapp.c:967-975` (parse — **always readable**, regardless of
`CONF_WITH_DESKTOP_CONFIG`, per the source comment "we allow all
versions of the ROM to _load_ menu item shortcuts", `:964-966`),
`:1184-1192` (save — **only written if `CONF_WITH_DESKTOP_CONFIG`**).
Exactly `NUM_SHORTCUTS`=33 (`deskmain.h:18`) 2-hex fields, one byte per
menu item slot, each holding an ASCII shortcut-key character (or `0x00`
for none) — see [`deskinf.md`](#inf_pref--inf_backgrounds--inf_conf--preference-dialogs)'s
`inf_conf` for how these are edited.

### `#W <hsl> <vsl> <xc> <yc> <wc> <hc> <00> <path>@\r\n` — saved window geometry

`deskapp.c:889-913` (parse), `:1195-1204` (save,
`"#W %02X %02X %02X %02X %02X %02X %02X"` then `" %s@\r\n"`). **Seven**
2-hex fields, then a path string:

| Field | Meaning | Unit |
|---|---|---|
| `hsl` | horizontal slider position | slider units (opaque, passed through as-is) |
| `vsl` | vertical slider position | slider units |
| `xc` | window X position | **character cells** — multiplied by `gl_wchar` on load (`app_start:897-898`), divided by `gl_wchar` on save (`app_save:1200`) |
| `yc` | window Y position | character cells, × `gl_hchar` |
| `wc` | window width | character cells, × `gl_wchar` |
| `hc` | window height | character cells, × `gl_hchar` |
| `00` | **unused placeholder**, always written as literal `0`, **read but never stored anywhere** — `app_start`'s parser does not even call `scan_2` for it, it does a raw `pcurr += 4` to skip " XX " (the field plus its two surrounding separators), `app_start:906` | — |

Up to `NUM_WNODES`=7 (`deskconf.h:30`) `#W` lines are read into
`cnxsave->cs_wnode[0..wincnt-1]` in **file order** (the `wincnt`
counter increments per line seen, capped at `NUM_WNODES` — excess `#W`
lines beyond the 7th are parsed for side effects (path chars still
consumed) but **not stored anywhere**, `app_start:891-912` — the `if
(wincnt < NUM_WNODES)` guard wraps the whole body). The **character-cell
→ pixel scaling** (`gl_wchar`/`gl_hchar`, the current font's cell size)
means a saved window position is **resolution/font-dependent** — moving
to a different video mode or font size before the next load shifts
every saved window position by the ratio of old to new cell size. Built-
in default (`desk_inf_data1`, `deskapp.c:177-180`): four `#W` lines all
reading `00 00 02 06/08/0A/0D 26 0C 00 @` — i.e. staggered cascading
default positions (row 6, 8, 10, 13 in character-cell Y), width `0x26`
=38 chars, height `0x0C`=12 chars, no saved path (empty, meaning "not yet
associated with an open directory" — the `#T`/`ANODE` lines are what
actually get displayed; these four blank `#W` lines are just pre-seeded
"remember where the next few windows the user opens should default to"
slots).

### `#<C> [<xx> <yy>] <aa> <dd> [<L>] <pappl>@ <pdata>@ [<f><kk> <pargs>@]\r\n` — one `ANODE`

`deskapp.c:295-436` (`app_parse`, the field reader shared by every
prefix below), `:1216-1283` (`app_save`'s mirroring writer). General
grammar (bracketed groups are conditional, detailed per-tag below):

| Segment | Present when | Encoding |
|---|---|---|
| `<C>` | always | 1-char tag, see table below |
| `<xx> <yy>` | tag implies `AF_ISDESK` (T/M/O/X/V) | 2×2-hex: desktop grid **column**/**row** (0-255) — see the dual-unit lifecycle already noted in `_data-model.md` |
| `<aa> <dd>` | always | 2×2-hex: `a_aicon`/`a_dicon` index into `G.g_iblist[]`; `FF`→`-1` |
| `<L>` | tag implies `AF_ISDESK` | 1 literal char: drive letter, or space for none |
| `<pappl>@` | always | `@`-terminated string, semantics per tag (see `deskapp.h:83-98`, already in `_data-model.md`) |
| `<pdata>@` | always | `@`-terminated string, semantics per tag |
| `<f><kk> <pargs>@` | `a_type==AT_ISFILE && !AF_ISDESK` (tags G/Y/F/P/I/N) | `<f>` = **one un-padded hex digit** (`INF_AT_APPDIR`=1 / `INF_AT_ISFULL`=2 bit flags), `<kk>` = 2-hex function-key number, then `<pargs>@` |

**Parser control-flow precision (re-verified against `app_parse`, `deskapp.c:385-396`, for the 1:1 rebuild)**: only the `<f><kk>` prefix is actually gated in the parser — `if (*pcurr == ' ') { …parse f/kk… }` (`:385-395`). The final `pcurr = scan_str(pcurr, &pa->a_pargs);` (`:396`) runs **unconditionally, for every tag**, immediately after that `if` block — it is not itself inside the conditional. For tags that never write the `<f><kk> <pargs>@` segment (`T`/`M`/`O`/`X`/`V`/`D`), `app_save` writes `\r\n` immediately after `<pdata>@`, so on read the `if (*pcurr==' ')` test fails (next char is `\r`) and the subsequent unconditional `scan_str` call sees `\r` immediately and yields an empty `a_pargs` — the **file-level grammar** (what's present in the bytes) is correctly gated as the table shows, but a faithful **1:1 function-structure port** of `app_parse` must keep the `a_pargs` scan outside the `if`, called once per line regardless of tag, not branch it per-tag.

Tag table (`app_parse:301-350` / `app_save:1218-1256`):

| Tag | `a_type` | `a_flags` set | Meaning | Config gate |
|---|---|---|---|---|
| `T` | `AT_ISTRSH` | `AF_ISDESK` | Trash icon | — |
| `M` | `AT_ISDISK` | `AF_ISDESK` | Disk/media icon | — |
| `O` | `AT_ISPRNT` | `AF_ISDESK` | Printer icon | `CONF_WITH_PRINTER_ICON` |
| `X` | `AT_ISFILE` | `AF_ISDESK` | File shortcut on desktop | `CONF_WITH_DESKTOP_SHORTCUTS` |
| `V` | `AT_ISFOLD` | `AF_ISDESK` | Folder shortcut on desktop | `CONF_WITH_DESKTOP_SHORTCUTS` |
| `D` | `AT_ISFOLD` | `0` | Folder (default type-association entry) | — |
| `G` | `AT_ISFILE` | `AF_ISCRYS` | GEM application, no parameters | — |
| `Y` | `AT_ISFILE` | `AF_ISCRYS\|AF_ISPARM` | GEM application, needs parameters | — |
| `F` | `AT_ISFILE` | `0` | TOS application/file, no parameters | — |
| `P` | `AT_ISFILE` | `AF_ISPARM` | TOS application, needs parameters | — |
| `I` | `AT_ISFILE` | `AF_WINDOW\|AF_ISEXEC` | Window icon, executable file | parsed unconditionally; **created** only via `CONF_WITH_WINDOW_ICONS` (see [Edge cases](#edge-cases-in-the-format)) |
| `N` | `AT_ISFILE` | `AF_WINDOW` | Window icon, non-executable file | same as `I` |

Every `M`/`T`/`O`/`G`/`Y`/`F`/`P`/`D`/`I`/`N`/`X`/`V` line is created via
`app_alloc()` (`app_start:876-878`) — if the fixed 64-entry `ANODE` pool
(`NUM_ANODES`, `deskapp.h:75`) is already exhausted, `app_alloc` itself
issues `STAPGONE` and `app_start` **returns immediately**, silently
abandoning the rest of the file's still-unparsed lines
(`app_start:877-878` — `if (!pa) return;`) — **a truncated
`EMUDESK.INF` beyond the 64th `ANODE` line is simply never read**, no
error surfaced beyond that one alert.

**Built-in default `#M`/`#T`/`#O` lines** (`build_inf`, `deskapp.c:698-753`,
used only when no `EMUDESK.INF`/shell-buffer data is found at all):
generated **not** from a string constant but computed per mounted drive:
`"#M %02X %02X %02X FF %c %s %c@ @\r\n"` — `xx`=`icon_index % xcnt`,
`yy`=`icon_index / xcnt` (row-major fill from the top-left, **same
placement rule** as `deskins.md`'s `install_drive`/`ins_posdisk`, though
this is an independent code path used only for the very-first-ever-boot
default), `aicon`=`IG_HARD`(drives >1) or `IG_FLOPPY`(A/B),
`dicon`=literal `FF`(-1, no override), drive letter twice (once as the
`<L>` field, once embedded in the label text `pappl` — "Disk C" for
example, `pdata` is empty (just `" @"` — a lone space then terminator,
i.e. an empty string after the leading-space-skip)). The Trash icon is
appended after all drives, positioned bottom-left unless a drive icon
would overlap that row (in which case bottom-**right**,
`build_inf:735-737`); the Printer icon (`CONF_WITH_PRINTER_ICON`) goes
to the *other* bottom corner, but **only if the Trash ended up on the
left** (`:745` `if (icon_x==0)`) — if the drive count pushed Trash to
the right, **no Printer icon is generated at all** in the built-in
default (the user would have to install one manually via
[`deskins.md`](deskins.md)'s `ins_icon`).

**Built-in default `#F`/`#N`/`#D`/`#Y`/`#G`/`#G`/`#P`/`#F` lines**
(`desk_inf_data2`, `deskapp.c:181-189`, literal constant) — these are
**generic per-extension type-default entries**, not tied to any specific
installed file (their `pappl` field holds a **wildcard**, e.g. `*.APP`,
not a real path — see the source comment at `deskapp.c:167-169`: "the
first `F` entry (for `*.*`) exists for compatibility... not otherwise
necessary"):

```
#F FF 07 @ *.*@\r\n      -> generic file, no app icon, IG_DOCU data icon, matches *.*
#N FF 07 @ *.*@\r\n      -> window-icon analogue of the same
#D FF 02 @ *.*@\r\n      -> generic folder, IG_FOLDER data icon
#Y 06 FF *.GTP@ @\r\n    -> GEM app needing parms, matches *.GTP, IG_APPL icon
#G 06 FF *.APP@ @\r\n    -> GEM app, matches *.APP
#G 06 FF *.PRG@ @\r\n    -> GEM app, matches *.PRG
#P 06 FF *.TTP@ @\r\n    -> TOS app needing parms, matches *.TTP
#F 06 FF *.TOS@ @\r\n    -> TOS app, matches *.TOS
```

---

## Edge cases in the format

- **`INF_E2_ALLOWOVW`'s sense is inverted relative to its name**: bit
  set (`1`) means "allow overwrites **without confirmation**" — the
  *preference* variable `cs_confovwr` is the **logical negation**
  (`app_start:925` `cnxsave->cs_confovwr = ((envr & INF_E2_ALLOWOVW) ==
  0);`) — reading the raw bit as "confirm overwrite = true when set"
  would be backwards.
- **The `#E` line's video-mode bytes are write-only** — computed fresh
  every save (`desk_get_videomode()`) but never restored on load
  (explicitly `scan_2(pcurr,&dummy)` twice, discarding both). Their only
  purpose is informational/diagnostic in a saved file, not functional.
- **The `#Z` autorun line's flag byte is write-only** in the same way —
  see above; the actual `AF_ISCRYS` state of the autorun program comes
  from whichever `#G`/`#Y`/`#F`/`#P` line's path matches, not from the
  `#Z` line's own byte.
- **`#W`'s 7th field is a placeholder that was never removed** — the
  source comment "skip no-longer-used field" (`app_start:906`) confirms
  this is legacy cruft kept only for file-format stability across
  EmuTOS versions (and, presumably, cross-compatibility with real Atari
  TOS's own EMUDESK.INF, which likely used this field for something once
  — not independently verified against Compendium/Profibuch in this
  pass).
- **Window position/size are stored in character cells, not pixels** —
  meaning a saved desktop layout is **not** resolution- or font-
  independent; loading a file saved under a different font size shifts
  every window.
- **`I`/`N` (window-icon) lines are parsed unconditionally**, regardless
  of `CONF_WITH_WINDOW_ICONS` — only their *creation* via the interactive
  "Install window icon" dialog is gated (see [`deskins.md`](deskins.md)).
  A file saved by a `CONF_WITH_WINDOW_ICONS`-enabled build, then loaded
  by a build without that feature, still gets those `ANODE`s created
  (they just can't be edited/created interactively going forward).
- **The flags nibble in the `G/Y/F/P/I/N` extra block is tested via a
  raw ASCII-bitwise-AND on the character itself, not a parsed hex
  value** (`app_parse:388-390`: `if (*pcurr & INF_AT_APPDIR)` where
  `INF_AT_APPDIR`=`0x01`, applied directly to the ASCII digit
  character `'0'`-`'3'`) — this only produces the correct result because
  the flags value is restricted to `0-3` and ASCII `'0'` (`0x30`) has
  bits 0-1 clear, so testing bits 0/1 of the character byte happens to
  equal testing bits 0/1 of the numeric value. A reimplementation that
  parses this nibble as an actual hex digit first (the "obviously
  correct" way) will still produce the identical result for the valid
  range `0-3`, but should not copy the raw-character-AND trick itself.
- **64-`ANODE`-pool exhaustion truncates the whole rest of the file
  silently** (`app_start:876-878`) — any lines after the 64th `ANODE`
  line (of any tag) are never read, not even `#W` lines that might
  physically appear later in the file (the file's line order is
  whatever `app_save` wrote — see next point).
- **File line order** (from `app_save`, `deskapp.c:1106-1300`): `#R`,
  then all `#Z` (autorun, at most one in practice since only one `ANODE`
  can carry `AF_AUTORUN`), then `#E`, then `#Q` (`CONF_WITH_BACKGROUNDS`),
  then `#K` (`CONF_WITH_DESKTOP_CONFIG`), then all 7 `#W` lines
  (always exactly `NUM_WNODES`, even for unopened window slots — their
  `pth_save` is empty), then every `ANODE` line in **reverse-of-reverse**
  order (`app_revit()` is called before *and* after the save loop,
  `:1213`/`:1287` — this ensures generic/built-in `ANODE`s are written
  before user-installed ones, so that re-reading via `app_alloc`'s
  head-insertion produces the same relative order as before the save —
  see the source comment at `:1206-1212`).

---

## Verification criteria (format)

- **Round-trip**: parsing a saved `EMUDESK.INF`, then immediately saving
  it again (with no user changes in between), must reproduce the
  **same set of `ANODE`s** with the same fields (byte-for-byte
  reproduction of the file is *not* guaranteed — e.g. video-mode/autorun-
  flag write-only fields may differ — but the resulting in-memory state
  after re-parsing the re-saved file must be identical to the original
  in-memory state).
- **`FF` sentinel**: an `aicon`/`dicon` field of `FF` must parse to `-1`
  (not `255`), verified via `pa->a_aicon == -1`.
- **`e2` overwrite bit inversion**: `#E .. 10 ..` (bit 4 set) must result
  in `cs_confovwr == FALSE` (confirmation **disabled**).
- **Window cell scaling**: a `#W` line with `xc=02` under a font with
  `gl_wchar=8` must produce `x_save == 16` pixels.
- **64-ANODE truncation**: a file with 65 `ANODE`-tagged lines followed
  by a `#W` line must result in exactly 64 `ANODE`s and the trailing
  `#W` line's data **not** overwriting any `cs_wnode[]` slot beyond
  whatever earlier `#W` lines (if any occurred before the 65th ANODE
  line) already set.

---

## Dependencies (format)

### Requires
- `deskapp.c`'s `ANODE` pool (`app_alloc`/`app_free`, `NUM_ANODES`=64)
- `G.g_atext[]` arena (`SIZE_BUFF`=4096 bytes, shared string storage)
- `gl_wchar`/`gl_hchar` (current font cell size, for `#W` scaling)

### Used by
- `deskapp.c`'s `app_start()` (load) / `app_save()` (save) — the only
  two callers of the format as a whole
- `deskmain.c`'s `SAVEITEM` (`cnx_put()`+`app_save(TRUE)`) and
  `app_read_inf()` (`CONF_WITH_READ_INF`, default on) for the "Read
  desktop" menu-equivalent reload — neither traced further in this pass

## Related data structures

- `docs/api/desktop/_data-model.md` — `ANODE`, the `a_xspot`/`a_yspot`
  dual-unit lifecycle (grid units on disk ↔ pixels in memory)
- `docs/api/desktop/deskapp.md` — `app_start`'s icon/grid metrics, which
  the `#M`/build-in-default placement math depends on (`G.g_icw`/`g_ich`)

## Cross-references

Not cross-checked against Compendium/Profibuch in this pass (see the
note in the main body above) — flagged for follow-up if NewTOS ever
adopts real-`EMUDESK.INF`-file compatibility as a goal (currently out of
scope per `_data-model.md`'s existing note on this topic).

## Configuration / version notes

- `CONF_WITH_BACKGROUNDS` — gates `#Q`
- `CONF_WITH_DESKTOP_CONFIG` — gates `#K`'s *write* (not its read) and
  the `e5` `APPDIR`/`ISFULL` fields' effect
- `CONF_WITH_SIZE_TO_FIT` / `CONF_WITH_CACHE_CONTROL` — gate two `e5`
  bits' effect (always parsed, only conditionally *applied*)
- `CONF_WITH_PRINTER_ICON` — gates the `O` tag and the built-in default's
  printer-icon generation
- `CONF_WITH_DESKTOP_SHORTCUTS` — gates `X`/`V` tags and the
  revision-0-format fixup for them
- `CONF_WITH_WINDOW_ICONS` — gates only the *interactive creation* of
  `I`/`N` tags, not their parsing (see edge case above)
- `INF_REV_LEVEL`=2 is the only revision EmuTOS currently writes;
  revision 0 and 1 read-compatibility fixups exist for icon-index
  remapping and (shortcuts only) `a_pappl`/`a_pdata` swap

---

## NewTOS implementation notes — `inf_pref` and the date/time formatters (DESK02)

`Options → Set preferences` had no handler (`Item::SetPrefs` fell through
`do_optnmenu`'s `_ => {}` and `men_update` kept it disabled). DESK02 implements
it as `programs/desktop/src/inf.rs::inf_pref`, run through DESK08's `inf_show` chain
and dispatched from `main::do_optnmenu` as `if inf_pref() { desk_all(…) }`
(`deskmain.c:885-887`).

### The tree

`ADSETPRE` (`desk/desk_rsc.c:1369-1560`) is an `OUTLINED` root box 40×21 with a
title, six labelled radio groups and an OK/Cancel pair. **Every group hangs under
a `G_IBOX` of its own**, and that box is load-bearing, not decorative: the AES
deselects siblings by walking the **parent's** child chain
(`fm_button` → `select_radio`, `tos-api/src/aes/form.rs`), so two groups sharing
a parent are one group as far as the AES is concerned. Pinned by
`tos-api/tests/aes_desk_prefs_radio.rs`, whose third case is the counter-example.

NewTOS's tree, in EmuTOS's field order:

| # | Object | Group |
|---|---|---|
| 0 | `G_BOX` root, 382×212 | |
| 1 | `G_STRING` `SET PREFERENCES` | |
| 2 / 3 / **4,5** | label, `G_IBOX`, ` Yes ` ` No ` | *Confirm deletes* (`SPCDYES`) |
| 6 / 7 / **8..12** | label, `G_IBOX`, `1`…`5` | *Double-click speed* (`SPDC1`) |
| 13 / 14 / **15,16** | label, `G_IBOX`, ` 12H ` ` 24H ` | *Time format* |
| 17 / 18 / **19,20** | label, `G_IBOX`, ` MDY ` ` DMY ` | *Date format* |
| **21, 22** | ` OK ` (`DEFAULT\|EXIT`), ` Cancel ` (`EXIT\|LASTOB`) | `SPOK`, `SPCNCL` |

Pre-selection and read-back are EmuTOS's, ported as free functions:
`inf_which` (`deskinf.c:705-716`), `inf_gindex` and `inf_what`
(`util/optimize.c:134-170`). The two format groups keep EmuTOS's
"read the value off the left-to-right button order" trick: with `12H` then `24H`
on screen `inf_which` yields 1 then 0, which *is* `TIMEFORM_12H` / `TIMEFORM_24H`
(`deskbind.h:68-69`); likewise `MDY`/`DMY` against `DATEFORM_MDY`/`DATEFORM_DMY`.

### Three fields of `ADSETPRE` are absent, and why

A preference that gates nothing would be a dead control, so it is left out rather
than shown and ignored:

* **Confirm copies** (`SPCCYES`/`SPCCNO`) — EmuTOS's switch turns on the
  per-item copy dialog inside `d_doop` (`deskdir.c:939`). NewTOS's copy engine
  has no per-item dialog at all (`programs/desktop/src/dir.rs`'s module header records
  that deviation), so there is nothing to turn on.
* **Confirm overwrites** (`SPCOWYES`/`SPCOWNO`) — same, one level deeper:
  EmuTOS asks before replacing an existing file (`deskdir.c:325`), NewTOS's copy
  always overwrites (GEMDOS `Fcreate` truncates).
* **The `Default` button of each format group** (`SPTF_DEF`/`SPDF_DEF`) —
  "default" means *take the format from the `_IDT` cookie*, which EmuTOS reads
  with `Supexec(get_idt_cookie)` (`desksupp.c:219`). XBIOS `Supexec` (0x26) is
  not implemented (`tos-api/src/xbios/mod.rs:129`) and the desktop may not reach
  kernel state except through a trap (DESK10), so the value cannot be honoured.

The `Idt` values are still **represented and round-tripped**: `env_to_prefs` /
`prefs_to_env` decode and re-encode `INF_E2_IDTDATE`/`INF_E2_IDTTIME` faithfully,
so a `NEWDESK.INF` written by EmuTOS keeps its bits. What resolves them is
`model::TimeFmt::rendered()` / `DateFmt::rendered()` — **one rule with two
readers**, the formatters and the dialog's pre-selection, so the box cannot show
a format the text view does not use. (Before that rule existed the measured run
showed `DMY` pre-selected while `format_fnode` printed MDY; the fallbacks had
been written twice and disagreed.) The fallbacks are `H24` and `MDY`, i.e. what
the text view printed before the preference existed.

### Where each field lives

| Field | Live state | Consumer | `#E` bits |
|---|---|---|---|
| Confirm deletes | `DeskState::g_cdelepref` | `dir::confirm_deletes()` gating `fun_del` and the window→Trash drop (`deskdir.c:906`) | `INF_E1_CONFDEL` |
| Double-click speed | the **AES**, via `evnt_dclick` (26); `g_cdclkpref` is only the cache `cnx_put` writes | `tos-api/src/aes/event.rs` `DCRATES` | `INF_E1_DCMASK` |
| Time format | `DeskState::g_ctimeform` | `inf::fmt_time` → `win::format_fnode`, `inf_file_folder` | `INF_E2_IDTTIME`/`INF_E2_24HCLOCK` |
| Date format | `DeskState::g_cdateform` | `inf::fmt_date` → the same two | `INF_E2_IDTDATE`/`INF_E2_DAYMONTH` |

The desktop→Trash drop is **not** gated: its source is a whole drive, and EmuTOS
puts `wants_to_delete_files` in front of that case unconditionally
(`deskfun.c:1049-1051`); the preference only reaches the per-item question
`d_doop` asks afterwards.

Persistence follows EmuTOS's split exactly: `app_start`'s `'E'` arm
(`deskapp.c:917-934`) → `CSave` (`cs_confdel`/`cs_dblclick`/`cs_timefmt`/
`cs_datefmt`, new in `global.rs`) → `cnx_get_prefs` → `GLOBES`; and back through
`cnx_put` (`deskmain.c:1380-1385`) → `app_save`'s env assembly
(`deskapp.c:1138-1160`) → the `#E` line.

### `fmt_date` / `fmt_time`

`win.rs` carried its own `fmt_date`/`fmt_time` with a fixed MDY / 24-hour format;
they now live in `inf.rs`, where EmuTOS keeps them (`deskinf.c:74-180`), and both
readers go through them. Two notes:

* `fmt_time` always writes **7 characters**: `hh:mm` plus `am`/`pm`, or plus the
  two blanks EmuTOS uses for the 24-hour case (`char *suffix = "  ";`,
  `deskinf.c:77`). That constant width is what lets the text view keep fixed
  columns; in 24-hour mode the trailing-blank trim removes it again, so the
  default line is byte-identical to the pre-DESK02 one.
  **DESK42 made the `hh`/`mm` separator an argument**, because EmuTOS varies it
  through the `sprintf` format string it hands in and its two callers differ:
  the text view passes `"%02d:%02d%s"` (`:293`) and `inf_dttm` `"%02d%02d%s"`
  (`:467`) — `ADFFINFO`'s `Time:   __:__ __  ` template already carries the
  colon, so a six-character `hhmm` + suffix is what that field wants. The text
  view's call passes `":"` and is unchanged.
* The **separator is a parameter**, and that is a divergence rather than a
  design. EmuTOS passes `DATEFORM_SEP` (`'/'`, `deskbind.h:64`) from both call
  sites; NewTOS's text view has printed `MM-DD-YY` with a dash since it was
  written. DESK02 binds the *order* to the preference and leaves each call site's
  separator as it was, so no screen changes that this ticket did not ask to
  change. The dash is reported as its own finding.

### `desk_all`'s argument — named deviation

EmuTOS calls `desk_all(FALSE)`, a pure repaint, because its text lines are
formatted at draw time by the `G_USERDEF` `dr_fnode` (`deskinf.c:310-326`).
NewTOS builds the line once in `win_bldview` → `format_fnode` and caches it in the
object's label, so a format change needs a **rebuild**: `do_optnmenu` passes
`VIEW_CHANGED`. `inf_pref`'s return value is unchanged — it is still `TRUE` only
for a text view whose time or date format actually moved (`deskinf.c:807-814`),
compared against the *rendered* old value so that leaving `Idt` for the `H24` the
screen already showed orders no rebuild.

### Measured

QEMU raspi4b, 640×480, `measurements/measurement-2026-09-24-desk01-desk02/`: the dialog lands
at **(129, 143, 382, 212)** = `form_center`'s value; `Date format → DMY` + `OK`
leaves the screen byte-identical to before the dialog opened; `Save desktop`
writes `#E 1A 86 00 00 60` (`0x86` = `BLITTER|DAYMONTH|24HCLOCK`, both `IDT` bits
now cleared because the user made an explicit choice); after a **cold boot** from
that image the dialog re-opens with `DMY` pre-selected, and the text view prints
`23-09-26` where the unchanged image prints `09-23-26`.

---

## NewTOS implementation notes — die drei nachgereichten Felder und der `_IDT`-Keks (DESK13, 2026-09-23)

DESK02 liess drei Felder von `ADSETPRE` weg, weil nichts dahinter stand. DESK13
hat die drei Hintergruende gebaut, also stehen die Felder jetzt drin — `inf_pref`
zeigt alle **sechs** Gruppen in EmuTOS' Reihenfolge unter EmuTOS'
Objektnummern (`desk_rsc.h:164-184`: `SPCDYES` 4, `SPCCYES` 8, `SPCOWYES` 12,
`SPDC1` 16, `SPTF_DEF` 23, `SPDF_DEF` 28, `SPOK` 31).

* *Confirm copies* / *Confirm overwrites* schalten jetzt wirklich etwas:
  den `ADCPYDEL`-Kasten in `dir_op` und den `ADCPALER`-Kasten in
  `output_fname` (siehe `deskdir.md`, Abschnitt DESK13).
* Der **`Default`-Knopf** beider Formatgruppen meint `DATEFORM_IDT` /
  `TIMEFORM_IDT`, also den `_IDT`-Keks. Der ist erreichbar — siehe unten.

Damit faellt auch DESK02s Behelf weg: die Vorbelegung liest wieder den **rohen**
Wert (`deskinf.c:754-778`, mit `default:` auf dem `Default`-Knopf), und die
Rueckgabe vergleicht rohe Werte (`:807-814`). `inf_which(tree, SPTF_DEF, 3)`
liefert 2/1/0 fuer `Default`/`12H`/`24H` — genau `TIMEFORM_IDT`/`_12H`/`_24H`
(`deskbind.h:68-70`), und ebenso `DATEFORM_IDT`/`_MDY`/`_DMY` (`:65-67`).

### Der `_IDT`-Keks: nicht `Supexec`, sondern `Ssystem`

EmuTOS liest ihn mit `Supexec((LONG)get_idt_cookie)` (`desksupp.c:219`) — nicht
weil der *Keks* Privileg braucht, sondern weil die Dose auf m68k nur ueber den
Zeiger bei `$5A0` im Supervisor-Speicher erreichbar ist. Das ist ein
m68k-Mechanismus, und `docs/COMPATIBILITY.md` stellt Mechanismen frei.

Auf NewTOS gibt es dafuer weder Bedarf noch Mittel: XBIOS `Supexec` (0x26) ist
eine offene ADR (`docs/api/xbios/26-supexec.md`, "NewTOS status: decision
deferred"), und die Keksdosen-Spezifikation benennt den Ersatz selbst — "There
is no `Supexec` in the kernel … the low-memory area an app wants is already in
its own address space" (`docs/api/bios/_cookie-jar.md` §Status 2026-09-14).
Der Desktop liest den Keks also **als Wert** ueber den GEMDOS-Trap:
`Ssystem(S_GETCOOKIE, '_IDT', 0)` (`docs/api/gemdos/154-ssystem.md`), gebunden
als `desk::dosbind::dos_getcookie` (seit DESK43 U6b `newtos_gem::gemdos::dos_getcookie`). Das dritte Argument ist 0, damit der Wert in
`x0` zurueckkommt und kein Zeiger durch den Trap geht — die Zeigerform schreibt
mit einem unprivilegierten `sttr` und faellt auf einer Kernadresse.
Gemessen und festgenagelt: `tos-api/tests/gemdos_ssystem_idt_cookie.rs`
(`_IDT = 0x0000112F`), und auf dem Brett
`[desk13] _IDT = 0x112f via Ssystem(S_GETCOOKIE)`.

`G.g_idt` (`deskbind.h:183`) wird aufgefrischt, wo EmuTOS es auffrischt — in
`do_wredraw` (`desksupp.c:219`) — und zusaetzlich einmal beim Desktopstart,
weil NewTOS die Textzeile beim **Bau der Ansicht** formatiert, EmuTOS dagegen
beim **Zeichnen** (`dr_fnode`, `deskinf.c:320`).

### Datumstrenner: eine Regel, ein Ort

Der `sep`-Parameter von `fmt_date` ist weg. EmuTOS faengt bei `DATEFORM_SEP`
(`'/'`, `deskbind.h:64`) an und ueberschreibt ihn nur im `DATEFORM_IDT`-Zweig
aus dem Keks (`deskinf.c:129,141-143`); beide Aufrufstellen uebergeben nichts.
NewTOS hatte den Trenner je Aufrufstelle festgeschrieben — die Textansicht
druckte `MM-DD-YY`, der Show-Info-Kasten `MM/DD/YYYY`. Eine Regel, zwei
Antworten; genau der Fehler, den DESK02 eine Ebene hoeher schon einmal gefunden
hatte.

Die Regel steht jetzt einmal, in `model::DateFmt::rendered(idt) -> (DateOrder,
u8)` und `model::TimeFmt::rendered(idt) -> TimeFmt`. `DateOrder` ist EmuTOS'
zweites `switch` (`deskinf.c:154-176`) mit allen vier `_IDT`-Reihenfolgen
(`MMDDYY`/`DDMMYY`/`YYMMDD`/`YYDDMM`, `biosdefs.h:168-171`); vorher konnte
NewTOS nur zwei davon ausdruecken. `fmt_date` formatiert die drei Felder mit
`%02d`-Semantik (`push_dec02`), weil EmuTOS' `sprintf(…"%02d%c%02d%c%02d"…)`
eine vierstellige Jahreszahl nicht abschneidet.

Mit `_IDT = 0x112F` (24H, DMY, `/`) und EmuTOS' eigener Voreinstellung ohne
`.INF` (`INF_E2_DEFAULT` traegt beide `IDT`-Bits, `deskapp.c:103`) druckt die
Textansicht `23/09/26 18:55` und der Show-Info-Kasten `23/09/2026 18:55` —
gemessen, `measurements/measurement-2026-09-23-desk13/screendumps/c-text.png` und
`e-showinfo.png`. Vorher waren es `09-23-26` und `09/23/2026`, also zwei
verschiedene Antworten auf dieselbe Frage.

### Nachtrag zur DESK02-Tabelle oben

Die Zeile *Confirm deletes* nennt `dir::confirm_deletes()`; die Funktion gibt es
nicht mehr. Der Leser ist `dir_op`s `confirm`-Schalter
(`programs/desktop/src/dir.rs`), und `CSave` traegt jetzt auch `cs_confcpy` und
`cs_confovwr` (`INF_E1_CONFCPY`, `INF_E2_ALLOWOVW` — letzteres **invertiert**,
`deskapp.c:98`: gesetzt heisst *nicht* nachfragen).

---

## NewTOS implementation notes — the 3D flag set and the About content (DESK01b)

Follow-up to DESK01 and DESK02 after the dialogs were seen on screen. Two
corrections: the desktop's hand-built trees carried **no** `FL3D*` flag, and the
About box's body was replaced with the approved credit block.

### 1. The flag set, derived from `desk_rsc.c` tree by tree

`desk_rsc.c` is a generated resource, so the flags are data, not policy. Every
object of all twenty trees was extracted mechanically (type, `RBUTTON` /
`EXIT` / `TOUCHEXIT`, root-or-not → 3D role) and the roles grouped; the result is
one convention with **no exception anywhere in the file** (`FL3DACT` is spelled
`FL3DBAK|FL3DIND` by the generator — the same `0x0600` pattern,
`obdefs.h:93-97`):

| object | 3D role | seen on |
|---|---|---|
| ROOT `G_BOX` | `FL3DBAK` | all 19 dialog trees, `ob_state = OUTLINED`, `ob_spec = 69888L`; `ADDINFO` alone `135424L` |
| `G_BUTTON` with `RBUTTON` (with or without `EXIT`) | `FL3DIND` | `SPCDYES`…, `APTOS`…, `FFRWRITE`…, and `BGDESK`/`BGWIN` (`:2254`, `RBUTTON` **and** `EXIT`) |
| `G_BUTTON` with `EXIT`, no `RBUTTON` | `FL3DACT` | every OK / Cancel / Skip / Install / Remove |
| `G_BUTTON` with `TOUCHEXIT` (arrow / stepper gadgets) | `FL3DACT` | `ID_UP`/`ID_DOWN` (`:990,996`), `IW_UP`/`IW_DOWN` (`:2094,2100`), `DCFUN*`/`DCMNU*` (`:1683-1749`) |
| `G_FTEXT` / `G_FBOXTEXT` field | `FL3DBAK` | every edit and display field; one exception, `DCMNUTXT` (`:1737`, flagless) |
| `G_IBOX` (radio-group parent) | none | `ADSETPRE` `:1390` ff. (`ob_spec = 0L`), `ADINSAPP` `:1071` ff. (`4352L`) |
| `G_IMAGE`, `G_STRING`, `G_TITLE` | none | throughout. Four `G_STRING`s in `ADINSDSK`/`ADINSWIN` carry a stray `FL3DBAK` (`:948,1002,2058,2106`); a `G_STRING` has no interior, so it is inert |
| `G_BOX` as a swatch, or as a sub-panel | none / `FL3DBAK` | `ADBKGND`'s `TOUCHEXIT` swatches (`:2280` ff.) flagless; `ID_IBOX`/`IW_IBOX`/`DCFUNBOX`/`FMT_BAR` `FL3DBAK` |

Two of those rows settle questions this ticket would otherwise have had to
reason its way to: a radio button that also carries `EXIT` is still an
**indicator** (`ADBKGND:2254`), and a stepper gadget is an **activator**
(`ADINSDSK:990`). NewTOS's Install-Application flavour buttons and the
Set-background radios `BGDESK`/`BGWIN` (since DESK64 the `ADBKGND` tree
itself; the `-`/`+` stepper buttons of the earlier stand-in are gone) are
exactly those cases, so they take `FL3DIND` and `FL3DACT` from the resource
rather than by analogy.

`desk_rsc.c` also sets `0x4000` / `0x8000` in some `ob_flags`. Those are
resource-editor bits: **no** EmuTOS AES code reads a flag above `SUBMENU`
(`0x0800`) — the only `ob_flags` tests in `aes/` are `SUBMENU`
(`gemmnext.c:410`), `FL3DOBJ` (`gemgrlib.c:375,379`), `INDIRECT`
(`gemobed.c:61`, `gemobjop.c:36`) and `EXIT`/`DEFAULT` (`gemobjop.c:61,63`).
They are therefore *not* copied.

### 2. The box spec: hollow is load-bearing

Every `desk_rsc.c` dialog ROOT carries `69888L` = `(1<<16) | 0x1100`; `ADDINFO`
alone carries `135424L` = `(2<<16) | 0x1100`. Cracked (`gr_crack`): **border
pen 1 (black), text pen 1 (black), inside pattern 0 (`IP_HOLLOW`), inside pen 0
(white)**, over a **positive** — i.e. *inner* — border of 1 (or 2) pixels.

NewTOS used `(1<<16) | 0x1070`: pattern **7** = `IP_SOLID`. That difference is
not cosmetic. `just_draw` only substitutes the 3D face colour when the box is
`IP_HOLLOW` **and** white (`gemoblib.c:619-636`), so an `FL3DBAK` flag on a
solid-white box would have been silently ignored and the panel would have stayed
flat white. The flag and the spec had to change together.

The sign is *not* changed: a dialog box's border is inner (positive). A100
measured the desktop's **drop-down** boxes as needing `0xFF1100` (thickness −1,
outer); that is the menu's convention, not the dialog's, and `menu.rs` keeps it.

The same colour word is the one every `desk_rs_tedinfo` entry uses
(`te_color = 4352` = `0x1100`, `desk_rsc.c:33-200`), so NewTOS's dialog
`TEDINFO`s move from `1 << 8` (border pen **0** = white) to `0x1100`.

**One named deviation on the fields.** `desk_rsc.c` gives its edit fields
`te_thickness = 0` — no frame — because they are *formatted* fields
(`G_FBOXTEXT` with an underscore `te_ptmplt`, e.g. `"Name: ________.___"`), and
the template is what shows the field's extent on the flat grey panel. NewTOS's
fields are plain `G_BOXTEXT` with an empty template, so `te_thickness = 1` is
kept: without it the field would be grey-on-grey and invisible. Measured:
`measurements/measurement-2026-09-24-desk01b/screendumps/mask-C-mask.png` / `zoom-field.png` — a 1-px black frame around the
`backgrcol` interior with the text and caret inside.

### 3. Geometry the 3D grow required

An `FL3DIND`/`FL3DACT` object is **grown** by `ADJ3DSTD` (2 px per side) before
it is drawn, and its negative border is laid outside *that* (`gemoblib.c:504`,
`tos-api/src/aes/object.rs`). A radio button therefore claims 3 px more per side
than its rectangle says, an exit button 4, a default exit button 5. Two trees had
been laid out tighter than that:

* `inf_pref`: row pitch 26 → **32** px and inter-button gap 6 → **8** px, both
  `ADSETPRE`'s own values (`desk_rsc.c:1386` ff. — char rows 3/5/7/9, i.e. 32 px;
  `:1462-1493` — 15 px buttons on a 23 px pitch). The dialog grows from
  382 × 212 to **390 × 244**. At the old pitch the borders of two neighbouring
  rows would have shared pixels.
* `ins_app_dialog`: the flavour row goes from 48 px buttons on a 52 px pitch to
  **44 px on 54**, and `APPMFILE` moves 240 → 244. `ADINSAPP` leaves the same
  room (40 px buttons on a 48 px pitch, `:1098-1124`). At the old pitch `GEM`'s
  right border and `TOS`'s left border overlapped by 4 px.

Nothing else needed to move; the remaining dialogs already had ≥ 2 px clear,
which is what `desk_rsc.c` itself leaves.

### 4. The About box's content

`do_aboutdesk`'s body is now the approved credit block, laid out on `ADDINFO`'s
row grid (blank rows at 1, 4, 7 and 10, as `desk_rsc.c:790-866` leaves char
rows 3, 6, 8 and 13 empty, and row 15 as the gap above the OK button):

```
                    - NewTOS -

        Version    NewTOS 0.6.1
        Machine    Raspberry Pi 4 (BCM2711)

     NewTOS is an independent project -
          not original Atari software.

   Reimplemented from the EmuTOS sources
   (c) 2001-2026 The EmuTOS development team

            Based on 'GPLed' sources
          (c) 1987 Digital Research, Inc.
       (c) 1999 Caldera Thin Clients, Inc.
              (c) 2001 Lineo, Inc.
```

**2026-10-09:** the box carried a further row 16, a NewTOS authorship line
(`Adapted by …`) with no `ADDINFO` counterpart. It was dropped on user decision
— the boot screen had dropped it the same day (`a2b58038`) — which moves the
content closer to EmuTOS's `ADDINFO`.

Everything but the `Version`/`Machine` pair block is centred; the pair block is
`ADDINFO`'s `DELABEL`/`DEVERSN` arrangement (`desk_rsc.c:800-813`).

**Where the strings live, and why.** Version, machine name and the independence
notice stay in `crate::initinfo` because the **boot
screen prints them too** — that is EmuTOS's own arrangement, `desk_xlate_fix`
pointing `DEVERSN->ob_spec` at the BIOS `version[]` the boot screen prints
(`deskmain.c:1880`). The six credit lines are **local to the dialog**, for the
same reason EmuTOS keeps them local: `bios/initinfo.c` prints none of the
`'GPLed'` chain, only `desk_rsc.c`'s `ADDINFO` carries it.

**Wording.** `Based on 'GPLed' sources`, `1987 Digital Research, Inc.`,
`1999 Caldera Thin Clients, Inc.` and `2001 Lineo, Inc.` are `ADDINFO`'s strings
verbatim (`desk_rsc.c:834-856`), with EmuTOS's `\275` — the Atari charset's ©
glyph — written as `(c)`. `(c) 2001-2026 The EmuTOS development team` is
`ADDINFO`'s `"Copyright \275 2001-"` + `COPYRIGHT_YEAR` + `"The EmuTOS
development team"` (`:806-828`, patched at `deskmain.c:1892` from
`obj/header.h:19`); NewTOS has no build-year constant, so the span is a literal.
`Reimplemented from the EmuTOS sources` has no `ADDINFO` counterpart — EmuTOS
*is* the code it credits.

**Size.** The box grows from 384 × 276 to **384 × 436** — the width is unchanged
because the longest line (the 42-character authorship line) is still wider than
the machine line on either board. *(Since 2026-10-09, without the authorship
row: **376 × 396**; the longest line is now the 41-character EmuTOS copyright
line. The figures below are the 2026-09-24 measurement.)* At 640 × 480 it is centred at (128, 31) and the
`OUTLINED` 3D outline reaches (125, 28, 390, 442): 9 px clear below the menu bar,
10 px above the bottom edge. It fits on both board profiles; the qemu-virt
machine name is shorter, so the box is the same size there.

### Measured (`measurements/measurement-2026-09-24-desk01b/`)

QEMU raspi4b, 640 × 480, the DESK08/09/10 scratch-copy IKBD injector.

* **Desk → Desktop info… → OK** (`about-*.png`). The box measures, by scanline,
  `x 128..511`, `y 31..466` = (128, 31, 384, 436) — `form_center`'s value; the
  `draw_3d_outline` sits at 125/126-127 (grey/white-white) on the left, 512-514
  (grey) on the right, and the ROOT's own 2-px black border at 128-129 / 510-511,
  i.e. `ADDINFO`'s `(2<<16)` thickness. Interior `189,189,189` = `GRAY_LT` =
  `backgrcol`. The `OK` button carries the `add_3d_effect` bevel (white top/left,
  `GRAY_DK` bottom/right inside a 3-px black default-exit frame,
  `zoom-ok.png`). `about-D-closed.png` == `about-A-idle.png`, md5
  `710d7f24cc078d43e9351725a205a16e`, **0 differing pixels**.
* **Options → Set preferences… → Cancel** (`prefs-*.png`). The dialog measures
  (125, 127, 390, 244) = `form_center`'s value. All four groups render as 3D
  indicators — unselected raised, selected in the darker face with a sunken
  bevel and white text (`zoom-radios.png`); pre-selection is `Yes / 3 / 24H /
  MDY`, unchanged from DESK02. `prefs-D-closed.png` == `prefs-A-idle.png`,
  **0 differing pixels**.
* **Options → Set background… → Cancel** (`bkgnd-*.png`, smoke check for the
  four stepper buttons the flag change also touches): bevelled, 2 px clear
  between neighbours (`zoom-steppers.png`); closed == idle, **0 differing
  pixels**.
* **File → Set file mask… → Cancel** (`mask-*.png`, smoke check for the
  `EDITABLE | FL3DBAK` field): the field renders as a framed `backgrcol` box
  with `*.*` and the caret; closed == the window before it, **0 differing
  pixels**.
* 0 `[EXC]` / `[PANIC]` in all four serial logs.

Not driven here (they need typing or a multi-step selection, and the Tester's
harness covers them): `inf_file_folder`'s Show-Info box and `ins_app_dialog`.
Real-Pi4-only: the 1920 × 1080 look of all of these.

---

## NewTOS implementation notes — `inf_conf` (DESK05)

`Options → Desktop configuration…` had no handler (`Item::DeskConfig` fell
through `do_optnmenu`'s `_ => {}` and `men_update`'s catch-all kept it greyed).
DESK05 implements it as `programs/desktop/src/inf.rs::inf_conf`, run through DESK08's
`start_dialog` / `form_do` / `end_dialog` chain and dispatched from
`main::do_optnmenu` as the bare `inf_conf();` of `deskmain.c:895-898`.

`CONF_WITH_DESKTOP_CONFIG` defaults to **1** (`include/config.h:1774-1776`); the
`0` at `:349-351` is the non-Atari/minimal-ROM block, not our Falcon030/TOS4
profile. `CONFITEM` appears in no `ILL_*` table and in no `menu_ienable` call
(`deskmain.c:143-160`, `:464-609`), so the item is **always enabled** — that is
the `men_update` arm, beside `PREFITEM`.

### The scope line against DESK02

None: `ADSETPRE` (`inf_pref`, DESK02) and `ADDESKCF` (`inf_conf`) are two
different resource trees behind two different menu items, with no field in
common. `inf_pref` owns the `G.g_c*pref` preferences; `inf_conf` owns
`G.g_appdir`/`G.g_fullpath`, the `menu_shortcuts[]` array and two read-outs.

### The tree

`ADDESKCF` (`desk_rsc.c:1603-1772`) is an `OUTLINED` 39 × 22-cell root with a
title, two radio groups, two framed panels each holding a "step through a list"
mini-UI, a free-RAM line and OK/Cancel. NewTOS builds it at 8 × 16 from
`ADDESKCF`'s own character grid, **minus the two radio rows** (see below), so
everything from *Function keys* down moves up by four character rows and the
root loses the same four cells: 312 × 288 px, `form_center` → ROOT (164, 105),
reported rect (161, 102, 318, 294).

| # | Object | EmuTOS name |
|---|---|---|
| 0 | `G_BOX` root `FL3DBAK` + `OUTLINED`, 312 × 288 | |
| 1 | `G_STRING` `DESKTOP CONFIGURATION` (centred by `align_title`) | |
| 2 | `G_STRING` `Function keys:` | |
| 3 | `G_BOX` `FL3DBAK` + `OUTLINED`, `ob_spec` `0xFF1100` | `DCFUNBOX` |
| 4 | `G_FTEXT` `F__` / `"9"` / `TE_CNTR` / `4480` / th −1 / txtlen 3 | `DCFUNNUM` |
| 5 | `G_BUTTON` `TOUCHEXIT\|FL3DACT` `"\004"` | `DCFUNLT` |
| 6 | `G_FBOXTEXT` 22 `_` / `"X"` / `4352` / th 0 / txtlen 23 | `DCFUNPTH` |
| 7 | `G_BUTTON` `TOUCHEXIT\|FL3DACT` `"\003"` | `DCFUNRT` |
| 8, 9 | `G_BUTTON` `TOUCHEXIT\|FL3DACT` `"\001"` / `"\002"` | `DCFUNPRV`, `DCFUNNXT` |
| 10 | `G_STRING` `Menu shortcut keys:` | |
| 11 | `G_BOX` `FL3DBAK` + `OUTLINED`, `0xFF1100` | `DCMNUBOX` |
| 12 | `G_FTEXT` `EDITABLE\|FL3DBAK` `Key: ^_` / `"a"` / `4480` / th −1 / txtlen 2 | `DCMNUKEY` |
| 13 | `G_BUTTON` `TOUCHEXIT\|FL3DACT` `Clear all` | `DCMNUCLR` |
| 14 | `G_FBOXTEXT` **no 3D flag** (the resource's `NONE`), 32 `_` / `4352` / th −1 / txtlen 33 | `DCMNUTXT` |
| 15, 16 | `G_BUTTON` `TOUCHEXIT\|FL3DACT` `"\001"` / `"\002"` | `DCMNUPRV`, `DCMNUNXT` |
| 17 | `G_FTEXT` `Free RAM: ` + 21 `_` / `4480` / th 0 / txtlen 22 | `DCFREMEM` |
| 18, 19 | ` OK ` (`DEFAULT\|EXIT`), ` Cancel ` (`EXIT\|LASTOB`), both `FL3DACT` | `DCOK`, `DC_CNCL` |

Three details of the resource that a reimplementation has to take rather than
invent:

* **Two colour words, not one.** `4480` (`0x1180`) is REPLACE-mode text — bit 7
  set, `gr_crack` — used for the three fields that sit directly on the panel
  face (`DCFUNNUM`, `DCMNUKEY`, `DCFREMEM`); `4352` (`0x1100`) is the
  TRANSPARENT one used for the two scrollable fields. `te_thickness` likewise
  varies per field (−1 frames it, 0 does not).
* **`DCMNUTXT` carries no 3D flag** while every other field in the tree carries
  `FL3DBAK`, so it stays a white box on the grey panel.
* **`adjust_3d_positions`** (`deskmain.c:1647-1656`) moves `DCFUNPRV` and
  `DCMNUPRV` left by `3 * ADJ3DSTD` = 6 px "to avoid button overlap", because
  the resource puts them flush against their `NXT` neighbour and a 3D activator
  grows 2 px per side plus its `th = −1` border. Reproduced literally.

### `menu_shortcuts[]`: the machinery the dialog edits

The shortcut letters live in a 33-byte array, not in the menu strings; the
strings only ever carry a rendering of it. The chain, and where each piece now
lives in NewTOS (`programs/desktop/src/menu.rs` unless noted):

| EmuTOS | NewTOS |
|---|---|
| `menu_shortcuts[NUM_SHORTCUTS]` (`deskmain.c:119`) | `MENU.shortcuts`, reached through `shortcut_key` / `set_shortcut_key` / `shortcuts` / `set_shortcuts` |
| `shortcut_mapping[]` (`:169-260`) | `SHORTCUT_MAPPING: [Option<Item>; 33]` |
| `default_shortcuts[]` (`:269-307`) | `DEFAULT_SHORTCUTS` |
| `copy_menu_items` (`:1755-1813`) | the RAM label buffer `MENU.labels`, filled in `build_menu` |
| `install_shortcuts` (`:1825-1858`) | `install_shortcuts`, called from `install_menu_bar` |
| `recover_shortcuts` (`deskinf.c:1062-1090`) | `recover_shortcuts` |
| `lookup_ascii_shortcut` (`deskmain.c:1169-1198`) | `lookup_ascii_shortcut`, now array-driven |
| `case 'K'` / `#K` (`deskapp.c:961-975`, `:1183-1193`) | `InfData::shortcuts` in `programs/desktop/src/inf.rs` |

Consequences worth naming:

* **The drop-downs are three characters wider and now show `^X`.** EmuTOS's
  `copy_menu_items` adds `1 + SHORTCUT_SIZE` to the widest entry of each menu
  (`:1781`) and pads every entry to `width - 1` characters; `adjust_menu` then
  measures those padded strings, so the box is `widest + 4` cells.
  `install_shortcuts` writes `'^'` and the letter at
  `(ob_width + CHAR_WIDTH - 1) / CHAR_WIDTH - SHORTCUT_SIZE` (`:1845`). NewTOS
  showed no shortcuts at all before DESK05; it now shows EmuTOS's defaults
  (`^O ^I ^N ^H ^U ^W ^A ^D ^Z` in *File*, `^R ^S` in *Options*). Measured box
  widths: Desk 22 cells, File 23, View 20, Options 30.
* **Separators are generated, not literal.** `copy_menu_items` skips them
  (`:1773`), so they still point at the resource's one-character `"-"` when
  `adjust_menu` measures, and `adjust_menu` then re-points each at `m` dashes
  (`:1623`). NewTOS's `build_menu` fills the RAM label with `cells` dashes for
  the same reason; the hand-written dash-run literals are gone.
* **The `#K` line is now written**, between `#Q` and the `#W` lines, in
  `app_save`'s order (`deskapp.c:1176-1193`). EmuTOS lets every build *read* it
  and only a `CONF_WITH_DESKTOP_CONFIG` build write it (`:961-965`); ours has
  the dialog, so it does both. A slot whose menu item this build does not have
  is cleared by `install_shortcuts` (`:1833-1839`), which is what keeps a
  foreign `#K` from re-saving a shortcut for an absent item.
* **Nothing here writes the file.** `inf_conf` ends at `end_dialog`; the new
  shortcuts reach `NEWDESK.INF` through *Save desktop*, exactly as in EmuTOS.

### *Default dir* is back (DESK18), *Parameter* is back (DESK19)

Same rule as DESK02: a control whose effect is not implemented is left out
rather than shown and ignored — and it comes back when the effect lands. Both
have now landed, so `ADDESKCF` is complete: every one of its 28 objects is in
the tree and the numbering is the resource's own (`desk_rsc.h:191-211`).

* **Default dir** (`DCDEFAPP`/`DCDEFWIN` → `G.g_appdir`) picks the current
  directory a *non-installed* program is launched with: `do_aopen` chooses the
  top window's path or the program's own and calls `set_default_path()`
  (`desksupp.c:896-916`). DESK05 left the group out because NewTOS's launch
  path set no current directory at all. **DESK18 built that step**
  (`desk::pro::set_default_path` + `desk::main::set_launch_dir`, see
  `deskpro.md` §*The current-directory step*), so the group is in the tree
  again, as `ADDESKCF` objects 2..5 verbatim.
* **Parameter** (`DCPMFULL`/`DCPMFILE` → `G.g_fullpath`) picks whether a file
  dropped on a program icon is passed as a full path or a bare name
  (`build_selected_path`, `deskfun.c:903-928`), reached from `fun_win2win`'s
  drop-on-an-application branch (`deskfun.c:958-970`). DESK05 left it out
  because NewTOS's `fun_win2win` had no such branch — a drop on a program was
  a copy. **DESK19 built that branch** (see `deskfun.md` §*Dropping a
  selection on a program*), so this group is back too, as `ADDESKCF` objects
  6..9 verbatim.

**NewTOS notes (DESK18, DESK19).**

* **Object numbers.** `ADDESKCF`'s own, 0..27 (`desk_rsc.h:191-211`), with
  EmuTOS's names, as in `inf_pref`. DESK05's four-lower shift and DESK18's
  two-lower shift are both gone.
* **Geometry.** The box is `ADDESKCF`'s full `39 x 22` cells = **312 x 352**;
  `form_center` puts the ROOT at (164, 73) on a 640x480 screen, measured. No
  row is displaced any more, which also restores the blank character row the
  resource leaves between *Parameter* and *Function keys*. That spacing
  matters: every 3D gadget here is grown by `ADJ3DSTD` (2 px per side) and
  then laid out one pixel of border wider still (`aes/gemoblib.c:504`,
  `sst_th` `G_BUTTON` → `-1`), so the 8 px a blank cell is worth is nearly
  eaten by the growth. The two groups' inner spacing is the resource's own
  (*Application* 12 cells at column 0, *Window* 9 at column 13; *Full path*
  11 cells at column 0, *File name* 10 at column 12).
* **Each group has its own `G_IBOX`** (`ADDESKCF` objects 3 and 7,
  `ob_spec = 4352` = `0x1100`, no 3D flag), because that is what makes the AES
  deselect the sibling of the button that was hit — and only the siblings
  *under the same parent* (`tos-api/tests/aes_desk_prefs_radio.rs`). Two
  groups under one box would clear each other. A `G_IBOX` draws nothing — its
  thickness byte is 0 and `objc_draw` skips the interior — so carrying the
  resource's colour word costs nothing and keeps the tree faithful.
* **Pre-select and commit are EmuTOS's** (`deskinf.c:1105-1116`, `:1226-1227`):
  `deselect_all` is implicit (every `ob_state` starts at 0), `G.g_appdir`
  selects `DCDEFAPP` or `DCDEFWIN` and `G.g_fullpath` selects
  `DCPMFULL`/`DCPMFILE`, and `OK` reads both back with `inf_which(tree, …, 2)`
  — 1 = the first of the pair. Any other exit leaves both alone.
* **Persistence.** `G.g_fullpath` rides `CSave::cs_fullpath` through
  `cnx_put`/`cnx_get` (`deskmain.c:1392`, `:1454`) into the `#E` line's byte 5
  as `INF_E5_ISFULL` (`deskapp.c:111`, `:943`, `:1165`) — the bit that used to
  round-trip untouched now has both a writer and a reader.

### The function-key panel is a viewer, and it has something to view (DESK20)

`get_all_funkeys` (`deskinf.c:928-948`) collects the `ANODE`s whose `a_funkey`
is in `FIRST_FUNKEY..LAST_FUNKEY`; `DCFUNPRV`/`DCFUNNXT` walk them and
`DCFUNLT`/`DCFUNRT` scroll the path inside the fixed-width field
(`scroll_conf_funkeys`, `:952-991`). **Nothing in this dialog assigns a function
key** — EmuTOS assigns them in `ins_app_dialog` (`deskins.c`, `APFUNKEY`), and
the display is read-only there too. Until DESK20 NewTOS's Install-Application
dialog had no such field, so the panel was *always* empty; it now shows what
that dialog assigned (see `deskins.md` §`ins_app`). With nothing bound it still
shows its template, which is exactly what EmuTOS shows on a fresh
`EMUDESK.INF`. EmuTOS walks `G.g_ahead`, the one list that holds both desktop
icons and associations; NewTOS splits those two, so the walk is over
`desk::inf`'s `SESSION` model, which is the half that carries `a_funkey`.

Measured 2026-09-23 (`measurements/measurement-2026-09-23-desk19-desk20/`): after installing
`C:\CMDLINE.NCP` with key 7, the panel reads `F7  C:\CMDLINE.NCP______`
(`measurements/measurement-2026-09-23-desk19-desk20/screendumps/d20-funkey-d-conf.png`).

### Free RAM

`dos_avail_stram()` / `dos_avail_altram()` = `Mxalloc(-1, MX_STRAM|MX_TTRAM)`
(`util/gemdos.c:100-113`), new bindings in `programs/newtos-gem/src/gemdos.rs` (until DESK43 U6b kernel/src/desk/dosbind.rs). EmuTOS
gates the second term on `has_alt_ram`; NewTOS gates it on a non-zero return,
which is the same condition one step later (`pmdalt` null ⇒ 0).

### Measured (2026-09-23, QEMU raspi4b, 640 × 480, IKBD injector)

`measurements/measurement-2026-09-23-desk05/`. ROOT (164, 105, 312, 288), `form_center` rect
(161, 102, 318, 294). Stepping `DCMNUNXT` walks Open (`^O`) → Info/Rename…
(`^I`) → Search… (no key). *Clear all* + *Cancel* leaves the File drop-down
**byte-identical** to before (`recover_shortcuts`); Cancel and OK-with-no-change
both leave the desktop byte-identical. Backspace + `x` + OK turns Open's
shortcut into `^X` in the drop-down and into `#K` slot 0 = `58` on disk, and a
cold boot from that image brings it back. `Ctrl-S` still runs *Save desktop*
through the rewritten array-driven lookup. 0 `[EXC]`/`[PANIC]` in all six
serial logs.

Note for a future reader: the key field shows a **blank** when the slot has no
shortcut (`init_conf_shortcuts` writes `' '`, `deskinf.c:1006-1010`) but an
**underscore** right after *Clear all* (`inf_sset(tree, DCMNUKEY, "")`,
`:1221`, so the template shows through). That asymmetry is the source's, not a
defect.

## `inf_disk` — the `ADDISKIN` box (DESK38, 2026-09-25)

`inf_disk(dr_id)` (`deskinf.c:650-693`) did not exist in NewTOS; it is the
first of `do_info`'s four arms (see
[`desksupp-redraw.md`](desksupp-redraw.md)) and also what `SHOWITEM` falls back
to when nothing is selected (`deskmain.c:683-684`).

```c
if (!valid_drive(dr_id)) return 1;
desk_busy_on();
srcpth = "X:\\*.*"; more = count_ffs(srcpth);
if (!more) { desk_busy_off(); return 1; }
dos_space(dr_id - 'A' + 1, &total, &avail);
desk_busy_off();
if (!dos_label(dr_id - 'A' + 1, label)) label[0] = '\0';
inf_sset(tree, DIDRIVE, drive);
set_tedinfo_name(tree, DIVOLUME, label);
inf_fifosz(tree, DINFILES, DINFOLDS, DIUSED);
inf_numset(tree, DIAVAIL, avail);
inf_show(tree, ROOT);
return inf_what(tree, DIOK);
```

Three details of the source that are easy to lose, and that the implementation
keeps:

* **`total` is read and never shown.** The dialog has a field for `avail`
  only; the *used* figure is `inf_fifosz`'s `G.g_size`, i.e. the bytes the
  **walk** added up — not `total - avail`. On a drive with cluster slack,
  `used + available < total`, and that is TOS's arithmetic.
* **`inf_fifosz` decrements the folder count** — "reproduce TOS's folder count
  in this situation" (`:448`). `count_ffs` → `d_doop` counts the directory it
  was started on as well (its post-order `dirs++`), and the drive root is not
  a folder the user put there.
* **A missing drive is not silent.** `valid_drive` (`desksupp.c:1810-1826`)
  puts up `STNODRIV` and `inf_disk` then returns `1`, so a `SHOWITEM` loop
  carries on to the next icon.

`dos_space` and `dos_label` are new in `desk::dosbind`: `Dfree` with EmuTOS's
`buf[3]*buf[2]` arithmetic (done in 64 bits here — a 500 MB FAT16 partition
already overflows EmuTOS's signed 32-bit byte count), and `Fsfirst` on
`"X:\\*.*"` with attribute `0x08`, which matches volume labels exclusively.
`valid_drive` is new in `desk::main`; `check_alt_letter_key` keeps its own
inline drive-map test, because EmuTOS filters impossible letters out *before*
calling `valid_drive` for exactly that path (`deskmain.c:1147-1152`).

DESK38 built the box as seven hand-laid `G_STRING` lines (prompt and value in
one string) in a 340 × 230 rectangle. **DESK42 replaced that with `ADDISKIN`
itself** — see the next section; the paragraph that used to stand here, and
argued that a `G_STRING` needs no 3D flag because it has no interior, described
a tree that no longer exists.

### `inf_file_folder` returns three answers (DESK38, 2026-09-25)

It returned `Option<String>`, in which *Cancel* and "OK, nothing changed" were
the same `None`. `do_info`'s contract needs them apart: its `0` stops the
`SHOWITEM` loop. The return became `InfFF::{Cancel, Continue, Renamed(name)}`,
EmuTOS's `0`/`1`/`-1` (`:487-491`).

DESK38 named two deviations here: EmuTOS renames inside `inf_file_folder`, in a
`while(1)` that re-shows the whole dialog after a `STRENAME` *Retry*
(`:551,629-634`), and it writes back a changed read-only attribute
(`:591-609`), while NewTOS handed the new name back and had a read-only
attribute row. **DESK42 closed both**, and with them the `InfFF` enum: the
function is `inf_file_folder(ppath, pf) -> i16` again, EmuTOS's own signature
and its own `0`/`1`/`-1`.

### Measured (DESK38)

`measurements/measurement-2026-09-25-desk-rest/` §3 — `DISK INFORMATION` for `C:`: label
`NEWTOS` (through the `FA_VOL` `Fsfirst`), 11 folders, 40 files, 1 597 248
bytes used, 65 241 088 available. Pi checklist item 69, where the four real
partitions make the `count_ffs` walk long enough to be worth watching.

## NewTOS implementation notes — the two boxes are the resource trees (DESK42, 2026-09-25)

DESK38 reproduced *Show info*'s **behaviour**; its two boxes were laid out by
hand and, in the user's words, had nothing in common with EmuTOS's. DESK42
rebuilds both trees object for object out of `desk_rsc.c` — same order, same
object numbers, same labels, same measurements — the way DESK03 did `ADINSDSK`
and DESK05 `ADDESKCF`.

### Reading `desk_rsc.c`'s numbers

`ob_x`/`ob_y`/`ob_width`/`ob_height` in the generated resource are **packed**:
`LOBYTE` counts character cells, the signed `HIBYTE` adds pixels
(`fix_chpos`, `aes/gemrslib.c:53-83`). `ADDISKIN`'s ROOT height `2060` is
`(8 << 8) | 12` = 12 rows + 8 px, and its buttons' `2058` is row 10 + 8 px. At
the 8 × 16 cell that is:

| tree | cells | pixels |
|---|---|---|
| `ADDISKIN` (`:709-768`, `desk_rsc.h:75-84`) | 38 × (12 rows + 8 px) | **304 × 200** |
| `ADFFINFO` (`:622-703`, `desk_rsc.h:61-72`) | 40 × 16 | **320 × 256** |

Measured at run time: `form_center` puts `ADDISKIN`'s ROOT at (168, 149) and
`ADFFINFO`'s at (160, 121) on a 640 × 480 screen.

### `ADDISKIN` — ten objects

| # | name | type / flags | rect (cells) | content |
|---|---|---|---|---|
| 0 | ROOT | `G_BOX`, `FL3DBAK`, state `OUTLINED`, `ob_spec` 69888 | 0,0,38,12+8px | |
| 1 | `DITITLE` | `G_STRING`, no flag | 3,1,16,1 | `DISK INFORMATION` |
| 2 | `DIDRIVE` | `G_FBOXTEXT`, `FL3DBAK` | 3,3,32,1 | ted[6], `TE_RIGHT` |
| 3 | `DIVOLUME` | `G_FBOXTEXT`, `FL3DBAK` | 3,4,32,1 | ted[7], `TE_LEFT` |
| 4 | `DINFOLDS` | `G_FBOXTEXT`, `FL3DBAK` | 3,5,32,1 | ted[8], `TE_RIGHT` |
| 5 | `DINFILES` | `G_FBOXTEXT`, `FL3DBAK` | 3,6,32,1 | ted[9], `TE_RIGHT` |
| 6 | `DIUSED` | `G_FBOXTEXT`, `FL3DBAK` | 3,7,32,1 | ted[10], `TE_RIGHT` |
| 7 | `DIAVAIL` | `G_FBOXTEXT`, `FL3DBAK` | 3,8,32,1 | ted[11], `TE_RIGHT` |
| 8 | `DIOK` | `G_BUTTON`, `SELECTABLE\|DEFAULT\|EXIT\|FL3DACT` | 7,10+8px,9,1 | `OK` |
| 9 | `DICNCL` | `G_BUTTON`, `SELECTABLE\|EXIT\|LASTOB\|FL3DACT` | 22,10+8px,9,1 | `Cancel` |

### `ADFFINFO` — fourteen objects

| # | name | type / flags | rect (cells) | content |
|---|---|---|---|---|
| 0 | ROOT | `G_BOX`, `FL3DBAK`, `OUTLINED` | 0,0,40,16 | |
| 1 | `FFTITLE` | `G_STRING`, no flag | 11,1,1,1 | `STFIINFO` / `STFOINFO` |
| 2 | `FFNAME` | `G_FBOXTEXT`, `EDITABLE\|FL3DBAK` | 4,3,26,1 | ted[0], `TE_LEFT` |
| 3 | `FFSIZE` | `G_FBOXTEXT`, `FL3DBAK` | 4,4,26,1 | ted[1], `TE_RIGHT` |
| 4 | `FFDATE` | `G_FBOXTEXT`, `FL3DBAK` | 4,5,26,1 | ted[2], `TE_RIGHT` |
| 5 | `FFTIME` | `G_FBOXTEXT`, `FL3DBAK` | 4,6,26,1 | ted[3], `TE_RIGHT` |
| 6 | `FFNUMFOL` | `G_FBOXTEXT`, `FL3DBAK` | 4,8,32,1 | ted[4], `TE_RIGHT` |
| 7 | `FFNUMFIL` | `G_FBOXTEXT`, `FL3DBAK` | 4,9,32,1 | ted[5], `TE_RIGHT` |
| 8 | — | `G_IBOX`, no flag, `ob_spec` 0 | 2,11,36,1 | parent of 9/10 |
| 9 | `FFRWRITE` | `G_BUTTON`, `SELECTABLE\|RBUTTON\|FL3DIND` | 0,0,18,1 | `Read/Write` |
| 10 | `FFRONLY` | `G_BUTTON`, `SELECTABLE\|RBUTTON\|FL3DIND` | 19,0,17,1 | `Read-Only` |
| 11 | `FFOK` | `G_BUTTON`, `SELECTABLE\|DEFAULT\|EXIT\|FL3DACT` | 2,14,10,1 | `OK` |
| 12 | `FFSKIP` | `G_BUTTON`, `SELECTABLE\|EXIT\|FL3DACT` | 15,14,10,1 | `Skip` |
| 13 | `FFCNCL` | `G_BUTTON`, `SELECTABLE\|EXIT\|LASTOB\|FL3DACT` | 28,14,10,1 | `Cancel` |

Objects 9 and 10 are positioned **inside** object 8, which is what makes the
AES deselect the sibling of the button that was hit — `RBUTTON` deselection is
per parent, the same reason `ADDESKCF`'s two radio groups each have their own
`G_IBOX` (DESK18/DESK19 above).

### The `TEDINFO`s, and why the lines are not flush left

`desk_rs_tedinfo[0..11]` (`desk_rsc.c:34-93`) all share `te_font = IBM`,
`te_color = 4352` (`0x1100`) and **`te_thickness = 0`** — so none of these
"fields" has a frame; the template's underscores are the whole of what the user
sees as a field. Their `te_txtlen`/`te_tmplen` are copied verbatim, which is
what makes the merge and the edit behave as they do:

| field | template | `te_txtlen` | `te_tmplen` | `te_just` |
|---|---|---|---|---|
| `FFNAME` | `Name: ________.___` | 12 | 19 | `TE_LEFT` |
| `FFSIZE` | `Size:   __________` | 11 | 19 | `TE_RIGHT` |
| `FFDATE` | `Date:   __________` | 11 | 19 | `TE_RIGHT` |
| `FFTIME` | `Time:   __:__ __  ` | 7 | 19 | `TE_RIGHT` |
| `FFNUMFOL` | `Number of folders:         _____` | 6 | 33 | `TE_RIGHT` |
| `FFNUMFIL` | `Number of files:         _____` | 6 | 31 | `TE_RIGHT` |
| `DIDRIVE` | `Drive identifier:  _           ` | 2 | 32 | `TE_RIGHT` |
| `DIVOLUME` | `Disk label:  ________.___` | 12 | 26 | `TE_LEFT` |
| `DINFOLDS` | `Number of folders:         _____` | 6 | 33 | `TE_RIGHT` |
| `DINFILES` | `Number of files:         _____` | 6 | 31 | `TE_RIGHT` |
| `DIUSED` | `Bytes used:    __________` | 11 | 26 | `TE_RIGHT` |
| `DIAVAIL` | `Bytes available:    __________` | 11 | 31 | `TE_RIGHT` |

`te_just` does **two** things, and the second one is easy to miss: `ob_format`
fills the template's underscores from the left or the right end, and
`gr_gtext`/`gr_just` (`gemgraf.c:567-589`) then place the *whole formatted
string* left, centred or right inside the object rectangle. `ADDISKIN`'s six
fields all sit at char column 3 with the same 32-cell rectangle, but only
`DINFOLDS`'s template is 32 characters long; the shorter `TE_RIGHT` ones are
pushed right by their slack. The left edges that come out of that, relative to
the box, are 32 / 24 / 24 / 40 / 80 / 40 — the staircase the screendump shows
is the resource's own, not a layout defect.

### The three helpers the trees needed

* **`fmt_str` / `unfmt_str`** (`util/optimize.c:43-93`) — `SAMPLE.PRG` ↔
  `SAMPLE  PRG`, the form the `FFFFFFFF.FFF` template wants. `FFNAME` shows the
  formatted name and `inf_file_folder` unformats it back before renaming;
  `set_tedinfo_name` (`deskinf.c:424-430`) is `fmt_str` + `inf_sset` and is how
  `DIVOLUME` gets the disk label.
* **`inf_numset`** (`:409-419`) — `sprintf("%*lu", te_txtlen-1, value)`: the
  number right-aligned in the field's capacity. A value with more digits than
  the field holds keeps its **last** `te_txtlen-1` digits here; EmuTOS's
  `sprintf` writes the whole number and overruns into the next field's storage,
  but what the user sees is the same, because a `TE_RIGHT` `ob_format` fills
  from the right end of the text. Unreachable for both callers on FAT16.
* **`align_title`** (`deskmain.c:1731-1747`) — object 1 centred on the box when
  it is a `G_STRING` one character row down. EmuTOS moves `ob_x` and leaves
  `ob_width` alone, which is sound because a `G_STRING` is drawn from `ob_x`
  with no clamping (`gemoblib.c:817-845`); `ADFFINFO`'s title object really is
  one cell wide in the resource. **This is the classic-TOS arm.**
  `CONF_WITH_ALT_DESKTOP_GRAPHICS` is 1 in the reference build and left-aligns
  the title with an underline instead; every NewTOS dialog centres, and
  changing that for all of them at once is `TASK/GUI05.md`.

### `inf_file_folder` is EmuTOS's function again

`inf_file_folder(ppath, pf) -> i16`, with the `while(1)` of `:551-635` inside
it, and the caller (`desksupp.c:1335-1337`) reduced to the two lines the source
has. That closes DESK38's two named deviations:

* **the attribute write-back** (`:585-605`) — `dos_chmod(srcpth, F_SETMOD,
  attr)`, a new `desk::dosbind` binding on GEMDOS `Fattrib` (0x43). Its result
  is deliberately not checked: "like Atari TOS, we don't check the return code
  from dos_chmod()" (`:592-593`);
* **the `STRENAME` retry** (`:628-634`) — a failed `dos_rename` puts up
  `[1][Rename error!][ Retry | Cancel ]` (`desk_rsc.c:2557`) and *Retry* loops
  back to `inf_show` with the attempted name still in the field.

For a **folder** the box now counts its contents (`count_ffs` → `inf_fifosz`
into `FFNUMFIL`/`FFNUMFOL`/`FFSIZE`, `:519-533`); for a **file** those two
objects get `DISABLED` and an empty text, so the template shows through greyed
(`:535-545`). `FFSKIP` — the third exit button, which NewTOS did not have — is
`inf_what(tree, FFSKIP)`'s `1`: continue with the next item of the `SHOWITEM`
selection without changing this one.

**One named deviation remains.** EmuTOS's `pf` is the window's live `FNODE`, so
a changed name or attribute is visible in it at once; ours is the copy
`do_info` made from it (`desksupp.c:1326` builds exactly such a copy for the
desktop-shortcut case, so the shape is the source's). The window is refreshed
by the rebuild the same `-1` triggers — `fun_mark_for_rebld` +
`fun_rebld_marked` re-scan the directory — so the two agree by the time
anything reads them again.

### Measured (DESK42)

`measurements/measurement-2026-09-25-desk42/`. Both trees dumped object by object at run
time (`measurements/measurement-2026-09-25-desk42/serial/dlg-dumps.txt`) and
 screendumped: `ADDISKIN` on the C: drive,
`ADFFINFO` for `C:\README.TXT` (FILE INFORMATION) and for `C:\SUB` (FOLDER
INFORMATION, 3 files / 0 folders / 6059 bytes), the Read-Only switch read back
off the card (`attr` 0x20 → 0x21), and the `STRENAME` *Retry* bringing the same
box back. Zero `[EXC]`/`[PANIC]` in ten serial logs, `fsck_msdos -n` clean on
ten card images.

**Side finding, filed separately and not fixed here:** GEMDOS `Frename` accepts
an **empty** destination file name and writes a directory entry with an
all-blank 8.3 name (`tos-api/src/bdos/fs/dir.rs:148`, `fcb_name("")` →
`Some([b' '; 11])`). EmuTOS's `xrename` refuses it at `bdos/fsdir.c:912-913`.
EmuTOS's `inf_file_folder` has no empty-name special case and neither does
ours any more, so emptying the name field and pressing OK now reaches the
defect; the pre-DESK42 NewTOS box rejected an empty name itself, which EmuTOS
does not do. Reproduced in `measurements/measurement-2026-09-25-desk42/` §6.

---

## `opn_appl` — the command-tail prompt (DESK47, 2026-09-30)

### EmuTOS

`opn_appl(papname, ptail)` (`deskinf.c:1250-1271`, the last function of the
file) is the *"Open application icon"* box: it asks for the command line of a
program that takes parameters, and it is the only place `ADOPENAP` is used.

```c
tree = desk_rs_trees[ADOPENAP];

set_tedinfo_name(tree, APPLNAME, papname);
inf_sset(tree, APPLPARM, "");
inf_show(tree, APPLPARM);

/* now find out what happened */
if (inf_what(tree, APPLOK))
{
    inf_sget(tree, APPLPARM, ptail);
    return TRUE;
}

return FALSE;
```

Four details of those six lines matter:

* **`inf_show(tree, APPLPARM)`, not `ROOT`.** The start field is the parameter
  line, so the edit cursor opens there. Every other `deskinf.c` dialog passes
  `ROOT` and lets `fm_inifld` find the first editable object
  (`aes/gemfmlib.c:153-160`); here the resource's one editable object is named
  explicitly.
* **The name is shown through `set_tedinfo_name`**, i.e. `fmt_str` into the
  `________.___` template (`CMDLINE.NTP` → `CMDLINE NTP`), not as typed.
* **`inf_what`'s third answer counts as TRUE.** `if (inf_what(tree, APPLOK))`
  is a plain non-zero test, so the `-1` returned when neither button ended up
  `SELECTED` also launches the program. `0` — Cancel — is the only answer that
  does not.
* **The tail field is `X`-validated** (`desk_rs_tedinfo[13]`), so it admits
  every character `check()` lets through, including bytes above 0x7E.

### Where it sits in the flow

`do_aopen` (`desksupp.c:969-986`), **after** `set_default_path` and the
`app_path` rejoin, inside the `isapp` branch:

```c
ret = TRUE;
if (tail)        strcpy(ptail, tail);     /* a file was dropped on the program */
else if (isparm) ret = opn_appl(pname, ptail);
return ret ? pro_run(isgraf, app_path, G.g_work, G.g_cwin, curr) : FALSE;
```

So the prompt is asked **only** when `tail == NULL` — the double-click /
File→Open path (`do_open` → `do_aopen(…, NULL)`, `desksupp.c:1229`) and the
function-key path (`process_funkey` → `do_aopen(pa, TRUE, -1, …, NULL)`,
`deskmain.c:1099`). A file dropped on a program icon (`fun_win2win`,
`deskfun.c:967`) brings its own tail and skips the box. The
`installed_datafile` arm returns before this point entirely (`:951`): for a
document association the document *is* the tail.

`isparm` is `pa->a_flags & AF_ISPARM` (`desksupp.c:877`), which `app_parse`
sets for exactly `#P *.TTP` and `#Y *.GTP` (`deskapp.c:181-188`, `:310-322`).

### `ADOPENAP` — seven objects

`desk_rsc.c:877-919`, `desk_rsc.h:93-98`. Packed measurements as in DESK42
above (`LOBYTE` cells + signed `HIBYTE` pixels): the ROOT's `2058` is
`(8 << 8) | 10` = 10 rows + 8 px and the buttons' `2056` is row 8 + 8 px, so at
the 8 × 16 cell the box is **320 × 168** with the buttons at y 136…152 — room
for the 2 px per side an `FL3DACT` object grows by before it is drawn.

| # | name | type / flags | rect (cells) | content |
|---|---|---|---|---|
| 0 | ROOT | `G_BOX`, `FL3DBAK`, state `OUTLINED`, `ob_spec` 69888 | 0,0,40,10+8px | |
| 1 | `APPLOPEN` | `G_STRING`, no flag | 11,1,16,1 | `OPEN APPLICATION` |
| 2 | `APPLNAME` | `G_FBOXTEXT`, `FL3DBAK` | 1,3,21,1 | ted[12], `TE_LEFT` |
| 3 | — | `G_STRING`, no flag | 1,5,11,1 | `Parameters:` |
| 4 | `APPLPARM` | `G_FBOXTEXT`, `EDITABLE\|FL3DBAK` | 1,6,38,1 | ted[13], `TE_LEFT` |
| 5 | `APPLOK` | `G_BUTTON`, `SELECTABLE\|DEFAULT\|EXIT\|FL3DACT` | 7,8+8px,9,1 | `OK` |
| 6 | `APPLCNCL` | `G_BUTTON`, `SELECTABLE\|EXIT\|LASTOB\|FL3DACT` | 24,8+8px,9,1 | `Cancel` |

Object 3 is the one object of the tree `desk_rsc.h` does not name — nothing in
`deskinf.c` addresses it.

The two `TEDINFO`s, from the same `desk_rs_tedinfo[]` the Show-Info boxes draw
on (`desk_rsc.c:34-104`; `te_font = IBM`, `te_color = 4352`,
`te_thickness = 0`, so neither field has a frame of its own):

| field | template | `te_pvalid` | `te_txtlen` | `te_tmplen` | `te_just` |
|---|---|---|---|---|---|
| `APPLNAME` (ted[12]) | `Name: ________.___` | `f` | 12 | 19 | `TE_LEFT` |
| `APPLPARM` (ted[13]) | 38 × `_` | `X` | 39 | 39 | `TE_LEFT` |

`APPLNAME` shares its template literal (`rs_str_Name______________`,
`desk_rsc.c:18`) with `ADFFINFO`'s `FFNAME`, so the name is shown in the same
8.3-split form the Show-Info box uses. `APPLPARM`'s 39-byte text is the hard
limit on the tail this box can produce — well inside `CMDTAILSIZE`.

### NewTOS

`programs/desktop/src/inf.rs` `opn_appl(papname, ptail)` — the tree built object for
object with [`mk`]/[`mk_root`]/[`rsc_ted`]/`align_title`, exactly as DESK42
built `ADDISKIN`/`ADFFINFO`. `ptail` is a `Vec<u8>` rather than a `String`
because of the `X` validation above; `inf_sget` is added next to `inf_sset` as
its `util/optimize.c:119-126` counterpart.

The call sites are the two `do_aopen` embodiments in `programs/desktop/src/main.rs`:
`open_item`'s `App` arm (double click / File→Open) and `do_aopen_drop`, whose
`tail` parameter became `Option<&[u8]>` so that EmuTOS's `tail == NULL`
distinction is carried rather than approximated by an empty string — `None` is
`process_funkey`, `Some` the three drop paths in `programs/desktop/src/dir.rs`.

**Closed by DESK50 (2026-10-01), kept here for the history:** the paragraph
below described the state after DESK47. `do_wopen`, `do_diropen` and
`pro_run`'s `(wh, curr)` pair are built; see `desksupp-wopen.md`.

*Was missing in the same corner, and named rather than half-built:*
`pro_run`'s `(wh, curr)` pair and `do_wopen` (`desksupp.c:274-322`). `do_wopen`
has two callers in EmuTOS — `pro_run` (`deskpro.c:61`) and `do_diropen`
(`desksupp.c:454`) — and `do_diropen` itself has four (`deskfun.c:413`,
`deskmain.c:1496`, `desksupp.c:1067`, `:1156`). NewTOS has neither function:
the `do_diropen` body is spread over `open_dir_window_at`, `navigate_window`,
`rebuild_window`, `open_root_window` and the `cnx_get` restore path, none of
which does `do_wopen`'s `graf_growbox` zoom or its
`act_chg(…, FALSE, FALSE)` deselect. Building `do_wopen` for `pro_run` alone
would produce a function wearing the name without being it; it needs its own
unit, together with a `graf_growbox` binding in `programs/desktop/src/lib.rs`
(AES opcode 73 is implemented in `tos-api/src/aes/graf.rs:356` but the desktop
has no wrapper for it).
