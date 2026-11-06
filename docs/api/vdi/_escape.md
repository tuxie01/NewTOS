# VDI `v_escape` (opcode 5) — alpha/cursor sub-functions (Phase 00b D6)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_esc.c` (`vdi_v_escape`,
> `esctbl[]`, `escfn0..escfn19`, `esc_init`/`esc_exit`).
> Calling convention: [`_dispatcher.md`](_dispatcher.md). Line-A cell/
> cursor globals: `include/lineavars.h:51-52` (`v_cel_mx`/`v_cel_my`),
> `bios/lineavars.S:177-185` (`_v_cel_mx`/`_v_cel_my`/`_v_cur_cx`/
> `_v_cur_cy`).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** opcode 5, the `CONTRL[5]` sub-function numbers, which
> `INTIN`/`PTSIN` slots each reads, and what `INTOUT`/`CONTRL[4]` each
> returns. **Reference:** that EmuTOS implements the cursor moves by
> writing VT-52 escape sequences to `Bconout(2,…)` and the inquiries by
> reading Line-A cell/cursor globals — NewTOS reimplements those against
> its own console/framebuffer.

`v_escape` is VDI opcode **5**; the actual sub-function is selected by
`CONTRL[5]` (`vdi_esc.c:336`). These are the VT-52-style **alpha
(text-mode) cursor** controls plus a few inquiries. They operate on the
text/alpha console layer, not on the graphics-attribute state.

---

## Trap interface

| | |
|--|--|
| Opcode | `CONTRL[0] = 5` |
| Sub-function | `CONTRL[5]` = escape id (0..19; also 99 when `HAVE_BEZIER`) |
| Handle | `CONTRL[6]` = workstation handle (resolved by the dispatcher) |
| Inputs | `INTIN[]` (per sub-function); `PTSIN[]` (sub 18) |
| Outputs | `INTOUT[]` + `CONTRL[4]` set by the inquiry sub-functions |

Dispatch (`vdi_v_escape`, `vdi_esc.c:334-350`), in prose:

