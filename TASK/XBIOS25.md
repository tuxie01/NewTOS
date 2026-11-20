# XBIOS25 — Move the `Random` seed case X01 from the probe to tosdiff

Status: OPEN (2026-11-20) | Priority: P3

Type: developer (tosdump), then tester. Owner decision 2026-11-20. Follow-up of TASK/XBIOS11.md under the rule for
fixes accepted by a probe (CLAUDE.md, "Testing").

## Facts

- TASK/XBIOS11.md was accepted with the permanent probe `tosdump/probes/xsseed.c` (case X01: the first `Random()`
  after the start lies in the window of the two `_hz_200` reads around it), run on the pinned oracle and on NewTOS
  (`measurements/measurement-2026-11-20-xbios11-test/RUNINFO.TXT`).
- A tosdump table case compares returned values; X01 is a relation between three values, and it holds only for the
  first `Random` call after boot.

## Work

1. Find out whether a table case can express X01 (a case that must run first, and a return class "seed of the
   `_hz_200` window"); if it can, add it and its oracle reference; if it cannot, say why and leave the probe as the
   permanent test, wired so that it does not rot (a documented way to run it on both sides).

## Acceptance

- Either the case is in a table with an oracle dump and an equal NewTOS result, or the reason is recorded in
  `docs/TOS-REFERENCE-DUMPER.md` and the probe has a documented run command.
- Existing tables and frozen references unchanged; `cargo xtask test` green.
