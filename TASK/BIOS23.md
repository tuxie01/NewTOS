# BIOS23 — Review of the parked BIOS and XBIOS tickets against the EmuTOS yardstick

Status: OPEN (2026-11-20) | Priority: P2

Type: researcher (reading). Owner decision 2026-11-20: EmuTOS is the yardstick and the BIOS level is to be finished
soon; "no program calls it" is no longer a sufficient reason to leave a difference parked.

## Scope

TASK/BIOS13.md, TASK/BIOS14.md, TASK/XBIOS06.md, TASK/XBIOS07.md, TASK/XBIOS11.md, TASK/XBIOS16.md,
TASK/XBIOS19.md, TASK/XBIOS20.md, TASK/XBIOS24.md.

## Work

One row per ticket in a new dated analysis under docs/analysis/ (bios-parked-review-2026-11-20.md, to be written):

- what differs from EmuTOS today (from the ticket, re-checked against the code at `main`; a difference that no
  longer exists is said so);
- class: (a) can be made equal to EmuTOS without hardware the Pi lacks, with the size of the change (functions,
  files) and the spec state; (b) bound to hardware the Pi does not have (MFP, MIDI, floppy, Videl), with the ADR or
  hardware fact; (c) documentation or tooling only;
- for (a): what the fix's acceptance case would be and whether the pinned oracle can show it;
- the proposed order of the (a) tickets.

## Acceptance

- The file is landed; no ticket is edited, reopened or closed by the researcher.
- The lead brings the list to the owner: (a) into the queue, (b) as accepted deviations, (c) as decided per ticket.

## Review landed; owner decisions (2026-11-20)

Landed as `0aa912a7b` (researcher `81ffb0fa3`, GAPS: two return values on the oracle are unmeasured):
`docs/analysis/bios-parked-review-2026-11-20.md`. Owner 2026-11-20, on the lead's five recommendations ("ich gehe
mit deinen empfehlungen"):

1. The analysis is landed as it is.
2. The four tickets that can be made equal to EmuTOS go into the queue in this order: TASK/XBIOS11.md,
   TASK/XBIOS06.md, TASK/XBIOS16.md, TASK/XBIOS19.md. Each is set OPEN (and translated) when its turn comes.
   XBIOS16 starts with a measurement on the pinned oracle (the `d0` of `Jdisint` and `Jenabint`) and a spec file.
3. TASK/XBIOS20.md: the table case for `Mfpint` and `Xbtimer` and the measurement of `Initmous`'s `d0` go with
   XBIOS16 (the same oracle run). The hardware part (`Midiws`, `Offgibit`, `Ongibit`, `Setpalette`'s `colorptr`
   and VBL coupling, `Initmous`'s effect) becomes an accepted deviation under ADR-031 once the measurement is in.
4. The documentation rests of TASK/BIOS13.md and TASK/XBIOS07.md and the two stale spec files the review found go
   into TASK/SPEC07.md; both tickets are closed after it. TASK/BIOS14.md stays parked behind the product fixes.
5. TASK/XBIOS24.md stays parked until the owner chooses among the options of the analysis of 2026-11-18.
