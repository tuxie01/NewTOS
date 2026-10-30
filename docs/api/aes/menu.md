# AES Menu Manager (`menu_*`) — opcodes 30–39

> **PHASE-00b D8, Batch 5 (full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemmnlib.c` (`mn_bar`/`do_chg`/`mn_register`/`mn_cleanup`/
> `mn_getownid`, the drop-down tracking `mn_do`), `aes/gemmnext.c`
> (`mn_popup`/`mn_attach`/`mn_istart`/`mn_settings` — AES 3.30 extensions);
> dispatch arm `aes/gemsuper.c:167-201` (`crysbind`); the MN_SELECTED message is
> built by the control manager `aes/gemctrl.c:318-371` (`hctl_rect`/`ct_msgup`)
> → `aes/gemwmlib.c:851-863` (`ap_sendmsg`); opcode + slot macros
> `include/aesdefs.h:36-45,266-299`; menu structs `aes/gemmnext.h:16-33`.
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (the active menu tree `gl_mntree`/`gl_mnppd`, the DA
> registration slots, the message pipe): [`_state.md`](./_state.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** opcode numbers (30–39); the `int_in`/`addr_in` slot layout of
> each call (**every menu selector puts its primary index in `int_in[0]` and its
> state/flag in `int_in[1]`**, not `int_in[1]`/`int_in[2]`); `menu_bar`'s
> `show=1`/`0` draw/erase with the tree in `addr_in[0]`; the object `ob_state`
> bits flipped (`CHECKED`/`DISABLED`/`SELECTED`); the `menu_ienable` item bit-15
> = *redraw-now* flag; the **MN_SELECTED (10) message layout** (`msg[3]`=title
> obj, `msg[4]`=item obj, and — with `CONF_WITH_MENU_EXTENSION` — `msg[5..7]` =
> tree-ptr hi/lo + submenu parent); the `menu_register` slot-id / -1 return; the
> `ME_INQUIRE`/`ME_ATTACH`/`ME_REMOVE` flags; the `MENU`/`MN_SET` struct field
> layout; **the menu-bar tree topology `menu_sub` reveals** — a title's
> drop-down box is *not* its child, it is the *N*-th object in the `ob_next`
> sibling chain rooted at `THESCREEN`'s `ob_tail` child, *N* = the title's
> ordinal position (D-085) — because this defines how every real menu-bar
> resource is laid out and must be interpreted, even though `menu_sub`'s own
> C loop is reimplementable; **the mouse-ownership guarantee over a bar
> track** — the mouse-down that selects an item, and its later mouse-up, are
> never delivered to the application as `MU_BUTTON` (see "The drop-down
> tracking loop" below) — because an application's `evnt_multi` loop is
> written against that guarantee, even though the `gl_mowner`/`gl_ctmown`
> ownership bookkeeping that produces it is reimplementable; **`menu_popup`'s
> selecting press is likewise consumed inside the call (A73, 2026-09-20)** —
> the function does not return until the button that picked (or cancelled)
> the item is back up (`aes/gemmnext.c:314,351`), so the caller's next
> `evnt_*` wait for a press sees `button == 0` — because callers written
> against `menu_popup` rely on this exactly as bar-track callers rely on the
> guarantee above, even though `handle_popup`'s tracking loop and the
> `fm_own`-based ownership bracket around it are reimplementable.
> **Reference (NewTOS reimplements freely):**
> `do_chg`'s exact signature, `menu_down`/`menu_sr`, the `mn_do`
> mouse-tracking state machine, `bb_save`/`bb_restore`/`popup_blit` raster
> save-restore, the `gl_mntree`/`gl_mnppd`/`desk_ppd[]`/`D.g_acctitle[]`/
> `gl_submenu` internals, and `ct_msgup`'s button-wait loop — these are the
> EmuTOS AES internals NewTOS reimplements on the object manager + its own
> event/IPC model.

---

## The menu bar IS an OBJECT tree

A menu bar is an ordinary **OBJECT tree** (see [`object-form.md`](./object-form.md))
with a fixed shape. Object indices used by the menu code
(`aes/gemmnlib.c:46-48`, `aes/gemctrl.c:42`):

| Index | Constant | Object | Role |
|------:|----------|--------|------|
| 0 | `THESCREEN` (= `ROOT`, `obdefs.h:20`) | `G_IBOX` | the tree root; **two** children (see below) |
| 1 | `THEBAR` | `G_BOX` | the visible bar strip drawn by `menu_bar` — `THESCREEN`'s `ob_head` child |
| 2 | `THEACTIVE` | `G_IBOX` | the box holding the application titles |
| 3 | `THEDESK` | `G_TITLE` | the **Desk** title (system menu; must equal `DESKMENU` in the desktop resource) |

Under `THEACTIVE` the **titles** (`G_TITLE`) are siblings, `THEDESK`(3) being
the first. The AES draws the bar, tracks the mouse over titles, drops the box,
highlights the item under the cursor, and on button release posts an
**`MN_SELECTED`** message carrying `[title obj, item obj]`. So most `menu_*`
calls simply flip one object's `ob_state` bit and (sometimes) redraw it —
routed through the shared `do_chg` helper. The **tree layout, the drop-down
tracking result, and the message format are contract**; the pixel drawing is
the object manager's job (`ob_draw`/`ob_change`, cross-ref — do not re-specify
here).

### Correction (D-085): a title's drop-down box is *not* its child — it is looked up positionally

> **Correction vs. the previous spec.** The old file claimed "each title's
> child is a `G_BOX` drop-down whose children are the items." **This is
> wrong.** The drop-down boxes are **not** parented under their titles at all
> — they are found by `menu_sub` (`aes/gemmnlib.c:71-86`), which walks a
> completely separate part of the tree:
>
> ```c
> static WORD menu_sub(OBJECT *tree, WORD ititle)
> {
>     themenus = (tree+THESCREEN)->ob_tail;         /* THESCREEN's 2nd child */
>     imenu = (tree+themenus)->ob_head;              /* 1st drop-down box    */
>     for (i = ititle-THEACTIVE; i > 1; i--)
>         imenu = (tree+imenu)->ob_next;             /* walk sibling chain   */
>     return imenu;
> }
> ```
>
> `THESCREEN`'s **`ob_tail`** (its *second*, not first, child — `THEBAR` is
> `ob_head`) is a hidden container object (`themenus`); **its children, in
> `ob_next` sibling-chain order, are the drop-down `G_BOX`es, one per title —
> ordered to match title order, not linked to any specific title object.**
> Title *N* (counting from `THEDESK`=3 as position 1) maps to the *N*-th box in
> that chain. `menu_fixup` (`gemmnlib.c:89-125`) confirms this independently:
> it computes the **Desk** menu's own drop-down box as
> `dabox = tree[themenus].ob_head` (`gemmnlib.c:101-102`) — i.e. literally
> "`themenus`'s first child", matching `menu_sub`'s result for `ititle =
> THEDESK` (loop body skipped, `i = THEDESK-THEACTIVE = 1`).
>
> **This is contract, not internal**, even though `menu_sub`'s C loop itself
> is a reimplementable detail: any menu-bar `OBJECT` resource built by an
> external resource compiler (and any real GEM program's static resource
> file) is laid out this way, and a NewTOS object-tree/menu-resource walker
> that instead assumed "drop-down = title's `ob_head` child" would
> misinterpret every real menu-bar resource — the item highlighting, the
> drop-down draw, and `MN_SELECTED`'s `item` object number would all be
> wrong. What each drop-down box's *children* are (the **items**,
> `G_STRING`) is unaffected by this correction — that part of the old text
> was right. See D-085.

### `do_chg` — the shared state-setter (reference)

`do_chg(tree, iitem, chgvalue, dochg, dodraw, chkdisabled)`
(`gemmnlib.c:145-172`) sets (`dochg≠0`) or clears (`dochg=0`) the `chgvalue`
bit in object `iitem`'s `ob_state`, optionally redraws it (`dodraw`), and — if
`chkdisabled` — refuses when the item is already `DISABLED`. It returns `TRUE`
unless the disabled-guard blocked it. **The important per-call detail is which
of `dochg`/`dodraw`/`chkdisabled` each menu selector passes** (table below);
`do_chg` itself is a reimplementable helper.

### Object `ob_state`/`ob_flags` bits touched (`include/obdefs.h:82-107`)

| Bit | Value | Field | Set by |
|-----|------:|-------|--------|
| `SELECTED` | 0x0001 | `ob_state` (`obdefs.h:101`) | `menu_tnormal` (title highlight), the tracking loop (item/title hilite) |
| `CHECKED`  | 0x0004 | `ob_state` (`obdefs.h:103`) | `menu_icheck` (the ✓ mark) |
| `DISABLED` | 0x0008 | `ob_state` (`obdefs.h:104`) | `menu_ienable` (greyed / non-selectable) |
| `SUBMENU`  | 0x0800 | `ob_flags` (`obdefs.h:98`) | `menu_attach` — marks an item as having a submenu; **`menu_attach` also encodes the submenu-table index into `ob_type`'s upper byte**, see D-087 below |

---

## Parameter-block slot map (`include/aesdefs.h:266-294`)

Every menu call enters via the GEM trap (`svc #2`, **selector 200 in `x0`**,
AESPB pointer in `x1`); the dispatcher reads `control[0]`=opcode,
`control[1..3]`=#int_in/#int_out/#addr_in, and returns `int_out[0]`=`RET_CODE`
(see [`_dispatcher.md`](./_dispatcher.md)). The exact `aesdefs.h` slot macros:

