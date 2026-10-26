# `OBJECT` — the AES object-tree node

> Phase 00 spec (AES Object/Form batch). Source: EmuTOS @ `1a7484ab` —
> `include/obdefs.h:141-154` (struct), `:65-140` (type/flag/state/colour
> constants). Walked by every `objc_*` / `form_*` call; see
> [`../aes/object-form.md`](../aes/object-form.md).
>
> **Contract vs. reference** — see [`../../COMPATIBILITY.md`](../../COMPATIBILITY.md).
> The **field layout, the `G_*` type numbers, the `ob_flags`/`ob_state` bit
> values, and the `ob_spec` colour-word bit layout** are **contract** — a
> resource file / program reads and writes these directly. The 24-byte m68k
> size vs. the native 64-bit-`ob_spec` variant is the ABI split described in
> COMPATIBILITY "Native vs m68k struct variants".

## Purpose

A GEM user interface is a **flat array of `OBJECT` nodes**. Tree structure is
expressed by **sibling/child indices** into that array, not by pointers:
`ob_head`/`ob_tail` give the first/last child, `ob_next` the next sibling.
`ROOT = 0` (`obdefs.h:20`), `NIL = -1` (`obdefs.h:21`) means "none". Every
`objc_*`/`form_*` call takes the array base (`addr_in[0]`) plus an object
index.

The end of the tree is the object whose `ob_flags` has `LASTOB` (0x20) set;
the last child of a parent is the one whose `ob_next` points back at the
parent (this is how `get_par` walks siblings to the parent, `gemobjop.c:145`).

## Binary layout (m68k: 24 bytes, fixed by ABI)

```c
typedef struct {
/* 0x00 */ WORD  ob_next;    /* next sibling index (-1 = none)        */
/* 0x02 */ WORD  ob_head;    /* first child index (-1 = none)         */
/* 0x04 */ WORD  ob_tail;    /* last child index (-1 = none)          */
/* 0x06 */ UWORD ob_type;    /* G_* graphic type (low byte; see note) */
/* 0x08 */ UWORD ob_flags;   /* SELECTABLE/DEFAULT/EXIT/... bitfield  */
/* 0x0A */ UWORD ob_state;   /* SELECTED/CHECKED/DISABLED/... bitfield*/
/* 0x0C */ LONG  ob_spec;    /* type-specific payload (see below)     */
/* 0x10 */ WORD  ob_x;       /* x, relative to parent                 */
/* 0x12 */ WORD  ob_y;       /* y, relative to parent                 */
/* 0x14 */ WORD  ob_width;   /* width                                 */
/* 0x16 */ WORD  ob_height;  /* height                                */
/* 0x18 — end (24 bytes) */
} OBJECT;
```

> **Native (AArch64) variant.** `ob_spec` is a 64-bit pointer/value, so the
> native `OBJECT` is **32 bytes** (8-byte aligned). The first six WORDs are
> unchanged; `ob_spec` is padded up to its natural alignment, so it sits at
> **0x10** (not 0x0C: bytes 0x0C..0x0F are padding) and occupies 8 bytes, and
> the `ob_x`..`ob_height` block follows at **0x18** (COMPATIBILITY
> "Native vs m68k struct variants"). Same fields, same meaning.
>
> | Offset | Field | Size |
> |-------:|-------|-----:|
> | 0x00 | `ob_next` | 2 |
> | 0x02 | `ob_head` | 2 |
> | 0x04 | `ob_tail` | 2 |
> | 0x06 | `ob_type` | 2 |
> | 0x08 | `ob_flags` | 2 |
> | 0x0A | `ob_state` | 2 |
> | 0x0C | (padding) | 4 |
> | 0x10 | `ob_spec` (u64) | 8 |
> | 0x18 | `ob_x` | 2 |
> | 0x1A | `ob_y` | 2 |
> | 0x1C | `ob_width` | 2 |
> | 0x1E | `ob_height` | 2 |
> | 0x20 | end (32 bytes) | |
>
> **Source of the native offsets:** the compile-time assertions in
> `tos-types/src/aes.rs:509-522` (`size_of::<Object>() == 32`,
> `align_of == 8`, `offset_of!(ob_spec) == 16`, `offset_of!(ob_x) == 24`, ...)
> and the `#[repr(C)]` struct at `tos-types/src/aes.rs:33-60`. This is a
> NewTOS layout (not in EmuTOS); the m68k table above is checked against
> `include/obdefs.h:141-154` (24 bytes, unchanged). **Correction 2026-10-16
> (SPEC01):** the earlier text put native `ob_spec` at 0x0C and `ob_x` at 0x14,
> which is only true if `ob_spec` were unaligned and contradicts both the
> `repr(C)` layout and the assertions. In the native struct `ob_spec` is a
> `u64`; for box types it holds `(thickness << 16) | colour-word`
> (`tos-types/src/aes.rs` field doc), i.e. the same value the m68k LONG holds.
> Coordinates `ob_x`..`ob_height` are **in pixels** after `rs_obfix()` converts
> a resource from character to pixel units (the alert builder does this at
> `gemfmalt.c:298-301`).