1. `escfun = CONTRL[5]`.
2. `#if HAVE_BEZIER` **only**: `escfun == 99` calls `v_bez_qual()` (set
   Bezier curve quality) and returns. `HAVE_BEZIER` is **not** a
   `CONF_WITH_*`/target-variant switch — it is a single hardcoded
   `#define HAVE_BEZIER 0` (`vdi_defs.h:18`, comment: "entirely
   untested"), unconditional across **every** EmuTOS build/target in this
   source tree (no `config.h`/Makefile override exists — confirmed by
   exhaustive grep for `HAVE_BEZIER` across the tree). So sub-function 99
   is dead code / a permanent no-op in EmuTOS as shipped, not a
   variant-dependent behaviour.
3. If `escfun > 19` (`ldri_escape`, `vdi_esc.c:23`) → **return, no-op**.
4. Otherwise call `esctbl[escfun](vwk)` (`vdi_esc.c:293-315,349`).

The dispatcher's `jmptb1` entry for opcode 5 declares `{nptsout,nintout} =
{0,0}` (`vdi_main.c:37`); `screen()` copies these into `CONTRL[2]`/`CONTRL[4]`
**before** calling `vdi_v_escape` (`contrl[2] = jmptab->nptsout; contrl[4] =
jmptab->nintout;`, `vdi_main.c:158-159`). The inquiry sub-functions (1, 15,
16) then overwrite `CONTRL[4]` themselves (`2`, `2`, `1` respectively) —
**no** escape sub-function ever sets `CONTRL[2]`, so **`PTSOUT` is never
populated by any `v_escape` sub-function**; `CONTRL[2]` stays `0` for the
whole opcode, always.

---

## `CONF_WITH_*` gating

`vdi/vdi_esc.c` itself has **no `CONF_WITH_*` conditional compilation at
all** — confirmed by reading the whole file (369 lines): the only `#if` is
the hardcoded, non-target-dependent `HAVE_BEZIER` guard discussed above.
Every one of the 20 sub-functions (0–19) is present, unconditionally, in
every EmuTOS build/target.

The underlying VT-52 state machine (`bios/vt52.c`), which `escfn2..14`
drive via `Bconout(2,…)`, **does** have gating, but it is orthogonal to the
`v_escape` contract: `CONF_SERIAL_CONSOLE`/`CONF_SERIAL_CONSOLE_ANSI`
(`include/config.h:1423-1440`, both default **0**) additionally *mirror*
console output to a physical serial line (as raw VT-52 bytes or as
translated ANSI, respectively) — a debug/dual-head feature. It does not
change what a `v_escape` caller observes on the primary display or in
`INTOUT`/`CONTRL[4]`; not relevant to the NewTOS bring-up profile (no
serial-mirrored console).

---

## Sub-function table (`CONTRL[5]`, `esctbl[]`)

| `CONTRL[5]` | Fn | Action | Source |
|:-----------:|----|--------|--------|
| 0  | `escfn0`         | no-op padding entry (simplifies lookup) | `:49` |
| 1  | `vq_chcells`     | inquiry: rows/columns — see below | `:62` |
| 2  | `v_exit_cur`     | exit alpha mode → enter **graphics** mode | `:81` |
| 3  | `v_enter_cur`    | enter alpha mode → exit graphics mode | `:93` |
| 4  | `v_curup`        | alpha cursor up one line (`ESC A`) | `:102` |
| 5  | `v_curdown`      | alpha cursor down one line (`ESC B`) | `:111` |
| 6  | `v_curright`     | alpha cursor right one column (`ESC C`) | `:120` |
| 7  | `v_curleft`      | alpha cursor left one column (`ESC D`) | `:129` |
| 8  | `v_curhome`      | alpha cursor home (`ESC H`) | `:138` |
| 9  | `v_eeos`         | erase to end of screen (`ESC J`) | `:147` |
| 10 | `v_eeol`         | erase to end of line (`ESC K`) | `:156` |
| 11 | `vs_curaddress`  | set cursor row/col — see below | `:172` |
| 12 | `v_curtext`      | output cursor-addressable text — see below | `:196` |
| 13 | `v_rvon`         | reverse video on (`ESC p`) | `:213` |
| 14 | `v_rvoff`        | reverse video off (`ESC q`) | `:222` |
| 15 | `vq_curaddress`  | inquiry: current row/column — see below | `:231` |
| 16 | `vq_tabstatus`   | inquiry: tablet availability — see below | `:248` |
| 17 | `v_hardcopy`     | screen dump via XBIOS `Scrdmp()` | `:260` |
| 18 | `v_dspcur`       | display graphics cursor — see below | `:274` |
| 19 | `v_rmcur`        | remove graphics cursor (`vdi_v_hide_c`) | `:284` |

`esc_init` (`vdi_esc.c:356`) is called from `v_opnwk` and runs `escfn2`
(sub 2, enter graphics mode); `esc_exit` (`:365`) is called from
`v_clswk` and runs `escfn3` (sub 3, enter alpha mode). See
[`workstation/01-v_opnwk.md`](workstation/01-v_opnwk.md).

**Contract point — physical only:** the two call sites are exhaustive
(confirmed by grepping the whole `vdi/` tree for `esc_init`/`esc_exit`):
`vdi_v_opnwk`/`vdi_v_clswk` (`vdi_control.c:564,589` — opcodes **1**/**2**,
the *physical* workstation). Opening/closing a **virtual** workstation
(`v_opnvwk`/`v_clsvwk`, opcodes **100**/**101**, `vdi_control.c`) does
**not** call `esc_init`/`esc_exit` and therefore does **not** toggle the
alpha/graphics mode — only `v_opnwk`/`v_clswk` do. See
[`workstation/101-v_clsvwk.md`](workstation/101-v_clsvwk.md).

---

## Sub-functions with inputs / outputs

### 1 — `vq_chcells` (`escfn1`, `vdi_esc.c:62-67`)

- **Output:** `CONTRL[4] = 2`; `INTOUT[0] = v_cel_my + 1` (**number of
  rows**), `INTOUT[1] = v_cel_mx + 1` (**number of columns**).
- `v_cel_my`/`v_cel_mx` are Line-A "rows−1 / columns−1" of the text cell
  grid (`lineavars.h:51-52`, `bios/lineavars.S:177-178`).

**Geometry: live-derived, not a fixed constant.** `v_cel_mx`/`v_cel_my` are
computed by `font_set_default()` (`bios/font.c:72-88`):
```c
font = (V_REZ_VT < 400) ? &fon8x8 : &fon8x16;
v_cel_mx = (V_REZ_HZ / font->max_cell_width) - 1;
v_cel_my = (V_REZ_VT / font->form_height) - 1;
```
i.e. screen-resolution ÷ font-cell-size, exactly the same "computed from
`V_REZ_HZ/VT`" pattern as the resolution-dependent `DEV_TAB` entries (see
`D-VDI-7`). `font_set_default()` is called from `vt52_init()`
(`bios/vt52.c:921-924`), which itself runs once at BIOS console bring-up
(`bios/bios.c:340-341`) **and again every time the resolution changes**:
`esetshift()` (TT-shifter mode change, call at `bios/screen.c:201`),
`set_rez_hacked()` (call at `bios/screen.c:673`), and the generic
`Setscreen`/videl mode-change path (call at `bios/screen.c:1161`) all
re-invoke `vt52_init()` → `font_set_default()`. So `vq_chcells`'s answer
tracks the *current* device geometry across resolution changes within a
session, not a boot-time-only snapshot — the same "live-derived, device
dependent" characterisation the spec already uses for `DEV_TAB`.
**Reference:** NewTOS derives these from its own framebuffer font-cell
geometry (`yres/cell_h`, `xres/cell_w`) and must likewise recompute them if
NewTOS ever supports a resolution change at runtime (out of scope for the
single-fixed-640×480 bring-up profile, see `D-076`/`D-077`/`D-078`).