| Macro | Slot | Used by |
|-------|------|---------|
| `MM_ITREE`  | `addr_in[0]` | bar, icheck, ienable, tnormal, text |
| `MM_PSTR`   | `addr_in[0]` | register (the DA menu-item string) |
| `MM_PTEXT`  | `addr_in[1]` | text (the replacement string) |
| `SHOW_IT`   | `int_in[0]`  | bar (1 = show, 0 = erase) |
| `ITEM_NUM`  | `int_in[0]`  | icheck, ienable (item object index; ienable: bit 15 = redraw-now) |
| `TITLE_NUM` | `int_in[0]`  | tnormal (title object index) |
| `MM_PID`    | `int_in[0]`  | register (the caller's AES pid) |
| `CHECK_IT`  | `int_in[1]`  | icheck (1 = add ✓, 0 = remove) |
| `ENABLE_IT` | `int_in[1]`  | ienable (1 = enable, 0 = disable) |
| `NORMAL_IT` | `int_in[1]`  | tnormal (1 = normal, 0 = highlighted) |
| `MPOP_IN`   | `addr_in[0]` | popup (`MENU*` in), attach/istart (`OBJECT*` tree) |
| `MPOP_OUT`  | `addr_in[1]` | popup / attach (`MENU*` result) |
| `MPOP_SET`  | `addr_in[0]` | settings (`MN_SET*`) |
| `MPOP_XPOS` | `int_in[0]`  | popup (x) |
| `MPOP_YPOS` | `int_in[1]`  | popup (y) |
| `MPOP_FLAG` | `int_in[0]`  | attach/istart/settings (operation selector) |
| `MPOP_ITEM` | `int_in[1]`  | attach (item) / istart (menu box object) |
| `MPOP_ITEM2`| `int_in[2]`  | istart (start item) |

> **Correction vs. the previous spec.** The old file placed `ITEM_NUM`/
> `TITLE_NUM` at `int_in[1]` and the state flags (`CHECK_IT`/`ENABLE_IT`/
> `NORMAL_IT`) at `int_in[2]`. The source is the opposite: the **primary index
> is `int_in[0]`** (`ITEM_NUM`/`TITLE_NUM`/`MM_PID`/`SHOW_IT`) and the **state/
> flag is `int_in[1]`** (`aesdefs.h:275-283`). See D-051.

---

## Function table (dispatch → effect)

| Op | Name | Dispatch (`gemsuper.c:167-201`) | Return (`int_out[0]`) |
|---:|------|---------------------------------|-----------------------|
| 30 | `menu_bar`     | `mn_bar(MM_ITREE, SHOW_IT)` | `1` (`mn_bar` is `void`; `ret` stays `TRUE`) |
| 31 | `menu_icheck`  | `do_chg(MM_ITREE, ITEM_NUM, CHECKED, CHECK_IT, FALSE, FALSE)` | `1` (`do_chg` `TRUE`) |
| 32 | `menu_ienable` | `do_chg(MM_ITREE, ITEM_NUM&0x7fff, DISABLED, !ENABLE_IT, (ITEM_NUM&0x8000)!=0, FALSE)` | `1`/`0` (`do_chg`) |
| 33 | `menu_tnormal` | `do_chg(MM_ITREE, TITLE_NUM, SELECTED, !NORMAL_IT, TRUE, TRUE)` | `1`/`0` (`do_chg`; `0` if title `DISABLED`) |
| 34 | `menu_text`    | `strcpy(tree[ITEM_NUM].ob_spec, MM_PTEXT)` | `1` (`void`; `ret` stays `TRUE`) |
| 35 | `menu_register`| `mn_register(MM_PID, MM_PSTR)` | slot id `0..5`, or `-1` |
| 36 | `menu_popup`†  | `mn_popup(MPOP_IN, MPOP_XPOS, MPOP_YPOS, MPOP_OUT)` | `1` (item chosen) / `0` |
| 37 | `menu_attach`† | `mn_attach(MPOP_FLAG, MPOP_IN, MPOP_ITEM, MPOP_OUT)` | inquire/attach/remove result (`1`/`0` or `s_start`) |
| 38 | `menu_istart`† | `mn_istart(MPOP_FLAG, MPOP_IN, MPOP_ITEM, MPOP_ITEM2)` | the submenu's `s_start` item, or `0`/`FALSE` |
| 39 | `menu_settings`†| `mn_settings(MPOP_FLAG, MPOP_SET)` | `1` (`void`; `ret` stays `TRUE`) |

† **`#if CONF_WITH_MENU_EXTENSION`** (default **1**, `include/config.h:1305`),
opcodes 36–39, the whole of `aes/gemmnext.c` (`gemsuper.c:188-201`). With the
flag off, opcodes 36–39 have **no `case`** → the dispatcher `default` arm returns
**`-1`** (see [`_dispatcher.md`](./_dispatcher.md) §unsupported-opcode). Opcodes
30–35 are unconditional (the original TOS set).

---

## `menu_bar` (30)

**Source:** `gemsuper.c:168-170` → `aes/gemmnlib.c:703-732` (`mn_bar`).
Helpers: `menu_fixup()` (`gemmnlib.c:89-125`, splices the DA items in),
`ob_draw`/`ob_actxywh` (object manager), `gsx_sclip`/`gsx_cline` (VDI draw),
`post_keybd(ctl_pd->p_cda, 0)` (wakes the control manager to re-measure its
wait rectangle). **Globals written:** `gl_mntree` (the active menu tree ptr;
`NULL` when no bar), `gl_mnppd` (the AES process that owns the bar),
`gl_ctwait.m_gr` (the control-manager wait rectangle) (`gemmnlib.c:62-63`,
`712-726`; see [`_state.md`](./_state.md)).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MM_ITREE`) | pointer to the menu OBJECT tree |
| `int_in[0]` (`SHOW_IT`) | **1 = install/draw** the bar; **0 = remove** it |
| `int_out[0]` | `1` (`mn_bar` is `void`; `crysbind` `ret` stays `TRUE`) |

**Behaviour.** `show=1`: record `gl_mnppd = rlr`'s descriptor and
`gl_mntree = tree`; call `menu_fixup()` to append the desk-accessory items to
the tree; stretch `THEBAR`'s width to the screen; set the control-wait rect to
`THEACTIVE`'s extent; draw `THEBAR` (`ob_draw(tree, THEBAR, MAX_DEPTH)`); draw
the separator line under the bar in black replace-mode. `show=0`: clear
`gl_mntree = NULL` and put `gl_rmenu` back into the control-wait rect
(`gemmnlib.c:726-727`). Either way, post a null key to the control manager's
console queue so it re-reads the wait rectangle (`gemmnlib.c:730`).

> **`gl_ctwait` with and without a bar (A87, 2026-09-21).** The control
> manager's wait rectangle has exactly two values. With a bar it is
> **`THEACTIVE`'s absolute extent** — `ob_actxywh(tree, THEACTIVE,
> &gl_ctwait.m_gr)` (`gemmnlib.c:717`), i.e. the titles' width, not the whole
> bar row; an RSC menu's `THEACTIVE` is sized to its titles (EmuTOS desktop
> `adjust_menu`, `desk/deskmain.c:1528-1534`: `mbar->ob_width = x`). Without
> a bar it is **`gl_rmenu`** = `(0, 0, gl_width, gl_hbox)`, the whole bar row
> (`gemgraf.c:427`) — `ctlmgr`'s initial value (`gemctrl.c:437-438`) and what
> `mn_bar(…, FALSE)` restores (`gemmnlib.c:727`); *"full desktop"* in earlier
> wording was wrong. The two consumers of `gl_ctwait` both gate on
> `gl_mntree` first — `mchange`'s hand-over `if ((gl_mowner != ctl_pd) &&
> (button == 0) && gl_mntree && in_mrect(&gl_ctwait))` (`geminput.c:388-389`)
> and `ctlmgr`'s `MU_M1` arm `if (gl_mntree) ev_which |= MU_M1`
> (`gemctrl.c:451-452`) — so the `gl_rmenu` value never decides a hand-over.
> Consequences: a pointer **moving** in the bar row right of the last title
> stays with the application (its `MU_M1`/`MU_M2` complete there, no track
> starts, its timer/keys keep flowing); a **press** anywhere in the bar row
> is a different rule and is *not* the application's — `bchange` → `chkown`
> → `chk_ctrl` answers `-1` for `inside(mx, my, &gl_rmenu)`
> (`geminput.c:96-97, 316-322, 334-335`), `post_button(ctl_pd, …)` never
> visits the application's parked wait, the control manager's own
> `MU_BUTTON` finds no window there (`wm_find` = 0, `gemctrl.c:467-470`) and
> drops it, and after the release `w_setactive` → `set_mown(app)` posts
> `button == 0` (`geminput.c:225-231`), which satisfies no press-wait. Such a
> click is swallowed, with or without a bar.

> `mn_bar` ignores `tree` when `show=0` (`gemmnlib.c:723-726`) — the tree used
> for erase is the one recorded in `gl_mntree` from the matching `show=1`.

### A dead client's menu bar (NewTOS rule, A107, 2026-09-22)

`mn_bar(tree, TRUE)` records **both** the tree and its owner: `p = fpdnm(NULL,
rlr->p_pid); … gl_mnppd = p; gl_mntree = tree;` (`gemmnlib.c:707,712-713`).
EmuTOS never clears either on a program's exit — `gl_mntree` is written in
exactly two places, `mn_bar` (`:713,726`) and `fm_own`'s dialog park/restore
(`gemfmlib.c:89-90,103`); `mn_cleanup` (`:740-760`, called from `ap_exit`
`gemaplib.c:254` and from `sh_ldapp`'s `AP_OPEN` cleanup `gemshlib.c:652`)
only sends `AC_CLOSE` to the accessories and frees the submenu array, and
neither `wm_new` (`gemwmlib.c:1875-1893`) nor `wm_init` (`:1247-1310`) touches
it. A program that ends with its bar installed leaves the pointer standing
until the next program's `menu_bar(tree, 1)` overwrites it (the desktop's own,
`desk/deskmain.c:1904`).

On EmuTOS that dangling pointer is inert — no MMU, the dead TPA is still
readable memory. On NewTOS it is a fault: the tree is an EL0 VA of an address
space that is gone, and the Control Manager dereferences it at EL1 under the
*next* caller's `TTBR0` (`menu::track` → `ObjTree::new`) — the same class of
defect A97 closed for `WF_NEWDESK`. **NewTOS therefore drops the bar with the
client that installed it:** the per-slot sweep
([`window-graphics.md`](window-graphics.md) § `wind_new (109)`, A107) clears
`gl_mntree` (and any tree `fm_own` parked) when `gl_mnppd` is the dead slot,
and only then — another client's bar is untouched. State only: nothing is
drawn, exactly as `mn_bar(…, FALSE)`'s state half (`gemmnlib.c:726-727`); the
bar row belongs to whoever paints next (TeraDesk's `regen_desktop`,
`sdk/apps/teradesk/icon.c:2509-2518`).

---

## `menu_icheck` (31)

**Source:** `gemsuper.c:171-173` → `do_chg(tree, ITEM_NUM, CHECKED, CHECK_IT,
FALSE, FALSE)`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MM_ITREE`) | menu tree |
| `int_in[0]` (`ITEM_NUM`) | item object index |
| `int_in[1]` (`CHECK_IT`) | **1 = add** the ✓ check mark, **0 = remove** it |
| `int_out[0]` | `1` (`do_chg` returns `TRUE`; `chkdisabled=FALSE`) |

