# `desksupp.c` — `do_wredraw` and the redraw rectangle loop

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/desksupp.c:202-232`. This entry covers only `do_wredraw()`; the
> rest of `desksupp.c` (selection clearing, `do_xyfix`/`do_wopen`/
> `do_wfull`, disk format, show-file/print-file) is out of scope for the
> rendering-core task (see `_overview.md`'s file map) and not re-derived
> here beyond what's needed to place `do_wredraw` in context.

## EmuTOS source

**Primary location:** `desk/desksupp.c:202-232` — `do_wredraw(WORD
w_handle, GRECT *gptr)`

**Installed handler / vector:** `(none)` — called directly, not through
an indirect vector. It is, however, the terminal step of the AES
`WM_REDRAW` message handling in `deskmain.c` (see "Used by" below).

**Helpers called:**
- `desk/deskwin.c:198` `win_find()` — resolves `w_handle` to its `WNODE`
  (to get `w_root`); returns `NULL`/no-op if the window no longer exists
- `util/cookie.c:83` `get_idt_cookie()` (not traced further; declared
  `include/cookie.h:94`) — run via `Supexec()` in supervisor mode to
  refresh `G.g_idt` from the `_IDT` cookie-jar entry
- AES: `graf_mouse(M_OFF/M_ON, NULL)` — hide/show the mouse cursor sprite
  around the draw (avoids the cursor XOR image corrupting the blit)
- AES: `wind_get_grect(w_handle, WF_FIRSTXYWH/WF_NEXTXYWH, &t)` — the
  **rectangle-list walk**, see below
- `util/rectfunc.c` (not traced here
; already extracted) —
  `rc_intersect(&clip_r, &t)` — clips the AES-owned rectangle `t` to the
  caller's dirty rectangle `clip_r`, returns whether the result is
  non-empty
- AES: `objc_draw(G.g_screen, root, MAX_DEPTH, t.g_x, t.g_y, t.g_w,
  t.g_h)` — the actual paint call, one per non-empty intersected
  rectangle

**Headers / type definitions used:**
- `include/obdefs.h:30` — `MAX_DEPTH` = 8 (max `objc_draw`/`objc_find`
  recursion depth)
- `include/aesdefs.h:544-545` — `WF_FIRSTXYWH`=11, `WF_NEXTXYWH`=12
- `desk/deskwin.h:44` — `DROOT`=1
- `desk/deskbind.h:30` — `DESKWH`=0

**System variables / globals read:**
- `G.g_screen` (`desk/deskbind.h:233`) — the object tree passed to
  `objc_draw`

**System variables / globals written:**
- `G.g_idt` (`desk/deskbind.h:183`) — refreshed every call, **even if
  the loop below never finds a non-empty rectangle** (the `Supexec` call
  happens unconditionally before the loop)

**Build configuration affecting this function:** none — `do_wredraw`
itself has no `#if CONF_WITH_*` gating.

---

## Source extraction summary

`do_wredraw(w_handle, gptr)` repaints whatever part of the AES object
tree rooted at a window (or the desktop) is (a) currently visible on
screen — i.e. not covered by some other, higher window — **and** (b)
within the caller-supplied dirty rectangle `*gptr`. It does not compute
"visible" itself: that's the AES window manager's job, exposed through
the `wind_get(WF_FIRSTXYWH)`/`wind_get(WF_NEXTXYWH)` pair, which iterates
a window's **owner rectangle list** — the set of non-overlapping
rectangles that together make up the parts of the window's work area not
obscured by any window above it. `do_wredraw` walks that list, intersects
each piece with its own dirty rectangle, and calls `objc_draw` once per
non-empty intersection — so a window half-covered by another window, with
only a small corner of it marked dirty, results in at most one `objc_draw`
call covering just that corner, not a call for the whole window.

---

## The rectangle-iteration loop, precisely

