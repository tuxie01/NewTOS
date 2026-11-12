# XBIOS21 — `Setcolor(n,-1)` returns 0: the ST palette shadow starts empty

Status: OPEN (2026-11-12) | Priority: P2

Type: developer (spec check first). Owner decision 2026-11-12 ("ja ticket": yes, a ticket), from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Facts

- `tosdump/cases/bios-r1.tsv` cases 93 `Setcolor(0,-1)` and 94 `Setcolor(1,-1)` differ on both profiles: the oracle
  returns `0x0FFF` and `0x0F00`, NewTOS `0`.
- `Setcolor` returns the previous palette word; `-1` is the query form (`docs/api/xbios/07-setcolor.md`,
  `references/emutos/bios/screen.c:1015-1036`).
- NewTOS keeps the 16-entry shadow in `palette()` (`tos-api/src/xbios/screen.rs:393@PAL`), initialised to all zeros;
  `setcolor` (`tos-api/src/xbios/screen.rs:335@setcolor`) returns from it. Nothing seeds it from the default palette.
- Failure picture: a program that saves the palette with `Setcolor(n,-1)` and restores it on exit sets every colour
  to black.

## Work

1. Spec: from EmuTOS, which palette the shadow holds at boot (the default ST/STe palette table, its source line, and
   whether it depends on the profile); whether `Setpalette` and `VgetRGB` (XBIOS22) read the same shadow. Update
   `docs/api/xbios/07-setcolor.md`.
2. Seed the shadow at boot from that table, keeping it consistent with the VDI colour LUT the backend displays
   (`apply_to_clut`, `tos-api/src/xbios/screen.rs:359@apply_to_clut`): state which of the two leads at boot.
3. Host tests: `Setcolor(n,-1)` after boot returns the default word for every n; a set then a query returns the set
   word; the query does not change the shadow.

## Acceptance

- `bios-r1` cases 93 and 94 EQUAL on tt and falcontc; no case equal in
  `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT` differs anew.
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.
