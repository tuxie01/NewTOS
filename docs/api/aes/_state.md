# AES shared state — process descriptors, ids, the message pipe

> **PHASE-00b D8 (AES).** Source: EmuTOS @ `1a7484ab` —
> `aes/aesvars.h` (private AES globals), `aes/geminit.c` (start-up init),
> `aes/struct.h` (`AESPD`/`QPB`/`EVB`), `aes/gemlib.h` (`THEGLO`/`AESPROCESS`),
> `aes/gempd.c` (descriptor lookup), `aes/gemqueue.c` (the message pipe).
>
> Referenced by every AES manager spec for the shared identity/queue model.
> Per-call state (the `AESGLOBAL` `global[]` array) is documented in
> [`application.md`](./application.md); the trap/marshalling in
> [`_dispatcher.md`](./_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** the *observable* facts — the ap_id space (**0** = the whole
> application chain: desktop, launched program, every nested `Pexec` child;
> **1** = a sender stamp on `msg[1]` for Screen-Manager-originated messages
> only, not a process a program can address; **2+** = desk accessories — see
> [`application.md`](./application.md) §"AES process identity and pid",
> corrected 2026-09-20/A62+A56), `ap_count = 1` reported, the 8-char
> space-padded name format, the 16-byte standard message, the `-1` not-found
> convention.
> **Reference (NewTOS reimplements freely):** the `AESPD`/`EVB`/`QPB` structs,
> the ready/deferred/not-ready run lists, `dsptch`/`ev_block`, and the fixed
> `p_queue[128]` layout — these are the EmuTOS cooperative-scheduler internals,
> not something a program observes.

---

## 1. Process descriptors and the run lists

The AES models each application (and each desk accessory) as an **`AESPD`**
process descriptor (`aes/struct.h:119-155`). At start-up
(`geminit.c:810-877`):

- `num_accs = count_accs()` desk accessories are counted (`geminit.c:810`);
- `totpds = num_accs + 2` descriptors exist. **Correction (2026-09-20,
  A62/A56):** the previous wording here — "pids 0 and 1 are the two AES
  system processes … pids 2…totpds-1 are desk accessories / the running
  application" — put the running application in the wrong bucket. Per
  `geminit.c:850-888`: **pid 0** is `D.g_int[0]`, the descriptor the
  **entire application chain** runs on (the desktop, every shell-launched
  program, every `Pexec` child of it — `rlr` is never reassigned by a nested
  `Pexec`, `gemshlib.c:572`); **pid 1** is `D.g_int[1]`, a genuine second
  process running the **Screen Manager** (`SCRENMGR`/`ctlmgr`); **pids
  2…totpds-1** are desk accessories only (`geminit.c:236-257`, `struct.h:36`
  `NUM_PDS = NUM_ACCS + 2`). Full model:
  [`application.md`](./application.md) §"AES process identity and pid";
- `curpid` hands out pids sequentially (`gempd.c:60-70`, `geminit.c:875-877`).

The descriptors are held inside the single AES global area `THEGLO D`
(`geminit.c:141`, `aes/gemlib.h:118-144`): `D.g_int[2]` for the two system
processes and `D.g_acc[num_accs]` for accessories (`gemlib.h:118,143`).
`pd_index(i)` maps an index to its descriptor (`gempd.c:32-35`).

**Run-list roots** (`aes/aesvars.h:17-20`, `geminit.c:131`):

| Global | Role |
|--------|------|
| `rlr` | **run-list root** — the currently running AES process descriptor |
| `drl` | deferred/ready list |
| `nrl` | not-ready list (blocked processes) |
| `eul` | free `EVB` list; `dlr`/`zlr` deferred/zombie event lists |
| `indisp` | non-zero while inside `dsptch` (`aesvars.h:38`) |

`rlr` is what `appl_init`/`appl_exit` read and write (`rlr->p_pid`,
`rlr->p_flags`). NewTOS's `rlr` equivalent is "the current AES process".

**`rlr` is never reassigned by a nested `Pexec`/`dos_exec`**
(`aes/gemshlib.c:572`; full derivation in [`shell-loop.md`](./shell-loop.md)
§0/§4) — this is why the application chain (desktop → launched program → its
`Pexec` children, arbitrarily deep) all runs as AESPD 0, not a separate
descriptor per program. See [`application.md`](./application.md) §"AES
process identity and pid" (2026-09-20, A62/A56) for the reported-value
consequences (`ap_id`, `msg[1]`, `WF_OWNER`).

---

## 2. Application id (`ap_id`)

The application id returned by `appl_init` and stored in `global[2]` is simply
`AESPD.p_pid` (`gemsuper.c:94`, `struct.h:129`). Properties (contract):

- **Correction (2026-09-20, A62/A56):** not "0 and 1 are system, DAs/apps are
  2+" — **0** is the application (the whole chain, including nested `Pexec`
  children), **1** is the Screen Manager's `msg[1]` sender stamp (not a
  process a program can be), **2+** are desk accessories. See
  [`application.md`](./application.md) §"AES process identity and pid".
