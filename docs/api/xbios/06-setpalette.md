# `Setpalette` (XBIOS 0x06)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1168` (`setpalette`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the 16-word palette pointer input, the
> apply-at-next-VBL timing, the packed palette word. Reference: `colorptr`
> and the VBL handler that consumes it.

## EmuTOS source

**Primary location:** `bios/screen.c:1168` —
`void setpalette(const UWORD *palettePtr)`, slot 6 of `xbios_vecs[]`
(`bios/xbios.c:1292`).

**Installed handler / vector:** (none) — direct table entry. The actual
palette load is done later by the VBL handler.

**Helpers called:** none directly (defers to the VBL).

**Headers / type definitions used:**
- `include/tosvars.h` — `colorptr` (the deferred-palette pointer sysvar).

**System variables / globals read:** none.

**System variables / globals written:**
- `colorptr` (`0x45A`) — set to `palettePtr` (`bios/screen.c:1182`).

**Build configuration affecting this function:** none directly. The VBL
consumer differs by video type (`bios/videl.c:796-805` for Videl; the STe
path loads `ST_PALETTE_REGS`).

---

## Source extraction summary

`Setpalette` loads all 16 hardware colour registers from the 16 words at
`palettePtr`. It does **not** write the registers itself: it stores
`palettePtr` in the OS variable `colorptr` and returns immediately. The next
vertical-blank interrupt notices `colorptr != 0`, copies the 16 words into
the palette registers, and clears `colorptr`. This means the palette change
takes effect at the start of the next frame (avoiding mid-frame tearing) and
is deferred, not immediate.

`palettePtr` must be word-aligned (the hardware loads 16-bit values). Each
word uses the ST/STe packed format (bits 11-8 red, 7-4 green, 3-0 blue). The
function returns nothing.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x06
- **Stack frame on entry (m68k):** `Setpalette(palettePtr)` =
  `xbios_v_l(6, a)` (`include/xbiosbind.h:25`):
  ```
  +2: palettePtr (LONG, pointer)
   0: function_id (word) = 0x0006
  ```
- **Total parameter bytes:** 6 (L).
- **Native (AArch64) ABI:** ID `0x06` in `x8`; arg in `x0`; **`void`** — no
  return value (the palette is delivered as a side effect at the next VBL).

  | m68k stack | AArch64 reg | Name         | Type / size |
  |------------|-------------|--------------|-------------|
  | +2         | `x0`        | `palettePtr` | `const UWORD *` (ptr) |

---

## Inputs

| Position | Name         | Type      | Size | Source location      | Notes |
|----------|--------------|-----------|------|----------------------|-------|
| arg0     | `palettePtr` | `UWORD *` | 4    | `bios/screen.c:1182` | word-aligned; 16 packed colour words |

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Encoding:** —.
- **Source location:** `bios/screen.c:1168` (declared `void`).

### Side effects on global state

- `colorptr = palettePtr` (`bios/screen.c:1182`).

### Side effects on hardware

- **Deferred:** at the next VBL, the 16 words are written into the palette
  registers and `colorptr` is cleared (`bios/videl.c:796-805` / the STe VBL
  path). No hardware write happens inside `Setpalette` itself.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Setpalette` has no error path.

---

## Verification criteria

- **Deferred apply:** after `Setpalette(pal16)`, the palette registers match
  `pal16[0..15]` no later than the next VBL (not necessarily on return).
- **`colorptr` set:** immediately after the call, `colorptr == pal16`.
- **Full 16 entries:** all 16 registers are loaded (not a subset).
- **Word format:** entry `n` uses bits 11-8 R / 7-4 G / 3-0 B; a program that
  reads back via `Setcolor(n, -1)` sees the (masked) value it wrote.

---

## Edge cases discovered in source

1. **Deferred to the VBL — not immediate.** The registers are not written by
   `Setpalette`; the VBL consumes `colorptr` (`bios/videl.c:796-805`). A
   program that reads the palette immediately after the call may still see
   the old values until a frame elapses.
2. **`colorptr` low-bit is a Videl selector.** On Videl the VBL treats
   `colorptr | 0x01` as "load Falcon regs" and `colorptr == 0x01` as "load
   256 shadow regs"; the plain even pointer from `Setpalette` selects the STe
   16-register load. NewTOS on the framebuffer collapses this to a 16-entry
   VideoCore palette load.
3. **Word alignment required.** The hardware reads 16-bit values; an unaligned
   `palettePtr` is a caller bug (undefined on Atari; NewTOS should treat it
   as such).

---

## Dependencies

### Requires (must be initialized / functional)

- The VBL handler (consumes `colorptr`), the palette registers / shadow.

### Used by

- Fade effects, palette animation, VDI colour setup, demos.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — palette shadow + `colorptr`.
- `docs/api/xbios/07-setcolor.md` — single-entry counterpart + the packed
  word format.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the
  16-word block + "applied at next VBL".)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The VBL consumer differs ST/STe vs Videl; the ST/STe 16-register path is
  the relevant one for the NewTOS profile.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Setpalette` records the 16 words and schedules a VideoCore
`set_palette` for the next present; the deferred-until-VBL timing is
preserved so palette-animation programs stay tear-free.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the STe/Videl 16 palette registers +
> the `colorptr`/VBL deferred load) at the NewTOS 16-entry ST(e) palette
> shadow. The **contract is unchanged** — a 16-word ST-packed palette pointer
> input, `void` return.

- **What `setpalette` does (XBIOS21, owner decision 2026-11-12).** A null
  pointer is ignored. Otherwise the 16 caller words are read in the caller's
  space first (a fault ends the call with the palette untouched), then stored
  `& 0x0FFF` into the same shadow `Setcolor` reads and writes
  (`tos-api/src/xbios/screen.rs:304@setpalette`; the shadow and its boot
  state: `docs/api/xbios/07-setcolor.md`, Pi binding). EmuTOS shares that store
  too (the ST(e) registers, `bios/vectors.S:358-383`).
- **Immediate, not at the next VBL.** NewTOS has no ST(e) palette that is
  scanned out, so there is no tearing to avoid; `colorptr` is not written.
  Unlike EmuTOS, a `Setcolor(n,-1)` right after `Setpalette` already returns
  the new word (EmuTOS: the old one until the VBL has run; inferred from the
  code, not measured).
- **No VDI colour-table write.** The framebuffer is 32 bpp
  (`kernel/src/drivers/bcm2711/framebuffer.rs:34@DEPTH_BPP`) and the VDI is the truecolor one
  (ADR-024); its pens are not the XBIOS palette, as on EmuTOS's Falcon/truecolor
  profile, where `vs_color` touches no XBIOS palette (`vdi/vdi_col.c:601-614`).
  So `Setpalette` changes no displayed colour (owner decision 2, `TASK/XBIOS21.md`).
- **Datasheet.** No peripheral register is involved.
- **1080p.** Unaffected by resolution.