```c
void do_wredraw(WORD w_handle, GRECT *gptr)
{
    GRECT clip_r, t;
    WNODE *pw;
    WORD root;

    clip_r = *gptr;                              /* desksupp.c:208 */

    root = DROOT;                                /* desksupp.c:210 */
    if (w_handle != DESKWH)
    {
        pw = win_find(w_handle);
        if (!pw)            /* window (no longer) exists */
            return;                              /* desksupp.c:214-215 */
        root = pw->w_root;
    }

    G.g_idt = Supexec((LONG)get_idt_cookie);      /* desksupp.c:219 */

    graf_mouse(M_OFF, NULL);                      /* desksupp.c:221 */

    wind_get_grect(w_handle, WF_FIRSTXYWH, &t);   /* desksupp.c:223 */
    while (t.g_w && t.g_h)                        /* desksupp.c:224 */
    {
        if (rc_intersect(&clip_r, &t))            /* desksupp.c:226 */
            objc_draw(G.g_screen, root, MAX_DEPTH, /* desksupp.c:227 */
                      t.g_x, t.g_y, t.g_w, t.g_h);
        wind_get_grect(w_handle, WF_NEXTXYWH, &t); /* desksupp.c:228 */
    }

    graf_mouse(M_ON, NULL);                       /* desksupp.c:231 */
}
```

Step by step:

1. **Resolve the object-tree root** (`:210-217`): `w_handle==DESKWH` (0)
   always uses `DROOT` (the desktop's `g_screen[]` root, index 1);
   otherwise looks up the `WNODE` by AES handle and uses its `w_root`.
   **If the window handle no longer maps to a live `WNODE`** (closed
   between the redraw being queued and processed), the function returns
   immediately — no drawing, no mouse-cursor toggling, nothing. This is
   the same defensive `win_find()`-returns-`NULL` pattern used throughout
   `deskwin.c`.
2. **Refresh `G.g_idt`** (`:219`) unconditionally, via a supervisor-mode
   call to `get_idt_cookie()` (`util/cookie.c:83`, not traced further) —
   the comment says this is "for `format_fnode()`", i.e. it ensures the
   `_IDT` cookie's date/time-format
   preference is current before any `V_TEXT`-mode `dr_code` callback
   formats a file's date/time during this redraw. **Not traced further**
   (`format_fnode`/`dr_code` live in `desk/deskinf.c`, out of scope for
   this task — flagged needs-deeper-analysis if a later task covers
   text-view line formatting).
3. **Hide the mouse cursor** (`graf_mouse(M_OFF,NULL)`) for the duration
   of the draw, restored (`M_ON`) at the end — an AES-level idiom (not
   re-derived here; `graf_mouse` is presumably already contract-
   documented under `docs/api/aes/`).
4. **The rectangle walk** (`:223-229`): `wind_get_grect(handle,
   WF_FIRSTXYWH, &t)` seeds `t` with the **first** owner rectangle of the
   window's *work area* (already AES-clipped to what's not covered by a
   higher window); the loop condition `t.g_w && t.g_h` is the AES
   contract's **termination sentinel** — a rectangle with zero width
   *or* zero height means "no (more) visible pieces," both at the very
   first call (fully-obscured window: nothing to draw at all) and after
   the last real piece (`WF_NEXTXYWH` returns `w=h=0`). For each
   non-zero piece: `rc_intersect(&clip_r, &t)` clips it *in place* against
   the caller's own dirty rectangle and returns whether anything remains;
   only if so does `objc_draw` get called, painting exactly that
   intersected rectangle. **Every** iteration re-fetches the next piece
   via `WF_NEXTXYWH` regardless of whether the current one intersected —
   i.e. the loop always walks the *entire* owner-rectangle list once,
   even if most pieces fall outside `clip_r`.