### 2 — `v_exit_cur` (`escfn2`, `vdi_esc.c:81-84`)

- **No input, no output.** Writes the VT-52 sequence `ESC H ESC f ESC E`
  (home, hide alpha cursor, clear-and-home) to console output via
  `Bconout(2,…)`. Switches from alpha/text to **graphics** mode. Called by
  `esc_init` at `v_opnwk`.

### 3 — `v_enter_cur` (`escfn3`, `vdi_esc.c:93-96`)

- **No input, no output.** Writes `ESC E ESC e CR` (clear-and-home, show
  alpha cursor, carriage return) — the CR resets the BDOS column counter
  for tab expansion. Switches back to alpha/text mode. Called by
  `esc_exit` at `v_clswk`.

### 4–10, 13, 14 — cursor moves + erase + reverse video

- **No input, no output.** Each writes one VT-52 escape to `Bconout(2,…)`:
  `ESC A/B/C/D/H` (up/down/right/left/home), `ESC J/K` (erase to end of
  screen/line), `ESC p/q` (reverse video on/off). **Reference:** NewTOS
  maps these onto the existing console cursor/erase routines.

### 11 — `vs_curaddress` (`escfn11`, `vdi_esc.c:172-183`)

- **Input:** `INTIN[0]` = cursor **row** (1-based, contract range `1 ..
  v_cel_my+1` per the source comment `vdi_esc.c:169`), `INTIN[1]` =
  cursor **column** (1-based, contract range `1 .. v_cel_mx+1`,
  `vdi_esc.c:170`).
- **No output.** Emits VT-52 `ESC Y <0x20+row-1> <0x20+col-1>`; the
  handler converts the 1-based row/col to the 0-based VT-52 offsets. The
  cursor is shown at the new location unless currently hidden.
- **In-contract-range behaviour:** the VT-52 parser (`bios/vt52.c:242-286`,
  `esc_ch1`/`get_row`/`get_column`) decodes the two bytes back to 0-based
  row/col and calls `move_cursor(col,row)`, which **clamps** to
  `[0,v_cel_mx]`×`[0,v_cel_my]` (`bios/conout.c:416-429`). For a
  well-formed 1-based `INTIN` in the documented range this clamp is a
  no-op.
