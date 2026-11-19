# XBIOS14 — Oracle limit: Hatari accepts no write access to the RTC, `Gettime` after `Settime` is not comparable

Status: OPEN (2026-11-19) | Priority: P3

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS14.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Orakelgrenze, blockiert keinen Fix)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: Orakelgrenze; blockiert keinen Fix) · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-007 — S4 — MEASURED (oracle limit).** Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace/trace.txt`: after `Settime(0x2A4320A3)` EmuTOS (`nvram_setregs`, `references/emutos/bios/clock.c:893-907`) writes year 33, month 2, day 3, 4:05:06 into the registers 9…0; the following `Gettime` reads 58/10/28 12:20:12 (host clock). In all EmuTOS runs `Gettime` stays the host clock.
- Consequence: `Gettime` after `Settime` and `Gettime` with bit 31 ([XBIOS09](XBIOS09.md)) are not comparable on the oracle; the audit compares `Settime` through the GEMDOS cache (`Tgetdate`/`Tgettime`), which `Settime` always writes on EmuTOS (`references/emutos/bios/clock.c:1221-1222`).

## Work

1. Record the limit in `docs/analysis/ORACLE.md`; check whether a profile or a Hatari option provides a writable RTC (not from memory — give the source or the measurement).

## Acceptance

- The limit stands in `docs/analysis/ORACLE.md` with its evidence; the two rows `+14:0x16` / `14:0x17` of report section 5 are noted as "waits for an RTC-capable oracle".
- No tosdump case that was equal before differs newly; `cargo xtask test` green; `cargo xtask doc-check` of the edited docs 0 failures.

## Status history

- 2026-11-19: reopened (was "Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: oracle limit, blocks no fix | Reopen: a fix is blocked by the oracle limit"): owner 2026-11-19: the time functions must be clean, the oracle limit of the clock is recorded

## State at the reopening (lead, 2026-11-19)

Owner 2026-11-19: reopen TASK/XBIOS13.md and this ticket ("mit 1 weiter").

- TASK/XBIOS09.md is DONE (`4fa990dad`): its acceptance case was the host test, because of this limit.
- TASK/XBIOS08.md is ACCEPTED: `Settime` with impossible fields is a sanctioned deviation, compared through the
  GEMDOS cache.
- The check for a writable RTC reads the pinned Hatari's source or documentation, or measures on the pinned
  oracle; it changes neither the pin nor the oracle configuration. A finding that another profile or option would
  give a writable RTC is reported to the owner as a proposal, not applied.
- A documentation-only ticket: it has no `fix`/`feat` commit and cannot reach DONE through `accept`; after the
  landing the lead brings it to the owner.

## Landed with the gap accepted (2026-11-19)

The researcher returned GAPS (`ac9cbf8d6`): section 9 of `docs/analysis/ORACLE.md` states the limit and the two
waiting rows, but "the pinned Hatari has no option for a writable RTC" rests on the manual, the todo list and the
release notes of the Hatari 2.6.1 bundle; the Hatari source is not in `references/`.

Owner 2026-11-19, asked with three options (1: land as it is and accept the gap; 2: fetch the Hatari 2.6.1 source
and have the code read; 3: `--rtc-year` in the oracle configuration): option 1. Landed as `fecd9c426`.

The ticket stays OPEN until the owner closes it.
