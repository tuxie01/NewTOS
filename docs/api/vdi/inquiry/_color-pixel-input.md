# VDI colour / pixel / input-mode inquiries (Phase 00b D5)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_col.c:845` (`vdi_vq_color`),
> `vdi/vdi_fill.c:1095` (`vdi_v_get_pixel`), `vdi/vdi_input.c:154`
> (`vdi_vqin_mode`), `vdi/vdi_input.c:110` (`vdi_vq_key_s`). Dispatcher
> jump-table entries + output counts: `vdi/vdi_main.c:58,80,90,103`.
> Calling convention: [`../_dispatcher.md`](../_dispatcher.md). `vq_extnd`
> (102) is covered in
> [`../workstation/102-vq_extnd.md`](../workstation/102-vq_extnd.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** the opcodes; the input slots; the return-slot maps; the
> **raw-vs-actual** colour distinction (`vq_color` returns the *unclamped*
> `REQ_COL` for flag 0, the DAC read-back for flag ≠0); the out-of-range
> `−1` marker; the `vqin_mode` `mode−1` return quirk; the `Kbshift & 0x0f`
> mask. **Reference:** the ST/STe/TT/VIDEL DAC scalers, the palette
> storage, the framebuffer pixel-read path — NewTOS substitutes its own.

## Opcodes and output counts

| Op | Fn | Input | `# INTOUT` | `# PTSOUT` | Src |
|----|----|-------|:----------:|:----------:|-----|
| 26 | `vq_color`    | `INTIN[0]`=colour idx, `INTIN[1]`=flag | 4 | 0 | `vdi_main.c:58` |
| 105 | `v_get_pixel` | `PTSIN[0..1]`=x,y | 2 | 0 | `vdi_main.c:80` |
| 115 | `vqin_mode`   | `INTIN[0]`=device (1..4) | 1 | 0 | `vdi_main.c:90` |
| 128 | `vq_key_s`    | — | 1 | 0 | `vdi_main.c:103` |

Native ABI (all): GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in
`x1`; opcode in `CONTRL[0]`, handle in `CONTRL[6]`.

---

## `vq_color` (26) — query colour-index table

Source: `vdi_col.c:845-934`. Reads back a palette entry as VDI
RGB (three components, each **0..1000** *nominally*).

**Inputs:** `INTIN[0]` = colour index, `INTIN[1]` = flag
(0 = last *requested* value, ≠0 = *actual* hardware value).

**Behaviour (ST-shifter / palette path):**

1. `INTOUT[1] = INTOUT[2] = INTOUT[3] = 0` (default RGB) (`:851`).
2. If `INTIN[0] < 0` or `INTIN[0] >= numcolors` → `INTOUT[0] = -1`,
   RGB left 0, **return** (`:854-859`).
3. `INTOUT[0] = INTIN[0]` (the valid index) (`:860`).
4. **flag == 0 (requested):** `INTOUT[1..3]` = `REQ_COL[idx]` (or
   `req_col2[idx-16]` for `EXTENDED_PALETTE` indices ≥ 16) — the value
   **`vs_color` stored verbatim, unclamped** (`:875-892`).
5. **flag ≠ 0 (actual):** read the hardware DAC register
   `MAP_COL[idx] & (numcolors-1)` and scale it to VDI 0..1000 via the
   shifter-specific scaler (`st2vdi`/`ste2vdi`/`videl2vdi`,
   `:897-933`).

| Slot | Value |
|------|-------|
| `INTOUT[0]` | valid index, or `-1` if out of `[0, numcolors)` |
| `INTOUT[1]` | red   (requested `REQ_COL` raw, **or** actual DAC → 0..1000) |
| `INTOUT[2]` | green (as above) |
| `INTOUT[3]` | blue  (as above) |

> **Raw-request read-back (contract).** `vs_color` (op 14) copies the raw
> `INTIN[1..3]` into `REQ_COL[idx]` **before** clamping to 0..1000 for the
> DAC (`vdi_col.c:687-700`). So `vq_color(idx, flag=0)` returns the
> *unclamped* request — it can hand back a component **< 0 or > 1000** if
> the program set one. The clamped value only shows up on the flag ≠ 0
> ("actual") path, after a DAC round-trip that may also differ by
> rounding. This is the shared palette-request state documented in
> [`../_state.md`](../_state.md#the-colour-request-palette-req_col--req_col2).

**Truecolor (`CONF_WITH_VDI_16BIT`, `v_planes > 8`):** delegates to
`vq_color16` (`:862-868`) — out of the palette bring-up scope.

## `v_get_pixel` (105) — read a pixel

Source: `vdi_fill.c:1095-1115`. Reads the pixel at `PTSIN[0]`,`PTSIN[1]`
via `pixelread(x,y)`.

| | Palette screen | Truecolor (`CONF_WITH_VDI_16BIT`) |
|--|----------------|-----------------------------------|
| `INTOUT[0]` | `pel` = raw pixel value (plane bits / index) | `0` |
| `INTOUT[1]` | `REV_MAP_COL[pel]` = VDI pen index | `pel` = 16-bit RGB |

So a palette program gets **both** the raw device value (`INTOUT[0]`)
**and** the logical VDI pen (`INTOUT[1]`) in one call. No range/error
return.

### Coordinates outside the screen

`pixelread` (`vdi_fill.c:671-690`) has no bounds check: it reads the word
at the screen address of `(x, y)` whatever `(x, y)` is. The truecolor
address is `v_bas_ad + x*2 + muls(y, v_lin_wr)` (`get_start_addr16`,
`vdi/vdi_inline.h:14-17`), the planar one `v_bas_ad + ((x & 0xfff0) >>
v_planes_shift) + muls(y, v_lin_wr)` with the bit `0x8000 >> (x & 0xf)`
(`get_start_addr`, `vdi/vdi_misc.c:166-177`; `vdi_fill.c:685-689`). Both
are linear in the screen memory, so an `x` past the right edge reads the
start of the next row and an `x` left of 0 the end of the previous row:
pixel `(x, y)` is the pixel at **store index `y * stride + x`**, stride =
the row length in pixels (`v_lin_wr` / bytes per pixel). Measured on the
oracle (Falcon truecolor, `tosdump/cases/vdi-v18.tsv`, reference
`EMUTOS-1a7484ab-VDI-v18-falcontc`): `(640,10)` reads pixel `(0,11)`
(case 15), `(-1,11)` reads `(639,10)` (case 18), `(700,10)` reads
`(60,11)` (case 21); case 24 is the on-screen control.

A store index before the first pixel of the logical screen or past its last
(`height * stride` pixels from `Logbase`) is memory that is not the
screen; EmuTOS returns whatever word is there. **NewTOS** reads the same
store index inside the logical screen — the run-on into the neighbouring
row included, and on a framebuffer whose pitch is wider than the screen the
padding pixels — and answers **0** outside it: truecolor `INTOUT[0] = 0`,
`INTOUT[1] = 0`; palette `INTOUT[0] = INTOUT[1] = 0`. User decision B
(TASK/V18, 2026-11-05), recorded in `docs/COMPATIBILITY.md`. The notion of
"inside the logical screen" is the one lines with clipping off use
(`tos-api/src/vdi/line.rs:83@index`, decision A), so a pixel a run-on line
draws reads back at the same coordinates.

`v_contourfill` reads through the same `pixelread`, but only inside its
search bounds (the clip rectangle cut to the screen, seed outside → no
fill, `vdi_fill.c:965-967`), so the off-screen rule never reaches it.

## `vqin_mode` (115) — inquire input mode

Source: `vdi_input.c:154-176`. Returns, for the device in `INTIN[0]`
(1=locator, 2=valuator, 3=choice, 4=string), the stored mode global
(`loc_mode`/`val_mode`/`chc_mode`/`str_mode`) into `INTOUT[0]`.

> **`vqin_mode` returns `mode − 1`, not the documented 1/2.** EmuTOS —
> like every version of Atari TOS — stores `mode−1` in the globals
> (`vsin_mode` does `i = INTIN[1]; i--`, `vdi_input.c:122-123`) and
> returns that value **unmodified**: `0` = request, `1` = sample, **not**
> 1/2 (`vdi_input.c:146-152` comment + `:161-174`). A program that set
> REQUEST (1) reads back **0**; SAMPLE (2) reads back **1**.

| Slot | Value |
|------|-------|
| `INTOUT[0]` | `0` (request) or `1` (sample) for the queried device |

If `INTIN[0]` is not 1..4 the `switch` writes nothing and `INTOUT[0]`
holds whatever the caller left there. See
[`../_state.md`](../_state.md#input-mode-set-by-vsin_mode-33-read-by-vqin_mode-115)
for the mode globals and [`../input/_input.md`](../input/_input.md).

> **The string device is never in its default state under the AES.** The
> mode globals default to `0` (REQUEST) in `vdimouse_init`
> (`vdi_mouse.c:575-578`), but the EmuTOS **AES** issues
> `vsin_mode(4, 2)` — SAMPLE on the string device — on *every* keyboard
> poll: `gsx_char()` (`aes/gemgsxif.c:459-463`) sets it before each
> `v_string`, and `chkkbd()` calls `gsx_char()` from the dispatcher's
> scheduler on every pass (`aes/gemdisp.c:157-177`). So on any machine
> where the AES has run, `vqin_mode(4)` answers **1**, not 0 — which is
> what the measured EmuTOS 1.4 reference on a TT reports
> (`tosdump/reference/EMUTOS14-VDI-r2.BIN`, case 51, and
> `EMUTOS14-VDI-all.BIN`, case 77). The other three devices stay 0. This
> is AES-side state, not a VDI default; a VDI-only harness measures 0.

## `vq_key_s` (128) — inquire shift-key state

Source: `vdi_input.c:110-113`, `gshift_s` at `:185-188`.

`INTOUT[0] = gshift_s()` = **`Kbshift(-1) & 0x000f`** — only the low
4 bits: right-shift `0x01`, left-shift `0x02`, ctrl `0x04`, alt `0x08`.
Caps-lock (`0x10`) and any higher BIOS `Kbshift` bits are **masked off**,
so `vq_key_s` reports *less* than a raw `Kbshift`.

| Slot | Value |
|------|-------|
| `INTOUT[0]` | shift/ctrl/alt bitmask, `Kbshift & 0x0f` |

---

## Verification criteria

- **`vq_color` requested (flag 0):** after `vs_color(5, 200,400,600)` →
  `vq_color(5, flag=0)` returns `INTOUT[0]=5`, `INTOUT[1..3]=200,400,600`
  verbatim.
- **`vq_color` raw unclamped:** after `vs_color(5, -50, 1500, 0)` →
  `vq_color(5, flag=0)` returns `INTOUT[1..3] = -50, 1500, 0` (the raw,
  out-of-range request), while `vq_color(5, flag=1)` returns the DAC
  read-back clamped to `[0,1000]`.
- **`vq_color` out of range:** `vq_color(numcolors, flag)` →
  `INTOUT[0]=-1`, `INTOUT[1..3]=0`.
- **`v_get_pixel`:** at a pixel drawn in pen `p` (device value `pel`),
  `v_get_pixel(x,y)` returns `INTOUT[0]=pel`, `INTOUT[1]=p`
  (`= REV_MAP_COL[pel]`).
- **`v_get_pixel` off the screen (truecolor, 640×480):** `(640,10)` returns
  the word of `(0,11)`, `(-1,11)` that of `(639,10)`, `(700,10)` that of
  `(60,11)`; `(640,479)` and `(-1,0)` — past the last and before the
  first pixel — return `INTOUT[0..1] = 0, 0`.
- **`vqin_mode` quirk:** after `vsin_mode(dev=1, mode=2)` (sample),
  `vqin_mode(1)` → `INTOUT[0] = 1` (**not** 2); after `mode=1`
  (request) → `INTOUT[0] = 0`.
- **`vq_key_s` mask:** with left-shift + caps-lock held, `INTOUT[0]`
  has bit `0x02` set and bit `0x10` **clear**.

## Edge cases discovered in source

1. **`vqin_mode` returns `mode−1`** (0/1), contradicting the Atari-
   documented 1/2 — a deliberate TOS-compatible quirk EmuTOS keeps
   (`vdi_input.c:146-152`).
2. **`vq_color` flag 0 returns the *raw* `REQ_COL`** stored by
   `vs_color` before clamping — the read-back can be < 0 or > 1000
   (`vdi_col.c:875-892` vs `:687-700`).
3. **`vq_color` writes `INTOUT[1..3]=0` first**, so an out-of-range
   index yields `-1` with zeroed RGB, not stale data (`:851,857`).
4. **`v_get_pixel` returns two values** on a palette screen — the raw
   device pixel *and* the VDI pen — but only one (RGB) on truecolor,
   where `INTOUT[0]` is forced to 0 (`vdi_fill.c:1104-1114`).
6. **`v_get_pixel` has no bounds check** — an off-screen point reads the
   pixel its linear screen address lands on (`vdi_fill.c:671-690`,
   `vdi/vdi_inline.h:14-17`); see § "Coordinates outside the screen".
5. **`vq_key_s` masks to 4 bits** (`& 0x0f`), dropping caps-lock etc.,
   so it is **not** identical to a raw BIOS `Kbshift` (`vdi_input.c:187`).

## Dependencies

### Requires
- An open workstation. `vq_color`: `REQ_COL`/`req_col2` seeded by
  `init_colors`, `MAP_COL[]`, `numcolors`. `v_get_pixel`: the framebuffer
  read path + `REV_MAP_COL[]`. `vqin_mode`: the mode globals (input
  subsystem). `vq_key_s`: BIOS `Kbshift`.

### Used by
- Applications probing colours (`vq_color`), sampling the screen
  (`v_get_pixel`), polling input mode/keys (`vqin_mode`, `vq_key_s`).

## Related data structures

- [`../_state.md`](../_state.md) — the `REQ_COL`/`req_col2` palette-request
  state (`vq_color`) and the `loc_mode`/`val_mode`/`chc_mode`/`str_mode`
  input-mode globals (`vqin_mode`).
- [`../workstation/102-vq_extnd.md`](../workstation/102-vq_extnd.md) —
  device colour capability (`DEV_TAB[13]`=`numcolors`).

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the `vqin_mode`
  `mode−1` quirk, the `vq_color` raw-request read-back, and the
  `vq_key_s` `& 0x0f` mask against the documented ABI is **deferred**.

## NewTOS implementation notes

- `vq_color` "actual" path reads the per-device palette (ADR-011) and
  scales the DAC width to VDI's 0..1000; "requested" returns the stored
  `REQ_COL` verbatim (**including out-of-range components**). Bring-up
  screen is palette-based (truecolor `vq_color16` deferred).
- `v_get_pixel` uses the framebuffer pixel-read path; for a palette
  screen `INTOUT[0]` (device value) and the pen index coincide only after
  `REV_MAP_COL`, which NewTOS must round-trip with `MAP_COL`.
- `vqin_mode`/`vq_key_s` bridge to the input subsystem: keep the
  `mode−1` return and the `Kbshift & 0x0f` mask exactly. `gshift_s` maps
  to the same shift-state source as BIOS `Kbshift` (native keyboard
  driver, see [MEMORY: keyboard via Atari IKBD]).