5. `objc_draw(G.g_screen, root, MAX_DEPTH, x, y, w, h)` — draws the
   subtree at `root` (depth limit `MAX_DEPTH`=8, `obdefs.h:30`; never
   binding here since the desktop only ever builds 2-level trees: a
   `G_IBOX` root with flat `G_ICON`/`G_USERDEF` children — see
   `_data-model.md`'s `g_screen[]` layout), clipped to the given
   rectangle. `objc_draw`'s own clipping/redraw-per-object behaviour is
   AES-internal and not re-derived here.

**The rectangle-list mechanics themselves (`WF_FIRSTXYWH`/
`WF_NEXTXYWH`, the owner-rectangle list `w_rlist`/`w_rnext`, how it's
built by `newrect`/`mkrect`/`brkrct`) are AES-internal and already fully
extracted in `docs/api/aes/window-graphics.md`** ("The rectangle-list
iteration" section) — this entry deliberately does not re-derive them,
per the task's scope boundary (rendering core of `desk/`, not AES
internals). What's new here is how the **desktop application** consumes
that contract: one clip-and-draw per AES-yielded piece, using its *own*
window-to-root mapping (`DROOT` vs. `WNODE.w_root`) rather than any
AES-level window-content pointer.

---

## Edge cases discovered in source

- **Silent no-op on a stale handle**: a `WM_REDRAW` message for an
  already-closed window causes `do_wredraw` to return before doing
  *anything* — not even the mouse-cursor toggle. This means a caller that
  relies on `graf_mouse` being called as a side effect (there is none such
  in this codebase, but worth noting) would be surprised.
- **`G.g_idt` is refreshed even when nothing gets drawn** (e.g. the
  window is fully obscured, so the `while` loop body never executes) —
  the `Supexec` call happens unconditionally before the loop starts.
- **The loop always fully drains `WF_NEXTXYWH`** even after finding (and
  drawing) an intersecting piece — there's no early exit once the dirty
  rectangle has been "fully accounted for"; a window with many small
  owner rectangles (many overlapping windows above it) pays for every one
  of them even if only the first intersects.
- **`objc_draw` gets the *intersected* rectangle, not the original AES
  piece** — so a dirty rectangle smaller than a given visible piece
  results in a `objc_draw` call scoped to just the overlap, potentially
  much smaller than either input rectangle.

---

## Dependencies

### Requires (must be initialized / functional)

- `G.g_screen` — the object tree must already reflect the desired
  content (i.e. `app_blddesk()`/`win_bldview()` must have already run;
  `do_wredraw` only **paints**, it does not build the object tree)
- The AES window manager's owner-rectangle list for `w_handle` (built
  and maintained by AES window creation/move/resize/top — out of scope,
  see `docs/api/aes/window-graphics.md`)
- `get_idt_cookie`/`G.g_idt` — for correct date/time formatting in
  `V_TEXT` mode (not required for `V_ICON` mode's correctness, but always
  refreshed regardless of view mode)

### Used by

Call sites (all in `desk/`, cited for the dependency graph — not
individually re-derived):

| Call site | Context |
|---|---|
| `desk/deskmain.c:1279` | The **primary** caller: AES `WM_REDRAW` message handling in the main event switch, `do_wredraw(handle, (GRECT*)&G.g_rmsg[4])` — the dirty rectangle comes straight from the AES message buffer's `int_out[4..7]` slot (see `docs/api/aes/event.md` for the message layout contract) |
| `desk/deskwin.c:635` | `win_blt()` — after a scroll, redraws whatever the partial `bb_screen` blit didn't cover |
| `desk/deskwin.c:673` | `win_dispfile()` (`CONF_WITH_SEARCH`) — after scrolling to show a searched-for file |
| `desk/deskapp.c:1403` | `app_blddesk()` — full desktop rebuild, `do_wredraw(DESKWH, &G.g_desk)` |
| `desk/deskact.c:156,159,461,550` | Selection/highlight state changes (out of scope — interaction, not rendering) |
| `desk/deskfun.c:591` | After a file operation (copy/move/delete/rename, out of scope), "redraw the original window (may be desktop)" |
| `desk/deskinf.c:907` | After an install/preferences dialog changes desktop content (out of scope) |

## Related data structures

- `docs/api/aes/window-graphics.md` — `WF_FIRSTXYWH`/`WF_NEXTXYWH`
  contract, `wind_get_grect` helper shape
- `docs/api/aes/event.md` — AES message buffer layout (`G.g_rmsg[]`)
- `docs/api/structures/object.md` — `OBJECT` (what `objc_draw` walks)
- `docs/api/desktop/_data-model.md` — `G.g_screen[]` root-index layout
  (`DROOT`, `WNODE.w_root`)

## NewTOS implementation notes

