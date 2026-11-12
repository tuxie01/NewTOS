# XBIOS10 — `NVMaccess` is not wired: every operation returns `0x2E`

Status: ACCEPTED (2026-11-12) | Priority: P2 | Ref: ADR-012

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS10.md status line: GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28: kein NVRAM, Antwort wie EmuTOS ohne NVRAM-Chip — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Facts

**XBIOS-SYS-003 — S3 — CONFIRMED** (S2 if the Falcon profile has to show NVRAM). Evidence: report section 2, run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Cases: `bios-r1` 189–194 and `xbios-system-r1` 68–76, all EXACT-DIFF (EmuTOS `0x0000` or `0xFFFB`, NewTOS `0x002E`; on a read the buffer stays poisoned) (`measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-xbios-system-r1/tosdiff-newtos-vs-emutos.txt`; falcontc the same case IDs).
- Input check: EmuTOS — 9 calls without arguments in the trace (`measurements/measurement-2026-10-28-xbios-system-audit/runs/TRACE-EMUTOS-xs/trace-program.txt`), results match the arms of the table (read 0, range/op error -5); NewTOS — the answer depends on no argument.
- Localized: `tos-api/src/xbios/mod.rs:148@other` (catch-all arm).
- EmuTOS location: `references/emutos/bios/nvram.c:155-214`. Noticed, not measured: `0x2E` is EmuTOS' answer on a machine without NVRAM (`references/emutos/bios/nvram.c:161-162`, `!has_nvram`); there is no pinned profile without NVRAM.
- Spec contradictory: `docs/api/xbios/_triage.md:80` ("`CONF_WITH_NVRAM` off") against `docs/api/xbios/_full-function-inventory.md` (`make 512`: `CONF_WITH_NVRAM` = 1); `docs/COMPATIBILITY.md` only in the keyboard section "NewTOS has no NVRAM".
- Failure picture: Falcon/TT control panels see "no NVRAM" and fall back; no data is lost.

## Work

1. Present the decision: "no NVRAM" with a COMPATIBILITY entry, or emulated NVRAM; make the spec consistent ([XBIOS13](XBIOS13.md)).
2. A developer ticket only on a decision for NVRAM; fix against `references/emutos/bios/nvram.c:155-214`.

## Acceptance

- On a decision for NVRAM: `bios-r1` 189–194 and `xbios-system-r1` 68–76 EQUAL against the oracle (tt and falcontc); otherwise an entry in `docs/COMPATIBILITY.md` and the cases ACCEPTED.
- No case equal before in the four tables differs anew; `cargo xtask test` green; `cargo xtask doc-check` of the changed docs 0 failures.

## Owner decision 2026-10-28

- The NVRAM question follows from which machine NewTOS reports (`_MCH`).
  ADR-012 only fixes that there is an own identifier; the value in the
  code is provisional. First [ARCH05](ARCH05.md) (ADR), then this ticket.

## Owner decision 2026-10-28 (after ARCH05)

- Independent of `_MCH` (EmuTOS switches NVRAM via `has_nvram`,
  `references/emutos/bios/nvram.c:161`). NewTOS has no NVRAM → `NVMaccess`
  answers like EmuTOS without an NVRAM chip.
- Measurement for it with an **ST profile** in Hatari (a profile without NVRAM;
  `tt` and `falcontc` have one): as a new oracle profile with its own
  canary like TOOL06 (`docs/analysis/ORACLE.md`), existing profiles
  unchanged — part of this ticket, because the acceptance needs it.
- Localization (lead) before the start per CLAUDE.md "Testing".

## Parked (owner, 2026-10-30)

No program on the card is affected (lead's check 2026-10-30); resume when a
port needs it.

## Owner decision 2026-11-12: accepted

Owner: "genauso ist es mit DSP, TT-SHIFTER, NVMAccess" (the same goes for DSP, TT shifter, NVMaccess, as for the
obsolete floppy routines). NewTOS has no NVRAM and answers `0x2E`, EmuTOS' answer without an NVRAM chip
(ADR-012). The cases above count as ACCEPTED: `docs/COMPATIBILITY.md`, "XBIOS for absent Atari hardware: NVRAM, TT
shifter, DSP". The ST-profile measurement is dropped; the spec contradiction stays with [XBIOS13](XBIOS13.md).
Reopen when a port needs NVRAM.

## Status history

- 2026-11-12: set to ACCEPTED by `cargo xtask set-status` (was "Status: PARKED (2026-11-11) | Priority: P2 | Reason: owner 2026-10-30: until a port needs it (no NVRAM, answer like EmuTOS without an NVRAM chip) | Reopen: a port needs it")
