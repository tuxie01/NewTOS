# AES Screen-Background Ownership Model (desktop vs. AES)

> **Scoped research task** (lead-assigned): extract EmuTOS's model for *who
> draws the raw screen background* — the AES or the desktop application — and
> catalog NewTOS's divergence (the `DESK_PAINTER` function-pointer hook).
> Source: EmuTOS @ `1a7484ab` (`references/emutos/`, same pinned commit as
> `docs/api/desktop/deskmain.md` / `docs/api/aes/window-graphics.md`).
> Primary: `aes/gemshlib.c` (`sh_main`/`sh_draw`/`sh_show`/`sh_ldapp`),
> `aes/gemwmlib.c` (`w_drawdesk`/`draw_change`/`wm_set`/`wm_init`),
> `aes/geminit.c` (`set_aes_background`/`run_accs_and_desktop`),
> `desk/deskmain.c` (`deskmain`), `desk/deskobj.c` (`obj_init`),
> `desk/desksupp.c` (`do_wredraw`), `desk/deskinf.c` (`inf_backgrounds`).
>
> This document is **additive** to two existing extractions and does not
> repeat their tables: [`docs/api/aes/window-graphics.md`](../aes/window-graphics.md)
> already covers the `wind_set`/`WF_NEWDESK` opcode contract (field number,
> pointer-packing correction D-094, `gl_newdesk`/`gl_newroot` globals) and
> [`docs/api/desktop/deskmain.md`](./deskmain.md) already covers `deskmain()`'s
> startup/shutdown sequence and `do_wredraw`'s per-window-vs-desktop root
> resolution. What is new here: the **AES-side** `w_drawdesk()` dual-tree
> mechanism (the actual place the screen background gets painted), the
> **app-launch `CLEAR_SCREEN` flow** (`gemshlib.c`, not covered by either doc
> above), and the concrete tie to our `tos-api`/`kernel` divergence.

---

## 1. Three "ROOT-like" objects — do not conflate them

EmuTOS has **three** distinct background-related objects; the whole model
turns on keeping them apart.

| # | Object | Where it lives | Who owns it | Purpose |
|---|--------|-----------------|-------------|---------|
| A | `rs_trees[DESKTOP][ROOT]` | AES's own compiled-in resource (`gem.rsc`), loaded by `gem_rsc_init()` | **AES** | Persists the current background pattern/colour byte (low byte of `ob_spec`); source for (B)'s snapshot and for the app-launch white-out (§3). Also has a child `DTNAME` (a `TEDINFO` text field) used to show the loading program's name (§3). |
| B | `W_TREE[ROOT]` (`gl_wtree[ROOT]`) | AES's internal window-manager scratch tree (`D.w_awind`/`W_TREE`, `gemwmlib.c:53-58`) | **AES** | The *fallback* background object `w_drawdesk()` draws when no app has registered its own tree (§2) — `ob_type` forced to `G_BOX`, `ob_spec` = a snapshot of (A)'s `ob_spec` (`gemwmlib.c:1269-1271`). |
| C | `G.g_screen[ROOT]` / `G.g_screen[DROOT]` | The **desktop application's own** object tree (`desk/deskobj.c:93`, allocated at desktop startup) | **the desktop app** (a normal AES client) | `ROOT` (object 0) is a full-screen `G_IBOX` anchor (`ob_spec=0`, `deskobj.c:38-40,139-140`); `DROOT` (its first child) is a `G_BOX` carrying the *visible* desktop pattern/colour (`deskobj.c:36-40,142-146`), plus `DROOT+1..DROOT+NUM_WNODES` — one background box per possible window. This tree is what `w_drawdesk()` draws **instead of** (B) once the desktop registers it (§2). |

(A) and (B) are pure **AES internals** the desktop never touches directly.
(C) is built and owned entirely by the desktop, using ordinary heap memory
(`dos_alloc_anyram`, `deskobj.c:86-93`) — it becomes AES-visible only through
the real, trap-callable `wind_set(DESKWH, WF_NEWDESK, …)` call (§2).

