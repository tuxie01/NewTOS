# `deskapp.c` — icon/grid metrics (`app_start`, `app_blddesk` consumption)

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/deskapp.c`, `desk/deskapp.h`. This entry covers **only** the
> icon-geometry / grid-metric portion of `app_start()` and how
> `app_blddesk()` consumes it. The rest of `deskapp.c` (ANODE list
> management, EMUDESK.INF parse/build, icon loading `app_rdicon`, install
> dialogs) is narrated in `docs/api/desktop.md` and not re-derived here
> except where it feeds the metrics below.

## EmuTOS source

**Primary locations:**
- `desk/deskapp.c:781-1014` — `app_start()` (icon metrics at
  `:810-817`, per-`ANODE` pixel placement at `:979-1004`)
- `desk/deskapp.c:1306-1331` — `set_background()`
- `desk/deskapp.c:1337-1404` — `app_blddesk()` (consumes `g_wicon`/
  `g_hicon` at `:1362-1363`, recentres `ib_xicon` at `:1391`)
- `desk/deskapp.c:454-532` — `setup_iconblks()`, the helper (called from
  `app_rdicon()`, `:662-674`, either directly for the built-in set at
  `:673` or indirectly via `load_icon_file()`/`setup_mono_icons()` for a
  user-supplied resource) that allocates `G.g_iblist[]`/transforms the
  bitmaps and contains the "fix up the ICONBLKs" loop (`:509-521`), which
  sets `ib_wtext`/`ib_htext`/`ib_ytext` for **every** loaded icon block,
  but leaves `ib_xtext` untouched

**Helpers called:**
- `desk/deskobj.c:220` — `obj_ialloc()` — allocates + places one item
  `OBJECT` (used by `app_blddesk` for desktop icons, by `win_bldview` for
  window items)
- `desk/deskins.c:146-184` — `snap_icon()` — grid-rounding of a pixel
  position (used by `app_start()`'s INF-position-apply loop, `:983-985`,
  see below). **Resolved** (was flagged "needs deeper analysis" here) —
  full extraction, including the important finding that `snap_icon`
  itself performs **no** collision avoidance, is in
  [`deskins.md`](deskins.md), section "`snap_icon`/`ins_posdisk`/
  `grid_free` — desktop grid placement".
- `include/obdefs.h:258-272` — `r_set`/`r_get` GRECT helpers. **Correction
  (this validation pass, 2026-07-21):** an earlier draft of this entry cited
  these as living in `aes/gemwrect.c`, "not traced." That file exists but
  does not define or call `r_set`/`r_get` (grepped, zero hits). They are
  tiny `static __inline__` field-copy functions in the **shared** header
  `include/obdefs.h`, included by both `desk/*.c` and `aes/*.c` — calling
  them is not a dependency on AES internals at all (unlike `gsx_trans`, see
  `docs/api/desktop/gembind.md`); trivial to reimplement, nothing left to
  trace.
- **Real AES calls made from this file** (added in this validation pass —
  the earlier draft's helper list didn't include any AES calls, but
  `app_start()`/`app_rdicon()`/`app_blddesk()` do make some, all via
  `desk/gembind.c`'s trap binding, see `docs/api/desktop/gembind.md` for the
  full inventory and the client/server trap mechanism): `rsrc_load`/
  `rsrc_free` (`load_icon_file()`/`setup_mono_icons()`, `deskapp.c:554,575,
  593,600`), `shel_get`/`shel_put` (`deskapp.c:832,1123,1293,1532,1533`),
  `fsel_exinput` (`deskapp.c:1505`, outside this doc's grid-metric scope —
  install-application file selector), and (via `obj_ialloc`/`obj_wfree`,
  not directly) `objc_add` (`deskobj.c:145,232`). See the "`app_start()` —
  full AES/GEMDOS call sequence" and "`app_blddesk()` — full call sequence"
  sections below for the ordered call chains.

**Headers / type definitions used:**
- `include/obdefs.h` — `ICONBLK` (see `docs/api/structures/iconblk.md`)
- `desk/deskbind.h:47-56` — `SMALL_HSPC`/`LARGE_HSPC`/`MIN_WINT`/`MIN_HINT`
- `desk/deskapp.c:74` — `MAX_ICONTEXT_WIDTH` (12)

**System variables / globals read:**
- `G.g_desk` (`desk/deskbind.h:164`) — desktop work-area `GRECT`
- `G.g_iblist[0]` — the hard-disk icon's `ICONBLK` (`ib_xtext`, `ib_hicon`,
  `ib_wicon`)
- `gl_wschar` (`include/gsxdefs.h:85`) — width of the **small font**
  character cell (used for icon labels; distinct from `gl_wchar`, the
  normal/window-text font)

**System variables / globals written:**
- `G.g_wicon`/`G.g_hicon` (`desk/deskapp.c:810-811`)
- `G.g_icw`/`G.g_ich` (`desk/deskapp.c:814,817`)
- `G.g_xyicon[]`/`G.g_xytext[]` (`desk/deskapp.c:990-1011`)
- `pa->a_xspot`/`pa->a_yspot` for every `AF_ISDESK` `ANODE`
  (`desk/deskapp.c:983-985`)

**Build configuration affecting this function:**
- `CONF_WITH_BACKGROUNDS` (default 1) — `set_background()`'s `ob_spec`
  value; `INITIAL_ICON_STATE` in `deskwin.c` (see `deskwin.md`)
- `CONF_WITH_SIZE_TO_FIT` (default 1) — does **not** affect the metrics
  in this file directly (it affects `win_view`/`win_ocalc`, see
  `deskwin.md`)
- Neither is TARGET_192-disabled behaviour relevant to NewTOS (RPi4 build
  follows the full/general variant — see `_overview.md`)

---

## Source extraction summary

`app_start()` runs once at desktop boot (after the icon set is loaded by
`app_rdicon()`) and derives two related but **distinct** pairs of
numbers:

1. **`g_wicon`×`g_hicon`** — the size of **one icon cell** (icon bitmap +
   its label, as a single bounding box), derived from the hard-disk icon
   (`G.g_iblist[0]`) and the current small-font metrics. This value is
   used both for **desktop** icons (`app_blddesk`) and for **window**
   icon-view items (`win_view`, see `deskwin.md`) — there is only one
   icon-cell size system-wide, not one per icon type or per window.

2. **`g_icw`×`g_ich`** — the **desktop's** grid pitch (column/row pixel
   step), computed by fitting as many `g_wicon`+`MIN_WINT`-wide columns
   (resp. `g_hicon`+`MIN_HINT`-tall rows) as possible into `g_desk`, then
   **redistributing the desktop's exact pixel width/height evenly**
   across that many columns/rows — so the last column never has leftover
   slack, and spacing looks symmetric at every resolution. This is a
   *different* pitch computation than the one `win_view()` uses inside a
   window (see `deskwin.md`): windows do **not** evenly redistribute —
   they pack items left-to-top with a fixed `MIN_WINT`/`MIN_HINT` gap and
   let any remainder be unused/scrollable space at the window's right/
   bottom edge.

`app_blddesk()` then places each desktop `ANODE` at its (already
pixel-converted, see `_data-model.md`) `a_xspot`/`a_yspot`, sized
`g_wicon`×`g_hicon`, and **recentres the icon bitmap inside that cell
every time** using the *live* `g_wicon` value — but does not touch the
label's centering offset (`ib_xtext`), which was set once, resolution-
independently, when the icon was loaded.

---

## The exact grid-metric formulas

### 1. Icon-cell size (`g_wicon`/`g_hicon`) — `deskapp.c:810-811`

```
G.g_wicon = (MAX_ICONTEXT_WIDTH * gl_wschar) + (2 * G.g_iblist[0].ib_xtext);
G.g_hicon = G.g_iblist[0].ib_hicon + gl_hschar + 2;
```

- `MAX_ICONTEXT_WIDTH` = 12 (`deskapp.c:74`) — the maximum label length in
  characters (excluding the trailing NUL) that the desktop ever displays
  under an icon; longer names are truncated elsewhere (out of scope here).
- `gl_wschar`/`gl_hschar` — small-font character cell width/height
  (`gsxdefs.h:85-86`), i.e. **not** the same font metric as window title
  text (`gl_wchar`/`gl_hchar`).
- `G.g_iblist[0]` is **always** the hard-disk icon (`IG_HARD`=0,
  `deskapp.h:54`) — `app_start()` uses **only this one icon's** dimensions
  for the universal cell size, relying on the documented assumption
  (`deskapp.c:462-464` comment) that "all icons have the same width &
  height as the first." A user- or theme-supplied icon set with a
  differently-sized icon #0 would silently resize *every* icon's cell.
- **`ib_xtext` is not recomputed here** — it is whatever value the icon
  resource (`icon.rsc`, compiled at build time from `icon.def` by the `erd`
  tool; **resolved to `0`, see below**) carried for icon #0. `setup_iconblks()`'s "fix up" loop
  (`deskapp.c:509-521`, called from `app_rdicon()`, `:662-674`) explicitly
  sets `ib_ytext`/`ib_wtext`/`ib_htext` for every icon but **does not
  touch `ib_xtext`**.
- **Geometric reading:** if a label box of width `wtext` is meant to be
  horizontally centred within a cell of width `W`, then
  `xtext = (W - wtext) / 2`, i.e. `W = wtext + 2*xtext`. Substituting
  `wtext = MAX_ICONTEXT_WIDTH*gl_wschar` (this is exactly what
  `setup_iconblks()` sets `ib_wtext` to, `deskapp.c:519`) gives precisely the
  `g_wicon` formula above. **This only produces a correctly-centred label
  at the font size the resource's `ib_xtext` was authored for** — since
  `wtext` scales with the live `gl_wschar` but `xtext` is a fixed baked-in
  resource constant, a resolution/font change that alters `gl_wschar`
  shifts the label's centring, not just its width. This is an EmuTOS
  quirk inherent to the resource format, not something a rebuild needs to
  reproduce byte-for-bit (visual centring is not contract) — but it is
  the authoritative reasoning behind why the cell width is
  `wtext + 2*xtext` and not simply `wtext`.
- **RESOLVED (this validation pass, 2026-07-21) — was a STOP block, the
  premise was wrong.** The earlier draft claimed the `erd` resource
  decompiler was "not part of this reference tree." It **is**:
  `references/emutos/tools/erd.c` is checked in, and the `Makefile` shows
  exactly how to build/invoke it for the icon resource specifically
  (`erd: tools/erd.c` → `$(NATIVECC) $< -o $@`, `Makefile:963-964`; the icon
  variant is built with `-DICON_RSC` and invoked as `./ird -picon desk/icon
  desk/icons`, `Makefile:1017`). Reproduced read-only, outside the repo
  (`/tmp`, nothing written into `references/emutos/` or `docs/`):
  ```
  cc references/emutos/tools/erd.c -DICON_RSC -o /tmp/ird
  cp references/emutos/desk/icon.{def,rsc} /tmp/desk/
  cd /tmp && ./ird -picon desk/icon desk/icons   # → desk/icons.c, desk/icons.h
  ```
  `desk/icon.def`/`desk/icon.rsc` are confirmed unchanged since before the
  pinned commit (`git log --oneline -- desk/icon.def desk/icon.rsc` shows no
  commits after `1a7484ab`), so the generated `icons.c` reflects exactly
  what `app_rdicon()`/`setup_iconblks()` see at runtime. The generated
  `icon_rs_iconblk[]` array (field order per `ICONBLK`,
  `include/obdefs.h:185-201`: `ib_pmask, ib_pdata, ib_ptext, ib_char,
  ib_xchar, ib_ychar, ib_xicon, ib_yicon, ib_wicon, ib_hicon, ib_xtext,
  ib_ytext, ib_wtext, ib_htext`) entry `[0]` (`IG_HARD`=0, `deskapp.h:54` —
  confirmed the hard-disk icon by its `ib_char = 0x1000|'C'`, the classic
  first-hard-disk drive letter):
  ```c
  { (WORD*)rs_iconmask0, (WORD*)rs_icondata0, "", 0x1000|'C',
    /*xchar*/13, /*ychar*/16, /*xicon*/20, /*yicon*/0,
    /*wicon*/32, /*hicon*/32, /*xtext*/0, /*ytext*/32,
    /*wtext*/72, /*htext*/8 },
  ```
  **`G.g_iblist[0].ib_xtext = 0`.** So in practice `G.g_wicon =
  MAX_ICONTEXT_WIDTH*gl_wschar + 2*0 = 12*gl_wschar` exactly — there is
  **no** extra margin term for the shipped built-in icon set; the
  "resolution-dependent label centring" property discussed above is a real
  formula property but does not manifest with the actual built-in hard-disk
  icon, since its baked offset is zero. Per `_rebuild-plan.md` §5,
  `ib_xtext` is a free NewTOS design choice (not contract) — this resolved
  value is useful precedent (pick `0`, or document why not), not a
  requirement.

### 2. Desktop grid pitch (`g_icw`/`g_ich`) — `deskapp.c:813-817`

```
xcnt = G.g_desk.g_w / (G.g_wicon + MIN_WINT);   /* max columns that fit */
G.g_icw = G.g_desk.g_w / xcnt;                  /* pitch: redistribute width evenly */