- **Out-of-contract-range input is not safely clamped by `escfn11`
  itself:** `out[2] = 0x20 + INTIN[0] - 1` truncates to a single `char`
  *before* the VT-52 parser (and hence `move_cursor`'s clamp) ever runs.
  A large `INTIN[0]`/`[1]` (outside roughly `1..223`) wraps modulo 256 in
  that assignment, so the row/col value the VT-52 layer actually decodes
  is an arithmetic artifact of the byte truncation, not a defined
  saturation to `v_cel_my+1`/`v_cel_mx+1`. Contract: callers must supply
  `INTIN` in the documented 1-based range; behaviour for out-of-range
  input is unspecified (matches the general Atari-TOS convention of
  trusting VDI input — `escfn11` performs no bounds check of its own on
  `INTIN[0]`/`[1]` before the byte truncation).

### 12 — `v_curtext` (`escfn12`, `vdi_esc.c:196-207`)

- **Input:** `CONTRL[3]` = character count; `INTIN[]` = the character
  array (one char per WORD).
- **No output.** Writes each character via `Crawio()` (GEMDOS raw
  console I/O). Text appears at the current cursor position.
- **`INTIN[i] == 0x00FF` cannot be written — genuine EmuTOS source-level
  gap.** `escfn12`'s inner loop is `crawio(*chr++)` → `Crawio(ch)` →
  GEMDOS trap `0x06` = `xrawio` (`bdos/console.c:441-453`):
  ```c
  long xrawio(int parm)
  {
      if (parm == 0xFF)
      {
          i = HXFORM(run->p_uft[0]);
          return constat(i) ? getch(i) : 0L;   /* READ, not write! */
      }
      Bconout(HXFORM(run->p_uft[1]), parm);
      return 0;
  }
  ```
  `xrawio` uses `parm == 0xFF` as a **read-vs-write sentinel** (documented
  in [`../gemdos/console/06-crawio.md`](../gemdos/console/06-crawio.md)).
  A `v_curtext` character array entry of exactly `0x00FF` (character code
  255 — a defined, renderable glyph: the system font's `first_ade=0,
  last_ade=255`, `bios/fnt_st_8x8.c:150-151`) therefore does **not** get
  written to the console at all; instead `xrawio` probes/consumes stdin
  (a pending keystroke, if any) and the intended output byte is silently
  dropped. Logged as `D-VDI-22`/global `D-083`.

### 15 — `vq_curaddress` (`escfn15`, `vdi_esc.c:231-236`)

- **Output:** `CONTRL[4] = 2`; `INTOUT[0] = v_cur_cy + 1` (**current
  row**, 1-based), `INTOUT[1] = v_cur_cx + 1` (**current column**,
  1-based). `v_cur_cx`/`v_cur_cy` are Line-A current cursor col/row
  (`lineavars.S:184-185`).

### 16 — `vq_tabstatus` (`escfn16`, `vdi_esc.c:248-252`)

- **Output:** `CONTRL[4] = 1`; **`INTOUT[0] = 1`** (a "tablet device" /
  mouse is available). Atari TOS always returns 1, and EmuTOS matches it.
- **Correction:** an earlier revision of this doc claimed `INTOUT[0] = 0`.
  The source unconditionally returns **1** (`vdi_esc.c:251`). Logged in
  [`_discrepancies.md`](_discrepancies.md) D-VDI-2.

### 17 — `v_hardcopy` (`escfn17`, `vdi_esc.c:260-263`)

- **No input, no output.** Calls the XBIOS **`Scrdmp()`** (screen dump to
  printer). In EmuTOS this is a *real* screen dump, **not** a no-op.
- **NewTOS:** the bring-up profile has no printer device, so `Scrdmp()`
  is itself out of bring-up scope; `v_hardcopy` must therefore be a
  **documented no-op** (return cleanly), recorded under `docs/STATUS.md`
  "Blocked" if a program actually needs it — an honest deferral, not a
  faked success. (This is a NewTOS scope decision, not EmuTOS behaviour.)
  Logged in [`_discrepancies.md`](_discrepancies.md) D-VDI-4.

### 18 — `v_dspcur` (`escfn18`, `vdi_esc.c:274-278`)

- **Documented input** (per Atari docs): `PTSIN[0..1]` = the `x,y` where
  the graphics cursor should appear. **EmuTOS, like Atari TOS, ignores
  these** — it forces `INTIN[0] = 0` ("show regardless") and calls
  `vdi_v_show_c()` (VDI opcode 122). So the passed coordinates have no
  effect. **No `INTOUT`/`PTSOUT`** — but the `INTIN[0] = 0` store is
  **written into the caller's own `INTIN` array** and is therefore an
  observable output of the call, the only VDI handler that modifies an
  input array. Measured: `tosdump/reference/EMUTOS14-VDI-all.BIN` case 187
  records `intin[0] = 0` where the un-written poison would have been
  `0xDEAD`. NewTOS reproduces it (`tos-api/src/vdi/escape.rs`, sub-op 18).

### 19 — `v_rmcur` (`escfn19`, `vdi_esc.c:284-287`)

- **No input, no output.** Calls `vdi_v_hide_c()` (VDI opcode 123) to
  remove the graphics cursor.

Sub-functions 4–10, 13, 14, and 18/19 share the mouse/graphics-cursor
and console paths documented in [`input/_mouse.md`](input/_mouse.md).

---

## Verification criteria

- **`vq_chcells` (sub 1):** on a screen whose text grid is `R` rows ×
  `C` columns, returns `CONTRL[4]=2`, `INTOUT[0]=R`, `INTOUT[1]=C`.
- **`vq_curaddress` (sub 15):** with the alpha cursor at row `r`, col `c`
  (1-based), returns `CONTRL[4]=2`, `INTOUT[0]=r`, `INTOUT[1]=c`.
- **`vq_tabstatus` (sub 16):** returns `CONTRL[4]=1`, **`INTOUT[0]=1`**
  (always).
- **`vs_curaddress` (sub 11):** `INTIN=[1,1]` moves the cursor to the
  top-left cell (VT-52 `ESC Y 0x20 0x20`).
- **`v_dspcur` (sub 18):** `PTSIN` coordinates are ignored; the cursor is
  shown regardless.
- **Out-of-range:** `CONTRL[5] = 20` (or any value > 19, and ≠ 99 in a
  non-Bezier build) is a silent no-op.
- **No `PTSOUT` ever:** for every `CONTRL[5]` value 0..19, `CONTRL[2]`
  (point-output count) is `0` after the call — no sub-function writes a
  vertex.
- **`v_curtext` (sub 12) with a `0x00FF` character:** the byte is **not**
  written to the console (dropped; `xrawio`'s read-sentinel intercepts
  it instead) — this is source-observable behaviour to test against, not
  a hypothetical.
- **Virtual-workstation open/close does not toggle alpha mode:**
  `v_opnvwk`/`v_clsvwk` (100/101) never call `esc_init`/`esc_exit`; only
  `v_opnwk`/`v_clswk` (1/2) do.

---

## Edge cases discovered in source

1. **`vq_tabstatus` returns 1, not 0** (`vdi_esc.c:251`) — "there is a
   mouse". Easy to get wrong from the function name.
2. **`v_dspcur` ignores its coordinates** (`vdi_esc.c:274-278`) — Atari
   docs specify `v_dspcur(handle,x,y)` but TOS and EmuTOS both ignore
   `x,y` and show the cursor at its current position.
3. **`v_exit_cur` sends a leading `ESC H`** (`vdi_esc.c:81-84`) as a
   harmless sequence: if a `.TOS`/`.TTP` program left a bare `ESC` in the
   console before entering graphics mode, the first char of the real
   sequence would be eaten, so a throwaway `ESC H` absorbs it.
4. **Sub-function 99 = Bezier quality** (`vdi_esc.c:340-345`), only when
   `HAVE_BEZIER` — outside the normal 0..19 range, handled before the
   range check. `HAVE_BEZIER = 0` here, so it is a no-op.
5. **`v_curtext` uses GEMDOS `Crawio`, the others use BIOS `Bconout(2)`**
   (`vdi_esc.c:29-43,205`). The BIOS path is chosen for the cursor moves
   so they are uninterruptible by Ctrl-C; `v_curtext` deliberately uses
   raw GEMDOS console I/O for the text bytes.
6. **`v_curtext` cannot output character code `0xFF`** (`D-VDI-22`/global
   `D-083`) — `Crawio`'s `parm == 0xFF` read-vs-write sentinel
   (`bdos/console.c:445`) fires for that one character value, turning an
   intended *write* of `INTIN`-array byte `255` into a stdin *read*
   instead; the byte is silently dropped. Not a `v_escape`-side bug —
   `escfn12` does nothing wrong — but a real, source-confirmed gap in the
   `Crawio`-based text path this sub-function chose to use (edge case 5).
7. **`vs_curaddress` (sub 11) has no bounds check of its own** — the
   1-based `INTIN[0]`/`[1]` are truncated straight into a `char` VT-52
   byte before the VT-52 parser's `move_cursor()` clamp ever runs;
   out-of-contract-range values wrap mod 256 rather than saturating
   (`vdi_esc.c:172-183` vs. `bios/conout.c:416-429`). Well-formed
   in-range calls are unaffected.

---

## Dependencies

- `Bconout` (BIOS 3) for the VT-52 sequences; `Crawio` (GEMDOS **0x06**)
  for `v_curtext`; `Scrdmp` (XBIOS 20) for `v_hardcopy`.
  **Correction:** an earlier revision of this doc said "GEMDOS 0x07" —
  wrong. `Crawio` is GEMDOS function **0x06**
  (`include/bdosbind.h:22`: `#define Crawio(w) trap1(0x06, w)`;
  confirmed against [`../gemdos/console/06-crawio.md`](../gemdos/console/06-crawio.md),
  which documents the same function under its correct number). `0x07` is
  `Crawcin` (a *different*, unrelated GEMDOS function), not `Crawio`.
  Logged as `D-VDI-21`.
- `vdi_v_show_c` (122) / `vdi_v_hide_c` (123) for `v_dspcur`/`v_rmcur`.
- Line-A cell/cursor globals `v_cel_mx/my`, `v_cur_cx/cy` for the
  inquiries.
- The VT-52 console emulation (framebuffer text layer).

## Cross-references

- **Atari Compendium / Profibuch:** not present in `references/` (no PDFs).
  Cross-check of the documented `v_escape` sub-function ABI (esp. the
  `vq_tabstatus` return and the `v_dspcur` coordinate semantics) is
  **deferred** until the references are available.

## NewTOS implementation notes (Phase 12)

- The alpha cursor layer is the **VT-52 text console** the framebuffer
  already drives (Phase 06 console output). `v_enter_cur`/`v_exit_cur`
  toggle between the graphics workstation and that text layer; map them
  onto the existing console cursor/erase routines rather than porting the
  VT-52 byte streams literally.
- `vq_chcells`/`vq_curaddress` report rows/cols from the framebuffer font
  cell geometry; keep the values 1-based on output, matching EmuTOS. The
  count must be **live-derived** from the current device geometry (see
  the `vq_chcells` section above), not a boot-time constant — matching
  EmuTOS's `font_set_default()` pattern. The **cell itself** is that
  function's font, too: 8×8 when `V_REZ_VT < 400`, else 8×16
  (`bios/font.c:76`) — glyph rows, cursor height and scroll steps included
  (V32: `tos_api::vdi::font::font_set_default`, used by `escape.rs` for every
  cell; before V32 the console drew 8×16 on every screen).
- `v_hardcopy` (17) is a documented no-op (no printer) — see above.
- `v_dspcur`/`v_rmcur` bind to the mouse-cursor sprite path
  ([`input/_mouse.md`](input/_mouse.md)); ignore the `PTSIN` coordinates
  exactly as EmuTOS does.
- **Current NewTOS status (CON04, 2026-10-22):** the alpha sub-functions
  write their VT52 strings into the one console automaton
  (`tos-api/src/vdi/escape.rs`, spec `docs/api/bios/_vt52.md`), as
  `bios_conws` does; `v_curtext` (12) goes through `Crawio`
  (`gemdos::console::rawio`) and so keeps the `D-083` `0x00FF` sentinel;
  `vq_chcells`/`vq_curaddress` read the console's live `v_cel_*`/`v_cur_*`.
  **Deviation Q6:** `vs_curaddress` (11) sets the position directly through
  `move_cursor`'s clamp instead of the byte-coded `ESC Y` (no 223-column
  limit, no wrap-around; `docs/COMPATIBILITY.md`). `esc_init`/`esc_exit`
  run in `v_opnwk`/`v_clswk` (`vdi_control.c:564,589`).