## `ob_type` — the `G_*` graphic types (`obdefs.h:65-80`)

`ob_sst` masks `ob_type` with `0x00ff` before dispatching
(`gemobjop.c:40`), so the **low byte** selects the drawing type; the high
byte is an extended-type field written by the resource compiler
(`erd.c:2087-2090`) and preserved but ignored by the low-byte dispatch.

| `G_*` | # | `ob_spec` payload |
|-------|--:|-------------------|
| `G_BOX`      | 20 | colour word (LONG; low WORD = colour, byte 1 = border thickness) |
| `G_TEXT`     | 21 | `TEDINFO *` |
| `G_BOXTEXT`  | 22 | `TEDINFO *` |
| `G_IMAGE`    | 23 | `BITBLK *` |
| `G_USERDEF`  | 24 | `USERBLK *` (`ub_code`/`ub_parm`) |
| `G_IBOX`     | 25 | colour word (invisible/inside box — border only) |
| `G_BUTTON`   | 26 | `char *` (button text) |
| `G_BOXCHAR`  | 27 | colour word; the single char is byte 0 of `ob_spec` |
| `G_STRING`   | 28 | `char *` |
| `G_FTEXT`    | 29 | `TEDINFO *` (formatted/editable text) |
| `G_FBOXTEXT` | 30 | `TEDINFO *` (formatted/editable, boxed) |
| `G_ICON`     | 31 | `ICONBLK *` |
| `G_TITLE`    | 32 | `char *` (menu title) |
| `G_CICON`    | 33 | `CICONBLK *` (colour icon; a `CICONBLK` starts with an `ICONBLK`) |

> **`INDIRECT` indirection.** If `ob_flags & INDIRECT` (0x100), `ob_spec` is
> **a pointer to** the real spec value, not the value itself; `ob_sst`
> dereferences one level (`*(LONG *)objptr->ob_spec`, `gemobjop.c:36-37`,
> and `ob_getsp` for editing, `gemobed.c:60-63`).

## `ob_spec` — the colour word (for `G_BOX`/`G_IBOX`/`G_BOXCHAR`)

Decoded by `gr_crack(UWORD color, …)` (`gemgraf.c:612-628`). For box types
`ob_spec` is a LONG whose **byte 1** (bits 8–15, i.e. `*(((char*)spec)+1)`,
`gemobjop.c:57`) is the **border thickness** and whose **low WORD** is the
colour word:

| Colour-word bits | Field | Meaning |
|------------------|-------|---------|
| 15–12 | border colour | `*pbc = (color >> 12) & 0x0f` |
| 11–8  | text colour   | `*ptc = (color >> 8)  & 0x0f` |
| 7     | text writing mode | bit set → `MD_REPLACE` (opaque); clear → `MD_TRANS` (transparent) |
| 6–4   | inside pattern | `*pip = (color >> 4) & 0x07` — 0 = `IP_HOLLOW`, 7 = `IP_SOLID` |
| 3–0   | inside colour | `*pic = color & 0x0f` |

Masks named in `obdefs.h:126-127`: `FILLPAT_MASK = 0x70`, `FILLCOL_MASK =
0x0f`. Colour numbers `WHITE=0, BLACK=1, RED=2 … LMAGENTA=15`
(`obdefs.h:109-124`).