ycnt = G.g_desk.g_h / (G.g_hicon + MIN_HINT);   /* max rows that fit */
G.g_ich = G.g_desk.g_h / ycnt;                  /* pitch: redistribute height evenly */
```

- `MIN_WINT`/`MIN_HINT` (`deskbind.h:53-56`):
  ```c
  #define SMALL_HSPC  4   /* horizontal, ST low/medium, Falcon double-line */
  #define LARGE_HSPC  8   /* horizontal, ST high, Falcon not-double-line */
  #define MIN_WINT    ((gl_height<=300) ? SMALL_HSPC : LARGE_HSPC)
  #define MIN_HINT    2   /* vertical, fixed */
  ```
  i.e. the *minimum* required horizontal gap is itself resolution-
  dependent (4px in low/medium-height modes, 8px in high-res modes),
  chosen because "because the desktop (between resolution changes) is a
  fixed size, the actual spacing between desktop icons is adjusted for
  symmetrical spacing" (`deskbind.h:47-52` comment).
- Both divisions are **integer** (`WORD` arithmetic) — `xcnt`/`ycnt` are
  floored column/row counts, and `g_icw`/`g_ich` is then the *exact*
  integer pixel pitch that makes `xcnt`/`ycnt` columns/rows tile
  `g_desk.g_w`/`g_h` **exactly** (any remainder from the floor division
  is absorbed into a *slightly wider* pitch for every column/row, not
  left as unused space at one edge) — this is the "adjusted for
  symmetrical spacing" behaviour. Concretely: `g_icw >= g_wicon+MIN_WINT`
  always (by construction, since `xcnt` was chosen as the floor), and the
  excess `g_icw - (g_wicon+MIN_WINT)` is spread across every column
  equally by virtue of being a per-column pitch rather than a fixed gap.
- **This is the desktop-only pitch.** A directory *window*'s icon-view
  pitch (`g_iwspc`/`g_ihspc`, see `deskwin.md`) is computed differently —
  `g_wicon+MIN_WINT` directly, **without** the redistribution step — so
  windows can have slightly tighter or looser spacing than the desktop at
  the same resolution, and (unlike the desktop) any leftover window width
  simply becomes unused/scrollable space.

### 3. Per-`ANODE` desktop placement (pixel conversion) — `deskapp.c:979-987`

```c
for (pa = G.g_ahead; pa; pa = pa->a_next) {
    if (pa->a_flags & AF_ISDESK) {
        x = pa->a_xspot * G.g_icw;
        y = pa->a_yspot * G.g_ich + G.g_desk.g_y;
        snap_icon(x, y, &pa->a_xspot, &pa->a_yspot, 0, 0);
    }
}
```

Converts the EMUDESK.INF-supplied **grid-unit** spot (`a_xspot`/`a_yspot`
as parsed — column/row indices) into pixel coordinates, offsetting `y` by
the desktop's top (`g_desk.g_y`, below the menu bar), then hands both to
`snap_icon()`, which writes the **final pixel** coordinates back into the
same `a_xspot`/`a_yspot` fields. **Correction to an earlier draft of this
entry:** `snap_icon()` does **not** perform collision avoidance — it only
rounds the pixel position to the nearest grid cell and clamps it to the
desktop bounds (full extraction: [`deskins.md`](deskins.md)). This loop
therefore applies **no** collision avoidance at all between the
`AF_ISDESK` `ANODE`s it places: two EMUDESK.INF entries with the same (or
rounding-adjacent) saved grid position produce two icons drawn on top of
each other, silently. See `_data-model.md` for the full unit-lifecycle
note (grid units in the INF file, pixels in memory afterwards).

### 4. `app_blddesk()` — full call sequence (`deskapp.c:1337-1404`)

**Expanded in this validation pass** — the earlier draft only extracted the
grid-metric-relevant lines (`obj_ialloc`/`ib_xicon`) from this function; for
a call-for-call 1:1 rebuild, the *entire* function's call sequence matters,
not just the metric-consuming half. In order:

1. `obj_wfree(DROOT, 0, 0, gl_width, gl_height)` (`deskapp.c:1346`) — frees
   every existing desktop item object back to the `g_screenfree` chain
   (`deskobj.c:187-211`) and resets `DROOT`'s own box to the full screen.
   **No AES call** (desk-internal free-chain op on `G.g_screen[]`, which the
   desktop already owns).
2. `set_background()` (`deskapp.c:1349`, see below) — sets `ob_spec` on
   `DROOT` and every window slot. **No AES call.**
3. `app_revit()` (`deskapp.c:1356`, defined `:1019-1033`) — reverses the
   `G.g_ahead` singly-linked `ANODE` list in place. Comment
   (`deskapp.c:1352-1355`): *"reverse the order of the ANODEs, so that the
   disk icons will be allocated in the same sequence as they appear on the
   desktop. this is important for 'Show info'."* **Not previously documented
   in this file or in `docs/api/desktop.md`.** Since placement itself uses
   `a_xspot`/`a_yspot` (pixels, not list order), a rebuild that walks its
   ANODE-equivalent list in a different order will still draw icons at the
   *same* pixel spots, but will assign `g_screen[]`/`a_obid` indices in a
   *different* sequence — which this comment flags as user-visible via
   'Show info' (not re-traced this pass; `deskinf.c` territory).
4. For each `ANODE` with `AF_ISDESK` set, walked in the **reversed** (i.e.
   original on-disk) order from step 3 — **the loop body below was
   previously only shown as two lines; the full body, not previously
   documented, is:**
   ```c
   obid = obj_ialloc(DROOT, pa->a_xspot, pa->a_yspot,
                               G.g_wicon, G.g_hicon);
   ...
   pa->a_obid = obid;

   /* choose appropriate icon (deskapp.c:1374-1378) */
   if ((pa->a_dicon < 0)
    || ((pa->a_type == AT_ISFILE) && (pa->a_flags & AF_ISEXEC)))
       icon = pa->a_aicon;
   else
       icon = pa->a_dicon;

   pob->ob_type = G_ICON;
   memcpy(pic, &G.g_iblist[icon], sizeof(ICONBLK));   /* deskapp.c:1390 */
   pic->ib_xicon = ((G.g_wicon - pic->ib_wicon) / 2);  /* deskapp.c:1391 */

   /* label pointer varies by ANODE type (deskapp.c:1393-1396) */
   if ((pa->a_type == AT_ISFILE) || (pa->a_type == AT_ISFOLD))
       pic->ib_ptext = pa->a_pdata;
   else
       pic->ib_ptext = pa->a_pappl;

   pic->ib_char |= pa->a_letter;                       /* deskapp.c:1397 */
   ```
   - `obj_ialloc()` (`deskobj.c:220-240`) allocates the item `OBJECT` from
     the free chain and internally calls **`objc_add(G.g_screen, wparent,
     objnum)`** (`deskobj.c:232`) — **the one real AES call `app_blddesk()`
     triggers per placed desktop icon** (indirect, via `obj_ialloc`, not a
     direct `app_blddesk()` call).
   - Icon selection (`:1374-1378`, **not previously documented**): use the
     "application" icon (`a_aicon`) when there is no data icon (`a_dicon <
     0`) or the ANODE is an executable file; otherwise use the "data" icon
     (`a_dicon`).
   - The `OBJECT` box is sized exactly `g_wicon`×`g_hicon` (the object box
     **equals the icon cell**, not just the icon bitmap); `ib_xicon` is
     **recentred live** against `g_wicon` (`:1391`, already correctly
     extracted in the earlier draft) — but `ib_xtext` (copied verbatim by
     the `memcpy`) is **not** recentred here.
   - Label pointer (`:1393-1396`, **not previously documented**):
     `ib_ptext = pa->a_pdata` for `AT_ISFILE`/`AT_ISFOLD` ANODEs (matches the
     "usage of `a_pappl`/`a_pdata`" comment, `deskapp.h:86-98`: file/folder
     desktop items keep the label in `a_pdata`), else `ib_ptext =
     pa->a_pappl` (disks/trash/printer keep their label in `a_pappl`).
   - `pic->ib_char |= pa->a_letter` (`:1397`, **not previously documented**)
     — ORs the drive letter (0 for non-drive icons) on top of the high byte
     `setup_iconblks()` already stripped clean (`deskapp.c:517`).
5. `app_revit()` again (`deskapp.c:1401`) — restores `G.g_ahead` to its
   original order. **Not previously documented.**
6. `do_wredraw(DESKWH, &G.g_desk)` (`deskapp.c:1403`) — the repaint; full
   mechanism in `desksupp-redraw.md` (which already cites this exact line).
   `app_blddesk()`'s **second** AES-call dependency, indirect via
   `do_wredraw`'s own `wind_get`/`objc_draw` calls
   (`docs/api/desktop/gembind.md`).

**Net AES call footprint of `app_blddesk()` itself: zero direct calls** —
exactly `objc_add` (via `obj_ialloc`, once per placed desktop icon) and
whatever `do_wredraw` makes, both indirect. Steps 1-2 and the loop's
in-object-tree bookkeeping (icon selection, label pointer, `ib_char`) are
pure `G.g_screen[]`/`ANODE` manipulation the desktop performs on data it
already owns.

**Edge case discovered in source — icon-bitmap vs. label recentring
asymmetry:** `app_blddesk()` (desktop icons) recomputes `ib_xicon` live
against `g_wicon` (so the bitmap is always correctly centred in the cell,
regardless of resolution/font), but `win_bldview()` (window icons, see
`deskwin.md`) does **not** perform this recentre — window icons rely on
whatever `ib_xicon` came from `G.g_iblist[i_index]` unchanged. Both paths
leave `ib_xtext` untouched in all cases. Net effect: the **icon bitmap**
is guaranteed centred on the desktop but only "as-authored" in windows;
the **label** is only "as-authored" (both places) — a discrepancy that
only reading both call sites side-by-side reveals.

**Cross-doc observation (out of this task's editable scope):**
`docs/api/desktop.md`'s own `app_blddesk()` narrative ("Desktop object tree
+ drive icons") independently covers steps 1-2 and most of the loop body
correctly, but likewise omits the double `app_revit()` (steps 3/5) and
states the icon-selection/label-selection logic only loosely ("`ob_spec` to
a copy of the disk `ICONBLK`... `ib_ptext` = the drive's name string" — true
only for the `AT_ISDISK` case, not the general `AF_ISDESK` case which also
covers files/folders/shortcuts under `CONF_WITH_DESKTOP_SHORTCUTS`). Noted
for the lead since `docs/api/desktop.md` is not part of this task's
editable file set.

---

## `app_start()` — full AES/GEMDOS call sequence (`deskapp.c:781-1014`)

**New in this validation pass.** The grid-metric formulas above are two
lines (`:810-817`) inside a 234-line function; the earlier draft's "EmuTOS
source"/"Helpers called" section only listed `obj_ialloc`, `snap_icon`, and
`r_set`/`r_get` as dependencies. For a 1:1 rebuild's call-surface inventory,
the ordered list of *every* AES/GEMDOS call `app_start()` makes, directly or
via its immediate helpers, is:

1. `initialise_anodes()` (`deskapp.c:794`, defined `:611-645`) —
   `dos_alloc_anyram()` ×2 (GEMDOS `Malloc`/`Mxalloc` via `trap1`,
   `bdosbind.h:45,47`). No AES.
2. `app_rdicon()` (`deskapp.c:801`, defined `:662-674`) — tries the
   user-supplied icon resource first, falls back to the built-in set:
   - `load_icon_file()` (`:537-559`): `dos_sfirst()` (GEMDOS `Fsfirst`,
     checks the resource file exists *before* asking the AES to load it —
     avoiding a slow multi-directory `rsrc_load` search or a spurious form
     alert, per the comment at `:545-549`), then **`rsrc_load(icon_rsc_name)`**
     (`:554`) — a real AES Resource Manager call.
   - `setup_mono_icons()` (`:565-603`), only if `rsrc_load` succeeded: reads
     the loaded resource header via `AP_1RESV` (an AES global the desktop
     reads directly, `:571` — not a trap call, a shared-memory read of a
     value the AES already wrote), then **`rsrc_free()`** (`:575`/`:593`/
     `:600`, always reached before returning — `setup_iconblks()` has
     already `memcpy`'d everything it needs out of the resource, so it's
     safe to free).
   - Fallback: `setup_iconblks(icon_rs_iconblk, BUILTIN_IBLKS)` (`:673`) —
     `dos_alloc_anyram()` only, no AES.
3. `must_alloc_buf(SIZE_SHELBUF)` (`deskapp.c:808`) — GEMDOS allocation.
4. Grid-metric formulas (`:810-817`, already documented above). No AES/GEMDOS.
5. **`shel_get(buf, SIZE_SHELBUF)`** (`deskapp.c:832`) — a real AES Shell
   Manager call, **not previously documented in this spec**. The AES-side
   handler, `sh_get()` (`aes/gemshlib.c:203-206`), `memcpy`s from
   `D.g_shelbuf`, a small AES-internal buffer explicitly documented as
   *"used by the DESKTOP to recall ... previously 'put' desktop-context
   information"* (`aes/gemshlib.c:200-201`) — a fast in-memory staging area
   the desktop uses to carry its EMUDESK.INF text across a restart (e.g.
   after a resolution change, see the correction below) without necessarily
   re-reading the disk file.
6. `read_inf_file()` (`deskapp.c:836`, defined `:680-692`), conditionally,
   if `shel_get`'s buffer didn't contain valid `'#'`-signed data:
   `dos_load_file()` → wraps `Fopen`/`Fread`/`Fclose` (GEMDOS; the wrapper
   itself lives in `bios/`, not traced further — out of scope).
7. `build_inf()` (`:840`), if still no data — pure in-memory text
   generation, no AES/GEMDOS.
8. The `'#'`-line parser loop (`:846-977`) — pure in-memory (`app_alloc`/
   `app_parse`/`scan_2`/`scan_str`). No AES/GEMDOS.
9. The per-`ANODE` pixel-conversion + `snap_icon()` loop (`:979-987`,
   already documented above) — pure in-memory (per `deskins.md`).
10. Drag-outline setup (`:989-1011`) — pure in-memory.
11. `dos_free(buf)` (`:1013`) — GEMDOS `Mfree`.

**Net AES call footprint of `app_start()`:** `rsrc_load`/`rsrc_free` (only on
the user-icon-resource path, step 2) and `shel_get` (step 5, unconditional).
**Net GEMDOS call footprint:** `Fsfirst` (step 2), `dos_alloc_anyram`/
`dos_free` (real `Malloc`/`Mfree` via `trap1`, steps 1/3/11), and
`dos_load_file`'s `Fopen`/`Fread`/`Fclose` (step 6, conditional). Full
catalogue and cross-reference to `tos-api`: `docs/api/desktop/gembind.md`.

### Correction — resolution change does *not* call `app_start()` from `deskrez.c`

The earlier draft's "Dependencies / Used by" section (below) speculated:
*"`deskrez.c` (out of scope, resolution change) — presumably re-triggers
`app_start()`'s metric recompute on a resolution switch; not traced in this
task."* **This validation pass read `deskrez.c` in full and greps for
`app_start`/`win_view`/`obj_init`/`g_wicon`/`g_icw` all return zero hits** —
`deskrez.c` never calls any of them. The actual mechanism:
`deskmain.c:900-913` (`RESITEM` case in the menu handler) calls
`change_resolution()` (`deskrez.c:316-333`, only presents the dialog and
computes the *requested* mode) and, on a confirmed change, calls
**`shel_write(SHW_RESCHNG, newmode_or_newres, ...)`** (`deskmain.c:910,913`)
— a real AES Shell Manager call whose AES-side handler
(`aes/gemshlib.c:187-192`) just records `gl_changerez`/`gl_nextrez` and
clears the command tail — then sets `done = TRUE`, **exiting the desktop's
own main loop**. The grid-metric recompute happens because the resolution
change causes the **entire desktop to restart** from scratch once the video
mode actually changes (outside `desk/`, in whatever boot/shell sequencer
relaunches the desktop program — not traced this pass, out of scope), not
via any in-place "resolution changed, patch `g_icw`/`g_ich`" code path. A 1:1
rebuild must reproduce "resolution change ⇒ full desktop reboot," not an
incremental metric patch grafted onto a live desktop.

---

## `set_background()` (`deskapp.c:1306-1331`)

Not a grid-metric function, but the other half of what `app_blddesk()`
does before placing icons — included here for completeness since
`docs/api/desktop.md` already covers its narrative (default `g_patcol`
values, `#Q` INF line). Reproduced structurally: chooses a plane-count
bucket `n` (0 = 1-plane, 1 = 2-plane, 2 = other), sets every window
object's `ob_spec = G.g_patcol[n].window | BORDER_TEXT_COLOURS` and
`G.g_screen[DROOT].ob_spec = G.g_patcol[n].desktop | BORDER_TEXT_COLOURS`
(`CONF_WITH_BACKGROUNDS`); otherwise (`#else`) `G.g_screen[DROOT].ob_spec
= AP_PRIVATE` (a fixed AES-private fill, no user pattern/colour choice).
`BORDER_TEXT_COLOURS` = `(BLACK<<12)|(BLACK<<8)` (`deskapp.c:80`) — border
and text colour nibbles fixed to black regardless of the pattern/colour
choice.