Sets/clears the `CHECKED` (0x0004) `ob_state` bit. **`dodraw = FALSE`** — the
change is *not* drawn immediately; the check mark appears the next time the
drop-down is drawn (`gemsuper.c:172`, the `do_chg` call site — not
`gemmnlib.c:172`, a citation error in the previous spec: that line is
`do_chg`'s closing brace, not the `FALSE, FALSE` arguments). Contract detail:
`menu_icheck` alone never repaints.

---

## `menu_ienable` (32)

**Source:** `gemsuper.c:174-177` → `do_chg(tree, ITEM_NUM & 0x7fff, DISABLED,
!ENABLE_IT, (ITEM_NUM & 0x8000) != 0, FALSE)`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MM_ITREE`) | menu tree |
| `int_in[0]` (`ITEM_NUM`) | item object index in bits 0–14; **bit 15 = redraw-now flag** |
| `int_in[1]` (`ENABLE_IT`) | **1 = enable**, **0 = disable** the item |
| `int_out[0]` | `1` (`do_chg` returns `TRUE`; `chkdisabled=FALSE`) |

Sets (`ENABLE_IT=0`) or clears (`ENABLE_IT=1`) the `DISABLED` (0x0008) bit — a
disabled item is greyed and not selectable. The item index is masked with
`0x7fff`; **bit 15 of the item word is the `dodraw` flag**: if set, `do_chg`
redraws the item immediately, otherwise the change is deferred to the next draw
(`gemsuper.c:175-176`).

> **Correction vs. the previous spec.** The old file called `item & 0x8000` a
> "title-bar/special item" flag. It is not — it is the **redraw-now** (`dodraw`)
> flag passed as `do_chg`'s 5th argument (`gemsuper.c:176`). See D-051.

---

## `menu_tnormal` (33)

**Source:** `gemsuper.c:178-180` → `do_chg(tree, TITLE_NUM, SELECTED, !NORMAL_IT,
TRUE, TRUE)`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MM_ITREE`) | menu tree |
| `int_in[0]` (`TITLE_NUM`) | **title** object index |
| `int_in[1]` (`NORMAL_IT`) | **1 = draw normal**, **0 = draw highlighted** (inverted) |
| `int_out[0]` | `1` (`do_chg` `TRUE`), or **`0` if the title is `DISABLED`** (`chkdisabled=TRUE`) |

Sets (`NORMAL_IT=0`) or clears (`NORMAL_IT=1`) the title's `SELECTED` (0x0001)
bit and **always redraws** (`dodraw=TRUE`). This is how an application
un-highlights a menu title after acting on an `MN_SELECTED` message — the
standard idiom is `menu_tnormal(tree, title, 1)`. **`chkdisabled=TRUE`**: on a
disabled title `do_chg` refuses and returns `0`, leaving the state unchanged.

---

## `menu_text` (34)

