# `VsetMode` (XBIOS 0x58)

> Spec written 2026-11-18 (TASK/XBIOS22, owner decision 2026-11-12 item 4, researcher). Source: EmuTOS @ `1a7484ab`,
> `bios/videl.c:619-659` (`vsetmode`), slot `bios/xbios.c:1412`. Measured: case 211 of
> `tosdump/cases/bios-r1.tsv` (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).
> Scope of this entry: the **query form only** (`mode == -1`); the owner's decision keeps `mode != -1` out.

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the Falcon mode-word bit layout, the query convention `-1`, the
> "function number" answer for everything not implemented. Reference: `current_video_mode`.

## EmuTOS source

**Primary location:** `bios/videl.c:619-659` — `WORD vsetmode(WORD mode)`, slot `0x58` of `xbios_vecs[]`
(`bios/xbios.c:1412`, inside `#if CONF_WITH_VIDEL`, `:1411`).

**Behaviour (read):**
1. `!has_videl` -> returns `0x58` (the function number, `bios/videl.c:624-625`).
2. `mode == -1` -> returns the stored `current_video_mode` (`:627-628`): the mode word last
   accepted, **not** a register read-back (the variable, `:614`).
3. otherwise: `vfixmode(mode)` (`:633`), `set_videl_vga` programs the registers, an invalid mode
   returns the old word without change (`:635-637`), a valid one stores the new word, returns the
   old one (`:639-641`) and sets `sshiftmod`: `FALCON_REZ` (3) for a non-ST-compatible word
   (`:648-656`).
4. `current_video_mode` is initialised at boot from NVRAM or `FALCON_DEFAULT_BOOT`, fixed up with
   `vfixmode` and checked against the mode table (`bios/screen.c:491,529-552`).

**Mode word** (`include/biosdefs.h:146-158`): bit 8 `VERTICAL`, 7 `COMPAT` (ST), 6 `OVERSCAN`,
5 `PAL`, 4 `VGA`, 3 `80COL`, bits 2-0 depth (0 1bpp, 1 2bpp, 2 4bpp, 3 8bpp, 4 16bpp truecolor).

## Trap interface

- **Trap number:** 14. **Function ID:** `0x58` (88). `VsetMode(mode)` = `xbios_w_w(88, a)`
  (`include/xbiosbind.h:61`): one WORD parameter, WORD result.
- **Native (AArch64) ABI:** ID `0x58` in `x8`; `mode` in `x0` (WORD); WORD result in `x0`.

## Measured (oracle)

| Profile | Case 211 `VsetMode(-1)` | Reading |
|---------|------------------------:|---------|
| `tt` (no Videl) | `0x0058` | function number (`:624-625`); NewTOS equal today |
| `falcontc` | `0x001C` | `VGA|80COL|TRUECOLOR` (bits below); NewTOS today `0x0058` (DIFF) |

The oracle's `0x1C` is not `FALCON_DEFAULT_BOOT` fixed for a VGA monitor (that is `0x1A`: 16 colours,
`bios/videl.h:56`, `vfixmode` `:933-942`); so it came from the boot-mode source read at
`bios/screen.c:529` (NVRAM) — inferred, the NVRAM content was not read in this run.

## NewTOS answer — draft for the owner (no decision taken on the exact word)

Owner 2026-11-12 (`TASK/XBIOS22.md`): `VsetMode(-1)` answers a fixed word; `VsetMode(mode != -1)` stays out
(keeps the function number `0x58`, the catch-all arm `tos-api/src/xbios/mod.rs:157@other`).

**Proposed word: `0x001C`** (VGA, 80 columns, 16-bit truecolor; no VERTICAL, COMPAT, OVERSCAN, PAL).

What a program concludes from each field (read in the sources; the TeraDesk lines are the overlay
`sdk/overlays/teradesk/video.c`, the vendored upstream `sdk/apps/teradesk/video.c` has the same logic):