**Inferred, not directly confirmed:** `rs_trees[DESKTOP][ROOT]`'s `ob_type`.
`gem.rsc` is a compiled binary resource (`aes/gem.rsc`, `file` reports "data");
its object records are not visible as C source, so the `ob_type` value can't
be grepped. It is drawn as a *visible* filled rectangle in `sh_draw()`
(`ob_draw(tree, ROOT, 0)`, `gemshlib.c:280`) and in `w_drawdesk()`'s fallback
path, so it must be a **drawable** box type (`G_BOX`, not `G_IBOX` — an
invisible box never paints); this is a behavioral inference, not a source
citation. Flagging per protocol rather than asserting it as read.

---

## 2. How the AES draws the background — `w_drawdesk()` (`gemwmlib.c:340-360`)

```c
void w_drawdesk(GRECT *pc)
{
    OBJECT  *tree;
    WORD    depth;
    WORD    root;

    if (gl_newdesk)
    {
        tree = gl_newdesk;      /* the desktop's OWN tree, registered via WF_NEWDESK */
        depth = MAX_DEPTH;      /* draw ROOT *and* every child (icons!) */
        root = gl_newroot;
    }
    else
    {
        tree = gl_wtree;        /* AES's internal fallback (object B above) */
        depth = 0;              /* draw ONLY the flat rectangle, no children */
        root = ROOT;
    }

    do_walk(DESKWH, tree, root, depth, pc);
}
```

`do_walk` (`gemwmlib.c:286-312`) clips to `gl_rfull`, walks window-0's owner
rectangle list (the pieces of the desktop not covered by any window), and for
each piece: `gsx_sclip(&t); ob_draw(tree, obj, depth);` — a **plain
`objc_draw`-style call**, exactly the ordinary object-manager drawing engine
used for every AES form/dialog/window frame. There is **no special-cased
"paint desktop background" function** distinct from `ob_draw`.

This confirms the user's requirement directly: the background is drawn by
`objc_draw`/`ob_draw` of a real `OBJECT` whose `ob_spec` encodes the fill
(border/text-color/pattern/fill-color nibbles, per `gr_crack`,
`aes/gemgraf.c:612-627`), not a bespoke raster routine.

**Call sites of `w_drawdesk`** (both **inside the AES**, never from the
desktop):
- `aes/gemwmlib.c:1218-1219` — inside `draw_change` (the `WF_CXYWH`/`WF_TOP`
  redraw driver, already documented in `window-graphics.md:362-417`), gated
  `if (start == DESKWH)` — i.e. only when a window move/resize/open/close
  actually exposes area down to the desktop level.
- `aes/gemfmlib.c:377` — when a form/dialog/alert ends and needs to expose
  the desktop underneath it.
- (3D-object build only) `aes/gemwmlib.c:1902-1941` `w_redraw_desktop()` —
  same `gl_newdesk` dual-path, used by the control manager to animate 3D
  objects; not gated by `CONF_WITH_3D_OBJECTS=0` builds.

**Why `gl_newdesk` matters for synchronous redraw:** because it points
directly at the desktop's **live, in-memory** `G.g_screen` array, the AES can
draw the desktop's icons **without scheduling/messaging the desktop
process** — `w_drawdesk()` runs synchronously inside whatever process
triggered the window change (any app moving/closing its own window), reading
the desktop's object data directly. This is why exposing desktop area behind
a closing/moving app's window repaints the icons immediately, even though the
desktop process itself may be blocked in `evnt_multi()` at that moment.

### `wm_init()` resets the registration every shell-loop iteration

`aes/gemwmlib.c:1247-1326` (`wm_init`), called from `sh_main()` (§3) before
*every* app or desktop (re)launch:

```c
W_TREE[ROOT].ob_type = G_BOX;
tree = rs_trees[DESKTOP];
W_TREE[ROOT].ob_spec = tree->ob_spec;   /* :1269-1271 — snapshot A → B */
...
gl_newdesk = NULL;                       /* :1300 — un-register any prior tree */
```

