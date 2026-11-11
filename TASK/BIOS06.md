# BIOS06 — `Getmpb`: kernel and SDK header disagree on the `MD` layout

Status: DONE (2026-11-11) | Priority: P1 | Commit: d715e93b7 | Receipt: BIOS06-STATUS-DONE-2.MD

Migrated (owner batch 14, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/BIOS06-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit f77ba8bac (fix(bios): Getmpb publishes the LP64 header layout (BIOS06)), VERIFIED: diff of f77ba8bac (fix(bios): Getmpb publishes the LP64 header layout (BIOS06)) touches tos-api/src/bios/getmpb.rs, kernel/src/selftest_bios.rs, sdk/apps/apitest/apitest.c (+6), names the id by subject, 212 files, legacy line: ERLEDIGT 2026-10-29 ([Quittung](BIOS06-STATUS-DONE.MD)), O3 gemessen (EmuTOS: Bu

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](BIOS06-STATUS-DONE.MD)); O3 gemessen (EmuTOS: Busfehler + Panic), stiller Erfolg bleibt bis zur Anwenderentscheidung · Prioritaet: **P1** · Typ: Researcher (Vertrag festlegen), danach Developer oder SDK — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

(Translated from German 2026-11-11; the `Migrated` line and the quoted old line 3 above stay verbatim.)

## Facts

**BIOS-CORE-003 — S2 — MEASURED — NOT LOCALIZED.** Evidence: report section 2, run `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- Kernel: `tos-types/src/memory.rs:61@MD` (`m_start`/`m_length` 4 bytes each at offset 8/12, 24 bytes). SDK: `sdk/libc/overlay/include/mint/ostruct.h:293@_md` (`long md_start`/`md_length`, under LP64 8 bytes each at 8/16, 32 bytes). Which side breaks the contract is a contract question (COMPATIBILITY "Native vs m68k struct variants"; EmuTOS' `m_start` is a pointer `UBYTE *`). Observed through `Getmpb` (`tos-api/src/bios/getmpb.rs:154@getmpb`).
- Probe G00–G06 (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`): EmuTOS `md_start = 0x00023BA0 = _membot`, `md_length = 0x003B6B60`, `start+length = _memtop`; NewTOS through `_MD`: `md_start = 0x00A00000_00400000`, `md_length = 0`; raw bytes G05 show the kernel layout with the right values (0x00400000, 0x00A00000). Equal: `mp_mfl` a TOS address, `mp_mal = NULL`, `mp_rover` untouched, `m_link`/`m_own` NULL, second call same pointer.
- Side finding: EmuTOS' `themd` lies at `$48E` in protected memory (user-mode read = bus error, the probe reads via `Supexec`); NewTOS allows the user-mode read — the harmless direction.
- `MDX` in `sdk/apps/apitest/apitest.c` (until BIOS06) worked around the discrepancy with its own kernel layout.
- EmuTOS location: `references/emutos/bios/biosmem.c:134-151`.
- Failure: every SDK program that calls `Getmpb` and reads `mp_free->md_length` (memory info programs) reads 0 and a nonsensical start address.

## Work

1. Researcher: settle the contract from COMPATIBILITY and `references/emutos/bios/biosmem.c:134-151` (does the pointer width follow the ABI?), spec for it.
2. Developer or SDK (overlay `ostruct.h` — like [SDK02](SDK02.md)) implements the contract; host test or `_Static_assert` against the spec.

## Acceptance