**Source:** `gemsuper.c:181-184` → **inline** `strcpy((char *)tree[ITEM_NUM].
ob_spec, (char *)MM_PTEXT)` (no `mn_*` helper).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MM_ITREE`) | menu tree |
| `addr_in[1]` (`MM_PTEXT`) | new NUL-terminated string |
| `int_in[0]` (`ITEM_NUM`) | item object index |
| `int_out[0]` | `1` (inline `strcpy`, no return; `crysbind` `ret` stays `TRUE`) |

Replaces the item's text **in place**: copies the new string over the buffer at
`tree[item].ob_spec`. **No length check, no redraw** — the caller must ensure
the new string is no longer than the original allocation, and repaint the item
itself (e.g. via `objc_draw`, or a subsequent `menu_ienable` with bit 15).

> **Edge case / danger.** The in-place `strcpy` (`gemsuper.c:183`) will overrun
> the original `ob_spec` buffer if the replacement is longer. This is TOS
> behaviour a program relies on being "same length or shorter"; it is contract
> that `menu_text` overwrites the existing buffer (does not allocate a new one).

---

## `menu_register` (35)

**Source:** `gemsuper.c:185-187` → `aes/gemmnlib.c:789-816` (`mn_register`).
Helpers: `menu_fixup()` (splices the DA into the tree), `build_menuid_lookup()`
(`gemmnlib.c:766-782`, the display-slot → menu-id table). **Globals written:**
`D.g_accreg` (count of registered DAs), `D.g_acctitle[NUM_ACCS]` (the DA title
string pointers), `desk_ppd[NUM_ACCS]` (the DA owner process descriptors),
`acc_display[]` (the lookup table) (`gemmnlib.c:64-65,798-810`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`MM_PID`) | the registering application's AES pid |
| `addr_in[0]` (`MM_PSTR`) | pointer to the menu-item title string |
| `int_out[0]` | the assigned **slot id** `0 .. NUM_ACCS-1` (0–5), or **`-1`** on failure |

Registers a **desk-accessory** entry under the *Desk* menu. If the pid is valid
(`≥ 0`) and there is room (`D.g_accreg < NUM_ACCS`, `NUM_ACCS = 6`,
`sysconf.h:37`): increment the count, find the first free `g_acctitle[]` slot,
record `desk_ppd[slot] = rlr` and `g_acctitle[slot] = pstr` (**stores the caller's
pointer, exactly like Atari TOS — the string is not copied**), splice the item
into the menu tree (`menu_fixup`), rebuild the lookup, and **return the slot
index** (`gemmnlib.c:798-810`). If the pid is negative or all six slots are
full, return **`-1`** (`gemmnlib.c:815`).

The returned slot id is the accessory's *menu id*: when the user later picks the
DA entry, the control manager maps the display slot back to this id via
`acc_display[]`/`mn_getownid` and sends the owner an **`AC_OPEN`** message with
that id in `msg[4]` (`gemctrl.c:337-346`, `aes/gemmnlib.c:838-848`)
.

> The string pointer is stored, not duplicated (`gemmnlib.c:806`) — the caller
> must keep the title string alive for the life of the registration.

---

## `menu_popup` (36) — `CONF_WITH_MENU_EXTENSION`

**Source:** `gemsuper.c:189-191` → `aes/gemmnext.c:304-356` (`mn_popup`).
Helpers: `wm_update(BEG_MCTRL/END_MCTRL)` (grab/release the mouse),
`ev_button` (wait for button-up, called **both before and after** the
tracking loop, `gemmnext.c:314,351`), `popup_blit` (save/restore background),
`handle_popup` (the pop-up tracking loop; it sets `keystate` itself at
`gemmnext.c:272` — see D-086 below), `clamp_ypos` (keep the box on-screen).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`MPOP_IN`) | input `MENU *` describing the pop-up (tree/menu box/start item/scroll) |
| `int_in[0]` (`MPOP_XPOS`) | x of the **start item** (not the box) |
| `int_in[1]` (`MPOP_YPOS`) | y of the start item |
| `addr_in[1]` (`MPOP_OUT`) | output `MENU *` — filled with the choice |
| `int_out[0]` | **`1`** if an item was chosen, **`0`** otherwise |

**Behaviour.** `mn_popup` first grabs the mouse (`wm_update(BEG_MCTRL)`) and
**blocks until any currently-held mouse button is released**
(`ev_button(1, 0x00ff, 0x0000, rets)`, `gemmnext.c:314`) — the popup is not
shown while a button is still down. It then positions the parent box at
(x,y), where (x,y) is the position of the *start* item; the parent box y is
derived by subtracting the start item's y offset, then the box is clamped to
the screen (`clamp_ypos`, `gemmnext.c:114-127`: while `ob_y >
gl_rscreen.g_h - ob_height` step `ob_y -= gl_hchar`, then while `ob_y <
gl_rfull.g_y` step `ob_y += gl_hchar` — the bar-row rule wins; `ob_x = xpos`
is **not** clamped, `:322`; the clamp precedes `popup_blit`, so the caller's
tree shows it even on the `buf == NULL` branch — A88, 2026-09-21). It saves
the background (`popup_blit`), runs the
tracking loop (`handle_popup`), and restores the background. On a pick,
fills `*mdata`: `mn_tree`/`mn_menu` copied from the input (**no submenus
from popups — restriction #1 below**), `mn_item` = the chosen object
number, `mn_scroll` copied, `mn_keystate` = the shift state at selection
(`rets[3]` = `kstate`, bits 0-3 only: `Kbshift(-1) & 0x000f`, A126);
returns `1`. On no pick, sets `mdata->mn_keystate = 0` **and nothing else**
— the `else` branch is that one statement (`gemmnext.c:345-349`); `mn_tree`,
`mn_menu`, `mn_item` and `mn_scroll` of the output block keep whatever the
caller had there (A102, 2026-09-21) — and returns `0` (`gemmnext.c:336-349`). Either way, it waits for button-up again
(`gemmnext.c:351`) and releases the mouse (`wm_update(END_MCTRL)`,
`gemmnext.c:353`) before returning.

> **Contract addition — the whole call is the application's own mouse
> ownership, and it consumes the selecting press (A73, 2026-09-20).**
> `wm_update(BEG_MCTRL)` (`gemmnext.c:312`; `BEG_MCTRL` = `2 + BEG_UPDATE`,
> `include/aesdefs.h:559-562`) resolves to `fm_own(TRUE)`
> (`gemwmlib.c:1778-1797`), which clears `gl_mntree`, widens the control
> rectangle to the whole screen and calls `ct_chgown(rlr, &gl_rscreen)` —
> i.e. `set_mown(rlr)`, **the calling application itself**
> (`gemfmlib.c:82-107`, `gemctrl.c:392-397`). No `ct_mouse`/`gl_ctmown`
> bracket is ever entered — `grep -n 'ct_mouse\|gl_ctmown' aes/gemmnext.c`
> matches nothing — so the drop-down tracking loop's ownership machinery
> ("Mouse ownership over the track" below) does not apply here; there is
> nothing for it to guard against, because no *other* process ever holds
> the mouse during `mn_popup`.
>
> This is also why `mn_popup` must wait out the button itself: the popup is
> not shown while a button from *opening* it is still down
> (`ev_button(1, 0x00ff, 0x0000, rets)` before `handle_popup`,
> `gemmnext.c:314`), and `handle_popup`'s own tracking loop
> (`ev_multi(MU_M1|MU_BUTTON, …)`, `gemmnext.c:268`) returns on the
> selecting/cancelling `MU_BUTTON` itself (`:290-291`) — i.e. on the press,
> button still down. The second `ev_button(1, 0x00ff, 0x0000, rets)`
> (`gemmnext.c:351`), run after the pick/no-pick branch and before
> `wm_update(END_MCTRL)` (`:353`), blocks until that press is released.
> **Contract:** by the time `menu_popup` returns, the button is up — the
> press that picked (or cancelled) the item, and its release, are both
> consumed inside the call; the caller's own next `evnt_*` wait for a press
> is never satisfied by them.

> **Correction / addition (D-086).** The previous spec listed a helper
> function `submenu_event` that "sets `keystate`". **No such function
> exists anywhere in EmuTOS** (`grep -rn submenu_event references/emutos/`
> matches only a stale comment, `gemmnext.c:342`: `/* set by
> submenu_event() */`). `keystate` is in fact set directly inside
> `handle_popup` (`gemmnext.c:272`, `keystate = rets[3];`) — the comment
> referencing a `submenu_event()` helper is inaccurate/outdated EmuTOS
> source-comment cruft (per the project rule "comments lie, code doesn't").
> The previous spec derived a fictitious helper name from this stale
> comment instead of from the code. See D-086.

> **New edge case (D-086).** If `popup_blit`'s background-save allocation
> fails (`dos_alloc_stram` returns `NULL`, `gemmnext.c:157-163`), `mn_popup`
> **never calls `handle_popup`** (`gemmnext.c:329-334` is skipped, `buf ==
> NULL`) — `objnum` stays `-1` and the function falls straight through to
> the "no pick" branch (`mdata->mn_keystate = 0`, `rc = FALSE`,
> `gemmnext.c:336,345-349`), while still correctly waiting for button-up
> and releasing the mouse (`gemmnext.c:351,353`). A caller cannot
> distinguish "no item chosen" from "out of ST-RAM" — both return `0`.

---

## `menu_attach` (37) — `CONF_WITH_MENU_EXTENSION`

**Source:** `gemsuper.c:192-194` → `aes/gemmnext.c:535-557` (`mn_attach`).
Helpers: `query_submenu` (`gemmnext.c:401-433`), `attach_submenu`
(`gemmnext.c:465-530`), `detach_submenu` (`gemmnext.c:438-460`) — manage the
`SMIB` submenu-info array `gl_submenu` (`aes/struct.h:32,53`: `NUM_SMIBS =
128` entries, allocated on first use).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`MPOP_FLAG`) | operation: `ME_INQUIRE`=0, `ME_ATTACH`=1, `ME_REMOVE`=2 (`aesdefs.h:297-299`) |
| `addr_in[0]` (`MPOP_IN`) | the main-menu-bar `OBJECT *` tree |
| `int_in[1]` (`MPOP_ITEM`) | the item to attach the submenu to |
| `addr_in[1]` (`MPOP_OUT`) | `MENU *` — submenu descriptor (in for attach, out for inquire) |
| `int_out[0]` | `1`/`TRUE` on success, `0`/`FALSE` on failure |

Attaches (`ME_ATTACH`=1), removes (`ME_REMOVE`=2), or queries (`ME_INQUIRE`=0)
a submenu on a menu-bar item. **Guard:** the target item must be a `G_STRING`
(`tree[item].ob_type & 0xff == G_STRING`), else return `FALSE` immediately,
**for all three operations** (`gemmnext.c:539-541`). Attaching sets the
item's `SUBMENU` (0x0800) `ob_flags` bit and writes a right-arrow marker
(0x03) two chars from the end of the item text (`gemmnext.c:522-527`).

> **Correction / addition (D-087).** The previous spec said attaching "sets
> the item's `SUBMENU` `ob_flags` bit and writes a right-arrow marker" — true
> but **incomplete**: `attach_submenu` **also writes the `SMIB` array index
> into the item's `ob_type` upper byte**: `obj->ob_type |= ((submenu -
> gl_submenu) + 128) << 8;` (`gemmnext.c:524`). This is how `query_submenu`
> and `find_submenu_from_obtype` (`gemmnext.c:361-375`) later recover which
> `SMIB` entry belongs to the item (`n = ((ob_type>>8)&0xff) - 128;`,
> `gemmnext.c:369`). `detach_submenu` reverses this: it clears the upper byte
> (`obj->ob_type &= 0x00ff;`), clears `SUBMENU`, and blanks the marker
> character with a space (`gemmnext.c:449-452`). **This mutates `ob_type`, a
> field a program's own resource/tree code may read directly** (TOS has no
> memory protection between an app and its own OBJECT tree) — a NewTOS
> object-tree/menu implementation must reproduce this exact encoding if it is
> to remain source-compatible with programs that inspect `ob_type` after
> calling `menu_attach`/`menu_istart`. See D-087.

> **Addition (D-088) — `menu_attach` behaviour and failure paths not
> previously documented:**
> - **`ME_ATTACH` on an item that already has a submenu *replaces* it**:
>   `attach_submenu` detaches the existing submenu first, then proceeds to
>   attach the new one (`gemmnext.c:471-477`) — it does not fail or leak the
>   old `SMIB` slot.
> - **`ME_REMOVE` on an item with *no* attached submenu returns `FALSE`**
>   (nothing to remove): `attach_submenu(tree, item, NULL)` — `mdata==NULL`
>   for `ME_REMOVE` — falls through `query_submenu`'s `FALSE` case straight to
>   `if (!mdata) return FALSE;` (`gemmnext.c:471-481`).
> - **`gl_submenu` array allocation failure** (`dos_alloc_anyram`, first
>   `ME_ATTACH` call ever) → `FALSE` (`gemmnext.c:484-488`).
> - **`SMIB` table full** (all `NUM_SMIBS`=128 entries in use, no matching or
>   free slot) → `FALSE` (`gemmnext.c:509-512`).
> - **`ME_INQUIRE` tolerates `mdata == NULL`** as an EmuTOS-only convenience
>   ("not specified by TOS", `gemmnext.c:397-400`) to test "does this item
>   have a submenu" without needing the descriptor — real TOS's documented
>   behaviour for a null output pointer is unspecified; do not rely on this
>   as contract.
>
> None of these paths were in the previous spec's "Error conditions" table.
> See D-088.

> **EmuTOS restriction (`gemmnext.c:13-28`):** submenus can be attached **only
> to the main menu bar** — one level of submenus, no submenus of submenus, and
> no submenus from popups.

---

## `menu_istart` (38) — `CONF_WITH_MENU_EXTENSION`

**Source:** `gemsuper.c:195-197` → `aes/gemmnext.c:562-587` (`mn_istart`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`MPOP_FLAG`) | **0 = inquire**, **1 = update**; any other = `FALSE` |
| `addr_in[0]` (`MPOP_IN`) | the tree containing the submenu |
| `int_in[1]` (`MPOP_ITEM`) | the submenu box object |
| `int_in[2]` (`MPOP_ITEM2`) | the desired start item (update only) |
| `int_out[0]` | the submenu's **`s_start`** item after the operation, or **`FALSE` (0)** if no such submenu / bad flag |

Gets or sets the *starting* (top) item of a scrollable submenu. On update it
clamps `start` to the submenu box's `[ob_head, ob_tail]` child range and stores
it as `s_start`; on inquire it just returns the current `s_start`. Returns
`FALSE` (0) if the tree/menu has no attached submenu or the flag is neither 0
nor 1 (`gemmnext.c:566-587`).

---

## `menu_settings` (39) — `CONF_WITH_MENU_EXTENSION`

**Source:** `gemsuper.c:198-200` → `aes/gemmnext.c:596-616` (`mn_settings`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`MPOP_FLAG`) | **0 = get** current values, **1 = set** new values |
| `addr_in[0]` (`MPOP_SET`) | pointer to a `MN_SET` struct (out for get, in for set) |
| `int_out[0]` | `1` (`mn_settings` is `void`; `crysbind` `ret` stays `TRUE`) |

Reads or writes the global submenu/scroll timing settings. `flag=0` copies the
current `settings` into `*set`; `flag=1` copies each **non-negative** field from
`*set` into `settings` (negative values are ignored, matching TOS), clamping
`height` to `[MIN_HEIGHT_VALUE=5, gl_rfull.g_h / gl_hchar]` (`clamp_height`,
`gemmnext.c:83-95`; `mn_settings` `:600-615`). `gl_rfull` is the desktop
area **below the menu bar**, `(0, gl_hbox, gl_width, gl_height - gl_hbox)`
(`gemgraf.c:424`), so the upper bound is `(gl_height - gl_hbox) / gl_hchar`
— 28 on a 480-row screen with the 16 px font, not `480 / 16` = 30 (A102,
2026-09-21). **Any other `flag` is a no-op:**
`mn_settings` is a `switch(flag)` with the arms `0` and `1` and no `default`
(`:598-615`), so `flag ∉ {0, 1}` neither reads nor writes either block —
`*set` and `settings` both stay as they were; `int_out[0]` is still `1`.

**Initial values** (`mnext_init`, `gemmnext.c:103-110`, run once after the
workstation is up; `INIT_*_VALUE` `:71-75`): `display` 200, `drag` 10000,
`delay` 250, `speed` 0, `height` `clamp_height(16)` = 16 on every NewTOS
profile. These are what `menu_settings(0)` returns before any `flag=1` call.

> **Minimal implementation (`gemmnext.c:594`):** "none of these values are
> relevant" — EmuTOS stores the settings but its submenu code does not act on
> the delay/drag/speed values (the scroll flag is ignored, restriction #2). The
> **`MN_SET` field layout and the get/set-non-negative contract** hold; the
> timing *behaviour* those fields imply is not realised in this build.

---

## Trap interface (parameter-block form)

All `menu_*` calls enter via the GEM trap (`svc #2`) with **selector 200 in
`x0`** and an **AESPB pointer in `x1`** — see [`_dispatcher.md`](./_dispatcher.md).

| Op | Function | `int_in` | `addr_in` | `int_out[0]` return |
|---:|----------|----------|-----------|---------------------|
| 30 | `menu_bar`     | `[0]`=show(1/0) | `[0]`=tree | `1` |
| 31 | `menu_icheck`  | `[0]`=item, `[1]`=check(1/0) | `[0]`=tree | `1` |
| 32 | `menu_ienable` | `[0]`=item(+bit15 redraw), `[1]`=enable(1/0) | `[0]`=tree | `1` |
| 33 | `menu_tnormal` | `[0]`=title, `[1]`=normal(1/0) | `[0]`=tree | `1`/`0` (0 if title disabled) |
| 34 | `menu_text`    | `[0]`=item | `[0]`=tree, `[1]`=new text | `1` |
| 35 | `menu_register`| `[0]`=pid | `[0]`=title str | slot `0..5` / `-1` |
| 36 | `menu_popup`   | `[0]`=x, `[1]`=y | `[0]`=`MENU*` in, `[1]`=`MENU*` out | `1`/`0` *(ext.)* |
| 37 | `menu_attach`  | `[0]`=flag(0/1/2), `[1]`=item | `[0]`=tree, `[1]`=`MENU*` | `1`/`0` *(ext.)* |
| 38 | `menu_istart`  | `[0]`=flag(0/1), `[1]`=menu box, `[2]`=start | `[0]`=tree | `s_start` / `0` *(ext.)* |
| 39 | `menu_settings`| `[0]`=flag(0/1) | `[0]`=`MN_SET*` | `1` *(ext.)* |