- `ap_count` reported by `appl_init` is hard-coded **1** (`gemsuper.c:93`) even
  though EmuTOS runs several AES processes — the *reported* count is 1.
- `appl_find` resolves an 8-char name → pid (or -1); `appl_read`/`appl_write`
  resolve a pid → descriptor via `fpdnm(NULL, pid)` (`gempd.c:38-57`).

**Name format** (`gempd.c:77-86`, `p_nameit`): the descriptor's `p_name`
(`AP_NAMELEN = 8`, `struct.h:111,125`) is the first ≤8 filename chars, stopping
at the first `.` (extension stripped), **space-padded to 8** with `' '`.
`appl_find` compares all 8 bytes with `strncmp` (`gempd.c:48`).

---

## 3. The message pipe (16-byte standard message)

Each descriptor owns a fixed queue: `p_queue[QUEUE_SIZE]` with
`QUEUE_SIZE = 128` (`struct.h:57,152`), a cursor `p_qaddr` and byte count
`p_qindex` (`struct.h:150-151`; initialised `p_qaddr = p_queue`,
`p_qindex = 0` at start-up, `geminit.c:864-865`).

- **`appl_write` (`MU_SDMSG`)** appends a message to the *target's* queue;
  **`appl_read` (`MU_MESAG`)** removes one from the *caller's* queue
  (`gemaplib.c:69-89`, `gemqueue.c:aqueue`/`doq`).
- The **GEM standard message is 16 bytes** = a `WORD msg[8]` array; `msg[0]` is
  the message id (`MN_SELECTED`/`WM_*`/`AC_*`, `include/aesdefs.h:227-243`),
  `msg[1]` = sender ap_id, `msg[2]` = extra bytes, `msg[3..7]` message-specific.
- `ap_rdwr` has a fast 16-byte path (no blocking) when the queue holds exactly
  16 bytes; otherwise it blocks on `ev_block` (`gemaplib.c:77-88`).
- The same pipe backs `evnt_mesag`/`evnt_multi` (`gemevlib.c:116`) — the event
  and application managers share it. See [`event.md`](./event.md).

`doq` coalesces redraw messages (unions `WM_REDRAW` rects for the same window,
overwrites pending `WM_ARROWED`) — a reference-level pipe optimisation
(`gemqueue.c:44-72`).

---

## 4. Process flags (`p_flags`)

`aes/struct.h:115-117`:

| Flag | Value | Meaning |
|------|------:|---------|
| `AP_OPEN`    | 0x0001 | application is between `appl_init` and `appl_exit` |
| `AP_MESAG`   | 0x0002 | application has waited for a message |
| `AP_ACCLOSE` | 0x0004 | application has seen an `AC_CLOSE` message |

`appl_init` sets `AP_OPEN`; `appl_exit` clears it (`gemaplib.c:60,260`).
`evnt_mesag`/`evnt_multi` set `AP_MESAG`/`AP_ACCLOSE`; `appl_exit` uses
`wait_for_accs(AP_ACCLOSE)` to let DAs drain their close (`gemaplib.c:255`).

---

## 5. Other shared AES globals used by the application manager