- Confirming case: probe G00–G06 with the contract layout equal to the oracle (`md_length` = remaining length, `start+length` = end of memory); proposed row `13:0 b:0 0:12.L` (report section 5) needs a super-state prelude ([BIOS13](BIOS13.md)).
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` newly differs.
- `cargo xtask test` green.
- `docs/STATUS.md` and spec in the same commit; `cargo xtask doc-check` of the changed docs 0 errors.

## User decision 2026-10-28

- The content behind the returned pointer is accepted by tosdiff as soon as
  [TOOL16](TOOL16.md) (dumper reads through returned pointers, layout from
  `docs/api/`) is in place; TOOL16 comes before it in the queue.
- Until then the probe rule from CLAUDE.md "Testing" applies to the MEASURED
  part (probe as a permanent test, both sides against the pinned oracle, with
  input trace; follow-up ticket to tosdiff).

## State 2026-10-28 (contract, draft)

- Contract as a proposal in `docs/api/bios/00-getmpb.md` ("NewTOS contract
  [BIOS06]", V1–V5) and `docs/api/structures/md.md`: LP64 header layout
  (`_MD` 32 bytes, `_MPB` 24 bytes); by derivation the wrong side is the
  kernel, which publishes its internal 24-byte BDOS `MD`; content a static
  snapshot `[membot, memtop)`, only `mp_mfl`/`mp_mal` are written.
  Decisions O1–O4 with the user.
- Program survey: the only reader is `sdk/apps/apitest/apitest.c` (its own
  `MDX`). Card gap: empty by construction for the standard image (all
  executables from local recipes,
  `docs/analysis/iorec-survey-2026-10-28.md`, addendum).
- Side finding: `docs/COMPATIBILITY.md` (TSR blocks findable through `Getmpb`;
  "`Getmpb` unchanged") contradicted the code (`mp_mal` always NULL, one MD).

## User decisions 2026-10-28

- O1 = A (LP64 header layout `_MD` 32 / `_MPB` 24 bytes, a public descriptor
  of its own in the kernel, `offsetof`/`sizeof` checks in the SDK and in the
  kernel, `apitest` loses `MDX`/`MPBX`); O2 = (a) static snapshot
  (COMPATIBILITY already corrected, `c173cddc`); O3 = (a) stays, **first
  measure EmuTOS on `Getmpb(NULL)` on the oracle**; O4 confirmed.
- Localization (lead) before the start, per CLAUDE.md "Testing".

## State 2026-10-29 (TOOL16)

- Case `Getmpb` in `tosdump/cases/xbios-ptr-r1.tsv` (case 34, layout from
  `docs/api/structures/md.md`): oracle `mp_free` `0x48E`, `md_start`
  `0x23BA0`, `md_length` `0x3B6B60`; NewTOS reads the 24-byte mixed format as
  LP64 (`md_start` `0x00A00000_00400000`, `md_length` 0) — DIFF, expected
  until the fix (BIOS-CORE-003).

## Localization (lead, 2026-10-29)

- `getmpb` → `tos-api/src/bios/getmpb.rs@getmpb` (`mapped`); published
  descriptor until BIOS06 `ThemdPage` in `tos-api/src/bios/getmpb.rs` (internal
  24-byte BDOS `MD`). `--up --layers bios,bdos`: 7 functions, `unmapped` only
  `bios_init`, `biosmain` (start-up).
- Cases: `tosdump/cases/xbios-ptr-r1.tsv` case 34 (TOOL16, layout from
  `docs/api/structures/md.md`), probe G00–G06; SDK asserts for `_MD`/`_MPB`
  (SDK04, `sdk/libc/overlay/include/mint/ostruct.h`).

## Owner decision 2026-11-11 (O3)

- `Getmpb(NULL)` behaves like EmuTOS, not "silent success". Measured on the
  oracle (receipt [BIOS06-STATUS-DONE.MD](BIOS06-STATUS-DONE.MD), section on
  O3; `measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`, "O3"): the
  first store `mpb->mp_mfl = &themd` (`references/emutos/bios/biosmem.c:149`)
  takes a bus error in supervisor state and EmuTOS panics — the system halts.
- NewTOS removes the NULL special case: a program's NULL MPB is an address it
  cannot write, so the call and the program end the way an unwritable MPB
  ends them (crash report "Bus Error", `Pterm(-1)`); the system keeps running.
  The halt vs. program end is a sanctioned deviation (`docs/COMPATIBILITY.md`,
  "`Getmpb(NULL)` ends the program, the system keeps running").

## Owner decision 2026-11-11 (O3, super state)

- Option (a): `Getmpb(NULL)` from a program in super state (inside
  `Super()`/`Supexec`, where page 0 is EL0 read-write) also ends the program
  like the user-state case, and nothing in page 0 is written — no `$0`–`$F`
  store, so the page-0 check never adopts garbage into the bus- and
  address-error vector cells `$8`/`$C`.

## Implementation (2026-11-11)

- First commit: the `if mpb == 0 { return 0; }` branch of
  `tos-api/src/bios/getmpb.rs:154@getmpb` removed; the two stores moved into
  `tos-api/src/bios/getmpb.rs:205@fill_mpb` (mp_free, then mp_used; the first
  fault ends the call with `FAULTED`), so the fault path is host-testable.
  In user state a NULL store faulted on page 0; in super state it would have
  written `$0`–`$F` (the open point the super-state decision settles).
- Second commit (super state): a program's NULL is refused **before any
  store, in every state** (`tos-api/src/bios/getmpb.rs:162@refuse_user_write`).
  `tos-api/src/uaccess.rs:890@refuse_user_write` records the fault a store
  to a read-only page reports (`tos-api/src/uaccess.rs:872@refused_write_record`:
  EC `0x25`, ISS `0x4F` = `WnR` + permission L3 as measured for an `sttr`,
  `docs/analysis/arch03-stage0b-spike.md` §5; `FAR` = 0), so the syscall tail
  ends the program through the same path as a faulting store:
  `kernel/src/arch/aarch64/exceptions.rs:775@take_pending_fault` →
  `kernel/src/arch/aarch64/exceptions.rs:1510@uaccess_fault_tail` (crash
  report "Bus Error", `Pterm(-1)`). An in-kernel caller's NULL stays refused
  without a record (`tos-api/src/bios/getmpb.rs:161@is_user`): its pointers
  are kernel addresses, a NULL there is a kernel bug.
- Any other MPB a program cannot write: `Space::User` → `put_user_u64` →
  `sttr` (`tos-api/src/uaccess.rs:198@put_user_u64`), a permission fault at
  the listed `sttr` (e.g. page 0 in user state,
  `tos-api/src/bdos/process/mmu.rs:826@UserRoNx`), resumed at its fixup
  (`kernel/src/arch/aarch64/exceptions.rs:281@fixup_for`), then the same tail.
- Host tests in `tos-api/src/bios/getmpb.rs`:
  `null_mpb_from_a_program_is_refused_before_any_store_in_any_state`,
  `unwritable_mpb_faults_and_writes_nothing`,
  `writable_mpb_gets_free_then_used_only`,
  `null_mpb_from_the_kernel_is_refused_before_any_store`. Receipt
  [BIOS06-STATUS-UNFINISHED.MD](BIOS06-STATUS-UNFINISHED.MD).
