# BIOS21 — tosdump: a `peek` case type that reads system variables on the oracle and on NewTOS

Status: IMPLEMENTED (2026-11-20) | Priority: P3 | Commit: self | Missing: tester re-run of bios-sysvars-r1 on the pinned oracle (tt, falcontc) and raspi4b, the existing tables bios-r1 and bios-core-r1 unchanged on both sides, make -C tosdump host-test; the lead decides whether the hardware/missing rows get documented-difference cases

Type: developer, then tester. Owner decision 2026-11-19, from TASK/BIOS19.md; starts after TASK/BIOS20.md.

## Facts

- The acceptance of TASK/BIOS19.md asks for oracle read cases for the served system variables.
- `Setexc(num, -1)` reads only addresses that are multiples of 4 and returns a LONG: of the 37 served variables 11
  are reachable that way, 3 of them narrower than a LONG (`docs/api/bios/_sysvars-contract.md`, section 8). The
  existing cases are rows 55-62 of `tosdump/cases/bios-core-r1.tsv`.
- A user-mode read below `$800` is a bus error on EmuTOS; `tosdump/probes/hkprobe.c` already reads through
  `Supexec`.

## Work

1. A case type `peek:<addr>:<width>` (width 1, 2 or 4), executed inside a `Supexec` callback, read-only, the
   address limited to the system-variable area, as section 8 of the contract file proposes; the dumper
   documentation (`docs/TOS-REFERENCE-DUMPER.md`) describes it.
2. A case table with one case per served variable, with the return class section 8 maps it to; a class for a
   monotone counter if `_hz_200` needs one.
3. Reference dump on the pinned oracle, the NewTOS run, tosdiff.

## Acceptance

- Every served row of the contract table has a case; the table runs on the oracle and on NewTOS; every difference
  is either fixed in another ticket or a row of `docs/DEVIATIONS.md`.
- The existing tables and their frozen references are unchanged; `cargo xtask test` green; doc-check 0 failures.
