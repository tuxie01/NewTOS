# SND06 — Sound: the PSG calls (`Giaccess`, `Dosound`, `Ongibit`, `Offgibit`) play through the sound backend

Status: OPEN (2026-11-20) | Priority: P2

Type: researcher (analysis and ADR draft), then developer, tester. Owner decision 2026-11-20.

## Owner decision 2026-11-20

From the finding of TASK/SPEC07.md that `Giaccess`, `Ongibit`, `Offgibit` and `Dosound` drive the sound chip on the
oracle while NewTOS answers as a build without one, and that `docs/DEVIATIONS.md` has no row for it. The owner:
what is hardware-bound can be argued, but the interface to the sound must conform, otherwise no program can play a
tone; where necessary it may be extended by the functions the Pi brings, documented ("die Schnittstelle zum
Soundinterface muss konform sein sonst kann kein Programm einen Ton abspielen. ggf. wenn notwendig erweitern um
die funktionen abzudecken die der PI mit sich bringt. Allerdings dokumentiert").

## Facts

- `CONF_WITH_YM2149` is 1 in the reference build (`docs/api/xbios/_psg-group.md`, corrected by SPEC07,
  `95eddbb26`); the four calls act on the PSG there.
- NewTOS returns the values of a build without a PSG (`tos-api/src/xbios/system.rs:101@YM2149`); `_SND` is `0x14`
  without the PSG bit (ADR-012, amendment 2026-11-12).
- A sound backend exists since TASK/SND05.md: the DMA ring on the PWM jack, used by XBIOS 0x80-0x8D.
- The bell and the key click of EmuTOS go through `Dosound`; NewTOS has neither (ADR-031 row `bell_hook`,
  `kcl_hook`: "no sound device yet").
- CLAUDE.md lists "no YM2149 in the kernel" among the non-goals (no hardware-level emulation); ADR-012 says NewTOS
  does not emulate the register image. What the owner asks for is the call interface, not the chip at its bus
  address.

## Work

1. Analysis (researcher): what the four calls and the `Dosound` command stream do on EmuTOS, register by register
   (tone, noise, mixer, volume, envelope; port A and B bits of `Ongibit`/`Offgibit`, of which only the sound-relevant
   ones have a counterpart); which programs in the trees use them; what a software model of the PSG's sound
   registers costs that renders into the existing backend, and how it mixes with DMA sound; what `_SND` then says;
   the bell and key click; which Pi-side functions would be an extension and how they would be documented. Options
   with their cost go to the owner.
2. ADR draft for the owner: the call-level PSG (a change of the ADR-012 amendment and of the CLAUDE.md non-goal
   wording, if the owner decides so).
3. Implementation and tests as decided.

## Acceptance

- The four calls answer as the pinned oracle in the tosdump cases (register read-back, `Dosound` pointer return).
- A test program plays a tone through `Giaccess` and through `Dosound` on the real Pi (owner's ear), mixed with or
  beside DMA sound as decided.
- `docs/COMPATIBILITY.md` and `docs/DEVIATIONS.md` state what conforms and what is extension.

## Not in this ticket

`Midiws` (no MIDI ACIA) and `Ikbdws` (no IKBD ACIA; ADR-031, TASK/XBIOS04.md) stay hardware-bound and get their rows
in `docs/DEVIATIONS.md` with TASK/SPEC08.md.
