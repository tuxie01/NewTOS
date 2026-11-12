# `VgetRGB` (XBIOS 0x5E)

> Spec written 2026-11-12 (TASK/XBIOS22 spec step, researcher). Source: EmuTOS @ `1a7484ab`,
> `bios/videl.c:862-909` (`vgetrgb`), slot `bios/xbios.c:1418`. Measured: case 214 of
> `tosdump/cases/bios-r1.tsv` (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`;
> the 64 captured bytes are in `measurements/measurement-2026-11-12-xbios-full/runs/bios-r1/tosdiff-EMUTOS-1a7484ab-BIOS-falcontc.txt`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: ABI, the `0x00RRGGBB` longword format, the 16/256 index limit, the `-1` error,
> the "function number when no Videl" answer. Reference: which software table holds the value.

## EmuTOS source

**Primary location:** `bios/videl.c:862-909` — `WORD vgetrgb(WORD index, WORD count, ULONG *rgb)`,
slot `0x5E` (`bios/xbios.c:1418`, `CONF_WITH_VIDEL`).

**Algorithm (read):**
1. `!has_videl` -> returns `0x5e` (`:869-870`).
2. `index < 0 || count <= 0` -> `-1` ("Generic error", `:872-873`).
3. `mode = vsetmode(-1)` (the current mode word); `limit = 256` if the depth field is 8bpp
   (`VIDEL_8BPP`), else `16` — so **truecolor, 4bpp, 2bpp and 1bpp all have limit 16**
   (`:875-876`). `index + count > limit` -> `-1` (`:878-879`).
4. `use_ste_palette(mode)` (`:778-792`: mono monitor, or 2bpp, or ST-compatible 4bpp): values are
   derived from `ste_shadow_palette[index..]` (`:885-896`): each word `v` is rotated back from
   STe bit order, `value = ((v & 0x0777) << 1) | ((v & 0x0888) >> 3)`, and each channel becomes
   `nibble * 16` (so full scale is `0xF0`, not `0xFF`); the longword is `0x00RRGGBB`.
5. Otherwise from `falcon_shadow_palette[index..]` (`:899-908`): the shadow stores
   `R,G,0,B` bytes (`bios/videl.c:42-107`, `FRGB_*` in `bios/videl.h:27-42`); the output longword is
   `0x00RRGGBB` (byte 0 cleared, R and G shifted right one byte, B unchanged).
6. Returns `0` (`:908`).

**The tables.** `falcon_shadow_palette[256]` (`:110`) and `ste_shadow_palette[16]` (`:111`) are
file-static, written by `initialise_falcon_palette` (`:998-1042`: all 256 entries from
`videl_dflt_palette[]` (`:42-107`), entries 1 or 3 replaced by entry 15 in 1bpp/2bpp
modes, `:1008-1015`; `ste_shadow_palette` only when `use_ste_palette`, `:1037-1041`) at boot and
at `Setscreen` with a rez change (`bios/screen.c:999-1000`), and by `VsetRGB` (`:806-859`).
`Setcolor` and `Setpalette` write neither (see `07-setcolor.md`, "Boot palette").

**Return type.** The binding says `xbios_v_wwl` (`include/xbiosbind.h:65`, void), the handler
returns `WORD` (`:862`) and the table slot is the handler itself (`VEC`, `bios/xbios.c:1268-1271`,
no wrapper unless `DBG_XBIOS`, where `xbios_5e` is `void`, `:990-992`): the oracle returns `0`
(case 214).

## Trap interface

- **Trap number:** 14. **Function ID:** `0x5E` (94). Parameters `(WORD index, WORD count, ULONG *rgb)`
  (10 bytes on the m68k stack).
- **Native (AArch64) ABI:** ID `0x5E` in `x8`; `index` in `x0`, `count` in `x1`, `rgb` pointer in `x2`;
  WORD result in `x0`. The buffer is `count` longwords written to the caller's space (fault -> the
  usual uaccess handling, ARCH03).

## Outputs

- **Return:** `0` ok, `-1` (index/count/limit), `0x5E` without Videl.
- **Memory:** `count` longwords `0x00RRGGBB` at `rgb`; untouched on error and without Videl.
- **Side effects:** none.

## Measured (oracle, case 214 `VgetRGB(0,16,buf)`)

| Profile | Return | Buffer |
|---------|-------:|--------|
| `tt` | `0x5E` (= NewTOS today) | untouched |
| `falcontc` | `0` | 16 longs of the `videl_dflt_palette` head: white `00FFFFFF`, red `00FF0000`, green `0000FF00`, yellow `00FFFF00`, blue `000000FF`, ..., light grey `00BBBBBB`, grey `00888888`, ..., black `00000000` (captured bytes `[0..3]`, `[4..7]`, `[28..31]`, `[32..35]`); NewTOS today returns `0x5E` and leaves the sentinel `0xDE 0xAD` pattern |

## Edge cases discovered in source

1. The truecolor boot mode has **no STe branch** (`use_ste_palette` false), so the oracle's answer
   in truecolor is the Falcon table, `0xFF` full scale. The same palette read through the STe branch
   would give `0xF0` full scale and `0xA0` for light grey (`0x0555 -> 0x0AAA -> 0xA0`): the two
   branches are not interchangeable (inferred arithmetic from `:888-895`).
2. `index+count > 16` is an error even in truecolor, although the Videl has 256 registers
   (`:876`).
3. Entry 15 is the black end of the table; in 1bpp/2bpp the shadow additionally has entry 1/3 :=
   entry 15 (`:1008-1015`).

## NewTOS mapping — options for the owner (no decision taken here)

Question from XBIOS22: what does `VgetRGB` return in a truecolor mode, and how does it relate to
the `Setcolor` shadow (XBIOS21)? EmuTOS facts: in truecolor the VDI does not use `VsetRGB`/`VgetRGB`
(it writes per-workstation 5-bit pseudo-palettes, `vdi/vdi_col.c:601-614,648-662`), and `VgetRGB`
returns a stored table that nothing else changes unless `VsetRGB` (0x5D, not in this ticket) runs.

| Option | `VgetRGB` returns | Consequence |
|--------|-------------------|-------------|
| A. A separate Falcon table (recommended) | A 16-entry (limit 16) table seeded from `videl_dflt_palette[0..16]` (head: `FRGB_*` constants, `bios/videl.h:27-42`), in `0x00RRGGBB`; `-1` for `index+count > 16` | Byte-equal to case 214, matches EmuTOS's independence of `Setcolor`; stays constant because `VsetRGB` is not implemented (observation: the table is read-only until 0x5D exists). |
| B. Derived from the `Setcolor` shadow | each ST word expanded as in the STe branch (`nibble*16`) | One table with XBIOS21; case 214 DIFF (`0xF0` vs `0xFF`, light grey `0xA0` vs `0xBB`), because the Falcon defaults are not `nibble*16` of the ST defaults. Programs that read RGB via VgetRGB and compare to what they set via Setcolor would only agree on the 4-bit grid. |
| C. Derived from the VDI colour table (pens 0..15 as 8-bit) | the displayed colour of pen `n` | "What is on screen", but pen order is not register order (see `07-setcolor.md` Pi binding), colours are per-workstation under ADR-024, and EmuTOS does not do it; case 214 DIFF. |
| D. Return `-1`, no palette in truecolor | error | Not what the oracle does in truecolor (returns 0 and the table); a program that saves/restores the palette around its own mode would treat `-1` as failure. |

Open points: (1) the relation to the XBIOS21 shadow is "none" under A (two tables, as in EmuTOS); an
owner who wants one palette picks B and accepts the case 214 difference in COMPATIBILITY;
(2) the 256-register limit applies only to 8bpp modes (`:876`) — NewTOS is truecolor-only, so limit
16 always; (3) `VsetRGB` (0x5D) is outside TASK/XBIOS22: without it A is a constant table.

Host tests: ret `0` and the 16 values for `(0,16)`; `-1` for `index < 0`, `count <= 0`,
`index+count > 16`; the buffer untouched on error.