---

## Inputs / Outputs summary

Inputs arrive in the AESPB arrays (no register/stack params). Outputs are
`int_out[0]` only; there are no `addr_out` results (the menu manager never uses
`addrout` — only `rsrc_gaddr` does, see [`_dispatcher.md`](./_dispatcher.md) §2).
`menu_popup`/`menu_attach` write their result **through the caller-supplied
`MENU *` in `addr_in[1]`/`addr_in[0]`**, not through `addr_out`. The
**`MN_SELECTED` message** (below) is delivered asynchronously through the
16-byte message pipe, *not* as a return of any `menu_*` call.

---

## `MN_SELECTED` message (the payload)

When the user picks an application menu item, the control manager posts a 16-byte
message to the owning application's pipe. Built by `ct_msgup` →
`ap_sendmsg(appl_msg, MN_SELECTED, owner, title, item, treehi, treelo, parent)`
(`gemctrl.c:353-368`, `gemwmlib.c:851-863`):

| `msg[]` word | Value | Meaning |
|:---:|-------|---------|
| `[0]` | `MN_SELECTED` = **10** (`aesdefs.h:227`) | message id |
| `[1]` | `rlr->p_pid` | sender (the AES/control pid) |
| `[2]` | `0` | extra-byte count |
| `[3]` | **`title`** | the **title** object index the item lives under |
| `[4]` | **`item`** | the selected **item** object index |
| `[5]` | `treehi` = `HIWORD(tree)` | menu-tree ptr, high WORD *(`CONF_WITH_MENU_EXTENSION` only; else 0)* |
| `[6]` | `treelo` = `LOWORD(tree)` | menu-tree ptr, low WORD *(ext.; else 0)* |
| `[7]` | `parent` = `get_par(tree,item)` | submenu parent object *(ext.; else 0)* |

> **Correction / completion vs. the previous spec.** The old file gave only
> `msg[3]=title, msg[4]=item` (correct) but omitted `msg[5..7]`. With
> `CONF_WITH_MENU_EXTENSION` (the default build) `msg[5]`/`msg[6]` carry the
> menu-tree pointer (hi/lo) and `msg[7]` the submenu parent object; without the
> extension all three are 0 (`gemctrl.c:347-368`). See D-051.

> **NewTOS binding (TASK/ZVIEW04).** `tree` is the tree `mn_do` returns the
> pick in (`*ptree`, `gemmnlib.c:494-500`): `gl_mntree` for a bar item, and —
> as NewTOS delivers only submenus attached inside the bar's own tree (step 8
> of the tracking loop below) — `gl_mntree` for those as well. `msg[5]`/`msg[6]`
> are the high/low WORD of that pointer **as the client passed it to
> `menu_bar`** (its own address, which is below 2 GiB for every NewTOS user
> address, `docs/sdk/05-abi-pitfalls.md` A2, so the two WORDs carry it whole);
> `msg[7]` is `get_par(tree, item)` (`gemobjop.c:145-161`), read through the
> bar owner's accessor. An application that tells a desktop-menu pick from a
> window-menu pick by these words depends on them: WinDom's `_do_evnt_msg`
> compares `msg[5..6]` with every window's `menu.root`
> (`sdk/windom/overlay/src/evnt_windom.c:74-83`, upstream line 56), so a zero there matched any
> window *without* a menu (`menu.root == NULL`) and routed the pick to that
> window's empty binding table — zView's *File > Quit* did nothing while its
> browser window was open (TASK/ZVIEW04).

> **NewTOS binding — the recipient is the bar owner, wherever the Control
> Manager runs (A117, 2026-10-25).** `hctl_rect` sends an application-menu pick
> to `owner = gl_mnppd` (`gemctrl.c:353,365`), the descriptor that called
> `menu_bar(tree, 1)` (`gemmnlib.c:707,712`). In EmuTOS the Control Manager is a
> process of its own (`ctl_pd`), so which application happens to be waiting
> never enters into it. NewTOS runs the Control Manager **inline** in the
> `evnt_multi(MU_MESAG)` of whichever client is waiting
> ([`event.md`](./event.md), A91) — a desk accessory idling in
> `evnt_multi(MU_MESAG)` runs the track as readily as the application does —
> and the message still goes to `gl_mnppd`, **never to the waiter**. Because
> `gl_mnppd` is a descriptor and the application chain is one descriptor
> (`D.g_int[0]`, `aes/geminit.c:851-877`), the recorded owner is resolved the
> way `appl_write` resolves its pid ([`application.md`](./application.md)
> contract (b)): a waiting chain member is the chain itself; an accessory's
> wait reaches the chain's innermost started member (the program a `Pexec`
> parent is suspended for); an accessory that owns the bar is reached through
> its own descriptor. Before A117 NewTOS posted the pick to the waiter: with an
> accessory waiting in `evnt_multi(MU_MESAG)` a desktop *Desk → Desktop info…*
> landed in the accessory's pipe and the desktop never saw it
> (`measurements/measurement-2026-10-25-a117/RUNINFO.TXT`). `AC_OPEN` was
> always routed to the accessory owner (`mn_getownid`, `gemctrl.c:337-340`).
> Test: `tos-api/tests/aes_a117_mn_selected_owner.rs`.

When the item picked is a **desk-accessory** entry (`title == THEDESK` and the
item is at/after `gl_dafirst`), the control manager instead sends **`AC_OPEN`**
(40) to the accessory owner, with the DA's menu id in `msg[4]` and `msg[5..7]=0`
(`gemctrl.c:337-349`). After acting on `MN_SELECTED`, the application usually
calls `menu_tnormal(tree, title, 1)` to un-highlight the title — and the control
manager itself already clears the title's `SELECTED` bit before/after sending
for the DA case (`gemctrl.c:341`).

---

## The drop-down tracking loop (reference)

The mouse tracking that turns a click on the bar into an `MN_SELECTED` message
lives in the **control manager**, not in the `menu_*` dispatch: `hctl_rect`
(`gemctrl.c:318-371`) calls `mn_do` (`gemmnlib.c:249` extension variant / `:518`
plain variant), a state machine (`START/INTITLE/INITEM/OUTSIDE/SUBMENU_STATE`)
that hit-tests on every rectangle-enter/leave or button transition — **not
only while the button is held**, correcting the phrasing here (A73,
2026-09-20): the bar opens and tracks purely on hover, the instant the
cursor enters `THEACTIVE` with the button up (`START_STATE`,
`gemmnlib.c:286-292`; a stray `MU_BUTTON` seen there is simply ignored,
`:371-372`) — the button only *finalizes* a selection once an item is
reached (point 3 below). It uses `ob_find` to locate the title/item under
the cursor, `menu_down`/`menu_sr` to drop the box and save the screen area
beneath it (`bb_save`/`bb_restore` — into **the one** AES screen buffer
`gsx_malloc` took, `gl_tmp`, the same the alert uses and `wind_get(WF_SCREEN)`
hands out; NewTOS `tos-api/src/aes/gsxif.rs`, A57), and `do_chg(...,
SELECTED, ...)` to hilite the tracked item/title. On button release over an
enabled item it returns the `(title, item)` pair, which `hctl_rect` turns into
the message above. A nested submenu opened from the drop-down does **not**
share that buffer — the drop-down is still down and holds it — but takes a
fresh ST-RAM block per opening (`popup_blit`, `gemmnext.c:136-215`,
`dos_alloc_stram`), freed on the restore; so does `menu_popup`.

**This whole loop is reference** — NewTOS reimplements the tracking over the
object manager (hit-testing, `objc_draw`/`objc_change`) and the VDI raster
save/restore. What is **contract** is the *result*: the correct `(title, item)`
object indices delivered in the `MN_SELECTED` message, the title left highlighted
until `menu_tnormal`, and the screen restored under a closed drop-down.

### Mouse ownership over the track (contract: no `MU_BUTTON` leaks to the app)

The tracking loop above does not just hit-test the mouse — for its whole
duration it also **holds exclusive ownership of the mouse/button event
stream**, away from the application whose menu bar it is servicing. This is
not incidental: it is why the click that selects an item is never separately
delivered to the application as `MU_BUTTON`, only as the one `MN_SELECTED`
message.

1. **Entering the bar hands the mouse to the control manager.** While no
   button is down, `mchange` gives the mouse to `ctl_pd` — the fixed internal
   `AESPD` that runs `ctlmgr()` (`geminit.c:888`, `iprocess("SCRENMGR",
   ctlmgr)`) — the instant the cursor enters the bar's wait rectangle and a
   menu is installed: `if ((gl_mowner != ctl_pd) && (button == 0) &&
   gl_mntree && in_mrect(&gl_ctwait)) gl_mowner = ctl_pd;`
   (`aes/geminput.c:388-389`, inside `mchange`, `:360-391`). This happens
   *before* `ctlmgr`'s own `MU_M1` wait (over `gl_ctwait`) even wakes it into
   `hctl_rect`.
2. **`mn_do` adds a second, stronger lock for the whole track.** On entry it
   calls `ct_mouse(TRUE)` (`aes/gemmnlib.c:277`), which sets `gl_ctmown =
   TRUE` (`aes/gemctrl.c:400-409`, `ct_mouse`). This is a *different* flag
   from `gl_mowner` — it exists purely to freeze `gl_mowner` at `ctl_pd` for
   the duration: `bchange`'s ownership reassignment is wrapped in
   `if (!gl_ctmown && (new == MB_DOWN) && (button == 0)) gl_mowner =
   chkown();` (`aes/geminput.c:334-335`, inside `bchange`, `:328-345`) — while
   `gl_ctmown` is set, a fresh button-down does **not** hand the mouse
   anywhere else, no matter where on screen it lands. `mn_do` releases the
   lock only at the very end, via `ct_mouse(FALSE)` (`gemmnlib.c:513`), which
   clears `gl_ctmown` (`gemctrl.c:410-417`).
3. **Selection fires on a press or a release, depending on how the button
   already stands when the cursor arrives on the item.** Entering
   `INITEM_STATE`, `mn_do` computes its wait predicate from the *live* button
   state: `buparm = (button & 0x0001) ? 0x00010100L : 0x00010101L;`
   (`aes/gemmnlib.c:314`, `CONF_WITH_MENU_EXTENSION` build; the non-extension
   variant mirrors this at `gemmnlib.c:568`). If the button is **up** when the
   item is entered, `mn_do` waits for a **press** to select it; if the button
   is **already down** (the user dragged onto the item with the button held),
   it waits for the **release** instead. Either way, the matching
   `MU_BUTTON` breaks the state loop (`gemmnlib.c:369-376`: `if (menu_state
   != INTITLE_STATE) break;`) and the cleanup path (`:482-513`) restores the
   drop-down (`menu_sr`), returns `(title, item)`, and calls `ct_mouse(FALSE)`.
