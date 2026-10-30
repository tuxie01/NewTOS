# AES Window Manager (`wind_*` 100–109) + Graphics Manager (`graf_*` 70–79)

> **PHASE-00b D8, Batch 6 (full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemwmlib.c` (`wm_create`/`wm_open`/`wm_close`/`wm_delete`/
> `wm_get`/`wm_set`/`wm_find`/`wm_update`/`wm_calc`/`wm_new`, the frame builder
> `w_bldactive`, the redraw driver `draw_change`/`w_update`, the topped/untopped
> title colour), `aes/gemwrect.c` (the rectangle-list algorithm
> `newrect`/`mkrect`/`brkrct`/`w_owns`), `aes/gemgrlib.c` (`gr_rubbox`/
> `gr_dragbox`/`gr_movebox`/`gr_growbox`/`gr_shrinkbox`/`gr_watchbox`/
> `gr_slidebox`/`gr_mouse`/`gr_mkstate`); the WM_\* messages are built by the
> **control manager** `aes/gemctrl.c:81-315` (`ct_msgup`/`hctl_window`/
> `handle_arrow_msg`/`perform_untop`) → `aes/gemwmlib.c:851-863` (`ap_sendmsg`);
> dispatch arm `aes/gemsuper.c:265-360`; constants `include/aesdefs.h:68-77,
> 89-98,226-253,390-562`; window structs `aes/gemlib.h:38-94`.
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (the window list `D.w_win[]`, `gl_wtop`, the rectangle
> list, the input mirrors `xrat`/`yrat`/`button`/`kstate`): [`_state.md`](./_state.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** the opcode numbers (100–109, 70–79); the AESPB slot layout of
> each call; the **window-kind gadget bits** (`NAME`…`HSLIDE`); the **`WF_*`
> field numbers** and each field's `wind_get`/`wind_set` return; the **`WC_BORDER
> 0`/`WC_WORK 1`** `wind_calc` border↔work model; the **`WF_FIRSTXYWH`/
> `WF_NEXTXYWH` rectangle-list contract** (non-overlapping visible pieces of the
> work area, walked until an empty rect); the **`WM_*` message ids + 16-byte
> word layouts** and the **`WA_*` arrowed sub-actions**; the **`graf_mouse`
> form numbers** (0–7, `USER_DEF`, `M_OFF`/`M_ON`); `graf_handle`'s VDI-handle +
> cell-size return; `graf_mkstate`'s x/y/button/shift; the `wind_update`
> BEG/END_UPDATE / BEG/END_MCTRL modes; the `wind_create` handle / `-1` return.
> **Reference (NewTOS reimplements freely):** `w_bldactive`'s W_ACTIVE
> frame-object tree and the exact XOR rubber-band tracking (`gr_rubwind`/
> `gr_wait`/`gr_2box`/`gr_stepcalc`), the `ORECT` free-list allocator
> (`or_start`/`get_orect`), the `WINDOW`/`ORECT` structs, the `w_move`/`bb_screen`
> blit optimisation, and the cooperative screen-lock (`tak_flag`/`ev_block`/
> `fm_own`) — these are EmuTOS internals NewTOS reimplements over its own
> object manager, framebuffer XOR draw, event/IPC model, and process locks.

---

## The frame IS a private OBJECT tree (reference)

The window *border* is not a data structure the application sees. EmuTOS keeps
one scratch object tree `W_ACTIVE[NUM_ELEM]` (`gemwmlib.c:58`, `NUM_ELEM = 19`)
and, each time it must draw a window's frame, `w_bldactive(handle)`
(`gemwmlib.c:479`/`716`) rebuilds that tree from the window's `w_kind` and
current rect, then `do_walk`/`ob_draw` paints it clipped to the rectangle list.
The element indices (`gemlib.h:38-56`) are:

| # | Element | Object type | # | Element | Object type |
|--:|---------|-------------|--:|---------|-------------|
| 0 | `W_BOX` | `G_IBOX` | 10 | `W_UPARROW` | `G_BOXCHAR` |
| 1 | `W_TITLE` | `G_BOX` | 11 | `W_DNARROW` | `G_BOXCHAR` |
| 2 | `W_CLOSER` | `G_BOXCHAR` | 12 | `W_VSLIDE` | `G_BOX` |
| 3 | `W_NAME` | `G_BOXTEXT` | 13 | `W_VELEV` | `G_BOX` |
| 4 | `W_FULLER` | `G_BOXCHAR` | 14 | `W_HBAR` | `G_BOX` |
| 5 | `W_INFO` | `G_BOXTEXT` | 15 | `W_LFARROW` | `G_BOXCHAR` |
| 6 | `W_DATA` | `G_IBOX` | 16 | `W_RTARROW` | `G_BOXCHAR` |
| 7 | `W_WORK` | `G_IBOX` | 17 | `W_HSLIDE` | `G_BOX` |
| 8 | `W_SIZER` | `G_BOXCHAR` | 18 | `W_HELEV` | `G_BOX` |
| 9 | `W_VBAR` | `G_BOX` | | | |

This tree, `w_bldactive`, and the pixel drawing are **reference** — the object
manager ([`object-form.md`](./object-form.md)) does the drawing. What is
**contract** is the *observable geometry* the frame produces: the work rect
(`WF_WORKXYWH`), the border thicknesses (`wind_calc`), and which pixels each
gadget occupies (fed back to the app through `wind_get`).

### Active vs. inactive title cue (`gemwmlib.c:150-151`, `537`, `778`)

The topped (active) window's name is drawn differently from an untopped one.
The two TEDINFO colour words (in the non-`CONF_WITH_WINDOW_COLOURS` build) are:

| State | `te_color` | Decoded (border/text/mode/pattern/fill) |
|-------|-----------:|------------------------------------------|
| **topped / active** | `TOPPED_COLOR` = **0x11a1** | border 1, text 1, **opaque**, **`IP_2PATT` (pattern 2)**, fill 1 |
| **untopped / inactive** | `UNTOPPED_COLOR` = **0x1100** | border 1, text 1, transparent, `IP_HOLLOW` (pattern 0), fill 0 |

> **Cue direction (contract, corrects an easy-to-invert assumption).** The
> **active/topped** title carries the **`IP_2PATT` dither** fill (`0x11a1`); the
> **inactive/untopped** title is **plain/hollow** (`0x1100`)
> (`gemwmlib.c:150-151`). `w_bldactive` also sets `W_NAME.ob_state = NORMAL`
> when topped, `DISABLED` when not (`gemwmlib.c:537`/`778`); on an untopped
> window the CLOSER/FULLER/scroll gadgets are not built at all in the non-3D
> build (drawn only `if (istop)`, `gemwmlib.c:758-773`). Pattern-2 fill under
> the title bytes = "this is the active window". (Constants `IP_HOLLOW`=0,
> `IP_2PATT`=2, `obdefs.h:32-39`.)

> **Gap found while extracting A79 (2026-09-20): the two rows above are the
> `!CONF_WITH_WINDOW_COLOURS` compile-time constants (`gemwmlib.c:538-539,
> 779-780`, gated `#if !CONF_WITH_WINDOW_COLOURS`), which never execute on our
> profile — `CONF_WITH_WINDOW_COLOURS` defaults to **1** (`include/config.h:1324-1330`),
> required for `AES_VERSION 0x0340` (`include/config.h:1346`).** With it on (the actual
> build), `W_NAME`'s topped/untopped words instead come from `gl_wtcolor`/
> `gl_wbcolor[W_NAME]`, copied into every window at `w_setup`
> (`gemwmlib.c:220-227`) and written by `w_setcolor` (`:382-396`). The
> **topped** default is identical (`gl_wtcolor[W_NAME] = 0x1101 | 0xa0 =
> 0x11a1` = `TOPPED_COLOR`, `wm_start`, `gemwmlib.c:1352`); the **untopped**
> default is `0x1101`, not `UNTOPPED_COLOR` (`0x1100`) — `ic` is 1 (`BLACK`)
> rather than 0 (`WHITE`). This has no visible consequence (`ip=IP_HOLLOW`
> either way, so the fill is unpainted regardless of `ic`, see "`WF_COLOR`/
> `WF_DCOLOR` → pixel mapping" under "NewTOS implementation notes" below), but
> it means the two rows above are **not** what a `wind_get(WF_COLOR, W_NAME)`
> on an untopped window returns on this profile — that call returns `0x1101`.

---

## Window-kind gadget bits (`wind_create` `kind`, `aesdefs.h:519-531`)

`kind` is an OR of the widgets the border shows:

| Bit | hex | Widget | Bit | hex | Widget |
|-----|----:|--------|-----|----:|--------|
| `NAME`   | 0x0001 | title bar (name) | `UPARROW` | 0x0040 | scroll-up arrow |
| `CLOSER` | 0x0002 | close box | `DNARROW` | 0x0080 | scroll-down arrow |
| `FULLER` | 0x0004 | full box | `VSLIDE`  | 0x0100 | vertical slider |
| `MOVER`  | 0x0008 | drag / move bar | `LFARROW` | 0x0200 | scroll-left arrow |
| `INFO`   | 0x0010 | info line | `RTARROW` | 0x0400 | scroll-right arrow |
| `SIZER`  | 0x0020 | size box | `HSLIDE`  | 0x0800 | horizontal slider |
| `HOTCLOSE` | 0x1000 | close-on-single-click *(PC-GEM, `CONF_WITH_PCGEM`)* — see below | | | |

> **`HOTCLOSE` (0x1000)** (`aesdefs.h:531`, "PC-GEM feature"; `CONF_WITH_PCGEM`
> is **1** in the 512 profile, `include/config.h:1319-1321` — only `TARGET_192` turns
> it off, `:352-354`). The bit draws nothing and needs `CLOSER` for a closer
> to exist (`w_bldactive` adds `W_CLOSER` only for `CLOSER`); it changes what
> the Control Manager does with a press on that closer (`hctl_window`,
> `gemctrl.c:208-214`): `if (kind & HOTCLOSE) { message = WM_CLOSED; break; }`
> — **`WM_CLOSED` is sent on the press**, without `gr_watchbox`'s
> press-and-release-inside cycle. Having posted it, `ct_msgup` does not wait
> for the release either but **fakes the button up** (`:86-96`: `button = 0;
> return;` — "otherwise a slow click will cause us to stay in ctlmgr(), eating
> keys"), so the press still held neither re-triggers the Control Manager nor
> satisfies the application's `MU_BUTTON` wait (`ev_multi`'s quick check and
> `abutton` test `button`, `gemevlib.c:185-200`, `geminput.c:549`); the
> physical release is a non-change, the next press starts afresh. **NewTOS**
> (A49): `window::intercept`'s closer arm posts `WM_CLOSED` at once for a
> `HOTCLOSE` window and calls `input::ct_msgup_hotclose` (`button = 0`); the
> Control Manager's entry test and `evnt_multi`'s button level read the AES's
> `button`, so they see the fake. Host test
> `tos-api/tests/aes_window_hotclose.rs`; on hardware a visible check only
> (QEMU has no pointer).

Gadget **groups** used by the frame builder and `wind_calc` (`gemwmlib.h:16-18`):

- `TGADGETS = NAME | CLOSER | FULLER | MOVER` — anything in the **title band**.
- `VGADGETS = UPARROW | DNARROW | VSLIDE` — the **vertical scroll bar**.
- `HGADGETS = LFARROW | RTARROW | HSLIDE` — the **horizontal scroll bar**.

> **Edge case (`gemwmlib.c:542-551`, `CONF_WITH_3D_OBJECTS` build only — see
> D-097):** in the TOS4-compatible (3D-object) build you may specify `MOVER`
> **without** `NAME` — a drag bar with no name. The non-3D build has no such
> case (`gemwmlib.c:750-786`): `MOVER` alone reserves no title band there, or
> in `wind_calc`. A `SIZER` with no scroll gadgets still forces a vertical bar
> band (see `wind_calc`).

---

## `WF_*` field numbers (`wind_get`/`wind_set`, `aesdefs.h:534-552`)

| # | Name | Meaning | # | Name | Meaning |
|--:|------|---------|--:|------|---------|
| 1 | `WF_KIND` | kind bitmask † | 11 | `WF_FIRSTXYWH` | first visible rect (list head) |
| 2 | `WF_NAME` | title string ptr ‖ | 12 | `WF_NEXTXYWH` | next visible rect |
| 3 | `WF_INFO` | info string ptr ‖ | 14 | `WF_NEWDESK` | desktop background tree ‡ |
| 4 | `WF_WXYWH` (WORKXYWH) | work-area rect | 15 | `WF_HSLSIZ` | h-slider size 1..1000 / −1 |
| 5 | `WF_CXYWH` (CURRXYWH) | current (border) rect | 16 | `WF_VSLSIZ` | v-slider size 1..1000 / −1 |
| 6 | `WF_PXYWH` (PREVXYWH) | previous rect | 17 | `WF_SCREEN` | menu/alert buffer addr+len |
| 7 | `WF_FXYWH` (FULLXYWH) | full-screen rect | 18 | `WF_COLOR` | per-window gadget colour § |
| 8 | `WF_HSLIDE` | h-slider position 1..1000 | 19 | `WF_DCOLOR` | default gadget colour § |
| 9 | `WF_VSLIDE` | v-slider position 1..1000 | 20 | `WF_OWNER` | owner pid + z-order ¶ |
| 10 | `WF_TOP` | handle of the top window | | | |

- † `WF_KIND` (1) is **not** handled by `wm_get`/`wm_set` in this source — it
  falls into `default: return FALSE`. See "Fields with no handler" below.
- ‡ `WF_NEWDESK` (14) is **`wind_set`-only** (sets the desktop-background object
  tree `gl_newdesk`/`gl_newroot`); `wm_get` has no case for it → `FALSE`. Where
  the root index (`gl_newroot`) travels depends on which pointer convention
  the caller used — EmuTOS always packs (`pinwds[2]` = `int_in[4]`); NewTOS
  additionally accepts the native `addr_in[0]` + `int_in[2]` form. See the
  `WF_NEWDESK` row under "`wind_set` (105)" and "Edge cases discovered in
  source" item 16 (A81) below.
- ‖ `WF_NAME` (2) and `WF_INFO` (3) are **`wind_set`-only** as well
  (`gemwmlib.c:1660`, `:1668`). `wm_get` has no case for either, so both fall
  into `default: return FALSE` (`:1591-1592`) and the caller's string buffer is
  **never written** — the AES stores a title, it does not hand one back. Note
  that gemlib *declares* the read (`a_wind_get.c:344` packs a `char*` into
  `int_in[2..]`): the binding is not the authority, the AES is. Measured on
  EmuTOS 1.4: `tosdump/cases/aes-m05-addrin.tsv` case 102 → `int_out[0] = 0` on
  a window with a title (TASK/A42).
- § `WF_COLOR` (18) / `WF_DCOLOR` (19) exist only with
  `CONF_WITH_WINDOW_COLOURS` (default **1**). `wind_set` uses a **3-word
  `int_in`** (gadget#, topped-colour, untopped-colour) — the one `wind_set`
  exception to the usual 2-word input (`gemwmlib.c:1719-1734`); `wind_get`
  needs only **one** input word (gadget#, `pinwds[0]` = `int_in[2]`) and
  returns the two colours **not** at `int_out[1]`/`int_out[2]` but at
  `int_out[2]`/`int_out[3]` (`poutwds[1]`/`poutwds[2]` — **D-095**, see
  `wind_get (104)` below).
- ¶ `WF_OWNER` (20) exists only with `AES_VERSION >= 0x330` (the default build
  is 0x0340). Returns owner pid, open-flag, and (a partial) top/bottom z-order.
  **The owner pid reported is 0 for every window created by a chain member**
  (desktop, program, `Pexec` child) and the accessory's pid (2, 3, …) for a
  window created by an accessory (corrected 2026-10-17, A113) (`w_owner->p_pid`,
  `gemwmlib.c:1580`; `w_owner` set to `rlr` by `w_setup` at `wind_create` time,
  `:1377`, and for the desktop window (`DESKWH`) by `wm_init`, `:1255,1290`) —
  see [`application.md`](./application.md) §"AES process identity and pid"
  (corrected 2026-09-20/A62+A56).

**Note there is no `WF_HSLSIZE`/`WF_VSLSIZE` spelling** in the source — the
macros are `WF_HSLSIZ` (15) / `WF_VSLSIZ` (16). The field *numbers* 15/16 match
the Compendium `WF_HSLSIZE`/`WF_VSLSIZE`.

---

## Trap interface (parameter-block form)

All `wind_*`/`graf_*` calls enter via the GEM trap (`svc #2`), **selector 200
in `x0`**, AESPB pointer in `x1`. The dispatcher (`gemsuper.c:xif`) reads
`control[0]`=opcode and `control[1..3]`=#int_in/#int_out/#addr_in, calls the
handler, and returns **`int_out[0]` = `RET_CODE`** (`aesdefs.h:140`). `RET_CODE`
is initialised to `TRUE` (1) and stays 1 for the `void` handlers; the geometry
results travel in **`int_out[1..4]`** (`WM_OX..WM_OH`, `GR_O1/O2`, `GR_WCHAR..`,
`GR_MX..`). See [`_dispatcher.md`](./_dispatcher.md).

### Window Manager AESPB slots (`aesdefs.h:471-516`)

| Op | Function | `int_in` | `int_out` (`[0]`=ret) | `addr_in` |
|---:|----------|----------|-----------------------|-----------|
| 100 | `wind_create` | `[0]`=kind, `[1..4]`=full-size x/y/w/h | `[0]`=**handle** ≥0 / −1 | — |
| 101 | `wind_open`  | `[0]`=handle, `[1..4]`=x/y/w/h | `[0]`=1/0 | — |
| 102 | `wind_close` | `[0]`=handle | `[0]`=1/0 | — |
| 103 | `wind_delete`| `[0]`=handle | `[0]`=1/0 | — |
| 104 | `wind_get`   | `[0]`=handle, `[1]`=field, (`[2]`=gadget for COLOR) | `[0]`=1/0; `[1..4]`=`WM_OX..WM_OH` | — |
| 105 | `wind_set`   | `[0]`=handle, `[1]`=field, `[2..5]`=value(s) — for `WF_NAME`/`INFO`/`NEWDESK`, `[2..3]` is a **packed pointer** (D-094) | `[0]`=1/0 | — |
| 106 | `wind_find`  | `[0]`=mx, `[1]`=my | `[0]`=**handle** (0=desktop) | — |
| 107 | `wind_update`| `[0]`=mode (0/1/2/3) | `[0]`=1 | — |
| 108 | `wind_calc`  | `[0]`=type, `[1]`=kind, `[2..5]`=in x/y/w/h | `[0]`=1; `[1..4]`=`WM_WCOX..WM_WCOH` | — |
| 109 | `wind_new`   | — | `[0]`=1 | — |

> **`wind_get`/`wind_set` value slots.** The generic input value is `int_in[2]`
> (`WM_IX`) onward; for rect fields it is `int_in[2..5]` = x/y/w/h (`WM_IX..
> WM_IH`, `aesdefs.h:504-507`); for slider fields `int_in[2]` (`WM_ISLIDE`); for
> `WF_NAME`/`WF_INFO`/`WF_NEWDESK` the pointer is **packed as a LONG across
> `int_in[2..3]`** (`WM_IX`/`WM_IY`), read via `*(char **)pinwds` where
> `pinwds = &int_in[2]` — **not** through `addr_in[0]`. `aesdefs.h:502` defines
> a `WM_IOTITLE = addr_in[0]` macro seemingly for this purpose, but it is never
> referenced anywhere in the EmuTOS tree; the `WIND_SET`/`WIND_GET` dispatch
> arms (`gemsuper.c:342-347`) don't even pass an `addr_in` argument to
> `wm_set`/`wm_get`. See **D-094** — this corrects the previous spec, which
> wrongly documented the pointer as travelling via `addr_in[0]`. `wind_get`
> results come back in `int_out[1..4]` for rects/z-order, `poutwds[0]`
> (`=int_out[1]`) for most scalars, and `poutwds[1..2]` (`=int_out[2..3]`) for
> `WF_COLOR`/`WF_DCOLOR` (see **D-095**). `WF_FIRSTXYWH`/`WF_NEXTXYWH` fills
> `int_out[1..4]` with a rect.

### Graphics Manager AESPB slots (`aesdefs.h:390-421`)

| Op | Function | `int_in` | `int_out` (`[0]`=ret) | `addr_in` |
|---:|----------|----------|-----------------------|-----------|
| 70 | `graf_rubberbox` | `[0..3]`=x,y,minw,minh | `[0]`=1; `[1]`=`GR_O1` lastw *(signed)*, `[2]`=`GR_O2` lasth *(signed)* | — |
| 71 | `graf_dragbox`   | `[0..3]`=w,h,sx,sy; `[4..7]`=bound x/y/w/h (`GR_I5`) | `[0]`=1; `[1]`=lastx, `[2]`=lasty | — |
| 72 | `graf_movebox`   | `[0..3]`=w,h,sx,sy; `[4]`=dx, `[5]`=dy | `[0]`=1 | — |
| 73 | `graf_growbox`   | `[0..3]`=small x/y/w/h; `[4..7]`=big x/y/w/h (`GR_I5`) | `[0]`=1 | — |
| 74 | `graf_shrinkbox` | `[0..3]`=small; `[4..7]`=big (`GR_I5`) | `[0]`=1 | — |
| 75 | `graf_watchbox`  | `[1]`=obj, `[2]`=instate, `[3]`=outstate | `[0]`=**final in/out flag** | `[0]`=`GR_TREE` |
| 76 | `graf_slidebox`  | `[0]`=parent, `[1]`=obj, `[2]`=isvert | `[0]`=**0..1000** | `[0]`=`GR_TREE` |
| 77 | `graf_handle`    | — | `[0]`=**VDI handle** `gl_handle`; `[1]`=wchar, `[2]`=hchar, `[3]`=wbox, `[4]`=hbox | — |
| 78 | `graf_mouse`     | `[0]`=form# | `[0]`=1 | `[0]`=`GR_MADDR` (`MFORM*`) |
| 79 | `graf_mkstate`   | — | `[0]`=1; `[1]`=mx, `[2]`=my, `[3]`=button, `[4]`=kstate | — |

> `graf_rubberbox`/`dragbox`/`movebox`/`growbox`/`shrinkbox` are **`void`** —
> `int_out[0]` stays 1; the useful outputs are the geometry words in
> `int_out[1..4]` (for rubberbox/dragbox) or the in/out GRECTs the caller
> passed (movebox has none; grow/shrink animate only). `graf_growbox`/
> `graf_shrinkbox` take the **big** rect at `int_in[4..7]` (`GR_I5`), i.e. one
> WORD gap after the small rect at `int_in[0..3]`.

---

## `wind_create` (100)

**Source:** `gemsuper.c:330-332` → `gemwmlib.c:1369` (`wm_create`).
**Globals written:** `D.w_win[i]` (`w_setup`, `gemwmlib.c:205`), the CURR/PREV
rects set to `gl_rzero`, FULL set to the caller's max-extent rect.

Allocates the first free window slot (`w_flags & VF_INUSE == 0`) among
`NUM_WIN` windows (`NUM_WIN = 16` in the default `AES_VERSION > 0x0320` build;
8 otherwise, `sysconf.h:31-36`). Records `w_owner = rlr` (the caller's
*identity*; the pid `WF_OWNER` **reports** back is 0 for every application
window regardless — see [`application.md`](./application.md) §"AES process
identity and pid"), `w_kind = kind`,
empty name/info, sliders at top-left (`w_hslide = w_vslide = 0`), default
slider sizes (`w_hslsiz = w_vslsiz = -1`), and stores the caller's rect as the
window's **FULL** (max) extent. Returns the **handle** (0..NUM_WIN−1) or **−1**
if all slots are in use (`gemwmlib.c:1384`). The window is created *closed*
(no `VF_ISOPEN`); nothing is drawn.

Handle **0** (`DESKWH`) is reserved for the **desktop** root window, set up once
by `wm_init` (`gemwmlib.c:1290`); `wm_create` only ever returns 1..NUM_WIN−1.
Its `WS_WORK` is `gl_rfull` (`:1294`) — until a `wind_set(0, WF_TOP)` rewrites
it to `(1, 1, W−2, H−2)` (see `wind_set` § "`WF_TOP` on the desktop handle",
A80, 2026-09-20).

---

## `wind_open` (101)

**Source:** `gemsuper.c:333-335` → `gemwmlib.c:1410` (`wm_open`).
Helpers: `wm_update(BEG_UPDATE/END_UPDATE)` (screen lock), `w_obadd(W_TREE,
ROOT, handle)` (add to the on-screen window tree at the top), `draw_change`
(the redraw driver). **Globals written:** `w_flags |= VF_ISOPEN`, the W_TREE
sibling list, `gl_wtop`, and the **PREV rect = the caller's rectangle**:
after `draw_change(w_handle, &t)` (`gemwmlib.c:1426`), `wm_open` overwrites
`WS_PREV` once more with the rectangle it was given — `w_setsize(WS_PREV,
w_handle, pt)` (`gemwmlib.c:1427`, pinned commit `1a7484ab`; `w_setsize` is a
plain `rc_copy`, `:265-268`). Observable: immediately after `wind_open`,
`wind_get(WF_PXYWH)` returns the same rectangle as `WF_CXYWH` (measured
against the TT, round 1 case 253, A12).

Displays the created window at rect `pt`. Returns `FALSE` if the handle is
invalid or the window is already open (`gemwmlib.c:1416-1419`). Otherwise it
locks the screen, links the window as the new top of `W_TREE`, calls
`draw_change` to compute the new rectangle lists and repaint, and — as part of
that repaint — the owner receives the **first `WM_REDRAW`** for the newly
exposed work area (via `w_update`→`w_redraw`, `gemwmlib.c:887-907`). The window
becomes the top (active) window.

**The previous top is repainted deactivated, in full (A94, 2026-09-21).**
`draw_change`'s open/close block (`gemwmlib.c:1194-1209`): the opened window is
now `W_TREE[ROOT].ob_tail` and the old top no longer is (`:1194`); when the
old top is neither `NIL` nor the opened window (`:1203`) and the opened
window's `WS_PREV` — the `WS_CURR` it had before the open, `gl_rzero` for a
created (`wm_create`, `:1377`) or closed (`wm_close` → `draw_change(…,
&gl_rzero)`, `:1449,1073`) window — equals `gl_rzero` (`:1204-1206`),
`w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)` runs (`:1207`). That rebuilds the old
top's element tree with `istop = (gl_wtop == w_handle)` now false
(`w_bldactive`, `:495`: name `DISABLED` `:537`, the `w_bcolor` words
`:382-397`) and draws it through the old top's **own** rectangle list (rebuilt
at `:1088` with the new order) clipped to its `WS_TRUE` (`:363-379`,
`do_walk` `:286-312`). Contract: after `wind_open` of a second window,
**every** visible piece of the old top's frame shows the untopped decorations
— the pieces outside the new window's rectangle too, and also when the two
windows do not overlap at all (no overlap test is involved); where the new
window covers the old top nothing of it is drawn (an empty list when it is
fully covered). The `w_update(w_handle, &c, DESKWH, FALSE)` that follows
(`:1222`) visits the new window alone (`bottom == top`, `:1014-1043`): its
frame (`:1029`) and its `WM_REDRAW`; the new window's work area is left to
that message. Reference: `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/
shots/falcon-tc-two-windows.png` (the first window's band white after the
second opened at boot); NewTOS host test
`tos-api/tests/aes_window_open_oldtop.rs`, QEMU dumps
`measurements/measurement-2026-09-21-a94/screendumps/`.

### A preset `WS_CURR` — which branches of `draw_change` an open can take (A104, 2026-09-23)

The paragraph above is the branch an open reaches when the window's old
`WS_CURR` is `gl_rzero`, which is what `wm_create` (`:1377`) and `wm_close`
(`:1449,1073`) leave. One call puts a **non-zero** rectangle there without
opening the window: `wind_set(WF_CXYWH, r1)` on a created-but-unopened window
stores `WS_PREV := WS_CURR`, `WS_CURR := r1`, `WS_WORK := wm_calc(…)` and then
returns at the `VF_ISOPEN` test without drawing (`gemwmlib.c:1069-1086`). A
`wind_open(r2)` after that enters `draw_change` with `c = r1 != gl_rzero`, and
the branch tree at `:1104-1191` splits by how `r2` relates to `r1`:

| `r2` vs. `r1` | branch | EmuTOS |
|---|---|---|
| equal | top request, `:1110-1136` | `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)` (`:1118-1123`); when neither window is `VF_BROKEN`, `w_cpwalk(gl_wtop, …)` and `return` (`:1129-1135`) |
| same origin, other size | size request, `:1138-1154` | damage = union of both sizes + shadow; on a shrink `stop = w_handle`, `moved`, `start = DESKWH` |
| superset, other origin | grow/open, `:1160-1167` | `c = pt` |
| same size, other origin | move of top guy, `:1173-1177` | `w_move` **blits** the screen rectangle at `r1` to `r2`; `start = DESKWH`, `moved` |
| anything else | `:1184-1190` | damage = union of `r1` and `r2`, `start = DESKWH` |

**What NewTOS models.** The top-request branch's `w_cpwalk(oldtop, …)` is
implemented: it is the same full deactivated repaint of the previous top as
above, reached by the other gate, and the contract is identical (every visible
piece of the old top's frame, overlap or not). The grow/open branch is the one
already described — note that `:1203-1206`'s `WS_PREV == gl_rzero` test fails
for a preset rectangle, so in that branch the old top is **not** repainted in
full, in EmuTOS and here alike.

**What it does not model, and why that is observable only as extent.** The
damage rectangle stays the opened rectangle plus its drop shadow in the size,
shrink and move branches, where EmuTOS takes the union with `r1`. The union is
larger, but the extra ground is the rectangle the window was *assigned* and
never shown at, so repainting it writes back the pixels that are already
there: the resulting screen is identical. Host receipt:
`tos-api/tests/aes_window_open_oldtop.rs`
`a104_a_preset_the_window_never_showed_at_repaints_to_the_same_pixels`.

**Two named deviations in that same corner.**

1. *No `w_move` blit on an open.* In the same-size branch EmuTOS carries
   `WS_PREV + shadow` to the new place with a screen blit (`w_move`,
   `gemwmlib.c:946-970`) and then leaves the window out of both the frame
   repaint and the `WM_REDRAW` fan-out (`moved`, `:1025`). For a *move* those
   are the window's own pixels; for an **open** they are whatever stood where
   the window never was. NewTOS paints the window. Receipt:
   `a104_open_at_a_same_size_move_of_the_preset_paints_the_window_not_a_blit`.
2. *The open always posts its `WM_REDRAW`.* The top-request branch returns at
   `:1133-1135` when neither window is `VF_BROKEN`, i.e. without `w_update`, so
   an EmuTOS window opened this way is framed but never asked to draw its
   content. NewTOS posts the `WM_REDRAW` as in every other open.

Both deviations are confined to `wind_open` after `wind_set(WF_CXYWH)` on an
unopened window; no caller in the desktop, TeraDesk or QED uses that sequence.

---

## `wind_close` (102)

**Source:** `gemsuper.c:336-338` → `gemwmlib.c:1438` (`wm_close`).
Helpers: `ob_delete(gl_wtree, handle)` (unlink from W_TREE), `draw_change` with
a **zero rect** (drives the uncover-and-redraw of whatever was underneath).

Hides an open window. Returns `FALSE` if the handle is invalid or the window is
not open (`gemwmlib.c:1444-1447`). Sets the target size to `gl_rzero`, unlinks
it, and `draw_change` redraws the desktop and lower windows that were covered
(each gets its own `WM_REDRAW`). Clears `VF_ISOPEN` but **keeps** `VF_INUSE` —
the handle remains valid and can be reopened.

---

## `wind_delete` (103)

**Source:** `gemsuper.c:339-341` → `gemwmlib.c:1466` (`wm_delete`).

Frees the handle. Returns `FALSE` on an invalid handle. If the window is still
open, `wm_delete` closes it first (`gemwmlib.c:1473-1474`). It returns the
window's rectangle list to the free pool (`newrect`), resets its rects to
screen/full, and clears `w_flags = 0` and `w_owner = NULL`
(`gemwmlib.c:1476-1483`). After this the handle may be reused by `wind_create`.

---

## `wind_get` (104)

**Source:** `gemsuper.c:342-344` → `gemwmlib.c:1505` (`wm_get`).
**Reads:** `D.w_win[handle]`, the window's rect fields via `w_getxptr`/
`w_getsize` (`gemwmlib.c:231-262`), the rectangle list `w_rlist`/`w_rnext`, the
screen buffer via `gsx_mret`. Result rect → `int_out[1..4]`; scalars →
`poutwds[0]` (= `int_out[1]` for most, but the binding fills the caller's array).

| Field | Returns (in `int_out`) | Source |
|-------|------------------------|--------|
| `WF_WXYWH` (4) | **work-area** rect x/y/w/h | `w_getsize(WS_WORK)` `gemwmlib.c:1529` |
| `WF_CXYWH` (5) | **current border** rect | `w_getsize(WS_CURR)` `:1532` |
| `WF_PXYWH` (6) | **previous** rect | `w_getsize(WS_PREV)` `:1535` |
| `WF_FXYWH` (7) | **full-screen (max)** rect | `w_getsize(WS_FULL)` `:1538` |
| `WF_HSLIDE` (8) | `w_hslide` position (1..1000) | `:1542` |
| `WF_VSLIDE` (9) | `w_vslide` position (1..1000) | `:1544` |
| `WF_HSLSIZ` (15) | `w_hslsiz` (1..1000, −1=default) | `:1547` |
| `WF_VSLSIZ` (16) | `w_vslsiz` (1..1000, −1=default) | `:1550` |
| `WF_TOP` (10) | handle of the top window (`w_top()`) | `:1553` |
| `WF_FIRSTXYWH` (11) | **first** visible work rect (starts the walk) | `:1556` |
| `WF_NEXTXYWH` (12) | **next** visible work rect (continues) | `:1557` |
| `WF_SCREEN` (17) | menu/alert save-buffer address + length | `gsx_mret` `:1563` |
| `WF_COLOR` (18) | `int_out[2]`=topped, `int_out[3]`=untopped colour for gadget `int_in[2]` § (D-095) | `:1567` |
| `WF_DCOLOR` (19) | `int_out[2]`=default topped, `int_out[3]`=untopped colour § (D-095) | `:1572` |
| `WF_OWNER` (20) | `[0]`=owner pid, `[1]`=is-open, `[2]`=`gl_wtop`, `[3]`=bottom ¶ | `:1579` |

> **D-095 — `WF_COLOR`/`WF_DCOLOR` colours land one slot later than
> documented.** The handler writes `poutwds[1] = w_tcolor[gadget]; poutwds[2]
> = w_bcolor[gadget]` (`gemwmlib.c:1567-1571`, and analogously for
> `WF_DCOLOR` at `:1572-1576`) — it never touches `poutwds[0]`. Because the
> dispatcher calls `wm_get(WM_HANDLE, WM_WFIELD, &WM_OX, &WM_IX)` with
> `poutwds = &WM_OX = &int_out[1]` (`gemsuper.c:343`), `poutwds[1]` is
> `int_out[2]` and `poutwds[2]` is `int_out[3]` — **not** `int_out[1]`/
> `int_out[2]` as a previous spec revision stated. `int_out[1]` — and
> `int_out[4]` — are written by **nobody**: `wm_get` never touches
> `poutwds[0]` or `poutwds[3]` in these two arms. What the caller finds there
> is **not** its own value and **not** the gadget number: `xif` copies all
> `OUT_LEN` words back out of its local `int_out[O_SIZE]` unconditionally
> (`aes/gemsuper.c:432,444-445`), so within `OUT_LEN` the caller's array is
> overwritten in every word. On EmuTOS the two unwritten words are the
> **uninitialised dispatcher stack** (in practice the previous call's
> answer); on NewTOS they are **0** — the sanctioned deviation *"An AES
> handler that writes fewer than `OUT_LEN` words delivers 0"* in
> [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md), inventory in
> `docs/analysis/aes-underwriting-handlers-2026-09-19.md` (A27). The source's
> own doc comment (`gemwmlib.c:1494-1503`) describes `parm1` as the gadget
> *input* only; gemlib nevertheless copies `int_out[1]` back into `*W1` after
> the trap (`sdk/gemlib/gemlib-0.44.0/a_wind_get.c:366,372`), so the
> application's gadget variable is **clobbered** by the call — with stack on
> EmuTOS, with 0 on NewTOS. `w2` (`int_out[2]`) is the topped colour, `w3`
> (`int_out[3]`) the untopped colour; a conforming caller reads those two and
> the return code, nothing else. (Corrected 2026-09-19, A27: the previous
> sentence here — "`int_out[1]` reads back unchanged / the input gadget
> number" — was wrong for both machines.)

> **`WF_OWNER`'s pid is a reported value, computed from identity, not looked
> up from it (2026-09-20, A62/A56).** `wm_get` returns `pwin->w_owner->p_pid`
> verbatim (`gemwmlib.c:1580`), and in EmuTOS that literally *is* the value —
> `w_owner` points at the shared `rlr`, which is always pid 0 for any
> application window. A NewTOS whose `Window.owner` is keyed by GEMDOS slot
> (the identity, §"AES process identity and pid" in
> [`application.md`](./application.md)) must translate at this boundary: the
> stored owner is the slot, the value `WF_OWNER` hands back is 0 for every
> window owned by a chain member regardless of which slot actually owns it, and
> the accessory's own pid (2, 3, …, `application.md` "Pid assignment,
> accessories, and a failed load") for a window an accessory created
> (`w_setup(rlr, …)`, `gemwmlib.c:1377`; corrected 2026-10-17, A113).

`WF_TOP`, `WF_SCREEN`, `WF_DCOLOR` do **not** require a valid handle
(`gemwmlib.c:1517-1520`); every other field returns `FALSE` if the handle is
invalid. Any unlisted field → `default: return FALSE`.

> **`WF_SCREEN` (17) — the menu/alert screen buffer, address and length as
> two LONGs (A57, 2026-09-20).** `wm_get` calls `gsx_mret((LONG *)poutwds,
> (LONG *)(poutwds + 2))` (`gemwmlib.c:1564`) with `poutwds = &int_out[1]`
> (`gemsuper.c:343`); `gsx_mret` stores `gl_tmp.fd_addr` and `gl_mlen`
> (`aes/gemgsxif.c:125-129`). So the **address is a LONG over
> `int_out[1..2]`, the length a LONG over `int_out[3..4]`** — two native
> 32-bit stores at WORD-aligned addresses, not four independent WORDs —
> and `int_out[0]` is TRUE (`:1595-1598`). The buffer is the one
> `gsx_malloc` takes when the AES goes graphic (`sh_tographic`,
> `aes/gemshlib.c:236`; `gemgsxif.c:108-114`, `dos_alloc_anyram` =
> `Mxalloc(n, MX_PREFTTRAM)`) and that `fm_alert` and `menu_sr` save the
> screen into (`gemfmalt.c:363,390`, `gemmnlib.c:208-223`); its length on
> EmuTOS is `MENU_BUFFER_SIZE * gl_wchar * gl_hchar * gl_nplanes / 8`
> (`:98-104`; 40 960 on the reference TT, four planes).
>
> **NewTOS** (`tos-api/src/aes/gsxif.rs`, `window.rs` `WF_SCREEN` arm):
> the same buffer, one kernel-owned block of the BDOS pool taken by the
> same `Mxalloc(…, MX_PREFTTRAM)` from `sh_tographic`, reported with the
> same two native LONG stores (`Aespb::set_intout_long`). Two things are
> NewTOS' own, both registered in `docs/COMPATIBILITY.md`:
> *the length* is one screen in device pixels (`bb_save_words(W, H) * 4`:
> 614 400 at 640×480, 4 147 200 at 1920×1080), larger than EmuTOS'
> formula gives, never smaller — a program that takes it as an upper bound
> for its own saves (TeraDesk `sdk/apps/teradesk/resource.c:307-320`) gets more room; and
> *the address* depends on who asks: an **EL0 process** receives the block
> published into its own address space, read-write (FreeMiNT `PROT_G`,
> ADR-023), lazily on its first `WF_SCREEN` and the same user VA on every
> later call; an **EL1 caller** (the in-kernel desktop) receives the
> physical/pool address (ADR-021). **Without a buffer** — the pool could
> not supply it at boot — the field answers FALSE and writes nothing (the
> A54 answer, kept as the fallback; EmuTOS would answer TRUE with address
> 0). Pop-ups and nested submenus do **not** use this buffer: `popup_blit`
> takes a fresh block per opening (`gemmnext.c:136-215`, [`menu.md`](./menu.md)).

### The rectangle-list iteration (`WF_FIRSTXYWH`/`WF_NEXTXYWH`) — the redraw contract

This is the heart of the window-redraw protocol and the most bug-prone part.

- `wind_get(handle, WF_FIRSTXYWH, …)` starts the walk at the window's owner
  rectangle list `pwin->w_rlist`; `WF_NEXTXYWH` continues from the saved cursor
  `pwin->w_rnext` (`gemwmlib.c:1559`).
- `w_owns` (`gemwmlib.c:1230-1241`) walks the `ORECT` list, **intersecting each
  owner rectangle with the window's work area** and skipping rects that clip to
  nothing; it advances `w_rnext` and returns the **first non-empty** clipped
  rect. When the list is exhausted it writes **only two words**: `g_w = g_h =
  0` (`gemwmlib.c:1240`, the sole statement after the loop). `g_x`/`g_y` are
  **not** reset there — they keep whatever `*poutwds` already held: either the
  last rect `rc_intersect` wrote before failing (`util/rectfunc.c:72-75` write
  all four words even on a `FALSE` return) if the loop ran at least once, or —
  if the owner list was already empty on entry — the content of the
  **dispatcher's local `int_out`**, *not* the caller's buffer: `poutwds` is
  `&int_out[1]` of `xif`'s uninitialised stack array (`aes/gemsuper.c:343,432`),
  and `xif` copies all `OUT_LEN` words of it back unconditionally
  (`:444-445`), so the caller's own `g_x`/`g_y` are overwritten either way. On
  EmuTOS that is stack (measured on the TT: `31, 63` and `0, 19` in one run,
  `TASK/A15-STATUS-UNFINISHED.MD`, `tosdump/cases/ERRATA.md`); on NewTOS it is
  0 (`docs/COMPATIBILITY.md`, the A27 entry). A conforming redraw handler must
  test `w==0 && h==0` only — `g_x`/`g_y` on the terminating rect have no
  defined value to read. (Corrected 2026-09-19, A27: the A18 wording "the
  caller's buffer content unchanged" was the reading the poison measurement
  had already refuted.)
- The application's redraw idiom: `wind_get(h, WF_FIRSTXYWH)`, then repeatedly
  intersect its own dirty rect with the returned piece, draw, and
  `wind_get(h, WF_NEXTXYWH)` **until `w==0 && h==0`**. Each returned rect is a
  **piece of the window's work area not covered by any higher window**, so
  drawing clipped to each piece never paints over an overlapping window.
- The owner rectangle list itself is built by `newrect`/`mkrect`/`brkrct`
  (`gemwrect.c:64-205`): when window geometry changes, each window's rect is
  broken against every higher window's true rect into up to four
  (TOP/LEFT/RIGHT/BOTTOM) non-overlapping pieces (`mkpiece`, `gemwrect.c:64-98`).
  **This piece order is fixed and observable — it is not free for NewTOS to
  choose.** `TOP=0, LEFT=1, RIGHT=2, BOTTOM=3` (`gemwrect.c:30-33`), and
  `brkrct`'s loop appends a piece to the list only when its `have_piece[]` flag
  is true, walking `i = 0..3` in exactly that order and skipping any index
  whose flag is false (`for (i = 0; i < 4; i++) if (have_piece[i]) p =
  (p->o_link = mkpiece(i, new, r));`, `gemwrect.c:118-122`); the flags
  themselves are computed at `gemwrect.c:113-116`. Consequently the list's
  **head** — the rect `WF_FIRSTXYWH` returns — is whichever of TOP, LEFT,
  RIGHT, BOTTOM the break actually produced first, **not always the TOP
  piece**: when a covering window's true rect reaches all the way to the
  broken window's top edge, `have_piece[TOP]` (`gemwrect.c:113`) is false, the
  TOP node is never created, and the list opens with LEFT, RIGHT, or BOTTOM
  instead. A window that got broken is flagged `VF_BROKEN` (can't be
  block-moved). The list therefore always describes the **visible,
  non-overlapping** portions, in that fixed piece order.
  `w_getxptr`/`w_getsize` (`gemwmlib.c:231-260`) show that the cutting rect for
  a break is the window's **`WS_TRUE`** size — its `WS_CURR` border rect with
  `DROP_SHADOW_SIZE` (2 px, `gemwmlib.c:49`) added to width and height — while
  the final clip in `w_owns` is against `WS_WORK` (the inset work rect); the
  drop shadow therefore participates in *cutting other windows' lists* but is
  clipped away again in what a redraw handler actually receives.
- **Pool size and exhaustion (A21, 2026-09-19).** The pieces of *all* lists
  are `ORECT` cells from one pool of `NUM_ORECT = NUM_WIN * 10`
  (`gemlib.h:88`, with the source's own "is this enough???") — **160** at
  `NUM_WIN = 16` (`sysconf.h:32`, AES ≥ 0x0330), 80 at 8. `or_start`
  threads them onto the free list once in `wm_init` (`gemwrect.c:40-50`,
  `gemwmlib.c:1258`); `newrect`/`brkrct` return cells as they rebuild
  (`gemwrect.c:174-181,124-127`). **EmuTOS has no exhaustion path:**
  `get_orect` returns `NULL` from an empty free list (`gemwrect.c:53-61`)
  and every caller dereferences it unchecked — `mkpiece` `:68-75`, `newrect`
  `:201-203`, `wm_init` `gemwmlib.c:1287-1289` — writing the link and four
  rectangle words to address 0 and cutting the list short at that piece
  (`:121`, `:125`). Undefined behaviour, not contract; and which
  `draw_change` runs dry depends on the old lists the sweep has not yet freed
  (`gemwmlib.c:1089`), so the exact point is not a function of the window
  arrangement. **NewTOS:** the capacity is `NUM_ORECT` *per list*
  (`tos-api/src/aes/window.rs`, `RCAP`) — every arrangement that fits the
  EmuTOS pool fits — and a list that outgrows it stops the call with a named
  panic rather than dropping pieces silently (the earlier per-list cap of 16
  did exactly that, A21). Test: `tos-api/tests/aes_window_rectlist_capacity.rs`.

> **Correction, 2026-09-12 — the "any way it likes" freedom below did not
> exist and has been struck.** Until this date this section read:
>
> > **Contract:** the *sequence of visible pieces* a redraw handler receives —
> > non-overlapping, clipped to the work area, terminated by an empty rect. The
> > `ORECT` free-list, the four-way split, and `VF_BROKEN` are **reference**;
> > NewTOS **may compute the visible-region decomposition any way it likes as
> > long as** `WF_FIRSTXYWH`/`WF_NEXTXYWH` **yield the same set of pieces**.
>
> A measurement against the pinned EmuTOS build
> (`docs/analysis/aes-r3-wind-state-einordnung-2026-09-12.md`, Befund C;
> `tosdump/cases/aes-r3-wind-state.tsv` cases 439–588) falsifies the premise
> that a "same set of pieces" independent of subtraction order exists at all:
> for **the same three window rects**, opened in a different order, EmuTOS
> itself returns **four** pieces in one ordering and **five** in the other
> (worked out from source below, and confirmed digit-for-digit against the
> EmuTOS dump). There is no order-independent decomposition to converge on —
> the piece count and boundaries are a deterministic function of the order in
> which covering windows are subtracted, and that order is exactly the thing
> the struck sentence declared free. This paragraph was the license under
> which a real defect was built (NewTOS subtracts covering windows in handle/
> slot order instead of ascending z-order — Befund C, `tos-api/src/aes/
> window.rs:788-792`); the code is out of scope for this correction, only the
> spec is fixed here.

#### The subtraction order is normative — ascending z, not handle order

`draw_change` rebuilds **every** window's rectangle list on any geometry
change with one traversal:

```c
everyobj(gl_wtree, ROOT, NIL, (EVERYOBJ_CALLBACK)newrect, 0, 0, MAX_DEPTH);
```
— `gemwmlib.c:1089`, followed by `gl_wtop = W_TREE[ROOT].ob_tail;` at `:1093`.

`everyobj` (`gemobjop.c:76-131`) walks the `W_TREE` sibling chain from
`ob_head` to `ob_tail` (the `child`/`sibling` traversal, abort conditions at
`gemobjop.c:93` and `:121`). A window's position in that chain **is** its
z-order: `w_obadd` (`gemwmlib.c:186-201`) — called from `wm_open`
(`gemwmlib.c:1425`) whenever a window is opened — always appends the new
child at `ob_tail`, and `wm_mktop` (`gemwmlib.c:1605-1610`) moves a topped
window to `ob_tail` via `ob_order(gl_wtree, w_handle, NIL)` (`NIL` = append
after the current tail, `gemoblib.c:1201-1240`, esp. the `new_pos == NIL`
branch). So **`ob_head` → `ob_tail` is bottom-to-top**, and `gl_wtop` (the
topmost window) is always the tail.

For each window `wh` visited in that ascending-z sweep, `newrect(tree, wh)`
(`gemwrect.c:165-205`) does two things in order:

1. Discards `wh`'s own current rectangle list and rebuilds it as **one
   unbroken node** covering `wh`'s own `WS_TRUE` rect (`:172-195`, `:200-204`)
   — at this point `wh` is treated as if nothing covers it yet.
2. Calls `everyobj(tree, ROOT, wh, mkrect, …)` (`:198`) — traverses the
   **same** `ob_head`→`ob_tail` chain, calling `mkrect(tree, ow)`
   (`gemwrect.c:136-162`) for every window `ow` visited **before** reaching
   `wh` (the `this == last` / `tmp1 == last` aborts in `everyobj` stop the
   walk exactly at `wh`, never processing `wh` itself). Because the chain is
   ascending z, "visited before `wh`" means **every window with lower z than
   `wh`** — i.e. every window `wh` could be covering. `mkrect` then calls
   `brkrct(new=wh's rect, r, p)` (`gemwrect.c:101-132`) against `ow`'s
   *existing* rectangle list, replacing each surviving piece `r` that
   overlaps `wh`'s rect **in place** with up to four (TOP/LEFT/RIGHT/BOTTOM,
   `mkpiece`, `:64-98`) sub-pieces, in that fixed order.

Net effect, read across the whole `draw_change` sweep: window `W`'s final
owner-rectangle list is `W`'s own true rect, cut **once per window above it,
one window at a time, in ascending z order of those covering windows** —
never in handle order, slot-array order, or any other order. This is exactly
what `w_obadd`/`ob_order` guarantee by always appending at the tail: the
processing order **is** the z order by construction.

**Why the order changes both the count and the boundaries, not just which
piece is called "first".** `brkrct` replaces a *surviving* piece with its
sub-pieces at the moment it is cut; a piece already removed by an earlier cut
is never revisited by a later one, and a piece that survives a cut is
individually re-split by every subsequent one. Cutting with window `A` first
and then `B` therefore does not, in general, produce the same fragments as
cutting with `B` first and then `A` — even though both orders remove exactly
the union `A ∪ B` from `W`'s area. The worked example below is the proof: the
same three rects, opened in a different order, yield four pieces in one case
and five in the other.

> **Contract (corrected).** The window's owner-rectangle list is **not** free
> to compute "any way that yields the same set of pieces" — no such
> order-independent set exists. What is contract is the **algorithm's
> observable result**: for a given window and a given set of open, higher
> windows in a given z-order, `WF_FIRSTXYWH`/`WF_NEXTXYWH` must walk the
> **exact ordered sequence of non-overlapping, work-clipped pieces** that
> results from subtracting each covering window's true rect **one at a time,
> in ascending z-order**, terminated by an empty (`w=h=0`) rect. NewTOS is
> free to reach that result with any internal data structure or algorithm
> (a `Vec`-backed region list, a rectangle-difference routine, etc. — the
> `ORECT` free-list allocator `or_start`/`get_orect`, `gemwrect.c:40-61`, and
> the `VF_BROKEN` block-move-optimisation flag remain **reference**, being
> unobservable through any `wind_get`/`wind_set` field) — but the **subtraction
> order (ascending z, never handle/slot order) and the resulting piece
> boundaries/count are contract**, because they are what a conforming redraw
> handler actually walks.

#### Worked example — the two orderings of the same three windows (measured)

From `tosdump/cases/aes-r3-wind-state.tsv` §D (cases 404–588), re-derived by
hand from the source above and confirmed **digit-for-digit** against the
EmuTOS reference dump. All three windows are created with `wind_create(kind =
0x0FEF, …)` — every gadget bit except `INFO` (`0x0FEF` has no `0x0010` bit) —
then opened at:

| Window | Border rect (`x,y,w,h`) | `WS_TRUE` rect (+`DROP_SHADOW_SIZE`=2) |
|---|---|---|
| 1 | `30,40,320,240` | `30,40,322,242` |
| 2 | `60,80,200,160` | `60,80,202,162` |
| 3 | `180,160,200,160` | `180,160,202,162` |

Window 1's `WS_WORK` rect is `31,63,296,194` (the border thicknesses this
implies — `lb=1, tb=rb=bb=23` — are **backed out algebraically** from matching
the measured pieces below, consistent with the `wind_calc` border formulas
above for `CONF_WITH_3D_OBJECTS`, but not independently re-derived from
`gl_hbox`/`gl_wbox`'s system-font pixel values in this session; the
**border-thickness formulas** are the ones cited under `wind_calc` (108)
above and are not affected by this correction).

**Group I — opened 1, 2, 3** (ascending z: 1, 2, 3 — window 1 is broken by 2,
then by 3):

Breaking window 1's true rect by window 2's true rect (all four
`have_piece[]` true, `gemwrect.c:113-116`) gives, in `mkpiece` order
TOP/LEFT/RIGHT/BOTTOM: `30,40,322,40` · `30,80,30,162` · `262,80,90,162` ·
`30,242,322,40`. Breaking that list again by window 3's true rect: the TOP
and LEFT pieces don't overlap window 3 and survive unchanged; RIGHT
(`262,80,90,162`) is replaced by its TOP-only sub-piece `262,80,90,80` (window
3 extends past `RIGHT`'s right and bottom edges, so only `have_piece[TOP]` is
true); BOTTOM (`30,242,322,40`) is replaced by its LEFT-only sub-piece
`30,242,150,40` (window 3's left edge, `180`, is right of `BOTTOM`'s left
edge, `30`, so `have_piece[LEFT]` is true; window 3 extends past `BOTTOM` on
the other three sides, so `TOP`/`RIGHT`/`BOTTOM` are false). Clipping the
resulting four-node list to `WS_WORK` (`31,63,296,194`) in `w_owns`:

| Piece | Raw (true-rect space) | Clipped to `WS_WORK` (what the app sees) | Case |
|---|---|---|---|
| 1 | `30,40,322,40` | **`31,63,296,17`** | 439 |
| 2 | `30,80,30,162` | **`31,80,29,162`** | 448 |
| 3 | `262,80,90,80` | **`262,80,65,80`** | 458 |
| 4 | `30,242,150,40` | **`31,242,149,15`** | 469 |
| — | *(list ends)* | `w=h=0` | 481 |

**Four** pieces. Matches the measured EmuTOS dump exactly.

**Group II — opened 1, 3, 2** (ascending z: 1, 3, 2 — window 1 is broken by
3, then by 2):

Breaking window 1's true rect by window 3's true rect first: `have_piece[TOP]`
and `have_piece[LEFT]` are true, `RIGHT`/`BOTTOM` false (window 3 extends past
window 1's true rect on the right and bottom) → list = `30,40,322,120` (TOP),
`30,160,150,122` (LEFT). Breaking **both** of those by window 2's true rect:
the TOP piece (`30,40,322,120`) overlaps and splits into **three**
(TOP/LEFT/RIGHT, `BOTTOM` false): `30,40,322,40` · `30,80,30,80` ·
`262,80,90,80`; the LEFT piece (`30,160,150,122`) overlaps and splits into
**two** (LEFT/BOTTOM, `TOP`/`RIGHT` false): `30,160,30,82` · `30,242,150,40`.
Concatenated, the five-node raw list, clipped to the same `WS_WORK`:

| Piece | Raw (true-rect space) | Clipped to `WS_WORK` (what the app sees) | Case |
|---|---|---|---|
| 1 | `30,40,322,40` | **`31,63,296,17`** | 519 |
| 2 | `30,80,30,80` | **`31,80,29,80`** | **528** |
| 3 | `262,80,90,80` | **`262,80,65,80`** | 538 |
| 4 | `30,160,30,82` | **`31,160,29,82`** | **549** |
| 5 | `30,242,150,40` | **`31,242,149,15`** | **561** |
| — | *(list ends)* | `w=h=0` | 574 |

**Five** pieces. Matches the measured EmuTOS dump exactly, including the
bolded cases (528/549/561) where NewTOS's handle-order implementation
diverges: NewTOS returns Group I's four-piece list for **both** openings,
because it does not know which covering window is "higher" — it iterates its
window-slot array in slot order, not ascending z.

Both groups remove the identical union of area (window 2 ∪ window 3, clipped
to window 1) from window 1 — the *visible area* is identical — but the
**piece count differs (4 vs. 5)** and two of the surviving pieces have
different heights (`…,29,162` in Group I where Group II has `…,29,80` **and**
a fifth piece `…,29,82` that Group I has no equivalent for at all). This is
the concrete evidence that no order-independent "same set of pieces" exists.

---

## `wind_set` (105)

**Source:** `gemsuper.c:345-347` → `gemwmlib.c:1623` (`wm_set`).
Helpers: `wm_update(BEG_UPDATE/END_UPDATE)`, `draw_change` (WF_CXYWH),
`wm_mktop` (WF_TOP), `w_cpwalk` (redraw one gadget of the **target window
alone**, clipped to that window's own rectangle list — corrected 2026-09-20,
A82; not "all/topped windows", see "Redraw scope" below).
**Globals written:** the `WINDOW` field being set; for WF_NEWDESK
`gl_newdesk`/`gl_newroot`.

> **D-094 — `WF_NAME`/`WF_INFO`/`WF_NEWDESK` do *not* use `addr_in[0]`.** A
> previous spec revision (and the `WM_IOTITLE = addr_in[0]` macro at
> `aesdefs.h:502`) claimed the string/tree pointer for these three fields
> arrives via `addr_in[0]`. Tracing the actual dispatch
> (`gemsuper.c:345-347`: `wm_set(WM_HANDLE, WM_WFIELD, &WM_IX)`) shows `wm_set`
> is called with only **three** arguments — handle, field, and `pinwds =
> &int_in[2]` — no `addr_in` array is passed at all (`gemwmlib.c:1623`:
> `BOOL wm_set(WORD w_handle, WORD w_field, WORD *pinwds)`). Inside, `WF_NAME`/
> `WF_INFO` do `*(char **)pinwds` and `WF_NEWDESK` does `*(OBJECT **)pinwds`
> plus `pinwds[2]` for the root index (`gemwmlib.c:1660-1669,1685-1689`) — i.e.
> the 32-bit pointer is packed directly into `int_in[2]` (high word) and
> `int_in[3]` (low word), exactly like a rect's x/y occupy two `int_in` words,
> and — since `pinwds = &int_in[2]` — the root index `pinwds[2]` is
> `int_in[4]`, the word right after the packed pointer (see A81 below for how
> this generalises to an 8-byte LP64 pointer).
> `WM_IOTITLE` is dead code: grepping the whole EmuTOS tree finds no other
> reference to it. **Contract impact:** a caller must pack the pointer into
> `int_in[2..3]`, not populate `addr_in[0]` — the opposite of what the
> previous spec (and the unused macro) implied.

| Field | Effect | Triggers redraw of |
|-------|--------|--------------------|
| `WF_NAME` (2) | `w_pname = *(char**)pinwds` — pointer packed in `int_in[2..3]`, **not** `addr_in[0]` (D-094) | `W_NAME` gadget of **this one window**, if it is open (`w_cpwalk(w_handle,…)`, `:1665,1754-1755`; corrected 2026-09-20, A82 — see "Redraw scope" below) |
| `WF_INFO` (3) | `w_pinfo = *(char**)pinwds` — pointer packed in `int_in[2..3]`, **not** `addr_in[0]` (D-094) | `W_INFO` gadget of **this one window**, if it is open (`:1673,1754-1755`; A82) |
| `WF_CXYWH` (5) | move/resize → `draw_change(handle, rect)` | **whole frame + uncovered areas** (`:1677`); a pure move of the top window is **blitted** and the mover gets no `WM_REDRAW` — see the `draw_change` note below |
| `WF_TOP` (10) | `if (w_handle != gl_wtop) wm_mktop(w_handle)` (`:1679-1683`) — compared against the **raw** `gl_wtop` (`NIL` while nothing is open), so handle 0 always qualifies; `wm_mktop` = `ob_order(…, NIL)` + `draw_change(h, WS_CURR)` with `WS_PREV` restored afterwards (`:1605-1613`) | full topping redraw for an **open** window (`:1682`); **no pixel and no message** for the desktop handle 0 or for a created-but-unopened handle — see "`WF_TOP` on the desktop handle" below (A80, 2026-09-20) |
| `WF_NEWDESK` (14) | set desktop-background tree `gl_newdesk`, root `gl_newroot` — root at `pinwds[2]` = `int_in[4]` (EmuTOS packed convention, D-094); NewTOS's native/packed dual mapping is A81, "Edge cases" item 16 | (next desktop redraw) `:1685` |
| `WF_HSLSIZ` (15) | `w_hslsiz` (clamped 1..1000, or −1) — only if changed | `W_HSLIDE` gadget `:1690` |
| `WF_VSLSIZ` (16) | `w_vslsiz` (clamped, or −1) — only if changed | `W_VSLIDE` gadget `:1697` |
| `WF_HSLIDE` (8) | `w_hslide` position (clamped 1..1000) — only if changed | `W_HSLIDE` gadget `:1704` |
| `WF_VSLIDE` (9) | `w_vslide` position (clamped 1..1000) — only if changed | `W_VSLIDE` gadget `:1711` |
| `WF_COLOR` (18) § | per-window `w_tcolor`/`w_bcolor[gadget]` (−1 skips) | that gadget of **this one window** (`w_cpwalk`, `:1719,1754-1755`; corrected 2026-09-20, A82) |
| `WF_DCOLOR` (19) § | default `gl_wtcolor`/`gl_wbcolor[gadget]` (−1 skips) | **no** redraw (`gadget=-1`) `:1727` |

Any unlisted field → `ret = FALSE` (`gemwmlib.c:1736`).

> **Redraw scope (contract detail, corrected 2026-09-20, A82 — this entry
> previously claimed `WF_NAME`/`WF_INFO`/`WF_COLOR` redraw "all open
> windows"; they do not).** `WF_NAME`/`WF_INFO`/`WF_COLOR` set `do_cpwalk =
> TRUE` and reach `w_cpwalk(w_handle, gadget, MAX_DEPTH, TRUE)`
> (`gemwmlib.c:1754-1755`), which draws through `do_walk(wh, …)` over
> `D.w_win[wh].w_rlist` alone (`:286-312,363-379`) — **the owner rectangle
> list of `w_handle`, the one window the call named**, and only if that
> window is open (`if (pwin->w_flags & VF_ISOPEN)`, `:1662,1670`; an
> unopened window's pointer/colour is stored but nothing is drawn). EmuTOS's
> own comment on the `WF_NAME` arm — "update name applies to all open
> windows" (`:1665`, likewise `:1673` for `WF_INFO`) — reads as "every open
> window", but the code it comments contradicts that reading: in context it
> draws the contrast with the *closed*-window case immediately above it
> (pointer stored, no redraw at all) and with the slider arms below (redraw
> additionally gated on **topped**, not merely on open) — not "one window"
> vs. "every window".
> Slider fields (`WF_HSLIDE`/`VSLIDE`/`HSLSIZ`/`VSLSIZ`) redraw **only the
> `W_HSLIDE`/`W_VSLIDE` object subtree of that same one window** — the
> slider track plus its elevator/thumb child (`W_HELEV`/`W_VELEV`,
> `w_adjust(W_HSLIDE, W_HELEV,…)`/`w_adjust(W_VSLIDE, W_VELEV,…)`, 3D variant
> `:430-436,468-474`, non-3D variant `:663-669,704-710`), **not** the arrow
> gadgets: `W_LFARROW`/`W_RTARROW`/`W_UPARROW`/`W_DNARROW` are siblings of
> `W_HSLIDE`/`W_VSLIDE` under `W_HBAR`/`W_VBAR`, not descendants, and
> `ob_draw`'s `everyobj(tree, obj, tree[obj].ob_next, …)`
> (`gemoblib.c:959-977`) walks exactly `obj`'s own subtree. In the non-3D
> build this redraw additionally requires the window to be topped
> (`if (w_handle == gl_wtop)`, `gemwmlib.c:1747`); with `CONF_WITH_3D_OBJECTS`
> (default) sliders are always visible so they redraw regardless of topped
> state, but still scoped to that one window's subtree. `WF_CXYWH` and
> `WF_TOP` go through `draw_change`, which is the one path that legitimately
> repaints **multiple** windows — the frame **and** every uncovered area
> (whole-tree redraw with per-window `WM_REDRAW`). The slider-position/size
> setters **early-out with no redraw if the value is unchanged**
> (`if (pwin->w_hslide != pinwds[0])`) — NewTOS reproduces that guard
> (A84, `TASK/A84.md`, 2026-09-20) and the arrow exclusion (A98,
> `TASK/A98.md`, 2026-09-21: `redraw_?slider` draws the `W_?SLIDE` subtree
> alone; unobservable in pixels, only in the per-pixel write mask — see
> edge case 5 below).

> **`WF_NAME`/`WF_INFO` store a pointer, not a copy; no length limit
> (contract, A83, 2026-09-20).** EmuTOS's `wm_set` stores exactly the
> pointer the caller passed — `w_pname = *(char **)pinwds` (`WF_NAME`,
> `gemwmlib.c:1661`), `w_pinfo = *(char **)pinwds` (`WF_INFO`, `:1669`) — no
> string is copied anywhere in this arm. `w_bldactive` reads that pointer
> straight into `gl_aname.te_ptext`/`gl_ainfo.te_ptext` on **every** frame
> build — both the `CONF_WITH_3D_OBJECTS` variant (our reference profile,
> `:501-502`) and the non-3D variant (`:737-738`) — reached from every
> redraw path that calls `w_cpwalk` (`:377`): `wm_set`'s own `do_cpwalk`
> above, `draw_change`'s frame repaint (`w_update`→`w_cpwalk`, `:1029`), and
> the Control Manager's gadget hit test (`gemctrl.c:187`). Consequently:
> - **The application may change its title buffer *in place* after the
>   `wind_set` call**; the next frame redraw (top, uncover, move, or any
>   other trigger that calls `w_bldactive`) paints the buffer's *current*
>   contents, not a snapshot taken at `wind_set` time.
> - **There is no length limit in the AES.** `w_bldactive` hands the raw
>   pointer to the `TEDINFO` the object drawer paints; anything that does
>   not fit the `W_NAME`/`W_INFO` gadget rect is clipped by the object
>   drawer's own text clip, exactly like any other `TEDINFO` string — the
>   AES itself never truncates or bounds-checks the string.
> - **A NULL pointer is undefined in EmuTOS**: `*(char **)pinwds` may
>   legally be NULL, and `w_bldactive` passes it straight to the
>   text-drawing code with no NULL check.
> - **Lifetime is the application's responsibility, as in EmuTOS**: the
>   pointer must stay valid for as long as the window exists (until the
>   next `WF_NAME`/`WF_INFO` call, `wind_delete`, or process exit) — the AES
>   keeps only the pointer, never a copy.
>
> **Decision (Lead, 2026-09-20, A83): NewTOS matches this contract** — store
> the pointer, read it live at every redraw, treat a NULL pointer as an
> empty string (EmuTOS leaves NULL undefined; NewTOS resolves the ambiguity
> to `""` rather than reproducing EmuTOS's unspecified behaviour).
> **Implemented 2026-09-20 (A83)** — `tos-api/src/aes/window.rs` `LiveStr`:
> `wind_set(WF_NAME)`/`wind_set(WF_INFO)` store the caller's pointer together
> with the caller's process slot; `draw_frame` reads the string at every
> frame build through the owner's mapping (`usermem::resolve_kva`, page by
> page, so the walk ends at the owner's block end), a kernel pointer
> directly; NULL or an unresolvable address renders as `""`. The walk is
> bounded by the gadget's width in glyphs plus a 512-byte ceiling — a walk
> bound, not a contract limit. The earlier 80-byte copy (`NAME_CAP`,
> `copy_cstr`) is gone. The kernel desktop keeps its titles in the `WNode`
> (`programs/desktop/src/win.rs` `w_name[116]`/`w_info[72]`, as EmuTOS
> `desk/deskwin.h:73,79`).

> **`WF_NEWDESK` stores a pointer read through the registrant's mapping
> (NewTOS rule, A97, 2026-09-21).** `wm_set` stores the pointer —
> `gl_newdesk = *(OBJECT **)pinwds; gl_newroot = pinwds[2]`
> (`gemwmlib.c:1685-1689`) — and `w_drawdesk` draws that tree at
> `MAX_DEPTH` on every background repaint (`:340-359`), which the AES runs
> from **any** process's `wind_open`/`wind_close`/move and from the
> desktop's own redraw. On EmuTOS's flat machine that is one address space;
> on NewTOS every EL0 process has its own `TTBR0` with the TPAs at the same
> low VAs (`kernel/src/arch/aarch64/el0.rs`, `bdos/process/mmu.rs`), so a
> raw dereference at composite time goes through whichever tables are
> installed — a `Pexec` child compositing its parent's background, the
> kernel desktop repainting after a child's `Pterm`. Contract: the tree, its
> `ob_spec` sub-structs (TEDINFO/ICONBLK/CICONBLK/BITBLK/USERBLK), their
> strings and rasters are read **through the registering process's
> mapping** — `wind_set(WF_NEWDESK)` records the caller's process slot next
> to the pointer (`NEWDESK_OWNER`, as `WF_NAME`/`WF_INFO` do, A83), and the
> composite resolves every address through `usermem::resolve_kva(slot, va)`
> (`object.rs` `ObjTree::app_owned`; rasters and strings page by page, a
> span only through one contiguous alias). A tree whose root record the
> owner no longer maps counts as **not registered** (the AES fallback root
> is drawn); an unmapped sub-struct, string or raster inside the tree is
> drawn as nothing. A `G_USERDEF` in such a tree is called out to in the
> **current** process, whoever registered the tree — EmuTOS's
> `call_usercode` is a plain `jsr` on the current stack
> (`gemoblib.c:411-426`, from `ob_user` `:433-448`). Since ARCH03 stage 1 a
> user address means the same in every process (VA = PA), so the owner's
> `ub_code` is reachable; with memory protection on the current process
> holds the owner's rights for the duration of the callout and loses them
> afterwards (ARCH03 3c, E10 (a); FreeMiNT `prot_temp`,
> `sys/arch/mprot030.c:775-805`). Until 3c the callout was skipped when the
> owner was not the current process (A97). A kernel (EL1) registrant's tree
> is a kernel VA and is read directly. Host-tested at the resolver seam
> (`window.rs` `newdesk_owner_tests`, the A83 method).

> **`WF_TOP` on the desktop handle and on a created-but-unopened handle
> (A80, 2026-09-20).** `wm_set` runs `wm_mktop(w_handle)` whenever `w_handle !=
> gl_wtop` (`gemwmlib.c:1679-1683`), and `gl_wtop` is never 0 (`NIL` or an open
> window's handle, `:1093,1297`), so **`wind_set(0, WF_TOP)` always reaches
> `wm_mktop(0)`** (`:1605-1613`). There, `ob_order(gl_wtree, 0, NIL)` refuses
> `ROOT` (`gemoblib.c:1209-1210`) — the z-order is untouched; `draw_change(0,
> WS_CURR)` stores the geometry — `WS_PREV := WS_CURR`, `WS_CURR := pt`
> (unchanged, it *is* `WS_CURR`), `WS_WORK := wm_calc(WC_WORK, w_kind, pt)`
> (`:1069-1077`) — and returns **before touching the window tree or the screen**
> because the desktop window carries `VF_INUSE` only, never `VF_ISOPEN`
> (`w_setup`, `:205-213,1290`; the test `:1085-1086`); `wm_mktop` then restores
> `WS_PREV` (`:1613`). Net effect: **0 pixels, no message, `gl_wtop` unchanged,
> and `WS_WORK(0)` becomes `wm_calc(WC_WORK, NONE, WS_CURR(0))`** — with
> `w_kind = NONE` every border is 1 px (`:1807-1866`), so for the screen-sized
> desktop window (`WS_CURR(0) = gl_rscreen`, `:1291`) `wind_get(0, WF_WXYWH)`
> answers `(1, 1, W−2, H−2)` from then on instead of the `gl_rfull` `wm_init`
> stored (`:1294`) — it now *includes* the menu row and *excludes* a 1-px
> margin. The same stored rectangle is the clip of the desktop's own
> `WF_FIRSTXYWH`/`WF_NEXTXYWH` walk (`w_owns`, `:1558`) and, while no window
> is open, the Control Manager's ctrl rect (`w_setactive`, `:326-328`). The
> value stays until `wm_init` rewrites the desktop window (`:1294`); `wm_new`
> skips handle 0 (`:1886`). Derived from the source, not measured (A59 T5).
>
> A **created-but-unopened handle ≥ 1** never gets that far in EmuTOS:
> `ob_order` → `get_par` walks `W_TREE[obj].ob_next`, which `w_nilit` left at
> `NIL` (`:172-178,1262`), and reads `tree[-1]` (`gemobjop.c:145-160`) —
> **undefined**, nothing to rebuild. NewTOS contract for that case: `TRUE`,
> no repaint, no message, no z-order change, the window's rectangles left as
> they are (host test `aes_window_top_unopened.rs`).

> **`WF_CXYWH` on an open window — `draw_change` branch by branch
> (`gemwmlib.c:1058-1223`; added 2026-09-20, A78).** `draw_change` first
> stores the geometry — `WS_PREV := WS_CURR`, `WS_CURR := rect`, `WS_WORK :=
> wm_calc(WC_WORK, kind, rect)` (`:1069-1077`) — and returns there for a window
> that is not open (`:1085-1086`). For an open one it rebuilds every rectangle
> list (`:1089`) and then, with `c` = the **old** rectangle, `pt` = the new one,
> `S` = `DROP_SHADOW_SIZE` and `gl_wtop` unchanged (`wm_set` never reorders),
> chooses `start`/`stop`/`moved` and the damage rectangle:
>
> | Case | Pixels the AES paints | `WM_REDRAW` fan-out (`w_update(start, c, stop, moved)`, `:1222`) |
> |---|---|---|
> | `pt == c` (`:1108-1116`) | nothing | none — `draw_change` returns before drawing (`:1115-1116`) |
> | same corner, `pt.w <= c.w && pt.h <= c.h` — **pure shrink** (`:1141-1146`) | `gl_wtop`'s frame (`w_cpwalk`, `:1144`), then `w_drawdesk` over the damage (`start = DESKWH`, `:1149-1150`) | `stop = handle`, `moved = TRUE` (`:1143,1145`): windows from `handle` **downwards**, `gl_wtop` skipped (`:1025`) — the shrunk window itself gets **no** message when it is on top. Damage `(c.x, c.y, max(pt.w, c.w) + 2S, max(pt.h, c.h) + 2S)`: the shadow is added in the size branch (`:1153-1154`) *and* in the common tail (`:1214-1215`) |
> | same corner, any other size change (`:1138-1155`) | as above, `moved = FALSE` | every window top→bottom, same double-shadow damage |
> | different corner, `pt ⊇ c` — **superset grow** (`:1160-1167`) | frames only: `start = handle`, so no `w_drawdesk` (`:1218`) | windows from the top **down to `handle`** (inclusive; lower ones are not visited), damage `pt + S` |
> | **pure move of `gl_wtop`** (`:1174-1177` → `w_move`, `:938-996`) | **blit** `WS_PREV + S` → `WS_TRUE` on screen (`bb_screen`, `:970`) unless the source reaches past the right or bottom screen edge **and** the move goes toward that edge (`:951-957`); both rectangles are clipped to `gl_rfull` and a source or destination at `x = -1` has its left fringe realigned (`w_mvfix`, `:915-930,963-964,972-982`) | blit case: `stop = handle`, `moved = TRUE` (`:959,994`) — the moved window gets **no** message; every other window (top→bottom) gets its part of `s + S`, `s` = the **vacated** rectangle (`:984,992`). No-blit case: `stop = DESKWH`, `moved = FALSE`, damage `(s ∪ d) ∩ gl_rfull + S` (`:954,988`), every window including the moved one |
> | any other move / move-and-size / zero rect (`:1180-1190`) | `w_drawdesk` + frames (`start = DESKWH`) | damage `(pt ∪ c) + S` (zero rect: `c + S`), every window top→bottom |
>
> **Order and rectangle.** `w_update` (`:1004-1048`) clips the damage to
> `gl_rfull` (`:1010`) and visits the windows from `stop` (`DESKWH` = the top of
> the z-order, `W_TREE[ROOT].ob_tail`) **down** to `start` (`DESKWH` = the
> bottom, `ob_head`), `:1014-1043`; each visited window first has its frame
> drawn (`w_cpwalk`, `:1029`) and is then sent, by `w_redraw` (`:887-907`),
> `WM_REDRAW(handle, damage ∩ hull(w_rlist))` iff `damage ∩ WS_WORK ≠ ∅` and
> its list is non-empty. So with `h1` under `h2` and `h1` moved, `h2`'s message
> is queued **before** `h1`'s. The blit is a screen-to-screen `vro_cpyfm(S_ONLY)`
> with clip `gl_rfull` under `gsx_moff`/`gsx_mon` (`gemgraf.c:328-363`); its
> mechanism is reference (header above), its **effect is contract**: after a
> pure move of the top window its pixels stand at the new place and the owner
> has **not** been asked to redraw.
>
Slider **value validation** (`gemwmlib.c:1643-1656`): `WF_HSLIDE`/`WF_VSLIDE`
positions and `WF_HSLSIZ`/`WF_VSLSIZ` sizes are clamped to **1..1000**; a size
of **−1** ("use default") is passed through unclamped.

> **D-098 — the "slider creep" rounding fix is only half-applied in the
> reference build.** EmuTOS commit `a04ebb02` ("Fix 'slider creep' in AES
> windows", 2021) replaced `mul_div` with `mul_div_round` in `gr_slidebox`
> (`gemgrlib.c:410`, drag → stored 0..1000 value) and in the **non-3D**
> `w_bldvbar`/`w_bldhbar` (`gemwmlib.c:667-669,708-710`, stored value → pixel
> geometry). The **3D-object (default, `CONF_WITH_3D_OBJECTS=1`)**
> `w_bldvbar`/`w_bldhbar` (`gemwmlib.c:433-434,471-472`) still use the
> unrounded `mul_div` — 3D-object support (commit `dfa61fe9`) was added
> *after* the rounding fix and reused the old, unrounded pattern for its own
> elevator-geometry code, so the fix never propagated there. Net effect in
> EmuTOS's default build: dragging a slider is converted to a value with
> `mul_div_round`, but the elevator's *redrawn* size/position from that same
> stored value uses plain (truncating) `mul_div` — a residual, genuine
> EmuTOS source-level rounding asymmetry (same class as D-081/D-082), not a
> spec-reading error. This sits entirely inside the frame-drawing code
> (`w_bldactive`/`w_bldvbar`/`w_bldhbar`) already marked **reference** at the
> top of this document, so it is not contract; NewTOS's own elevator geometry
> is free to round consistently instead of reproducing the creep, as long as
> the **contract** value — `WF_HSLIDE`/`WF_VSLIDE`/`WF_HSLSIZ`/`WF_VSLSIZ`,
> always 1..1000 or −1 — round-trips correctly through `wind_get`/`wind_set`.

---

## `wind_find` (106)

**Source:** `gemsuper.c:348-350` → `gemwmlib.c:1767` (`wm_find`), which is
`ob_find(gl_wtree, 0, 2, x, y)`.

Returns the handle of the topmost window whose rect contains (x,y).
`ob_find` searches the on-screen window tree `W_TREE` from the root to depth 2.
Because handle 0 (`DESKWH`) is the root box covering the whole screen, a click
not inside any open window returns **0 = the desktop**.

---

## `wind_update` (107)

**Source:** `gemsuper.c:351-353` → `gemwmlib.c:1778` (`wm_update`).
`void`; `int_out[0]` stays 1.

| `int_in[0]` | Mode | Effect (`gemwmlib.c:1781-1796`) |
|--:|------|--------|
| 0 | `END_UPDATE` | release the window-tree lock (`unsync(&wind_spb)`) |
| 1 | `BEG_UPDATE` | take the window-tree lock (`tak_flag`; block on `ev_block(MU_MUTEX)` if held) |
| 2 | `END_MCTRL` | release mouse-control ownership (`fm_own(0)`) |
| 3 | `BEG_MCTRL` | take mouse-control ownership (`fm_own(1)`) |

`BEG_UPDATE`/`END_UPDATE` guard the window tree while the app redraws from its
rectangle list, so the AES doesn't restructure windows mid-redraw (it is a
counting mutex — the AES itself brackets `wm_open`/`wm_close`/`wm_set` with the
same lock). `BEG_MCTRL`/`END_MCTRL` let the app take over the mouse (e.g. to run
its own rubber-band) so the control manager stops interpreting clicks as
window-gadget hits.

> **`fm_own`'s edge bodies (A102, 2026-09-21).** `fm_own(TRUE)`
> (`gemfmlib.c:82-96`) takes `BEG_UPDATE` and, on the outermost `ml_ocnt == 0`
> → 1 transition only, parks the menu tree (`ml_mnhold = gl_mntree; gl_mntree
> = NULL`), remembers the control rectangle and the mouse owner
> (`get_ctrl(&ml_ctrl); get_mown(&ml_pmown)`, `:91-92`) and gives both to the
> caller: `ct_chgown(rlr, &gl_rscreen)` (`:93`) = `set_ctrl(&gl_rscreen); if
> (!gl_ctmown) set_mown(rlr);` (`gemctrl.c:392-397`), where `set_mown` moves
> `gl_mowner` only with the buttons up (`if (!button)`, `geminput.c:222`).
> `fm_own(FALSE)` (`:98-106`) decrements and, on the 1 → 0 transition only,
> restores in the same order reversed: `ct_chgown(ml_pmown, &ml_ctrl)`
> (`:102`) — the owner found on entry gets the mouse back under the same two
> guards — then `gl_mntree = ml_mnhold`, then `END_UPDATE`. Observable only
> when that owner was `ctl_pd` (the pointer had entered `gl_ctwait` with the
> buttons up): `BEG_MCTRL` makes the application the owner, `END_MCTRL` makes
> `ctl_pd` the owner again.

> **Reference vs. contract.** The lock *semantics* a program observes — "while I
> hold BEG_UPDATE the window layout is frozen and my rectangle list stays
> valid", and "a second process's BEG_UPDATE waits until I let go" (see
> "The refusal parks the taker" below) — are contract; the
> `tak_flag`/`ev_block`/`sy_tas` cooperative-mutex mechanics are EmuTOS
> internals NewTOS reimplements over its own scheduler.

### Each counter carries its owner (A110, 2026-09-23)

`wind_spb` is an `SPB`, and `tak_flag` writes the taking process into
`sy_owner` (`aes/gemflag.c:77-82`) while `unsync` clears it on the 1 → 0 edge
(`:125-129`). The field names exactly one process: a second one asking for a
held lock is refused and parked on `sy_wait` (`wm_update`'s
`ev_block(MU_MUTEX, (LONG)&wind_spb)`, `gemwmlib.c:1785-1786`), so the count is
the one owner's nesting depth. It is read in a single place — `sh_ldapp` takes
the screen lock off a program that has ended **only** when that program held
it: `if (wind_spb.sy_owner == rlr) unsync(&wind_spb)` (`aes/gemshlib.c:668-669`).

`ml_ocnt` has no owner field (`aes/gemfmlib.c:51`). It does not need one in
EmuTOS: the only force-drain is `wm_new`'s (`while (ml_ocnt > 0)
wm_update(END_MCTRL)`, `gemwmlib.c:1879-1881`), which runs from the shell loop
after the swept program was the only client (`gemshlib.c:676-677`).

NewTOS keeps an owner on **both** counters. The screen lock's is EmuTOS's own
field; the mouse-control counter's is a named NewTOS addition, forced by the
`Pexec`-child sweep (`wind_new (109)` below, A107): that sweep runs while the
parent client is alive, so it must drain the dead slot's holds and no others.
Draining the count blindly would hand the parent's mouse back and un-park its
menu tree in the middle of its own modal dialog. The sweep drains `ml_ocnt`
first and through `END_MCTRL`, as `wm_new` does, so `fm_own`'s 1 → 0 edge body
runs (`gemfmlib.c:99-104`) and the `BEG_UPDATE` it took comes off with it; then
the screen lock, under `sh_ldapp`'s owner test. Receipts:
`tos-api/tests/aes_a96_zombie_windows.rs` `a110_*`.

### The refusal parks the taker — a real mutex (GH1, 2026-10-21)

*Correction.* Until 2026-10-21 this section said that `tak_flag`'s refusal was
"the one thing that is not reproduced" and that "nothing observes it". Both
were wrong. GitHub issue #1 (`TASK/GH1.md`,
`measurements/measurement-2026-10-19-gh1/RUNINFO.TXT`) is what observes it: a
desk accessory opened from the *Desk* menu called `wind_update(BEG_UPDATE)`,
`form_center`, `form_dial`, `objc_draw` and `form_do`'s `fm_own(TRUE)` while
the press that picked its menu entry was still held, so `set_mown` refused to
move the mouse (`if (!button)`, `aes/geminput.c:222`), the desktop stayed the
mouse owner, the desktop's parked `MU_BUTTON` was completed by the clicks in
the accessory's dialog, and the desktop repainted its window frame over the
dialog. In EmuTOS the accessory cannot get there before the release, because
the lock it asks for is held by the Control Manager across exactly that wait.

**EmuTOS** (`aes/gemflag.c:68-130`, `aes/gemwmlib.c:1778-1797`):

```c
WORD tak_flag(SPB *sy) {                 /* :68-86 */
    sy->sy_tas++;
    if ((sy->sy_owner != rlr) && (sy->sy_tas != 1))
        sy->sy_tas--;                    /* refused */
    else
        sy->sy_owner = rlr;
    return (sy->sy_owner == rlr);
}
/* wm_update(BEG_UPDATE): */
if (!tak_flag(&wind_spb))
    ev_block(MU_MUTEX, (LONG)&wind_spb); /* amutex: evinsert(e, &sy->sy_wait) */
void unsync(SPB *sy) {                   /* :105-130 */
    if (sy->sy_tas <= 0) { sy->sy_tas = 0; return; }
    sy->sy_tas--;
    if (sy->sy_tas == 0) {
        if ((p = sy->sy_wait) != 0) {    /* hand over to the head of the line */
            sy->sy_wait = p->e_link;
            sy->sy_owner = p->e_pd;
            sy->sy_tas = 1;
            azombie(p, 0);
            dsptch();
        } else
            sy->sy_owner = NULL;
    }
}
```

The Control Manager is a taker like any other: `ctlmgr`'s loop is
`w_setactive(); ev_multi(…); wm_update(BEG_UPDATE); /* take the screen */ …
hctl_window / hctl_rect … wm_update(END_UPDATE); /* give up the screen */`
(`aes/gemctrl.c:439-484`), and a menu selection's `ct_msgup` waits for the
selecting press to come up *inside* that bracket (`while (button & 0x0001)
dsptch();`, `:100-101`). The desktop brackets every event it handles the same
way (`desk/deskmain.c:2094,2121`).

**Contract (what NewTOS reproduces):**

1. A `BEG_UPDATE` — the mode itself, or the one `fm_own(TRUE)` takes
   (`BEG_MCTRL`, `form_do`, `form_alert`, `form_error` for a code with an
   alert, `menu_popup`, the file selector), or the AES's own (see "The AES's
   own takers (A118)" below) — by a process while **another** process holds the
   lock does not return until the lock has been handed to it. Waiters are
   served in arrival order; the holder's last `END_UPDATE` hands the lock to
   the head of the line with the count at 1.
2. "Another process" is EmuTOS's `AESPD`: the desktop, every launched program
   and every `Pexec` child are one (they nest on each other's hold), each desk
   accessory is one of its own.
3. The Control Manager holds the lock from the moment it acts on an event —
   a menu track, a frame click — until it is done with it; after a menu
   selection or an `AC_OPEN` that is **the release of the selecting press**.
   An accessory that answers `AC_OPEN` with `wind_update(BEG_UPDATE)` (or a
   dialog) therefore runs on only after the release, with the button up, and
   its `fm_own(TRUE)` gives it the mouse. The same holds for a press the
   Control Manager owns that produces **no** message: on a window,
   `hctl_window` ends in `ct_msgup(0, …)` and waits for the release under the
   lock (`aes/gemctrl.c:81-102,182,314,463-470`) — the press that cancelled a
   drop-down over the top window's work area (`bchange` left it with
   `ctl_pd`, `gl_ctmown` was up, `aes/geminput.c:334-335`;
   `docs/api/aes/menu.md` point 5). An application that takes the lock or
   mouse control (`BEG_MCTRL`) after that press gets it with the button up,
   so `set_mown` gives it the mouse; were the lock free, it would hold the
   screen with `ctl_pd` owning the mouse and, by point 4, nobody to hand it
   back (the GH1 regression, QED hang,
   `measurements/measurement-2026-10-21-gh1-fix2/`).
4. While a process holds the lock the Control Manager does not act: no bar
   track, no frame click, no `w_setactive` hand-back of the mouse. The pointer
   over the bar and a latched frame press stay pending until the holder's
   `END_UPDATE`.
5. `wind_new`/`wm_new`'s drain (`while (wind_spb.sy_tas > 0)
   wm_update(END_UPDATE)`, `gemwmlib.c:1882-1883`) ends with the count at 0
   and every waiter released.

**NewTOS mapping** (`tos-api/src/aes/window.rs`): `tak_flag`/`unsync` are
the EmuTOS functions over `UPDATE_LOCK`/`UPDATE_OWNER`, the wait line is
`UPDATE_WAIT`. A refused caller is parked by `update_block` — called by the
opcode **before** it commits anything (`wind_update`, and the dispatcher's
gate for opcodes 36/50/52/53 and, since A118, 19/70/71/90/91/101/102/103/105) — on the scheduler's `WAIT_MUTEX` wake class;
the hand-over sets `UPDATE_GRANTED` for the waiter and wakes it, and its
re-executed call (the S2 restartable block) collects the hand-over in
`tak_flag` without counting it twice. The inline Control Manager takes the lock
in `ctl_pd`'s name (`ctl_begin`/`ctl_finish`, the `CTL_PD` owner), keeps it
across `ct_msgup`'s wait (`input::ct_msgup_hold`; for a press on the top
window that yields no message, with the mouse not the application's:
`window::hctl_window`'s `Gadget::None` arm) and gives it up when that
wait ends (`input::ct_msgup_busy` → `ctl_release`); the event loops run it
only while `ctl_may_run()`. A waiter parked behind the Control Manager also
wakes on input, because the release that frees the lock is input. Each
identity's share of the count is kept in `UPDATE_HOLDS`, so the dead-slot
sweep below releases exactly the dead client's holds, through `unsync`,
handing the lock on to the next waiter. Receipts:
`tos-api/tests/aes_gh1_update_mutex.rs`, the GH1 run
`measurements/measurement-2026-10-21-gh1/`.

### The AES's own takers (A118, 2026-10-25)

*Correction.* Until 2026-10-25 this section listed, as "not reproduced", that
`wm_open`/`wm_close`/`wm_set`, `w_redraw_desktop`, `graf_rubberbox`/
`graf_dragbox`, `ap_exit` and the file selector did not take the lock, and
that the shell loop's `fm_error` could not wait for it. Checked one by one
against EmuTOS (`TASK/A118.md`); every one of them but `w_redraw_desktop` is
now a taker like `wind_update` itself:

| Taker (EmuTOS) | Taken | Not taken for |
|---|---|---|
| `wm_open` (`aes/gemwmlib.c:1422,1429`) | around the open | an unknown handle, an open window (`:1415-1419`) |
| `wm_close` (`:1450,1456`) | around the close | an unknown handle, a window that is not open (`:1443-1447`) |
| `wm_delete` (`:1473-1474`) | through `wm_close`, for an open window | a closed window, an unknown handle |
| `wm_set` (`:1638,1757`) | around the whole body, every exit (the unknown-field `FALSE` too) | an unknown handle — except for `WF_DCOLOR`, which needs none (`:1632-1636`) |
| `gr_rubwind` behind `graf_rubberbox` (`aes/gemgrlib.c:205,219`) | around the track | — |
| `gr_dragbox` behind `graf_dragbox` (`:250,267`) | around the drag | — |
| `ap_exit` (`aes/gemaplib.c:253,258`) | from before `mn_cleanup` to after `wait_for_accs(AP_ACCLOSE)` | — |
| `fs_input` → `fm_do` → `fm_own(TRUE)` (`aes/gemfslib.c:718`, `aes/gemfmlib.c:287,349`) | around the dialog, with the mouse for the caller | a `NULL` path (`gemfslib.c:630-631`) |
| `sh_main` → `fm_error` → `fm_alert` (`aes/gemshlib.c:722-723`, `aes/gemfmalt.c:361`) | around the alert | a code without an alert (`n > 63`) |

Consequences a program sees:

1. A second process's `wind_open`, `wind_close`, `wind_delete` of an open
   window, `wind_set`, `graf_rubberbox`, `graf_dragbox`, `appl_exit` or
   `fsel_input`/`fsel_exinput` does not return while another process holds
   the lock (point 1 above). A call the function refuses first returns its
   `FALSE` at once.
2. A desk accessory that answers the `AC_CLOSE` of an exiting program with a
   `BEG_UPDATE` — or a `wind_close`, `wind_set`, a dialog — runs on only after
   the exiting program's `wait_for_accs` has seen every accessory's
   `AC_CLOSE`; the program's `END_UPDATE` hands the lock to it.
3. A file selector opened by a desk accessory owns the mouse and the
   keyboard for its duration (`fm_own(TRUE)` → `ct_chgown(rlr, …)`,
   `aes/gemfmlib.c:93`): a press in it completes no other process's parked
   `MU_BUTTON` (the GH1 symptom, for the file selector).

**NewTOS mapping.** Each opcode waits in the dispatcher before it commits
anything (`aes::crysbind`'s gate, `window::opcode_takes_lock` for the four
window opcodes, `update_block`), and then brackets its body with the
`wm_update` pair it has in EmuTOS (`window::update_beg`/`update_end`;
`fm_own` for the file selector). `appl_exit` takes the lock once per
`AC_CLOSE` round — the call is re-executed while it waits for the
accessories (the S2 restartable block) — and gives it back after the wait.
The shell loop issues its alert as the AES call `form_error` through the GEM
trap from its EL1 context (`kernel/src/shell_loop.rs` `fm_error`), so the
dispatcher's gate can park it like any client (ACC03 kernel park, the `svc`
re-executed). Receipts: `tos-api/tests/aes_a118_lock_takers.rs`.

**A refusal after the wait (A118 rework, 2026-10-26).** EmuTOS makes the
window checks of `wm_open`/`wm_close`/`wm_set` *before* `wm_update(
BEG_UPDATE)`; a process woken from that wait is the holder (`unsync`,
`aes/gemflag.c:121-122`) and ends its hold with its own `END_UPDATE`. NewTOS
re-executes the parked call from its `svc`, so the checks run again — and
when the holder has meanwhile deleted, opened or closed the window, the
re-executed call refuses (`FALSE`). It then still collects the lock it was
handed and ends it (`window::end_unused_grant`), so the lock goes on to the
next waiter exactly as after the caller's own `END_UPDATE`; it does not stay
with the caller (found by the Tester,
`measurements/measurement-2026-10-25-a118-test/RUNINFO.TXT`). Named
difference: EmuTOS would run the body on the window as it found it before the
wait (e.g. `wm_open` on a slot another process has since freed); NewTOS
answers with the refusal the current table gives. The lock outcome is the
same.

Two places where NewTOS's shape differs from EmuTOS's, named:

* **The file selector waits before it draws.** `fs_input` draws the dialog
  (`fm_dial(FMD_START)`, `ob_draw`, `gemfslib.c:707-709`) and then waits in
  `fm_do`'s `fm_own(TRUE)`; NewTOS waits at the opcode's entry, because a
  call that has to give the CPU away is re-executed from its `svc` and must
  not have drawn yet. Observable only while another process holds the lock
  when the selector is called: the dialog appears when the holder lets go,
  not before. EmuTOS also takes and releases the lock around each `fm_do`
  round of the selector's loop (and around the slider drag,
  `gemfslib.c:790-792`); NewTOS's loop gives the CPU to nobody between its
  rounds, so one bracket around the loop is the same.
* **Non-waiting takers that remain.** A host-less unit test, and the
  `Pexec`-child sweep (`wm_new_for_slot`, a NewTOS rule with no EmuTOS
  counterpart, `wind_new (109)` below), which closes a dead child's windows
  in the living caller's name at the next AES dispatch: finding the lock
  held by another process, they count their bracket on top of the owner's
  hold and take it back off (`update_beg`); the owner keeps the lock.

`w_redraw_desktop` (`aes/gemwmlib.c:1902-1941`, 3D build) is **not** a missing
taker but a missing function: it is the Screen Manager's answer to a
`WM_REDRAW` in its own queue (`hctl_msg`, `aes/gemctrl.c:380-384`), and NewTOS
has no such queue — `appl_write` to pid 1 succeeds and the message goes
nowhere (`appl.rs`, "`hctl_msg` … not reproduced"). Open as a function of its
own, not under A118.

---

## `wind_calc` (108) — the border↔work conversion (the geometry contract)

**Source:** `gemsuper.c:354-357` → `gemwmlib.c:1807` (`wm_calc`). `void`; the
four output words are `int_out[1..4]` (`WM_WCOX..WM_WCOH`).

Given `type`, `kind`, and an input rect (x,y,w,h) it returns an output rect:

- **`WC_WORK` (1):** input is the **border** (outer) rect → returns the **work**
  (inner) rect. `x += lb; y += tb; w -= lb+rb; h -= tb+bb`.
- **`WC_BORDER` (0):** input is the **work** rect → returns the **border** rect.
  Same code with the four border thicknesses **negated** (`gemwmlib.c:1856-1862`),
  i.e. it grows the rect outward.

The four border thicknesses are computed from `kind` (base 1 px each side):

**Non-3D build (`CONF_WITH_3D_OBJECTS = 0`)** (`gemwmlib.c:1841-1853`):

| Border | Base | Added when |
|--------|-----:|-----------|
| left `lb` | 1 | (never grows) |
| top `tb` | 1 | `+ (gl_hbox-1)` if `kind & (NAME\|CLOSER\|FULLER)`; `+ (gl_hbox-1)` again if `INFO` |
| right `rb` | 1 | `+ (gl_wbox-1)` if `UPARROW\|DNARROW\|VSLIDE\|SIZER` |
| bottom `bb` | 1 | `+ (gl_hbox-1)` if `LFARROW\|RTARROW\|HSLIDE\|SIZER` |

> **D-097 — `TGADGETS` does not apply to the non-3D title-band test.** The
> literal at `gemwmlib.c:1842` is `kind & (NAME|CLOSER|FULLER)`, **not** the
> `TGADGETS` macro (`NAME|CLOSER|FULLER|MOVER`, `gemwmlib.h:16`) — a previous
> spec revision labelled this row `TGADGETS`, which is wrong for the non-3D
> branch (it *is* correct for the 3D branch below, which uses `kind &
> TGADGETS` at `gemwmlib.c:1818`). Consequence: in the **non-3D** build, a
> `MOVER`-only window (no `NAME`/`CLOSER`/`FULLER`) gets **no** title band
> from `wind_calc`/`wm_calc`, consistent with `w_bldactive`'s own non-3D
> branch using the identical literal with no `MOVER`-without-`NAME` special
> case (`gemwmlib.c:750`). Low real-world impact (`CONF_WITH_3D_OBJECTS`
> defaults to 1), but a factual correction to the table as stated.

**3D build (`CONF_WITH_3D_OBJECTS = 1`, the default)** (`gemwmlib.c:1817-1840`):

| Border | Base | Added when |
|--------|-----:|-----------|
| top `tb` | 1 | `+ (adj_hbox-1)` if `TGADGETS`; `+ gl_hbox` if `INFO` |
| right `rb` | 1 | `+ (adj_wbox-1)` if a **vertical bar** is needed |
| bottom `bb` | 1 | `+ (adj_hbox-1)` if a **horizontal bar** is needed |
| left `lb` | 1 | (never grows) |

where `adj_wbox = gl_wbox + 2·ADJ3DSTD`, `adj_hbox = gl_hbox + 2·ADJ3DSTD`,
`ADJ3DSTD = 2` (`gemwmlib.c:1311-1312`, `obdefs.h:136`), and `gl_hbox =
gl_hchar+3`, `gl_wbox = max(gl_wchar+4, gl_hbox·aspect)` (`gemgraf.c:412-415`).

> **The SIZER-but-no-scroll-bar band (contract edge case).** In both builds a
> `SIZER` counts toward *both* the vertical and horizontal bar tests, so a
> window with only `SIZER` (no scroll gadgets) reserves both a right and a
> bottom band. In the 3D build `wind_calc` additionally mirrors
> `w_bldactive`'s rule: when both bars would be present *only because of* the
> SIZER, it strips SIZER from both and, if that leaves neither bar, forces the
> **vertical** bar back on (`gemwmlib.c:1827-1834`). So a SIZER-only window has
> a **right band + a 1-px bottom** (the vertical bar carries the sizer corner),
> matching TOS4. This is exactly the layout `w_bldactive` builds
> (`gemwmlib.c:590-592`).

> **Correction — two places where `w_bldactive`'s work area does NOT match
> `wm_calc` (EmuTOS-internal inconsistency; found 2026-08-31 with `557614fb`,
> moved here 2026-10-26 from the STATUS log, lines re-read at the pin
> `1a7484ab`).** The sentence above holds for the **bars** `w_bldactive`
> draws (`gemwmlib.c:573-592`), not for the `W_WORK` object it lays out:
>
> 1. **SIZER.** `wm_calc` strips `SIZER` from both bar tests when both are
>    set and forces only the vertical one back (`gemwmlib.c:1825-1834`), with
>    the comment *"the same as we do in w_bldactive()"* (`:1829`). But
>    `w_bldactive` shrinks the work area with the **raw** tests,
>    `if (havevbar || (kind & SIZER))` / `if (havehbar || (kind & SIZER))`
>    (`:598-601`). For any `SIZER` window that lacks one bar's gadgets —
>    `SIZER` alone, or `SIZER` with only vertical or only horizontal gadgets —
>    the drawn `W_WORK` is `adj_wbox-1` narrower or `adj_hbox-1` shorter than
>    the work rect `wm_calc` reports. The comment at `:1829` is not true.
> 2. **Title band.** With `TGADGETS`, `w_bldactive` puts `W_DATA` at
>    `adj_hbox` (`:556`) and `W_WORK` one pixel inside it (`:595`, `:603`), so the
>    drawn work area starts at `adj_hbox + 1`; `wm_calc` reports `tb = 1 +
>    (adj_hbox - 1) = adj_hbox` (`:1818-1819`) — one pixel higher. The non-3D
>    branch has no such offset.
>
> **Why it has no effect in EmuTOS:** `W_DATA` and `W_WORK` are `G_IBOX` with
> border thickness 0 (`gl_watype`/`gl_waspec`, `gemwmlib.c:69-70`, `:92-93`)
> and draw nothing; everything that uses the work area reads the **stored**
> `WS_WORK`, which comes from `wm_calc`. **NewTOS** follows `wm_calc` and
> paints the frame bands exactly complementary to the reported work rectangle
> (`557614fb`, commit message).

`wind_calc` never touches the window list — it is a pure function of
`kind`, `gl_wbox`/`gl_hbox`, and the config. `draw_change` itself calls
`wm_calc(WC_WORK, …)` to derive each window's work rect from its border rect
(`gemwmlib.c:1075`), so `WF_WORKXYWH` and `wind_calc(WC_WORK, kind, border)`
agree by construction.

---

## `wind_new` (109)

**Source:** `gemsuper.c:358-360` → `gemwmlib.c:1875` (`wm_new`). `void`.

Cleanup call: releases **all** outstanding locks (`END_MCTRL` while `ml_ocnt>0`;
`END_UPDATE` while `wind_spb.sy_tas>0`) and then **closes and deletes every
window** 1..NUM_WIN−1 (`gemwmlib.c:1880-1892`). Used to reset the window manager
to a clean state (e.g. a crashed/exiting app's leftover windows). The desktop
window (handle 0) is not deleted.

> **After a launched application returns, the window table is swept
> (NewTOS rule, A96, 2026-09-21).** EmuTOS runs `wm_new()` from the shell
> loop after *every* program it ran: `sh_ldapp` → `dos_exec(PE_LOADGO, …)`
> returns → `AP_OPEN` cleanup → `unsync(&wind_spb)` → `if (gl_shgem)
> wm_new();` (`gemshlib.c:645-677`; [`shell-loop.md`](shell-loop.md) §8).
> Whatever windows the program left — open or merely created — are closed
> (`wm_close`, with its `draw_change` repaint and `WM_REDRAW` fan-out) and
> deleted, and its `wind_update` locks are gone, before anyone runs again.
> The desktop's windows go too: in EmuTOS the desktop is itself a program
> the shell loop restarts (`sh_main`, `gemshlib.c:708-731`), and it rebuilds
> its windows from its own context save (`cnx_get`, `desk/deskmain.c:2055`).
>
> NewTOS's shell loop (`kernel/src/shell_loop.rs`) does exactly that. The
> built-in desktop's double-click launch does **not** pass through the shell
> loop (`programs/desktop/src/main.rs` `run_gem_app`, AES gap G1): the program runs
> nested in the desktop's event loop and the desktop **resumes in place**,
> repainting the windows it still holds in the shared table (gap G2 — no
> `cnx_get`). So the sweep NewTOS runs when the desktop regains control
> (`desktop_resume` → `tos_api::aes::window::wm_new_keep_desktop`) is
> `wm_new` **minus the windows the desktop identity owns** (`w_owner ==
> pd::KERNEL_CONTEXT`): every other window — the program's, or a `Pexec`
> descendant's that had no sweep of its own — is closed if open and deleted,
> both lock counters are reset, and a `WF_NEWDESK` tree the program
> registered is dropped (EmuTOS resets `gl_newdesk` in `wm_init` before the
> desktop restarts, `gemwmlib.c:1300`; `desktop_resume` re-registers the
> desktop's own tree right after, `deskmain.c:2049`). When the desktop is
> back no EL0 process is alive (single-tasking), so "not the desktop's" is
> "a dead program's". Observable contract, host-tested
> (`tos-api/tests/aes_a96_zombie_windows.rs`): after the resume the dead
> program's handle is free, `wind_find` inside its rectangle answers the
> desktop, the whole-screen composite paints its rectangle as bare desktop,
> and the desktop's own windows are untouched.
>
> **A `Pexec` child of a living GEM parent is swept at the parent's next AES
> call (NewTOS rule, A107, 2026-09-22).** EmuTOS has no such child: a GEM
> program that starts another one does it through the shell — `shel_write`
> ends the parent, `sh_ldapp` runs the child and sweeps, `sh_main` starts the
> parent again (`gemshlib.c:600-612,708-731`). NewTOS runs an EL0 `Pexec`
> child **nested** inside its living parent (TeraDesk → QED,
> `sdk/apps/teradesk/startprg.c:259,309`), so neither the shell loop nor
> `desktop_resume` sees it. The child's death is marked on its GEMDOS slot
> (`aes::release_client`, called from `bdos::process::pterm::release_slot`)
> and the sweep runs at the **next AES dispatch** (`aes::dispatch` →
> `sweep_dead_clients` → `window::wm_new_for_slot`), before that call's
> opcode: `release_slot` runs on the `Pterm` exception stack with the dying
> process's mapping installed and the AES fiber possibly parked, and the
> sweep draws (`wm_close` → `draw_change`, `gemwmlib.c:1453,1216-1222`),
> so it must not run there; the dispatch entry is the context every other
> AES draw uses. For that slot — and only that slot — its windows are closed
> and deleted, its `WF_NEWDESK` tree is dropped if it registered the current
> one, and its menu bar is dropped if it installed the current one
> ([`menu.md`](menu.md) § "A dead client's menu bar"). Since A110 the two
> `wind_update` counters go the same way: each carries the slot that took it
> (see `wind_update (107)` § "Each counter carries its owner"), the sweep
> drains `ml_ocnt` through `END_MCTRL` and then the screen lock, both only
> while the owner is the dead slot — `wm_new`'s drain loops
> (`gemwmlib.c:1879-1883`) under `sh_ldapp`'s owner test (`if
> (wind_spb.sy_owner == rlr) unsync(&wind_spb)`, `gemshlib.c:668-669`). A
> hold of the **living** parent is left standing, which is the whole point of
> the test. Host-tested in `tos-api/tests/aes_a96_zombie_windows.rs` (three
> `a107_*` cases: the window, the background tree and the menu bar of the
> child go, the parent's stay; three `a110_*` cases for the counters).

---

## Graphics Manager

The interactive trackers own the mouse for the duration, draw XOR feedback over
the screen, and block until the button is released. All frame the tracking with
`wm_update(BEG_UPDATE)`/`(END_UPDATE)` and draw in `MD_XOR` mode via
`gr_setup`/`gsx_xbox` (`gemgrlib.c:76-80`, reference).

### `graf_rubberbox` (70)

**Source:** `gemsuper.c:266-268` → `gr_rubbox` → `gr_rubwind`
(`gemgrlib.c:199-232`). Pins one corner at `(x,y)` and stretches the opposite
corner to follow the mouse until the button comes up. `gr_clamp`
(`gemgrlib.c:83`) computes the returned extent as
**`w = max(xrat - x + 1, wmin)`, `h = max(yrat - y + 1, hmin)`** (`xrat`/`yrat` =
live mouse). The result is therefore **signed**: `w`/`h` are the
pointer-minus-origin deltas (**note the `+1`**), clamped *by a signed `max`* to
`wmin`/`hmin`. A normal app passing **positive** `wmin`/`hmin` gets a positive
minimum-size box that only grows down-right (a leftward drag is clamped back up
to `wmin`). To get **free 4-quadrant** growth (negative `w`/`h` when dragging
up/left), the caller passes **`wmin = hmin = SHRT_MIN`** — exactly what the
desktop's rubber-band select does (`deskact.c:631`
`graf_rubbox(mx, my, SHRT_MIN, SHRT_MIN, ...)`). Returns `int_out[1]`/`int_out[2]`
(`GR_O1`/`GR_O2`) = **signed** `w`/`h`. `void`; `int_out[0]`=1. **NewTOS note:**
implement `int_out[1]/[2] = max(cursor - origin + 1, wmin/hmin)` directly — the
`+1` and the signed `max` (not a magnitude-clamp) are load-bearing.

### `graf_dragbox` (71)

**Source:** `gemsuper.c:269-271` → `gr_dragbox` (`gemgrlib.c:243-268`). Drags a
fixed-size `w`×`h` box, keeping the mouse's grab offset from `(sx,sy)`, until
button-up, **constrained inside** the bounding rect `int_in[4..7]`
(`rc_constrain`). Returns the final top-left x/y in `int_out[1]`/`int_out[2]`.

### `graf_movebox` (72)

**Source:** `gemsuper.c:272-274` → `gr_movebox` (`gemgrlib.c:287-302`). Animates
an XOR outline box `w`×`h` sliding from `(srcx,srcy)` to `(dstx,dsty)` in
`gr_scale`-derived steps. `void`, no result; a pure visual.

### `graf_growbox` (73) / `graf_shrinkbox` (74)

**Source:** `gemsuper.c:275-280` → `gr_growbox`/`gr_shrinkbox`
(`gemgrlib.c:309-334`). Zoom-rect animations between a **small** rect
(`int_in[0..3]`) and a **big** rect (`int_in[4..7]`): grow small→big (used at
`wind_open`), shrink big→small (used at `wind_close`). `void`.

### `graf_watchbox` (75)

**Source:** `gemsuper.c:281-283` → `gr_watchbox` (`gemgrlib.c:337-355`). While
the button is held, toggles object `obj`'s `ob_state` between `instate` (mouse
inside its rect) and `outstate` (outside), redrawing on each transition. Returns
in `int_out[0]` the **final `out` flag** after button-up: **`out` is `TRUE`
(1) when the button was released *inside* the object, `FALSE` (0) when
outside**. The control manager treats this **inside-release (`out == TRUE`)**
as "clicked" — e.g. a CLOSER release inside fires `WM_CLOSED`
(`gemctrl.c:218-220`, `if (gr_watchbox(...))` fires exactly on `TRUE`).

> **D-096 — direction correction.** An earlier revision of this spec stated
> the semantics **backwards** ("`out` is `FALSE` when released inside, `TRUE`
> when outside") in its lead sentence, directly contradicted by a "corrects"
> callout immediately below it that already had the right direction (`TRUE` =
> inside) — an unresolved internal inconsistency. Traced `gr_watchbox`
> (`gemgrlib.c:337-355`) step by step: `out` starts `FALSE`; the first loop
> iteration draws `instate` (`out==FALSE` selects `instate`) and only *then*
> flips `out` to `TRUE`, before calling `gr_stilldn(out, …)` to wait for
> either a button-state change or a boundary crossing. In the common
> single-click case (button released before the mouse ever leaves the
> gadget), `gr_stilldn` returns `FALSE` (button-up detected) and the
> `do…while` loop exits immediately — with `out` still holding the `TRUE` it
> was flipped to just before the wait. Only if the mouse first leaves the
> rect (drawing `outstate`, flipping `out` back to `FALSE`) and *then* the
> button comes up does the function return `FALSE`. So **`TRUE` = released
> inside, `FALSE` = released outside** — matching `gemctrl.c:218`'s
> `if (gr_watchbox(...))` firing `WM_CLOSED`/`WM_FULLED` exactly on `TRUE`.
> NewTOS must reproduce this direction (`TRUE` = inside-release), not the
> previously-documented inverted one.

### `graf_slidebox` (76)

**Source:** `gemsuper.c:284-286` → `gr_slidebox` (`gemgrlib.c:358-413`). Drags a
child object (a scrollbar elevator) within its parent, constrained to the
parent's rect, and returns the resulting position as **0..1000**
(`mul_div_round(travel, 1000, range)`), or **0** if the track has zero range.
`isvert` (`int_in[2]`) selects vertical vs. horizontal travel.

### `graf_handle` (77) — the VDI bridge

**Source:** `gemsuper.c:287-293`. Returns, with no helper call, four cached
globals set at AES start-up (`gemgraf.c:40-52`, `405-415`): `int_out[0]` =
**`gl_handle`** (the physical VDI workstation handle the AES draws through),
`int_out[1]` = `gl_wchar`, `[2]` = `gl_hchar` (system-font cell width/height),
`[3]` = `gl_wbox`, `[4]` = `gl_hbox` (the gadget box width/height,
`gl_hbox = gl_hchar+3`). An application calls this once to learn the AES VDI
handle, then issues VDI calls on it.

**`gl_handle` is a real, open workstation.** It is the `CONTRL[6]` the VDI
answered to the AES's own `v_opnwk` (`gsx_wsopen`, `gemgsxif.c:169-186`,
`v_opnwk(intin, &gl_handle, &gl_ws)`), run by `gsx_init`
(`gemgsxif.c:267-270`) at the start of every AES session
(`run_accs_and_desktop`, `geminit.c:675`, before the accessories load) — i.e.
`VDI_PHYS_HANDLE` = **1** (`vdi_control.c:542-544`). `gsx_start`
(`gemgraf.c:383-428`) then sets that workstation up: `vqt_attributes`, the
two `vst_height` calls (`ws_chminh`, then back to the current height,
`:405-410`), `vsl_type(7)`, `vsl_width(1)`, `vsl_udsty(0xffff)`
(`:420-422`). Consequences an application observes:

- every VDI call on the `graf_handle` handle is answered — `vq_extnd(h, 0)`
  returns `DEV_TAB` (`[13]` colours, `[0]/[1]` the screen size) and
  `vq_extnd(h, 1)` `INQ_TAB` (`[4]` the plane count). WinDom's `ApplInit`
  (`sdk/windom/windom-1.21.3/src/appl_init.c:42-46`) takes `app.nplanes`/`app.color` from exactly these
  two calls on the `graf_handle` handle;
- the first `v_opnvwk` of a program gets handle **2** (the free-slot scan
  starts at `VDI_PHYS_HANDLE + 1`, `vdi_control.c:420`) — the desktop draws
  on `gl_handle` itself (`deskmain.c:1956`), it holds no virtual
  workstation of its own;
- the workstation is closed (`gsx_wsclose` → `v_clswk`, `gemgsxif.c:191-194`)
  when the session ends (`geminit.c:749`), which closes every virtual
  workstation and leaves the physical one open (`vdi_control.c:573-590`).

**NewTOS binding.** `tos_api::aes::gsxif::gsx_init` / `gsx_wsclose`, called
by the kernel's AES session loop (`kernel/src/lib.rs`) at the
`geminit.c:675`/`:749` positions; `graf_handle` returns
`gsxif::gl_handle()`. `gl_handle` is 0 until the first `gsx_init`, as
EmuTOS's zeroed BSS global is. Two `gsx_init` steps are not VDI calls on
`gl_handle` here: `gsx_setmb` (the button/motion vectors are the AES's
`user_but`/`user_mot`, called directly by the VDI mouse feed,
`tos-api/src/vdi/mouse.rs`) and `vex_wheelv`
(installed once at boot, `aes::init::install_wheel_vector`); `gsx_wsopen`'s
`intout[45] = VsetMode(-1)` is not passed — NewTOS has no `VsetMode` and its
`v_opnwk` switches no mode (`../vdi/workstation/01-v_opnwk.md`). The AES's
own metrics (`gl_wchar`…`gl_hbox`, `gl_nplanes`) are derived from the same
font ring and backend the VDI answers from, not read back through the trap.

### `graf_mouse` (78) — set the mouse cursor

**Source:** `gemsuper.c:294-303` → `gr_mouse` (`gemgrlib.c:419-455`). If the
control manager currently owns the mouse (`gl_ctmown`), the dispatcher
temporarily drops and restores that ownership around the call
(`gemsuper.c:295-302`). Form numbers (`aesdefs.h:423-438`):

| # | Name | Cursor |
|--:|------|--------|
| 0 | `ARROW` | the standard arrow |
| 1 | `TEXT_CRSR` | text I-beam |
| 2 | `HOURGLASS` | busy-bee / hourglass |
| 3 | `POINT_HAND` | pointing hand |
| 4 | `FLAT_HAND` | open (flat) hand |
| 5 | `THIN_CROSS` | thin crosshair |
| 6 | `THICK_CROSS` | thick crosshair |
| 7 | `OUTLN_CROSS` | outlined crosshair |
| 255 | `USER_DEF` | caller-supplied `MFORM` in `addr_in[0]` |
| 256 | `M_OFF` | hide the cursor (`gsx_moff`) |
| 257 | `M_ON` | show the cursor (`gsx_mon`) |
| 258 | `M_SAVE` † | save current form into the AESPD |
| 259 | `M_RESTORE` † | restore the AESPD-saved form |
| 260 | `M_PREVIOUS` † | restore the previously-used global form |

Forms **0..7** select `mouse_cursor[mode]` from the eight preset MFORMs loaded
at start-up (`geminit.c:124`, `NUM_MOUSE_CURSORS = 8`). **Every number that is
not one of the named modes is the `default:` arm** (`gemgrlib.c:426-429`): a
mode outside `ARROW..OUTLN_CROSS` — 8…254, 261 and up, or negative — is clamped
to `ARROW` ("fail safe") and that preset is then **installed** by the
`gsx_mfset` the arm falls through to (`:454`). `gr_mouse` is `void` and has no
"unknown number, leave the cursor alone" path; `int_out[0]` is `1` in every
case. `USER_DEF` uses the caller's `MFORM`
(a 74-byte struct: hotspot x/y, planes, bg/fg mask colours, 16 mask + 16 data
words, `aesdefs.h:441-450`). † `M_SAVE`/`M_RESTORE`/`M_PREVIOUS` (258–260) exist
only with `CONF_WITH_GRAF_MOUSE_EXTENSION` (default **1**, AES 3.20+).

### `graf_mkstate` (79) — poll input

**Source:** `gemsuper.c:304-306` → `gr_mkstate` (`gemgrlib.c:458-464`). Returns
the **instantaneous** input state (no wait): `int_out[1]`=mouse x (`xrat`),
`[2]`=mouse y (`yrat`), `[3]`=button state (`button`), `[4]`=keyboard shift
state (`kstate`). These are the same live input mirrors the event manager reads
([`_state.md`](./_state.md) §6). `kstate` is `gsx_kstate()` = `Kbshift(-1) &
0x000f` (`aes/gemgsxif.c:432-436`, `vdi/vdi_input.c:183-187`): bits 0-3 only —
CapsLock + Alt reads `K_ALT` (0x08) — see [`event.md`](./event.md)
"`kstate` carries bits 0-3 only" (A126).

---

## The `WM_*` messages (generated by the control manager, not by `wind_*`)

A user action on a window gadget does **not** change the window — it posts a
16-byte message to the owner, who then calls the matching `wind_*` to act.
Built by `ct_msgup`/`hctl_window`/`handle_arrow_msg`/`perform_untop` →
`ap_sendmsg` (`gemctrl.c:81-315`, `gemwmlib.c:851-863`). `ap_sendmsg` always
sets `msg[1] = rlr->p_pid` (sender), `msg[2] = 0` (extra bytes):

| `msg[0]` id | value | `msg[3]` | `msg[4]` | `msg[5]` | `msg[6]` | `msg[7]` | Fired by |
|-------------|------:|----------|----------|----------|----------|----------|----------|
| `WM_REDRAW`  | 20 | wh | rect x | rect y | rect w | rect h | `w_redraw` (`gemwmlib.c:904`) — dirty piece to repaint |
| `WM_TOPPED`  | 21 | wh | 0 | 0 | 0 | 0 | click on an inactive window (`gemctrl.c:178`) |
| `WM_CLOSED`  | 22 | wh | **`WS_CURR` x** | **`WS_CURR` y** | **`WS_CURR` w** | **`WS_CURR` h** | CLOSER released inside (`gemctrl.c:220`); with `HOTCLOSE` **on the press** (`:210-213`). Rect = the window's current border rect, fetched once before the gadget switch (`w_getsize(WS_CURR, w_handle, &t); r_get(&t, &x, &y, &w, &h)`, `:196-197`) and passed unchanged to `ct_msgup(message, owner, wh, x, y, w, h)` (`:314`) — corrected from 0 (A77, 2026-09-20) |
| `WM_FULLED`  | 23 | wh | **`WS_CURR` x** | **`WS_CURR` y** | **`WS_CURR` w** | **`WS_CURR` h** | FULLER released inside (`gemctrl.c:220`); rect as for `WM_CLOSED` (`:196-197`, `:314`) — corrected from 0 (A77, 2026-09-20) |
| `WM_ARROWED` | 24 | wh | **`WA_*` action** | 0 | 0 | 0 | scroll arrow/page click (`gemctrl.c:125`) |
| `WM_HSLID`   | 25 | wh | **0..1000 pos** | **`WS_CURR` y** | **`WS_CURR` w** | **`WS_CURR` h** | h-elevator drag (`gemctrl.c:300-305`). The elevator arm overwrites **`x` alone** — `x = gr_slidebox(gl_awind, cpt-1, cpt, (cpt == W_VELEV))` (`:305`) — so `ct_msgup(message, owner, wh, x, y, w, h)` (`:314`) sends the `WS_CURR` `y, w, h` fetched once before the gadget switch (`:196-197`) in `msg[5..7]` — corrected from 0 (A93, 2026-09-21) |
| `WM_VSLID`   | 26 | wh | **0..1000 pos** | **`WS_CURR` y** | **`WS_CURR` w** | **`WS_CURR` h** | v-elevator drag (`gemctrl.c:300-305`); words as for `WM_HSLID` (`:196-197`, `:305`, `:314`) — corrected from 0 (A93, 2026-09-21) |
| `WM_SIZED`   | 27 | wh | new x | new y | new w | new h | SIZER rubber-resize (`gemctrl.c:259`) |
| `WM_MOVED`   | 28 | wh | new x | new y | new w | new h | MOVER/name drag (`gemctrl.c:238`) |
| `WM_UNTOPPED`| 30 | wh | 0 | 0 | 0 | 0 | previous top is being deactivated (`gemctrl.c:151`) |

> **`msg[1]` (sender) is 1 for every row above except `WM_REDRAW`, which is the
> caller's pid — 0 for a chain member, ≥ 2 for an accessory (2026-09-20,
> A62/A56; widened 2026-10-17, A113).** `ap_sendmsg` always stamps `msg[1] = rlr->p_pid`
> (`gemwmlib.c:855`) — the value depends on *who is running* when the send
> happens, not on the message id. `WM_TOPPED`/`WM_CLOSED`/`WM_FULLED`/
> `WM_ARROWED` (queued case)/`WM_HSLID`/`WM_VSLID`/`WM_SIZED`/`WM_MOVED`/
> `WM_UNTOPPED` are all posted from inside `gemctrl.c`'s `ctlmgr()` — `rlr ==
> ctl_pd`, pid **1** — via `ct_msgup`/`handle_arrow_msg`
> (`gemctrl.c:81-315,111-139`). `WM_REDRAW` is posted by `w_redraw`
> (`gemwmlib.c:904`) from `draw_change`, called synchronously from the
> *caller's own* `wind_open`/`wind_close`/`wind_set(WF_CURRXYWH)` trap
> (`gemwmlib.c:1426,1453,1612`) — `rlr` is the caller itself: pid **0** for a
> chain member, the accessory's pid (≥ 2) when an accessory is the caller
> (widened 2026-10-17, A113).
> A held-down scroll arrow can additionally produce a **non-queued**
> `WM_ARROWED` with `msg[1]` = the reader's pid (0 for the chain) instead of 1, synthesised by the reader's
> own `evnt_mesag` — see [`event.md`](./event.md) §`evnt_mesag` and
> [`application.md`](./application.md) §"AES process identity and pid".

> **`WM_SIZED`/`WM_MOVED` carry the *proposed new border rect* in `msg[4..7]`**
> (x,y,w,h) — the app is expected to `wind_set(WF_CURRXYWH, …)` it (possibly
> after clamping). For `WM_HSLID`/`WM_VSLID`, `msg[4]` is the **new slider
> position 0..1000** the app should `wind_set(WF_HSLIDE/VSLIDE)`; `msg[5..7]`
> are the window's `WS_CURR` `y, w, h` (A93). For `WM_ARROWED`, `msg[4]` is
> the sub-action below.

### `WA_*` arrowed sub-actions (`aesdefs.h:245-253`, `gemctrl.c:63-74`)

`WM_ARROWED`'s `msg[4]` is one of, mapped from the clicked gadget:

| Value | Name | Gadget clicked |
|------:|------|----------------|
| 0 | `WA_UPPAGE` | above the vertical elevator (`W_VSLIDE`) |
| 1 | `WA_DNPAGE` | below the vertical elevator (`W_VELEV`) |
| 2 | `WA_UPLINE` | up arrow (`W_UPARROW`) |
| 3 | `WA_DNLINE` | down arrow (`W_DNARROW`) |
| 4 | `WA_LFPAGE` | left of the horizontal elevator (`W_HSLIDE`) |
| 5 | `WA_RTPAGE` | right of the horizontal elevator (`W_HELEV`) |
| 6 | `WA_LFLINE` | left arrow (`W_LFARROW`) |
| 7 | `WA_RTLINE` | right arrow (`W_RTARROW`) |

`WM_ARROWED` repeats continuously while the button stays down
(`handle_arrow_msg`, `gemctrl.c:111-139`).

**The held arrow is drawn `SELECTED` (A109, 2026-09-22).** `hctl_window`
brackets the call with `ob_change(gl_awind, gadget, selected, TRUE)` …
`handle_arrow_msg(w_handle, cpt)` … `ob_change(gl_awind, gadget, normal,
TRUE)` (`gemctrl.c:271-284`), where `normal = gl_awind[gadget].ob_state &
~SELECTED` and `selected = normal | SELECTED` (`:189-194`) — the only bit that
moves is `SELECTED`. Both `ob_change`s are guarded by `if ((gadget !=
W_HSLIDE) && (gadget != W_VSLIDE))`: a **page** click in the track gets no
feedback, because there `gadget` (what `ob_find` answered, `:188`) is the
slide box while `cpt` was moved on to the elevator (`:262-270`). With the
default `actcolchange = 0` a pressed frame gadget shows as `add_3d_effect`'s
sunk bevel plus the `acttxtmove` glyph nudge (see § "Text placement …, the
pressed `ACT` gadget", A105); the arrow keeps that state for as long as the
button is held — unlike a closer/fuller, which `gr_watchbox` un-selects when
the pointer leaves it (`gemgrlib.c:337-355`). NewTOS drew no arrow feedback at
all before A109; the `WM_ARROWED` cadence is unchanged by it.

> **`WM_NEWTOP` (29) and `WM_ONTOP` (31) are defined but NOT generated by this
> build.** `aesdefs.h:238,240` declares them (they are TOS4/MultiTOS window
> messages), but no code in `aes/` or `desk/` posts them (confirmed by grep of
> the tree). NewTOS should recognise the ids but need not emit them unless it
> later adds MultiTOS-style multi-app window ordering. Open question below.

---

## Inputs / Outputs summary

Inputs arrive in the AESPB arrays. `int_out[0]` is the return code (`RET_CODE`,
initialised 1); geometry results are in `int_out[1..4]`. `addr_in[0]` carries
the object tree (`graf_watchbox`/`graf_slidebox`) or the `MFORM` (`graf_mouse`
USER_DEF) — **not** the `wind_set` `WF_NAME`/`WF_INFO`/`WF_NEWDESK`
string/tree pointer, which travels packed as a LONG in `int_in[2..3]` instead
(D-094). No `addr_out` is used by either manager (only `rsrc_gaddr` uses `addr_out`, see
[`_dispatcher.md`](./_dispatcher.md)). The `WM_*` messages are delivered
**asynchronously through the 16-byte message pipe**, never as a return value.

---

## Error conditions

The window/graphics managers are "trusted" AES code with a small error surface —
no negative TOS `E*` codes, only `TRUE`/`FALSE`/handle/`-1`:

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `wind_create` with all `NUM_WIN` slots in use | `gemwmlib.c:1384` | `-1` |
| `wind_open`/`close`/`delete`/`get`/`set` on an invalid or unused handle | `gemwmlib.c:1395-1401` (`get_pwin`) | `FALSE` (0) |
| `wind_open` on an already-open window | `gemwmlib.c:1418` | `FALSE` |
| `wind_close` on a not-open window | `gemwmlib.c:1446` | `FALSE` |
| `wind_get` unknown field (incl. `WF_KIND`, `WF_NEWDESK`) | `gemwmlib.c:1591` | `FALSE` |
| `wind_set` unknown field | `gemwmlib.c:1736` | `FALSE` |
| `WF_TOP`/`WF_SCREEN`/`WF_DCOLOR` — valid without a handle | `gemwmlib.c:1517-1520` | proceed |
| `graf_slidebox` with a zero-range track | `gemgrlib.c:411` | `0` |

`graf_handle`/`graf_mkstate`/`graf_mouse`/the box trackers have no error path —
they return their value (or 1) unconditionally.

### Fields with no handler (return `FALSE`)

`WF_KIND` (1) is **not** implemented by `wm_get`/`wm_set` in this source — the
kind is set only at `wind_create` and cannot be queried/changed here (both fall
to `default: return FALSE`). Compendium documents `WF_KIND` for both; **this
build does not support it** (discrepancy D-053).

`wind_get` refuses three more fields the same way, and for the same reason —
`wm_get` has no case for them: `WF_NAME` (2), `WF_INFO` (3) and `WF_NEWDESK`
(14), all three of which exist on the `wind_set` side only
(`gemwmlib.c:1660,1668,1685`). A refused field returns from `default:` before
`poutwds` is touched, so nothing is written: not an `int_out` word, and not a
byte of any buffer the caller passed (TASK/A42).

---

## Verification criteria

- **`wind_create` handle / exhaustion:** `wind_create(kind, rect)` returns a
  handle `1..NUM_WIN-1` and records `w_kind=kind`, `w_owner=rlr`,
  `w_hslsiz=w_vslsiz=-1`; the `(NUM_WIN)`-th concurrent create returns `-1`
  (`gemwmlib.c:1373-1384`).
- **`wind_calc(WC_WORK)` subtracts the border:** for `kind = NAME|CLOSER|
  FULLER|MOVER|SIZER|UPARROW|DNARROW|VSLIDE` and a full-screen border rect,
  `wind_calc(WC_WORK, kind, x,y,w,h)` returns `(x+lb, y+tb, w-lb-rb, h-tb-bb)`
  with `tb` including the title band and `rb` the vertical-bar band; the inverse
  `wind_calc(WC_BORDER, kind, work)` grows it back (`gemwmlib.c:1856-1867`).
- **`wind_calc(WC_WORK)` round-trips `WF_WORKXYWH`:** after `wind_open(h,
  rect)`, `wind_get(h, WF_WORKXYWH)` equals `wind_calc(WC_WORK, kind, rect)`
  (both go through `wm_calc`, `gemwmlib.c:1075`).
- **SIZER-only band:** `wind_calc(WC_WORK, SIZER, full)` reserves a right band
  (vertical bar) and a 1-px bottom, not a full bottom band (`gemwmlib.c:1827-
  1834`).
- **`wind_get(WF_FIRSTXYWH/NEXTXYWH)` rectangle list:** for a window fully
  covering the screen with no higher windows, `WF_FIRSTXYWH` returns the whole
  work rect and `WF_NEXTXYWH` returns `w=h=0` (only `g_w`/`g_h` are actually
  zeroed, `gemwmlib.c:1240`); with one higher window overlapping, the walk
  returns up to four pieces **in the fixed `mkpiece` order TOP, LEFT, RIGHT,
  BOTTOM** (`gemwrect.c:30-33` defines the order, the `brkrct` loop
  `gemwrect.c:118-122` appends them in exactly that order and drops any piece
  whose `have_piece[]` flag is false) then `w=h=0`. The rect each covering
  window is subtracted with is that window's **`WS_TRUE`** rect — its border
  rect **plus `DROP_SHADOW_SIZE`** on width and height, not its plain border
  rect (`w_getsize(WS_TRUE, wh, &gl_mkrect.o_gr)` at `gemwrect.c:190`, and the
  window's own list-seed rect fetched the same way at `gemwrect.c:203`; the
  addition itself is `w_getsize`'s `WS_TRUE` branch, `gemwmlib.c:254-262`;
  `DROP_SHADOW_SIZE = 2`, `gemwmlib.c:49`). Because empty pieces are dropped,
  `WF_FIRSTXYWH`'s returned rect need not be the TOP piece — if the covering
  window's `WS_TRUE` rect reaches the broken window's top edge,
  `have_piece[TOP]` is false and the list opens with LEFT, RIGHT, or BOTTOM
  instead.
- **`wind_get(WF_TOP)` needs no handle:** `wind_get(0, WF_TOP)` returns the top
  window handle (or `DESKWH`=0) even for an invalid non-zero handle
  (`gemwmlib.c:1517,1553`).
- **`wind_get(0, WF_WXYWH)` is `gl_rfull` until the desktop handle is topped:**
  `wm_init` stores `gl_rfull` (`gemwmlib.c:1294`); after `wind_set(0, WF_TOP)`
  it answers `wm_calc(WC_WORK, NONE, gl_rscreen)` = `(1, 1, W−2, H−2)` and
  the call itself changes no pixel and queues no message (`wm_mktop(0)`,
  `:1605-1613,1069-1086`; see `wind_set` § "`WF_TOP` on the desktop handle",
  A80, 2026-09-20). Topping a created-but-unopened handle is undefined in
  EmuTOS (`gemobjop.c:145-160`); NewTOS: `TRUE`, nothing else happens.
- **`wind_set(WF_HSLIDE)` clamp + change-guard:** `wind_set(h, WF_HSLIDE, 0)`
  clamps to 1; `2000` clamps to 1000; setting the current value again causes no
  redraw (`gemwmlib.c:1652-1655,1705`).
- **`wind_set(WF_NAME)` redraws that one window's name gadget, if it is
  open** (`w_cpwalk`, `gemwmlib.c:1665,1754-1755`; corrected 2026-09-20, A82
  — not "all windows", see "Redraw scope" under `wind_set` (105) above),
  while `wind_set(WF_VSLIDE)` redraws only the slider subtree of that same
  window and — non-3D — only if topped (`gemwmlib.c:1747-1751`).
- **`wind_find` desktop fallthrough:** `wind_find(x,y)` over empty desktop
  returns `0` (`gemwmlib.c:1769`).
- **`graf_handle`:** returns `gl_handle` (the AES VDI workstation handle) in
  `int_out[0]` and non-zero `gl_wchar/gl_hchar/gl_wbox/gl_hbox` in
  `int_out[1..4]` (`gemsuper.c:288-292`).
- **`graf_mouse(2)` shows the hourglass;** `graf_mouse(0)` the arrow;
  `graf_mouse(255, mform)` the caller's shape; `graf_mouse(256)` hides it
  (`gemgrlib.c:424-454`).
- **`graf_mkstate`** returns the live `xrat/yrat/button/kstate`
  (`gemgrlib.c:460-463`); `kstate` has bits 0-3 only (A126).
- **`graf_slidebox`** returns a value in `0..1000` (or `0` for a zero-range
  track) (`gemgrlib.c:409-412`).
- **Active-title cue:** the topped window's `W_NAME` `te_color` is `0x11a1`
  (`IP_2PATT` dither); an untopped window's is `0x1100` (hollow)
  (`gemwmlib.c:150-151,539,780`).
- **`WM_SIZED` payload:** dragging the SIZER posts `msg[0]=27, msg[3]=wh,
  msg[4..7]=` proposed new border rect (`gemctrl.c:259,314`).

---

## Edge cases discovered in source

1. **The active-window cue is the `IP_2PATT` dither on the topped title, plain
   on untopped** — `TOPPED_COLOR=0x11a1` vs `UNTOPPED_COLOR=0x1100`
   (`gemwmlib.c:150-151`). In the non-3D build, CLOSER/FULLER/scroll gadgets are
   built **only for the topped window** (`if (istop)`, `gemwmlib.c:758-773`), so
   an inactive window shows just its (plain) title and box.
2. **`WF_FIRSTXYWH`/`WF_NEXTXYWH` clip each owner rect to the *work* area** and
   skip empty pieces; the walk **terminates with a `w=h=0` rect**, not an error
   (`w_owns`, `gemwmlib.c:1230-1241`) — but the terminating write sets **only
   `g_w = g_h = 0`** (`gemwmlib.c:1240`); `g_x`/`g_y` are left **unwritten**
   by `w_owns`, not zeroed — and "unwritten" means the caller gets the
   dispatcher's local `int_out` words, never its own (`xif` copies `OUT_LEN`
   words back unconditionally, `gemsuper.c:432,444-445`): stack on EmuTOS,
   0 on NewTOS (`docs/COMPATIBILITY.md`, A27). A redraw handler must loop on
   `w==0 && h==0` and must not read `g_x`/`g_y` off that sentinel rect.
3. **`WF_KIND` (1) is not queryable/settable** in this build — both `wm_get`
   and `wm_set` fall through to `return FALSE` (D-053).
4. **`WF_NEWDESK` (14) is set-only;** `WF_COLOR`/`WF_DCOLOR` use a **3-word
   input** (the only `wind_get`/`set` exception), and `WF_DCOLOR` needs no valid
   handle and never redraws (`gemwmlib.c:1491-1503,1727-1734`). See "`WF_COLOR`/
   `WF_DCOLOR` → pixel mapping" under "NewTOS implementation notes" below for
   how the two colour words each field carries map onto rendered pixels
   (A79, 2026-09-20).
5. **Slider setters early-out with no redraw when the value is unchanged**
   (`if (pwin->w_hslide != pinwds[0])`, `gemwmlib.c:1705`); and, non-3D, sliders
   redraw **only on the topped window** (`gemwmlib.c:1747`). With 3D objects
   (default) they always redraw. The redraw itself is scoped to the
   `W_HSLIDE`/`W_VSLIDE` subtree (slider + elevator), not the arrow gadgets —
   see "Redraw scope" under `wind_set` (105) above. NewTOS reproduces the
   change-guard (A84, `TASK/A84.md`, 2026-09-20) and the subtree scope
   (A98, `TASK/A98.md`, 2026-09-21; pinned by `tos-api/tests/aes_window_slider_guard.rs`
   and `tos-api/tests/aes_window_slider_subtree.rs`).
6. **`WM_NEWTOP` (29) and `WM_ONTOP` (31) are declared but never emitted** by
   EmuTOS's window/desktop code — TOS4 messages this build does not generate.
7. **`WM_ARROWED` repeats while the button is held** (`handle_arrow_msg`); the
   position messages `WM_HSLID`/`WM_VSLID` carry a **0..1000** value, and
   `WM_SIZED`/`WM_MOVED` carry the **proposed new border rect** (`gemctrl.c:111-
   139,238,259,300`).
8. **`graf_watchbox` returns `TRUE` when released inside, `FALSE` when
   released outside** (D-096; a previous spec revision had this backwards).
   CLOSER/FULLER fire their message exactly when `gr_watchbox` returns `TRUE`
   (button up inside the gadget) (`gemgrlib.c:337-355`, `gemctrl.c:218-220`)
   — except a `HOTCLOSE` closer, which fires `WM_CLOSED` on the press and
   fakes the button up (`gemctrl.c:208-214`, `:86-96`; kind-bit table).
9. **`graf_growbox`/`graf_shrinkbox` take the big rect at `int_in[4..7]`
   (`GR_I5`), one word after the small rect at `int_in[0..3]`**
   (`gemsuper.c:275-279`) — a slot layout easy to get wrong.
10. **`wind_delete` closes an open window first;** `wind_close` keeps
    `VF_INUSE` (reopenable); `wind_new` releases all locks and closes+deletes
    every window (crash cleanup) (`gemwmlib.c:1473,1454,1875`).
11. **`draw_change` (the redraw driver) computes each window's work rect via
    `wm_calc(WC_WORK,…)`** so `WF_WORKXYWH` and `wind_calc(WC_WORK,…)` always
    agree (`gemwmlib.c:1075`).
12. **The mouse is dropped/restored around `graf_mouse` if the control manager
    owns it** (`gl_ctmown`, `gemsuper.c:295-302`), so a busy-cursor change from
    an app doesn't fight the ctlmgr.
13. **The "slider creep" rounding fix is only half-applied in the default
    build** (D-098): `gr_slidebox` and the non-3D `w_bldvbar`/`w_bldhbar`
    round (`mul_div_round`); the 3D-object (default) `w_bldvbar`/`w_bldhbar`
    still truncate (`mul_div`) — a genuine, still-present EmuTOS source gap in
    reference-level (non-contract) elevator-drawing code.
14. **`wind_set`'s `WF_NAME`/`WF_INFO`/`WF_NEWDESK` pointer travels packed in
    `int_in[2..3]`, not via `addr_in[0]`** (D-094) — the `WM_IOTITLE` macro is
    defined but dead code, referenced nowhere in the EmuTOS tree.
15. **`wind_get(WF_COLOR/WF_DCOLOR)` writes the two colour words to
    `int_out[2]`/`int_out[3]`, not `int_out[1]`/`int_out[2]`** (D-095) —
    `int_out[1]` and `int_out[4]` are written by nobody, and the caller does
    **not** keep its own value there: `xif`'s unconditional copy-back delivers
    the dispatcher's local word — stack on EmuTOS, 0 on NewTOS
    (`docs/COMPATIBILITY.md`, A27). gemlib copies that `int_out[1]` back over
    the application's gadget variable (`a_wind_get.c:366,372`).
16. **`WF_NEWDESK`'s root companion word moves with the pointer convention in
    use, not a fixed `int_in` slot** (A81). EmuTOS's `pinwds[2]` = `int_in[4]`
    (item 14/D-094) is correct only for its one, 2-word (68k) packed pointer.
    NewTOS accepts two conventions (see "Pointers in `addr_in[]`" under
    "NewTOS implementation notes" above for the full rule): the native form
    (`addr_in[0]` = tree, `control[3] >= 1`) puts the root at `int_in[2]`;
    the packed form (`control[3] == 0`) puts it at `int_in[2 + ptr_words]` —
    `int_in[4]` for a 2-word (ILP32) pointer, `int_in[6]` for a 4-word (LP64)
    one. Before A81 the handler read a fixed `int_in[4]` for every caller
    width, which for an LP64-packed caller aliases into the pointer's own
    upper half (`int_in[2..5]`) and silently corrupts it whenever the root is
    non-zero — measured with a tree at `0x0000_0001_0177_E4C0` and root 2
    reassembling as `0x0000_0002_0177_E4C0` (`TASK/A81.md`).

---

## Dependencies

### Requires (must be initialized / functional)

- **The Object Manager** ([`object-form.md`](./object-form.md)) — the frame is
  the `W_ACTIVE` OBJECT tree; `ob_draw`/`ob_find`/`ob_change`/`ob_order`/
  `ob_actxywh`/`ob_relxywh`/`ob_offset` do all frame drawing, hit-testing
  (`wind_find`, `gr_watchbox`), and z-ordering (`WF_TOP`). `graf_watchbox`/
  `graf_slidebox` operate on caller OBJECT trees.
- **The VDI** — `graf_handle` hands out the AES's VDI workstation handle; the
  XOR box trackers (`gr_setup`/`gsx_xbox`), the frame draw (`gsx_sclip`/
  `ob_draw`), the window blit (`bb_screen`), and the mouse form (`gsx_mfset`/
  `gsx_moff`/`gsx_mon`) are VDI/line-A operations. `gl_wchar/hchar/wbox/hbox`
  come from VDI font metrics (`gemgraf.c:405-415`).
- **The Event Manager** ([`event.md`](./event.md)) — `WM_*` messages are
  delivered through the 16-byte pipe `evnt_mesag`/`evnt_multi` read; the trackers
  block on `ev_multi(MU_BUTTON|MU_M1|…)`; `graf_mkstate` reads the same input
  mirrors (`xrat`/`yrat`/`button`/`kstate`).
- **The Control Manager** (`gemctrl.c`) — turns clicks on window gadgets into
  the `WM_*` messages and drives `w_setactive`/`ct_chgown` for topping.
- **The AES process model / screen lock** ([`_state.md`](./_state.md)) — `rlr`
  (window owner), `wind_spb` (the `wind_update` mutex), `fm_own`
  (mouse-control ownership), `ap_sendmsg` → `ap_rdwr`.

### Used by

- **Every GEM application with a window** calls `wind_create`/`wind_open` at
  start, handles `WM_REDRAW`/`WM_SIZED`/`WM_MOVED`/`WM_CLOSED`/`WM_ARROWED`/
  `WM_*SLID` in its event loop (each answered by the matching `wind_set`), and
  `wind_close`/`wind_delete` at exit; it walks `WF_FIRSTXYWH`/`NEXTXYWH` inside
  every `WM_REDRAW`.
- **The desktop** (`programs/desktop/src/`) owns the desktop window (handle 0) and
  registers its object tree as the AES screen background via
  `wind_set(DESKWH, WF_NEWDESK, g_screen, 1, DROOT)` at startup (un-registered
  with a NULL tree on app-launch / exit). This is **implemented**
  (`tos-api/src/aes/window.rs`: `WF_NEWDESK` stores `gl_newdesk`/`gl_newroot`;
  `paint_bg_clip` is NewTOS's `w_drawdesk`, drawing the registered `DROOT`
  background box + icon children at `MAX_DEPTH`, or a white `CLEAR_SCREEN`
  fallback root when none is registered — no `objc_draw`-bypassing painter hook).
  See [`../desktop/aes-background-screen-model.md`](../desktop/aes-background-screen-model.md).
- **`graf_handle`** is the mandatory bridge every app crosses to reach the VDI.

---

## Related data structures

- **`WINDOW`** (`gemlib.h:66-86`, `D.w_win[NUM_WIN]`) — the private per-window
  record: `w_flags` (`VF_INUSE`/`VF_ISOPEN`/`VF_BROKEN`, `gemlib.h:60-63`),
  `w_owner`, `w_kind`, `w_pname`/`w_pinfo`, the FULL/WORK/PREV/CURR rects,
  `w_hslide`/`w_vslide`/`w_hslsiz`/`w_vslsiz`, and the rectangle-list pointers
  `w_rlist`/`w_rnext`. **Reference** — NewTOS reimplements the storage (linked
  `ORECT` nodes vs. any other structure); **not** reference is the sequence of
  values that storage must yield through `wind_get(WF_FIRSTXYWH/NEXTXYWH)` —
  see the corrected contract and worked example under "`wind_get` (104)" §
  "The rectangle-list iteration" above (2026-09-12).
- **`ORECT`** (`struct.h`, free-listed by `gemwrect.c`, `NUM_ORECT = NUM_WIN·10`)
  — a link + `GRECT` node in the owner rectangle list. **Reference.**
- **Window-kind bit table** / **`WF_*` field-number table** / **`WM_*` message
  layout table** / **`WA_*` sub-action table** / **`graf_mouse` form-number
  table** — the contract tables above (`aesdefs.h:226-253,390-562`).
- **`MFORM`** (`aesdefs.h:441-450`, 74 bytes) — `graf_mouse(USER_DEF)` cursor:
  hotspot x/y, planes, mask/data colours, 16 mask + 16 data words. **Contract**
  (a struct a program passes).
- **The 16-byte GEM message** (`WORD msg[8]`) — shared with the event and menu
  managers ([`_state.md`](./_state.md) §3, [`event.md`](./event.md)).
- **`GRECT`** (`{WORD g_x,g_y,g_w,g_h}`) — the universal rect all window/graf
  calls exchange.

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of the opcode numbers
  (100–109, 70–79), the `WF_*` field numbers, the `WC_BORDER`/`WC_WORK` border
  model, the `WF_FIRSTXYWH`/`NEXTXYWH` rectangle-list contract, the `WM_*`
  message word layouts, the `WA_*` sub-actions, and the `graf_mouse` form
  numbers is **deferred** until the PDFs are available. Two items to confirm
  against Compendium when available: **`WF_KIND` support** (Compendium documents
  it; this build returns `FALSE` — D-053) and **`WM_NEWTOP`/`WM_ONTOP`
  generation** (defined but not emitted here).

### AES managers

- [`object-form.md`](./object-form.md) — the frame is an OBJECT tree; `objc_draw`
  (`ob_draw`), `objc_find` (`ob_find`), `objc_change` do the drawing/hit-testing;
  `graf_watchbox`/`graf_slidebox` act on caller trees.
- [`event.md`](./event.md) — `evnt_mesag`/`evnt_multi` deliver `WM_*`; the box
  trackers block on `ev_multi`; `graf_mkstate` mirrors the live input state.
- [`menu.md`](./menu.md) — the menu bar sits above `gl_rfull` (top `gl_hbox`
  row); `graf_handle`'s `gl_hchar`/`gl_hbox` size that bar.
- [`_state.md`](./_state.md) — the window list `D.w_win[]`/`gl_wtop`, the
  rectangle list, the input mirrors, the message pipe.
- [`_dispatcher.md`](./_dispatcher.md) — selector-200 call model, `control[]`
  counts, `int_out[0]`=`RET_CODE` return, results in `int_out[1..4]`.

### VDI

- `graf_handle` returns the VDI workstation handle; the XOR rubber-band, the
  window blit (`bb_screen` = `vro_cpyfm`-class), and the mouse form
  (`vsc_form`) are VDI operations. Do not edit the VDI docs — cross-reference
  only (VDI Phase 12).

---

## Configuration / version notes

- **`CONF_WITH_3D_OBJECTS`** (default **1**, `include/config.h:1259`) changes the frame
  builder (`w_bldactive`) and the `wind_calc` border thicknesses (adds
  `2·ADJ3DSTD` per gadget band via `adj_wbox`/`adj_hbox`), and makes scroll
  sliders always visible (so `wind_set(WF_*SLIDE)` redraws regardless of topped
  state). With it off, the classic TOS1-3 thin borders apply and inactive
  windows omit their gadgets.
- **`CONF_WITH_WINDOW_COLOURS`** (default **1**, `include/config.h:1329`) enables
  `WF_COLOR`/`WF_DCOLOR` (18/19) and the per-window `w_tcolor`/`w_bcolor`
  arrays; with it off those fields fall to `default: return FALSE`, and the
  topped/untopped title colour comes from the fixed `TOPPED_COLOR`/
  `UNTOPPED_COLOR`.
- **`CONF_WITH_GRAF_MOUSE_EXTENSION`** (default **1**, `include/config.h:1283`) enables
  `graf_mouse` forms `M_SAVE`/`M_RESTORE`/`M_PREVIOUS` (258–260, AES 3.20+).
- **`CONF_WITH_PCGEM`** (default **1**, `include/config.h:1320`) enables the `HOTCLOSE`
  (0x1000) kind bit (close on single click).
- **`NUM_WIN`** is **16** when `AES_VERSION > 0x0320` (the default full build is
  `AES_VERSION = 0x0340`), else **8** (`sysconf.h:31-36`). `WF_OWNER` (20)
  exists only for `AES_VERSION >= 0x330`.
- Opcodes 100–109 and 70–79 are unconditional across all builds; only the
  *fields*/*forms* above are gated. `graf_watchbox`/`graf_slidebox`/`graf_mouse`
  gain minor `CONF_WITH_3D_OBJECTS`/`MENU_EXTENSION` adjustments internally but
  keep their ABI.

---

## NewTOS implementation notes

(Empty during Phase 00.)

- **The rectangle list is the core invariant to get right, and there is no
  free-form region algorithm that gets it right by construction (corrected
  2026-09-12).** A redraw handler walks `WF_FIRSTXYWH`/`WF_NEXTXYWH` and clips
  every VDI draw to each visible piece; if the decomposition is wrong, windows
  paint over each other — the classic redraw bug. This is **not** merely "any
  decomposition of the same visible area": EmuTOS's own algorithm produces a
  **different piece count** for the same three overlapping windows depending
  on the order they were opened in (four pieces vs. five — worked out from
  source under "`wind_get` (104)" § "The rectangle-list iteration" /
  "Worked example" above, measured cases 528/549/561). NewTOS must reproduce,
  for a given window and a given set of higher windows in a given z-order, the
  exact ordered piece sequence that results from subtracting each covering
  window's true rect **one at a time, in ascending z-order** (never handle/
  slot-array order) — with up to four TOP/LEFT/RIGHT/BOTTOM sub-pieces per cut,
  in that order. NewTOS is free to implement this with any internal data
  structure (a `Vec`-backed region list, a proper rectangle-difference
  algorithm, …) — the EmuTOS `ORECT` free-list allocator itself
  (`or_start`/`get_orect`, `gemwrect.c:40-61`) is reference — as long as the
  *algorithm's result* (subtraction order and resulting boundaries/count)
  matches. `tos-api/src/aes/window.rs:788-792` currently subtracts covering
  windows in window-slot order, not ascending `z`; fixing that is a Developer
  task, not covered by this spec correction.
- **`WF_COLOR`/`WF_DCOLOR` → pixel mapping (design, A79, 2026-09-20).** `wind_set`'s
  redraw *scope* for `WF_COLOR` is already right on both sides — EmuTOS's
  `w_cpwalk(w_handle, gadget, MAX_DEPTH, TRUE)` (`gemwmlib.c:1754-1755`)
  repaints only the calling window (`do_walk` iterates
  `D.w_win[wh].w_rlist`, `gemwmlib.c:301`, using the same `wh`), and NewTOS's
  `wind_set(WF_COLOR)` already calls `redraw_open(handle)` →
  `draw_frame_pieces` for that one window (`window.rs:3464,3487-3502,:1031`).
  The gap A79 found is that `draw_frame`'s fill/text calls never read
  `w.tcolor`/`w.bcolor` at all — every region was a fixed constant. This bullet
  is the wiring design; it was implemented on 2026-09-20 (`gadget_ink` /
  `draw_box_ink` in `window.rs`) and the look question it left open is
  **decided (user decision, 2026-09-20): the literal rendering of the colour
  words is the contract** — every gadget is painted from its stored word
  exactly as `just_draw` paints the `W_ACTIVE` tree, with no per-gadget
  constant standing in for a default word. The compiled defaults therefore
  render EmuTOS's own window look: white (hollow) faces and bands, 1 px black
  gadget borders, the `IP_2PATT` stipple on the topped name, `IP_1PATT` slider
  tracks, the bevel on every `FL3DACT` gadget. The evidence is EmuTOS itself
  on an emulated Falcon030 (Hatari, the pinned source's own `etos512us.img`,
  truecolor **and** 256 colours):
  `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/` (README + `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/shots/`);
  NewTOS's rendering next to it: `measurements/measurement-2026-09-20-a79/screendumps/
  final-compare-ref-vs-newtos.png`, remaining pixel differences (bevel width,
  name-box bevel, glyph centring, one-row/one-column bar geometry, the
  `DISABLED` overlay) listed in
  `measurements/measurement-2026-09-20-a79/RUNINFO.TXT` § FINAL → A95; item (6) there, the
  old top's repaint on `wind_open`, is fixed by A94 (see the `wind_open`
  entry). The one fixed
  colour left in the frame is the drop shadow (`DROP_SHADOW_COLOR`, a separate
  documented look decision, see the table in (3)).

  **(1) What each field of a gadget's colour word governs.** `gr_crack`
  (`gemgraf.c:612-628`) splits a colour word into border colour `bc` (bits
  15-12), text colour `tc` (11-8), mode `md` (bit 7: 0 = `MD_TRANS`, 1 =
  `MD_REPLACE`), interior pattern `ip` (6-4: `IP_HOLLOW`=0 … `IP_SOLID`=7,
  `obdefs.h:32-39`), interior colour `ic` (3-0, named `WHITE`=0 … `LMAGENTA`=15,
  `obdefs.h:109-124`). `just_draw` (`gemoblib.c:471-953`) reads a gadget's word
  from `ob_spec`'s low 16 bits for `G_BOX`/`G_BOXCHAR`/`G_IBOX` (cracked at
  `:614`) or from the indirect `TEDINFO.te_color` for `G_BOXTEXT` (cracked at
  `:568`); `w_setcolor` writes into whichever one the gadget uses
  (`gemwmlib.c:382-396`, `istop ? w_tcolor : w_bcolor`). For every gadget type
  that reaches the shared border+fill code (`gemoblib.c:660-696`):
  - `bc` → the object's border stroke, drawn only if `th != 0`
    (`gr_box`, `:663-667`). `th` comes from the object's *type*, not the
    colour word (`ob_sst`, `gemobjop.c:41-66`): `G_BOXTEXT` → `te_thickness`
    (always 1 for `W_NAME`/`W_INFO`, `gemwmlib.c:154-157,180` `te_thickness`
    field, `obdefs.h:180`); `G_BOX`/`G_BOXCHAR`/`G_IBOX` → the byte at
    `ob_spec+1`, i.e. `gl_waspec`'s compiled thickness (`gemwmlib.c:84-106`).
    **`W_BOX` is the one gadget where `bc` also colours the drop shadow** —
    the `SHADOWED` branch reuses the same `bcol` variable
    (`gemoblib.c:894-901`).
  - `ic`+`ip` → the interior fill, drawn only for non-`G_IBOX` types
    (`gemoblib.c:669-694`, `gr_rect`, `gemgraf.c:547-559`): `ip=IP_HOLLOW`
    is an **opaque white fill** (`FIS_HOLLOW` in `MD_REPLACE`: the VDI's
    hollow pattern is all-zero, so every pixel takes the background pen —
    corrected 2026-09-20; an earlier revision of this bullet said "paints
    nothing", and the "→ no paint" cells of table (3) are that reading);
    `ip=IP_SOLID`(7) paints `ic` at 100 % coverage (`FIS_SOLID`); `ip=1..6`
    paints the VDI dither of that index in `ic` on white (`FIS_PATTERN`;
    `IP_2PATT` = `[0x0000,0x5555,0x0000,0x5555]` — ink on odd rows at odd
    columns, `IP_1PATT` = `[0x0000,0x4444,0x0000,0x1111]`, both confirmed
    against the Hatari reference, `tos-api/tests/aes_window_color_words.rs`). The
    fill is issued with a **hardcoded** `MD_REPLACE` (`gr_rect`,
    `gemgraf.c:557-558`) regardless of `md` — `md` never affects the fill's
    opacity, only the later text draw.
  - **3D substitution (dormant by default).** If `(ip==IP_HOLLOW) &&
    (ic==WHITE)`, `just_draw` silently substitutes `ip=IP_SOLID` and `ic` = the
    gadget's `FL3DMASK` role colour — `indbutcol` (`FL3DIND`), `actbutcol`
    (`FL3DACT`), `backgrcol` (`FL3DBAK`) (`gemoblib.c:619-636` for
    `G_BOX`/`G_BOXCHAR`; `:582-600`, always `backgrcol`, for text types).
    `init_3d` sets all three to `LWHITE` (index 8) once the palette has more
    than 8 colours (`gemoblib.c:1336-1343`) — on our (truecolor) profile,
    exactly `GRAY_LT` (`tos-api/src/aes/window.rs:165`). **But none of the 19 gadgets' compiled default colour
    words have `ic==WHITE`** (table below — every default is
    `0x1101`/`0x1111`/`0x11a1`, `ic` always 1); the substitution only fires if
    a colour word with `ic=0,ip=0` is explicitly written via
    `WF_COLOR`/`WF_DCOLOR`. **Before the fix, hardcoded `GRAY_LT` gadget-face
    fills in `draw_frame` reproduced the *substituted* colour
    (`LWHITE`=8=`GRAY_LT`) unconditionally — not the literal default word,
    whose `IP_HOLLOW` interior is an opaque white fill (`gr_rect` issues
    `FIS_HOLLOW` in `MD_REPLACE`, `gemgraf.c:552-558`; the VDI's hollow
    pattern is all-zero → the background pen).** A coincidence, not the same
    thing — and decided (2026-09-20): the grey fills are gone, the face
    colour is the word's `ip`/`ic` (hollow = white for every compiled
    default); the substitution above is implemented (`ink_of`) and fires only
    for an application word with `ic == WHITE`.
  - `tc`+`md` → the gadget's text/glyph, painted via `gsx_attr(TRUE, tmode,
    tcol)` (`gemoblib.c:709`) before the final `switch` draws the string
    (`G_BOXTEXT`/`G_TEXT`, `:732-733`) or the single glyph char (`G_BOXCHAR`,
    `:722-731`). `md=MD_TRANS` leaves the already-painted fill/background
    between glyph strokes; `md=MD_REPLACE` makes the VDI paint each glyph
    cell's background white — needed for the *topped* `W_NAME` default
    (`0x11a1`, `md`=1), and is why `draw_frame` already paints a small white
    rectangle behind the name before the label (cite stale 2026-10-26: tos-api/src/aes/window.rs:2403-2409 — an
    approximation of per-glyph opaque-mode background with one rectangle; not
    documented today as tracking `md`). **For a colour word with `md=0` (e.g.
    this ticket's T7 word `0x1172`) that white-box step must not run** —
    transparent mode means the fill colour shows through between glyph
    strokes, not white.
  - `W_DATA`/`W_WORK` are structurally invisible: `th=0` (`gl_waspec` byte 2
    is `0x00`) and both are `G_IBOX`, so neither border nor fill is ever
    drawn, and `w_bldactive` never calls `w_setcolor` for either index
    (`gemwmlib.c:570,603` — `w_adjust` only, no colour call) — their compiled
    `0x1101` never changes at runtime. `WF_COLOR`/`WF_DCOLOR` on gadget 6/7 is
    a pure round-trip **in EmuTOS itself**, not only in NewTOS; no wiring is
    needed there.

  **(2) Per-gadget table** — object type, border thickness, `gl_3dflags` role
  (`gemwmlib.c:108-127`), and the **runtime default** topped/untopped colour
  word (`w_setup` copies `gl_wtcolor`/`gl_wbcolor` into every new window,
  `gemwmlib.c:220-227`; the `wm_start` loop default is `0x1101` for all 19,
  overridden for `W_NAME` (topped only, `|=0xa0` → `0x11a1`) and
  `W_VSLIDE`/`W_HSLIDE` (`|=0x10` → `0x1111`, both topped and — since
  `CONF_WITH_3D_OBJECTS` is on — untopped too, `gemwmlib.c:1347-1358`)):

  | # | Gadget | Type | `th` | 3D role | Default word (T/U) | `bc,tc,md,ip,ic` |
  |--:|---|---|--:|---|---|---|
  | 0 | `W_BOX` | `G_IBOX` | 1 | NONE | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 1 | `W_TITLE` | `G_BOX` | 1 | NONE | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 2 | `W_CLOSER` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 3 | `W_NAME` | `G_BOXTEXT` | 1 (`te_thickness`) | ACT when `kind&NAME` else NONE (`:553`) | **`0x11a1`/`0x1101`** | T:1,1,REPLACE,`IP_2PATT`,1 · U:1,1,TRANS,HOLLOW,1 |
  | 4 | `W_FULLER` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 5 | `W_INFO` | `G_BOXTEXT` | 1 (`te_thickness`) | NONE | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 6 | `W_DATA` | `G_IBOX` | **0** | NONE | `0x1101` (frozen, never `w_setcolor`'d) | n/a — nothing drawn |
  | 7 | `W_WORK` | `G_IBOX` | **0** | NONE | `0x1101` (frozen) | n/a — nothing drawn |
  | 8 | `W_SIZER` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 9 | `W_VBAR` | `G_BOX` | 1 | NONE | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 10 | `W_UPARROW` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 11 | `W_DNARROW` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 12 | `W_VSLIDE` | `G_BOX` | 1 | NONE | **`0x1111`/same** | 1,1,TRANS,`IP_1PATT`,1 |
  | 13 | `W_VELEV` | `G_BOX` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 14 | `W_HBAR` | `G_BOX` | 1 | NONE | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 15 | `W_LFARROW` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 16 | `W_RTARROW` | `G_BOXCHAR` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |
  | 17 | `W_HSLIDE` | `G_BOX` | 1 | NONE | **`0x1111`/same** | 1,1,TRANS,`IP_1PATT`,1 |
  | 18 | `W_HELEV` | `G_BOX` | 1 | ACT | `0x1101`/same | 1,1,TRANS,HOLLOW,1 |

  Every `ACT` gadget also gets the unconditional (colour-word-independent)
  bevel from `add_3d_effect` (`WHITE`/`LBLACK`, inverted when `SELECTED`,
  `gemoblib.c:220-277`, fixed colours at `:252,270-272`) — that part of the 3D
  look is **not** colour-word-driven in EmuTOS and needs no wiring. It is
  **one pixel wide** (corrected 2026-09-21, A95; "2px" here was NewTOS's old
  `bevel`, not EmuTOS): two 1-px polylines inside the `th` border, each
  stopping one pixel short of both ends — `(x, y+h-2) → (x, y) → (x+w-2, y)`
  in WHITE and `(x+1, y+h-1) → (x+w-1, y+h-1) → (x+w-1, y+1)` in LBLACK
  (`gemoblib.c:238-276`, after `gr_inside(&r, th)` `:231-237`), drawn last,
  after the `DISABLED`/`SELECTED` state block (`:949-950`). The `W_NAME`
  box is `FL3DACT` when the window has a `NAME` (`gemwmlib.c:553`) and gets
  it too.

  **Text placement in `W_NAME`/`W_INFO`, the pressed `ACT` gadget (A105,
  2026-09-21).** Both are `G_BOXTEXT` (`gl_watype`, `gemwmlib.c:61-81`)
  with `te_thickness = 1`; `gl_aname` is `TE_CNTR` (`:503`), `gl_ainfo`
  `TE_LEFT` (`gl_asamp`, `:154-157`). `just_draw` hands `gr_gtext` the rect
  **inside the border of the drawn object** — `c = gr_inside(&t, tmpth)`
  (`gemoblib.c:735-738`), `t` being the 3D-grown rect for the `FL3DACT` name
  (`:503-508`) and the plain rect for the `FL3DNONE` info line — and, for an
  `FL3DACT` object with `acttxtmove` set (default 1, `:1332`), first moves
  it: `c.g_y--` unselected, `c.g_x++` selected (`:742-755`). `gr_just`
  (`gemgraf.c:567-589`) then centres vertically by `(c.h - 16 + 1) / 2` and
  horizontally by `(c.w - textw + 1) / 2` (`TE_CNTR`) — each only when there
  is slack — after `gsx_tcalc` (`:480-497`) clamped `textw` to the cell and
  cut the string to the whole glyphs that fit (`numchs = min(n, textw /
  gl_wchar)`; a cell shorter than a glyph draws nothing). Hence: the info
  glyph cells start at `(1, 2)` of the info box (`(17 - 16 + 1) / 2 = 1`
  row of slack split), the name's at `x = name_x - 1 + (name_w + 2 - textw +
  1) / 2`, `y = name_y + 1`; a name wider than `name_w + 2` starts at
  `name_x - 1` (left-aligned, no slack) and shows `(name_w + 2) / 8` glyphs.
  `gl_aname.te_ptext` is the window's name only with a `NAME` in `kind`,
  else `""` (`gemwmlib.c:501`). The horizontal bar's default/minimum elevator
  is `gl_wbox`, the vertical one's `gl_hbox` (`:431-434`, `:469-471`). A
  pressed (`SELECTED`) `FL3DACT` gadget takes `changecol = actcolchange`
  (`gemoblib.c:509-513`; default 0): when set, a hollow interior goes solid
  black, any other takes `xor_color` (`:681-690`), the glyph is drawn in its
  complement in `MD_TRANS` (`:699-706`), and `add_3d_effect` judges its lower
  line against the changed interior colour (`:950`); with it clear the press
  is the sunk bevel and the moved glyph alone. The frame gadgets' default
  word `0x1101` is `IP_HOLLOW` with `ic = BLACK`, so the 3D face
  substitution (`:619-636`, hollow **and** white) does not apply to them —
  their face is `gr_rect`'s hollow white.

  **(3) Mapping onto `draw_frame`'s regions** — the gap as found on
  2026-09-20 **before** the fix (line numbers of that revision; ✓ = the then
  hardcoded pixel already equalled `gr_crack(default word)`; ✗ = it did not).
  Since the fix every region renders the "Default word gives" column (the
  drop-shadow row excepted — see its verdict); the table stays as the record
  of what changed:

  | Region | Call | Gadget(idx)·field | Today's constant | Default word gives | Verdict |
  |---|---|---|---|---|---|
  | Outer border | `rect_outline`, `:2349` | `W_BOX`(0)·`bc` | `BLACK` | `bc=1`=`BLACK` | ✓ |
  | Drop shadow | `draw_shadow`, `:2358` | `W_BOX`(0)·`bc` (EmuTOS reuses it) | `GRAY_LT` (deliberate deviation, `:187-204`) | `bc=1`=`BLACK` | ✗ **already an accepted, documented look decision** — wiring `W_BOX`'s `bc` into `rect_outline` must **not** also feed `draw_shadow` |
  | Title band base | `fill_rect(...,WHITE,...)`, `:2373` | `W_TITLE`(1)·`ip,ic` | `WHITE` | `ip=HOLLOW` → no paint | ✗ undocumented |
  | Title bottom border | `fill_rect(...,BLACK,...)`, `:2418` | `W_TITLE`(1)·`bc` | `BLACK` | `bc=1`=`BLACK` | ✓ |
  | Name stipple/fill | `fill_pattern(TITLE_STIPPLE,...)`, `:2394-2401` | `W_NAME`(3)·`ip,ic,md` (topped only) | fixed pattern-2 black-on-white, `md=Replace` | `ip=IP_2PATT,ic=1,md=REPLACE` (topped default) | ✓ **only for the default word** — hardcoded regardless of what `WF_COLOR` stores; **this is the T7 gap** |
  | Name pre-text white box | `fill_rect(...,WHITE,...)`, `:2408` | `W_NAME`(3)·`md` | unconditional when `istop` | approximates `md=REPLACE`'s opaque glyph background | must become conditional on `md` once wired (see (1)) |
  | Name/info label glyphs | `draw_label`→`draw_glyph(...,BLACK,...)`, `:2334` | `W_NAME`(3)/`W_INFO`(5)·`tc` | `BLACK` | `tc=1`=`BLACK` (both) | ✓ |
  | Info band fill | `fill_rect(...,GRAY_LT,...)`, `:2426` | `W_INFO`(5)·`ip,ic` | `GRAY_LT` | `ip=HOLLOW` → no paint | ✗ undocumented |
  | Info bottom border | `fill_rect(...,BLACK,...)`, `:2433` | `W_INFO`(5)·`bc` | `BLACK` | `bc=1`=`BLACK` | ✓ |
  | Closer/Fuller/Sizer/arrow face | `fill_rect(...,GRAY_LT,...)` in `gadget_sel`, `:2249` | resp. gadget·`ip,ic` | `GRAY_LT` | `ip=HOLLOW` → no paint (dormant 3D substitution would give `LWHITE`=`GRAY_LT`, see (1)) | ✗ undocumented, coincides with the *substituted* colour |
  | …glyph | `draw_glyph(...,BLACK,...)` in `gadget_sel`, `:2253` | resp. gadget·`tc` | `BLACK` | `tc=1`=`BLACK` | ✓ |
  | …border | *(none — `gadget_sel` draws no border line, only fill+bevel+glyph)* | resp. gadget·`bc` | — | `bc=1`=`BLACK` 1px `gr_box` stroke | ✗ EmuTOS draws a 1px black border on every gadget cell in addition to the bevel; `draw_frame` draws no such line |
  | Bevel (2px raised/sunken edge) | `bevel`, `:2258-2269` | *(not colour-word-driven — `add_3d_effect` uses fixed `WHITE`/`LBLACK`)* | `WHITE`/`GRAY_DK` | same mechanism | ✓ correctly independent of `WF_COLOR`, no wiring needed |
  | Vbar/Hbar column background | `fill_rect(...,GRAY_LT,...)`, `:2460,2474` | `W_VBAR`(9)/`W_HBAR`(14)·`ip,ic` | `GRAY_LT` | `ip=HOLLOW` → no paint | ✗ undocumented |
  | V/H-slider track fill | `fill_rect(...,GRAY_LT,...)`, `:2623,2701` | `W_VSLIDE`(12)/`W_HSLIDE`(17)·`ip,ic` | `GRAY_LT` solid | `ip=IP_1PATT,ic=1` → a black dither, **not** solid gray (exact density not verified this session) | ✗ undocumented, style mismatch (solid vs. patterned) even before the shade question |
  | V/H-slider track border | `rect_outline`, `:2624,2702` | `W_VSLIDE`(12)/`W_HSLIDE`(17)·`bc` | `BLACK` | `bc=1`=`BLACK` | ✓ |
  | Elevator (thumb) fill | `fill_rect(...,GRAY_LT,...)`, `:2630,2707` | `W_VELEV`(13)/`W_HELEV`(18)·`ip,ic` | `GRAY_LT` | `ip=HOLLOW` → no paint | ✗ undocumented (coincides with the substituted colour, as above) |
  | Elevator bevel | `bevel`, `:2631,2708` | *(not colour-word-driven)* | — | — | ✓ |
  | Sizer-corner (no `SIZER`) fill+bevel | `:2492-2493` | `W_SIZER`(8)·`ip,ic` + bevel | `GRAY_LT` + bevel | `ip=HOLLOW` → no paint; bevel independent | ✗ fill / ✓ bevel |

  Every ✗ row except the drop shadow was fixed by rendering the word
  literally (decision 2026-09-20, see the head of this bullet): the gadget
  faces, the info band, the bar column/band and the elevators are hollow →
  white, the slider tracks `IP_1PATT`, every gadget cell gets its 1 px `bc`
  border on the 3D-grown slot, and the name's white glyph-cell background is
  the `md=REPLACE` text mode of its word, not a fixed rectangle. A transient
  "keep today's grey while the word equals the compiled default" arm existed
  for one afternoon as a decision aid (`FrameLook`) and was removed with the
  decision; there is no switch and no second look.

  **The contract itself is already correct on both sides — only the
  colour-consuming half of `draw_frame` is missing:**
  - `wind_set(handle, WF_COLOR, gadget, tcol, bcol)` on an **open** window
    redraws **that window's** gadget immediately, clipped to its own visible
    pieces; NewTOS's `redraw_open`/`draw_frame_pieces` already have exactly
    this scope (above). The only change needed is for the fill/text calls
    inside `draw_frame` to read `w.tcolor[g]`/`w.bcolor[g]` (topped/untopped
    selected by `istop`, matching `w_setcolor`'s `istop ? w_tcolor :
    w_bcolor`, `gemwmlib.c:388`) instead of the constants in the table above.
  - `wind_set(_, WF_DCOLOR, gadget, tcol, bcol)` writes only `gl_wtcolor`/
    `gl_wbcolor` and explicitly sets `gadget = -1` **before** the `do_cpwalk`
    check, so **no window is redrawn** (`gemwmlib.c:1727-1734`, comment "do
    not call `w_cpwalk()` in this case"); it changes what `w_setup` copies
    into a window created **afterwards** (`:220-227`) — existing windows are
    unchanged. NewTOS's `wind_set(WF_DCOLOR)` (`tos-api/src/aes/window.rs:3999-4010`) already
    matches this exactly — it writes `gl_wtcolor()`/`gl_wbcolor()` and never
    calls `redraw_open`. **No change needed here beyond `draw_frame`
    consuming the per-window `tcolor`/`bcolor` a window already inherits at
    `wind_create` (`tos-api/src/aes/window.rs:3413-3414`).**

  **(4) Worked examples.** Window `(30,40,320,240)`, kind `0x0FEF`, open and
  **topped** (both colour words are equal in the T7 call, so topped/untopped
  doesn't matter for the visible result here, but an untopped `W_NAME` also
  gets the unrelated `DISABLED` dim-overlay, `gemoblib.c:917-930`, which would
  otherwise complicate the read):
  - **T7 — `wind_set(h, WF_COLOR, 3, 0x1172, 0x1172)`** (`W_NAME`).
    `gr_crack(0x1172)` = `bc=1,tc=1,md=`**`TRANS`**` (bit 7 of the low byte
    `0x72` is 0 — not `MD_REPLACE`; a "mode replace" framing of this word is
    imprecise, corrected here), `ip=7=IP_SOLID,ic=2=RED` (`obdefs.h:111`).
    Expected pixels in the `W_NAME` rect: **interior 100 % `RED`** (`ic=2`,
    `IP_SOLID` forces `FIS_SOLID` regardless of `md`, `gr_rect`,
    `gemgraf.c:552-558`), **1px black border** (`bc=1`), **name text in
    `BLACK`** (`tc=1`) drawn in **transparent** mode — red shows between the
    glyph strokes, no white glyph-cell boxes (`md=0`, unlike the compiled
    topped default `0x11a1`). Before the fix: 0 pixels change (today's
    hardcoded stipple path never reads `tcolor[3]`); after: the whole name
    band renders solid red with black text, no stipple.
  - **A second case — `wind_set(h, WF_COLOR, 12, 0x1171, 0x1171)`**
    (`W_VSLIDE`). `gr_crack(0x1171)` = `bc=1,tc=1,md=TRANS` (bit 7 of `0x71`
    is 0), `ip=7=IP_SOLID,ic=1=BLACK`. Expected pixels in the vertical
    slider's **track** rectangle (`draw_vslider`'s `track`,
    cite stale 2026-10-26: tos-api/src/aes/window.rs:2621-2624): **100 % `BLACK` fill**, border also `BLACK`
    (indistinguishable from the fill). Before the fix: today's hardcoded
    `GRAY_LT` fill (`:2623`) is unchanged; after: the track renders solid
    black.
- **The frame drawing + rubber-band mechanics are reference** (NewTOS
  reimplements `w_bldactive` and the XOR trackers over its own framebuffer XOR
  primitive fed by IKBD/USB-HID mouse position, [[keyboard-via-atari-ikbd]]).
  The **`WF_*` semantics, the `wind_calc` border model, the `WM_*` message
  formats, the rectangle-list contract, the window-kind bits, and every
  `wind_*`/`graf_*` ABI are hard contract.**
- **Bring-up order:** VDI workstation → `graf_handle` (hand out the handle) →
  `wm_init` (desktop = handle 0, `gl_rfull` below the menu row) →
  `wind_create`/`wind_open` → the rectangle list → the `WM_REDRAW` loop. Match
  the [`menu.md`](./menu.md) bar geometry (`gl_hbox` top row).
- **`wind_update` locking** presumes preemptible AES processes; single-app
  bring-up can implement it as a real (if usually-uncontended) screen lock, not
  a stub — its observable contract is "layout frozen while held".
- **Pointers in `addr_in[]`** (OBJECT trees for `graf_watchbox`/
  `graf_slidebox`, the `MFORM` for `graf_mouse`) are application pointers —
  apply the same boundary handling as other AES `addr*` reads; struct pointer
  widths follow the caller ABI (native vs m68k, see
  [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md)). **`wind_set`'s
  `WF_NAME`/`WF_INFO`/`WF_NEWDESK` string/tree pointer travels one of *two*
  ways on NewTOS, discriminated by `control[3]` (`AIN_LEN`) — corrected
  2026-09-20/A81.** A previous revision of this paragraph said the pointer
  "is, on NewTOS, `addr_in[0]`" as if that were the only form; that is the
  form the native desktop and (after A81's gemlib change) the pointer-typed
  bindings use, but the dispatcher (`wf_str_ptr`, `tos-api/src/aes/
  window.rs:2706-2712`) has always accepted a second, packed form as well —
  and for `WF_NEWDESK` the *root* companion word's position differs between
  the two, which the previous text never stated at all.
  - **Native form (`control[3] >= 1`):** the pointer is `addr_in[0]` — the
    same slot as every other AES pointer-class argument; apply the identical
    boundary validation. For `WF_NEWDESK` the root travels in `int_in[2]`
    (the generic first value word, otherwise idle once the pointer moves out
    of `int_in`). This is what the native desktop sends
    (`programs/desktop/src/lib.rs::wind_set_newdesk`: `addr_in[0]` = tree,
    `int_in[2]` = root, dispatched with `IN_LEN 3, OUT_LEN 1, AIN_LEN 1`)
    and is the mapping gemlib's pointer-typed bindings
    (`wind_set_str`/`_ptr`/`_ptr_int`, `sdk/gemlib/overlay/`) are being moved
    to under A81 — not yet landed as of this note; the pristine
    `sdk/gemlib/gemlib-0.44.0/a_wind_set_*.c` still emit the packed form
    below.
  - **Packed form (`control[3] == 0`):** the pointer is packed at the
    caller's own pointer width starting at `int_in[2]` — 2 words (4 bytes)
    for an ILP32 caller, 4 words (8 bytes) for an LP64 caller
    (`intin_ptr`, `tos-api/src/aes/mod.rs:313-335`) — read back through
    `wf_str_ptr`. This generalises the 68k convention below to a 64-bit
    pointer instead of inventing an unrelated slot. **For `WF_NEWDESK` the
    root sits in the word right after the packed pointer**,
    `int_in[2 + ptr_words]`: `int_in[4]` for a 2-word pointer — bit-exact
    with EmuTOS's `pinwds[2]` (D-094 above) — and `int_in[6]` for a 4-word
    (LP64) pointer (`I_SIZE` is 16, so `int_in[6]` is in range).
    **This is the rule A81 established; before it, the handler
    (`tos-api/src/aes/window.rs::set_newdesk`) read a fixed `int_in[4]`
    regardless of caller width.** For an LP64 caller that fixed slot lies
    *inside* the packed pointer's own bytes — bits 32..47 of an 8-byte
    pointer spanning `int_in[2..5]` — and gemlib's `a_wind_set_ptr_int.c`
    writes exactly there (`*(void**)(aes_intin+2) = v; aes_intin[4] = W3;`,
    `sdk/gemlib/gemlib-0.44.0/a_wind_set_ptr_int.c:27-28`), so a non-zero
    root silently overwrote the pointer's upper half instead of landing in
    its own slot — the corruption `TASK/A81.md` measured (tree at
    `0x0000_0001_0177_E4C0`, root 2, reassembled by `intin_ptr` as
    `0x0000_0002_0177_E4C0`). It went unnoticed for a root of 0 (TeraDesk,
    `sdk/apps/teradesk/icon.c:2511`; EmuTOS's own desktop passes `DROOT = 1`,
    `references/emutos/desk/deskmain.c:2049`, `desk/deskwin.h:44`, but never
    through this packed NewTOS path) because that write is a no-op, and
    because the pointer's upper half is 0 to begin with for every EL0
    address on the current stage-1 boot tables, which identity-map only
    `0..4 GiB` of the low-half VA window
    (`kernel/src/arch/aarch64/el0.rs:16-18`) — so the corruption is invisible
    until a non-zero root is packed for an LP64 caller. `a_wind_set_ptr.c`
    (`sdk/gemlib/gemlib-0.44.0/a_wind_set_ptr.c:26-27`, two single-word
    stores) zeroes the same `int_in[4..5]` — the pointer's own upper
    half — with a hardcoded `0` (no `W3`); `a_wind_set_str.c` (`:26`, one
    8-byte `*(const char**)(aes_intin+4) = 0` store) zeroes the wider
    `int_in[4..7]`, which for `WF_NAME`/`WF_INFO` carries no root anyway.
    Both are harmless by the same argument as above (the value written
    happens to match the pointer's already-zero upper half), not because
    the overlap itself is benign.
  This is still not the 68k mapping D-094 documents: on EmuTOS/68k TOS,
  `wm_set(w_handle, w_field, pinwds)` (`gemwmlib.c:1623`) receives the
  pointer packed as a raw LONG across two `int_in` words (`pinwds =
  &int_in[2]`, dispatched at `gemsuper.c:345-346`; read back via `*(char
  **)pinwds`/`*(OBJECT **)pinwds` at `gemwmlib.c:1660-1689`, root at
  `pinwds[2]` = `int_in[4]` for `WF_NEWDESK`, `gemwmlib.c:1688`) — correct
  for the 68k ABI, where the AESPB's `int_in` array holds only 16-bit words
  and a pointer is always 4 bytes. A native 64-bit AArch64 pointer does not
  fit in two `i16` words, so per
  [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md#native-vs-m68k-struct-variants)
  (pointer width follows the caller ABI; the API must not branch on caller
  type) the native convention above is what a freshly-written LP64 caller
  should use (`control[3] >= 1`); the packed convention is retained — at the
  caller's own pointer width — for a classic-ABI caller or any binding that
  still favours it (`control[3] == 0`). Consistent with every other AES
  pointer argument (`menu.rs`, `object.rs`, `appl.rs`), which accept only the
  native form; `wind_set`'s three fields are the one place two forms coexist.

### Open questions

- **`WF_KIND` (1)** is unsupported in this EmuTOS build (returns `FALSE`) but is
  documented by Compendium for both get and set. Does a NewTOS-targeted program
  rely on `wind_get(WF_KIND)`? If so, NewTOS should implement it (trivial —
  `w_kind` is stored) rather than mirror the EmuTOS omission. Decide against
  Compendium when the PDF is available (D-053).
- **`WM_NEWTOP` (29) / `WM_ONTOP` (31)** are defined but never emitted here.
  These are MultiTOS/TOS4 window-ordering messages. NewTOS's window model may
  want to emit them once multi-app window ordering exists; until then, honest
  non-support (recognise the ids, don't fabricate the messages).
- **`WF_OWNER` z-order** is partial in EmuTOS (returns only global top/bottom,
  not the immediate neighbours, `gemwmlib.c:1582-1588`). NewTOS can implement it
  fully; confirm no program depends on the EmuTOS partial behaviour.
- **`WF_OWNER`'s reported pid is a *value*, not a lookup key (2026-09-20,
  A62/A56).** `Window.owner` should keep recording the caller's *identity*
  (GEMDOS slot / kernel-desktop index) exactly as `wind_create` does today;
  what changes is the boundary that answers `wind_get(WF_OWNER)` — it must
  translate that identity to the *reported* value (0 for every application
  window) rather than handing the stored identity back verbatim. See
  [`application.md`](./application.md) §"AES process identity and pid".

### NewTOS implementation notes — the drop shadow's four unwritten pixels (DESK23, 2026-09-24)

**They are EmuTOS's, and nothing writes them.** A window whose frame was drawn
incrementally by `wind_open` and the same window after a full repaint were
measured to differ in two pixels
(`measurements/measurement-2026-09-23-desk06/`, runs `wdef` and `redraw`). The premise that
one of the two draws is wrong does not survive the source.

For a window at `(48,43,544,400)` with `W_BOX`'s border thickness 1
(`ob_spec 0x00011101`, `gemwmlib.c:63,84`), `ob_draw` paints

* the border with `gr_box(t.g_x, t.g_y, t.g_w, t.g_h, 1)` → x 48..591,
  y 43..442, and
* the `SHADOWED` decoration *after* `gr_inside(&t, th)`
  (`aes/gemoblib.c:889-900`):
  `bb_fill(… t.g_x, t.g_y+t.g_h+th, t.g_w+th, 2*th)` → x 49..591, y 443..444
  and `bb_fill(… t.g_x+t.g_w+th, t.g_y, 2*th, t.g_h+3*th)` → x 592..593,
  y 44..444.

Each band starts one pixel *inside* the corresponding edge, so **four** pixels
belong to neither the border nor a band: `(592,43)`, `(593,43)`, `(48,443)`,
`(48,444)`. `draw_frame`/`draw_shadow` in `tos-api/src/aes/window.rs` compute
the identical rectangles.

The desktop does not paint them either: all four lie inside the window's
`WS_TRUE` (the rectangle grown by `DROP_SHADOW_SIZE`, `gemwmlib.c:253-261`),
the rectangle lists are built from `WS_TRUE` (`newrect`, `gemwrect.c:190`),
and `w_drawdesk` walks the **desktop's** list (`do_walk(DESKWH, …)`,
`gemwmlib.c:340-359`). So their colour is simply whatever was on the screen
when the window opened, in EmuTOS as here.

That is what the measured difference was: the incremental picture had the
desktop under it, the repainted picture had the white screen a console program
had left behind. The desktop background is a green/white dither, which is why
only two of the four pixels showed up in the diff — at `(592,43)` and
`(48,443)` the pattern is white anyway and matches the console's white. A
control run of five window opens, toppings and full frame repaints with no
screen clear in between leaves all four on the desktop pattern from beginning
to end (`measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/`, §4, table from
`measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/method/pixels.py`).
 No change was made.
