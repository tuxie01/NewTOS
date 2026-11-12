# `VgetSize` (XBIOS 0x5B)

> Spec written 2026-11-12 (TASK/XBIOS22 spec step, researcher). Source: EmuTOS @ `1a7484ab`,
> `bios/videl.c:724-757` (`vgetsize`), table slot `bios/xbios.c:1415`. Measured: case 213 of
> `tosdump/cases/bios-r1.tsv` (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the LONG byte count for a Falcon mode word, the
> "function number when no Videl" answer. Reference: the `vmode` tables and `vctl` helpers.

## EmuTOS source

**Primary location:** `bios/videl.c:724-757` — `LONG vgetsize(WORD inmode)`, slot `0x5B`
(`bios/xbios.c:1415`, `CONF_WITH_VIDEL`).

**Algorithm (read):**
1. `!has_videl` -> returns `0x5b` (`:730-731`).
2. `mode = inmode & VIDEL_VALID` (`0x01FF`, `include/biosdefs.h:146`); if the bpp field
   (`VIDEL_BPPMASK` = `0x0007`) is greater than `VIDEL_TRUECOLOR` (4) it is set to 4
   (`:733-737`).
3. `mode = vfixmode(mode)` (`:739`): adjusts the mode word to the **current monitor**
   (`vmontype()`, PAL bit from the current mode, VGA/TV vertical bit; `bios/videl.c:917-956`).
   TOS 4 does not do this; EmuTOS does so the result matches what `Vsetscreen` would
   allocate (comment `:702-722`).
4. `lookup_videl_mode(mode)` (`:741`, `bios/videl.c:384-411`): the VGA table, the PAL table
   or the NTSC/other table, chosen by the VGA and PAL bits. Not found: returns
   `ST_VRAM_SIZE` (32000, `bios/screen.h:17`) if `VIDEL_COMPAT` is set, else `FALCON_VRAM_SIZE`
   (368640, `:19`) (`:742-746`).
5. `vctl = determine_vctl(mode, vmontype())` (`:749`, `:434-470`); `height = p->vde - p->vdb`;
   `>>= 1` if `!(vctl & 0x02)` (not interlaced); `>>= 1` if `vctl & 0x01` (double line)
   (`:750-753`).
6. Result `determine_width(mode) * 2 * height` (`:756`): `determine_width` is words per line,
   `(80COL ? 40 : 20) << bppfield`, times 1.2 for overscan when VGA is clear
   (`:417-428`); `* 2` makes bytes per line. **Bytes of one screen, no overscan rounding, no
   alignment padding.**

**Mode word** (`include/biosdefs.h:146-158`): bit 8 `VERTICAL`, 7 `COMPAT` (ST), 6 `OVERSCAN`,
5 `PAL`, 4 `VGA`, 3 `80COL`, bits 2-0 depth (0 1bpp, 1 2bpp, 2 4bpp, 3 8bpp, 4 16bpp truecolor).

**Worked example (cases 213):** mode `0x1C` = `VGA|80COL|TRUECOLOR`; the VGA table row for `0x1C`
(`bios/videl.c:139`) has `vdb = 0x003F`, `vde = 0x03FF` -> 960; `determine_vctl` gives
`0x08` (VGA, 80 col, `:436-437`), neither interlace bit set after the VGA test (`vctl & 0x02`
clear) -> `960 >> 1 = 480` lines; width `40 << 4 = 640` words = 1280 bytes;
`1280 * 480 = 614400 = 0x96000` — the measured oracle value (falcontc).

**Build configuration:** `CONF_WITH_VIDEL`.

## Trap interface

- **Trap number:** 14. **Function ID:** `0x5B` (91). `VgetSize(mode)` = `xbios_l_w(91, a)`
  (`include/xbiosbind.h:63`): one WORD parameter, LONG result.
- **Native (AArch64) ABI:** ID `0x5B` in `x8`; `mode` in `x0` (WORD); LONG result in `x0`.

## Outputs

- **Return:** LONG byte count (positive) or `0x5B` without Videl.
- **Side effects:** none. **Errors:** none (invalid modes are repaired, see steps 2 and 4).
- **Depends on the current monitor and current PAL bit** (via `vfixmode`): the same input word
  can give different sizes on different monitors (comment `:702-722`).

## Measured (oracle)

| Profile | Case 213 `VgetSize(0x1C)` | Reading |
|---------|--------------------------:|---------|
| `tt` | `0x5B` (equal to NewTOS today) | no Videl |
| `falcontc` | `614400` | formula above; NewTOS today `0x5B` (DIFF) |

## Edge cases discovered in source

1. Largest geometry in the VGA table is a 640x480 class (VGA 80-col truecolor `0x1C`); no
   Falcon mode word can name 1920x1080 (`bios/videl.c:120-306` tables; word has no width field,
   only the 40/80-column bit).
2. Not-in-table modes get the fixed sizes of step 4, not 0.
3. Truecolor is the highest bpp code; any bpp field above 4 is folded to it (step 2) — there is
   no 24/32-bit Falcon mode word.

## NewTOS mapping — options for the owner (no decision taken here)

Facts for the choice: NewTOS reports a config.txt-driven geometry (ADR-017 rev 2, default
1920x1080); the VDI device pixel is the Falcon 16-bit RGB565 word (ADR-024: `fd_nplanes=16`,
2 bytes/pixel in a memory MFDB); the **physical screen memory behind `Physbase` is 32-bpp**
(`tos-api/src/host.rs:573@bpp`, `:579@size_bytes`: `stride_bytes * height`, 1920*4*1080 =
8 294 400 bytes per screen at the default; two screens are published). A program calls
`VgetSize` to size a screen buffer for the mode it intends to use.

| Option | `VgetSize(mode)` answers | Consequence |
|--------|--------------------------|-------------|
| A. EmuTOS formula, port the Falcon tables | the Falcon-mode byte count for any mode word (case 213 = 614400, EQUAL) | Faithful to the oracle and answers the question "size of that Falcon mode" correctly, but not "size of NewTOS's screen"; needs `vfixmode`, `vmontype` (0x59) and the three init tables (or just the `vdb/vde` of the VGA table) ported; a program that allocates this for its own copy of the screen under-allocates by 13.5x at 1920x1080x32 (8 294 400 vs 614 400). |
| B. Current screen only | ignores `mode`; always the byte size of one NewTOS screen: `size_bytes` (32-bpp, 8 294 400 at 1920x1080) | Correct for "buffer I can blit Physbase into"; case 213 stays DIFF (record in COMPATIBILITY). The 16-bit alternative (`width*height*2` = 4 147 200) matches the VDI device pixel, but not the bytes a program that copies `Physbase` memory handles; choose one and document it. |
| C. Hybrid | the current-screen size when `mode` equals the word NewTOS reports as its current mode (what `VsetMode(-1)` would answer, see below), the Falcon formula for every other word | Programs that ask about the running mode get the right size, programs probing other modes get Falcon-table answers; needs the `VsetMode(-1)` word to be defined (open question in `04-getrez.md` and TASK/XBIOS22). Most code. |

Open points: (1) 32-bpp vs 16-bit bytes for B/C (the owner's ADR-024 contract says memory MFDB = 16-bit,
Physbase memory = 32-bit); (2) whether `mode == -1` (the `VsetMode` convention) is to be accepted —
EmuTOS masks it to `0x01FF` and runs the formula on `0x1FF` bpp-folded (`:733-737`), it is not a query.

Host tests: the chosen answer for the chosen inputs; the folded bpp (`mode & 7 > 4`) and the
not-in-table fallback if option A is taken.