4. **`hctl_rect` turns the pair into the message and waits out the button.**
   `ct_msgup(MN_SELECTED, gl_mnppd, title, item, treehi, treelo, parent)`
   (`aes/gemctrl.c:352-368`) posts the 16-byte message straight into the
   owning application's pipe via `ap_sendmsg` (`aes/gemctrl.c:81-84`), then
   busy-waits `while (button & 0x0001) dsptch();` (`gemctrl.c:100-101`) so
   `ctlmgr` does not loop again — and in particular does not run
   `w_setactive`/`set_mown` — until the button has physically come back up.
   `set_mown` itself is a no-op while a button is held (`if (!button) { … }`,
   `aes/geminput.c:222-235`), so ownership cannot move to the application
   mid-click even if something else tried.
5. **Leaving the bar *and* the open box does not close the drop-down, and a
   press outside is also swallowed by the control manager (A73,
   2026-09-20).** If the cursor leaves both the current title and the open
   item box without a button event, `mn_do`'s per-iteration hit-test falls
   through to "no man's land" and sets `menu_state = OUTSIDE_STATE`
   (`aes/gemmnlib.c:415-443`); the changed-item guard un-hilites the tracked
   item (`menu_select`/`item_changed`, only-on-change, `:178-186,192-200`,
   called at `:459`), but the *title* is unchanged, so the same guard
   leaves it hilited and skips the `menu_sr(FALSE, …)` restore that would
   drop the box (`:463-464`). The next wait round asks for `MU_BUTTON |
   MU_M1` (re-enter `THEACTIVE`) `| MU_M2` (re-enter the open box,
   `cur_menu`) `| MU_M3` (a showing submenu) (`:293-305`). Re-entering the
   box resumes item tracking exactly as before (`ob_find` against
   `cur_menu`, `menu_state = INITEM_STATE`, `:415-426`), and a press there
   selects the item through the same press-or-release rule as point 3
   above. **A press anywhere else** — outside both the bar and the box —
   breaks the loop as a cancel: a `MU_BUTTON` in any state other than
   `START_STATE`/`INTITLE_STATE` breaks the `while (!done)` loop
   (`:369-374`); the cleanup path un-hilites the title without selecting
   anything (`:509-510`) and `mn_do` returns `FALSE`, sending no message
   (`:515`). That cancelling press still falls inside the step 1–2
   ownership bracket above — `gl_ctmown` was set on entry (`:277`) and is
   only cleared at cleanup (`:513`) — so `bchange` still refuses to move
   `gl_mowner` off `ctl_pd` for it (`aes/geminput.c:334-335`), and, since
   the button never reaches a selecting transition, `set_mown` never runs
   for it either (`:222-235`): the cancelling press is consumed exactly
   like a selecting one, just without a message reaching the application.
   After `mn_do` the press is still `ctl_pd`'s and still held: `ctlmgr`
   gives up the screen, its loop head's `w_setactive` cannot move the mouse
   (`set_mown` refuses with a button down, `:222`), and its next `ev_multi`
   returns on the held button at once (`rlr == gl_mowner`,
   `aes/gemevlib.c:185-198`). It takes the screen again and, if the press is
   over a window (`wm_find > 0`), runs `hctl_window` on it
   (`aes/gemctrl.c:463-470`); on the top window's work area (no control
   point) that is `ct_msgup(0, …)` — no message, then `while (button &
   0x0001) dsptch();` **inside the screen lock** (`:81-102,182,314`). An
   application that asks for the lock meanwhile (`wind_update(BEG_UPDATE)`
   / `BEG_MCTRL`) runs on only after the release, and its `fm_own(TRUE)`
   then finds the button up and takes the mouse (`docs/api/aes/
   window-graphics.md` `wind_update (107)`, contract point 3; GH1 fix 2,
   `tos-api/tests/aes_gh1_update_mutex.rs` case 9).
6. **Out of a submenu into no man's land, the parent item stays; a press
   selects it (A89, 2026-09-21).** When the hit-test falls through with
   `menu_state == SUBMENU_STATE`, `mn_do` keeps `cur_item = last_item`
   before switching to `OUTSIDE_STATE` (`aes/gemmnlib.c:437-438`): the
   parent item stays hilited and the submenu stays displayed (`item_changed`
   is false, `:452-459`; only the submenu item is un-hilited, `:446-448`).
   The next wait adds `MU_M3` (re-enter the submenu, `:298-302`). A press
   there breaks the loop like any `OUTSIDE_STATE` press (`:373-374`), but the
   cleanup now finds `cur_item != NIL`: it undisplays the submenu
   (`:486-490`), restores the box (`:491`) and, the item enabled and
   `menu_state != SUBMENU_STATE`, returns `(cur_title, cur_item)` — the
   **parent** item (`:492-500`) → one `MN_SELECTED` for it. The same cleanup
   makes a press on the parent item itself while its submenu shows
   (`INITEM_STATE`, `cur_submenu == NIL`) a selection of the parent. Out of
   the parent *item* into no man's land (`INITEM_STATE` → `OUTSIDE_STATE`)
   is the ordinary change: `cur_item = NIL`, submenu undisplayed, parent
   un-hilited (`:452-459`), and a press there cancels (point 5).
7. **A DISABLED title never gets a box (A89, 2026-09-21).** `menu_down`
   (`aes/gemmnlib.c:229-244`) saves and draws the drop-down only if
   `do_chg(tree, ititle, SELECTED, TRUE, TRUE, TRUE)` succeeds, and `do_chg`
   refuses a DISABLED object (`:158-160`); `mn_do` reaches `menu_down` only
   through the same `do_chg` (`menu_select`, `:466-467`). Hovering a
   DISABLED title therefore hilites nothing and drops nothing
   (`INTITLE_STATE` with no box); a press there is `buparm ^= 1` and nothing
   else (`:375`); leaving it into no man's land ends `mn_do` without a
   selection (`cur_menu == NIL` → `cur_title = NIL` → `done`, `:406-411`, or
   the explicit DISABLED exit, `:429-434`).
