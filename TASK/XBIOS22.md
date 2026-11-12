# XBIOS22 — Falcon video queries: `VgetMonitor`, `VgetSize`, `VgetRGB` answer for the Pi screen

Status: OPEN (2026-11-12) | Priority: P2

Type: research (spec), then developer. Owner decision 2026-11-12, from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Owner decision 2026-11-12

"müssen wir anders lösen da wir die config.txt dazu anpassen müssen. Ausser VgetMonitor, VgetSize, VgetRGB brauchen
wir das nicht." (We have to solve this differently, since that would mean adapting config.txt. Apart from VgetMonitor,
VgetSize, VgetRGB we do not need it.)

- `VgetMonitor` (0x59), `VgetSize` (0x5B), `VgetRGB` (0x5E): implement, answering for the Pi's screen.
- `VsetMode` (0x58) and `Vfixmode` (0x5F): not implemented; a mode switch at run time would need `config.txt`
  (ADR-017). They keep answering their function number; a sanctioned-deviation entry in `docs/COMPATIBILITY.md`
  records it (part of this ticket). The other Videl calls (0x5A, 0x5C-0x5D, ...) are not in scope.

## Facts

- `tosdump/cases/bios-r1.tsv` cases 211-215 differ (falcontc; 215 also tt): oracle `VsetMode(-1)` 0x1C,
  `VgetMonitor` 2, `VgetSize(0x1C)` 614400, `VgetRGB(0,16,…)` 0 plus the palette, `Vfixmode(0x1C)` 0x1C (tt -32);
  NewTOS answers the function number and leaves the buffer untouched (catch-all arm,
  `tos-api/src/xbios/mod.rs:157@other`).
- EmuTOS: `references/emutos/bios/videl.c:664` (`vmontype`), `:724` (`vgetsize`), `:862` (`vgetrgb`).
- The screen is a native 32-bpp framebuffer with a Falcon RGB565 VDI device pixel (ADR-024), geometry from
  `config.txt` (ADR-017).

## Open for the owner (from the spec step, before code)

- Which monitor type `VgetMonitor` reports for HDMI (EmuTOS knows mono, colour, VGA, TV), and which mode word
  `VgetSize` accepts (a Falcon mode word cannot express 1920x1080; whether it answers for the current screen only).
- What `VgetRGB` returns in a truecolor mode (EmuTOS: the Videl palette registers), and its relation to the
  `Setcolor` shadow (XBIOS21).
- Whether `VsetMode(-1)` (the query of the current mode, used by Falcon programs to detect the mode) may stay
  unanswered; the owner's decision above excludes `VsetMode` as a whole.

## Work

1. Researcher: spec entries for 0x59, 0x5B, 0x5E from `references/emutos/bios/videl.c` per
   `docs/SOURCE-ANALYSIS-PROTOCOL.md`, with the NewTOS mapping proposed for the points above; owner approval.
2. Developer: implement; COMPATIBILITY entry for `VsetMode`/`Vfixmode`.
3. Host tests per function: the documented return for documented input and the documented error.

## Acceptance

- Cases 212-214 equal where the spec says equal, otherwise listed in the COMPATIBILITY entry; 211 and 215 ACCEPTED
  by the entry. No case equal in the full XBIOS run differs anew.
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.