### `do_info` — *Show info*, one box per icon type (DESK38, 2026-09-25)

`main::do_info_rename` returned immediately for `G.g_cwin == DESKWH`, so *Show
info* did not exist on the desktop surface **at all** — not for a drive, not
for the trash, not for the printer, and since DESK30 not for a shortcut
either. EmuTOS's `do_info` (`desksupp.c:1288-1354`) is a switch on the icon's
`a_type`, and every one of those four is an arm of it.

```c
pa = i_find(G.g_cwin, curr, &pf, NULL);
if (!pa) return ret;
switch(pa->a_type) {
case AT_ISFOLD: case AT_ISFILE:  ... inf_file_folder(pathptr, pf) ...  break;
case AT_ISDISK: drive = LOBYTE(get_iconblk_ptr(G.g_screen,curr)->ib_char);
                ret = inf_disk(drive); break;
case AT_ISTRSH: trash_alert(pa); break;
case AT_ISPRNT: printer_alert(pa); break;
}
return ret;
```

* **`AT_ISFOLD`/`AT_ISFILE`** has two sources for the same box. A **window**
  item takes its fields from the `FNODE` the object carries and its directory
  from the window's `p_spec`; a **desktop shortcut** (`AF_ISDESK`, which every
  `global::Anode` implicitly is) has no `FNODE`, so EmuTOS probes the file it
  points at — `file_exists(pa->a_pappl, NULL)` — and builds one on its stack
  from the DTA (`memcpy(&pf->f_attr, &dta->d_attrib, 23)`, `:1326`). A
  shortcut whose file is gone goes to `remove_locate_shortcut` instead of
  showing a box, exactly as opening it would. `win_find(DESKWH)` is the desktop
  pseudo-window and never NULL (`deskwin.c:202-203`), so the `if (pw)` guard is
  always taken on the desktop.
* **`AT_ISDISK`** is `inf_disk` — [`deskinf.md`](deskinf.md).
* **`AT_ISTRSH` / `AT_ISPRNT`** are `trash_alert` (`:1165-1172`) and
  `printer_alert` (`:1177-1183`), `STTRINFO` and `STPRINFO`
  (`desk_rsc.c:2517-2521`) with the icon's **current** name merged in — read
  from the `ANODE`, which is why renaming the trash changes its alert too.

### The `SHOWITEM` loop and `fun_rebld_marked`

`do_filemenu`'s `SHOWITEM` (`deskmain.c:674-689`) is a loop, and NewTOS only
ever showed one box:

```c
if (curr) {
    for ( ; curr; curr = win_isel(G.g_screen, G.g_croot, curr))
        if (!do_info(curr)) break;
    fun_rebld_marked();     /* rebuild any changed windows */
    break;
}
if (pw) inf_disk(pw->w_pnode.p_spec[0]);
```

Three selected items give three boxes in turn, and *Cancel* on one ends the
series — that is `do_info`'s `0`, which is why `inf_file_folder` had to grow a
third return value ([`deskinf.md`](deskinf.md)). With **nothing** selected the
top window's drive gets the disk box, "just like TOS" (`:683-684`).

`fun_rebld_marked` runs after the loop, not inside `do_info`: rebuilding a
window throws away the very `FNODE`s the loop is walking. That is the whole
reason `fun_mark_for_rebld`/`WN_REBUILD` exist — see
[`deskfun.md`](deskfun.md).

DESK38 named a deviation here — the rename happened in `do_info`'s helper
rather than inside `inf_file_folder`, and there was no `STRENAME` *Retry*
loop. **DESK42 closed it:** `inf_file_folder(ppath, pf)` renames and changes
attributes itself, in EmuTOS's `while(1)`, and the arm here is the source's two
lines (`ret = inf_file_folder(pathptr, pf); if (ret < 0)
fun_mark_for_rebld(pathptr);`). Details in [`deskinf.md`](deskinf.md).

### Measured

`measurements/measurement-2026-09-25-desk-rest/` §3 — all four icon types on the desktop
(`[d38] do_info(curr=…) cwin=0 atype=Disk|Trash|Printer|File`) plus the window
item, with a screendump of each box. Pi checklist item 69.
