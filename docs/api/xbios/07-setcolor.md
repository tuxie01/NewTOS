# `Setcolor` (XBIOS 0x07)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1192` (`setcolor`) and
> `bios/screen.c:1015` (`atari_setcolor`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `colorNum & 0x0F` range, the `color < 0` query,
> the return of the previous (masked) colour, the ST `0x0777` / STe `0x0FFF`
> mask, and the boot palette a first query sees (section "Boot palette").
> Reference: where EmuTOS keeps the value (the ST(e) palette *hardware
> registers*, no software shadow) and the register write.

> **Correction 2026-11-12 (XBIOS21 spec step).** Earlier versions of this
> file (and `_state.md §4`) said `atari_setcolor` reads and writes a
> `palette[]` software shadow. The source has none: `palette` in
> `atari_setcolor` is a local pointer to the ST(e) palette *registers*
> (`bios/screen.c:1020`, `ST_PALETTE_REGS`), and both the read
> (`:1031`) and the write (`:1033`) go straight to them. What a first
> `Setcolor(n,-1)` returns is therefore whatever boot wrote into those
> registers (`initialise_ste_palette`, `bios/screen.c:64-71`).

## EmuTOS source

**Primary location:** `bios/screen.c:1192` —
`WORD setcolor(WORD colorNum, WORD color)`, slot 7 of `xbios_vecs[]`
(`bios/xbios.c:1293`); delegates to `atari_setcolor` (`:1015-1036`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/screen.c:1015` — `atari_setcolor()` — the masked read-modify-write.

**Headers / type definitions used:**
- `bios/screen.h` — `RGB_*` colour constants; the palette-register base.

**System variables / globals read:**
- the ST(e) palette register `ST_PALETTE_REGS[colorNum]` (`bios/screen.c:1020,1031`)
  — hardware, not a shadow.
- the video type (`HAS_VIDEL || HAS_TT_SHIFTER || HAS_STE_SHIFTER`, to pick
  the `0x0FFF` vs `0x0777` mask, `bios/screen.c:1026-1029`).

**System variables / globals written:**
- the ST(e) palette register `ST_PALETTE_REGS[colorNum]` when `color >= 0`
  (`bios/screen.c:1033`), the unmasked word. Nothing else: no Falcon
  palette register, no `ste_shadow_palette`/`falcon_shadow_palette`.

**Build configuration affecting this function:**
- `CONF_WITH_ATARI_VIDEO` — the register write; without it the shadow-only
  path applies.

---

## Source extraction summary

`Setcolor` sets one hardware palette entry and returns its **previous** value.
`colorNum` is masked to `& 0x000F` (`bios/screen.c:1024`), so only the 16
ST/STe entries are addressable. It reads the old value masked with `0x0777`
on a plain ST or `0x0FFF` on STe/TT/Videl (`:1027-1031`) — i.e. 3 bits per
channel on ST, 4 bits on STe.

If `color >= 0` it writes the new value into the register `ST_PALETTE_REGS[colorNum]`
(`:1033`); if `color < 0` it does **not** write — this is the query
form, returning the current colour without changing it (`:1032-1033`). Either
way it returns the old (masked) value.

The packed word format is bits 11-8 red, 7-4 green, 3-0 blue (4 bits per
channel, only 3 significant on plain ST). Real STe/TT hardware rotates each
channel's LSB to the top bit for ST backward compatibility; EmuTOS stores and
masks straight nibbles, so the *stored/returned* word is contract and the
hardware nibble rotation is a driver detail.

---

## Boot palette — what a first `Setcolor(n,-1)` returns (XBIOS21, 2026-11-12)

Read from the source this session; the two measured values are from
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT` (cases bios-r1 93, 94;
`tosdiff-EMUTOS-1a7484ab-BIOS.txt` and `...-BIOS-falcontc.txt` in
`measurements/measurement-2026-11-12-xbios-full/runs/bios-r1/`).

**Source of the values.** `screen_init` ends with
`initialise_palette_registers(rez, boot_resolution)` (`bios/screen.c:616`; the
non-Videl build passes `0` as mode, `:618`). That routine (`bios/screen.c:432-457`)
first writes the 16 ST(e) registers from the constant table `dflt_palette[]`
(`bios/screen.c:54-59`) through `initialise_ste_palette(mask)` (`:64-71`,
`col_regs[i] = dflt_palette[i] & mask`) with `mask = 0x0FFF` if
`HAS_VIDEL || HAS_TT_SHIFTER || HAS_STE_SHIFTER`, else `0x0777` (`:437-440`),
then adds the Videl/TT palette (`:445-453`), then `fixup_ste_palette(rez)` (`:456`).

`dflt_palette[]` (register index = `colorNum`; constants `bios/screen.h:56-71`):

| n | name | word | n | name | word |
|--:|------|-----:|--:|------|-----:|
| 0 | `RGB_WHITE`   | `0x0FFF` | 8  | `RGB_GRAY`      | `0x0333` |
| 1 | `RGB_RED`     | `0x0F00` | 9  | `RGB_LTRED`     | `0x0F33` |
| 2 | `RGB_GREEN`   | `0x00F0` | 10 | `RGB_LTGREEN`   | `0x03F3` |
| 3 | `RGB_YELLOW`  | `0x0FF0` | 11 | `RGB_LTYELLOW`  | `0x0FF3` |
| 4 | `RGB_BLUE`    | `0x000F` | 12 | `RGB_LTBLUE`    | `0x033F` |
| 5 | `RGB_MAGENTA` | `0x0F0F` | 13 | `RGB_LTMAGENTA` | `0x0F3F` |
| 6 | `RGB_CYAN`    | `0x00FF` | 14 | `RGB_LTCYAN`    | `0x03FF` |
| 7 | `RGB_LTGRAY`  | `0x0555` | 15 | `RGB_BLACK`     | `0x0000` |

(Order of the initialiser, `bios/screen.c:54-59`; values `bios/screen.h:56-71`.)
This is the **hardware-register order** (register 1 red, register 15 black),
not the VDI pen order (`MAP_COL_ROM = { 0,15,1,2,4,6,3,5,7,8,9,10,12,14,11,13 }`,
`vdi/vdi_col.c:28-29`: pen 1 black -> register 15).

**Profile dependence.**
- `mask`: `0x0FFF` on every profile that has Videl, TT shifter or STe shifter
  (the oracle's `tt` and `falcontc` profiles: both rows 93/94 measured `0x0FFF`/`0x0F00`);
  `0x0777` only on a plain ST (values would be `0x0777`, `0x0700`, ... i.e. the table
  `& 0x0777`; inferred from `:437-440`, no oracle profile for it).
- `fixup_ste_palette(rez)` (`bios/screen.c:76-84`): `rez == ST_MEDIUM` copies
  register 15 to 3; `rez == ST_HIGH` copies 15 to 1 (so `Setcolor(1,-1)` would return
  `0x0000` on a mono boot). Not triggered on the measured profiles (the TT boot
  rez is `TT_MEDIUM`/`TT_HIGH`, `:564`; the Videl boot rez is `sshiftmod` after `vsetmode`, `:553-557`,
  `FALCON_REZ` for the default `0x1C`-class mode).
- Falcon/Videl: `initialise_falcon_palette(mode)` (`bios/videl.c:998-1042`) rewrites
  the 16 ST registers (`:1037-1041`) **only if** `use_ste_palette(mode)` (`:778-792`:
  mono monitor, any 2bpp mode, ST-compatible 4bpp). In a truecolor/8bpp mode on a
  colour monitor it does not, so the registers keep `dflt_palette & mask`. In the
  ST-compatible case the values change for registers 8-14 (comment `:1033-1036`:
  "for compatibility with Atari TOS") — **not measured**; needs the oracle if a
  NewTOS profile ever boots an ST-compatible mode. Default boot mode is
  `FALCON_DEFAULT_BOOT` (`bios/videl.h:56`) unless NVRAM or the monitor fix-up
  changes it (`bios/screen.c:491-545`).
- TT shifter: `initialise_tt_palette(rez)` (`bios/screen.c:370-387`) fills the 256 TT
  registers; it does not touch the ST(e) registers `Setcolor` reads.

**Do `Setpalette` and `VgetRGB` read/write the same store?**
- `Setpalette` and `Setcolor`: yes, the ST(e) palette registers. `Setpalette` only
  stores a pointer in `colorptr` (`bios/screen.c:1182`); the VBL handler copies 16 words
  to `0xFFFF8240` (`bios/vectors.S:358-383`, the `docopy` path with bit 0 of `colorptr`
  clear). `Setcolor` writes the register directly, so a `Setcolor(n,-1)` right after
  `Setpalette` still returns the *old* value until the next VBL has run (inferred
  from the two code paths, not measured).
- `VgetRGB`: **no**. It reads `ste_shadow_palette[]` (if `use_ste_palette(mode)`) or
  `falcon_shadow_palette[]` (`bios/videl.c:862-909`) — two software tables that
  `Setcolor`/`Setpalette` never write. After boot in the measured falcontc mode it
  returns the Falcon default palette (`videl_dflt_palette`, `bios/videl.c:42-107`), e.g.
  entry 1 `FRGB_RED` -> `0x00FF0000`; the 16 captured longs of case 214 match
  (`capture[0..3] = 00 FF FF FF` = white, `[4..7] = 00 FF 00 00` = red). So **EmuTOS
  holds up to three unrelated palettes at boot** (ST(e) registers, `falcon_shadow_palette`,
  `ste_shadow_palette`), and only `VsetRGB` (`:806-859`) keeps the Falcon/STe *shadow*
  coherent with the Videl registers; `Setcolor` and `Setpalette` do not touch them.
- The VDI does not use `Setcolor` on Videl: `set_color` calls `VsetRGB`
  (`vdi/vdi_col.c:540-545`) and `vq_color` calls `VgetRGB` (`:900-908`); in a truecolor
  mode (`CONF_WITH_VDI_16BIT`, ADR-024) `vs_color` only writes the per-workstation software
  palette (`set_color16`, `vdi/vdi_col.c:601-614`, called from `:648-662`) and touches no
  XBIOS palette at all. The VDI reaches `Setcolor` only on the non-Videl STe/ST paths
  (`:576-582`, `:920-931`).

**Measured vs. read.** Measured: `Setcolor(0,-1) = 0x0FFF`, `Setcolor(1,-1) = 0x0F00`
(tt and falcontc, cases 93/94); case 95 `Setcolor(15,-1)` is EQUAL for NewTOS's `0`
(black, matches table entry 15). Read, not measured: entries 2-14 and the
`0x0777` / mono / ST-compatible-Videl variants.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x07
- **Stack frame on entry (m68k):** `Setcolor(colorNum, color)` =
  `xbios_w_ww(7, a, b)` (`include/xbiosbind.h:26`):
  ```
  +4: color    (word)
  +2: colorNum (word)
   0: function_id (word) = 0x0007
  ```
- **Total parameter bytes:** 6 (W + W).
- **Native (AArch64) ABI:** ID `0x07` in `x8`; args in `x0..x1`; the WORD
  result **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name       | Type / size |
  |------------|-------------|------------|-------------|
  | +2         | `x0`        | `colorNum` | `WORD` (16) — masked `& 0x0F` |
  | +4         | `x1`        | `color`    | `WORD` (16) — `< 0` = query |

---

## Inputs

| Position | Name       | Type   | Size | Source location      | Notes |
|----------|------------|--------|------|----------------------|-------|
| arg0     | `colorNum` | `WORD` | 2    | `bios/screen.c:1024` | palette index, masked to `0..15` |
| arg1     | `color`    | `WORD` | 2    | `bios/screen.c:1032` | new packed colour; `< 0` = query only |

### Packed colour word

| Bits  | Channel | ST (plain) | STe/TT/Videl |
|-------|---------|------------|--------------|
| 11-8  | Red     | 3 sig. bits | 4 bits |
| 7-4   | Green   | 3 sig. bits | 4 bits |
| 3-0   | Blue    | 3 sig. bits | 4 bits |

Mask applied to the *returned* value: `0x0777` (ST) / `0x0FFF` (STe/TT/Videl).

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** WORD
- **Encoding:** the **previous** value of `palette[colorNum]`, masked
  (`0x0777`/`0x0FFF`).
- **Source location:** `bios/screen.c:1031,1035`.

### Side effects on global state

- none in software; the only state is the palette register written when
  `color >= 0` (`bios/screen.c:1033`).

### Side effects on hardware

- Writes the corresponding hardware palette register (Atari video). No write
  when `color < 0`.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Setcolor` has no error path; an out-of-range `colorNum` is silently masked,
not rejected.

---

## Verification criteria

- **Set + read old:** `Setcolor(1, 0x0700)` returns the previous entry-1
  colour and makes `Setcolor(1, -1)` subsequently return `0x0700 &
  mask`.
- **Query does not write:** `Setcolor(2, -1)` returns entry 2 unchanged and
  leaves the palette untouched.
- **Index masking:** `Setcolor(16, c)` writes entry `16 & 0x0F = 0`, not a
  17th entry.
- **ST mask:** on a plain-ST profile, the returned value has bits 0/4/8
  masked to the `0x0777` pattern.

---

## Edge cases discovered in source

1. **`color < 0` is the query form.** No write occurs; only the old colour is
   returned (`bios/screen.c:1032`). This is how a program reads a single
   palette entry.
2. **`colorNum & 0x0F` — silent wraparound.** Out-of-range indices wrap into
   0..15 rather than erroring (`:1024`).
3. **Return is masked, write is raw.** The old value is masked with
   `0x0777`/`0x0FFF` (`:1031`); the new value is stored unmasked (`:1033`),
   so a later query re-masks it. NewTOS must mask the returned value the same
   way.
4. **STe LSB rotation is a hardware detail.** EmuTOS stores straight nibbles;
   the ST-vs-STe intensity-bit order lives in the hardware, not the stored
   word. NewTOS's 4-bit→8-bit VideoCore mapping is free as long as the
   returned word matches.

---

## Dependencies

### Requires (must be initialized / functional)

- The ST(e) palette registers, loaded at boot by `initialise_palette_registers`
  (`bios/screen.c:432-457`, called from `screen_init` at `:616`/`:618`).

### Used by

- Palette animation, single-colour tweaks, VDI `vs_color`, demos.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — palette word format (its "palette shadow"
  wording is corrected by this file's note above; the `palette[]` shadow does
  not exist in EmuTOS).
- `docs/api/xbios/06-setpalette.md` — full-block counterpart.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `colorNum`
  0..15, `color = -1` query, return of the old colour.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- ST returns `& 0x0777` (3-bit channels); STe/TT/Videl `& 0x0FFF` (4-bit).
  The mask depends on the detected video, not a build flag alone.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Setcolor` writes one entry of the VideoCore palette (4-bit→8-bit
expansion) and returns the previous masked word; the `color < 0` query and
the `colorNum & 0x0F` masking are preserved.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the ST(e) palette registers) at the NewTOS
> palette shadow. The **contract is unchanged** — `colorNum & 0x0F`, `color < 0`
> query, the previous masked word returned — and includes the boot palette of the
> section above.

- **Owner decision 2026-11-12 (`TASK/XBIOS21.md`).** (1) Option A: the shadow leads,
  seeded at boot with `dflt_palette & 0x0FFF`; (2) an XBIOS palette write
  (`Setcolor`/`Setpalette`) does **not** reach the VDI colour table in the truecolor
  VDI; (3) no `0x0777` plain-ST mask, NewTOS is STe-class `0x0FFF`.
- **What `setcolor` does (XBIOS21).** The index is masked `& 0x0F`; the old word is read
  from the 16-entry shadow `PAL` (`tos-api/src/xbios/screen.rs:396@palette`) and
  returned `& 0x0FFF` as a WORD; when `color >= 0` the entry is stored `& 0x0FFF`
  (`tos-api/src/xbios/screen.rs:332@setcolor`). `color < 0` writes nothing. Storing the
  masked word instead of EmuTOS's raw one is unobservable: the only read is the masked
  return.
- **Boot state.** `PAL`'s initial value is `DFLT_PALETTE`
  (`tos-api/src/xbios/screen.rs:356@DFLT_PALETTE`), the table of section "Boot palette"
  in register order, loaded with the kernel image — the counterpart of EmuTOS's
  `initialise_ste_palette` at `screen_init`. No ST-medium/ST-high fix-up
  (`fixup_ste_palette`) and no ST-compatible Videl rewrite apply: NewTOS boots neither
  rez.
- **Not the VDI colour table.** `setcolor` and `setpalette` do not call the raster
  backend: the VDI pens (ADR-024, `kernel/src/vdi_backend.rs:56@VIDEL_PALETTE1`) are
  independent of the XBIOS palette, as on EmuTOS's Falcon/truecolor profile
  (`vdi/vdi_col.c:601-614,648-662`). The pre-XBIOS21 `apply_to_clut`, which pushed
  register `n` into VDI pen `n`, is gone; with it the register/pen order mismatch of the
  identity mapping. Consequence: a program that recolours the screen through
  `Setcolor`/`Setpalette` shows no change on screen, which is what a Falcon truecolor
  EmuTOS shows; a "save with `Setcolor(n,-1)`, restore with `Setcolor(n,saved)`" round
  trip changes nothing visible either.
- **Mask.** `0x0FFF` (STe class) on store and return, as measured on both oracle profiles.
- **Host tests** (`tos-api/tests/xbios21_boot_palette.rs`): the default word for every n
  after boot, set-then-query, a query does not write, `Setpalette` feeds `Setcolor`,
  index wrap `& 0x0F`, any negative colour queries, no write reaches the backend's colour
  table.
- **1080p.** Geometry-independent; unchanged.