---

## Edge cases discovered in source

- `g_wicon`/`g_hicon` derive from icon **index 0 only** — see formula
  discussion above. A theme/user icon set is implicitly required to keep
  icon #0's dimensions consistent with the rest, or all cells resize.
- `ib_xtext`'s resolution-dependence (label centring only correct at the
  font size the resource was authored for) — see above; visual-only, not
  contract.
- The icon-bitmap-vs-label recentring asymmetry between `app_blddesk`
  (recentres bitmap) and `win_bldview` (doesn't) — see above.
- `g_icw`/`g_ich` symmetric-redistribution vs. `g_iwspc`/`g_ihspc`'s
  plain fixed-gap pitch (desktop vs. window) — two different pitch
  algorithms for what looks like "the same" grid; conflating them would
  make desktop and window icon spacing subtly different from EmuTOS.
- `a_xspot`/`a_yspot` unit change (grid units → pixels) happens exactly
  once, in `app_start()`'s loop, via in-place field overwrite — see
  `_data-model.md`.

---

## Dependencies

### Requires (must be initialized / functional)

- `app_rdicon()` — must have populated `G.g_iblist[]` (specifically index
  0) before `app_start()` computes `g_wicon`/`g_hicon`
- `G.g_desk` — must already reflect the current resolution's desktop
  work-area rectangle
- `gl_wschar`/`gl_hschar` — AES font-metric globals, set during AES/VDI
  startup (`aes/gemgraf.c`, out of scope here)

### Used by

- `app_blddesk()` — desktop icon sizing/placement, `ib_xicon` recentre
- `win_view()` (`deskwin.c:83-86`, see `deskwin.md`) — `V_ICON` case sets
  `G.g_iwext = G.g_wicon`, `G.g_ihext = G.g_hicon`, so the same universal
  cell size governs window icon-view spacing too
- **Not** `deskrez.c` directly — see the correction above ("resolution
  change does *not* call `app_start()` from `deskrez.c`"). The metric
  recompute happens as a side effect of the desktop's full restart after a
  resolution change (`deskmain.c:900-913`'s `RESITEM`
  handler → `shel_write(SHW_RESCHNG,...)` → `done=TRUE`), not a call from
  `deskrez.c` itself.

## Related data structures

- `docs/api/desktop/_data-model.md` — `GLOBES` field table (`g_wicon`,
  `g_hicon`, `g_icw`, `g_ich`, `g_iblist`, `ANODE.a_xspot`/`a_yspot`)
- `docs/api/structures/iconblk.md` — `ICONBLK` field layout (`ib_xicon`,
  `ib_xtext`, `ib_wtext`, …)

## NewTOS implementation notes

(Empty during Phase 00 / this extraction task, except for one staleness
correction found while cross-checking: this note previously referenced
kernel/src/desktop.rs's `ICON_LABEL_W`/`ICON_GAP_W`/`ICON_STEP_W`
constants, commit `af426a0`, as "NewTOS's current" grid-metric code. As of
this validation pass, kernel/src/desktop.rs no longer exists — it has been
superseded by `programs/desktop/src/` (`app.rs`, per `_rebuild-plan.md`'s module
table), a substantially-progressed rebuild already wired into
`kernel/src/lib.rs`. Whether `programs/desktop/src/app.rs` reproduces the `g_wicon`/`g_hicon`/
`g_icw`/`g_ich` formulas above exactly, and whether it adopts the resolved
`ib_xtext=0` precedent or the desktop-vs-window pitch distinction, was **not
checked** this pass — out of scope for a docs-only validation task; flagged
for the lead to confirm against the current `programs/desktop/src/app.rs` before
treating this spec as implemented.

---

## NewTOS implementation notes — `app_read_inf()` (DESK14, 2026-09-23)

`Options → Read .INF file` is EmuTOS's `app_read_inf()`
(`deskapp.c:1491-1546`, `CONF_WITH_READ_INF`). NewTOS's counterpart is
`programs/desktop/src/inf.rs::app_read_inf`, called from
`programs/desktop/src/main.rs::do_read_inf` (the `READITEM` arm of the Options
menu). Before DESK14 that path read a fixed `C:\NEWDESK.INF`, accepted any
buffer whose first byte was `#`, and returned silently on every failure.

### What now matches EmuTOS

1. **The file selector** (`:1499-1508`). The prompt is
   `fsel_exinput(path, fname, &button, desktop_str_addr(STRDINF))` with
   `path` = `INF_FILE_WILD` and `fname` = `INF_FILE_NAME+3`, and the
   chosen name is merged back with `strcpy(filename_start(path), fname)`
   (`:1509`). NewTOS: `INF_FILE_WILD` = `"C:\\*.INF"`, `STRDINF` =
   `"Select .INF file"` (`desk_rsc.c:2488`), default selection
   `"NEWDESK.INF"`. `rc == 0 || button == 0` cancels, as in EmuTOS.
2. **The signature** (`:1523-1525`). `is_inf_signature` requires `#R`
   **or** `#E`, not the bare `#`. This is deliberately *stricter* than
   `app_start`'s boot-time test (`deskapp.c:833`, `inf_data[0] != '#'`),
   which NewTOS keeps as it stands in `inf::load_and_apply`: at boot the
   buffer is whatever `shel_get` handed over, while Read .INF validates a
   file the user just picked.
3. **The two alerts** (`:1536-1542`). A failed open with `EFILNF` →
   `STFILENF` = `[1][%s not found.][  OK  ]` (`desk_rsc.c:2548`) with the
   chosen name merged in (EmuTOS `fun_alert_merge`, `deskfun.c:88`); every
   other failure, an empty file and a wrong signature → `STINVINF` =
   `[1][Illegal .INF file format.][  OK  ]` (`desk_rsc.c:2560`). Both go
   through `dir::fun_alert`, i.e. the AES `form_alert`.

The AES side was already complete (`tos-api/src/aes/fsel.rs`); what DESK14
added is the desktop-side binding `desk/mod.rs::fsel_exinput` (opcode 91,
`AES_CTRL_CODE(FSEL_EXINPUT, 0, 2, 3)`, EmuTOS `desk/gembind.c:702-712`).

One read-size difference is worth naming: EmuTOS loads at most
`SIZE_SHELBUF-CPDATA_LEN-1` bytes (`sysconf.h:74`, 4192 minus the copy-data
header) because it stages the file into the *shell buffer*. With the restart
path deferred there is no such buffer here, so the read is the desktop's own
`MAX_INF_BYTES` (64 KiB). The truncation is a property of EmuTOS's mechanism,
not of the contract; a `.INF` that long would leave EmuTOS with a half-parsed
file and NewTOS with a whole one.

### The one named deviation: no desktop restart

EmuTOS answers a successful read by merging the bytes into the shell buffer
(`shel_get`/`shel_put`, `:1532-1533`) and **leaving** the desktop: `READITEM`
sets `restart_desktop = TRUE; done = TRUE;` (`deskmain.c:919-925`),
`deskmain` frees its allocations (`:2136-2148`) and `_deskstart` re-enters it,
so the new file is read exactly the way the boot path reads one.

NewTOS parses and applies the bytes **in the running desktop** —
`inf::apply_inf_bytes` (the same tail `load_and_apply` runs at boot) followed
by `main::do_read_inf`'s `cnx_get_prefs` + `install_shortcuts` + `win_view` +
`app_blddesk` + `win_srtall` + `redraw_desktop`. The restart path is not a
detail of this function but the move of the desktop from a kernel call to a
process `sh_main` can re-enter (`global::init` is idempotent, there is no
`win_start()` on re-entry, `wm_init()` would clear the AES window table, and
the `cnx_put(); app_save(FALSE)` → `shel_get` bridge, `deskmain.c:2152-2153`,
does not exist). Deferred with the move of the desktop into user scope —
lead decision of 2026-09-23, recorded in `TASK/DESK14.md`, and carried in the
source as a comment on `do_read_inf`. Everything *before* that point — prompt,
load, validation, alerts, return value — is EmuTOS's.

---

## NewTOS implementation notes — the built-in icon set and `app_parse`'s icon clamp (DESK16, 2026-09-23)

`app_blddesk`'s "choose appropriate icon" (`deskapp.c:1371-1375`) indexes
`G.g_iblist[icon]` unconditionally. It can, because `app_parse` has already
made the number usable: `a_aicon >= G.g_numiblks` becomes `IG_APPL`,
`a_dicon >= G.g_numiblks` becomes `IG_DOCU` (`:359-365`), and `scan_2` has
turned the file's `FF` into `-1` before that (`util/optimize.c:202-217`), which
is *below* the bound and therefore left alone — `-1` means "this node has no
icon of that kind", not an out-of-range index.

NewTOS's `desk::inf::parse_anode` reproduces that clamp (`clamp_icon`), and
`desk::app::app_blddesk` therefore resolves through `icons::icon_kind` with no
type-shaped fallback. `app::default_icon_kind`, which stood in for the clamp
while `IG_REMOV` had no raster, is gone.

**The raster.** `desk/icon.rsc` carries eight enabled `ICONBLK`s (six more are
`#if 0`). NewTOS had seven; `IG_REMOV` (5) was missing, which is what made a
fallback necessary in the first place. It was taken the same way the other
seven were: out of `references/emutos/desk/icons.c`, the C the EmuTOS build
generates from `icon.rsc` with `ird`. All seven pre-existing rasters were
re-verified byte-for-byte against that file at the same time.

One property of `icon_rs_iconblk[5]` is worth naming because it is easy to get
wrong: the removable disk has **no mask of its own**. The resource points it at
`rs_iconmask0`, the hard disk's, so `icons::rasters` returns `HARD_MASK` for it
— and its `ib_xchar`/`ib_ychar` are the hard disk's as well, `(13,16)`, so a
removable drive carries its drive letter in the same place.

**One named deviation**, on a value EmuTOS reads out of bounds: an `a_aicon` of
`-1` on a desktop line. Only a hand-written `NEWDESK.INF` can produce it —
`build_inf` always writes the number (`deskapp.c:723`) — and EmuTOS would index
`G.g_iblist[-1]`. NewTOS takes `app_parse`'s own answer for an unusable
`a_aicon`, `IG_APPL`.

Measured: `measurements/measurement-2026-09-23-desk16-desk17-desk21/`. The dialog's own
serial line goes from `curr_icon=0 of 6` to `of 7` (the floppy is filtered out
on a machine with no floppy drive — a separate, still-live NewTOS rule
documented in `deskins.md`), four clicks on the stepper's DOWN arrow land on the
new picture, `Save desktop` writes `#M 00 00 05 FF C DISK C@ @`, and a cold
start reads it back. `measurements/measurement-2026-09-23-desk16-desk17-desk21/method/pixcheck.py` compares
 every mask-set pixel of the
preview and of the desktop icon against `icons.c`: 654 and 618 pixels, zero
mismatching.

---

## NewTOS implementation notes — `app_start`'s drag outlines and the shortcut half of `app_blddesk` / `app_parse` / `app_save` (DESK30 + DESK36, 2026-09-24)

Two previously unimplemented pieces of `deskapp.c` landed together.

**`app_start:988-1011` — `G.g_xyicon` / `G.g_xytext`.** The two point lists
`gr_drgplns` translates once per dragged object are now built in
`DeskState::app_drag_outlines` (`programs/desktop/src/app.rs`) and carried in
`DeskState`. The icon outline is the 9-vertex silhouette of one icon cell —
a column `ib_wicon` wide over a full-cell-wide label box, stepping down at
`g_hicon - gl_hschar - 2`; the text outline is a `DRAG_BOX_WIDTH`
(`= 2 + LEN_ZNODE + 1 + LEN_ZEXT` = 14) × one-row rectangle. See
`deskact.md`'s DESK36 note for the loop that uses them.

**The `AF_ISDESK` file/folder node.** `app_parse`'s `#X`/`#V` arms
(`:333-343`), the pre-revision-2 `a_pappl`/`a_pdata` swap (`:404-415`), the
`AF_ISEXEC`-from-extension mark (`:422-426`), `app_save`'s `'X'`/`'V'` type
switch (`:1224-1243`) and `app_blddesk`'s two shortcut-dependent choices — the
icon (`:1374-1378`) and the label (`:1392-1396`) — are all implemented.
`programs/desktop/src/inf.rs` holds the `.INF` halves, `app.rs` the `app_blddesk`
half. `deskins.md`'s DESK30 note carries the rest of the feature and the
measurement.
