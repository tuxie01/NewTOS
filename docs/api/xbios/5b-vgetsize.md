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

## Owner decision B and the byte count — draft for the owner (2026-11-18, no decision taken)

Owner 2026-11-12: option B, the size of the current screen only; `mode` is ignored (case 213 then differs from
the oracle's 614400; COMPATIBILITY entry). Still to choose: which byte count.

Facts (read): the memory behind `Physbase` is linear 32-bpp XRGB8888 that the program writes directly
(`docs/api/xbios/02-physbase.md:231-246`); `Host::screen()` publishes `size_bytes` = `stride_bytes * height`
for **one** screen (`tos-api/src/host.rs:579@size_bytes`; 8 294 400 at 1920x1080, QEMU `raspi4b` default
640x480: 1 228 800, `02-physbase.md:256-259`). The VDI device pixel is 16-bit (ADR-024), so a memory MFDB of
the same area is `width * height * 2` (4 147 200 at 1920x1080). EmuTOS says what the call is for: the
video RAM a `Vsetscreen` of that mode needs (`bios/videl.c:702-722`, comment above `vgetsize`).

| Option | Bytes | For | Against |
|--------|-------|-----|---------|
| B1 (recommended) | `size_bytes`: one 32-bpp screen as `Physbase` shows it, stride padding included | The question the call answers ("RAM for a screen of the current mode") has this number as its answer on NewTOS: a program that allocates it and copies `Physbase`/`Logbase` memory (a screen save, an off-screen copy of what it writes) is exactly sized; over-allocation of a 16-bit consumer is harmless, under-allocation is a crash | Not the 16-bit pixel size a VDI memory-MFDB program needs (2x too large for that) |
| B2 | `width * height * 2` | equals the VDI device pixel, EmuTOS's truecolor bytes-per-pixel (`videl.c:756`, `determine_width` word count x 2) | A program that copies `Physbase` bytes into it overruns by 2x; no NewTOS memory is 16-bit-per-pixel at screen scale except an MFDB the program builds itself |

The developer reads `Host::screen().size_bytes`; before the framebuffer is up `screen()` is `None`
(`02-physbase.md`, "How it is read"): the answer then is not decided here (see open points).

## Open points

- STOP (owner): B1 or B2 (recommended B1), then the sanctioned-deviation row for case 213.
- STOP: `VgetSize` before the framebuffer is up (`Host::screen()` = `None`): which value (0 or the function number)?
  Not decidable from EmuTOS (no Videl-present analogue); Physbase yields 0 in that state.
- Inferred: that the callers' purpose is a screen-RAM allocation (EmuTOS comment `videl.c:702-722`, not a
  measurement of any NewTOS program; no ported program calls `VgetSize`: grep `sdk/apps`, `sdk/overlays`,
  `programs` 2026-11-18, only the libc header `sdk/libc/overlay/include/mint/falcon.h:152@VgetSize` defines it).
- Cited: with B the call never runs `vfixmode`, so `Vfixmode` stays unimplemented without affecting it.

## Owner decision 2026-11-18 and implementation

Owner 2026-11-18 (`TASK/XBIOS22.md`): option B1, the byte count of the 32-bpp `Physbase` screen; before the
framebuffer is up the function number `0x5B` (EmuTOS's own no-Videl answer, `bios/videl.c:730-731`). Implemented
in `tos-api/src/xbios/screen.rs:482@vgetsize`: `Host::screen()` → `size_bytes` (LONG, sign-extended), `None` → the
dispatcher's function number (`tos-api/src/xbios/mod.rs:151@VGETSIZE`); `mode` is not read. Case 213: sanctioned
deviation in `docs/COMPATIBILITY.md` ("XBIOS Falcon video"). Host tests `tos-api/tests/xbios22_falcon_video.rs`.

## At the 32-plane setting (ADR-038 point 9, V68)

Spec gate of TASK/V68 (researcher, 2026-11-18). **Answer:** `VgetSize(mode)` is the byte size of one 32-bpp screen at
`vdiplanes=16` and at `vdiplanes=32`, the same number (and `0x5B` before a framebuffer is up).

- **Contract (ADR-038 point 9):** unchanged. Reason, from the ADR: the answer is already the 32-bpp store as
  `Physbase` shows it (option B1 above); at 32 planes it "now agrees with the VDI", because the store word is the
  device pixel (ADR-038 point 4). The numbers above that compare it with a 16-bit memory MFDB (option B2) describe
  the 16-plane setting; at 32 planes a memory MFDB of the screen's area is 4 bytes a pixel, the same unit as the
  store (arithmetic from `docs/api/vdi/raster/_raster.md`, "32-plane setting: memory forms", section 3: row =
  `fd_wdwidth * 16` pixels, 4 bytes each; equal to `size_bytes` only when the stride has no padding) **[I]**.
- **Read from the code [C]:** `vgetsize` returns `Host::screen()`'s `size_bytes`
  (`tos-api/src/xbios/screen.rs:482@vgetsize`), which the kernel fills from `stride_bytes() * height`
  (`kernel/src/host_env.rs:303@size_bytes`, `kernel/src/vdi_backend.rs:1060@size_bytes`, `:1054@stride_bytes` =
  `stride_px * 4`). Neither reads `PLANES` (`kernel/src/vdi_backend.rs:44@PLANES`); the 32-bpp request is the
  constant `DEPTH_BPP` (`kernel/src/drivers/bcm2711/framebuffer.rs:34@DEPTH_BPP`). **No dependency on the VDI depth
  setting today.**
- `docs/DEVIATIONS.md` keeps the `VgetSize(mode)` row (differs from EmuTOS's Falcon-mode bytes at both settings).

## Open points

- Inferred: independence of the setting is read from the code, not yet shown by a run at `vdiplanes=32`.
- Inferred: "equal to a 32-plane memory MFDB of the screen" holds only without stride padding; not a statement
  the contract makes.
