# XBIOS21 — `Setcolor(n,-1)` returns 0: the ST palette shadow starts empty

Status: IMPLEMENTED (2026-11-12) | Priority: P2 | Commit: self | Missing: tester acceptance

Type: developer (spec check first). Owner decision 2026-11-12 ("ja ticket": yes, a ticket), from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Facts

- `tosdump/cases/bios-r1.tsv` cases 93 `Setcolor(0,-1)` and 94 `Setcolor(1,-1)` differ on both profiles: the oracle
  returns `0x0FFF` and `0x0F00`, NewTOS `0`.
- `Setcolor` returns the previous palette word; `-1` is the query form (`docs/api/xbios/07-setcolor.md`,
  `references/emutos/bios/screen.c:1015-1036`).
- NewTOS keeps the 16-entry shadow in `palette()` (`tos-api/src/xbios/screen.rs:401@PAL`), initialised to all zeros (before this fix);
  `setcolor` (`tos-api/src/xbios/screen.rs:335@setcolor`) returns from it. Nothing seeds it from the default palette.
- Failure picture: a program that saves the palette with `Setcolor(n,-1)` and restores it on exit sets every colour
  to black.

## Work

1. Spec: from EmuTOS, which palette the shadow holds at boot (the default ST/STe palette table, its source line, and
   whether it depends on the profile); whether `Setpalette` and `VgetRGB` (XBIOS22) read the same shadow. Update
   `docs/api/xbios/07-setcolor.md`.
2. Seed the shadow at boot from that table, keeping it consistent with the VDI colour LUT the backend displays
   (`apply_to_clut`, removed by this fix: see "Implementation"): state which of the two leads at boot.
3. Host tests: `Setcolor(n,-1)` after boot returns the default word for every n; a set then a query returns the set
   word; the query does not change the shadow.

## Acceptance

- `bios-r1` cases 93 and 94 EQUAL on tt and falcontc; no case equal in
  `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT` differs anew.
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.

## Spec step (researcher, 2026-11-12)

Written: `docs/api/xbios/07-setcolor.md` (new section "Boot palette", Pi binding rewritten, a correction
note). Findings, all source-cited there:

- **EmuTOS has no `palette[]` shadow.** `atari_setcolor` reads and writes the ST(e) palette *registers*
  (`references/emutos/bios/screen.c:1020,1031,1033`); the earlier spec text (and `_state.md §4`) said
  "shadow" and is corrected in `07-setcolor.md`.
- **Boot table:** `dflt_palette[]` (`bios/screen.c:54-59`, values `bios/screen.h:56-71`) `& 0x0FFF`
  on every profile with Videl/TT/STe shifter, `& 0x0777` on plain ST (`bios/screen.c:437-440`), loaded by
  `initialise_palette_registers` (`:432-457`); register 0 `0x0FFF`, 1 `0x0F00`, ..., 15 `0x0000`
  (full table in the spec). Measured: cases 93/94 on tt and falcontc.
- **`Setpalette`** shares the store (VBL copy of `colorptr` into the same registers,
  `bios/vectors.S:358-383`); **`VgetRGB` does not** (it reads `falcon_shadow_palette`/`ste_shadow_palette`,
  `bios/videl.c:862-909`); the VDI on Videl uses `VsetRGB`/`VgetRGB`, not `Setcolor`
  (`vdi/vdi_col.c:540-545`), and in truecolor neither.
- **Register order is not VDI pen order** (`MAP_COL_ROM`, `vdi/vdi_col.c:28-29`); NewTOS's `apply_to_clut`
  writes register index as pen (identity `MAP_COL`).

Proposal (options with consequences in the spec's Pi binding): **A** seed the shadow from the table at boot,
do not touch the VDI table (shadow leads); B (VDI table leads) cannot equal the oracle for n = 1..15;
C (shadow plus register-to-pen mapping).

Owner questions:
1. Shadow leads (A, recommended), VDI table leads (B) or mapped (C)?
2. Does an XBIOS palette write (`Setcolor`/`Setpalette`) still reach the VDI colour table in the truecolor
   VDI? EmuTOS Falcon/truecolor: no. Keeping it is today's visible effect, but a "save with `Setcolor(n,-1)`,
   restore with `Setcolor(n,saved)`" round trip then recolours VDI pens in register order (pen 1 red).
3. `0x0777` plain-ST mask: not applicable (NewTOS is STe-class `0x0FFF`), confirm.

## Owner decision 2026-11-12

Owner 2026-11-12: "ich stimme allem so zu wie du vorgeschlagen hast" (I agree to everything as you proposed), answering the lead's recommendations.

1. A: the shadow leads, seeded at boot with the EmuTOS table (`dflt_palette & 0x0FFF`, spec `docs/api/xbios/07-setcolor.md`).
2. No: an XBIOS palette write (`Setcolor`/`Setpalette`) does not reach the VDI colour table in the truecolor VDI.
3. Confirmed: no `0x0777` plain-ST mask; NewTOS is STe-class `0x0FFF`.

## Localization (developer, 2026-11-12)

`cargo xtask deps setcolor --down`: `setcolor` -> `atari_setcolor` (bios), NewTOS
`tos-api/src/xbios/screen.rs@setcolor`; complete, no unmapped function in the touched layer.
`--up` retest set: `xbiostrap`/`biostrap` (XBIOS dispatch) and the VDI callers `set_color`,
`vdi_vq_color`, `vdi_vs_color`, `init_colors`, `vdi_v_opnwk` and up to `ui_start`; all mapped, none
unmapped in bios, vdi, aes. The VDI callers are the EmuTOS non-Videl ST/STe paths
(`vdi/vdi_col.c:576-582,920-931`); NewTOS's VDI (truecolor, ADR-024) never calls `Setcolor`, so the
retest set on NewTOS is the XBIOS cases (bios-r1 93-95) plus the desktop/VDI colour look, which
this fix leaves untouched. `setpalette --up`: XBIOS dispatch only. Mapping rows updated:
`atari_setcolor` -> `setcolor`, `initialise_ste_palette` -> `palette` (the shadow's boot value).

## Implementation (developer, 2026-11-12)

- `tos-api/src/xbios/screen.rs`: the shadow `PAL` starts as `DFLT_PALETTE` (EmuTOS
  `dflt_palette & 0x0FFF`, register order); `setcolor` returns the old word `& 0x0FFF` and stores
  `& 0x0FFF` only for `color >= 0`; `setpalette` stores the 16 words `& 0x0FFF` in the same shadow;
  `apply_to_clut` is removed (owner decision 2: no VDI colour-table write).
- Host tests `tos-api/tests/xbios21_boot_palette.rs`.
- Specs: the Pi bindings of `docs/api/xbios/07-setcolor.md` and `docs/api/xbios/06-setpalette.md`.
- Visible effect of dropping the VDI write: no program in the tree or on the card's app set calls
  `Setcolor`/`Setpalette` to recolour (grep of `programs/`, `sdk/`, `sdk/apps/*`: only the osbind
  binding and `sdk/examples/uakid.c`'s fault probe `Setpalette(BAD)`); the desktop and VDI pens are
  not fed by the XBIOS palette. No visible change expected.
- Oracle rerun (bios-r1 93/94 on tt and falcontc): **not done by the developer**. `cargo xtask tosdump`
  builds its card with `tools/mkcard.sh`, which needs `hdiutil attach` and `diskutil
  partitionDisk/mount`; workers may not run those (owner rule, lead 2026-11-12), and the call was
  stopped (mkcard failed, "Volume on disk7s1 failed to mount"; trap cleanup ran). The rerun is left
  to the lead/tester.
