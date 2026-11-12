# XBIOS23 — `Getrez` must report the correct code; today a fixed `TT_MEDIUM` (4)

Status: OPEN (2026-11-12) | Priority: P2

Type: research (spec and ADR-017 amendment), then developer. Owner decision 2026-11-12 ("Müssen wir fixen das es
korrekt ist": we have to fix it so it is correct), from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Facts

- `tosdump/cases/bios-r1.tsv` case 87 (`Getrez`) differs on falcontc: oracle 2, NewTOS 4; equal on tt.
- `getrez` returns the constant `TT_MEDIUM` (`tos-api/src/xbios/screen.rs:164@getrez`), settled by ADR-017 rev 2
  (`docs/api/xbios/04-getrez.md`, section "NewTOS behavior (settled — ADR-017 rev 2, 2026-07-16)"), on the grounds
  that NewTOS is a 16-colour framebuffer. That ground is outdated: the VDI workstation is truecolor (ADR-024).
- EmuTOS on Videl maps the active mode to ST-compatible codes (`docs/api/xbios/04-getrez.md`, point 2 of the
  extraction; `references/emutos/bios/screen.c:1076`).
- Changing the value changes an ADR decision: it needs an ADR-017 amendment (CLAUDE.md precedence).

## Work

1. Researcher: from EmuTOS, the code `atari_getrez` returns for each Videl mode class, in particular truecolor
   (what the falcontc profile measures as 2), and what NewTOS's screen corresponds to; which programs read `Getrez`
   and what they conclude from it. Draft the ADR-017 amendment with the proposed value; owner decision.
2. Developer: implement the decided value; keep `Setscreen`'s rez handling coherent (`docs/api/xbios/04-getrez.md`,
   consistency with `sshiftmod`); update the spec section.
3. Host tests for the decided value.

## Acceptance

- Case 87 EQUAL on falcontc (or the decided difference recorded in the amendment and COMPATIBILITY); no case equal in
  the full XBIOS run differs anew.
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.
