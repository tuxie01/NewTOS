# `vq_extnd` — VDI opcode 102 (extended workstation inquire)

> Phase 00b spec (full-contract template). Source: EmuTOS @ `1a7484ab` —
> `vdi/vdi_control.c:626` (`vdi_vq_extnd`), `:76-123` (`INQ_TAB_rom`),
> `:193` (`update_rez_dependent`, patches `INQ_TAB[4]/[5]`).
> Calling convention: [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 102; the `INTIN[0]` mode flag; mode-0 returns the
> same tables as `v_opnvwk`; mode-1 returns the current clip rect in
> `PTSOUT[0..3]` plus the 45-word `INQ_TAB` with the live clip flag at
> index 19. **Reference:** the `INQ_TAB` storage and the `flip_y` write.

Returns extended device information. The `INTIN[0]` flag selects between
re-returning the standard open-workstation tables (mode 0) and returning
the *extended* inquiry table plus the current clipping rectangle (mode 1).

## EmuTOS source

**Primary location:** `vdi/vdi_control.c:626` — `vdi_vq_extnd(Vwk *vwk)`
(reached via `jmptb2[2]`, `vdi_main.c:77`).

**Globals read:** `INTIN[0]` (mode), `SIZ_TAB`, `DEV_TAB`, `INQ_TAB`,
`vwk->xmn_clip/ymn_clip/xmx_clip/ymx_clip`, `vwk->clip`.
**Globals written:** `flip_y = 1`, `INQ_TAB[19]` (updated to the live
clip flag), the caller's `INTOUT`/`PTSOUT`.

## Source extraction summary

`vq_extnd` sets `flip_y = 1`. In **mode 0** (`INTIN[0] == 0`) it copies
the 12-word `SIZ_TAB` into `PTSOUT` and the 45-word `DEV_TAB` into
`INTOUT` — identical to what `v_opnvwk` returns. In **mode 1**
(`INTIN[0] != 0`) it writes the workstation's current clip rectangle into
`PTSOUT[0..3]` (`xmn,ymn,xmx,ymx`), zeroes `PTSOUT[4..11]`, updates
`INQ_TAB[19]` to the queried `Vwk`'s live `clip` flag, and copies the
45-word `INQ_TAB` into `INTOUT`. The dispatcher pre-loads `CONTRL[2]=6`,
`CONTRL[4]=45` (`vdi_main.c:77`).

## Trap interface

| | |
|--|--|
| Opcode | `CONTRL[0] = 102` |
| Handle | `CONTRL[6]` (resolved to the queried `Vwk`) |
| `# INTIN` (`CONTRL[3]`) | 1 — `INTIN[0]` = mode (0 = standard tables, ≠0 = extended) |
| `# INTOUT / # PTSOUT` | 45 / 6 |

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.

## Inputs

| Idx | Name | Meaning |
|-----|------|---------|
| `INTIN[0]` | mode | `0` → return `SIZ_TAB`/`DEV_TAB` (as `v_opnvwk`); `≠0` → return the clip rect + `INQ_TAB`. |

## Outputs

### Mode 0 (`INTIN[0] == 0`)

Identical to [`v_opnvwk`](100-v_opnvwk.md): `INTOUT[0..44]` = `DEV_TAB`
(45 WORDs), `PTSOUT[0..11]` = `SIZ_TAB` (12 WORDs). See that spec for the
full byte-index tables.

### Mode 1 (`INTIN[0] != 0`)

- `PTSOUT[0..3]` = the queried workstation's current clipping rectangle
  `xmn_clip, ymn_clip, xmx_clip, ymx_clip`; `PTSOUT[4..11]` = 0.
- `INTOUT[0..44]` = `INQ_TAB` (45 extended-capability values), with
  `INQ_TAB[19]` set to the queried `Vwk`'s live `clip` flag first.

### `INQ_TAB` (45 WORDs, extended capabilities)

ROM template `INQ_TAB_rom` (`vdi_control.c:76-123`); `[4]`/`[5]` patched
by `update_rez_dependent`:

| Idx | Meaning | ROM value | Patched |
|-----|---------|-----------|---------|
| 0 | alpha/graphic controller type | 4 | — |
| 1 | # background colours | 1 | — |
| 2 | text styles supported (bitmask) | `0x1F` | — |
| 3 | scale rasters (0=no) | 0 | — |
| 4 | # planes | 1 | **`v_planes`** |
| 5 | video lookup table | 0 | **0 if 16-plane or mono monitor, else 1** |
| 6 | performance factor | 50 | — |
| 7 | contour-fill capability | 1 | — |
| 8 | character-rotation capability | 1 | — |
| 9 | # writing modes | 4 | — |
| 10 | highest input mode | 2 | — |
| 11 | text alignment flag | 1 | — |
| 12 | inking capability | 0 | — |
| 13 | rubber banding | 0 | — |
| 14 | max vertices | `MAX_VERTICES` (1024; 512 on the small ROM targets) | — |
| 15 | max `INTIN` (−1 = unbounded) | −1 | — |
| 16 | # mouse buttons | 1 | — |
| 17 | styles for wide lines | 0 | — |
| 18 | writing modes for wide lines | 0 | — |
| 19 | **clip flag** (filled at query time) | 0 | **`vwk->clip`** |
| 20 | extended-precision pixel-size info | 0 | — |
| 21 | pixel width (1/10..1/1000 micron) | 0 | — |
| 22 | pixel height (1/10..1/1000 micron) | 0 | — |
| 23 | horizontal resolution (dpi) | 0 | — |
| 24 | vertical resolution (dpi) | 0 | — |
| 25–27 | reserved | 0 | — |
| 28 | bezier flag (bit 1) | 0 | — |
| 29 | reserved | 0 | — |
| 30 | raster flag (does `vro_cpyfm` scale?) | 0 | — |
| 31–39 | reserved | 0 | — |
| 40 | unprintable left border (px) | 0 | — |
| 41 | unprintable upper border (px) | 0 | — |
| 42 | unprintable right border (px) | 0 | — |
| 43 | unprintable lower border (px) | 0 | — |
| 44 | page size (printers) | 0 | — |

Indices 40–44 are printer/plotter-only (all 0 on the screen).

## Error conditions

None — VDI has no error return. (A bad `CONTRL[6]` is a no-op in the
dispatcher before `vq_extnd` runs.)

## Verification criteria

- **Mode 0 == v_opnvwk:** `vq_extnd(INTIN[0]=0)` returns `INTOUT`/`PTSOUT`
  byte-identical to a matching `v_opnvwk` open (same `DEV_TAB`/`SIZ_TAB`).
- **Mode 1 clip rect:** with the workstation's clip set to
  `(x1,y1,x2,y2)`, `vq_extnd(INTIN[0]=1)` returns
  `PTSOUT[0..3] = x1,y1,x2,y2` and `PTSOUT[4..11] = 0`.
- **Mode 1 clip flag:** `INTOUT[19]` equals the queried workstation's live
  `clip` flag (0 when clipping disabled, 1 when enabled) — **per-`Vwk`**,
  not a global.
- **Planes:** on a `p`-plane screen, `INTOUT[4] = p`.

## Edge cases discovered in source

1. **`INQ_TAB[19]` is a shared global patched per query** — `vq_extnd`
   writes the *queried* `Vwk`'s clip flag into the one `INQ_TAB[19]` slot
   before copying it out (`vdi_control.c:651`). Two workstations with
   different clip flags each read their own value because the write
   happens per call.
2. **Mode is any non-zero value**, not a specific `1` — `INTIN[0] != 0`
   selects extended mode (`:633`). EmuTOS never reads `CONTRL[5]` for
   opcode 102 (`vdi_main.c:77`). **NewTOS exception:** `CONTRL[5] == 1`
   together with `INTIN[0] == 2` is the EdDI call `vq_scrninfo`, not mode 1
   — see "NewTOS extension: `vq_scrninfo`" below.
3. **`flip_y` is forced to 1** at entry (`:631`), like `init_wk`.

## Dependencies

### Requires
- An open physical workstation (populates `DEV_TAB`/`SIZ_TAB`/`INQ_TAB`);
  a valid queried `Vwk` for the clip fields.

### Used by
- AES (`gsx_1code(EXTENDED_INQUIRE,1)`, `aes/gemgsxif.c:499`) and
  applications probing device capabilities.

## Related data structures

- [`_state.md`](../_state.md) — `DEV_TAB`/`SIZ_TAB`/`INQ_TAB` ownership.
- [`docs/api/structures/vwk.md`](../../structures/vwk.md) — the clip fields.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the `INQ_TAB`
  index meanings is **deferred**.

## NewTOS implementation notes (Phase 12)

- `INQ_TAB` is per **physical device** (ADR-011): each screen device
  reports its own planes/colours/resolution. `[14]` (max vertices) and
  `[15]` (max intin) bound the array sizes the handlers accept.
- Index 19 must reflect the **queried** `Vwk`'s clip flag, not a global.
- `INQ_TAB[4]`/`[5]`: **resolved 2026-09-04** — see the section below.
  NewTOS is a Falcon truecolor workstation, so `[4] = 16` and `[5] = 0`.

### NewTOS extension: `vq_scrninfo` (102, sub-opcode 1) — beyond EmuTOS

Spec: [`../eddi-vq_scrninfo.md`](../eddi-vq_scrninfo.md) (approved
2026-10-02). EmuTOS has no EdDI; NewTOS installs the `EdDI` cookie and answers
**`CONTRL[5] == 1` and `INTIN[0] == 2`** (both, E7) with the 273-word
`vq_scrninfo` table instead of `INQ_TAB`: `INTOUT[0..=272]`, `CONTRL[4] =
272`, `CONTRL[2] = 0`, PTSOUT untouched, `d0 = 0`. Every other opcode-102 call
— `CONTRL[5] == 0` with any mode (including 2), or `CONTRL[5] == 1` with any
`INTIN[0] != 2` — is `vq_extnd` exactly as specified above. Implementation:
`tos-api/src/vdi/eddi.rs`, chosen in `tos-api/src/vdi/mod.rs` `dispatch`;
tests `tos-api/tests/vdi_eddi_scrninfo.rs`. Recorded as a sanctioned
deviation in `docs/COMPATIBILITY.md` (E8): a program that issues this exact
call **without** checking the cookie got `INQ_TAB` (EmuTOS), and gets the
EdDI table on NewTOS.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`INQ_TAB` storage, `v_planes`) at the
> BCM2711 VideoCore framebuffer read through the `RasterBackend`. The
> **contract is unchanged** — opcode 102, the `INTIN[0]` mode flag, the
> mode-0 tables, the mode-1 clip rect + 45-word `INQ_TAB`. Peripheral +
> driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

