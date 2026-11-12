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

## Results (auditor, 2026-11-12)

Run `measurements/measurement-2026-11-12-snd04/RUNINFO.TXT`, report `docs/audits/2026-11-12-snd04-dmasound.md`, probe
`tosdump/probes/snprobe.c` (a table cannot express the timing; DUMPBIOS refuses the set arms of this group). Oracle =
EmuTOS on Hatari 2.6.1's DMA-sound model; its Falcon (falcontc) and STe/TT (tt) models disagree in places.

- O1: lock pair 1, -129, 0, -128; boot `Soundcmd(0..6,-1)` falcontc 0 0 0x80 0x80 3 3 3, tt all 0; NewTOS answers
  the function number on all 41 calls.
- O2 (S3): `Buffoper(-1)` bit 0 drops at the end of a single-shot frame (16-17 ticks for an 81.8 ms frame), stays 3
  in loop mode.
- O3 (S4): the play pointer stays within [start, end]; falcontc byte granularity, tt even values; torn reads seen.
  S5: `Setbuffer` during a loop pass takes effect at the next pass.
- O4: the `Sndstatus(1)` conflict is settled for EmuTOS: the sound registers stay, playback continues.
- O5: one interrupt per frame pass (falcontc at the start of each pass, tt at the end); `Setinterrupt` mode 1 uses
  MFP channel 15 (vector 0x13C), not 7 as the spec said (corrected).
- Not measured: S7; the record path and the write arms of `Gpio`/`Dsptristate`/`Devconnect` only partly. Which
  Hatari model matches real hardware is unknown.