8. **The submenu box's own area picks `smroot`; a DISABLED item opens no
   submenu (A103, 2026-09-21).** `cur_submenu = ob_find(smtree, smroot, 1,
   x, y)` (`aes/gemmnlib.c:417`) answers the box **itself** for a point
   inside it that no child covers — `ob_find` returns the last object found
   (`aes/gemoblib.c:1034-1113`) — and `mn_do` treats that like any submenu
   item: `SUBMENU_STATE` (`:418-422`), `menu_select(smtree, cur_submenu,
   last_submenu, TRUE)` hilites it (`:474-475` → `do_chg` → `ob_change`, the
   box interior's XOR for a black-and-white `G_BOX`, `gemoblib.c:1295-1304`),
   leaving it un-hilites it (`:446-448`), and a press there breaks the loop
   (`:373-374`); the cleanup clears it without a draw (`:488`), undisplays
   the submenu, restores the box and — the **parent** item enabled, its
   SELECTED bit cleared by the same `do_chg` (`:493`) — returns `*ptitle =
   cur_title`, `*pitem = cur_submenu` = `smroot`, `*ptree = smtree`
   (`:494-496`). NewTOS delivers this for a submenu living in the bar's own
   tree (the index round-trips as `MN_SELECTED`'s `msg[4]`); a submenu in a
   separate tree still needs the `msg[5..7]` tree-pointer path (deferred,
   `menu::track`). `display_submenu` runs only inside `if
   (menu_select(tree, cur_item, last_item, TRUE))` (`:470-475`), and
   `menu_select` → `do_chg(…, chkdisabled = TRUE)` refuses a DISABLED object
   (`:158-160`): a DISABLED item with a submenu attached is neither hilited
   nor given its submenu, and a press on it is the cancel (`:493`).

**Net effect and the contract it produces.** For the whole bracket from step 1
through the end of step 5 above, the only process `gl_mowner` ever names is
`ctl_pd`, and
`post_button` delivers to **exactly one** recipient — whichever process
`gl_mowner` currently names (`aes/geminput.c:463-483`). An application parked
in `evnt_multi` has its own pending button-wait sitting on **its own**
`p_cda->c_bsleep` list (armed by `abutton` via `iasync(MU_BUTTON, …)`,
`aes/geminput.c:545-565`, `aes/gemasync.c:154-156`) — a list `post_button(ctl_pd,
…)` never visits. The application is woken only by the `MN_SELECTED` message
itself (`MU_MESAG`); `ev_multi`'s `acancel` then discards the still-unsatisfied
button wait (`aes/gemevlib.c:264`; `acancel` itself: `aes/gemasync.c:217-243`).
So:

> **The press that selects the item, and its later release, never reach the
> application as `MU_BUTTON`; it receives exactly one `MN_SELECTED`.** This
> holds for the ordinary click regardless of whether the item was entered
> with the button up (select-on-press) or already down (select-on-release,
> point 3 above).

**The one documented exception — a level test, not an event, and not gated by
ownership.** `abutton`'s arming check is unconditional: `if (downorup(button,
p)) azombie(e, 1);` (`aes/geminput.c:549-551`) tests the *live* `button`
global with no `gl_mowner`/`gl_ctmown` check at all. If the application only
(re-)calls `evnt_multi` — and so only (re-)arms this wait — *after* the
selecting transition but *while the button is still physically in the state
it is waiting for* (the long-press case: the user has not yet released by the
time the app processes `MN_SELECTED` and loops back into `evnt_multi`), it
gets an immediate, ownership-independent `MU_BUTTON` right there. This is
distinct from — and does not weaken — the guarantee above: it can only ever
fire for a button level the app *itself* observes after the fact, never as a
delivery of the track's own press/release events.

> **Quick-check vs. blocking arm.** [`event.md`](./event.md)'s `evnt_multi`
> step 3 quick-check is gated on `rlr == gl_mowner`
> (`aes/gemevlib.c:184-213`); that gate applies **only** to the quick,
> non-blocking check. The blocking arm described here (`abutton`) has no such
> gate — it is a separate, ownership-blind code path, which is exactly why
> the exception above exists independently of who owns the mouse.

---

## Error conditions

The menu manager has almost no error surface — like the rest of the AES it is
"trusted" code. Negative/false returns:

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `menu_register` with a negative pid | `gemmnlib.c:796,815` | `-1` |
| `menu_register` when all `NUM_ACCS` (6) slots are used | `gemmnlib.c:796,815` | `-1` |
| `menu_tnormal` on a **disabled** title (`chkdisabled=TRUE`) | `gemmnlib.c:158-159` | `0` (state unchanged) |
| `menu_ienable` — no error path (`chkdisabled=FALSE`) | `gemmnlib.c:172-176` | always `1` |
| `menu_attach` on a non-`G_STRING` item (any flag) | `gemmnext.c:539-541` | `0`/`FALSE` |
| `menu_attach(ME_REMOVE, …)` when the item has no submenu | `gemmnext.c:471-481` | `0`/`FALSE` (D-088) |
| `menu_attach(ME_ATTACH, …)` — `gl_submenu` allocation (OOM) fails | `gemmnext.c:484-488` | `0`/`FALSE` (D-088) |
| `menu_attach(ME_ATTACH, …)` — `SMIB` table full (`NUM_SMIBS`=128) | `gemmnext.c:509-512` | `0`/`FALSE` (D-088) |
| `menu_istart` with no attached submenu / bad flag | `gemmnext.c:568-569,584-585` | `0`/`FALSE` |
| `menu_popup` with no item chosen (incl. background-save OOM, D-086) | `gemmnext.c:336,345-349` | `0` |
| any of 36–39 when `CONF_WITH_MENU_EXTENSION == 0` | `gemsuper.c:406-417` | `-1` (unsupported opcode) |

There are **no** TOS negative error codes (`E*`) in the menu manager — returns
are `1` / `0` / slot-id / `-1` only.

---

## Verification criteria

- **`menu_bar` install:** `menu_bar(tree, 1)` sets `gl_mntree = tree`, draws
  `THEBAR`, and returns `1`; `menu_bar(tree, 0)` clears `gl_mntree = NULL` and
  returns `1` (`gemmnlib.c:709-726`).
- **`menu_icheck` sets CHECKED (deferred draw):** `menu_icheck(tree, item, 1)`
  sets the item's `CHECKED` (0x0004) `ob_state` bit and returns `1`;
  `menu_icheck(tree, item, 0)` clears it. **No redraw happens** (`dodraw=FALSE`)
  — verify by state bit, not by pixels (`gemsuper.c:172`).
- **`menu_ienable` toggles DISABLED:** `menu_ienable(tree, item, 0)` sets
  `DISABLED` (0x0008); `menu_ienable(tree, item, 1)` clears it. With bit 15 of
  `item` set, the item is redrawn immediately; without it, deferred
  (`gemsuper.c:175-176`).
- **`menu_tnormal` highlight + disabled guard:** `menu_tnormal(tree, title, 0)`
  sets the title's `SELECTED` bit and redraws (returns `1`);
  `menu_tnormal(tree, title, 1)` clears it. On a `DISABLED` title it returns
  `0` and leaves the state unchanged (`gemmnlib.c:158-159`, `179`).
- **`menu_text` in-place replace:** after `menu_text(tree, item, "New")`, the
  buffer at `tree[item].ob_spec` reads `"New"`; the pointer is unchanged (same
  buffer overwritten) and no redraw occurs (`gemsuper.c:183`).
- **`menu_register` slot / exhaustion:** `menu_register(pid, str)` with a valid
  pid returns a slot `0..5` and records `str` in `g_acctitle[slot]`; the 7th
  concurrent registration (and any negative pid) returns `-1`
  (`gemmnlib.c:796-815`; `NUM_ACCS = 6`).
- **`MN_SELECTED` layout:** on a menu pick, the owner's pipe receives
  `msg[0]=10`, `msg[3]=title obj`, `msg[4]=item obj`; in the default
  (`CONF_WITH_MENU_EXTENSION`) build `msg[5]/msg[6]` = the menu-tree ptr hi/lo
  and `msg[7]` = the submenu parent (`gemctrl.c:353-368`).
- **`menu_popup` result:** `menu_popup(in, x, y, out)` returns `1` and fills
  `out->mn_item` with the chosen object when an item is picked; returns `0` and
  sets `out->mn_keystate = 0` otherwise, including on background-save OOM
  (`gemmnext.c:336,345-349`).
- **`menu_attach` guard:** `menu_attach(ME_ATTACH, tree, item, m)` on an item
  whose `ob_type` is not `G_STRING` returns `0` (`gemmnext.c:539-541`).
- **`menu_attach` encodes into `ob_type`:** after a successful `ME_ATTACH`,
  `tree[item].ob_type`'s upper byte is non-zero (the `SMIB` index + 128) in
  addition to `ob_flags & SUBMENU`; after `ME_REMOVE`, the upper byte is back
  to `0` (`gemmnext.c:524`, `449`).
- **Extension gating:** with `CONF_WITH_MENU_EXTENSION = 0`, opcodes 36–39
  return `-1` (`gemsuper.c:406-417`).

---

## Edge cases discovered in source

1. **Primary index is `int_in[0]`, state is `int_in[1]`.** All of `SHOW_IT`,
   `ITEM_NUM`, `TITLE_NUM`, `MM_PID` are `int_in[0]`; `CHECK_IT`, `ENABLE_IT`,
   `NORMAL_IT` are `int_in[1]` (`aesdefs.h:275-283`). (The previous spec had
   these one slot too high.)
2. **`menu_icheck` does not redraw.** `dodraw = FALSE` — the check mark only
   appears on the next drop-down draw (`gemsuper.c:172`). `menu_tnormal` *does*
   redraw (`dodraw = TRUE`); `menu_ienable` redraws only if item bit 15 is set.
3. **`menu_ienable`'s item bit 15 is the redraw flag, not a title marker.**
   `(ITEM_NUM & 0x8000) != 0` becomes `do_chg`'s `dodraw` argument; the object
   index is `ITEM_NUM & 0x7fff` (`gemsuper.c:175-176`).
4. **`menu_tnormal` silently fails on a disabled title.** `chkdisabled = TRUE`,
   so if the title already has `DISABLED` set, `do_chg` returns `0` and does not
   toggle `SELECTED` (`gemmnlib.c:158-159`). None of `icheck`/`ienable` check
   the disabled state (`chkdisabled = FALSE`).
5. **`menu_text` is an unbounded in-place `strcpy`.** No length check, no
   allocation, no redraw (`gemsuper.c:183`). A longer replacement string
   overruns the original `ob_spec` buffer — programs must keep it ≤ the original
   length and repaint themselves.
6. **`menu_register` stores the caller's string pointer, not a copy.**
   `g_acctitle[slot] = pstr` "like Atari TOS" (`gemmnlib.c:806`) — the title
   string must outlive the registration. The return is the slot index (0..5),
   which is the DA's *menu id* used later to route `AC_OPEN`. `menu_fixup`
   then writes that pointer into the installed tree's Desk entries
   (`gemmnlib.c:118`), where the **application** reads it like any other
   entry of its own menu (WinDom's `menu_exec_cmd` `strstr`s them). NewTOS
   binding: the title is copied, and the application's tree gets the copy's
   address in a page published read-only to every process —
   `docs/api/aes/accessories.md` §9.3 (a) (TASK/ACCTITLE).
7. **`MN_SELECTED` carries the menu-tree pointer in `msg[5..7]`** in the
   default build. Without `CONF_WITH_MENU_EXTENSION` those three words are 0
   (`gemctrl.c:347-368`). Selecting a desk-accessory entry sends **`AC_OPEN`
   (40)** instead of `MN_SELECTED`, with the DA menu id in `msg[4]`
   (`gemctrl.c:337-349`).
8. **`menu_bar(tree, 0)` ignores its `tree` argument** and erases the bar
   recorded in `gl_mntree` from the matching `show=1` (`gemmnlib.c:723-726`).
9. **The extension calls are deliberately minimal.** `menu_settings` stores its
   `MN_SET` fields but the submenu code ignores the timing/scroll values; only
   one level of submenus is supported, attached only to the main bar, and no
   popups-with-submenus (`gemmnext.c:13-28`, `594`).
10. **Menu selection runs in the control manager, not the `menu_*` trap.** The
    `menu_*` calls only set object state / install the tree; the actual pick →
    `MN_SELECTED` happens later inside `hctl_rect`/`mn_do` while the app is in
    `evnt_multi` (`gemctrl.c:318-371`).
11. **A title's drop-down box is not its child object.** It is the *N*-th
    `ob_next` sibling under `tree[THESCREEN].ob_tail` (a hidden container),
    *N* = the title's ordinal position among titles (`menu_sub`,
    `gemmnlib.c:71-86`). See D-085 — this is a correction, not an addition,
    to the previous spec's tree-shape description.
12. **`menu_attach` writes `ob_type`, not just `ob_flags`.** Attaching encodes
    the `SMIB` array index into the item's `ob_type` upper byte
    (`gemmnext.c:524`); detaching clears it (`gemmnext.c:449`). Re-attaching
    over an existing submenu detaches the old one first (replace semantics,
    `gemmnext.c:471-477`); `ME_REMOVE` on an item with no submenu, an OOM
    on the (lazily-allocated, 128-entry) `SMIB` table, and a full `SMIB`
    table all return `FALSE` (`gemmnext.c:471-481,484-488,509-512`). See D-087/D-088.
13. **`menu_popup`'s `submenu_event` does not exist.** `handle_popup`
    (`gemmnext.c:245-299`) sets `keystate` itself; the `/* set by
    submenu_event() */` comment at `gemmnext.c:342` is stale EmuTOS source
    text referencing a function that is absent from the entire EmuTOS tree.
    `mn_popup` also blocks until any held mouse button releases before
    showing the popup, and silently returns "no pick" if the background-save
    allocation fails (`gemmnext.c:314,157-163`). See D-086.

---

## Dependencies

### Requires (must be initialized / functional)

- **The Object Manager** ([`object-form.md`](./object-form.md)) — the menu bar
  *is* an OBJECT tree; drawing (`ob_draw`), state change (`ob_change`), and
  hit-testing (`ob_find`) are how every menu operation renders. `menu_bar`,
  `do_chg`, and the tracking loop all call it.
- **The Event Manager** ([`event.md`](./event.md)) — `MN_SELECTED`/`AC_OPEN`
  are delivered through the 16-byte message pipe that `evnt_mesag`/`evnt_multi`
  read; the tracking loop runs on the mouse/button events the event manager
  feeds. The internal third mouse rectangle `MU_M3` (0x0100) belongs to this
  menu-extension code (see [`event.md`](./event.md) §config).
- **The VDI** raster save/restore (`bb_save`/`bb_restore`/`popup_blit`, the
  reference save-the-area-under-a-dropped-menu machinery) — a `vro_cpyfm`-class
  operation in NewTOS.
- **The AES process model / message pipe** ([`_state.md`](./_state.md)) —
  `gl_mnppd`/`rlr`, the DA owner descriptors `desk_ppd[]`, `ap_sendmsg`.
- **`menu_register` / the DA machinery** needs multiple AES processes (the
  accessories) — the same DA dependency `appl_exit`/`wait_for_accs` has
  ([`application.md`](./application.md)); absent at single-app bring-up.

### Used by

- **Every GEM application with a menu bar** calls `menu_bar` once at start-up
  and `menu_tnormal` after each `MN_SELECTED`; `menu_icheck`/`menu_ienable`/
  `menu_text` maintain item state.
- **The desktop** (`desk/`) owns the primary menu bar (this is the NewTOS
  desktop's bar).
- **Desk accessories** call `menu_register` to add their entry to the *Desk*
  menu.
- The **control manager** (`gemctrl.c`) drives the tracking and posts the
  messages; the **event manager** delivers them.

---

## Related data structures

- **Menu OBJECT tree** — a standard `OBJECT` tree (see
  [`object-form.md`](./object-form.md) and `docs/api/structures/` for the
  `OBJECT` layout) with the fixed index roles `THESCREEN`(0, root `G_IBOX`,
  **two** children) / `THEBAR`(1, `THESCREEN`'s `ob_head`) / `THEACTIVE`(2) /
  `THEDESK`(3, first title) (`gemmnlib.c:46-48`, `gemctrl.c:42`). Titles
  (`G_TITLE`) are children of `THEACTIVE`; drop-down boxes (`G_BOX`) are
  **not** children of their titles — they are the `ob_next` sibling chain
  under `THESCREEN`'s `ob_tail` (a hidden container), positionally
  correlated by title order (`menu_sub`, `gemmnlib.c:71-86`; **corrected**,
  see D-085); each drop-down's own children are the items (`G_STRING`). The
  `ob_state` bits `CHECKED`/`DISABLED`/`SELECTED` and the `ob_flags` bit
  `SUBMENU` (`obdefs.h:82-107`) are the contract fields the menu calls flip;
  `menu_attach` additionally writes the `SMIB` index into `ob_type`'s upper
  byte (`gemmnext.c:524`, see D-087).
- **`MN_SELECTED` / `AC_OPEN` messages** — the 16-byte `WORD msg[8]` layout
  above (`gemctrl.c:353-368`, `aesdefs.h:227,242`). Shared with the event and
  application managers ([`event.md`](./event.md), [`_state.md`](./_state.md) §3).
- **`MENU`** (`aes/gemmnext.h:16-22`) — popup/submenu descriptor:
  `OBJECT *mn_tree`, `WORD mn_menu` (enclosing box obj), `WORD mn_item` (start
  item), `WORD mn_scroll`, `WORD mn_keystate`. Used by `menu_popup`/`menu_attach`.
- **`MN_SET`** (`aes/gemmnext.h:27-33`) — `menu_settings` block: `LONG display`
  (submenu-display delay ms), `LONG drag` (submenu-drag delay ms), `LONG delay`
  (single-click scroll delay ms), `LONG speed` (continuous-scroll delay ms),
  `WORD height` (default scroll height in items, clamped to `[5, gl_rfull.g_h
  / gl_hchar]` on set — `clamp_height`, `gemmnext.c:83-95`). Get/set with
  non-negative-only writes.
- **`SMIB`** (`aes/struct.h:180-186`) — submenu info block: `WORD s_usage`
  (refcount), `OBJECT *s_tree`, `WORD s_menu` (enclosing box obj), `WORD
  s_start`. Stored in `gl_submenu[NUM_SMIBS]` (`NUM_SMIBS = 128`,
  `aes/struct.h:53`), lazily allocated on the first `menu_attach(ME_ATTACH,
  …)`. **Reference** — the array/index-encoding scheme is EmuTOS-internal;
  only the item's `ob_type`/`ob_flags` mutation (D-087) and the `menu_attach`/
  `menu_istart` return values are contract.
- **DA-slot model** — `D.g_acctitle[NUM_ACCS]` (title ptrs), `desk_ppd[NUM_ACCS]`
  (owner descriptors), `D.g_accreg` (count), `acc_display[]` (display-slot →
  menu-id lookup), `gl_dafirst` (first DA object in the tree); `NUM_ACCS = 6`
  (`sysconf.h:37`). **Reference** — NewTOS reimplements the storage; the
  *observable* contract is the slot-id (0..5)/-1 return and the `AC_OPEN`
  routing.

  > **Internal oddity found, not contract-relevant.**
  > `build_menuid_lookup`'s fill-remainder-with-`-1` loop
  > (`gemmnlib.c:778-781`) double-increments its index —
  > `for ( ; slot < NUM_ACCS; slot++) { acc_display[slot++] = -1; }` — so it
  > sets only every *other* `acc_display[]` slot above the registered count
  > to `-1`, leaving the skipped ones at their previous value (`0` from
  > static init, the first time). This looks like a real EmuTOS off-by-one,
  > but it is **unreachable through the documented API**: `mn_getownid`'s
  > `item` argument (`gemctrl.c:340`) is always an object index `ob_find`
  > returned from the *actual* tree `menu_fixup` built, which only ever
  > contains `D.g_accreg` real DA objects — so `acc_display[]` is never read
  > at an index beyond what the first (correct) loop already filled. Noted
  > for completeness; **not** given a D-number since it changes no observable
  > `menu_*` behaviour and NewTOS reimplements this storage freely anyway.

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of the menu opcode
  numbers (30–39), the `int_in[0]`/`int_in[1]` slot assignment, the
  `menu_ienable` item-bit-15 redraw flag, the `MN_SELECTED` `msg[3..7]` layout,
  the `menu_register` slot-id/-1 return, and the `MENU`/`MN_SET` field layouts is
  **deferred** until the documentation PDFs are available. (Compendium documents
  `menu_ienable`'s high item bit and the `msg[5..7]` tree pointer for AES 3.3;
  this build matches — verification deferred. See D-051.)

### AES managers

- [`object-form.md`](./object-form.md) — the menu bar is an OBJECT tree;
  `objc_draw`/`objc_change`/`objc_find` do all drawing, state change, and
  hit-testing. `menu_text` writes `ob_spec` directly; repaint is via `objc_draw`.
- [`event.md`](./event.md) — `evnt_mesag`/`evnt_multi` deliver `MN_SELECTED`;
  the tracking loop consumes the same mouse/button events; `MU_M3` is the
  menu-extension internal rectangle.
- [`application.md`](./application.md) — the message pipe (`ap_sendmsg` →
  `ap_rdwr`) and the DA process model `menu_register` shares.
- [`_dispatcher.md`](./_dispatcher.md) — selector-200 call model, `control[]`
  counts, `int_out[0]` return, the `CONF_WITH_MENU_EXTENSION` gating of 36–39,
  the unsupported-opcode `-1` path.
- [`_state.md`](./_state.md) — `gl_mntree`/`gl_mnppd`, the DA slots, the 16-byte
  message pipe.

### VDI

- The save/restore of the screen area under a dropped menu / popup
  (`bb_save`/`bb_restore`/`popup_blit`) maps to a VDI raster copy
  (`vro_cpyfm`-class). Do not edit the VDI docs.

---

## Configuration / version notes

- **`CONF_WITH_MENU_EXTENSION`** (default **1**, `include/config.h:1305`) gates
  opcodes **36–39** and the whole of `aes/gemmnext.c`, plus the `msg[5..7]`
  tree-pointer/parent fields of `MN_SELECTED` (`gemctrl.c:347-368`) and the
  internal `MU_M3` mouse rectangle. With it off, 36–39 return `-1` and
  `MN_SELECTED` carries only `msg[3]/msg[4]`. This flag is also one of the four
  that push `AES_VERSION` to 0x0330/0x0340 (see
  [`application.md`](./application.md)).
- The extension implementation is deliberately **minimal** (`gemmnext.c:13-28`):
  submenus only on the main bar, one level deep, no popup-submenus, scroll flag
  ignored. Popups, one-level submenus, and `menu_settings` storage do work.
- Opcodes **30–35** are unconditional across all EmuTOS builds and all TOS
  versions (the original TOS 1.x menu set); 36–39 are AES 3.30 additions.
- `NUM_ACCS = 6` (`include/sysconf.h:37`) bounds `menu_register` to 6 DA slots;
  this is a build constant, not an ABI value a program should assume beyond
  "register may fail with -1".

---

## NewTOS implementation notes

(Empty during Phase 00.)

- **The menu manager is mostly the object manager + one tracking loop.** Because
  the bar **is** an OBJECT tree, `menu_icheck`/`menu_ienable`/`menu_tnormal`
  reduce to `objc_change` on one `ob_state` bit (respecting each call's
  redraw/disabled-guard contract), and `menu_text` to writing `ob_spec` +
  repaint. The **only genuinely new logic** is the drop-down mouse-tracking
  state machine that produces `MN_SELECTED` — reimplemented over NewTOS
  hit-testing + the event pipe, not ported from `mn_do`.
- **The title→drop-down correlation (D-085) must be honoured by whatever
  parses/produces menu-bar resources for NewTOS**, since it defines the
  on-disk/in-memory tree shape any GEM menu-bar resource has: title *N*'s
  drop-down is the *N*-th sibling under the tree root's second child, not
  that title's own child. If NewTOS gains its own resource compiler/editor
  for menu-bar trees, it must emit this exact shape (or NewTOS's own
  menu-bar walker must special-case its own, more natural in-memory layout
  — an explicit ADR-worthy choice, since "reimplement freely" only covers
  the *algorithm*, not third-party `.RSC` files that already exist in this
  shape).
- **Bring-up order:** original 30–34 (bar + item state) first, `menu_register`
  and the extensions (36–39) later with the DA / submenu machinery — matching
  the [`application.md`](./application.md) DA-deferral. `menu_register` returning
  a real slot needs multiple AES processes; until then it is
  honest-unsupported (Rule 2), not a stub returning a fake slot.
- **Save/restore under a dropped menu** is a VDI raster op (`vro_cpyfm`); the
  top-of-screen bar occupies the system-font row height (`gl_hchar`/`gl_hbox`
  from `graf_handle`, see [`window-graphics.md`](./window-graphics.md)).
- **Pointers in `addr_in[]` (the tree, the text string, the `MENU`/`MN_SET`
  blocks) are user (application) pointers** — apply the same boundary handling
  as other AES `addr*` reads (see [`_dispatcher.md`](./_dispatcher.md) §NewTOS
  notes). The m68k-vs-native `OBJECT`/`MENU` pointer widths follow the caller
  ABI (see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md) "Native vs m68k
  struct variants").
- **`menu_text`'s in-place overwrite** must be preserved as contract (programs
  rely on same-or-shorter replacement into the same buffer); NewTOS should keep
  the same-buffer semantics but may bound the copy to the original `ob_spec`
  allocation to avoid the EmuTOS overrun — an ADR-worthy safety choice that does
  not change observable behaviour for well-formed callers.
