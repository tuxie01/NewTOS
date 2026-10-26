# `ICONBLK` — icon object payload (and the mono head of `CICONBLK`)

> Phase 00 spec (AES Object/Form batch). Source: EmuTOS @ `1a7484ab` —
> `include/obdefs.h:185-217`. Consumed by `just_draw` → `gr_gicon`
> (`aes/gemoblib.c:776-791`, `aes/gemgraf.c`). Pointed to by `ob_spec` for
> `G_ICON`; a `CICONBLK` (`G_CICON`) **begins with an `ICONBLK`**
> (`obdefs.h:213-217`, `gemoblib.c:780`).
>
> **Contract vs. reference** — the field layout and the mask/data/char/text
> geometry are **contract**. Colour-icon planes (`CICON`) are gated behind
> `CONF_WITH_COLOUR_ICONS` (default 1 in EmuTOS `1a7484ab`, `include/config.h:1267`).

## Purpose

An `ICONBLK` describes a monochrome icon: a **mask** bitmap and a **data**
bitmap (same dimensions), a single character drawn on the icon, and two
rectangles — one for the icon graphic, one for the icon's text label. The
mask clears the destination, then the data is drawn through it, so the icon
composites correctly over any background.

## Binary layout (m68k: 34 bytes)

```c
typedef struct {
/* 0x00 */ WORD *ib_pmask;  /* -> mask bitmap                        */
/* 0x04 */ WORD *ib_pdata;  /* -> data bitmap (same w/h as mask)     */
/* 0x08 */ char *ib_ptext;  /* -> icon text label                    */
/* 0x0C */ WORD  ib_char;   /* char + colours (see note)             */
/* 0x0E */ WORD  ib_xchar;  /* char x within icon                    */
/* 0x10 */ WORD  ib_ychar;  /* char y within icon                    */
/* 0x12 */ WORD  ib_xicon;  /* icon graphic rect x (rel. to object)  */
/* 0x14 */ WORD  ib_yicon;  /* icon graphic rect y                   */
/* 0x16 */ WORD  ib_wicon;  /* icon graphic rect width               */
/* 0x18 */ WORD  ib_hicon;  /* icon graphic rect height              */
/* 0x1A */ WORD  ib_xtext;  /* text rect x                           */
/* 0x1C */ WORD  ib_ytext;  /* text rect y                           */
/* 0x1E */ WORD  ib_wtext;  /* text rect width                       */
/* 0x20 */ WORD  ib_htext;  /* text rect height                      */
/* 0x22 — end (34 bytes) */
} ICONBLK;
```

> **Native (AArch64) variant.** The three leading pointers are 64-bit; WORD
> fields keep order/meaning (COMPATIBILITY "Native vs m68k struct variants").
> Native offsets (source: `tos-types/src/aes.rs:537-551` assertions,
> `repr(C)` struct `:92-128`): **48 bytes**, 8-byte aligned; `ib_pmask` 0x00,
> `ib_pdata` 0x08, `ib_ptext` 0x10, `ib_char` 0x18, `ib_xchar` 0x1A,
> `ib_ychar` 0x1C, `ib_xicon` 0x1E, `ib_yicon` 0x20, `ib_wicon` 0x22,
> `ib_hicon` 0x24, `ib_xtext` 0x26, `ib_ytext` 0x28, `ib_wtext` 0x2A,
> `ib_htext` 0x2C, then 2 bytes tail padding to 48. The m68k table above agrees
> with `include/obdefs.h:185-201` (34 bytes). (SPEC01 cross-check 2026-10-16:
> no earlier native number in this file; added.)
>
> `ib_char` packs the drawn character in the low byte and foreground/
> background colour indices in the high nibbles (per the icon-draw
> convention); a NewTOS renderer decodes it in `gr_gicon`.

## Positioning and the SELECTED swap

`just_draw` copies the `ICONBLK`, then **adds the object's absolute screen
x/y** to `ib_xicon`/`ib_yicon`/`ib_xtext`/`ib_ytext` before calling
`gr_gicon(state, &ib, cicon)` (`gemoblib.c:784-789`). So the four rect
offsets in the struct are **relative to the object's top-left**, not
absolute.

- **SELECTED foreground/background swap.** `gr_gicon` renders the icon
  inverted when `state & SELECTED`. After drawing, `just_draw` **clears the
  SELECTED bit** (`state &= ~SELECTED`, `gemoblib.c:790`) so the generic
  SELECTED XOR at the end of `just_draw` does **not** additionally invert the
  icon — the icon owns its own selection appearance. `ob_change` likewise
  **never uses XOR for `G_ICON`** (`gemoblib.c:1286-1287`); it always redraws.

## `CICONBLK` — colour icon (`obdefs.h:203-217`)

`G_CICON`'s `ob_spec` points to a `CICONBLK`, whose first member is a full
`ICONBLK` (`monoblk`) followed by a `CICON *mainlist` chain. Each `CICON`
(`obdefs.h:203-211`) has `num_planes`, `col_data`/`col_mask` (colour bitmap +
mask) and optional `sel_data`/`sel_mask` for the SELECTED state, linked via
`next_res`. When `CONF_WITH_COLOUR_ICONS` is set, `just_draw` passes
`cicon = ((CICONBLK *)spec)->mainlist` to `gr_gicon`; with it **off**,
`G_CICON` is not a separate case and `cicon` stays `NULL`, so only the mono
`ICONBLK` head is drawn (`gemoblib.c:776-789`).

## Related data structures

- [`object.md`](object.md) — the `OBJECT` node; `G_ICON`/`G_CICON` payloads
- [`bitblk.md`](bitblk.md) — the simpler raw-raster image payload (`G_IMAGE`)

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). The 34-byte layout, `ib_char`
  packing, and the mask/data compositing are documented in Compendium;
  cross-check **deferred**.