> **ADR-017 cross-reference (2026-07-09, `docs/MEMORY.md`).** See
> [`100-v_opnvwk`'s Pi/BCM2711 binding note](100-v_opnvwk.md#pi--bcm2711-binding).
>
> **Correction (A86, 2026-09-21):** this blockquote previously said
> `INQ_TAB[4]/[5]` (4 planes / colour LUT) held for "today's staged
> 640×480×16-colour bring-up geometry" and derived from `num_colors()` — both
> stale, and already contradicted by the "resolved 2026-09-04" section just
> below. The live backend is a Falcon truecolor workstation:
> `INQ_TAB[4] = planes() = 16` (not `num_colors()`), `INQ_TAB[5] = 0`
> (`kernel/src/vdi_backend.rs:35-40`; `tos-api/src/vdi/workstation.rs:301,
> 306`). Resolution is unaffected either way — a larger/smaller negotiated
> framebuffer changes neither `[4]` nor `[5]` — so that part of the original
> point stands, just not via `num_colors()`.

### Open question resolved — `INQ_TAB[4]`/`[5]`/`[14]` (2026-09-04)

`INQ_TAB[4]` and `[5]` are not free values: EmuTOS derives both from
`v_planes` in `update_rez_dependent` (`vdi/vdi_control.c:210-213`), so the
NewTOS answer is whatever that formula gives on the NewTOS workstation.
NewTOS' screen is a **Falcon truecolor workstation** — 16 planes, 256
colours (`kernel/src/vdi_backend.rs`, **ADR-024** — `docs/MEMORY.md`,
recorded A101 2026-09-21, resolves this placeholder) — not the
640×480×16-colour bring-up screen an earlier revision of this section
assumed:

| Idx | Meaning | EmuTOS rule | NewTOS value | TT reference |
|-----|---------|-------------|--------------|--------------|
| 4 | # planes | `INQ_TAB[4] = v_planes` (`vdi_control.c:210`) | **16** | 4 |
| 5 | video lookup table | `(v_planes == 16 \|\| MON_MONO) ? 0 : 1` (`vdi_control.c:211-213`) | **0** | 1 |
| 14 | max vertices | `MAX_VERTICES` | **1024** | 1024 |
| 19 | clip flag | live `vwk->clip` (`vdi_control.c:651`) | live `vwk.clip` | — |
| 16 | # mouse buttons | ROM template | 1 | 1 |

Implementation: `tos-api/src/vdi/workstation.rs`, `vq_extnd` — `inq[4] =
planes`, `inq[5] = if planes == 16 { 0 } else { 1 }`, `inq[19] = vwk.clip`.
`[4]` and `[5]` are therefore *machine-dependent by EmuTOS' own definition*
and cannot be byte-compared against a four-plane reference machine; the
conformance harness excludes exactly those two slots by name and checks the
formula instead (`tos-api/tests/vdi_reference/harness.rs`,
`MACHINE_EXCLUDED`).

### `INQ_TAB[14]` — max vertices = 1024 (resolved 2026-09-04)

`MAX_VERTICES` is **1024** for NewTOS. `config.h` overrides it to 512 only
inside the `TARGET_192` / `TARGET_256` / `TARGET_CART` / `TARGET_FLOPPY`
blocks (`include/config.h:388-390,445-447,548-550,560-562`). The reference
profile — EmuTOS `make 512`, Falcon030/TOS4 — defines `TARGET_512`
(`Makefile:561`), but **`config.h` never tests `TARGET_512`**, so that build
takes the default `#define MAX_VERTICES 1024` (`include/config.h:1652-1655`,
*"TOS2 allows 512 vertices, TOS3/TOS4 allow 1024"*). The measured EmuTOS 1.4
reference on a TT reports 1024 in `INQ_TAB[14]`
(`tosdump/reference/EMUTOS14-VDI-r3.BIN`, cases 117/119).

NewTOS therefore reports **1024** and clamps `CONTRL[1]` at 1024
(`tos-api/src/vdi/primitives.rs`, `MAX_VERTS`). An earlier revision of this
section recorded 256 and argued it was "not a contract violation (the cap is
device-reported)" — that reasoning is withdrawn: the number a program reads
out of `INQ_TAB[14]` is the number of vertices it may pass, so reporting a
figure the reference profile does not report is a divergence, not a choice.

### Pixel-size / resolution `INQ_TAB` fields

`INQ_TAB[20..24]` (extended pixel-size / dpi) stay `0` in the ROM template
(`workstation.rs:53-55`) — NewTOS does not patch them, so pixel width/
height in microns and dpi are reported as "unknown" (0). Consistent with
`DEV_TAB[3]/[4]` staying at the ROM default (see
[`100-v_opnvwk`](100-v_opnvwk.md#pi--bcm2711-binding) mismatch 2); a
truthful value needs the mailbox physical-display-size query, not yet done.

### 1080p later

`INQ_TAB[4]/[5]` derive from `planes()` (16) and the truecolor test
(`planes == 16`), both fixed by the Falcon-truecolor VDI model
(`workstation.rs:301,306`), not the resolution; a larger or smaller
negotiated framebuffer does not change the plane/LUT answers.
**Correction (A86, 2026-09-21):** this note previously said the fields
derived from `num_colors` (16) "until a truecolor path is adopted" — the
truecolor VDI is already the live model (`vdi_backend.rs:33,40`,
`NUM_COLORS = 256` / `PLANES = 16`, distinct constants); see the "resolved
2026-09-04" section above.
