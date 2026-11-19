# XBIOS09 — `Gettime`: packed LONG zero-extended; positive instead of negative from the year 2044 on

Status: DONE (2026-11-19) | Priority: P2 | Commit: 4fa990dad | Receipt: XBIOS09-STATUS-DONE.MD

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS09.md status line: GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-002 — S3 — MEASURED.** Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Probe `xshi.c` H01, `Settime(0x8CCF6000)` (2050-06-15 12:00:00): `ret=8ccf6000 hi=00000000 negative-as-long=no` (`measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b-repeat`).
- Localized: `tos-api/src/xbios/mod.rs:124@GETTIME` (`xbios_gettime() as i64` on a `u32`).
- EmuTOS side: `LONG gettime(void)` (`references/emutos/bios/clock.c:1266`); the upper half is the sign extension of the LONG (`docs/TOS-REFERENCE-DUMPER.md` §3, `ret_hi`). **Not measurable on the oracle:** Hatari returns the host clock (2026), never bit 31 (`measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace`, see [XBIOS14](XBIOS14.md)) — there the finding rests on the ABI rule (read, not measured).
- Spec: `docs/api/xbios/17-gettime.md` "LONG result **sign-extended** into `x0`", likewise `sdk/libc/mintlib/include/mint/osbind.h`; the comment at `tos-api/src/xbios/mod.rs:120-122@SETTIME` says "zero-extended". Same class as [XBIOS06](XBIOS06.md).
- Symptom: `long t = Gettime(); if (t < 0) …` comes out differently from 2044 on.

## Work

1. Sign extension of the LONG in the dispatcher; correct the comment at `tos-api/src/xbios/mod.rs:120-122@SETTIME`; host test with `0x8CCF6000`.

## Acceptance

- Confirming case: probe H01 (`hi=ffffffff`); not measurable on the oracle (XBIOS14), so the host test counts as the acceptance case; the proposed row `14:0x17 -` (report section 5) needs an oracle with a writable RTC.
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` differs newly.
- `cargo xtask test` green.
- `docs/STATUS.md` and the spec in the same commit; `cargo xtask doc-check` of the edited docs 0 failures.

## Owner 2026-10-28

- Sign extension by the rule (`docs/COMPATIBILITY.md`, "LONG
  results in `x0`"): `Gettime` extends with zeros today
  (`tos-api/src/xbios/mod.rs:124@GETTIME`).

## Parked (owner, 2026-10-30)

No program on the card is affected (lead's check 2026-10-30);
resume when a port needs it.

## Status history

- 2026-11-19: reopened (was "Status: PARKED (2026-11-11) | Priority: P2 | Reason: owner 2026-10-30: until a port needs it | Reopen: a port needs it"): owner 2026-11-19: the time functions must be clean

## Reopened; localization and retest set (lead, 2026-11-19)

Owner 2026-11-19: the time functions must be clean ("die Time funktionen müssen schon sauber sein"); the RTC module
is at hand (TASK/RTC01.md). Order: this ticket, then the documents of TASK/XBIOS08.md (owner 2026-11-19: option B,
the normalising `Settime` stays and is entered as a deviation), TASK/XBIOS13.md and TASK/XBIOS14.md.

- Target: `tos-api/src/xbios/mod.rs:124@GETTIME`; the value comes from `tos-api/src/gemdos/time.rs:227@xbios_gettime`
  (`docs/analysis/deps/mapping.tsv`, row `gettime`, mapped).
- `cargo xtask deps gettime --up`: the trap entry `xbios_vecs[0x17]` and `time_init` (`bdos/time.c:162`, mapped to
  `tos-api/src/gemdos/time.rs`, `time_init`). Unmapped in the touched layer: `bios_init`, `biosmain`
  (`bios/bios.c`), `cprint_asctime` (`bios/initinfo.c`): boot code and the boot screen's clock line, which call
  `gettime` inside EmuTOS and do not pass the trap return this ticket changes.
- `cargo xtask deps gettime --down`: unmapped in the touched layer `flush_data_cache`, `invalidate_data_cache`
  (`bios/processor.S`), reached through the RTC drivers, which are `design:hardware:rtc`.
- Retest: the host test of the work item; `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv` on raspi4b; `time_init` (the boot clock) unchanged.
- The acceptance line "`docs/STATUS.md` … in the same commit" predates the rule that only the lead edits
  `docs/STATUS.md`: the developer proposes its line in the receipt.
