# XBIOS11 — `Random` takes its first seed from the 60 Hz frame clock instead of `_hz_200`

Status: IMPLEMENTED (2026-11-20) | Priority: P3 | Commit: self | Missing: tester runs of probe X01 on both sides (bios-arg-trace on NewTOS), xsprobe R01-R03 and P08, and bios-r1, bios-core-r1, xbios-chardev-r1, xbios-system-r1 on raspi4b

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS11.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer TeraDesk-Formatieren)) · Prioritaet: **P3** · Typ: Developer (Seed aus `_hz_200`) oder Researcher (COMPATIBILITY-Eintrag) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-004 — S4 — MEASURED.** Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Probe `xsseed.c` X01 (first `Random()` after the start, `_hz_200` before and after): EmuTOS `yes` (tt, tt-repeat, falcontc), NewTOS `no` (both runs, `hz_200` 3024, value `91cc67` = seed from `frame_clock` 908) (`measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-NEWTOS-raspi4b`). The generator itself is equal (R01–R03, P08).
- Localized: `tos-api/src/xbios/system.rs:14-28@random` (`h.frame_clock()`; `s = 1` for a seed of 0, where EmuTOS goes on with 0 — unreachable in practice).
- EmuTOS place: `references/emutos/bios/xbios.c:421-430`.
- Spec: `docs/api/xbios/11-random.md` lists the auto seed as "Reference", not as contract; no COMPATIBILITY entry.
- Symptom: only a program that reads `_hz_200` and predicts the first `Random` value.

## Acceptance

- Confirming case: probe X01 (`yes`) equal to the oracle; R01–R03 and P08 stay equal.
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` differs newly.
- `cargo xtask test` green.
- `docs/STATUS.md` and the spec in the same commit; `cargo xtask doc-check` of the edited docs 0 failures.

## Status history

- 2026-11-20: reopened (was "Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: no card program depends on the seed origin (only user is TeraDesk formatting) | Reopen: a program depends on the seed origin"): owner 2026-11-20: EmuTOS is the yardstick, the review of the parked tickets (TASK/BIOS23.md) classed it as fixable; first of four in the queue


## Reopened; localization and retest set (lead, 2026-11-20)

Owner 2026-11-20: EmuTOS is the yardstick; the review `docs/analysis/bios-parked-review-2026-11-20.md` (TASK/BIOS23.md)
classed this ticket as fixable, first of four in the queue.

- The reason the spec gave for the frame-clock seed is gone: `_hz_200` is a live counter, readable in the kernel
  through `tos-api/src/bios/tick.rs:92@hz_200`.
- Target: `tos-api/src/xbios/system.rs:14-28@random` (`docs/analysis/deps/mapping.tsv`, row `random`, mapped). The
  bump of a zero seed to 1 goes as well: EmuTOS keeps 0.
- `cargo xtask deps random --up`: the trap entry `xbios_vecs[0x11]` and, inside EmuTOS, the desktop's `write_boot` /
  `protobt` path (layer `desk`, no trap return involved). No unmapped function in the touched layer (`bios`).
- Retest: the probe; the host tests of `Random`; `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv` on raspi4b.
- The probe `xsseed.c` exists only in the run of 2026-10-28. As the acceptance case of a fix it is committed as a
  permanent probe under `tosdump/probes/` and run on both sides (CLAUDE.md, "Testing"); a follow-up ticket moves
  the case to tosdiff if a table case can express it (the first call must come before any other `Random`).
- The acceptance line "`docs/STATUS.md` … in the same commit" predates the rule that only the lead edits
  `docs/STATUS.md`: the developer proposes its line in the receipt.
- Spec gate first: the Pi-binding section of `docs/api/xbios/11-random.md` still describes the frame-clock seed.

## Spec gate landed; developer stopped (2026-11-20)

Spec gate READY, landed as `9e0481fb7`: `docs/api/xbios/11-random.md` (found beyond the ticket: EmuTOS re-seeds on
every call that finds the state 0, not only on the first).

The developer was started and stopped on the owner's word the same night (machine noise) before it committed. Its
uncommitted work is in the worktree `target-agents/agent-acc781cc44394cbbc/wt` on branch `xbios11`: edits to
`tos-api/src/xbios/system.rs`, `docs/api/xbios/11-random.md`, `tosdump/Makefile`, and the new files of the host
test and the permanent probe. Nothing is tested. The developer is resumed or restarted on the owner's word.