So every time the shell loop is about to load something new, the AES's
fallback background (B) is refreshed from the persisted pattern byte (A), and
any previous registration (C) is cleared — the newly-launched thing (desktop
or app) starts with **no** background tree registered, i.e. the AES's own
flat-pattern fallback is what shows until (if it's the desktop) it
re-registers via `WF_NEWDESK`.

### Registering / un-registering — `wind_set(DESKWH, WF_NEWDESK, …)`

This is a **real, trap-callable AES call** (`WF_NEWDESK` = 14,
`include/aesdefs.h:546`; opcode/field contract already in
`window-graphics.md:121-149,362-401`). `wm_set`'s handler (`gemwmlib.c:1685-1689`):

```c
case WF_NEWDESK:
    pwin->w_owner = rlr;
    gl_newdesk = *(OBJECT **) pinwds;
    gl_newroot = pinwds[2];
    break;
```

- **Registration** (`desk/deskmain.c:2049`): `wind_set(DESKWH, WF_NEWDESK,
  G.g_screen, 1, 0)` — called once, **after** `app_blddesk()` has populated
  the desktop's own tree, **before** the desktop's `evnt_multi()` loop starts.
- **Un-registration** (`desk/deskmain.c:2163`): `wind_set(DESKWH, WF_NEWDESK,
  NULL, 0, 0)` — called on desktop exit, **before** `appl_exit()`.

Between registration and un-registration, **every** `w_drawdesk()` call draws
`G.g_screen` at `MAX_DEPTH` — i.e. the desktop's `ROOT`/`DROOT` background box
**and every icon object** parented under it. The desktop never has to redraw
its own background/icons in response to a window move elsewhere — the AES
does it, straight off the desktop's live data.

### The desktop's own redraw (icon content changes) — still a normal trap call

When the desktop itself needs to redraw (e.g. after an icon is
added/removed/moved, or in response to its own `WM_REDRAW` for handle 0), it
calls `desk/desksupp.c:202-232` `do_wredraw(DESKWH, rect)`, which — like any
AES client — calls the **ordinary trap-callable** `objc_draw`:

```c
wind_get_grect(w_handle, WF_FIRSTXYWH, &t);
while (t.g_w && t.g_h) {
    if (rc_intersect(&clip_r, &t))
        objc_draw(G.g_screen, root, MAX_DEPTH, t.g_x, t.g_y, t.g_w, t.g_h);  /* :227 */
    wind_get_grect(w_handle, WF_NEXTXYWH, &t);
}
```

So there are **two** paths that end up drawing the *same* `G.g_screen` tree,
both via `objc_draw`/`ob_draw` — never a bespoke pixel routine:
1. AES-internal, synchronous, triggered by *any* window change elsewhere
   (`w_drawdesk`, §2 above) — for repainting exposed desktop area.
2. Desktop-internal, trap-called, triggered by the desktop's own event loop
   (`do_wredraw` → `objc_draw`) — for repainting after the desktop's own
   state changes (already documented in `docs/api/desktop/desksupp-redraw.md`).

---

## 3. App-launch background = `CLEAR_SCREEN` — the full flow (`aes/gemshlib.c`)

```c
#define CLEAR_SCREEN  ((WHITE<<12) | (WHITE<<8) | (IP_HOLLOW<<4) | WHITE)   /* :64, = 0x00000000 */
```

`sh_main()` (`gemshlib.c:692-731`) is the **outer shell loop** — it runs once
per "next thing to load", whether that's the desktop itself, an autorun app,
a normal user-launched app, or the return-to-desktop after an app quits:

```c
do {
    sh_chdef(psh);                  /* set current dir for the next thing */
    sh_chgrf(psh);                  /* alpha/graphics mode switch */

    if (gl_shgem) {
        wm_init();                  /* :717 — resets gl_newdesk=NULL, resnaps W_TREE[ROOT] */
        ratinit();
        sh_draw(D.s_cmd, TRUE);     /* :719 — THE white-out, see below */
    }
    ...
    rc = sh_ldapp(psh);             /* :725 — actually load/run it */
} while (...);
```

`sh_draw(lcmd, clear)` (`gemshlib.c:269-289`) operates on `rs_trees[DESKTOP]`
(object **A**, the AES's own internal tree — **not** `G.g_screen`):

```c
if (clear) {
    LONG specsave = tree[ROOT].ob_spec;
    tree[ROOT].ob_spec = CLEAR_SCREEN;   /* temporarily force white/hollow */
    ob_draw(tree, ROOT, 0);              /* paint ONLY the root — full screen white */
    tree[ROOT].ob_spec = specsave;       /* restore the persisted pattern byte */
} else {
    TEDINFO *ted = (TEDINFO *)tree[DTNAME].ob_spec;
    ted->te_ptext = (char *)lcmd;        /* set the program-name text field */
    ob_draw(tree, ROOT, MAX_DEPTH);      /* paint ROOT + children (incl. DTNAME) */
}
```

So the flow, in order, for **every** transition (app→desktop, desktop→app,
app→app):

1. **`sh_draw(cmd, TRUE)`** (called unconditionally, `:719`) — full-screen
   white-out: temporarily overwrite `rs_trees[DESKTOP][ROOT].ob_spec` with
   `CLEAR_SCREEN` (= `0x00000000`, since `WHITE=0` and `IP_HOLLOW=0`,
   `include/obdefs.h:32,109`), draw *only* the root (depth 0 — no `DTNAME`
   child), then restore the previous `ob_spec` (the persisted background
   pattern/colour, set via `set_aes_background()`, §4).
2. **`sh_ldapp()`** (`gemshlib.c:592-689`) then, just before actually running
   the program (or the ROM desktop): calls **`sh_show(name)`**
   (`gemshlib.c:292-298`, only if `gl_shgem`) → `sh_draw(name, FALSE)` — this
   redraws the **same** tree, now at `MAX_DEPTH`, with `rs_trees[DESKTOP]`'s
   `ob_spec` back to its *persisted background pattern* value (not white —
   the temporary override was already undone in step 1) **plus** the
   program's filename in the `DTNAME` text field. This is the brief
   "loading `PROGRAM.PRG`" flash over the background-pattern-filled screen
   real Atari TOS users see between apps.
   - For the desktop itself, `sh_show("")` is called (`gemshlib.c:603`,
     "like TOS, we don't display a name").
3. `dos_exec()` (or, for the desktop, `aes_run_rom_program(deskstart)`)
   actually transfers control.

**Where the persisted pattern/colour comes from — `set_aes_background()`**
(`aes/geminit.c:631-641`, gated `CONF_WITH_BACKGROUNDS`, default **1**,
`include/config.h:1744-1748`):

```c
void set_aes_background(UBYTE patcol)
{
    OBJECT *tree = rs_trees[DESKTOP];
    tree[ROOT].ob_spec &= 0xffffff00L;
    tree[ROOT].ob_spec |= patcol;      /* only the low byte (fill colour) changes */
}
```

Called from the desktop's background-preferences dialog handler,
`desk/deskinf.c:825-921` (`inf_backgrounds`), when the user confirms a
changed desktop pattern (`BGOK`, `:899-908`):

```c
set_aes_background(curdesk & 0xff);
G.g_patcol[index].desktop = curdesk & 0xff;   /* persist for EMUDESK.INF */
G.g_screen[DROOT].ob_spec = curdesk;          /* update the desktop's OWN copy (object C) */
do_wredraw(DESKWH, &G.g_desk);                /* redraw via the normal trap path */
```

Note this is a **direct C function call** from `desk/` into `aes/`, not an AES
trap — `include/aesext.h:1-26` explicitly documents `set_aes_background` (and
several VDI/GSX helpers) as **"AES extensions not callable with trap"**,
because in EmuTOS the desktop and the AES are compiled into the **same ROM
image** and this narrow set of functions is exposed as ordinary linked C
calls rather than through the trap dispatcher. This is the *one* place the
"desktop is an AES client" model is not purely trap-mediated — worth noting
so NewTOS's plan (§6) doesn't assume a hidden AES trap exists for background
colour propagation; it doesn't. Everything else in this document (`wind_set`,
`wind_get`, `appl_init`, `objc_draw`, `menu_bar`, `evnt_multi`) **is** the
normal trap-callable AES API.

### Restoring the desktop's own pattern on return

Nothing separate needs to "restore" the desktop's pattern after an app exits:
`sh_draw(..., TRUE)`'s `specsave`/restore round-trip (step 1 above) means
`rs_trees[DESKTOP][ROOT].ob_spec` reverts to whatever `set_aes_background()`
last set it to — the white-out is *always* transient, on both entry directions.
When the desktop restarts and calls `wind_set(DESKWH, WF_NEWDESK, G.g_screen,
1, 0)` again (`deskmain.c:2049`), `w_drawdesk()` starts drawing (C) — the
desktop's own `DROOT`/icon tree — again, at `MAX_DEPTH`, over whatever the
transient white/pattern flash left behind.

---

## 4. The desktop is an ordinary AES client; the AES owns the screen

Confirmed directly from `desk/deskmain.c` (`deskmain()`, already
cross-documented in `docs/api/desktop/deskmain.md:170-180,300-345,370-375`):

- `gl_apid = appl_init();` (`:1953`) — real trap, like any app.
- `wind_get_grect(DESKWH, WF_WXYWH, &G.g_desk);` (`:1959`) — real trap;
  works because `DESKWH`=0 is a genuine, always-present AES window-manager
  slot (`D.w_win[0]`, set up unconditionally in `wm_init()`,
  `gemwmlib.c:1287-1294`), not a special code path bypassing `wind_get`.
- `wind_set(DESKWH, WF_NEWDESK, G.g_screen, 1, 0);` (`:2049`) — real trap;
  this is the **only** mechanism by which the desktop's own tree becomes
  visible as "the desktop's background" — see §2/§3.
- `menu_bar(menutree, 1);` (`:2073`) — real trap.
- `evnt_multi(...)` (`:2088`) in the main loop — real trap, exactly like any
  windowed AES app.
- `do_wredraw` → `objc_draw` (`desksupp.c:227`) — real trap, for the
  desktop's own redraws (§2).
- On exit: `menu_bar(NULL, 0); wind_set(DESKWH, WF_NEWDESK, NULL, 0, 0);
  appl_exit();` (`:2160-2166`) — real traps, symmetric un-registration.

**The AES, not the desktop, owns painting the raw screen background** in the
sense that matters for this task: whenever *no* application-registered tree
covers a piece of the screen (before the desktop registers, after it
un-registers, or — transiently — during the `CLEAR_SCREEN` white-out), the
**AES's own internal fallback object** (B, `W_TREE[ROOT]`/`gl_wtree`) is what
gets drawn, via the **same** `ob_draw` engine used for everything else. The
desktop is simply the one app that, once running, has told the AES "use
*my* tree instead of your flat fallback" — via a normal, documented,
trap-callable `wind_set` field. There is no AES code path that special-cases
"if this happens to be the desktop app, do something different"; `w_drawdesk`
only ever asks "is a tree registered (`gl_newdesk`) or not" — any GEM app
that called `wind_set(DESKWH, WF_NEWDESK, …)` would get exactly the same
treatment (this is how third-party desktop replacements on real Atari
hardware work).

---

## 5. NewTOS's current divergence

Our AES (`tos-api/src/aes/window.rs`) implements **none** of this. Instead:

- **`DESK_PAINTER`** (cite stale 2026-10-26: tos-api/src/aes/window.rs:246) — a raw `AtomicUsize` holding a
  `fn(&dyn RasterBackend, ClipRect)` pointer. `set_desktop_painter()`
  (`:249-251`) / `clear_desktop_painter()` (`:263-265`) let *any* caller
  (in practice, only `programs/desktop/src/`) install/remove it — a side channel
  that bypasses the AES trap dispatcher entirely; there is no `wind_set`
  opcode involved.
- **`paint_bg_clip()`** (cite stale 2026-10-26: tos-api/src/aes/window.rs:312-324) — called from `compose()`
  (`:346-370`, the single redraw compositor backing both `redraw_all` and
  `redraw_clip`) — checks `DESK_PAINTER`: if set, calls it directly (a raw
  pixel-push callback, **not** an object-tree draw); if unset, does a plain
  `backend.fill_rect(..., WHITE, ...)` — a hand-rolled equivalent of
  `CLEAR_SCREEN`, but achieved by literally special-casing "no painter" in
  Rust control flow, not by drawing an `OBJECT` whose `ob_spec` happens to be
  0.
- **`paint_desktop_bg`** (cite stale 2026-10-26: kernel/src/desk/mod.rs:509-517) — the registered
  painter. It fills the desktop pattern via a hand-built `PatternFill` (an
  `IP_PATTERNS` dither table reimplementing `gr_crack`'s bit layout, cite stale 2026-10-26: kernel/src/desk/mod.rs:485-496)
  straight through `RasterBackend`, then calls `global::render_desktop_icons`
  — **not** `objc_draw`/a shared drawing engine on a real `OBJECT` tree with
  `ROOT`/`DROOT` semantics. The desktop's icons are pushed to the backend by
  bespoke code, never through the `tos-api` object-manager's `draw_tree`.
- **`desktop_relinquish`/`desktop_resume`** (cite stale 2026-10-26: kernel/src/desk/main.rs:519-544)

  — the app-launch/exit toggle. `desktop_relinquish()` calls
  `save_windows()` + `clear_desktop_painter()` + `redraw()`, i.e. **the exact
  functional intent** of `wm_init()`'s `gl_newdesk = NULL` reset +
  `sh_draw(cmd, TRUE)`'s white-out (§3) — but achieved by zeroing a raw
  function pointer instead of an AES trap (`wind_set(DESKWH, WF_NEWDESK,
  NULL, 0, 0)`) clearing a registered tree pointer. `desktop_resume()`
  mirrors `deskmain()`'s re-registration (`wind_set(DESKWH, WF_NEWDESK,
  G.g_screen, 1, 0)`) by calling `set_desktop_painter()` again — same intent,
  same non-trap mechanism.
- **`tos-api/src/aes/window.rs:1190-1197`** — `wind_set`'s `match` arm
  explicitly documents the gap: *"NewTOS does not implement
  WF_NEWDESK/WF_COLOR/WF_DCOLOR at this altitude, so they honestly report
  FALSE rather than faking success."* `WF_NEWDESK` isn't even a named
  constant in this file yet (`include/aesdefs.h:546` defines it as `14`).

### A documentation/code mismatch this task surfaces

`docs/api/aes/window-graphics.md:929-930` (existing spec, same repo) states,
in its "Used by" section: *"**The desktop** (`desk/`) owns the desktop window
(handle 0) and the background tree (`WF_NEWDESK`) — this is the NewTOS
desktop."* This describes the **intended/EmuTOS-faithful** design, but the
actual `tos-api` code (cite stale 2026-10-26: tos-api/src/aes/window.rs:1190-1197, checked in the same tree) states
plainly that `WF_NEWDESK` is **not implemented**. This is not a case of a
spec misreading EmuTOS (EmuTOS-side citations in that doc are accurate); it's
the implementation having silently diverged from an already-correctly-stated
intent by building the `DESK_PAINTER` hook instead of wiring up the real
opcode. Flagging for the lead: the fix in §6 below reconciles the two.

---

## 6. Implementation-ready plan — replace `DESK_PAINTER` with the EmuTOS model

This assumes `tos-api`'s existing object-tree engine
(`tos-api/src/aes/object.rs`, `draw_tree`/`objc_draw`, already handling
`G_BOX`/`G_IBOX`/`G_ICON`/`G_STRING`/`gr_crack`-style colour cracking and
ABI-aware `ObjTree` reads) is the drawing primitive to reuse — it already
exists and already does what `ob_draw` does; nothing here proposes writing a
new rasterizer, only wiring the desktop through it.

1. **`tos-api/src/aes/window.rs` — implement `WF_NEWDESK` for real.**
   - Add `const WF_NEWDESK: i16 = 14;` (currently absent; falls through the
     catch-all).
   - Add module-level state mirroring `gl_newdesk`/`gl_newroot`: a tree
     pointer + root object index (e.g. `static NEWDESK: AtomicUsize` for the
     `ObjTree`'s base pointer/descriptor + a root index cell), defaulting to
     "none".
   - In `wind_set`'s `match`, add a `WF_NEWDESK` arm: store the tree
     pointer/root from `pb`'s packed `int_in[2..3]` (per the existing D-094
     correction in `window-graphics.md:370-385` — **not** `addr_in[0]**),
     `NULL`/`0` clears it. Return `TRUE` unconditionally (matches
     `gemwmlib.c:1685-1689`, which never fails).
   - Add a small "AES-internal fallback root" object (object **B**'s
     analogue): a single `G_BOX`-equivalent whose fill byte mirrors the
     persisted background colour (see point 4).