| Global | Source | Used by |
|--------|--------|---------|
| `gl_nplanes` | `geminit.c` (screen planes) | `appl_init` → `ap_planes` |
| `rs_trees[DESKTOP]` | resource manager | `appl_init` → `ap_private` (DESKTOP root `ob_spec`) |
| `D` (`THEGLO`) | `geminit.c:141` | `appl_init` → `ap_3resv = &D`; `D.g_valstr` scratch used by `appl_exit` |
| `gl_play`/`gl_recd`/`gl_rbuf`/`gl_rlen` | `gemaplib.c:45-48` | `appl_tplay`/`appl_trecord` tape state |

---

## 6. Event-manager input, timer, and double-click state

The event manager ([`event.md`](./event.md)) reads a separate set of AES
globals that mirror the current mouse / keyboard state and the timing config.
These are **reference** internals (fed by the input path and the system tick);
what a program observes is the values the `evnt_*` calls report, not the globals.

| Global | Decl. | Meaning | Read by |
|--------|-------|---------|---------|
| `xrat`, `yrat` | `geminput.h:16` | live mouse x, y | `ev_rets`, `in_mrect` |
| `button` | `geminput.h:16` | live mouse-button state | `ev_rets`, `ev_mouse`, `evnt_multi` quick-check |
| `kstate` | `geminput.h:16` | keyboard shift state (`EV_KS`): `gsx_kstate()` = `Kbshift(-1) & 0x000f`, bits 0-3 only, set by `chkkbd`/`kchange` (`gemdisp.c:165-176`, `geminput.c:288-293`; A126) | `ev_rets`, `gr_mkstate` |
| `mclick` | `geminput.h:16` | last button click count (`EV_BRET`) | `evnt_multi`/`evnt_button` |
| `mtrans` | `geminput.h:16` | pending button-transition flag (>1 ⇒ use the `pr_*` snapshot) | `ev_rets` |
| `pr_xrat`/`pr_yrat`/`pr_button`/`pr_mclick` | `geminput.h:17` | transition-time snapshot of the above | `ev_rets`, `evnt_multi` |
| `gl_mowner` | `geminput.h:19` | the AES process that currently owns the mouse | `evnt_multi` (button/rect quick-checks gated on `rlr == gl_mowner`); target of `post_button`/`post_mouse` (`geminput.c:446-483`) |
| `ctl_pd` | `geminput.c:53` | fixed internal `AESPD` for the "SCRENMGR" control-manager process (runs `ctlmgr()`, created at `geminit.c:888`; also `gl_mowner`'s initial value) | `chkown`, `mchange` (bar-entry handoff), `bchange` (reassignment target check) |
| `gl_ctmown` | `gemctrl.c:50` | TRUE while the control manager holds an *extra* mouse lock over `gl_mowner` for the duration of a menu-bar tracking loop (`ct_mouse(TRUE)`/`ct_mouse(FALSE)` in `mn_do`, `gemmnlib.c:277,513`; the same primitive is also used by `form_alert`'s own tracking, `gemfmalt.c:379,386`, out of scope here) | `bchange` (suppresses its `gl_mowner` reassignment, `geminput.c:334`); `ct_chgown` (suppresses `set_mown`, `gemctrl.c:395-396`); set/cleared by `ct_mouse` (`gemctrl.c:400-417`) |
| `gl_ticktime` | `gemevlib.c:36` | ms per system tick (`gsx_tick`, `geminit.c:704`) | `ev_timer`, `ev_dclick` (ms→ticks) |
| `gl_dcindex` | `gemevlib.c:34` | current double-click **speed index** 0..4 (returned by `evnt_dclick`) | `ev_dclick` |
| `gl_dclick` | `gemevlib.c:35` | click-window in **ticks** (`gl_dcrates[idx]/gl_ticktime`) | `b_click` (multi-click timing) |
| `gl_dcrates[5]` | `gemevlib.c:33` | rate table `{450,330,275,220,165}` ms, indices 0..4 | `ev_dclick` |
| `rlr->p_msg` | `struct.h:142-144` | control-manager fast-path `WM_ARROWED` (`action`/`wh`) that bypasses the queue | `ev_mesag` |

- **`ctl_pd`'s pid (1) is what `ap_sendmsg` stamps into `msg[1]` for every
  control-manager-originated `WM_*`/`MN_SELECTED`/`AC_OPEN` message** — see
  [`application.md`](./application.md) §"AES process identity and pid"
  (2026-09-20, A62/A56) for the exact call sites and the contrasting `msg[1]
  = 0` case (`WM_REDRAW`, and a non-queued `WM_ARROWED` synthesised by the
  reader itself).
- The click-count / double-click timing lives in `b_click`/`b_delay`
  (`geminput.c:114-179`): a first click starts a `gl_dclick`-tick window; further
  clicks into the same state increment `gl_bclick` and extend the window; mouse
  motion of more than a couple of pixels cancels it (`mchange`,
  `geminput.c:360-368`). The **default speed is index 3** (220 ms),
  `ev_dclick(3, TRUE)` at start-up (`geminit.c:708`).
- **Contract vs. reference:** the *values* the event calls report — mouse x/y,
  button/shift state, click count, and the double-click speed index and its ms
  semantics — are contract; the `xrat`/`mtrans`/`pr_*`/`gl_mowner` mirrors and
  the `b_click` timing machinery are EmuTOS internals NewTOS reimplements over
  its own USB-HID/IKBD input path and system tick. One *effect* of the
  `gl_mowner`/`gl_ctmown`/`ctl_pd` bookkeeping is contract, though: while the
  control manager holds `gl_ctmown` for a menu-bar track (see
  [`menu.md`](./menu.md) "The drop-down tracking loop"), the mouse-down that
  produces the eventual `MN_SELECTED`/`AC_OPEN` message, and its matching
  mouse-up, are never separately delivered to any application as `MU_BUTTON` —
  only the single resulting message is. The registers that produce this
  (`gl_ctmown`/`ctl_pd`) are reference; the non-delivery of that click is not.

---

## 6a. Menu-manager state — the active menu bar and DA registration

The menu manager ([`menu.md`](./menu.md)) keeps a small amount of shared state
identifying the currently installed menu bar and the registered desk
accessories. These are **reference** internals (fed by `menu_bar`/`menu_register`);
what a program observes is the drawn bar, the `MN_SELECTED`/`AC_OPEN` messages,
and the `menu_register` slot-id/-1 return.

| Global | Decl. | Meaning | Read/written by |
|--------|-------|---------|-----------------|
| `gl_mntree` | `gemmnlib.c:62` | ptr to the **active menu OBJECT tree** (`NULL` = no bar) | set/reset by `mn_bar` (`menu_bar`); read by the control manager's tracking loop `mn_do` |
| `gl_mnppd` | `gemmnlib.c:63` | the AES process descriptor that **owns** the current bar | set by `mn_bar`; used as the `MN_SELECTED` recipient (`gemctrl.c:353`) |
| `gl_ctwait` | `gemctrl.h:13` | the control-manager mouse-wait rectangle: `THEACTIVE`'s extent when a bar is up (`gemmnlib.c:717`), else `gl_rmenu` = the menu-bar row `(0, 0, gl_width, gl_hbox)` (`gemgraf.c:427`; `gemctrl.c:438`, `gemmnlib.c:727`) — *not* the full desktop (corrected A87, 2026-09-21; see `menu.md` § `menu_bar`) | `mn_bar` (`gemmnlib.c:715-726`); read by `mchange` (`geminput.c:388`) and `ctlmgr` (`gemctrl.c:456`) |
| `gl_dafirst` | `gemmnlib.c:68` | object # of the first desk-accessory entry in the tree | `menu_fixup`; the pick router (`gemctrl.c:337`) |
| `D.g_accreg` | `gemlib.h` (`THEGLO`) | count of registered DAs (bounded by `NUM_ACCS`) | `mn_register`/`mn_init` |
| `D.g_acctitle[NUM_ACCS]` | `gemlib.h` (`THEGLO`) | the DA menu-item **title string pointers** (stored, not copied) | `mn_register`/`mn_init` (`mn_cleanup` reads `desk_ppd[]`, not this array, `gemmnlib.c:745-748`; corrected 2026-10-17) |
| `desk_ppd[NUM_ACCS]` | `gemmnlib.c:65` | the owner process descriptor per DA slot | `mn_register`; `mn_getownid` (route `AC_OPEN`) |
| `acc_display[NUM_ACCS]` | `gemmnlib.c:66` | display-slot → DA menu-id lookup table | `build_menuid_lookup`; `mn_getownid` |
| `gl_submenu`/`gl_submenu_hwm` | `gemmnext.c:57-58` | submenu-info (`SMIB`) array (`CONF_WITH_MENU_EXTENSION`) | `menu_attach`/`menu_istart`; freed by `mn_cleanup` |

- **DA-slot model (contract vs. reference).** `NUM_ACCS = 6`
  (`include/sysconf.h:37`) → six DA slots. `menu_register` returns the slot index
  `0..5` (the DA's *menu id*) or `-1` when full/invalid — **the slot-id/-1 return
  is contract**; the `g_acctitle[]`/`desk_ppd[]`/`acc_display[]` storage and
  `gl_mntree`/`gl_mnppd` pointers are EmuTOS internals NewTOS reimplements.
- **Single-app note.** `menu_register` and the `AC_OPEN` routing presume
  concurrent AES processes (the accessories); at NewTOS single-app bring-up there
  are none, so this state is inert until the DA machinery exists (cf.
  [`application.md`](./application.md) `wait_for_accs`).

---

## 6b. Window-manager state — the window list, the top window, the rectangle list

The window manager ([`window-graphics.md`](./window-graphics.md)) keeps the set
of windows, their z-order, and the visible-region (rectangle) lists. These are
**reference** internals (fed by `wind_create`/`open`/`close`/`set`); what a
program observes is the `wind_get`/`wind_set` field values, the `WM_*` messages,
and the `WF_FIRSTXYWH`/`NEXTXYWH` rectangle sequence.

| Global | Decl. | Meaning | Read/written by |
|--------|-------|---------|-----------------|
| `D.w_win[NUM_WIN]` | `gemlib.h:66-86` (`WINDOW`) | the window records: `w_flags` (`VF_INUSE`/`VF_ISOPEN`/`VF_BROKEN`), `w_owner` (AESPD), `w_kind`, `w_pname`/`w_pinfo`, FULL/WORK/PREV/CURR rects, `w_hslide`/`w_vslide`/`w_hslsiz`/`w_vslsiz`, `w_rlist`/`w_rnext` | `w_setup`/`wm_create`/`wm_get`/`wm_set`; handle 0 = desktop (`wm_init`) |
| `W_TREE[NUM_WIN]` | `gemwmlib.c:57` | on-screen **window z-order tree** (root `ROOT`, siblings = open windows; `ob_tail` = topmost) | `w_obadd`/`ob_delete`/`ob_order`; `wind_find` = `ob_find(gl_wtree,0,2,…)` |
| `gl_wtree` | `gemwmlib.c:1298` | ptr to `W_TREE` (the desktop/window root tree) | `wind_find`, `draw_change`, `newrect` |
| `gl_wtop` | `gemwmlib.c:51` | handle of the **topped (active)** window, or `NIL` if none (then desktop) | `w_top()`, `w_bldactive` (istop), `draw_change`; reported by `WF_TOP` |
| `W_ACTIVE[NUM_ELEM]` / `gl_awind` | `gemwmlib.c:58,52` | scratch **frame OBJECT tree** rebuilt per draw by `w_bldactive` (`W_BOX`…`W_HELEV`, `gemlib.h:38-56`) | `w_bldactive`/`w_cpwalk`/`ob_draw`; hit-tested by the control manager |
| `w_rlist`/`w_rnext` (per window) | `gemlib.h:84-85` | the window's **owner rectangle list** (visible non-overlapping pieces) + the `WF_NEXTXYWH` cursor | `newrect`/`mkrect`/`w_owns`; `wind_get(WF_FIRSTXYWH/NEXTXYWH)` |
| `rul` / `D.g_olist[NUM_ORECT]` | `gemwrect.c:36`, `NUM_ORECT=NUM_WIN·10` | the `ORECT` **free list** the rectangle-list allocator draws from | `or_start`/`get_orect`/`newrect` |
| `gl_newdesk`/`gl_newroot` | `gemwmlib.c:54-55` | the desktop-**background** OBJECT tree + root object (set by `wind_set(WF_NEWDESK)`) | `w_drawdesk`, `w_redraw_desktop` |
| `wind_spb` | (AES sync block) | the `wind_update` **screen/window-tree lock** (counting mutex `sy_tas`) | `wm_update(BEG/END_UPDATE)`, `wm_open`/`close`/`set` |
| `ml_ocnt` | (mouse-ctrl lock) | mouse-control ownership count | `wm_update(BEG/END_MCTRL)` → `fm_own`, `wm_new` |
| `gl_wchar`/`gl_hchar`/`gl_wbox`/`gl_hbox` | `gemgraf.c:40-52`, set `:405-415` | system-font cell + gadget-box width/height (`gl_hbox=gl_hchar+3`) | `graf_handle`, `w_bldactive`, `wind_calc` |
| `gl_handle` | `gemgraf.c:52` | the AES **physical VDI workstation handle** | `graf_handle`; every AES VDI call (`contrl[6]`) |
| `mouse_cursor[8]` / `gl_mouse` / `gl_prevmouse` | `geminit.c:124-128` | the 8 preset mouse forms + current/previous form | `graf_mouse` (`gr_mouse`) |

- **Rectangle-list contract (the redraw invariant).** `newrect` rebuilds each
  window's `w_rlist` by breaking its true rect against every higher window
  (`mkrect`/`brkrct`/`mkpiece`, up to four TOP/LEFT/RIGHT/BOTTOM pieces); a
  window that got split is flagged `VF_BROKEN`. `wind_get(WF_FIRSTXYWH/NEXTXYWH)`
  walks that list via `w_owns`, clipping each piece to the work area and
  terminating with a `w=h=0` rect. **Contract** = the *sequence of visible,
  non-overlapping, work-clipped pieces* an app redraws; the `ORECT` free-list
  and four-way split are **reference**.
- **Contract vs. reference.** The *observable* facts — the `WF_*` field values,
  the top-window handle (`WF_TOP`), the `WM_*` messages, the
  `WF_FIRSTXYWH`/`NEXTXYWH` rect sequence, the `graf_handle` VDI handle + cell
  sizes — are contract; the `D.w_win[]`/`W_TREE`/`W_ACTIVE`/`ORECT` storage, the
  `w_bldactive` frame tree, the blit-move optimisation (`w_move`/`bb_screen`),
  and the `wind_spb`/`ml_ocnt` cooperative locks are EmuTOS internals NewTOS
  reimplements over its own object manager, framebuffer, and process locks.
- **Single-app note.** At NewTOS single-app bring-up there is one AES process
  (the desktop); `gl_wtop`, the window list, and the locks still apply, but the
  cross-process topping/`WM_UNTOPPED` machinery is inert until multi-app exists.

---

## 6c. Resource / scrap / shell state — the loaded RSC, the clipboard path, the shell buffer

The resource, scrap, and shell managers
([`resource-fileselector.md`](./resource-fileselector.md),
[`scrap-shell.md`](./scrap-shell.md)) keep the loaded-resource pointers, the
clipboard directory, and the process-launch context. The **resource** pointers
are *per-application* (in the caller's `AESGLOBAL`); the **scrap** + **shell**
globals are AES-wide (in `THEGLO D`). These are **reference** internals; what a
program observes is the `rsrc_gaddr` addresses, the `scrp_read` path, the
`shel_read`/`shel_get` values, and the `shel_find` result.

| Global | Decl. | Meaning | Read/written by |
|--------|-------|---------|-----------------|
| `ap_rscmem` | `gemrslib.h:22` (`AESGLOBAL`) | base of the loaded `.RSC` blob (`RSHDR *`), or NULL | `rs_load`/`rs_free` (`gemrslib.c:787,699`); `get_addr` via `rs_hdr` |
| `ap_rsclen` | `gemrslib.h:23` (`AESGLOBAL`) | length of the loaded resource in bytes | `rs_load` (`gemrslib.c:788`) |
| `ap_ptree` | `gemrslib.h:20` (`AESGLOBAL`) | the **tree table** (`OBJECT **`) — relocated `rsh_trindex` | `fix_trindex` (`gemrslib.c:597`); `rsrc_gaddr(R_TREE)`, the whole AES |
| `rs_hdr`/`rs_global` | `gemrslib.c:44-45` | file-local statics = the current resource's `RSHDR`/`AESGLOBAL` during a `rs_*` call | `rs_sglobe` sets them from `AESGLOBAL` (`gemrslib.c:675-679`) |
| `D.g_scrap` | `gemlib.h:120` (`THEGLO`, `LEN_ZPATH=114`) | the **scrap (clipboard) directory** path string | `sc_write`/`sc_read`/`sc_clear` (`gemsclib.c:42,59,84`) |
| `D.s_cmd` | `gemlib.h:122` (`THEGLO`, `MAXPATHLEN`) | fully-qualified name of the **next program to run** | `sh_write`/`sh_read`/`sh_ldapp` (`gemshlib.c:104,163,594`) |
| `ad_stail` | `gemshlib.h:13` (`char *`) | the **command tail** (`CMDTAILSIZE=128` bytes) for the next/this app | `sh_write`/`sh_read` (`gemshlib.c:105,170`) |
| `D.g_shelbuf` | `gemlib.h:129` (`THEGLO`, `SIZE_SHELBUF=4192`) | the **shell buffer** — desktop context carried across launches | `sh_get`/`sh_put` (`gemshlib.c:205,215`) |
| `ad_envrn` | `geminit.h:15` (`char *`) | the DOS **environment** block (double-NUL-terminated `NAME=VALUE` run) | `sh_envrn`, `sh_find`'s `PATH=` step, `dos_exec` (`gemshlib.c:338,448,645`) |
| `SHELL sh` | `gemshlib.c:85` | file-local shell state: `sh_doexec`/`sh_nextapp`/`sh_isgem`/`sh_desk`/`sh_cdir` | `sh_write`/`sh_rdef`/`sh_wdef`/`sh_main`/`sh_ldapp` |
| `gl_changerez`/`gl_nextrez` | `gemshlib.c:98-99` | pending resolution change (from `shel_write(SHW_RESCHNG)`) | `sh_write`/`sh_main` |
| `D.g_dta` | `gemlib.h:124` (`THEGLO`) | the AES's own DTA, swapped in for directory scans | `sc_clear`/`sh_find`/`fs_active` (save+restore the caller's DTA) |

- **Resource pointers are per-app (contract-adjacent).** `ap_rscmem`/`ap_rsclen`/
  `ap_ptree` live in the caller's `AESGLOBAL` (filled by `appl_init`, populated
  by `rsrc_load`); one resource per application. The *addresses* `rsrc_gaddr`
  returns are contract; the blob layout/statics are reference.
- **Scrap = one path string (contract vs. reference).** `D.g_scrap` is a bare
  directory path; the file-based clipboard convention (`SCRAP.*`) is the
  contract, the storage is reference. `scrp_read`/`scrp_write` never validate.
- **Shell context = launch command + tail + buffer.** `D.s_cmd`/`ad_stail` (128
  bytes) carry the next program + tail; `D.g_shelbuf` (4192 bytes) carries
  desktop context across the app-exit→desktop-relaunch loop. The **128-byte tail
  size and 4192-byte buffer size are contract**; the `SHELL sh` struct, the
  launch loop `sh_main`/`sh_ldapp`, and the alpha↔graphic switching are
  reference (NewTOS rebuilds them on Phase-06 `Pexec`/`Pterm`).

---

## 7. NewTOS mapping (contract vs. reference)

- **Contract to keep:** ap_id = a small process id — **0** for the
  application chain, **1** as the Screen-Manager sender stamp on `msg[1]`
  only, **2+** for desk accessories (corrected 2026-09-20, A62/A56 — see
  [`application.md`](./application.md) §"AES process identity and pid");
  `appl_init` reports `ap_count = 1`; the 8-char space-padded name; the 16-byte
  message; `-1` for not-found.
- **Free to reimplement:** the `AESPD`/`EVB`/`QPB` structs, the run-list
  scheduler (`dsptch`/`ev_block`/`mwait`), the fixed 128-byte `p_queue`, and the
  `THEGLO` monolith — NewTOS uses its Phase-06 process/IPC model. The AES
  application *is* a NewTOS process; `appl_init`/`appl_exit` bracket its life as
  `Pexec`/`Pterm` do at GEMDOS level.
- **Single-app vs. multi-app:** EmuTOS's concurrent-DA scheduling is reference;
  NewTOS's cooperative single-AES-app (desktop) bring-up satisfies the reported
  `ap_count = 1` with one process and can add DA concurrency later. Open
  question — see [`application.md`](./application.md) NewTOS notes.
