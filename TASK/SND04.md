# SND04 — Sound: oracle cases for the DMA sound group (O1-O5)

Status: OPEN (2026-11-12) | Priority: P2

Type: measurement (tosdump cases against the pinned oracle, falcontc profile). From [SND01](SND01.md), decisions
2026-11-12, Q-N7.

## Facts

- Today only the inquiries are measured (`tosdump/cases/bios-r1.tsv` 230-235); NewTOS answers the function number.
- Case list O1-O5: [SND01](SND01.md), section "Spec review (researcher, 2026-11-12)", §4; semantics S1-S7 in
  `docs/api/xbios/_dmasound-group.md` §9.1. S3 (does `Buffoper` bit 0 drop at the end of a single-shot frame) and S4
  (`Buffptr` granularity) are in no pinned source.
- The oracle is EmuTOS in Hatari: Hatari's DMA-sound model is the reference, not real silicon.

## Work

1. Cases for O1-O4 in a new table (or a probe where a table cannot express timing), recorded on the oracle
   (falcontc; tt where the group exists); O5 needs an m68k handler stub in tosdump.
2. Record the references per `docs/analysis/ORACLE.md`; run on NewTOS (today: function numbers, expected DIFF).
3. Feed the results into `docs/api/xbios/_dmasound-group.md` (S3, S4, the `Sndstatus(1)` source conflict).

## Acceptance

- O1-O5 recorded on the oracle with committed references; the spec states the measured S3/S4 and resolves the
  `Sndstatus(1)` conflict; doc-check 0 failures.
