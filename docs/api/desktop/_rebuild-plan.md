# Desktop rebuild — implementation plan (`programs/desktop/src/`)

> Team-lead synthesis (Task #4) from the extracted specs (`_overview.md`,
> `_data-model.md`, `deskapp.md`, `deskwin.md`, `desksupp-redraw.md`,
> `deskmain.md`, `_current-defects.md`) + the dir/file/INF extraction (Task #3,
> in progress). Governs the Developer tasks (#5–#9) and the Auditor gate (#10).
> Charter: **ADR-016** — EmuTOS-faithful reimplementation of the desktop
> *application* on top of the already-faithful AES/VDI/GEMDOS layer in `tos-api`.

> **2026-07-22 revision (ADR-022).** The plan below stands, but its principle is
> **tightened from "behavioural, internals-free" to 1:1 call-sequence
> correspondence + full trap routing** after the drift it allowed (own
> `do_wredraw`, direct-EL1 dialog `objc_draw`, `DESK_PAINTER` hook) let the
> desktop mask app-path bugs that QED exposed. The `docs/api/desktop/` spec was
> re-validated against EmuTOS across 4 chunks (real errors fixed; new
> `gembind.md` call-surface + `deskpro.md`; all docs 1:1-ready). See §1 and §11.

## 1. Principle

Reimplement EmuTOS `desk/` **1:1** in `#![no_std]` Rust: every EmuTOS `desk/`
function → a **same-named Rust function that makes the same `tos-api`
(AES/VDI/GEMDOS) calls in the same order/sequence**, going **through the AES trap**
(opcode 200) like EmuDesk over `aesbind`/`gembind` — **no** in-kernel shortcuts,
**no** hooks, **no** direct-EL1 `objc_draw` for client rendering. This is **not**
a C port (Rust stays Rust; the source-correspondence gives the "debug against the
EmuTOS reference" benefit). Purely-**private** data (`WNODE`/`FNODE`/`PNODE`/`G`/
screen arena — all internal per `_data-model.md`) may stay idiomatic Rust; the
debugging benefit rides on the call-sequence, not the data representation. What
must match exactly: the `OBJECT`/`ICONBLK` trees handed to `objc_draw`, **and the
AES/VDI/GEMDOS call sequence** (both spec-documented). Two EmuTOS single-binary
shortcuts are **not** replicated (they re-bypass the AES): `gsx_trans` → public
VDI `vr_trnfm`; `menu_text` → the real AES `menu_text` opcode.

## 2. Target module layout (`programs/desktop/src/`)

| Module | EmuTOS origin | Spec | Content |
|---|---|---|---|
| `mod.rs` | `deskstart.S` + `deskmain()` head | `deskmain.md` | `desk::run()` boot entry; one-time setup (icons, metrics, window/obj init, menu install, `app_blddesk`); hands off to the event loop |
| `global.rs` | `deskglob.c` + `GLOBES` (`deskbind.h`) | `_data-model.md` | `DeskState` (≙ `G`): view/sort, grid metrics (`g_iwext/ihext/iwint/ihint/iwspc/ihspc`, `g_wicon/hicon`, `g_icw/ich`), `g_desk` rect, window list head, the `g_screen[]`/`g_screeninfo[]` object arena + free chain |
| `model.rs` | `deskfpd.c` (`FNODE`/`PNODE`) + `SCREENINFO` | `_data-model.md` | `FNode`/`PNode`/`ScreenInfo`; directory scan (`pn_open`/`pn_active`/`pn_sort`) via GEMDOS `Fsfirst`/`Fsnext`. **No fixed 64-cap** (fix D9) — dynamic list |
| `app.rs` | `deskapp.c` (rendering parts) | `deskapp.md` | `app_start` metrics (`g_wicon`/`g_hicon`/`g_icw`/`g_ich`), `app_blddesk` (desktop pseudo-window tree), `set_background` |
| `win.rs` | `deskwin.c` | `deskwin.md` | `WNode` + window-list mgmt (`win_start`/`win_alloc`/`win_free`/`win_find`/`win_top`), **`win_view`/`win_ocalc`/`win_bldview`**, scrolling (`win_slide`/`win_arrow`/`win_blt`), bulk (`win_srtall`/`win_bdall`/`win_shwall`) |
| `redraw.rs` | `desksupp.c` `do_wredraw` | `desksupp-redraw.md` | `do_wredraw(handle, clip)` over the AES `WF_FIRSTXYWH`/`WF_NEXTXYWH` rect list |
| `main.rs` | `deskmain.c` event loop | `deskmain.md` | `evnt_multi` loop + message drain subloop; `hndl_button` (via `wind_find` — fix D3), `hndl_kbd` (fix D4), `hndl_menu` dispatch; `desk_all` view/sort pipeline |
| `menu.rs` | `deskmain.c` menu parts | `deskmain.md` | menu-tree build, `adjust_menu` (measure/shrink), **`men_update`** (enable/disable/check — fix D6) |
| `dir.rs` | `deskdir.c` + `deskfun.c` | *(Task #3)* | file ops: copy/move/delete/rename/new-folder dispatch + recursive helpers |
| `act.rs` | `deskact.c` | *(Task #3)* | selection / drag / rubber-band state machine; single geometry source (fix D7) |
| `ins.rs` | `deskins.c` | *(Task #3)* | install icon / install application (ANODE) |
| `inf.rs` | `deskinf.c` | *(Task #3)* | `NEWDESK.INF` parse/save (`#W`/`#M`/`#T`/`#O`/`#E`/`#Q`/…) |
| `rez.rs` | `deskrez.c` | *(Task #3)* | resolution change → recompute metrics |
| `icons.rs` | `icon.rsc` (built-in bitmaps) | — | NewTOS icon bitmaps (port/replace old `desktop_icons.rs`); NewTOS owns its `ICONBLK` params incl. `ib_xtext` (§5) |

## 3. Data model (NewTOS)

Per `_data-model.md`, all of `G`/`WNODE`/`PNODE`/`FNODE`/`SCREENINFO` are
desktop-internal (never cross an ABI boundary) → redesign cleanly:
- Prefer safe Rust ownership over EmuTOS's `malloc`'d free-chains where it costs
  nothing observable. `g_screen[]` stays an `OBJECT` array (it **is** the AES
  tree handed to `objc_draw`) but its indexing (`DROOT`/`WOBS_START`/free chain)
  is our choice.
- Drop the `FNODE`/`DTA` 23-byte `memcpy` overlap trick (`deskfpd.c:271`) — copy
  DTA fields explicitly; our `f_name` need not be 13 bytes.
- Scroll position is a **grid-cell** offset (`w_cvcol`/`w_cvrow`), converted to
  pixels only at draw time via `g_iwspc`/`g_ihspc` — **one** coherent unit for
  both views (fixes D5, the icon/text scroll-unit mismatch).

## 4. AES/VDI/GEMDOS call surface (the NewTOS "aesbind")

The desktop is a client of `tos-api`; it must go **through** these calls (like
EmuTOS `desk/` over `aesbind`), not reach into kernel internals (ADR-016
review charge). Confirmed available in `tos-api` (per `_current-defects.md` D2
and the AES specs):
- **Window mgr:** `wind_create/open/close/set/get/find/calc`; crucially
  `wind_get(WF_FIRSTXYWH/WF_NEXTXYWH)` — the occlusion-aware rect list already
  implemented in `tos-api/src/aes/window.rs:264-310` (unused today).
- **Objects:** `objc_draw`/`objc_find`/`objc_add`/`objc_offset`.
- **Menu:** `menu_bar`/`menu_ienable`/`menu_icheck`/`menu_tnormal`.
- **Events:** `evnt_multi` (`MU_BUTTON|MU_MESAG|MU_KEYBD|MU_TIMER`).
- **Graphics:** `graf_mouse` (M_OFF/M_ON, HGLASS — enables D11 busy cursor).
- **GEMDOS:** `Fsfirst`/`Fsnext` (`tos_api::bdos::fs::find`), file ops for `dir.rs`.

Developer: confirm exact Rust names/signatures against `tos-api/src/aes/*` before
each task; if a needed AES call is missing or wrong → **STOP**, report to lead
(do not reach around it into the kernel).

## 5. Grid-metric contract (root of the overlap bug — D1)

From `deskapp.md` (`app_start`, `deskapp.c:810-817`), the exact formulas NewTOS
must reproduce:

```
icon cell width   g_wicon = MAX_ICONTEXT_WIDTH(12) * char_w + 2*ib_xtext
icon cell height  g_hicon = ib_hicon(icon#0) + char_h + 2
window pitch      g_iwspc = g_wicon + MIN_WINT      (MIN_WINT = 4px if rows<=300 else 8px)
                  g_ihspc = g_hicon + MIN_HINT      (MIN_HINT = 2px)
desktop pitch     xcnt = g_desk.w / (g_wicon+MIN_WINT);  g_icw = g_desk.w / xcnt   (remainder spread evenly)
                  ycnt = g_desk.h / (g_hicon+MIN_HINT);  g_ich = g_desk.h / ycnt
```

Key correctness points (fixing D1):
- The label box is a **fixed** `MAX_ICONTEXT_WIDTH`-char width — **never**
  shrink-to-fit. The cell = label width **+ `2*ib_xtext`** (icon-0's centering
  offset), not label width alone.
- `char_w`/`char_h`: NewTOS uses its **fixed 8×16 font** (Phase-12 decision), so
  `char_w = 8` is our metric; that is legitimate as long as it is used
  consistently everywhere (renderer *and* metric) — Developer must confirm the
  VDI glyph renderer's real per-glyph width equals the metric before wiring D1.
- **`ib_xtext` is a NewTOS design choice** (EmuTOS's value lives in a build-time
  `icon.rsc` not in the checkout — R1 STOP-flag; not a blocker). Pick a small
  fixed centering offset for the NewTOS icon `ICONBLK` in `icons.rs` and use the
  **formula** everywhere. The formula is the contract; the value is ours.
- `draw_text` must clip glyphs to the object text box (the `tos-api` AES fix from
  the superseded mossy plan Phase 0) — verify it is in place / apply once.

## 6. Redraw contract (D2/D8)

From `desksupp-redraw.md` (`do_wredraw`, `desksupp.c:202-232`):
```
root = (handle==DESK) ? DROOT : win_find(handle).w_root   // else silent return
graf_mouse(M_OFF)
t = wind_get(handle, WF_FIRSTXYWH)
while t.w && t.h {                        // sentinel: w==0||h==0 ends
    if rc_intersect(clip, t) { objc_draw(g_screen, root, MAX_DEPTH, t) }
    t = wind_get(handle, WF_NEXTXYWH)     // always advance, even after a hit
}
graf_mouse(M_ON)
```
Every repaint (WM_REDRAW with **its own `msg[4..8]` rect** — fix D8; scroll blt;
selection change; view/sort rebuild) routes through this. Draw order becomes
irrelevant to correctness. `win_shwall` sends one WM_REDRAW **per window** with
that window's rect.

## 7. Task sequencing

| Task | Deliverable | Depends | Verify |
|---|---|---|---|
| **#5 Scaffold** | `mod.rs`/`global.rs`/`model.rs`/`win.rs` skeleton + `DeskState` + `FNode`/`WNode`/`PNode` + directory scan; wire `desk::run()` at the boot call site (replace `desktop::run()`); deprecate `desktop.rs`/`desktop_icons.rs` (guarded, kept until parity) | #4 | build+clippy both boards; **still boots** to a (possibly empty) desktop screen |
| **#6 View builder** | `app_start` metrics + `win_view`/`win_ocalc`/`win_bldview`; `find_entry_at` reuses the builder | #5 | QMP screendump on qemu raspi4b, **representative 12-char 8.3 SD image**: 4 clean icon columns, no overlap |
| **#7 Redraw** | `do_wredraw` over the rect list; route open/close/move/size/top/view/sort | #6 | screendump: overlap-on-top + view-switch residue gone (Auditor drives the residue repro) |
| **#8 Event/menu** | `evnt_multi` loop + `hndl_button`(`wind_find`) + `hndl_kbd` + `hndl_menu`; `men_update`; `adjust_menu` | #5 | keyboard + click-any-window work; menu items gray correctly |
| **#9 Dir/file/INF/rez** | `dir.rs`/`act.rs`/`ins.rs`/`inf.rs`/`rez.rs` | #5, #3 | file ops + INF round-trip; resolution change |
| **#10 Audit (gate)** | per-deliverable adversarial verification | each of #5–#9 | representative data; CONFIRMED/PLAUSIBLE findings; nothing "done" without sign-off |

## 8. Open scope decision — `CONF_WITH_*` feature gates

`deskmain.md` catalogued ~12 gates with no recorded NewTOS decision. They affect
which menu items/behaviours `menu.rs`/`dir.rs` include (they do **not** affect
#5–#7 core). **Recommended default:** the EmuTOS general-Atari variant (all
`CONF_WITH_*` **on**), *except* hardware-absent/legacy ones:
- **In:** SelectAll, FileMask (Set mask), Search, SizeToFit (full), Read/Save
  NEWDESK.INF preferences, PrinterIcon (we already draw one), DesktopShortcuts,
  BottomToTop (window cycling), PCGEM defaults.
- **Out:** Format (no floppy/removable-format on Pi4 SD — consistent with the
  FAT12/floppy out-of-scope decision), AltDesktopGraphics (cosmetic), EasterEgg.

→ **DECIDED 2026-07-04 (user):** the **full EmuTOS-default set** above — In as
listed (SelectAll, Set-mask/FileMask, Search, Size-to-fit, Read+Save NEWDESK.INF
preferences, Printer icon, Desktop shortcuts, BottomToTop, PCGEM); **Out:**
Format, Alt-Desktop-Graphics, Easter-Egg. `menu.rs`/`dir.rs` build against this
set.

## 9. Carried-forward risks / STOP-flags

- **View-switch residue** (R2 STOP-flag): unconfirmed mechanism; best hypothesis
  = instance of D2, possibly compounded by D5. The rebuild fixes both
  structurally; **Auditor must drive the specific repro** (single window, two
  overlapping, 3-window overlap, plus a menu/dialog overlap) to confirm.
- **`format_fnode`/`dr_code`** (text-view line format) — **resolved** by Task #3
  in `deskinf.md`. **`snap_icon`** — **resolved** by Task #19 in `deskins.md`
  (section "`snap_icon`/`ins_posdisk`/`grid_free` — desktop grid placement",
  cross-linked from `deskapp.md` and `_data-model.md`). Correction to the
  framing above: `snap_icon` itself is **not** collision avoidance — it only
  rounds a pixel position to the nearest desktop grid cell and clamps it to
  the desktop bounds. Collision avoidance against already-placed icons is a
  *separate* pair of helpers, `grid_free`/`ins_posdisk`, used only by the
  three new-icon-install call sites (`install_drive`, `install_desktop_icon`,
  and `ins_shortcut`'s 2nd+ icon in a multi-drop) — `ins_shortcut`'s *first*
  icon, `app_start()`'s INF-position-apply loop, and `move_drvicon()`'s
  drag-drop path all call `snap_icon` alone, with **no** collision check, so
  EmuTOS itself can (and does, given colliding input) silently overlap desktop
  icons. `ins.rs` (this plan's Task #9 "Dir/file/INF/rez"; tracker #20
  "Install-Icon-Platzierung") must decide explicitly whether NewTOS
  reproduces this gap or adds a stronger free-cell guarantee everywhere (a
  deliberate improvement, not contract — see `docs/COMPATIBILITY.md`).
  NEWDESK.INF
  grammar lives in `deskinf.md`'s appendix (sourced from `deskapp.c`), **not**
  `deskinf.c` — `inf.rs` builds against that.
- **AES gaps:** if #6/#7 hit a missing/incorrect `wind_*`/`objc_*` in `tos-api`,
  that is a `tos-api` fix (separate Developer task), **not** a desk-side
  workaround.

## 10. Verification protocol

Every Developer deliverable: `cargo build` + `cargo clippy -D warnings` clean for
**both** boards, system still boots, and — for anything visual — a QMP
screendump on **`qemu raspi4b` with a realistic 8.3-long-name SD image** (the
blind spot that hid the original bug). Then the **Auditor** gate (#10) with
adversarial/representative data. Nothing is "done" before Auditor sign-off.

## 11. 2026-07-22 validation outcome + 1:1 sequencing (ADR-022)

**Spec validation (4 chunks vs EmuTOS `desk/`, all `file:line`-cited).** All docs
are now **1:1-ready**. Real errors fixed: `deskins` (3D-objects default **ON**),
`deskfun` (multi-launch was on the wrong function — `fun_desk2desk`, not
`fun_desk2win`), `deskapp` (resolution change = **desktop reboot**, not in-place;
full `app_start`/`app_blddesk` call-sequences added), `_overview`/`_data-model`
(deskrez interaction). New docs: **`gembind.md`** (the AES/GEMDOS call-surface)
and **`deskpro.md`** (`pro_run`/`pro_exec` launch). `deskrez`/`desksupp-redraw`
were already exact (the load-bearing `do_wredraw` doc verified byte-for-byte).

**AES call-surface (gembind.md).** **No AES-server-side gap** — every opcode the
desktop calls already exists in `tos-api`. The rebuild adds the missing
**client-side trap wrappers** our kernel/src/desk/mod.rs lacks:
 `menu_icheck`/`menu_ienable`,
`objc_add`/`objc_offset`/`objc_order`, `form_dial`/`form_center`/`form_error`,
`graf_growbox`/`graf_shrinkbox`/`graf_handle`, `fsel_exinput`, `rsrc_*`, `shel_*`,
`appl_init`/`appl_exit`. Two non-trap EmuTOS shortcuts → public calls:
`gsx_trans` → **`vr_trnfm`** (VDI), `menu_text` → the real AES **`menu_text`** op.

**Concrete constraints surfaced.** Resolution change ⇒ reboot the desktop
(`shel_write(SHW_RESCHNG)`+`done=TRUE`); `ib_xtext=0` ⇒ `g_wicon = 12·char_w`
(erd-verified); deep copy needs **≥12 concurrent DTA slots** (`d_doop`
private-DTA-per-level) plus a 3rd DTA swap in `item_exists`; the redraw path
**skips `handle==DESKWH`** (background is the AES `WF_NEWDESK` job); `hndl_button`
does `wind_find` **first** (click into any window — the D3 mechanism our desk/
lacks).

**Gap to close before the launch module:** extract `desksupp.c`
(`do_aopen`/`do_open`/`i_find`/`build_root_path`/`is_installed`) into a new
`desksupp.md` — the function every launch path calls first is still undocumented.

**Revised 1:1 module order (redraw-path first), Developer → independent Tester →
lead-commit per module:**
1. **Redraw path** — `do_wredraw` over the AES `WF_FIRSTXYWH/WF_NEXTXYWH` rect
   list + route WM_REDRAW/TOPPED/SIZED/MOVED through it; **delete** the drifted
   own-redraw. (The app-path heart; where QED breaks.)
2. **Dialogs onto the app path** — no direct-EL1 `objc_draw`; the `gembind`
   wrapper layer (`form_*`, `fsel_*`, `rsrc_*`).
3. **Menu/window/event 1:1** — `hndl_button` (`wind_find`-first), `hndl_kbd`,
   `hndl_menu`, `men_update`, the `evnt_multi`+`wind_update(BEG/END)` loop.
4. **File/dir/INF/rez 1:1** — `deskdir`/`deskfun`/`deskact`/`deskinf`/`deskins`/
   `deskrez` + the `desksupp.md` launch layer (after the gap above is filled).
5. **HW retest** after step 1 (the QED/desktop menu redraw) and again at parity.

## 12. Receipt guards — mandatory in every DESK43 unit receipt (U5a, 2026-10-14)

Source: guards G2–G5 of `docs/analysis/desk43-u5-u10-review-2026-10-14.md` §4,
built in DESK43 U5a (`TASK/DESK43-U5a-STATUS-DONE.MD`). They extend plan v2 §5
("every receipt carries (a) the drift-probe number, (b) the removed bypasses
with their replacement, (c) every touched `fn` without an EmuTOS citation").
A unit receipt that lacks one of the four numbers below is not accepted.

| Guard | Command | What it says | Rule |
|---|---|---|---|
| G5 drift probe v2 | `tools/desk-drift-probe.sh` (`-v` for the lines) | `DRIFT-PROBE TOTAL` = code lines in which the desktop names `tos_api::`/kernel internals, the kernel names the desktop, or the desktop crate depends on more than `newtos-gem`/`tos-types`; `DRIFT-PROBE UNSAFE` = code lines with `unsafe` in the desktop | TOTAL never rises; UNSAFE never rises (the baseline is `UNSAFE_BASELINE` in the script — a unit that lowers the count lowers the baseline). The probe fails loudly (exit 1) when it cannot find the desktop source, instead of reporting an empty 0; a unit that moves the desktop (U8b) checks that it now reports the crate |
| G3 constants | `cargo xtask emutos-consts` (`-v` for the unchecked names) | every literal integer `const` of the desktop vs EmuTOS' `#define` of the same name, from the checked-in snapshot `tools/xtask/emutos-defines.snapshot` | every mismatch is listed in `tools/xtask/emutos-consts-deviations.txt` as `intended` (with the decision) or `defect`; the host test `emutos_consts::tests::desk_consts_match_emutos_snapshot` fails on an undocumented mismatch **and** on a stale entry. Regenerate the snapshot only with `--update` from a local `references/emutos/`, and name the EmuTOS commit in the receipt |
| G4 citations | `tools/desk-cite-list.sh --since <base commit>` | every `fn` touched since the unit's base without an EmuTOS `file.c:N`/`file.h:N`/`file.S:N` in the comment block above it; plus the global count (`DESK-CITE: fns= cited= newtos= uncited=`) | the receipt lists each touched uncited fn (with one line why it has no citation) and each `// NEWTOS:` fn it added. A fn with deliberately no EmuTOS counterpart carries `// NEWTOS: <why>` in its comment block — greppable, and counted apart from "uncited". The marker is a claim; review checks it |
| G2 call-sequence oracle | kernel feature `svc-seq-trace` (`cargo xtask build --board rpi4 --release --features svc-seq-trace`), `C:\SVCSEQ.LOG`, `tools/svc-seq.py` | every `svc` in completion order with its caller (`K` = EL1 desktop, `U<slot>` = EL0 process) | mandatory for units that change what the desktop calls or how it is started (U9 proof point 4: kernel-linked vs `.NAP` desktop, same injector, `diff` of `svc-seq.py --caller … --no-caller` output); optional otherwise. Off by default — the feature never goes into a product build |

Receipt block (copy into the unit's `TASK/DESK43-U*-STATUS-DONE.MD`):

```
Guards (docs/api/desktop/_rebuild-plan.md §12):
  DRIFT-PROBE TOTAL: <n> (before: <m>)   DRIFT-PROBE UNSAFE: <n> (baseline <b>)
  EMUTOS-CONSTS: total=… matched=… mismatched=… documented=… undocumented=0 stale=0 no-counterpart=…
  DESK-CITE: fns=… cited=… newtos=… uncited=…            (whole desktop)
  DESK-CITE --since <base>: <touched uncited / NEWTOS fns, one line each, or "none">
  Oracle: <run + diff result, or "not required: <why>">
```