> **This is a frequent desktop-rendering bug source.** Note the mode bit is
> **bit 7** of the colour WORD (not a separate byte) and pattern is a **3-bit**
> field (bits 4–6). A wrong shift here mis-draws every box/button fill.

## `ob_flags` — flag bits (`obdefs.h:82-98`)

| Flag | Hex | Meaning |
|------|----:|---------|
| `NONE`        | 0x000 | no flags |
| `SELECTABLE`  | 0x001 | object responds to selection |
| `DEFAULT`     | 0x002 | default exit (Return/Enter picks it) |
| `EXIT`        | 0x004 | selecting this object exits a `form_do` |
| `EDITABLE`    | 0x008 | editable text field (`TEDINFO`) |
| `RBUTTON`     | 0x010 | radio button (mutually exclusive among siblings) |
| `LASTOB`      | 0x020 | last object in the tree array |
| `TOUCHEXIT`   | 0x040 | exits `form_do` on button-down (not up) |
| `HIDETREE`    | 0x080 | object + its children are not drawn / not hit-tested |
| `INDIRECT`    | 0x100 | `ob_spec` points to the real spec (see above) |
| `FL3DOBJ`     | 0x200 | bit flag: 3D indicator OR activator present |
| `FL3DIND`     | 0x200 | 3D indicator (`= FL3DOBJ`) |
| `FL3DBAK`     | 0x400 | 3D background |
| `FL3DACT`     | 0x600 | 3D activator (`FL3DOBJ|FL3DBAK`) |
| `SUBMENU`     | 0x800 | item has a submenu (menu extension) |

The 3D selector is the 2-bit field `FL3DMASK = 0x600` with values
`FL3DNONE=0x000`, `FL3DIND=0x200`, `FL3DBAK=0x400`, `FL3DACT=0x600`
(`obdefs.h:92-98`). The FL3D bits are only acted on when
`CONF_WITH_3D_OBJECTS` is set (default 1 in EmuTOS `1a7484ab`,
`include/config.h:1259`); otherwise they are inert flag bits.

## `ob_state` — state bits (`obdefs.h:100-107`)

| State | Hex | Rendering effect (`just_draw`, `gemoblib.c:867-943`) |
|-------|----:|------------------------------------------------------|
| `NORMAL`   | 0x00 | nothing |
| `SELECTED` | 0x01 | XOR-invert the object rect (or colour-swap under 3D) |
| `CROSSED`  | 0x02 | draw an X across the box (`gsx_cline` diagonals) |
| `CHECKED`  | 0x04 | draw a check-mark char (0x08 from IBM font) at top-left |
| `DISABLED` | 0x08 | grey the object with a dither pattern (`IP_4PATT`) |
| `OUTLINED` | 0x10 | draw a double outline 3 px outside the object |
| `SHADOWED` | 0x20 | draw a drop shadow to the lower-right |
| `WHITEBAK` | 0x40 | white background (used with extended string underlining) |

> **Correction / caution.** There is **no `DRAW3D` (0x80) state bit** in this
> EmuTOS source — `obdefs.h:100-107` defines state bits only up to
> `WHITEBAK = 0x40`. Bit 0x80 of `ob_state` is unused as a standard state;
> `just_draw` treats the high byte of `ob_state` specially only for the
> extended-object underline test `(state & 0xFF00) == 0xFF00` under
> `CONF_WITH_EXTENDED_OBJECTS` (`gemoblib.c:851`). A NewTOS renderer must not
> invent a `DRAW3D` state bit; 3D drawing is driven by the **`ob_flags`
> FL3D\*** bits, not by an `ob_state` bit.

## Related data structures

- [`tedinfo.md`](tedinfo.md) — `G_TEXT`/`G_BOXTEXT`/`G_FTEXT`/`G_FBOXTEXT` payload
- [`iconblk.md`](iconblk.md) — `G_ICON` payload (and the mono head of `CICONBLK`)
- [`bitblk.md`](bitblk.md) — `G_IMAGE` payload

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). The 24-byte layout, the `G_*`
  numbers, the flag/state bit values and the colour-word bit split are
  documented in Compendium; that cross-check is **deferred** until the PDFs
  are available. EmuTOS `obdefs.h` is authoritative here.