| Field | In `0x1C` | Consequence for the reader |
|-------|-----------|----------------------------|
| bits 2-0 = 4 | truecolor, 16 bit | Equals the VDI device pixel (ADR-024: 16-plane, RGB565). TeraDesk (`:475` `npc[16] = 4`) maps its 64K-colour screen (`xd_nplanes` 16) to depth 4, so the word it rebuilds from the dialog equals the one it was told. |
| bit 3 `80COL` = 1 | 640-column class | TeraDesk: `currez = TT_MEDRES` (`:202-209`), the radio set today (Getrez 4 -> rmap, `ARCH06` run). Geometry is **inexpressible**: no Falcon word names 1920x1080 (`5b-vgetsize.md`, edge case 1); a program must take the width/height from VDI (`vq_extnd`/`DEV_TAB`) or `Physbase`. |
| bit 4 `VGA` = 1 | VGA | Agrees with `VgetMonitor` = 2 (`59-vgetmonitor.md`, owner decision 1). `vfixmode` on a VGA monitor keeps a word with VGA set unchanged (`videl.c:934-943`). |
| bit 5 `PAL` = 0 | NTSC/none | Ignored by VGA tables; `vfixmode` copies the PAL bit of the current word (`:928-931`), so `0x1C` stays `0x1C`. |
| bit 6 `OVERSCAN` = 0 | no overscan | TeraDesk leaves the Overscan box unticked (`:566`); it is disabled anyway on VGA (`:557-558`). |
| bit 7 `COMPAT` = 0 | not ST-compatible | Same class as `Getrez() = 2` (`bios/screen.c:971-972`, `04-getrez.md`) and `sshiftmod` 3 (`videl.c:648-656`): a program that must decide "ST bitplanes or not" sees "not". Required for coherence with XBIOS23 (`Getrez` = 2). |
| bit 8 `VERTICAL` = 0 | no double line (VGA) | TeraDesk shows "Doubleline" unticked (`:567`). |

Fixed point: `vfixmode(0x1C)` on a VGA monitor returns `0x1C` (`videl.c:934-943`; derived), so any program
that feeds the answer back through the Videl fix-up gets the same word.

| Option | Word | For | Against |
|--------|-----:|-----|---------|
| A (recommended) | `0x1C` | equals the oracle (case 211 EQUAL); coherent with `VgetMonitor` 2, `Getrez` 2, VDI 16-bit; the word of the table row for a real 640x480 VGA truecolor mode (`videl.c:139`) | claims 640 columns; geometry only from VDI |
| B | `0x11C` (+ VERTICAL) | none | VGA double-line = half vertical resolution (`videl.c:750-753` halves/doubles); differs from the oracle |
| C | `0x3C` (+ PAL) | none | PAL has no meaning on VGA; differs from the oracle |
| D | keep `0x58` | no work | reads as `OVERSCAN|VGA|80COL`, 1bpp, a valid-looking mono word; TeraDesk then shows Overscan ticked and builds a different word on OK (`ARCH06` run) |
| E | a NewTOS word outside the Falcon encoding | could carry 1920x1080 | breaks every reader of the Falcon layout; no source basis |

## Outputs, errors, side effects

- **Return** `mode == -1`: the fixed word. No error path, no side effect.
- **`mode != -1`**: stays the function number `0x58` (decision), with a COMPATIBILITY entry (part of
  `TASK/XBIOS22.md` Work 2); `Vfixmode` (0x5F) identical.

## Open points

- STOP (owner): the exact word, A recommended; nothing else decides it.
- Inferred: that the oracle's `0x1C` came from NVRAM (the boot default would be `0x1A`); the word is
  nevertheless measured (case 211), not inferred.
- Inferred: TeraDesk's behaviour with `0x1C` is read from `video.c`, not measured; a developer step
  (Falcon branch of `get_set_video(0)`) is a case for the tester.
- Out of scope: EmuTOS's own AES calls `VsetMode(-1)` unconditionally to fill `ws_pts0` / `intout[45]`
  (`aes/gemgsxif.c:184`) and the desktop reads it for its Falcon dialog (`desk/deskrez.c:159`); whether
  NewTOS's VDI `v_opnwk` should return the same word in `intout[45]` is not checked here.
