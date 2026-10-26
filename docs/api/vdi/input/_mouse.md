# VDI mouse cursor + exchange-vector functions (Phase 00b D6)

> Source: EmuTOS @ `1a7484ab`. Verified against:
> `vdi/vdi_mouse.c:513` (`vdi_vsc_form`) → `:1138` (`linea_transform_mouse`)
> → `:450` (`set_mouse_form`); `:321` (`vdi_v_show_c`) → `:1125`
> (`linea_show_mouse`) → `:127` (`dis_cur`); `:331` (`vdi_v_hide_c`) →
> `:1133` (`linea_hide_mouse`) → `:157` (`hide_cur`); `:341`
> (`vdi_vq_mouse`); `:370`/`:391`/`:414` (`vdi_vex_butv`/`motv`/`curv`);
> `vdi/vdi_misc.c:100` (`vdi_vex_timv`); `vdi/vdi_mouse.c:438`
> (`vdi_vex_wheelv`, gated); MFORM struct `include/aesdefs.h:441-450`;
> `validate_color_index` `vdi/vdi_control.c:224`; `Tickcal()` = BIOS 0x06
> (`include/biosbind.h:25`). Opcode numbers from
> [`../_dispatcher.md`](../_dispatcher.md) jump table 2.
> Calling convention: [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`../../../COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract** (must match TOS exactly): the opcode numbers, the 37-WORD
> MFORM `INTIN` layout, the hotspot/colour constraint rules, the hide-count
> semantics of show/hide, the `vq_mouse` `INTOUT`/`PTSOUT` output, and the
> `vex_*` "new in `CONTRL[7..8]`, old out in `CONTRL[9..10]`" exchange
> shape. **Reference** (NewTOS reimplements freely): `set_mouse_form`'s
> interleaved `Mcdb.maskdata[]`, the Line-A `mouse_cdb`/`mcs_ptr` storage,
> `cur_display`/`cur_replace` sprite rasterisation, and the VBL draw path —
> NewTOS uses its own `RasterBackend` (`tos-api/src/vdi/mouse.rs`,
> `backend.rs`).

| Op | Fn | Input | Output | Effect |
|----|----|-------|--------|--------|
| 111 | `vsc_form`  | `INTIN[0..36]` = MFORM (37 WORDs) | — | set cursor shape |
| 122 | `v_show_c`  | `INTIN[0]` = reset flag | — | show cursor / decrement hide-count |
| 123 | `v_hide_c`  | — | — | hide cursor / increment hide-count |
| 124 | `vq_mouse`  | — | `INTOUT[0]`=buttons; `PTSOUT[0..1]`=x,y | query mouse |
| 125 | `vex_butv`  | `CONTRL[7..8]`=new routine | `CONTRL[9..10]`=old | install button vector |
| 126 | `vex_motv`  | `CONTRL[7..8]`=new routine | `CONTRL[9..10]`=old | install motion vector |
| 127 | `vex_curv`  | `CONTRL[7..8]`=new routine | `CONTRL[9..10]`=old | install cursor-draw vector |
| 118 | `vex_timv`  | `CONTRL[7..8]`=new routine | `CONTRL[9..10]`=old; `INTOUT[0]`=ms/tick | install timer vector |
| 134 | `vex_wheelv`| `CONTRL[7..8]`=new routine | `CONTRL[9..10]`=old | install wheel vector (**`CONF_WITH_EXTENDED_MOUSE` only**) |

None of these has an error return (VDI convention: silent no-op on a bad
handle, per the dispatcher).

---

## `vsc_form` (111) — SET CURSOR FORM (install MFORM)

**Primary location:** `vdi/vdi_mouse.c:513` (`vdi_vsc_form`) →
`linea_transform_mouse()` (`:1138`) → `set_mouse_form(INTIN, &mouse_cdb)`
(`:450`).
**Helpers:** `validate_color_index()` (`vdi_control.c:224`) — clamp a
colour index; `MAP_COL[]` — pen → hardware PEL.
**Globals read:** `MAP_COL[]`, `numcolors` (via `validate_color_index`).
**Globals written:** `mouse_cdb` (the sprite); `mouse_flag`
(bracketed +1/−1 to suppress cursor redraw while redefining).
**Build config:** none.

### Parameter-block interface

`INTIN[0..36]` is a 37-WORD `MFORM` (`include/aesdefs.h:441-450`):

| INTIN index | MFORM field | Meaning |
|:-----------:|-------------|---------|
| 0 | `mf_xhot` | hot-spot x (0..15; masked `& 0x0f`) |
| 1 | `mf_yhot` | hot-spot y (0..15; masked `& 0x0f`) |
| 2 | `mf_nplanes` | reserved, must be 1 (**ignored** by `set_mouse_form`) |
| 3 | `mf_bg` | **mask** colour index (background) |
| 4 | `mf_fg` | **data** colour index (foreground) |
| 5..20 | `mf_mask[0..15]` | 16 mask rows (16 bits each, MSB = leftmost pixel) |
| 21..36 | `mf_data[0..15]` | 16 data rows |

`CONTRL[3]` (`N_INTIN`) = 37; `CONTRL[1]`/`N_PTSIN` = 0; no outputs
(`{nptsout=0, nintout=0}`).

### Behaviour

`set_mouse_form` (`vdi_mouse.c:450-492`):

1. Set `mouse_flag += 1` to disable cursor updates during the redefine.
2. **Hotspot constrained** to 0..15: `xhot = mf_xhot & 0x0f`,
   `yhot = mf_yhot & 0x0f` (`:461-464`) — values above 15 wrap, never
   overflow the 16×16 sprite.
3. **Colours mapped** through `validate_color_index` then `MAP_COL`:
   `bg_col = MAP_COL[validate_color_index(mf_bg)]`, likewise `fg_col`
   (`:466-472`). `validate_color_index` returns **1** for any index `< 0`
   or `≥ numcolors` (`vdi_control.c:224-230`) — an out-of-range mask/data
   colour silently becomes pen 1.
4. **Interleave** the 16 mask + 16 data rows into `Mcdb.maskdata[32]` as
   `mask[0], data[0], mask[1], data[1], …` (`:482-489`) — a **reference**
   storage detail (device-specific sprite format); the contract is only the
   37-WORD `INTIN` input.
5. `mouse_flag -= 1` re-enables drawing.

The default installed at init is the arrow `arrow_mform` (hotspot 1,0;
`mf_bg=0`, `mf_fg=1` — but only in the `!WITH_AES` build; with AES the AES
supplies the default form) (`vdi_mouse.c:69-110,593`).

> **Correction (D-VDI-20).** An earlier revision of this doc had the
> colours backwards (`mf_bg=1, mf_fg=0`). `arrow_mform`'s initializer
> `{ 1, 0, 1, 0, 1, {...}, {...} }` maps positionally onto `MFORM`'s
> field order `mf_xhot, mf_yhot, mf_nplanes, mf_bg, mf_fg, mf_mask[16],
> mf_data[16]` (`vdi_mouse.c:69-73`; struct `include/aesdefs.h:441-449`):
> `mf_xhot=1, mf_yhot=0, mf_nplanes=1, mf_bg=0, mf_fg=1`. Consistent
> with a black arrow (pen 1, the `mf_data`/foreground shape) outlined in
> white (pen 0, the `mf_mask`/background shape).

### Verification criteria

- `vsc_form` with hotspot `(3,5)`, `mf_bg=1`, `mf_fg=0`, a known 16-row
  mask/data → the installed sprite has `xhot=3`, `yhot=5`,
  `bg_col=MAP_COL[1]`, `fg_col=MAP_COL[0]`, and its 16×16 shape matches the
  supplied rows.
- `vsc_form` with `mf_bg = 256` (out of range on NewTOS' 256-pen truecolor
  device, `backend.num_colors()`, `tos-api/src/vdi/mouse.rs:569-570`) → the
  mask colour clamps to pen 1 (`validate_color_index`). **Correction (A86,
  2026-09-21):** this criterion previously used `mf_bg = 99` "on a
  16-colour device" — with the live 256-colour backend, 99 is a valid pen
  and would **not** clamp; the trigger value is corrected to exceed the
  actual pen count.

---

## Cursor visibility — `v_show_c` (122) / `v_hide_c` (123)

**Primary locations:** `vdi_mouse.c:321` (`vdi_v_show_c`) → `linea_show_mouse`
(`:1125`) → `dis_cur` (`:127`); `vdi_mouse.c:331` (`vdi_v_hide_c`) →
`linea_hide_mouse` (`:1133`) → `hide_cur` (`:157`).
**Globals read/written:** `HIDE_CNT` (the hide-depth counter); `draw_flag`.
**Build config:** none.

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode 122 (`v_show_c`) or 123 (`v_hide_c`) |
| `INTIN[0]` | (`v_show_c` only) **reset flag**: 0 = force show |
| — | no outputs (`{0,0}`) |

### Behaviour — hide-count model (contract)

TOS tracks a **hide depth** `HIDE_CNT`; the cursor is visible only when it
reaches 0. Show and hide are counted so nested hide/show pairs balance:

- **`v_hide_c`** (`hide_cur`, `:157-169`): `HIDE_CNT += 1`; if it becomes
  exactly 1 (was 0 → visible), physically remove the cursor
  (`cur_replace`). Further hides just deepen the count without redrawing.
- **`v_show_c`** (`dis_cur`, `:127-141`): if `HIDE_CNT != 1`, decrement it
  and, **if it went negative, clamp to 0** (`:132-133`) — so
  **`v_show_c` underflow is silently absorbed**, never uncovering the
  cursor more than a matching hide would. If `HIDE_CNT` is exactly 1, this
  is the last outstanding hide: physically display the cursor
  (`cur_display`) and decrement to 0.

### The reset flag (`v_show_c` `INTIN[0]`)

`linea_show_mouse` (`:1125-1131`): **if `INTIN[0] == 0` and the cursor is
currently hidden (`HIDE_CNT != 0`), force `HIDE_CNT = 1` before calling
`dis_cur`** — i.e. a "reset" show collapses any hide depth to a single
level so the following `dis_cur` immediately makes the cursor visible.
A non-zero `INTIN[0]` performs a normal counted show. (`v_dspcur`, escape
sub 18, calls `v_show_c` with `INTIN[0]` pre-set to 0 for exactly this
force-show, [`../_escape.md`](../_escape.md).)

### Verification criteria

- After init (`HIDE_CNT = 1`, hidden), `v_show_c(INTIN[0]=1)` → `HIDE_CNT =
  0`, cursor shown.
- `v_hide_c` ×3 then `v_show_c` ×2 → `HIDE_CNT = 1`, still hidden (nesting
  balances).
- `v_show_c` with `HIDE_CNT = 0` → stays 0 (underflow clamped, no double
  show).
- `v_show_c(INTIN[0]=0)` while `HIDE_CNT = 5` → forces to 1 then shows
  (reset-show).

---

## `vq_mouse` (124) — QUERY POSITION + BUTTONS

**Primary location:** `vdi/vdi_mouse.c:341` (`vdi_vq_mouse`).
**Globals read:** `MOUSE_BT`, `GCURX`, `GCURY` (read with interrupts
masked, `set_sr(0x2700)`, for an atomic snapshot).
**Build config:** none.

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode = 124 |
| `CONTRL[2]` (`N_PTSOUT`) | 1 (dispatcher pre-load) |
| `CONTRL[4]` (`N_INTOUT`) | 1 (dispatcher pre-load) |
| `INTOUT[0]` | out: button bitmask (`MOUSE_BT`) |
| `PTSOUT[0]`, `PTSOUT[1]` | out: cursor x, y (`GCURX`, `GCURY`) |

### Behaviour

Atomic read (`vdi_mouse.c:343-351`): `INTOUT[0] = MOUSE_BT`,
`PTSOUT[0] = GCURX`, `PTSOUT[1] = GCURY`. Button bits: 0 = left, 1 = right
(plus 2-4 for extended buttons under `CONF_WITH_EXTENDED_MOUSE`), per the
VDI/GEM convention — see [`_mouse-packet.md`](_mouse-packet.md) for the
IKBD→`MOUSE_BT` bit swap. Position is the clamped `GCURX`/`GCURY`
maintained by the packet handler.

### Verification criteria

- With `MOUSE_BT = 0x01`, `GCURX = 100`, `GCURY = 50` → `INTOUT[0] = 1`,
  `PTSOUT[0] = 100`, `PTSOUT[1] = 50`.
- No buttons down → `INTOUT[0] = 0`.

---

## Exchange vectors — `vex_butv`/`motv`/`curv` (125-127), `vex_timv` (118), `vex_wheelv` (134)

**Primary locations:** `vdi_mouse.c:370` (`vex_butv`), `:391` (`vex_motv`),
`:414` (`vex_curv`), `:438` (`vex_wheelv`, gated); `vdi_misc.c:100`
(`vex_timv`).
**Globals written:** the matching vector slot (below).
**Build config:** `vex_wheelv` only exists when `CONF_WITH_EXTENDED_MOUSE`
is defined (`vdi_mouse.c:422-443`; opcode 134 also gated in the
dispatcher).

### The exchange shape (contract, all five identical)

Each call **installs a new routine and returns the previous one** so the
caller can chain it:

| Slot | Meaning |
|------|---------|
| `CONTRL[7]`, `CONTRL[8]` | in: pointer to the new routine (32-bit, high/low WORDs) |
| `CONTRL[9]`, `CONTRL[10]` | out: pointer to the previous routine |

Implementation (`vdi_mouse.c:372-373`, identical for each): write the old
global into `CONTRL[9..10]`, then load the new pointer from `CONTRL[7..8]`
into the global. `vex_timv` and `vex_butv`/`motv`/`curv` differ only in
which global they swap and (for `vex_timv`) an extra output.

| Fn | Op | Global swapped | Called when | Init value |
|----|:--:|----------------|-------------|-----------|
| `vex_butv` | 125 | `user_but` | button state changes (from `mouse_int`) | `just_rts` |
| `vex_motv` | 126 | `user_mot` | cursor coordinate changes | `just_rts` |
| `vex_curv` | 127 | `user_cur` | cursor is (re)drawn | `mov_cur` |
| `vex_timv` | 118 | `tim_addr` | Timer-C tick (via `tick_int`) | `just_rts` |
| `vex_wheelv` | 134 | `user_wheel` | wheel movement (`CONF_WITH_EXTENDED_MOUSE`) | `just_rts` |

The button/motion/cursor vectors are called from the IKBD mouse packet
handler in the exact order button → motion → cursor — see
[`_mouse-packet.md`](_mouse-packet.md).

#### NewTOS: the slot index follows the caller's pointer width

`CONTRL[7..8]` / `CONTRL[9..10]` is the m68k reading of a rule gemlib states
in terms of the caller's ABI: VDI pointer `n` starts at CONTRL word
`7 + n * N_PTRINTS` with `N_PTRINTS = sizeof(void *) / sizeof(short)`
(`sdk/gemlib/gemlib-0.44.0/gem_vdiP.h:177,248`). That is 2 WORDs on m68k —
whence EmuTOS' fixed `CONTRL[9]` — and **4** for an LP64 NewTOS application,
whose `old` therefore lives at `CONTRL[11..14]`. NewTOS reads and writes both
slots at the calling process's width (`Vdipb::contrl_ptr` /
`Vdipb::set_contrl_ptr`); the same rule already governs the raster ops' MFDB
pointers. Test: `tos-api/tests/vdi_vex_wheelv.rs`.

### `vex_timv` extra output (`vdi_misc.c:100-112`)

Beyond the vector swap, `vex_timv` also sets **`INTOUT[0] = Tickcal()`** —
the number of milliseconds between Timer-C ticks (`Tickcal` = BIOS 0x06).
`CONTRL[4]` (`N_INTOUT`) is 1 (dispatcher pre-load). The swap runs with
interrupts masked (`set_sr(0x2700)`). The installed `tim_addr` is invoked
by `tick_int`, which also chains the old system timer vector
(`vdi_misc.c:78-90`).

### Verification criteria

- `vex_butv(new=P)` → `user_but` becomes `P`; `CONTRL[9..10]` holds the
  previous `user_but` (e.g. `just_rts` right after init).
- `vex_timv(new=P)` → `tim_addr` becomes `P`; `CONTRL[9..10]` = old
  `tim_addr`; `INTOUT[0]` = ms/tick from `Tickcal()`.
- Two successive `vex_curv` installs → the second call's `CONTRL[9..10]`
  returns the first call's pointer (chaining works).

---

## Edge cases discovered in source

- **`v_show_c` underflow is clamped, not wrapped** (`dis_cur:132-133`):
  extra shows never make `HIDE_CNT` negative, so an unbalanced show cannot
  "over-uncover" the cursor.
- **`v_show_c` reset flag collapses the hide depth** to 1 before showing
  (`linea_show_mouse:1127-1128`) — a force-show regardless of how deeply
  nested the hides were.
- **Out-of-range MFORM colour → pen 1**, silently
  (`validate_color_index:226-227`), not an error.
- **`vsc_form` ignores `mf_nplanes`** (`INTIN[2]`): the "reserved, must be
  1" field is never read by `set_mouse_form`.
- **`vex_*` return the raw 32-bit pointer** in `CONTRL[9..10]` even if the
  vector was never changed (returns the init default `just_rts`/`mov_cur`).

---

## Dependencies

### Requires

- **Mouse packet handler** feeding `GCURX`/`GCURY`/`MOUSE_BT`/`cur_ms_stat`
  and calling `user_but`/`user_mot`/`user_cur`
  ([`_mouse-packet.md`](_mouse-packet.md)).
- **Timer-C tick** for `vex_timv`/`tim_addr` (Phase-07 system timer).
- The workstation must be open (`CUR_WORK`) so `MAP_COL[]`/`numcolors`
  exist for `vsc_form`.

### Used by

- `v_dspcur`/`v_rmcur` (escape sub 18/19) call `v_show_c`/`v_hide_c`
  ([`../_escape.md`](../_escape.md)).
- The AES installs `user_but`/`user_mot` via `vex_butv`/`vex_motv` to drive
  `evnt_button`/`evnt_mouse`.

---

## Related data structures

- **MFORM (37-WORD)** — table above; struct `include/aesdefs.h:441-450`.
- **Mouse-packet / button-bit / `cur_ms_stat` layout** →
  [`_mouse-packet.md`](_mouse-packet.md).
- **Shared cursor/vector globals** (`HIDE_CNT`, `GCURX/Y`, `MOUSE_BT`,
  `user_but/mot/cur`, `tim_addr`, `mouse_cdb`, `mcs_ptr`) →
  [`../_state.md`](../_state.md#input-and-cursor-state).

---

## Cross-references

- **Atari Compendium / Profibuch:** cross-check of the MFORM layout, the
  hide-count semantics, and the `vex_*` ABI is **deferred** — no PDFs in
  `references/` (see [`../_discrepancies.md`](../_discrepancies.md)).
- **XBIOS:** `vsc_form` cursor delivery and the button/motion vectors
  depend on the IKBD mouse set up by `Initmous`
  ([`../../xbios/00-initmous.md`](../../xbios/00-initmous.md)); the
  keyboard/mouse vector base is `Kbdvbase`.

---

## Configuration / version notes

- `vex_wheelv` (op 134) and extended mouse buttons 3-5 exist only under
  `CONF_WITH_EXTENDED_MOUSE`; without it, opcode 134 is a dispatcher no-op
  and `MOUSE_BT` has only bits 0-1.
- `CONF_WITH_VDI_16BIT` selects a separate `cur_display16`/`cur_replace16`
  sprite path (TrueColor) — a **reference** rasterisation detail, invisible
  to the ABI.

## NewTOS implementation notes

- Cursor + button + position come from the **input subsystem** (USB-HID /
  IKBD), not the framebuffer. For QEMU bring-up the source is the
  virtio/USB-HID device; until wired, `vq_mouse` returns a parked cursor and
  zero buttons (documented limitation — NOT a stub returning fake motion).
- **`CONF_WITH_EXTENDED_MOUSE` is implemented** (GUI04, 2026-09-26):
  `mousexvec` (`tos-api/src/bios/vectors.rs`) carries the seven extended
  scancodes from the IKBD decoder to `mousex_handler`
  (`tos-api/src/vdi/mouse.rs`, EmuTOS `vdi_mousex_handler`), which maintains
  `MOUSE_BT` bits 2..4 for buttons 3/4/5 and calls the `user_wheel` vector for
  the four wheel codes. The USB source is `kernel/src/drivers/usb/hid.rs`,
  whose boot-report byte 3 becomes scancode `0x59`/`0x5a`.
- **The `user_wheel` vector is really dispatched**, unlike the other four
  `vex_*` slots (AUD-V6-002), because an application must be able to see wheel
  events. Three cases: the `just_rts` default does nothing; the AES's own
  handler is a same-privilege EL1 call; an application's handler is delivered
  as an EL0 upcall (`docs/analysis/coop-kernel/10-upcall-signal-design.md`,
  consumer **vex**) to the **owning** process only, at its next return to EL0.
  There is no lower-EL IRQ route, so the call is not ISR-timed (design §7.3).
  `vex_butv`/`motv`/`curv`/`timv` keep their address-sentinel behaviour: their
  default action is inlined in `feed_relative`, and no consumer has asked for
  the upcall yet.
- **ARCH03 stage 3a (F4) — deliberate no-op, re-checked against EmuTOS.**
  EmuTOS **does** call all four: `mouse_int` jumps through `user_but` on a
  button change with the new state in `d0` (`vdi/vdi_asm.S:283-284`), through
  `user_mot` with the clipped position in `d0`/`d1` **and takes the
  routine's `d0`/`d1` back as the new position** (`:359-363`, "call user to
  modify x,y"), and through `user_cur` to draw the cursor (`:367-368`);
  `tick_int` calls `tim_addr` on every timer tick (`vdi/vdi_misc.c:83`). All
  of it runs in interrupt context in supervisor mode. NewTOS stores the
  address an application installs and returns the previous one exactly, but
  **executes none of them** — so under per-process rights (ARCH03 stage 3)
  nothing follows from a stored EL0 address: there is no access through it.
  The `vex_wheelv` upcall cannot simply be reused: it is fire-and-forget,
  delivered at the owner's next return to EL0, whereas `user_mot` must hand
  a modified coordinate pair back into the very packet being processed and
  `tim_addr` is tick-timed. That needs an upcall contract with a return
  channel (and an IRQ-timed delivery decision) that has not been designed;
  it is not invented here. Open, for the lead: a program that relies on
  `vex_butv`/`motv`/`curv`/`timv` callbacks sees none (as before 3a).
- `vsc_form` installs the 37-WORD MFORM into the NewTOS `RasterBackend`
  sprite (`tos-api/src/vdi/mouse.rs`); the interleaved `Mcdb` layout is an
  EmuTOS detail — NewTOS may store the sprite however it likes, as long as
  the hotspot/colour constraint rules (mask `& 0x0f`, out-of-range → pen 1)
  are reproduced.
- The hide-count (`HIDE_CNT`) with underflow-clamp and reset-flag force-show
  is **contract** — reimplement exactly; nested AES hide/show relies on it.
- `vex_*` vectors are real function-pointer slots the IRQ/timer path calls;
  install/return semantics must be exact (programs chain the old vector).
  Pointer ABI is 32-bit for m68k callers; native callers use the NewTOS
  pointer width (see the trap pointer mapping).
- `vex_timv` ties into the system tick (Phase-07 timer); `Tickcal` (BIOS
  0x06) returns the tick period in ms for `INTOUT[0]`.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the Line-A cursor sprite drawn over the
> Shifter framebuffer, driven by the IKBD mouse ISR) at the NewTOS profile:
> the cursor is a **software 16×16 sprite composited onto the VideoCore
> framebuffer** by the `RasterBackend`, and its position/buttons come from the
> **USB-HID mouse** (via the IKBD packet synthesiser). The **contract is
> unchanged** — the opcode numbers, the 37-WORD MFORM `INTIN`, the hide-count
> semantics, `vq_mouse`'s `INTOUT`/`PTSOUT`, and the `vex_*` install/return
> shape. Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2 (Screen →
> framebuffer; Keyboard/mouse → USB-HID) and §3.1/§3.3.

- **Cursor rendering.** The sprite is drawn by save-under + XOR-style
  mask/data compositing directly into the framebuffer via the
  `RasterBackend` (`tos-api/src/vdi/mouse.rs:200-243`): `draw` saves the
  16×16 pixels under the cursor, paints data pixels in the FG pen and mask
  pixels in the BG pen; `erase` restores them. No hardware sprite — the
  Shifter cursor overlay is replaced by this software composite over the
  32bpp VideoCore buffer. **Correction (A86, 2026-09-21):** this used to say
  "the 16 CLUT indices map to RGB" — the live backend has **256** pens, each
  independently settable (not a 16-entry CLUT); the default mask/data pens
  (white/pen 0, black/pen 1) map to RGB through the per-workstation
  pseudo-palette in `kernel/src/vdi_backend.rs:129-142,232-239` — see
  [`docs/api/vdi/raster/_raster.md`'s "Palette" section](../raster/_raster.md#palette--vs_colorvq_color-on-256-truecolor-pens).
- **MFORM install (`vsc_form`, 111).** `vsc_form` reads the 37-WORD `INTIN`
  (hotspot, reserved plane word, mask/data colours, 16 mask + 16 data rows)
  into the sprite state (`tos-api/src/vdi/mouse.rs:248-266`); the hotspot is
  clamped to 0..15 (`:254-255`). Eight AES preset forms (arrow, I-beam,
  hourglass, …) are compiled in from `aes/mform.rsc` (`mouse.rs:64-97`);
  `graf_mouse` installs them via `set_preset`/`set_form`.
- **Position + buttons (`vq_mouse`, 124).** `GCURX`/`GCURY`/`MOUSE_BT` come
  from the **USB-HID mouse** through the IKBD packet path (`feed_relative`,
  `mouse.rs:126-134`) — see `_mouse-packet.md`. `vq_mouse`
  (`mouse.rs:349-355`) reports them and pre-loads `CONTRL[2]=1`,
  `CONTRL[4]=1`. **Correction to the notes above:** they say the mouse is
  "not yet wired" and `vq_mouse` returns a parked cursor — that is now
  **stale**: `feed_relative` is wired to the HID mouse (`ikbd.rs:121`), so the
  cursor moves and `vq_mouse` returns the live position. What is still true:
  buttons and motion are delivered by the *direct* `feed_relative` call, not
  through the `vex_*` vectors.
- **Hide-count.** The `HIDE_COUNT` model (power-up hidden = 1, nested
  hide/show balance, underflow clamp, reset-flag force-show) is implemented
  in `v_show_c`/`v_hide_c` (`mouse.rs:270-289`) and the kernel-desktop
  direct-control `hide`/`show`/`show_reset` (`:300-326`), matching the
  contract.
- **Step-2 — exchange vectors not driven; `Tickcal` unit.** `vex_butv`/
  `vex_motv`/`vex_curv`/`vex_wheelv`/`vex_timv` install/return their pointer
  slots correctly (`mouse.rs:359-388`), but the mouse packet path
  (`feed_relative`) does **not** call `user_but`/`user_mot`/`user_cur` on a
  packet, nor does any timer tick invoke `tim_addr`. So a program that hooks
  a `vex_*` vector never sees a callback — the cursor/AES event path uses the
  direct call instead. `vex_timv` returns `TICKCAL_MS = 20` for `INTOUT[0]`
  (`mouse.rs:17,384-388`), matching `Tickcal`. Recorded as K-series in
  `docs/STEP2-VALIDATION.md`.
- **Datasheet.** Framebuffer transport = VideoCore mailbox (§1.3); mouse
  transport = VL805 xHCI (§2). No peripheral cursor register.
