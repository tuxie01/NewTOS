# EmuTOS `desk/` — Rendering Core Overview

> Phase-00 extraction, Track A, per `docs/SOURCE-ANALYSIS-PROTOCOL.md`.
> Source: EmuTOS @ `1a7484ab` (pinned, see `docs/analysis/ORACLE.md` §1 "The pin"),
> directory `desk/`. This directory (`docs/api/desktop/`) extracts the
> **rendering core**: the data model, the icon/text grid metrics, view
> building (turning a file list into an on-screen `OBJECT` tree), and the
> redraw dispatch. It complements — does not replace — the existing
> `docs/api/desktop.md`, which covers the desktop **application** structure
> (main loop, menu, `app_blddesk`'s drive-icon/background narrative). Where
> the two overlap, this directory goes one level deeper into the geometry
> and cites the exact formulas; `desktop.md` is left as-is and cross-linked.

## Files in this directory

| File | Covers |
|---|---|
| `_overview.md` | this map |
| `_data-model.md` | `GLOBES` (`G`), `WNODE`, `PNODE`, `FNODE`, `ANODE` (rendering-relevant fields), `SCREENINFO`/`ICONINFO`, the `g_screen[]`/`g_screeninfo[]` layout |
| `deskapp.md` | `app_start()`'s icon/grid metrics (`g_wicon`/`g_hicon`/`g_icw`/`g_ich`), `app_blddesk()`'s use of them |
| `deskwin.md` | `win_view()`, `win_start()`, `win_ocalc()`, `win_bldview()`, plus the rest of `deskwin.c` (scrolling, window list, sort/rebuild-all) |
| `desksupp-redraw.md` | `do_wredraw()` and the `WF_FIRSTXYWH`/`WF_NEXTXYWH` rectangle-iteration contract it consumes |
| `deskmain.md` | *(sibling extraction, not produced by this task)* `deskmain()`'s event loop, menu dispatch and keyboard handling — the caller-side context for the `WM_REDRAW`→`do_wredraw` wiring documented in `desksupp-redraw.md` |
| `gembind.md` | **new, this validation pass (2026-07-04 re-validation, 2026-07-21):** the desktop's full AES/GEMDOS call-surface inventory — every live `gembind.c`/`aesbind.h` function, whether `desk/*.c` actually calls it (and where), its `tos-api` counterpart, the two real trap mechanisms (AES `trap #2`/GEMDOS `trap #1`), and the one found non-trap dependency (`gsx_trans`) |

## Map of `desk/` (what each source file does)

The full directory has 22 `.c` files; this task's scope is the five
covering data model + rendering core. The rest are listed for
orientation (already covered, or out of scope, per the task boundary):

| File | Role | Status |
|---|---|---|
| `deskglob.c/.h` | Declares/defines the single global instance `GLOBES G` | extracted → `_data-model.md` |
| `deskobj.c/.h` | `G.g_screen[]`/`g_screeninfo[]` allocation (`sob_malloc`), init (`obj_init`); **two distinct allocators** (precision fix, this validation pass — the earlier wording lumped them together as one "free-chain allocator"): `obj_walloc`/`obj_wfree` manage the fixed 7 **window** object slots (`obj_walloc` linear-scans for a zero-width/height slot, not a chain; `obj_wfree` returns a window's *children* to the item free chain but the window slot itself is never chained), while `obj_ialloc` is the actual **item**-object free-chain allocator (`G.g_screenfree`/`ob_next`); plus `obj_get_obid` | extracted → `_data-model.md` (structure) + `deskapp.md` (`obj_ialloc` call site)/`deskwin.md` (`obj_walloc`/`obj_wfree` call sites) |
| `deskfpd.c/.h` | `FNODE`/`PNODE` — open/close a directory path (`pn_open`/`pn_close`), scan it into a sorted `FNODE` list (`pn_active`/`pn_sort`), selection queries (`pn_selected`/`pn_count`/`pn_clear`) | extracted → `_data-model.md` |
| `deskapp.c/.h` | `ANODE` list; icon/grid metrics (`app_start`); desktop `OBJECT` tree build (`app_blddesk`); background pattern (`set_background`); EMUDESK.INF parse/build | **this task**: metrics + `app_blddesk`'s consumption of them → `deskapp.md`. INF parsing / ANODE installation already narrated in `docs/api/desktop.md`; not re-derived here except where it feeds grid placement (`a_xspot`/`a_yspot`) |
| `deskwin.c/.h` | `WNODE`; per-view sizing (`win_view`), window list management (`win_start`/`win_alloc`/`win_free`/`win_find`/`win_top`/`win_ontop`), the virtual-grid calculator (`win_ocalc`) and view builder (`win_bldview`), scrolling (`win_slide`/`win_arrow`/`win_blt`/`win_delta`), bulk operations (`win_srtall`/`win_bdall`/`win_shwall`), selection walk (`win_isel`), name/info line (`win_sname`/`win_sinfo`) | **this task** → `deskwin.md` |
| `desksupp.c/.h` | Grab-bag of desktop support routines: selection clearing (`desk_clear*`), `desk_verify`, **`do_wredraw`** (redraw dispatch), `do_xyfix`, `do_wopen`, `do_wfull`, file-exists check, disk format (`CONF_WITH_FORMAT`), show-file/print-file (`CONF_WITH_SHOW_FILE`/printer) | **this task**: `do_wredraw` only → `desksupp-redraw.md`. `do_wopen`/`do_diropen` → `desksupp-wopen.md` (DESK50). The rest (format, show-file, `do_wfull` widget handling) is out of scope for the *rendering core* task — flagged for a later batch if needed |
| `deskact.c/.h` | Object-selection/highlight state machine (`act_chg`, `act_allchg`, mouse-driven select/drag) | out of scope (interaction, not rendering) — calls `do_wredraw` (3 call sites), noted as a dependency in `desksupp-redraw.md` |
| `deskfun.c/.h` | Menu/file-operation dispatch (copy/move/delete/rename, open, new folder, …) | out of scope — one `do_wredraw` call site noted |
| `deskdir.c/.h` | Directory operation helpers (recursive count/copy/move/delete) used by `deskfun.c` | out of scope |
| `deskins.c/.h` | Install application / install icon dialogs | out of scope; referenced only for `a_aicon`/`a_dicon` semantics already covered by `deskapp.h` |
| `deskinf.c/.h` | "Show info" dialogs, text-view line formatting (`format_fnode`, `dr_code` `USERBLK` callback), disk info | out of scope — `dr_code`/`format_fnode` are the V_TEXT rendering callback referenced (not traced) from `win_bldview`; flagged **needs deeper analysis** if a later task needs the text-view line format |
| `deskmain.c/.h` | Main loop, menu dispatch, AES message switch (`hndl_button`, the `WM_REDRAW`/`MN_SELECTED`/… switch) | extracted in the sibling `deskmain.md` (not produced by this task); also narrated in `docs/api/desktop.md`. The `WM_REDRAW` case (the primary caller of `do_wredraw`) is additionally cited in this task's `desksupp-redraw.md` |
| `deskpro.c/.h` | Desktop "process" launch helper (`pro_run`/`pro_exec` → `shel_write(SHW_EXEC,…)`) | extracted 2026-07-21 → [`deskpro.md`](deskpro.md) (Researcher task, deskdir/deskfun/deskact re-validation). **Still missing**: `do_aopen`/`do_open` (`desksupp.c`), the actual caller of `pro_run` — see `deskpro.md`/`deskfun.md`'s "Missing coverage" notes. **Partially closed 2026-09-23 (DESK18)**: `do_aopen`'s current-directory step (`desksupp.c:893-916`) + `set_default_path` (`:1785-1801`) are extracted and implemented, `deskpro.md` § *The current-directory step* |
| `deskrez.c/.h` | Resolution-change handling | out of scope. **Correction (this validation pass):** `deskrez.c` does **not** call `app_start()`/recompute `g_wicon`/`g_icw` directly (grepped: zero hits) — it only presents the dialog and computes the requested mode; the metric recompute happens because a confirmed change triggers a full desktop restart (`deskmain.c:900-913` → `shel_write(SHW_RESCHNG,…)` → `done=TRUE`) — see `deskapp.md`'s corrected dependency note |
| `deskrsrc.c/.h` | Resource-string lookup (`desktop_str_addr`) | out of scope (utility) |
| `deskstart.S` | Desktop process entry stub | out of scope |
| `gembind.c/.h`, `aesbind.h` | AES binding stubs (trap glue for the desktop's own AES calls) | **corrected, this validation pass:** was marked "out of scope — already covered by `docs/api/aes/`," which is inaccurate — `docs/api/aes/` documents the AES's own per-function contract, not *which* of those functions `desk/` actually calls, how often, or in what order. Now extracted → [`gembind.md`](gembind.md), the full AES/GEMDOS call-surface inventory |
| `desktop.def`/`.rsc`, `icon.def`/`.rsc` | Resource source (menu tree text, icon bitmaps) — compiled at build time by the `erd` resource decompiler | **corrected, this validation pass:** the earlier note said the generated C ("`icons.c`") and the `erd` tool itself were "not present in this checkout" — `erd`'s **source** (`tools/erd.c`) *is* checked in, just not built; building it read-only (`cc tools/erd.c -DICON_RSC -o ird`, per the `Makefile`'s own recipe) and running it against the checked-out `icon.def`/`icon.rsc` resolves the `ib_xtext` STOP block in `deskapp.md` (`ib_xtext=0` for the hard-disk icon) |

> **Update (Task #3, 2026-07-04):** the six modules marked *out of scope* above
> (`deskdir`/`deskfun`/`deskact`/`deskins`/`deskinf`/`deskrez`) are **now
> extracted** into their own specs — `deskdir.md` (also covers `deskfpd.c`
> sort: Shell-sort, folders-first, 5 keys + name tiebreak), `deskfun.md`,
> `deskact.md`, `deskins.md`, `deskinf.md` (its appendix holds the exact
> **NEWDESK.INF line grammar**, sourced from `deskapp.c` `app_parse`/
> `build_inf`), `deskrez.md`. The V_TEXT renderer callback (`format_fnode`/
> `dr_code`) flagged *needs deeper analysis* here and in `desksupp-redraw.md`
> is resolved in `deskinf.md`.
>
> **Update (Researcher re-validation, 2026-07-21):** `deskdir.md`/
> `deskfun.md`/`deskact.md` re-checked function-by-function against current
> `references/emutos/desk/` source. One load-bearing correction found and
> fixed: `deskfun.md`'s `fun_desk2win`/`fun_desk2desk` description had the
> "multi-select drag onto an application icon can launch it multiple times"
> behaviour attributed to the **wrong** function (an unconditional `return`
> at `deskfun.c:1250`, misread as a loop-continuing branch, actually makes
> `fun_desk2win` launch **at most once**; the real multi-launch case is
> `fun_desk2desk`, which has no such early exit). `deskpro.c` (previously
> "out of scope", no doc) is now extracted → [`deskpro.md`](deskpro.md);
> `do_aopen`/`do_open` (`desksupp.c`), the function every launch path
> actually calls before reaching `pro_run`, remains **unextracted** — see
> `deskpro.md`/`deskfun.md`'s "Missing coverage" notes for the follow-up
> scope. `deskfpd.c`'s `pn_active`/`pn_open`/`pn_close`/`FNODE`/`DTA`
> mirroring (already in `_data-model.md`) was spot-re-verified and found
> accurate, including the 23-vs-22-byte `memcpy` overrun quirk
> (`deskfpd.c:271`), already correctly documented there.

## How the pieces connect (data flow for one redraw)

1. **Boot-once setup** (`win_start()`, `deskwin.c:105-135`): picks the
   default view type (`V_ICON`) and sort order, calls `win_view()` to
   derive the per-item grid pitch (`g_iwspc`/`g_ihspc`), *then* calls
   `obj_init()` (`deskobj.c:113-147`) which allocates `G.g_screen[]`/
   `g_screeninfo[]` sized using that pitch — **ordering dependency**:
   `win_view()` must run first (`deskwin.c:117-118` comment). Allocates the
   `WNODE` array (`G.g_wlist`, `NUM_WNODES`=7 entries, `deskbind.h:30`/
   `deskconf.h`), chained via `w_next`, all initially free (`w_id=0`).

2. **Icon/grid metrics** (`app_start()`, `deskapp.c:781-1014`, see
   `deskapp.md`): once the built-in/user icon set is loaded, derives the
   universal icon-cell size `g_wicon`×`g_hicon` from icon #0 (the hard-disk
   icon) and the **desktop's** grid pitch `g_icw`×`g_ich` (which evenly
   divides the desktop's pixel width/height — *not* the same computation
   `win_view()` uses for **window** grids).

3. **Opening a window / building the desktop** — either
   `app_blddesk()` (`deskapp.c:1337-1404`, the desktop pseudo-window) or
   `win_bldview()` (`deskwin.c:392-520`, a real directory window) walks a
   list — `ANODE`s flagged `AF_ISDESK` for the desktop, sorted `FNODE`s
   (from `pn_active()`/`pn_sort()`, `deskfpd.c`) for a window — and
   allocates one `G_ICON`/`G_USERDEF` `OBJECT` per visible item via
   `obj_ialloc()` (`deskobj.c:220-240`), placing it on the *virtual* grid
   computed by `win_ocalc()` (`deskwin.c:293-372`, windows only; the
   desktop has no scrolling, so it has no virtual/physical distinction).

4. **Redraw**: any AES `WM_REDRAW` message (`deskmain.c:1279`), any
   view-rebuild (`app_blddesk`, `win_blt` after scrolling, `win_dispfile`),
   or any selection-state change (`deskact.c`) ends by calling
   `do_wredraw(handle, clip_rect)` (`desksupp.c:202-232`, see
   `desksupp-redraw.md`), which walks the AES window manager's **owned
   rectangle list** (`WF_FIRSTXYWH`/`WF_NEXTXYWH` — the non-overlapping
   visible pieces of the window, already extracted in
   `docs/api/aes/window-graphics.md`) and calls `objc_draw()` once per
   piece, clipped to both the AES-owned rectangle and the caller's dirty
   rectangle.

## Config-variant notes for this extraction

Two `#if CONF_WITH_*` gates materially change the rendering core and are
called out per-entry below; both default **on** in the general/Atari
build (the one NewTOS should follow — not the ROM-constrained
`TARGET_192` build):

- **`CONF_WITH_SIZE_TO_FIT`** (`config.h:1850-1852`, default `1`; forced to
  `0` only under `TARGET_192`, `config.h:334-336`, a 192 KB-ROM ST-only
  build that strips advanced features for space). Changes `win_ocalc`'s
  virtual-column calculation, `win_bldview`'s horizontal-fit/skip logic,
  and adds the horizontal slider math. NewTOS should extract the
  `CONF_WITH_SIZE_TO_FIT=1` variant (full behaviour).
- **`CONF_WITH_BACKGROUNDS`** (`config.h:1747-1749`, default `1`; forced to
  `0` under `TARGET_192`, `config.h:328-330`). Changes `set_background()`'s
  `ob_spec` value and `INITIAL_ICON_STATE` (`deskwin.c:57-61`: `0` vs
  `WHITEBAK`, i.e. whether icon backgrounds are hard-coded white or
  configurable — this `INITIAL_ICON_STATE` detail is new in this
  extraction). The gate itself is already noted in `docs/api/desktop.md`
  (re the "Set background" menu item); repeated here in `deskapp.md`
  where it affects the grid-adjacent `set_background` call.

## Cross-references to already-extracted AES contract

- `docs/api/structures/object.md` — `OBJECT` field layout (`ob_x/y/width/
  height`, `ob_head/tail/next`, `ob_type`, `ob_spec`, `ob_state`/`ob_flags`
  bits `SELECTED`/`LASTOB`/…) — used verbatim by `g_screen[]`.
- `docs/api/structures/iconblk.md` — `ICONBLK` field layout (`ib_xicon`/
  `ib_wicon`/`ib_xtext`/`ib_wtext`/`ib_char`/`ib_ptext`/…) — used verbatim
  by `g_iblist[]`/`SCREENINFO.u.icon.block`.
- `docs/api/aes/window-graphics.md` — `wind_calc_grect`, the `WF_*`
  `wind_get`/`wind_set` field IDs, and (critically) the
  `WF_FIRSTXYWH`/`WF_NEXTXYWH` rectangle-list contract that
  `desksupp-redraw.md` builds on rather than re-deriving.
- `docs/api/aes/event.md` — the AES message array layout (`G.g_rmsg[]`
  ≙ `int_out`/message buffer), relevant to how `deskmain.c` feeds
  `do_wredraw` its clip rectangle.
- `docs/api/desktop/gembind.md` (new, this validation pass) — the desktop's
  full AES/GEMDOS **call-surface** inventory: which `docs/api/aes/`-
  documented functions `desk/` actually calls, from where, how many times,
  and the one found non-trap dependency (`gsx_trans`, `aes/gemgraf.c:369`)
  a genuinely separate-process NewTOS desktop cannot satisfy the same way
  EmuTOS's monolithic-binary `desk/` does.
