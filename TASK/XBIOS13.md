# XBIOS13 — Spec `docs/api/xbios/` (`Settime`, `Gettime`, NVRAM) contradicts measurement and code

Status: OPEN (2026-11-19) | Priority: P3

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS13.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Spec-Pflege)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Spec-Pflege) · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-006 (points 1, 2, 3, 5) — S4 — MEASURED (documentation contradicts measurement or code).** Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`. File group `docs/api/xbios/`:

1. `docs/api/xbios/16-settime.md` (Pi binding): "Hour/minute/second pass through unvalidated" — measured as normalised (T10–T14, [XBIOS08](XBIOS08.md)); likewise the code comment `tos-api/src/gemdos/time.rs:216@hour` "(EmuTOS parity)" (code comment: developer).
2. `tos-api/src/xbios/mod.rs:120-122@SETTIME` "Gettime returns the packed LONG (zero-extended)" against `docs/api/xbios/17-gettime.md` "sign-extended" ([XBIOS09](XBIOS09.md)).
3. `docs/api/xbios/_triage.md:80` "`CONF_WITH_NVRAM` off" and `docs/api/xbios/_dispatcher.md:171` ("NVRAM (0x2E) entries are `xbios_unimpl` in the profile") against `docs/api/xbios/_full-function-inventory.md` (=1 in the reference build) ([XBIOS10](XBIOS10.md)).
4. `docs/api/xbios/17-gettime.md` "fallback = 1980-01-01 if the clock was never set" — without the seed from `KERNEL8.IMG` ([XBIOS12](XBIOS12.md)).

(Points 4 and 6 of the report — ERRATA and ledger — are in [BIOS13](BIOS13.md).)

## Work

1. Correct points 1, 3, 4 from the code and the EmuTOS source, every correction with `file:line`; points that depend on XBIOS08–XBIOS12 after their decision.

## Acceptance

- Every corrected statement is cited; no entry contradicts probe T01–T33 or `xbios-system-r1.tsv` any more.
- `cargo xtask test` green (unchanged); `cargo xtask doc-check` of the edited docs 0 failures.

## Status history

- 2026-11-19: reopened (was "Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: pure spec maintenance | Reopen: the spec place is read by open work"): owner 2026-11-19: the time functions must be clean, spec maintenance of the time group

## State at the reopening (lead, 2026-11-19)

Owner 2026-11-19: reopen this ticket and TASK/XBIOS14.md ("mit 1 weiter").

- Point 1 is closed by TASK/XBIOS08.md (documents `2c429952f`): the spec states the normalisation. The two code
  comments stay for the next code change of `xbios_settime`.
- Point 2 is closed by TASK/XBIOS09.md (`4fa990dad`): the code sign-extends and the comment is corrected.
- Open: point 3 (the NVRAM lines; TASK/XBIOS10.md is ACCEPTED under ADR-012, `NVMaccess` answers as EmuTOS without
  NVRAM) and point 4 (the fallback sentence; since TASK/XBIOS12.md the boot clock starts at the kernel's build
  date, 00:00:00).
- The seed named in point 4 is out of date: the FAT timestamp of `KERNEL8.IMG` was replaced by the build date.
- A documentation-only ticket: it has no `fix`/`feat` commit and cannot reach DONE through `accept`; after the
  landing the lead brings it to the owner.