2. **`tos-api/src/aes/window.rs` — replace `paint_bg_clip`/`DESK_PAINTER`
   with a real draw call.**
   - Delete `DESK_PAINTER`, `set_desktop_painter`, `clear_desktop_painter`.
   - Rewrite `paint_bg_clip(backend, clip)` (called from `compose`, §5) to
     mirror `w_drawdesk`'s branch exactly: if `NEWDESK` is registered, call
     the object-manager's tree-draw (`draw_tree`/`objc_draw` equivalent) on
     the registered tree/root at **full depth** (`MAX_DEPTH`, i.e. with
     children — this is what makes the desktop's icons paint automatically,
     for free, on every window move elsewhere); if not registered, draw only
     the fallback root object (point 4) at depth 0 (no children) — this
     naturally reproduces `CLEAR_SCREEN` (a fallback root whose fill is
     white/`IP_HOLLOW` computes to the same all-zero `ob_spec` EmuTOS uses).
   - This one change removes the hand-rolled `IP_PATTERNS`/`PatternFill`
     dither logic in kernel/src/desk/mod.rs:485-517 (cite stale 2026-10-26) from the desktop crate
     entirely — pattern/colour decoding becomes the object manager's
     `gr_crack`-equivalent job (cite stale 2026-10-26: tos-api/src/aes/object.rs:1589-1609, already does this for
     other box types), used generically instead of duplicated per-caller.

