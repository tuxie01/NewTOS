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

## Spec step (researcher, 2026-11-12)

Written: `docs/api/xbios/04-getrez.md` (corrected edge case 2 with the per-mode table, a pointer to this
section); the draft ADR-017 amendment is below. `docs/MEMORY.md` is not edited.

Findings (`references/emutos/bios/screen.c:948-987`, read this session):
- `atari_getrez` on Videl uses the stored current mode word (`vsetmode(-1)`): ST-compatible mode ->
  1bpp 2, 2bpp 1, 4bpp 0; **every non-ST-compatible Falcon mode (1/2/4/8bpp and truecolor) -> 2**
  (`:956-972`). It never returns `FALCON_REZ` (3); that value lives only in `sshiftmod`
  (`bios/videl.c:648-656`). Earlier spec text was wrong on this (corrected).
- Why falcontc measures 2: boot mode `0x1C` (VGA, 80 columns, truecolor, no ST-compat bit; case 211).
- NewTOS's screen corresponds to a **non-ST-compatible truecolor Videl mode class** (16-bit device pixel,
  ADR-024; geometry not expressible, ADR-017) -> in EmuTOS terms, `Getrez` 2, `sshiftmod` 3.
- What programs conclude (read in `sdk/apps`, `sdk/overlays`, `Applications/cflib-git`):
  teradesk reads `Getrez` then, behind the `_VDO` cookie, replaces it with `VsetMode(-1)`/`VgetMonitor`
  (`sdk/apps/teradesk/video.c:150-190`); HighWire and the `xgdos` overlay use `Getrez() + 2` as the
  `v_opnvwk` device id (`sdk/apps/highwire/src/Nice_VDI.c:24`, `sdk/overlays/xgdos/xos_newtos.c:92@xos_screen_dev`);
  cflib treats 0/1 as ST low/mid (`Applications/cflib-git/appinit.c:72`). EmuTOS's own AES does not consult
  `Getrez` on Videl (`aes/gemgsxif.c:159-165`).
- The ADR-017 reasoning against `ST_HIGH` ("a program that read 2 would avoid colour") still holds for
  an ST-era program; the oracle gives Falcon programs the same 2 and relies on `VsetMode`/VDI for truth.

### Draft for the owner

**ADR-017, Revision 3 (draft, not applied) — `Getrez` reports the Falcon-truecolor class.**

*Context.* Rev 2 fixed `Getrez` at `TT_MEDIUM` (4) on the ground that NewTOS is a 16-colour framebuffer.
ADR-024 superseded that: the VDI is a Falcon 16-bit truecolor workstation, committed reference profile
Falcon030/TOS4. The oracle for that profile (falcontc, case 87) answers 2 for a truecolor Videl mode
(`bios/screen.c:971-972`). 4 matches neither the colour depth nor the geometry now.

*Proposed value (A, recommended): `Getrez()` = 2 (`ST_HIGH`).* This is exactly what EmuTOS returns for the
mode class NewTOS's screen belongs to (non-ST-compatible truecolor Videl); case 87 becomes EQUAL on falcontc
(tt stays measured-different, 4, and is not NewTOS's profile). `Getrez` is then, as on a Falcon, an ST-compat
hint only; truth is VDI `DEV_TAB`/`vq_extnd` and `Physbase`; Falcon programs use `VsetMode(-1)` (see below).

*Alternatives.*
| | Value | For | Against |
|--|------:|-----|---------|
| A | 2 `ST_HIGH` | equals the oracle (case 87); the Falcon profile's documented answer | ST-era programs read "monochrome 640x400"; `Getrez()+2` = 4 as `v_opnvwk` id; the rev 2 concern returns. Without a `VsetMode(-1)` answer (TASK/XBIOS22 excludes `VsetMode`) a program has no Falcon way to learn it is not mono |
| B | 3 `FALCON_REZ` | names "Videl mode, ask VsetMode" (what EmuTOS's AES passes, `gemgsxif.c:159-163`); not "mono" | EmuTOS `Getrez` never returns 3 -> case 87 stays DIFF (needs a COMPATIBILITY entry); `Getrez()+2` = 5 in programs built for ST codes; teradesk `rmap[3]` = 0 |
| C | 4 `TT_MEDIUM` (today) | no change; "colour" signal | wrong depth (4bpp, 640x480 class) since ADR-024; DIFF to the oracle's Falcon profile; only the TT oracle profile says 4 |
| D | 7 `TT_LOW` | "colour, 256" | no source basis for NewTOS |

*Consequences.* A: change `getrez` (`tos-api/src/xbios/screen.rs:164@getrez`), the selftest expecting 4
(`kernel/src/selftest_xbios.rs:136@getrez`), the Getrez sections of `04-getrez.md`, `docs/COMPATIBILITY.md`
if B; `Setscreen`'s `rez` stays accepted-but-ignored (`05-setscreen.md`), so the verification
"after `Setscreen(-1,-1,r)` `Getrez()` equals r" does not apply to NewTOS and should be removed from the
spec (no `sshiftmod` variable exists in `tos-api/src` or `kernel/src`: grep 2026-11-12). Interplay with
TASK/XBIOS22: with A, answer `VsetMode(-1)` (query only) or accept that Falcon-aware programs see 2 and
`0x58`.

Owner questions:
1. Value for `Getrez`: A 2 (recommended), B 3, C keep 4, D 7?
2. If A or B: answer `VsetMode(-1)` as a query so programs can tell truecolor from mono (TASK/XBIOS22 Q4)?
3. Amend ADR-017 as a Revision 3 (text above) or as an ADR-024 addendum?

## Owner decision 2026-11-12

Owner 2026-11-12: "ich stimme allem so zu wie du vorgeschlagen hast" (I agree to everything as you proposed), answering the lead's recommendations.

1. A: `Getrez()` = 2 (`ST_HIGH`).
2. Yes: `VsetMode(-1)` is answered as a query ([XBIOS22](XBIOS22.md) decision 4).
3. As ADR-017 Revision 3, the draft text above; applied to `docs/MEMORY.md` in the implementing commit.
