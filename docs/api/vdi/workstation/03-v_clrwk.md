# `v_clrwk` — VDI opcode 3 (clear workstation)

> Phase 00b spec. Source: EmuTOS @ `1a7484ab` —
> `vdi/vdi_control.c:599` (`vdi_v_clrwk`). Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 3; clears the *entire* screen; no inputs/outputs;
> ignores the clip rectangle; under truecolor every screen word becomes
> **`0xFFFF`** — observable through `v_get_pixel`, `vro_cpyfm` and
> `v_contourfill`, so the fill byte is part of the contract (V11/V12,
> 2026-09-21). **Reference:** the raw `memset(v_bas_ad, …)` of the Shifter/
> VIDEL framebuffer — NewTOS fills its own 32-bpp store with the device
> representation of that word instead.

Clears the entire screen surface to the background colour.

## EmuTOS source

**Primary location:** `vdi/vdi_control.c:599` — `vdi_v_clrwk(Vwk *vwk)`
(reached via `jmptb1[2]`, `vdi_main.c:34`).

**Globals read:** `v_lin_wr` (bytes per scanline), `V_REZ_VT` (vertical
resolution), `v_bas_ad` (screen base). **Build config:**
`CONF_WITH_VDI_16BIT` selects the fill byte.

## Source extraction summary

`v_clrwk` computes the framebuffer byte size as `v_lin_wr * V_REZ_VT` and
`memset`s the whole screen from `v_bas_ad`. The fill byte is **`0x00`**
for palette modes (colour index 0) and — **only when
`CONF_WITH_VDI_16BIT` is built and the mode is TrueColor** — `0xff` for
the 16-bit virtual palette. It clears the **full screen**, ignoring the
workstation's clipping rectangle.

## Trap interface

| | |
|--|--|
| Opcode | `CONTRL[0] = 3` |
| Handle | `CONTRL[6]` (resolved, but the whole screen is cleared) |
| `# INTIN / PTSIN / INTOUT / PTSOUT` | 0 / 0 / 0 / 0 |

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.

## Inputs / Outputs / Errors

No inputs, no outputs, no return value, no error path — always succeeds.

## Verification criteria

- After `v_clrwk`, every pixel of the framebuffer holds the fill word:
  colour index 0 in a palette mode, the raw word `0xFFFF` under truecolor
  (`v_get_pixel` answers `INTOUT[0] = 0, INTOUT[1] = 0xFFFF` at every
  coordinate — measured, `tosdump/reference/EMUTOS14-VDI-all-falcontc.BIN`
  cases 54/160/161; `tos-api/tests/vdi_reference_overlay_bit.rs`).
- The clip rectangle has **no** effect: a workstation with a small clip
  still clears the whole screen.

## Edge cases discovered in source

1. **Clip is ignored** — EmuTOS clears the full screen, not the clip
   region (`vdi_control.c:604-618`).
2. **Fill byte is `0x00` for palette modes** (index 0), `0xff` only in a
   `CONF_WITH_VDI_16BIT` TrueColor build.

## Dependencies

- Requires an open physical workstation (framebuffer base + geometry).
- `v_updwk` (opcode 4, "update workstation") is a **no-op** (`vdi_v_nop`,
  `vdi_main.c:36`) on the immediate-mode screen driver — no separate spec.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check deferred.

## NewTOS implementation notes

- Clears the whole `hal::framebuffer` via the `RasterBackend`; ignores the
  clipping rectangle (matches EmuTOS).