3. **`programs/desktop/src/` — build a real `OBJECT` tree for `ROOT`/`DROOT` +
   desktop icons instead of pushing raster pixels.**
   - Introduce a desktop-owned `OBJECT` array analogous to `G.g_screen`
     (`deskobj.c:36-146`): object 0 = `ROOT` (full-screen anchor box, e.g.
     `ob_spec=0`), object 1 = `DROOT` (a box covering the desk-below-menu-bar
     area, `ob_spec` carrying the pattern/colour byte from
     `global::patcol_desktop()` — this is the direct analogue of
     `G.g_screen[DROOT].ob_spec`), and the drive/trash/printer icons
     currently drawn by `global::render_desktop_icons` (cite stale 2026-10-26: kernel/src/desk/mod.rs:516)
     re-expressed as `G_ICON` children of `DROOT`, each with a real
     `ob_spec`-pointed `ICONBLK` (the `object.rs` engine already supports
     `G_ICON` drawing, cite stale 2026-10-26: tos-api/src/aes/object.rs:1776-1779).
   - Replace the direct pixel-pushing in `paint_desktop_bg`
     (cite stale 2026-10-26: kernel/src/desk/mod.rs:509-517) with: build/refresh this tree once
     (on background-pattern change, icon add/remove/move — i.e. exactly when
     `inf_backgrounds`/`app_blddesk` would run in EmuTOS), then call the same
     `objc_draw`-equivalent used by point 2's fallback path. The existing
     `desk::redraw::do_wredraw` (already documented in
     `docs/api/desktop/desksupp-redraw.md` as calling `objc_draw` on the
     desktop's own tree/root — check that file for the current call site)
     should draw from this **same** tree, matching EmuTOS's "two paths, one
     tree" structure (§2, point "The desktop's own redraw").

4. **Register/un-register at desktop start/exit — real trap calls.**
   - `programs/desktop/src/lib.rs::run()` (currently `set_desktop_painter(paint_desktop_bg)`
     at `:586`): replace with a real `wind_set(DESKWH, WF_NEWDESK,
     &desktop_tree, 1, ROOT)` AES call (via whatever internal call path
     `tos-api` exposes for kernel-side AES calls without a full SVC
     round-trip — or, if none exists yet, a direct call into the new
     `wind_set` `WF_NEWDESK` handling function added in point 1; **do not**
     invent a new bypass — reuse the exact function `wind_set` calls, just
     without going through the trap frame marshalling if a kernel-internal
     entry point is the established pattern elsewhere in this codebase — a
     question for whoever implements this to confirm against how other
     kernel→AES-internal calls are already wired, e.g. `menu_bar` at
     `main.rs:522/539` which is called directly, not via `Aespb`).
   - `programs/desktop/src/main.rs::desktop_relinquish()` (`:519-532`, currently
     `save_windows()` + `clear_desktop_painter()` + `redraw()`): replace
     `clear_desktop_painter()` with the `WF_NEWDESK(NULL,0,0)` un-registration
     — the subsequent `redraw()` then naturally paints the fallback root
     (white, per point 2), reproducing `sh_draw(cmd, TRUE)`'s `CLEAR_SCREEN`
     exactly, with no separate "white erase" special case needed in Rust.
   - `programs/desktop/src/main.rs::desktop_resume()` (`:536-544`): replace
     `set_desktop_painter(super::paint_desktop_bg)` with the
     `WF_NEWDESK(&desktop_tree, 1, ROOT)` re-registration.
   - `save_windows()`/`restore_windows()` (cite stale 2026-10-26: tos-api/src/aes/window.rs:279-308) are a
     **separate, legitimate** mechanism (hiding the desktop's *window list*)
     with no direct EmuTOS analogue documented here — out of this task's
     scope; leave as-is unless a follow-up task extracts `wm_new()`'s role in
     the real shell loop (`gemshlib.c:609`, called only on abnormal desktop
     termination) to check whether it's actually equivalent. **Flagging, not
     fixing** — see Out-of-scope observations below.

5. **Persisted background byte.** `global::patcol_desktop()` (already
   existing, cite stale 2026-10-26: kernel/src/desk/mod.rs:512) is the NewTOS analogue of
   `G.g_patcol[index].desktop`/`set_aes_background`'s `patcol` byte — keep it
   as the single source of truth feeding `DROOT`'s `ob_spec` (point 3) and
   the AES-internal fallback root's `ob_spec` (point 1); this mirrors
   EmuTOS's `set_aes_background()` keeping (A) and (C) in sync from one
   value. No AES trap exists for this in EmuTOS either (§3's "AES
   extensions not callable with trap" note) — a direct function call from
   the desktop crate into the AES fallback-root state is EmuTOS-faithful,
   not a shortcut.

**Net effect:** `compose()`/`paint_bg_clip()` in `tos-api` becomes a single,
generic "draw whatever `OBJECT` tree is registered for handle 0, or the
built-in fallback" — identical in shape to `w_drawdesk()` — and the desktop
crate stops owning any bespoke pixel-drawing logic for its own background;
it owns only the `OBJECT` tree data (exactly EmuTOS's split of
responsibility between `aes/` and `desk/`).

---

## Open questions / STOP-blocks

- **Kernel→AES-internal call convention for `WF_NEWDESK` registration**
  (plan point 4): this document doesn't know whether `programs/desktop/src/`
  already has an established "call an AES function directly, bypassing the
  `Aespb`/SVC marshalling" pattern (the `menu_bar` calls in `main.rs:522/539`
  suggest yes) or whether `WF_NEWDESK` registration should go through a full
  `Aespb`-shaped call. **Needs a Developer decision**, not a Researcher one —
  this is implementation-mechanics, not EmuTOS semantics (EmuTOS itself
  compiles desk/ and aes/ into one image and calls `wind_set` as an ordinary
  C function from `deskmain.c`, so there is no ABI question on the EmuTOS
  side to resolve).
- **`rs_trees[DESKTOP][ROOT]`'s exact `ob_type`** (§1): inferred as a visible
  box type from behavior, not confirmed from the (binary) compiled resource.
  Low-risk (the fallback-root's exact type doesn't change the `ob_spec`
  bit-layout or the `CLEAR_SCREEN`-equals-zero fact), but flagged rather than
  asserted.

## Out-of-scope observations

- `save_windows()`/`restore_windows()` (`tos-api/src/aes/window.rs:267-308`)
  hide/restore the desktop's *window list* across a GEM app launch; this
  document did not extract whether EmuTOS's `wm_new()` (called only on
  abnormal desktop termination, `gemshlib.c:609`) or some other mechanism is
  the faithful analogue, or whether this NewTOS mechanism is a deliberate,
  documented reference-only design choice. One line, not expanded here.
- `docs/api/aes/window-graphics.md:929-930`'s "Used by" note ("this is the
  NewTOS desktop") should be corrected or annotated once the `WF_NEWDESK`
  implementation lands, to stop describing unimplemented behavior as
  present tense fact — a documentation-hygiene follow-up, not part of this
  extraction.
