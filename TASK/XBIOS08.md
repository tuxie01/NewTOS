# XBIOS08 — `Settime` normalises or clamps impossible fields instead of taking them raw

Status: ACCEPTED (2026-11-19) | Priority: P2 | Ref: docs/COMPATIBILITY.md

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS08.md status line: GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: **P2** · Typ: Developer (rohe Felder halten wie EmuTOS) oder Researcher (Abweichung in COMPATIBILITY, Spec korrigieren) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-001 — S3 — MEASURED.** Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`; comparison `measurements/measurement-2026-10-28-xbios-system-audit/probe-diff-emutos-tt-vs-newtos.txt`; runs `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-NEWTOS-raspi4b`.

Probe T10–T22 (read back at once), T23–T28 (after the next clock tick). Set → read, EmuTOS / NewTOS:

- `2001-02-03 25:00:00` → `25:00:00` / `2001-02-04 01:00:00`
- `2001-02-03 04:05:62` → `04:05:62` / `04:06:02`; after one tick `04:06:00` / `04:06:04`
- `2001-13-03` → `2001-13-03` / `2001-12-03`; `2001-02-30` → `2001-02-30` / `2001-02-28`
- `0x00000000` → `1980-00-00 00:00:00` / `1980-01-01 00:00:00`
- `2001-02-03 23:59:62` after one tick → `23:60:00` / `2001-02-04 00:00:02`
- `2001-02-30 23:59:58` after one tick → `2001-03-01 00:00:00` / `2001-02-28 23:59:58`

Valid values (T01–T08: change of day, month and year, leap year 2000, 2100, overflow 2107→1980) are equal.

- Localized: `tos-api/src/gemdos/time.rs:205-216@xbios_settime` (month and day clamped, `compose` converts hour, minute and second into seconds); observed through `Tgetdate`/`Tgettime` and `Gettime`.
- Input check: EmuTOS — the 27 `Settime` arguments in the Hatari trace (`measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt/trace-program.txt`) are the `set=` values of the log; NewTOS indirectly (T01 reads back exactly).
- EmuTOS places: `references/emutos/bios/clock.c:1218-1222` (`current_time`/`current_date` set raw), `references/emutos/bdos/time.c:175-230` (`tikfrk` counts the raw fields on), `references/emutos/bdos/time.c:98-135` (`Tgetdate`/`Tgettime` return the cache).
- Spec: `docs/api/xbios/16-settime.md` notes the clamping as a "documented deviation", but claims that hour, minute and second "pass through unvalidated" (see [XBIOS13](XBIOS13.md)). No COMPATIBILITY entry.
- Symptom: clock-setting or test programs that write raw values and read them back; normal programs set valid times.

## Work

1. Decision researcher/owner: keep the raw fields or enter the deviation.
2. Fix against the EmuTOS places above; host tests for the example cases.

## Acceptance

- Confirming case: probe T10–T28 (Tgetdate/Tgettime after `Settime`) equal to the oracle; on the oracle `Settime` is comparable only through the GEMDOS cache ([XBIOS14](XBIOS14.md)).
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` differs newly.
- `cargo xtask test` green.
- `docs/STATUS.md` and the spec in the same commit; `cargo xtask doc-check` of the edited docs 0 failures.

## Parked (owner, 2026-10-30)

No program on the card is affected (lead's check 2026-10-30);
resume when a port needs it.

## Status history

- 2026-11-19: reopened (was "Status: PARKED (2026-11-11) | Priority: P2 | Reason: owner 2026-10-30: until a port needs it | Reopen: a port needs it"): owner 2026-11-19: the time functions must be clean; option B, the normalising Settime stays and is entered as a deviation

## Owner decision 2026-11-19: option B, the deviation is entered

Asked with the two options of work item 1 (A: raw fields like EmuTOS; B: NewTOS keeps normalising, entered in
`docs/COMPATIBILITY.md` and `docs/DEVIATIONS.md`, no code), the owner chose B ("dann würde ich sagen B"). Stated with
the question: a DS3231 (TASK/RTC01.md) cannot store impossible values.

So work item 2 and the first acceptance line (probe T10–T28 equal to the oracle) do not apply. What counts instead,
as described to the owner with option B:

- `docs/COMPATIBILITY.md` has an entry for `Settime` with the measured pairs above (what a program sets, what it
  reads back on EmuTOS and on NewTOS, at once and after a tick), and `docs/DEVIATIONS.md` a row by trigger.
- `docs/api/xbios/16-settime.md` states the normalisation of hour, minute and second and the clamping of month and
  day as they are in the code; the sentence "pass through unvalidated" and the code comment "(EmuTOS parity)" at
  `tos-api/src/gemdos/time.rs:211@hour` are wrong (the comment is a developer edit; it is listed for the next code
  change of that function, not made here).
- No code change; `cargo xtask doc-check` of the edited docs 0 failures.
- Then the lead sets the ticket ACCEPTED with the entry as its reference.

The points of TASK/XBIOS13.md that concern `Settime` (its point 1) are closed by the same document step.
- 2026-11-19: set to ACCEPTED by `cargo xtask set-status` (was "Status: OPEN (2026-11-19) | Priority: P2")

## Entered and accepted (2026-11-19)

Documents landed as `2c429952f` (researcher `c074f677b`): `docs/COMPATIBILITY.md`, entry "`Settime` with impossible
fields: NewTOS normalises, EmuTOS keeps them raw"; `docs/DEVIATIONS.md`, XBIOS section; `docs/api/xbios/16-settime.md`.

Corrections to the facts above, found by the researcher in the run's logs:

- For the set value `04:05:62` the NewTOS log has `04:06:02` in T24, not `04:06:04`.
- For T23–T28 the NewTOS values are reads at once, not after a tick: the probe polls until the read differs from the
  raw set value, and the normalised NewTOS value differs already. NewTOS after a tick is not measured there.
- The code comment "(EmuTOS parity)" stands at `tos-api/src/gemdos/time.rs:216@hour`; the doc comment above
  `xbios_settime` also says the fields pass through unchanged. Both are wrong and listed in the spec's open points
  for the next code change of that function.