- **Correction (V11/V12, 2026-09-21).** This section used to call the
  `0x00`/`0xff` fill-byte distinction "a reference detail" and claim NewTOS
  "writes background colour index 0 regardless". Both were wrong: the fill
  word is **observable**. Under `TRUECOLOR_MODE` EmuTOS fills with `0xff`
  (`vdi/vdi_control.c:608-618`), so every screen word is `0xFFFF`;
  `pixelread` returns that word **unmasked** (`vdi/vdi_fill.c:671-681`) and
  `vdi_v_get_pixel` reports `INTOUT[0] = 0, INTOUT[1] = 0xFFFF`
  (`vdi_fill.c:1095-1110`). Pen 0 is *not* that word: `set_color16` packs
  `(31<<11)|(31<<6)|31 = 0xFFDF` (`vdi/vdi_col.c:601-613`), and the two
  differ by exactly `OVERLAY_BIT` (`0x0020`, `vdi/vdi_fill.c:26`), which the
  truecolor seed fill masks out of the screen side only
  (`vdi_fill.c:702-709,726-733,755`). Measured on a Hatari Falcon in VIDEL
  truecolor: `0xFFFF` after `v_clrwk` (`EMUTOS14-VDI-all-falcontc.BIN`,
  cases 54/160/161). NewTOS therefore clears **through the word, not
  through a pen**: `v_clrwk` calls `RasterBackend::clear_screen_word(0xFFFF)`
  under `planes() > 8` and `fill_rect(…, PEL 0, …)` only in a palette mode
  (`tos-api/src/vdi/workstation.rs` `v_clrwk`).
- **The NewTOS device-pixel rule.** The 16-bit screen word round-trips
  **exactly** through the 32-bpp framebuffer, overlay bit included: each
  5-bit gun is replicated into its 8-bit byte and `OVERLAY_BIT` is
  replicated into the X byte (`0xFF` set / `0x00` clear), so `0xFFFF`
  (cleared) is the pixel `0xFF_FFFFFF` and `0xFFDF` (pen 0) the pixel
  `0x00_FFFFFF` — the same colour on the display, two different words to
  `v_get_pixel`, `vro_cpyfm` Screen→Memory and `v_contourfill`
  (`kernel/src/vdi_backend.rs` `word_to_device`/`device_to_word`;
  [`../raster/_raster.md`](../raster/_raster.md) § "The VDI device pixel").

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the raw `memset(v_bas_ad, …)` of the
> Shifter framebuffer) at a `RasterBackend` full-screen fill over the
> BCM2711 VideoCore buffer. The **contract is unchanged** — clears the
> whole screen to the fill word (`0xFFFF` under truecolor, PEL 0 in a
> palette mode), ignores the clip rect, always succeeds. Peripheral +
> driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

- **How it clears (corrected V11/V12, 2026-09-21).** Under the truecolor
  profile `v_clrwk` hands the raw word `0xFFFF` to
  `RasterBackend::clear_screen_word` (`tos-api/src/vdi/workstation.rs`
  `v_clrwk`), which the kernel backend turns into one bulk
  `Framebuffer::clear(word_to_device(0xFFFF))` over the full
  `width × height` — the live framebuffer geometry — clip ignored
  (`kernel/src/vdi_backend.rs` `clear_screen_word`). The earlier text here
  ("fills … with PEL 0 via `fill_rect`", "the `0x00`/`0xff` fill-byte
  choice is irrelevant") described a superseded implementation and a wrong
  reading of the contract; see the implementation notes above.
- **White on the display, `0xFFFF` to the API.** The cleared pixel is
  `0xFF_FFFFFF`: white (the display ignores the X byte), and the overlay
  bit set in the X byte so that `v_get_pixel` reads `0xFFFF` back, as a
  Falcon does. A later pen-0 fill writes `0x00_FFFFFF` — the same white,
  read back as `0xFFDF`.
- **Cache clean for scan-out.** The backend cleans the filled rectangle to
  PoC after writing (`vdi_backend.rs:118-125`, `:201`) so the VideoCore
  display DMA sees the cleared pixels on real hardware (the framebuffer is
  mapped Non-Cacheable on rpi4, making this a no-op there;
  cite stale 2026-10-26: kernel/src/drivers/bcm2711/framebuffer.rs:183-187).

- **1080p later.** Clears whatever geometry the `RasterBackend` reports;
  no dependency on the fixed 640×480 mode.
